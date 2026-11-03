//! Private paired-host staging transport, isolated from the Nostr event store.
use crate::state::AppState;
use axum::{
    body::{to_bytes, Body},
    extract::{ConnectInfo, OriginalUri, State},
    http::{header, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use buzz_core::{
    paired_host::{transport::verify_outer, MAX_EVENT_BYTES},
    TenantContext,
};
use buzz_db::paired_hosts::{Fields, Packet, Read};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use serde_json::{json, value::RawValue, Value};
use sha2::Sha256;
use std::{net::SocketAddr, sync::Arc};

const PREFIX: &str = "/api/paired-hosts/v1";
type Error = (StatusCode, Json<Value>);
fn error(code: StatusCode) -> Error {
    (code, Json(json!({"code":"paired_request_denied"})))
}
fn denied() -> Error {
    error(StatusCode::NOT_FOUND)
}
fn unauthorized() -> Error {
    error(StatusCode::UNAUTHORIZED)
}
fn invalid() -> Error {
    error(StatusCode::BAD_REQUEST)
}

pub(crate) fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest(
            PREFIX,
            Router::new()
                .route("/offers/{g}", get(handle).post(handle))
                .route("/offers/{g}/ack", axum::routing::post(handle))
                .route("/hosts", get(handle))
                .route("/bindings/{g}", get(handle))
                .route("/bindings/{g}/requests", get(handle))
                .route("/bindings/{g}/requests/{r}", get(handle))
                .route("/bindings/{g}/messages", get(handle).post(handle)),
        )
        .with_state(state)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BodyPacket<'a> {
    #[serde(borrow)]
    event: &'a RawValue,
    #[serde(borrow)]
    route: &'a RawValue,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Route<'a> {
    #[serde(borrow)]
    r: &'a RawValue,
    #[serde(borrow)]
    prev: &'a RawValue,
    expires_at: i64,
}
fn id(s: &str) -> Result<[u8; 32], Error> {
    if s.len() != 64
        || !s
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(invalid());
    }
    let mut bytes = [0; 32];
    hex::decode_to_slice(s, &mut bytes).map_err(|_| invalid())?;
    Ok(bytes)
}
fn optional_id(raw: &RawValue) -> Result<Option<[u8; 32]>, Error> {
    if raw.get() == "null" {
        return Ok(None);
    }
    let value: &str = serde_json::from_str(raw.get()).map_err(|_| invalid())?;
    id(value).map(Some)
}
fn parse(
    bytes: &[u8],
    community: &str,
    g: &str,
    actor: nostr::PublicKey,
    endpoint: &str,
) -> Result<Packet, Error> {
    if bytes.iter().find(|b| !b.is_ascii_whitespace()) != Some(&b'{') {
        return Err(invalid());
    }
    let packet: BodyPacket = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if !packet.route.get().starts_with('{') {
        return Err(invalid());
    }
    let route: Route = serde_json::from_str(packet.route.get()).map_err(|_| invalid())?;
    let (event, peer, op) =
        verify_outer(packet.event.get().as_bytes(), community, g, actor).map_err(|_| invalid())?;
    let owner_sends = match (endpoint, op.as_str()) {
        ("offers", "pair-claim")
        | (
            "messages",
            "unpair" | "inventory-query" | "create" | "confirm" | "activate" | "credentials"
            | "status-query",
        ) => true,
        ("ack", "pair-ack")
        | (
            "messages",
            "unpaired" | "inventory" | "challenge" | "key-offer" | "committed" | "result"
            | "status",
        ) => false,
        _ => return Err(invalid()),
    };
    let (owner, host) = if owner_sends {
        (actor, peer)
    } else {
        (peer, actor)
    };
    Ok(Packet {
        id: event.id.to_bytes(),
        generation: id(g)?,
        owner: owner.to_bytes(),
        host: host.to_bytes(),
        op,
        request: optional_id(route.r)?,
        prev: optional_id(route.prev)?,
        created: event
            .created_at
            .as_secs()
            .try_into()
            .map_err(|_| invalid())?,
        expiry: route.expires_at,
        event: packet.event.to_owned(),
        route: serde_json::from_str(packet.route.get()).map_err(|_| invalid())?,
    })
}

fn mac(state: &AppState, scope: &[&str]) -> Result<Hmac<Sha256>, Error> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(state.relay_keypair.secret_key().as_secret_bytes())
            .map_err(|_| invalid())?;
    mac.update(b"buzz-paired-host-v1\0");
    for part in scope {
        mac.update(part.as_bytes());
        mac.update(&[0]);
    }
    Ok(mac)
}
// A cursor must not disclose cross-tenant traffic via the global sequence.
// Derive a separate key for the resolved community, principal and view.
fn cursor_key(state: &AppState, scope: &[&str]) -> Result<ring::aead::LessSafeKey, Error> {
    let mut key = mac(state, scope)?;
    key.update(b"private-cursor");
    let key =
        ring::aead::UnboundKey::new(&ring::aead::CHACHA20_POLY1305, &key.finalize().into_bytes())
            .map_err(|_| invalid())?;
    Ok(ring::aead::LessSafeKey::new(key))
}
fn seal(state: &AppState, scope: &[&str], data: &str) -> Result<String, Error> {
    use ring::{
        aead::{Aad, Nonce},
        rand::{SecureRandom, SystemRandom},
    };
    let mut nonce = [0; 12];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| invalid())?;
    // Fixed-size encrypted positions conceal both the sequence and its width.
    let mut plain = vec![0; 65];
    if data.len() > 64 {
        return Err(invalid());
    }
    plain[0] = data.len() as u8;
    plain[1..=data.len()].copy_from_slice(data.as_bytes());
    cursor_key(state, scope)?
        .seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::empty(),
            &mut plain,
        )
        .map_err(|_| invalid())?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([&nonce[..], &plain].concat()))
}
fn open(state: &AppState, scope: &[&str], cursor: &str) -> Result<String, Error> {
    use ring::aead::{Aad, Nonce};
    let mut buf = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| denied())?;
    if buf.len() != 12 + 65 + 16 {
        return Err(denied());
    }
    let (nonce, body) = buf.split_at_mut(12);
    let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| denied())?;
    let plain = cursor_key(state, scope)?
        .open_in_place(nonce, Aad::empty(), body)
        .map_err(|_| denied())?;
    let data = plain.get(1..=usize::from(plain[0])).ok_or_else(denied)?;
    String::from_utf8(data.to_vec()).map_err(|_| denied())
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Query {
    include: Option<String>,
    after: Option<String>,
}

