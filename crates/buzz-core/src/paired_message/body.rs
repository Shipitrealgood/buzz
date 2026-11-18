//! Operation-specific plaintext schemas. Serde rejects extra or duplicate members.
use super::{canonical_id, create::valid_key, Error, Policy, Result};
use serde::{Deserialize, Serialize};

/// Peer-encrypted message fields; IDs and nonces are lowercase hex.
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Body {
    InventoryQuery {
        pair_ack_id: String,
        nonce: String,
    },
    Inventory {
        query_id: String,
        nonce: String,
        sequence: u64,
        label: String,
        runtimes: Vec<String>,
        capacity: u32,
        catalog_revision: String,
    },
    Create {
        request_id: String,
        catalog_id: String,
        runtime: String,
        name: String,
        #[serde(deserialize_with = "object")]
        policy: Policy,
        channels: Vec<String>,
    },
    Challenge {
        request_id: String,
        intent_id: String,
        phase: String,
        nonce: String,
    },
    Confirm {
        request_id: String,
        intent_id: String,
        challenge_id: String,
        phase: String,
        nonce: String,
    },
    KeyOffer {
        request_id: String,
        intent_id: String,
        confirm_id: String,
        agent: String,
        nonce: String,
        agent_sig: String,
    },
    Activate {
        request_id: String,
        intent_id: String,
        offer_id: String,
        agent: String,
        nonce: String,
    },
    Committed {
        request_id: String,
        intent_id: String,
        activate_id: String,
        offer_id: String,
        agent: String,
    },
    Credentials {
        request_id: String,
        intent_id: String,
        commit_id: String,
        agent: String,
        oa: [String; 4],
        #[serde(deserialize_with = "object")]
        registration: nostr::Event,
    },
    Result {
        request_id: String,
        intent_id: String,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        commit_id: Option<String>,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        agent: Option<String>,
        state: String,
        steps: Vec<String>,
    },
    StatusQuery {
        request_id: String,
        intent_id: String,
        nonce: String,
    },
    Status {
        request_id: String,
        intent_id: String,
        query_id: String,
        nonce: String,
        state: String,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        commit_id: Option<String>,
        #[serde(deserialize_with = "Deserialize::deserialize")]
        terminal_id: Option<String>,
    },
    Unpair {
        pair_ack_id: String,
        revocation_id: String,
    },
    Unpaired {
        pair_ack_id: String,
        tombstone_nonce: String,
    },
}

