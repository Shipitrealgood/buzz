//! Restrictive continuation of an admitted placement, independent of runtimes.
use super::*;
use buzz_core::host_deployment::{Action, Outcome};

/// Fresh reconciliation input, consumed under the native store lock. Private
/// fields prevent callers from manufacturing placement or renewal authority.
pub struct CleanupPlan {
    root: Authority,
    predecessor: String,
    unit: String,
    revision: u64,
    known: Vec<String>,
    reuse: Option<String>,
}

impl Device {
    /// A validated retained Cleanup permanently prevents this agent's Deploy.
    pub fn deployment_open(&self, selected: &str, keys: &Keys, a: &Authority) -> Result<()> {
        ensure!(
            !self.deployment_intents(selected, keys)?.iter().any(|i| {
                let other = &i.protected.authority;
                other.scope == a.scope && other.agent == a.agent && other.action == Action::Cleanup
            }),
            "Cleanup retained; this agent cannot be deployed again"
        );
        Ok(())
    }

    /// Restore display/recovery inputs from the bound original Deploy, never a
    /// renderer-selected record. This does not authorize rematerialization.
    pub fn cleanup_predecessor(
        &self,
        selected: &str,
        keys: &Keys,
        intent: &Intent,
    ) -> Result<Intent> {
        let data = self.restore_materialization(selected, keys, intent)?;
        ensure!(
            intent.protected.authority.action == Action::Cleanup,
            "not Cleanup"
        );
        let prior = self
            .deployment_intent(
                selected,
                keys,
                data["cleanup_predecessor"]
                    .as_str()
                    .context("missing Cleanup predecessor")?,
            )?
            .context("missing Cleanup predecessor")?;
        self.restore_materialization(selected, keys, &prior)?;
        ensure!(
            prior.protected.authority.action == Action::Deploy
                && prior.protected.authority.scope == intent.protected.authority.scope
                && prior.protected.authority.agent == intent.protected.authority.agent,
            "Cleanup predecessor changed"
        );
        Ok(prior)
    }

