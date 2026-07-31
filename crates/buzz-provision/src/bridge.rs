//! NIP-98 HTTP bridge client — the one publish/query path both mint halves
//! share.
//!
//! Exists because the relay's submitter-identity rule (`event.pubkey` must
//! equal the NIP-98 authenticated identity — pinned by the ceremony e2e)
//! means `buzz-mint attest` publishes the owner-signed kind:9000 batch
//! DIRECTLY, owner-authenticated, with fresh timestamps. One authority, two
//! consumers — the same rule `bundle` follows for the bundle schema.
//!
//! Nothing here formats secret material: signing keys go into signatures
//! only; errors carry status codes, URLs, and bounded response excerpts.

use base64::Engine;
use nostr::{Event, EventBuilder, JsonUtil, Keys, Kind, Tag};

/// Bound on echoed relay response bodies (diagnostics only, never secrets).
const MAX_ECHOED_BODY: usize = 500;

/// Bridge-call failures. `Rejected` is the "relay said no" variant —
/// callers that treat rejections as warnings (attest's 9000 batch) can match
/// on it; `Dominated` is acceptance that changed nothing; everything else is
/// transport/protocol.
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
    #[error(
        "relay kept an existing event over this one ({message}) — the write was silently \
         dominated; the published state did NOT change"
    )]
    Dominated { message: String },
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

/// Attempts per bridge call. Blind retry is safe BY CONSTRUCTION for this
/// tool: every event it publishes is replaceable (kind:0/10100) or
/// membership-idempotent (9000/9022 re-adds and re-leaves converge), so a
/// stored-but-unconfirmed write re-sent is harmless — unlike chat messages,
/// which is why buzz-cli needs a delivery-ambiguity taxonomy and this
/// client deliberately does not.
const ATTEMPTS: u32 = 3;

