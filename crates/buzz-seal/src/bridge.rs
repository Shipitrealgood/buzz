//! NIP-98 HTTP bridge client — the one publish/query path both mint halves
//! share.
//!
//! Extracted from `buzz-provision agent` (T4) when the T4 e2e pinned the
//! relay's submitter-identity rule (`event.pubkey` must equal the NIP-98
//! authenticated identity, ingest.rs) and #2694 ruled that `buzz-mint attest`
//! therefore publishes the owner-signed kind:9000 batch DIRECTLY, owner-
//! authenticated, with fresh timestamps. One authority, two consumers — the
//! same rule `bundle` follows for the bundle schema.
//!
//! Nothing here formats secret material: signing keys go into signatures
//! only; errors carry status codes, URLs, and bounded response excerpts.

use base64::Engine;
use nostr::{Event, EventBuilder, JsonUtil, Keys, Kind, Tag};

/// Bound on echoed relay response bodies (diagnostics only, never secrets).
const MAX_ECHOED_BODY: usize = 500;

/// Bridge-call failures. `Rejected` is the only "relay said no" variant —
/// callers that treat rejections as warnings (attest's 9000 batch, per the
/// #2694 amendment) can match on it; everything else is transport/protocol.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("cannot sign NIP-98 auth event: {0}")]
    Sign(String),
    #[error("{method} {url} failed: {detail}")]
    Transport {
        method: &'static str,
        url: String,
        detail: String,
    },
    #[error("relay returned {status}: {excerpt}{hint}")]
    Http {
        status: u16,
        excerpt: String,
        hint: &'static str,
    },
    #[error("relay did not accept the event: {message}")]
    Rejected { message: String },
    #[error("relay response not JSON: {excerpt}")]
    BadResponse { excerpt: String },
}

/// ws(s):// → http(s):// base URL, no trailing slash.
pub fn http_base_url(relay_url: &str) -> String {
    relay_url
        .replacen("wss://", "https://", 1)
        .replacen("ws://", "http://", 1)
        .trim_end_matches('/')
        .to_owned()
}

/// Build a blocking HTTP client with the bridge's standard timeouts.
pub fn client() -> Result<reqwest::blocking::Client, BridgeError> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| BridgeError::Transport {
            method: "INIT",
            url: String::new(),
            detail: e.to_string(),
        })
}

/// Sign a NIP-98 HTTP-auth event (kind:27235) → `Authorization` header value.
/// Same tag shape as buzz-cli's `sign_nip98`: u, method, payload, nonce.
pub fn nip98_header(
    keys: &Keys,
    method: &str,
    url: &str,
    body: &[u8],
) -> Result<String, BridgeError> {
    use nostr::hashes::{sha256, Hash};
    let payload_hex = sha256::Hash::hash(body).to_string();
    let mut tags = Vec::with_capacity(4);
    for pair in [
        ["u", url],
        ["method", method],
        ["payload", &payload_hex],
        // Nonce keeps rapid retries distinct under the relay's replay guard.
        ["nonce", &uuid::Uuid::new_v4().to_string()],
    ] {
        tags.push(Tag::parse(pair).map_err(|e| BridgeError::Sign(e.to_string()))?);
    }
    let event = EventBuilder::new(Kind::Custom(27235), "")
        .tags(tags)
        .sign_with_keys(keys)
        .map_err(|e| BridgeError::Sign(e.to_string()))?;
    Ok(format!(
        "Nostr {}",
        base64::engine::general_purpose::STANDARD.encode(event.as_json())
    ))
}

fn excerpt_of(text: &str) -> String {
    text.chars().take(MAX_ECHOED_BODY).collect()
}

/// POST one signed event to `<base>/events`, NIP-98-authenticated as
/// `signer`, optionally carrying the NIP-OA delegation header (`x-auth-tag`).
/// Returns the relay-confirmed event id.
pub fn post_event(
    client: &reqwest::blocking::Client,
    http_base: &str,
    signer: &Keys,
    auth_tag_json: Option<&str>,
    event: &Event,
) -> Result<String, BridgeError> {
    let url = format!("{http_base}/events");
    let body = event.as_json().into_bytes();
    let auth = nip98_header(signer, "POST", &url, &body)?;
    let mut req = client
        .post(&url)
        .header("Authorization", auth)
        .header("Content-Type", "application/json");
    if let Some(tag) = auth_tag_json {
        req = req.header("x-auth-tag", tag);
    }
    let resp = req.body(body).send().map_err(|e| BridgeError::Transport {
        method: "POST",
        url: url.clone(),
        detail: e.to_string(),
    })?;
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        let hint = if status.as_u16() == 403 {
            "\nhint: a 403 here usually means the relay is not running with \
             BUZZ_ALLOW_NIP_OA_AUTH=true, the submitter is not a relay member, or the relay \
             requires the event author itself to be the authenticated submitter"
        } else {
            ""
        };
        return Err(BridgeError::Http {
            status: status.as_u16(),
            excerpt: excerpt_of(&text),
            hint,
        });
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| BridgeError::BadResponse {
            excerpt: excerpt_of(&text),
        })?;
    let accepted = parsed
        .get("accepted")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !accepted {
        return Err(BridgeError::Rejected {
            message: parsed
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(&excerpt_of(&text))
                .to_owned(),
        });
    }
    Ok(parsed
        .get("event_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&event.id.to_hex())
        .to_owned())
}

/// POST a single Nostr filter to `<base>/query`, NIP-98-authenticated as
/// `signer` (with optional `x-auth-tag` for NIP-OA virtual members).
/// Returns the JSON array of matching events.
pub fn query_events(
    client: &reqwest::blocking::Client,
    http_base: &str,
    signer: &Keys,
    auth_tag_json: Option<&str>,
    filter: &serde_json::Value,
) -> Result<Vec<serde_json::Value>, BridgeError> {
    let url = format!("{http_base}/query");
    let body = serde_json::to_vec(&[filter]).map_err(|e| BridgeError::Sign(e.to_string()))?;
    let auth = nip98_header(signer, "POST", &url, &body)?;
    let mut req = client
        .post(&url)
        .header("Authorization", auth)
        .header("Content-Type", "application/json");
    if let Some(tag) = auth_tag_json {
        req = req.header("x-auth-tag", tag);
    }
    let resp = req.body(body).send().map_err(|e| BridgeError::Transport {
        method: "POST",
        url: url.clone(),
        detail: e.to_string(),
    })?;
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(BridgeError::Http {
            status: status.as_u16(),
            excerpt: excerpt_of(&text),
            hint: "",
        });
    }
    serde_json::from_str(&text).map_err(|_| BridgeError::BadResponse {
        excerpt: excerpt_of(&text),
    })
}