    /// Atomically retain one Cleanup winner. Caller holds its local mutation
    /// lock so recovery cannot reinsert a record across this permanent fence.
    pub fn retain_cleanup(
        &mut self,
        selected: &str,
        keys: &Keys,
        plan: CleanupPlan,
    ) -> Result<Intent> {
        self.deployment_schema()?;
        session(&plan.root.scope, selected, keys)?;
        let wall = now()?;
        self.check_time(wall)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let rows = {
            let mut stmt = tx.prepare(
                "SELECT data FROM deployment_intents WHERE community=? AND owner=? LIMIT 1025",
            )?;
            let rows = stmt
                .query_map(params![selected, keys.public_key().to_hex()], |r| {
                    r.get::<_, String>(0)
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        };
        let mut winner: Option<Intent> = None;
        for raw in &rows {
            let other: Intent = serde_json::from_str(raw)?;
            validate_materialization(selected, keys, &other)?;
            let a = &other.protected.authority;
            if a.scope != plan.root.scope || a.agent != plan.root.agent {
                continue;
            }
            if a.action == Action::Cleanup
                && (plan.reuse.as_deref() == Some(&a.operation_id)
                    || !plan.known.contains(&a.operation_id))
                && a.expected_revision >= plan.revision
            {
                ensure!(winner.is_none(), "competing Cleanup intents");
                winner = Some(other);
                continue;
            }
            ensure!(
                plan.known.contains(&a.operation_id),
                "journal changed; reconcile Cleanup again"
            );
        }
        if let Some(winner) = winner {
            return Ok(winner);
        }
        ensure!(rows.len() < 1024, "deployment intent capacity reached");
        let authority = Authority {
            action: Action::Cleanup,
            operation_id: hex::encode(rand::random::<[u8; 32]>()),
            expected_revision: plan.revision,
            expires_at: wall + 600,
            content_sha256: String::new(),
            ..plan.root
        };
        let (authority, event) = wire::seal_command(keys, authority, &Command::Cleanup {}, wall)?;
        let plain = Zeroizing::new(serde_json::to_vec(&Snapshot {
            authority: authority.clone(),
            data: serde_json::json!({"cleanup_predecessor":plan.predecessor,"cleanup_unit":plan.unit}),
        })?);
        let intent = Intent {
            protected: ProtectedCreateIntent {
                authority,
                ciphertext: event.content.as_bytes().to_vec(),
            },
            event: serde_json::to_string(&event)?,
            materialization: nip44::encrypt(
                keys.secret_key(),
                &keys.public_key(),
                &*plain,
                Version::V2,
            )?,
            // Cleanup has no creation/publication effects: retained = dispatch-ready.
            phase: 2,
            result: None,
        };
        tx.execute(
            "INSERT INTO deployment_intents VALUES(?,?,?,?)",
            params![
                selected,
                keys.public_key().to_hex(),
                intent.protected.authority.operation_id,
                serde_json::to_string(&intent)?
            ],
        )?;
        tx.commit()?;
        Ok(intent)
    }
}

impl Management {
    /// Reconcile every same-agent candidate before creating or explicitly renewing
    /// Cleanup. No catalog, model observation or materialization is consulted.
    pub async fn plan_cleanup(
        &mut self,
        device: &mut Device,
        signer: &impl Fn() -> Result<Keys>,
        operation: &str,
        renew: bool,
    ) -> Result<CleanupPlan> {
        let keys = signer()?;
        let root = device
            .deployment_intent(&self.community, &keys, operation)?
            .context("missing intent")?;
        device.restore_materialization(&self.community, &keys, &root)?;
        let a = &root.protected.authority;
        let intents: Vec<_> = device
            .deployment_intents(&self.community, &keys)?
            .into_iter()
            .filter(|i| {
                i.protected.authority.scope == a.scope && i.protected.authority.agent == a.agent
            })
            .collect();
        let known = intents
            .iter()
            .map(|i| i.protected.authority.operation_id.clone())
            .collect();
        let mut current = None;
        let mut cleanup = None;
        let mut missing_admitted = Vec::new();
        for intent in &intents {
            let p = &intent.protected.authority;
            let read = self.operation(device, signer, &p.operation_id).await?;
            if read.current {
                ensure!(current.is_none(), "conflicting current placements");
                current = Some((intent, read));
            } else if !read.admitted {
                if intent.phase >= 3 || intent.result.is_some() {
                    missing_admitted.push(intent);
                }
                if p.action == Action::Deploy && p.expected_revision > 0 {
                    ensure!(
                        p.expires_at <= now()?,
                        "competing setup admission unresolved"
                    );
                }
            }
            if p.action == Action::Cleanup {
                if cleanup.as_ref().is_none_or(|old: &&Intent| {
                    (
                        old.protected.authority.expected_revision,
                        old.protected.authority.expires_at,
                    ) <= (p.expected_revision, p.expires_at)
                }) {
                    cleanup = Some(intent);
                }
            }
        }
        let (prior, read) = current.context(
            "Admission has not been confirmed; check status before deleting this hosted setup",
        )?;
        let revision = read.revision.context("missing placement revision")?;
        let unit = read.unit.context("missing placement unit")?;
        // A retained replacement does not prove admission. Read older Cleanup
        // rows too: one may still be the current failed predecessor. The relay
        // may prune admitted replacements only after a later revision wins.
        for missing in missing_admitted {
            let p = &missing.protected.authority;
            ensure!(
                p.action == Action::Cleanup
                    && p.expected_revision
                        .checked_add(1)
                        .is_some_and(|r| r < revision),
                "operation admission unresolved"
            );
        }
        let reuse = None;
        if let Some(retained) = cleanup {
            let c = &retained.protected.authority;
            if c.operation_id == prior.protected.authority.operation_id {
                if !(renew && read.state == "failed") {
                    // The transaction returns this exact existing winner.
                    return Ok(CleanupPlan {
                        root: a.clone(),
                        predecessor: String::new(),
                        unit,
                        revision: c.expected_revision,
                        known,
                        reuse: Some(c.operation_id.clone()),
                    });
                }
            } else {
                ensure!(
                    c.expected_revision == revision,
                    "Cleanup placement conflict"
                );
                if c.expires_at > now()? || !renew {
                    return Ok(CleanupPlan {
                        root: a.clone(),
                        predecessor: String::new(),
                        unit,
                        revision: c.expected_revision,
                        known,
                        reuse: Some(c.operation_id.clone()),
                    });
                }
            }
        }
        let predecessor = if prior.protected.authority.action == Action::Cleanup {
            device
                .cleanup_predecessor(&self.community, &signer()?, prior)?
                .protected
                .authority
                .operation_id
        } else {
            prior.protected.authority.operation_id.clone()
        };
        Ok(CleanupPlan {
            root: a.clone(),
            predecessor,
            unit,
            revision,
            known,
            reuse,
        })
    }

    /// Require a fresh permitted read and the exact current host-signed Cleaned
    /// receipt before the native caller performs local deletion.
    pub async fn cleanup_state(
        &mut self,
        device: &mut Device,
        signer: &impl Fn() -> Result<Keys>,
        operation: &str,
    ) -> Result<(String, bool)> {
        let read = self.operation(device, signer, operation).await?;
        let keys = signer()?;
        let intent = device
            .deployment_intent(&self.community, &keys, operation)?
            .context("missing Cleanup")?;
        let a = &intent.protected.authority;
        ensure!(a.action == Action::Cleanup, "not Cleanup");
        let snapshot = device.restore_materialization(&self.community, &keys, &intent)?;
        ensure!(
            !read.admitted
                || (read.current && read.unit.as_deref() == snapshot["cleanup_unit"].as_str()),
            "Cleanup placement conflict"
        );
        let confirmed = read.current
            && read.state == "cleaned"
            && read.body.is_some_and(|b| {
                b.result.request == *a
                    && b.result.outcome == Outcome::Cleaned
                    && b.result.applied_revision == a.expected_revision.checked_add(1)
            });
        Ok((read.state, confirmed))
    }
}

pub(super) fn ensure_deploy_open(
    db: &rusqlite::Connection,
    selected: &str,
    keys: &Keys,
    a: &Authority,
) -> Result<()> {
    let mut stmt =
        db.prepare("SELECT data FROM deployment_intents WHERE community=? AND owner=? LIMIT 1025")?;
    let rows = stmt.query_map(params![selected, keys.public_key().to_hex()], |r| {
        r.get::<_, String>(0)
    })?;
    for raw in rows {
        let intent: Intent = serde_json::from_str(&raw?)?;
        validate_materialization(selected, keys, &intent)?;
        let other = &intent.protected.authority;
        ensure!(
            other.scope != a.scope || other.agent != a.agent || other.action != Action::Cleanup,
            "Cleanup retained; this agent cannot be deployed again"
        );
    }
    Ok(())
}