/// Parse a relay `retry in <N>s` hint out of a 429 body, capped at 30s.
fn parse_retry_hint_secs(body: &str) -> Option<u64> {
    let idx = body.find("retry in ")?;
    let digits: String = body[idx + "retry in ".len()..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse::<u64>().ok().map(|n| n.min(30))
}

/// One NIP-98-authenticated POST with bounded retry. The auth header is
/// re-signed on EVERY attempt (fresh created_at + nonce — the relay's ±60s
/// NIP-98 window and replay guard both punish reuse). Retries connect/
/// timeout-class transport failures and 429/502/503/504, honoring a 429
/// `retry in Ns` hint.
fn post_authed(
    client: &reqwest::blocking::Client,
    url: &str,
    signer: &Keys,
    auth_tag_json: Option<&str>,
    body: &[u8],
) -> Result<(u16, String), BridgeError> {
    let mut last: Option<BridgeError> = None;
    for attempt in 1..=ATTEMPTS {
        let auth = nip98_header(signer, "POST", url, body)?;
        let mut req = client
            .post(url)
            .header("Authorization", auth)
            .header("Content-Type", "application/json");
        if let Some(tag) = auth_tag_json {
            req = req.header("x-auth-tag", tag);
        }
        let outcome: Result<(), BridgeError> = match req.body(body.to_vec()).send() {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let text = resp.text().unwrap_or_default();
                if matches!(status, 429 | 502 | 503 | 504) && attempt < ATTEMPTS {
                    let wait = parse_retry_hint_secs(&text).unwrap_or(u64::from(attempt));
                    eprintln!(
                        "note: relay returned {status}; retrying in {wait}s \
                         (attempt {attempt}/{ATTEMPTS})"
                    );
                    std::thread::sleep(std::time::Duration::from_secs(wait));
                    Err(BridgeError::Http {
                        status,
                        excerpt: excerpt_of(&text),
                        hint: "",
                    })
                } else {
                    return Ok((status, text));
                }
            }
            Err(e) if (e.is_connect() || e.is_timeout()) && attempt < ATTEMPTS => {
                std::thread::sleep(std::time::Duration::from_secs(u64::from(attempt)));
                Err(BridgeError::Transport {
                    method: "POST",
                    url: url.to_owned(),
                    detail: e.to_string(),
                })
            }
            Err(e) => {
                return Err(BridgeError::Transport {
                    method: "POST",
                    url: url.to_owned(),
                    detail: e.to_string(),
                })
            }
        };
        last = outcome.err();
    }
    Err(last.unwrap_or(BridgeError::Transport {
        method: "POST",
        url: url.to_owned(),
        detail: "retries exhausted".to_owned(),
    }))
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
    let (status, text) = post_authed(client, &url, signer, auth_tag_json, &body)?;
    if !(200..300).contains(&status) {
        let hint = if status == 403 {
            "\nhint: a 403 here usually means the relay is not running with \
             BUZZ_ALLOW_NIP_OA_AUTH=true, the submitter is not a relay member, or the relay \
             requires the event author itself to be the authenticated submitter"
        } else {
            ""
        };
        return Err(BridgeError::Http {
            status,
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
    let message = parsed
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !accepted {
        return Err(BridgeError::Rejected {
            message: if message.is_empty() {
                excerpt_of(&text)
            } else {
                message.to_owned()
            },
        });
    }
    // The relay answers a DOMINATED replaceable write (older or same-second-
    // higher-id than the live head) with accepted:true + "duplicate:" — an
    // acceptance that changed nothing. Surfacing it as an error is what makes
    // "published" mean "live" (buzz-db replace_addressable_event semantics).
    if message.starts_with("duplicate:") {
        return Err(BridgeError::Dominated {
            message: message.to_owned(),
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
    let (status, text) = post_authed(client, &url, signer, auth_tag_json, &body)?;
    if !(200..300).contains(&status) {
        return Err(BridgeError::Http {
            status,
            excerpt: excerpt_of(&text),
            hint: "",
        });
    }
    serde_json::from_str(&text).map_err(|_| BridgeError::BadResponse {
        excerpt: excerpt_of(&text),
    })
}

/// The `created_at` a REPLACEABLE event must be signed with to be guaranteed
/// to become the live head: strictly after the currently-live event of the
/// same (kind, author).
///
/// The relay keeps the highest `created_at` and breaks same-second ties by
/// LOWEST event id (buzz-db `replace_addressable_event`) — so signing "now"
/// loses a same-second race ~50% of the time, silently. Same bump rule as
/// buzz-admin's `publish_membership_list_with_bump`. No caller may pick a
/// replaceable timestamp itself; route through this.
///
/// Best-effort on the read: if the live head cannot be queried (relay
/// unreachable — the subsequent publish will fail loudly anyway), falls back
/// to now with a warning.
pub fn replaceable_created_at(
    client: &reqwest::blocking::Client,
    http_base: &str,
    signer: &Keys,
    auth_tag_json: Option<&str>,
    kind: u16,
) -> nostr::Timestamp {
    let live = query_events(
        client,
        http_base,
        signer,
        auth_tag_json,
        &serde_json::json!({
            "kinds": [kind],
            "authors": [signer.public_key().to_hex()],
            "limit": 1,
        }),
    )
    .map_err(|e| {
        eprintln!(
            "warning: cannot read live kind:{kind} head for the timestamp bump ({e}); using now"
        );
    })
    .ok()
    .and_then(|events| {
        events
            .first()
            .and_then(|e| e.get("created_at"))
            .and_then(serde_json::Value::as_u64)
    });
    nostr::Timestamp::from(
        nostr::Timestamp::now()
            .as_secs()
            .max(live.map_or(0, |t| t + 1)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_hint_parses_and_caps() {
        assert_eq!(
            parse_retry_hint_secs("rate limited: quota exceeded; retry in 4s"),
            Some(4)
        );
        assert_eq!(parse_retry_hint_secs("retry in 4000s"), Some(30), "capped");
        assert_eq!(parse_retry_hint_secs("no hint here"), None);
        assert_eq!(parse_retry_hint_secs("retry in soon"), None);
    }
}