async fn handle(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    req: Request<Body>,
) -> Response {
    let mut response = process(&state, uri, req).await.into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}
async fn process(
    state: &AppState,
    uri: axum::http::Uri,
    req: Request<Body>,
) -> Result<Json<Fields>, Error> {
    let (parts, body) = req.into_parts();
    let post = parts.method == axum::http::Method::POST;
    let headers = &parts.headers;
    if headers.get_all(header::HOST).iter().count() != 1
        || headers.get_all(header::AUTHORIZATION).iter().count() != 1
    {
        return Err(unauthorized());
    }
    let tenant = crate::nip_fi_shadow::bind_tenant(state, headers)
        .await
        .ok_or_else(denied)?;
    if headers.get(header::HOST).and_then(|v| v.to_str().ok()) != Some(tenant.host()) {
        return Err(unauthorized());
    }
    let path = uri.path_and_query().ok_or_else(invalid)?.as_str();
    if path.len() > 1024 {
        return Err(invalid());
    }
    let url = super::bridge::nip98_expected_url(&state.config.relay_url, &tenant, path);
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Nostr "))
        .filter(|s| s.len() <= 8192)
        .ok_or_else(unauthorized)?;
    let auth = STANDARD.decode(auth).map_err(|_| unauthorized())?;
    let bytes = to_bytes(body, MAX_EVENT_BYTES)
        .await
        .map_err(|_| error(StatusCode::PAYLOAD_TOO_LARGE))?;
    if !post && !bytes.is_empty() {
        return Err(invalid());
    }
    // Count authenticated-door attempts separately from outstanding business
    // events. In particular an arbitrary signer cannot fill Redis replay keys
    // or acquire the DB writer at an unbounded rate. The socket peer is the
    // only IP source; forwarded headers are never trusted here.
    let ip = parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .ok_or_else(denied)?
        .0
        .ip();
    let t = tenant.community();
    let bucket = mac(state, &["IP", &t.to_string(), &ip.to_string()])?
        .finalize()
        .into_bytes();
    ingress_budget(state, &tenant, &bucket).await?;
    let (actor, event_id) = buzz_auth::nip98::verify_exact_request(
        std::str::from_utf8(&auth).map_err(|_| unauthorized())?,
        &url,
        parts.method.as_str(),
        post.then_some(&bytes[..]),
    )
    .map_err(|_| unauthorized())?;
    // The inclusive ±60s signature window needs more than 120s of real retention.
    if !state
        .nip98_replay
        .try_mark(&tenant, &event_id, 125)
        .await
        .map_err(|_| error(StatusCode::SERVICE_UNAVAILABLE))?
    {
        return Err(unauthorized());
    }
    let tail = uri.path().strip_prefix(PREFIX).ok_or_else(invalid)?;
    let segments: Vec<_> = tail.trim_start_matches('/').split('/').collect();
    let allow = state.config.pubkey_allowlist_enabled;
    if post {
        if uri.query().is_some() {
            return Err(invalid());
        }
        let (g, endpoint) = match segments.as_slice() {
            ["offers", g] => (*g, "offers"),
            ["offers", g, "ack"] => (*g, "ack"),
            ["bindings", g, "messages"] => (*g, "messages"),
            _ => return Err(invalid()),
        };
        let origin = format!("https://{}", tenant.host());
        let p = parse(&bytes, &origin, g, actor, endpoint)?;
        state
            .db
            .post_paired_packet(t, allow, &p, &bucket.into())
            .await
            .map_err(db_error)?;
        return Ok(Json(Fields::new()));
    }
    let q: Query = serde_urlencoded::from_str(uri.query().unwrap_or("")).map_err(|_| invalid())?;
    read_page(state, &tenant, actor, &segments, q).await
}
async fn ingress_budget(
    state: &AppState,
    tenant: &TenantContext,
    bucket: &[u8],
) -> Result<(), Error> {
    let mut conn = state
        .redis_pool
        .get()
        .await
        .map_err(|_| error(StatusCode::SERVICE_UNAVAILABLE))?;
    let key = format!(
        "buzz:{}:paired-host-ingress:{}",
        tenant.community(),
        hex::encode(bucket)
    );
    let count:i64=redis::cmd("EVAL").arg("local n=redis.call('INCR',KEYS[1]); if n==1 then redis.call('EXPIRE',KEYS[1],60) end; return n").arg(1).arg(key)
        .query_async(&mut *conn).await.map_err(|_|error(StatusCode::SERVICE_UNAVAILABLE))?;
    if count > 600 {
        return Err(error(StatusCode::TOO_MANY_REQUESTS));
    }
    Ok(())
}
fn db_error(e: buzz_db::DbError) -> Error {
    match e {
        buzz_db::DbError::AccessDenied(_) => denied(),
        buzz_db::DbError::PairedCursorPruned => {
            (StatusCode::GONE, Json(json!({"code":"cursor_pruned"})))
        }
        _ => error(StatusCode::SERVICE_UNAVAILABLE),
    }
}
async fn read_page(
    state: &AppState,
    tenant: &TenantContext,
    actor: nostr::PublicKey,
    path: &[&str],
    q: Query,
) -> Result<Json<Fields>, Error> {
    let t = tenant.community().to_string();
    let key = actor.to_hex();
    let kind = match path {
        ["hosts"] if q.include.is_none() => "hosts",
        ["hosts"] if q.include.as_deref() == Some("history") => "history",
        ["bindings", _] | ["bindings", _, "messages"] if q.include.is_none() => "messages",
        ["offers", _] if q.include.is_none() => "offers",
        ["bindings", _, "requests"] | ["bindings", _, "requests", _] if q.include.is_none() => {
            "requests"
        }
        _ => return Err(invalid()),
    };
    let scope = [&t[..], &key, kind, path.get(1).copied().unwrap_or("")];
    let after = q
        .after
        .as_deref()
        .map(|c| open(state, &scope, c))
        .transpose()?;
    let after = after.as_deref();
    let target = match path {
        ["hosts"] => Read::Hosts {
            history: kind == "history",
            after: after.map(id).transpose()?,
        },
        ["offers", g] if after.is_none() => Read::Offers(id(g)?),
        ["bindings", g, "requests"] => Read::Requests(id(g)?, after.map(id).transpose()?),
        ["bindings", g, "requests", r] if after.is_none() => Read::Request(id(g)?, id(r)?),
        ["bindings", g] if after.is_none() => Read::Binding(id(g)?),
        ["bindings", g, "messages"] => {
            Read::Messages(id(g)?, after.unwrap_or("0").parse().map_err(|_| invalid())?)
        }
        _ => return Err(invalid()),
    };
    let page = state
        .db
        .read_paired(
            tenant.community(),
            actor.as_bytes(),
            state.config.pubkey_allowlist_enabled,
            target,
        )
        .await
        .map_err(db_error)?;
    let mut data = page.data;
    if let Some(seq) = page.seq {
        let k = if path.len() == 2 {
            "high_water"
        } else {
            "after"
        };
        data.insert(
            k.into(),
            serde_json::value::to_raw_value(&seal(state, &scope, &seq.to_string())?)
                .map_err(|_| invalid())?,
        );
    }
    if kind == "hosts" || kind == "history" || (kind == "requests" && path.len() == 3) {
        data.insert(
            "after".into(),
            serde_json::value::to_raw_value(
                &page
                    .generation
                    .map(|g| seal(state, &scope, &hex::encode(g)))
                    .transpose()?,
            )
            .map_err(|_| invalid())?,
        );
    }
    Ok(Json(data))
}

#[cfg(test)]
mod postgres_tests;

#[cfg(test)]
mod phase_postgres_tests;