impl Body {
    /// Exact signed public operation tag.
    pub fn operation(&self) -> &'static str {
        match self {
            Self::InventoryQuery { .. } => "inventory-query",
            Self::Inventory { .. } => "inventory",
            Self::Create { .. } => "create",
            Self::Challenge { .. } => "challenge",
            Self::Confirm { .. } => "confirm",
            Self::KeyOffer { .. } => "key-offer",
            Self::Activate { .. } => "activate",
            Self::Committed { .. } => "committed",
            Self::Credentials { .. } => "credentials",
            Self::Result { .. } => "result",
            Self::StatusQuery { .. } => "status-query",
            Self::Status { .. } => "status",
            Self::Unpair { .. } => "unpair",
            Self::Unpaired { .. } => "unpaired",
        }
    }

    pub(super) fn from_owner(&self) -> bool {
        matches!(
            self,
            Self::InventoryQuery { .. }
                | Self::Unpair { .. }
                | Self::Create { .. }
                | Self::Confirm { .. }
                | Self::Activate { .. }
                | Self::Credentials { .. }
                | Self::StatusQuery { .. }
        )
    }

    pub(super) fn references(&self) -> (Option<&str>, &str) {
        let prev = match self {
            Self::Create {
                request_id,
                catalog_id,
                ..
            } => return (Some(request_id), catalog_id),
            Self::Challenge {
                request_id,
                intent_id,
                ..
            }
            | Self::StatusQuery {
                request_id,
                intent_id,
                ..
            } => return (Some(request_id), intent_id),
            Self::Confirm {
                request_id,
                challenge_id,
                ..
            } => return (Some(request_id), challenge_id),
            Self::KeyOffer {
                request_id,
                confirm_id,
                ..
            } => return (Some(request_id), confirm_id),
            Self::Activate {
                request_id,
                offer_id,
                ..
            } => return (Some(request_id), offer_id),
            Self::Committed {
                request_id,
                activate_id,
                ..
            } => return (Some(request_id), activate_id),
            Self::Credentials {
                request_id,
                commit_id,
                ..
            } => return (Some(request_id), commit_id),
            Self::Result {
                request_id,
                intent_id,
                commit_id,
                ..
            } => return (Some(request_id), commit_id.as_ref().unwrap_or(intent_id)),
            Self::Status {
                request_id,
                query_id,
                ..
            } => return (Some(request_id), query_id),
            Self::InventoryQuery { pair_ack_id, .. }
            | Self::Unpair { pair_ack_id, .. }
            | Self::Unpaired { pair_ack_id, .. } => pair_ack_id,
            Self::Inventory { query_id, .. } => query_id,
        };
        (None, prev)
    }

    pub(super) fn ttl(&self) -> u64 {
        if matches!(self, Self::Create { .. }) {
            900
        } else if matches!(
            self,
            Self::Unpair { .. }
                | Self::Unpaired { .. }
                | Self::Committed { .. }
                | Self::Credentials { .. }
                | Self::Result { .. }
        ) {
            86400
        } else {
            60
        }
    }

    pub(super) fn validate(&self) -> Result<()> {
        let (r, parent) = self.references();
        if r.is_some_and(|id| !canonical_id(id)) || !canonical_id(parent) {
            return Err(Error);
        }
        let is_state = |s: &str| {
            matches!(
                s,
                "pending"
                    | "committed"
                    | "ready"
                    | "failed"
                    | "expired"
                    | "revoked-before-commit"
                    | "unknown"
            )
        };
        let nonce = match self {
            Self::Create {
                runtime,
                name,
                channels,
                policy,
                ..
            } => {
                if !runtime_id(runtime)
                    || !display_name(name)
                    || !policy.valid()
                    || channels.len() > 8
                    || (!channels.is_empty() && policy.channel_add_policy == "nobody")
                    || channels.windows(2).any(|w| w[0] >= w[1])
                    || !channels
                        .iter()
                        .all(|s| uuid::Uuid::parse_str(s).is_ok_and(|v| v.to_string() == *s))
                {
                    return Err(Error);
                }
                return Ok(());
            }
            Self::Challenge {
                phase,
                nonce,
                intent_id,
                ..
            }
            | Self::Confirm {
                phase,
                nonce,
                intent_id,
                ..
            } => {
                if phase != "generate" || !canonical_id(intent_id) {
                    return Err(Error);
                }
                nonce
            }
            Self::Activate {
                intent_id,
                agent,
                nonce,
                ..
            }
            | Self::KeyOffer {
                intent_id,
                agent,
                nonce,
                ..
            } => {
                if !valid_key(agent) || !canonical_id(intent_id) {
                    return Err(Error);
                }
                nonce
            }
            Self::Committed {
                agent,
                intent_id,
                offer_id,
                ..
            } => {
                if !valid_key(agent) || !canonical_id(intent_id) || !canonical_id(offer_id) {
                    return Err(Error);
                }
                return Ok(());
            }
            Self::Credentials {
                intent_id,
                agent,
                oa,
                ..
            } => {
                if !canonical_id(intent_id)
                    || !valid_key(agent)
                    || oa[0] != "auth"
                    || !valid_key(&oa[1])
                    || !oa[2].is_empty()
                    || oa[3].len() != 128
                    || !oa[3]
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err(Error);
                }
                return Ok(());
            }
            Self::Result {
                intent_id,
                agent,
                state,
                steps,
                ..
            } => {
                if !canonical_id(intent_id)
                    || agent.as_ref().is_some_and(|p| !valid_key(p))
                    || !matches!(
                        state.as_str(),
                        "ready" | "failed" | "expired" | "revoked-before-commit"
                    )
                    || steps.is_empty()
                    || steps.len() > 24
                    || !steps.iter().all(|c| {
                        matches!(
                            c.as_str(),
                            "revoked"
                                | "expired"
                                | "catalog-changed"
                                | "capacity-or-name"
                                | "verified"
                                | "profile-verified"
                                | "directory-verified"
                                | "registration-verified"
                                | "unit-active"
                                | "presence-confirmed"
                                | "profile-conflict"
                                | "admission-denied"
                                | "membership-denied"
                                | "unit-failed"
                        ) || c
                            .strip_prefix("membership-verified:")
                            .or_else(|| c.strip_prefix("membership-denied:"))
                            .is_some_and(|s| {
                                uuid::Uuid::parse_str(s).is_ok_and(|v| v.to_string() == s)
                            })
                    })
                {
                    return Err(Error);
                }
                return Ok(());
            }
            Self::StatusQuery { nonce, .. } => nonce,
            Self::Status {
                intent_id,
                nonce,
                state,
                commit_id,
                terminal_id,
                ..
            } => {
                if !canonical_id(intent_id)
                    || !is_state(state)
                    || [commit_id, terminal_id]
                        .into_iter()
                        .flatten()
                        .any(|id| !canonical_id(id))
                {
                    return Err(Error);
                }
                nonce
            }
            Self::InventoryQuery { nonce, .. } => nonce,
            Self::Inventory {
                nonce,
                sequence,
                label,
                runtimes,
                capacity,
                catalog_revision,
                ..
            } => {
                if *sequence == 0
                    || *capacity == 0
                    || !display_name(label)
                    || runtimes.is_empty()
                    || runtimes.len() > 64
                    || runtimes.windows(2).any(|w| w[0] >= w[1])
                    || !runtimes.iter().all(|s| runtime_id(s))
                    || !canonical_id(catalog_revision)
                {
                    return Err(Error);
                }
                nonce
            }
            Self::Unpair { revocation_id, .. } => revocation_id,
            Self::Unpaired {
                tombstone_nonce, ..
            } => tombstone_nonce,
        };
        if !canonical_id(parent) || !canonical_id(nonce) {
            return Err(Error);
        }
        Ok(())
    }
}

/// Safe catalog token. A runtime token never names a path or executable.
pub fn runtime_id(s: &str) -> bool {
    let Some((plugin, kind)) = s.split_once('/') else {
        return false;
    };
    s.len() <= 96
        && [plugin, kind].iter().all(|part| {
            !part.is_empty()
                && part.as_bytes()[0].is_ascii_alphanumeric()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.".contains(&b))
        })
}

/// Bounded NFC presentation label.
pub fn display_name(s: &str) -> bool {
    !s.is_empty()
        && s.trim() == s
        && s.chars().count() <= 64
        && !s
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
        && unicode_normalization::is_nfc(s)
}

fn object<'de, D, T>(d: D) -> std::result::Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Object<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Object<T> {
        type Value = T;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a JSON object")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            map: M,
        ) -> std::result::Result<T, M::Error> {
            T::deserialize(serde::de::value::MapAccessDeserializer::new(map))
        }
    }
    d.deserialize_map(Object(std::marker::PhantomData))
}
