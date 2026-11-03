//! Public envelope checks for the private relay transport. A relay cannot
//! decrypt the packet or establish possession of a console invitation.

use super::{canonical_hex, canonical_origin, parse, Error};
use base64::{engine::general_purpose::STANDARD, Engine};
use nostr::{Event, PublicKey};

/// Verify the raw NIP-01 signature and exact public v1 routing tags. The
/// caller supplies the tenant, generation and independently proven HTTP key.
pub fn verify_outer(
    raw: &[u8],
    community: &str,
    generation: &str,
    signer: PublicKey,
) -> Result<(Event, PublicKey, String), Error> {
    if !canonical_origin(community) || !canonical_hex(generation, 64) {
        return Err(Error::Enrollment);
    }
    let event = parse(raw)?;
    let mut tags = event.tags.iter();
    let mut next = |name| {
        let [key, value] = tags.next()?.as_slice() else {
            return None;
        };
        (key == name).then_some(value.as_str())
    };
    let recipient = next("p")
        .filter(|key| canonical_hex(key, 64))
        .ok_or(Error::Message)?;
    let recipient = PublicKey::from_hex(recipient).map_err(|_| Error::Message)?;
    if next("community") != Some(community) || next("g") != Some(generation) {
        return Err(Error::Message);
    }
    let operation = next("op").ok_or(Error::Message)?.to_owned();
    if tags.next().is_some()
        || event.pubkey != signer
        || signer == recipient
        || recipient.xonly().is_err()
    {
        return Err(Error::Message);
    }
    let payload = STANDARD
        .decode(&event.content)
        .map_err(|_| Error::Message)?;
    if payload.len() < 99 || payload.first() != Some(&2) {
        return Err(Error::Message);
    }
    crate::verify_event(&event).map_err(|_| Error::Message)?;
    Ok((event, recipient, operation))
}
