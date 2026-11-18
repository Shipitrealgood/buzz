//! Private cursor authentication and the community-scoped ingress budget.
use crate::state::AppState;
use base64::Engine;
use buzz_core::tenant::TenantContext;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

fn mac(state: &AppState, scope: &[&str]) -> Result<Hmac<Sha256>, ()> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(state.relay_keypair.secret_key().as_secret_bytes())
            .map_err(|_| ())?;
    mac.update(b"buzz-paired-host-v1\0");
    for part in scope {
        mac.update(part.as_bytes());
        mac.update(&[0]);
    }
    Ok(mac)
}
// A cursor must not disclose cross-tenant traffic via the global sequence.
// Derive a separate key for the resolved community, principal and view.
fn cursor_key(state: &AppState, scope: &[&str]) -> Result<ring::aead::LessSafeKey, ()> {
    let mut key = mac(state, scope)?;
    key.update(b"private-cursor");
    let key =
        ring::aead::UnboundKey::new(&ring::aead::CHACHA20_POLY1305, &key.finalize().into_bytes())
            .map_err(|_| ())?;
    Ok(ring::aead::LessSafeKey::new(key))
}
pub(crate) fn seal_cursor(
    state: &AppState,
    scope: &[&str],
    data: &str,
    width: usize,
) -> Result<String, ()> {
    use ring::{
        aead::{Aad, Nonce},
        rand::{SecureRandom, SystemRandom},
    };
    let mut nonce = [0; 12];
    SystemRandom::new().fill(&mut nonce).map_err(|_| ())?;
    // Fixed-size encrypted positions conceal both the sequence and its width.
    let mut plain = vec![0; width + 1];
    if data.len() > width || width > 255 {
        return Err(());
    }
    plain[0] = data.len() as u8;
    plain[1..=data.len()].copy_from_slice(data.as_bytes());
    cursor_key(state, scope)?
        .seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::empty(),
            &mut plain,
        )
        .map_err(|_| ())?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([&nonce[..], &plain].concat()))
}
pub(crate) fn open_cursor(
    state: &AppState,
    scope: &[&str],
    cursor: &str,
    width: usize,
) -> Result<String, ()> {
    use ring::aead::{Aad, Nonce};
    let mut buf = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| ())?;
    if buf.len() != 12 + width + 1 + 16 {
        return Err(());
    }
    let (nonce, body) = buf.split_at_mut(12);
    let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| ())?;
    let plain = cursor_key(state, scope)?
        .open_in_place(nonce, Aad::empty(), body)
        .map_err(|_| ())?;
    let data = plain.get(1..=usize::from(plain[0])).ok_or(())?;
    String::from_utf8(data.to_vec()).map_err(|_| ())
}
pub(crate) async fn source_capacity(
    state: &AppState,
    tenant: &TenantContext,
    bucket: &[u8],
) -> Result<bool, ()> {
    let mut conn = state.redis_pool.clone();
    let key = format!(
        "buzz:{}:paired-host-ingress:{}",
        tenant.community(),
        hex::encode(bucket)
    );
    let count:i64=redis::cmd("EVAL").arg("local n=redis.call('INCR',KEYS[1]); if n==1 then redis.call('EXPIRE',KEYS[1],60) end; return n").arg(1).arg(key)
        .query_async(&mut conn).await.map_err(|_|())?;
    Ok(count <= 600)
}
