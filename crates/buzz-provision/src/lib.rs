//! buzz-provision — the Buzz virtual mint: provision relay-native agents on
//! a self-hosted VM.
//!
//! One crate, two binaries, split by ACTOR:
//!
//! - **`buzz-provision`** (host operator, runs on the VM): `host-init`,
//!   `pin-owner`, `keygen`, `agent`, `retire` — births agent keys into
//!   credential files, provisions systemd-supervised harness instances, and
//!   tears them down.
//! - **`buzz-mint`** (the agent's owner, runs where the owner key lives):
//!   `attest`, `inspect`, `retire` — signs NIP-OA attestations over
//!   host-born pubkeys, publishes the owner-signed membership adds, and
//!   archives retired identities.
//!
//! The ceremony: `buzz-provision keygen` on the host (key born where it
//! lives, never transits) → `buzz-mint attest` on the owner machine
//! (publishes the kind:9000 adds live; emits an all-public manifest bundle)
//! → `buzz-provision agent` on the host (verifies the credential against
//! the manifest, publishes kind:0 + kind:10100, enables the unit).
//!
//! This library holds what both binaries share: the bundle schema
//! ([`bundle`]), the NIP-98 bridge client ([`bridge`]), and the secret
//! handling primitives + sealed-envelope machinery ([`seal`]).

pub mod bridge;
pub mod bundle;
pub mod seal;

pub use seal::{is_canonical_64_hex, redact_key_material, validate_pubkey_arg};

/// Render a pubkey's npub form for operator output, with a printable
/// fallback (bech32 encoding of a valid key cannot realistically fail, but
/// no output path may panic).
pub fn npub_string(pk: &nostr::PublicKey) -> String {
    use nostr::nips::nip19::ToBech32;
    pk.to_bech32()
        .unwrap_or_else(|_| "<bech32 encoding failed>".to_owned())
}

/// Map an error chain to the repo's documented CLI exit-code contract
/// (buzz-cli / AGENTS.md): 0 ok, 1 input, 2 network/relay, 3 auth,
/// 4 other, 5 write conflict (a dominated replaceable write).
pub fn exit_code_for(err: &anyhow::Error) -> i32 {
    for cause in err.chain() {
        if let Some(b) = cause.downcast_ref::<bridge::BridgeError>() {
            return match b {
                bridge::BridgeError::Http {
                    status: 401 | 403, ..
                } => 3,
                bridge::BridgeError::Dominated { .. } => 5,
                bridge::BridgeError::Sign(_) => 4,
                _ => 2,
            };
        }
    }
    1
}
