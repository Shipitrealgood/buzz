//! Immutable create inputs and the candidate agent's possession signature.
use super::{canonical_id, Body, Error, Result, Scope};
use nostr::secp256k1::{schnorr::Signature, Message as DigestMessage};
use nostr::{Keys, PublicKey, SECP256K1};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Owner-selected response and membership policies. These are independent.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub respond_to: String,
    pub respond_to_allowlist: Vec<String>,
    pub channel_add_policy: String,
}

impl Policy {
    /// Whether response and channel-add policies form a supported v1 selection.
    pub fn valid(&self) -> bool {
        matches!(
            self.respond_to.as_str(),
            "owner-only" | "anyone" | "allowlist"
        ) && matches!(
            self.channel_add_policy.as_str(),
            "owner_only" | "anyone" | "nobody"
        ) && (self.respond_to == "allowlist") != self.respond_to_allowlist.is_empty()
            && self.respond_to_allowlist.len() <= 64
            && self.respond_to_allowlist.windows(2).all(|w| w[0] < w[1])
            && self.respond_to_allowlist.iter().all(|v| valid_key(v))
    }
}

pub(super) fn valid_key(s: &str) -> bool {
    canonical_id(s) && PublicKey::from_hex(s).is_ok_and(|p| p.xonly().is_ok())
}

/// Detached proof input. Canonical binary encoding is independent of JSON serialization.
#[allow(missing_docs)]
pub struct Possession<'a> {
    pub request_id: &'a str,
    pub intent_id: &'a str,
    pub confirm_id: &'a str,
    pub agent: PublicKey,
    pub nonce: &'a str,
    pub expires_at: u64,
}

fn digest(scope: &Scope, input: &Possession<'_>) -> Result<DigestMessage> {
    super::peers(scope, true)?;
    if input.agent == scope.host || input.agent == scope.owner || input.agent.xonly().is_err() {
        return Err(Error);
    }
    let mut hash = Sha256::new();
    hash.update(b"buzz:paired-host:v1:agent-pop\0");
    hash.update((scope.community.len() as u16).to_be_bytes());
    hash.update(scope.community.as_bytes());
    for s in [
        &scope.generation,
        &scope.owner.to_hex(),
        &scope.host.to_hex(),
        input.request_id,
        input.intent_id,
        input.confirm_id,
        &input.agent.to_hex(),
        input.nonce,
    ] {
        if !canonical_id(s) {
            return Err(Error);
        }
        hash.update(hex::decode(s).map_err(|_| Error)?);
    }
    hash.update(input.expires_at.to_be_bytes());
    Ok(DigestMessage::from_digest(hash.finalize().into()))
}

/// Sign only the v1 possession transcript for the candidate's own public key.
pub fn sign_possession(scope: &Scope, input: &Possession<'_>, agent: &Keys) -> Result<String> {
    if agent.public_key() != input.agent {
        return Err(Error);
    }
    Ok(agent.sign_schnorr(&digest(scope, input)?).to_string())
}

/// Validate the detached candidate signature under the exact offer's authenticated fields.
pub fn verify_possession(scope: &Scope, input: &Possession<'_>, sig: &str) -> Result<()> {
    if sig.len() != 128
        || !sig
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error);
    }
    let sig: Signature = sig.parse().map_err(|_| Error)?;
    SECP256K1
        .verify_schnorr(
            &sig,
            &digest(scope, input)?,
            &input.agent.xonly().map_err(|_| Error)?,
        )
        .map_err(|_| Error)
}

pub(super) fn verify_offer(scope: &Scope, body: &Body, expires_at: u64) -> Result<()> {
    if let Body::KeyOffer {
        request_id,
        intent_id,
        confirm_id,
        agent,
        nonce,
        agent_sig,
    } = body
    {
        let input = Possession {
            request_id,
            intent_id,
            confirm_id,
            agent: PublicKey::from_hex(agent).map_err(|_| Error)?,
            nonce,
            expires_at,
        };
        verify_possession(scope, &input, agent_sig)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidate_proves_current_offer_and_frozen_scope() {
        let owner = Keys::generate();
        let host = Keys::generate();
        let agent = Keys::generate();
        let scope = Scope {
            community: "https://private.example".into(),
            generation: "ab".repeat(32),
            owner: owner.public_key(),
            host: host.public_key(),
        };
        let (r, i, c, n) = (
            "10".repeat(32),
            "12".repeat(32),
            "22".repeat(32),
            "32".repeat(32),
        );
        let mut proof = Possession {
            request_id: &r,
            intent_id: &i,
            confirm_id: &c,
            agent: agent.public_key(),
            nonce: &n,
            expires_at: 1060,
        };
        let sig = sign_possession(&scope, &proof, &agent).unwrap();
        verify_possession(&scope, &proof, &sig).unwrap();
        proof.expires_at += 1;
        assert!(verify_possession(&scope, &proof, &sig).is_err());
        let offer = Body::KeyOffer {
            request_id: r,
            intent_id: i,
            confirm_id: c,
            agent: agent.public_key().to_hex(),
            nonce: n,
            agent_sig: sig,
        };
        let packet = crate::paired_message::seal(&host, &scope, offer.clone(), 1000, 1060).unwrap();
        assert_eq!(
            crate::paired_message::open(&owner, &scope, &packet, 1001)
                .unwrap()
                .body,
            offer
        );
        assert!(crate::paired_message::seal(&host, &scope, offer, 1001, 1061).is_err());
    }
}
