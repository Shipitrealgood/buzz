//! Exact native create intents in the existing protected device journal.
use super::*;
use anyhow::{bail, Context};
use buzz_core::host_deployment::{
    wire, Authority, Command, DeploymentResult, ProtectedCreateIntent, Status,
};
use nostr::{
    nips::nip44::{self, Version},
    PublicKey,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::Zeroizing;

/// Payload-free category for a failed setup operation read, including local failures.
pub fn operation_error_category(error: &anyhow::Error) -> &'static str {
    use buzz_ws_client::WsClientError;
    match error.downcast_ref::<WsClientError>() {
        Some(WsClientError::UnexpectedMessage(message)) => match message.as_str() {
            "reply-invalid" => "reply-invalid",
            "reply-duplicate" => "reply-duplicate",
            "reply-missing" => "reply-missing",
            "read-closed" => "read-closed",
            _ => "operation-read-failed",
        },
        Some(WsClientError::Timeout) => "read-timeout",
        Some(WsClientError::ConnectionClosed) => "read-closed",
        _ => "operation-read-failed",
    }
}

/// One atomic native intent. The recovery snapshot contains no agent secret;
/// that identity is recovered exclusively from the original command ciphertext.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    /// Immutable accepted contract and exact host ciphertext.
    pub protected: ProtectedCreateIntent,
    /// Exact owner-signed event, reused byte-for-byte after reply loss.
    pub event: String,
    /// Owner-self NIP44 encrypted main record/publication inputs.
    pub materialization: String,
    /// Monotonic materialization/publication/dispatch phase.
    pub phase: u8,
    /// Exact authenticated terminal or retryable host receipt.
    pub result: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    authority: Authority,
    data: Value,
}
impl Device {
    fn deployment_schema(&self) -> Result<()> {
        self.db.execute_batch("CREATE TABLE IF NOT EXISTS deployment_intents(community TEXT NOT NULL,owner TEXT NOT NULL,operation_id TEXT NOT NULL,data TEXT NOT NULL,PRIMARY KEY(community,owner,operation_id))")?;
        Ok(())
    }
    /// Read one scoped durable intent; ciphertext alone never grants authority.
    pub fn deployment_intent(
        &self,
        selected: &str,
        keys: &Keys,
        operation: &str,
    ) -> Result<Option<Intent>> {
        self.deployment_schema()?;
        ensure!(
            buzz_core::host_registry::identifier(operation),
            "invalid operation"
        );
        let raw: Option<String> = self.db.query_row("SELECT data FROM deployment_intents WHERE community=? AND owner=? AND operation_id=?",params![selected,keys.public_key().to_hex(),operation],|r|r.get(0)).optional()?;
        raw.map(|r| serde_json::from_str(&r).map_err(Into::into))
            .transpose()
    }
    /// Seal exactly the candidate supplied by main, after fresh capability checks,
    /// and atomically retain command plus recoverable main inputs before effects.
    pub fn prepare_deployment(
        &mut self,
        selected: &str,
        keys: &Keys,
        authority: Authority,
        command: &Command,
        status: &Status,
        snapshot: Value,
    ) -> Result<Intent> {
        ensure!(
            authority.expected_revision == 0,
            "create requires absent placement"
        );
        self.retain_deployment(selected, keys, authority, command, status, snapshot, None)
    }
    fn retain_deployment(
        &mut self,
        selected: &str,
        keys: &Keys,
        authority: Authority,
        command: &Command,
        status: &Status,
        snapshot: Value,
        previous: Option<(&str, &[String])>,
    ) -> Result<Intent> {
        self.deployment_schema()?;
        session(&authority.scope, selected, keys)?;
        let wall = now()?;
        self.check_time(wall)?;
        let Command::Deploy { config, .. } = command else {
            bail!("create requires deploy");
        };
        status.permits(
            &authority.scope,
            &config.catalog_revision,
            &config.runtime_id,
            wall,
        )?;
        // Main omits the key from this snapshot; it lives only in host ciphertext.
        ensure!(
            snapshot
                .get("record")
                .and_then(|r| r.get("private_key_nsec"))
                .is_none(),
            "recovery snapshot must omit agent key"
        );
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        cleanup::ensure_deploy_open(&tx, selected, keys, &authority)?;
        let raw: Option<String> = tx.query_row(
            "SELECT data FROM deployment_intents WHERE community=? AND owner=? AND operation_id=?",
            params![selected, keys.public_key().to_hex(), authority.operation_id],
            |r| r.get(0),
        ).optional()?;
        if let Some(raw) = raw {
            let old: Intent = serde_json::from_str(&raw)?;
            let old_command =
                wire::restore_command(keys, &old.protected.authority, &old.protected.ciphertext)?;
            ensure!(
                old.protected.authority.scope == authority.scope
                    && old.protected.authority.agent == authority.agent
                    && old.protected.authority.expected_revision == authority.expected_revision
                    && old_command == *command
                    && restore_snapshot(keys, &old)? == snapshot,
                "operation inputs changed"
            );
            return Ok(old);
        }
        if let Some((previous, absent_peers)) = previous {
            let mut stmt = tx.prepare(
                "SELECT data FROM deployment_intents WHERE community=? AND owner=? LIMIT 1025",
            )?;
            let rows = stmt.query_map(params![selected, keys.public_key().to_hex()], |r| {
                r.get::<_, String>(0)
            })?;
            for raw in rows {
                let other: Intent = serde_json::from_str(&raw?)?;
                validate_materialization(selected, keys, &other)?;
                let a = &other.protected.authority;
                ensure!(
                    a.operation_id == previous
                        || a.scope != authority.scope
                        || a.agent != authority.agent
                        || a.expected_revision < authority.expected_revision
                        || (a.expected_revision == authority.expected_revision
                            && a.expires_at <= wall
                            && other.phase < 3
                            && other.result.is_none()
                            && absent_peers.contains(&a.operation_id)),
                    "newer setup intent exists"
                );
            }
        }
        let (authority, event) = wire::seal_command(keys, authority, command, wall)?;
        let plain = Zeroizing::new(serde_json::to_vec(&Snapshot {
            authority: authority.clone(),
            data: snapshot,
        })?);
        ensure!(plain.len() <= 48 * 1024, "materialization inputs too large");
        let materialization =
            nip44::encrypt(keys.secret_key(), &keys.public_key(), &*plain, Version::V2)?;
        let intent = Intent {
            protected: ProtectedCreateIntent {
                authority,
                ciphertext: event.content.as_bytes().to_vec(),
            },
            event: serde_json::to_string(&event)?,
            materialization,
            phase: 0,
            result: None,
        };
        let count: i64 = tx.query_row(
            "SELECT count(*) FROM deployment_intents WHERE community=? AND owner=?",
            params![selected, keys.public_key().to_hex()],
            |r| r.get(0),
        )?;
        ensure!(count < 1024, "deployment intent capacity reached");
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
    /// Restore only with the original signer and exact command binding.
    pub fn restore_materialization(
        &self,
        selected: &str,
        keys: &Keys,
        intent: &Intent,
    ) -> Result<Value> {
        validate_materialization(selected, keys, intent)
    }
    /// Advance only after the caller observed the corresponding main effect.
    pub fn deployment_advance(
        &mut self,
        selected: &str,
        keys: &Keys,
        operation: &str,
        phase: u8,
    ) -> Result<()> {
        ensure!(phase <= 4, "invalid intent phase");
        self.deployment_schema()?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let raw: String = tx.query_row(
            "SELECT data FROM deployment_intents WHERE community=? AND owner=? AND operation_id=?",
            params![selected, keys.public_key().to_hex(), operation],
            |r| r.get(0),
        )?;
        let mut intent: Intent = serde_json::from_str(&raw)?;
        validate_materialization(selected, keys, &intent)?;
        ensure!(
            phase >= intent.phase && phase <= intent.phase + 1,
            "nonmonotonic intent phase"
        );
        if intent.protected.authority.action == buzz_core::host_deployment::Action::Deploy {
            cleanup::ensure_deploy_open(&tx, selected, keys, &intent.protected.authority)?;
        }
        intent.phase = phase;
        tx.execute(
            "UPDATE deployment_intents SET data=? WHERE community=? AND owner=? AND operation_id=?",
            params![
                serde_json::to_string(&intent)?,
                selected,
                keys.public_key().to_hex(),
                operation
            ],
        )?;
        tx.commit()?;
        Ok(())
    }
    /// Retain verified host evidence; pending/error responses never erase recovery.
    pub fn deployment_result(
        &mut self,
        selected: &str,
        keys: &Keys,
        operation: &str,
        event: &Event,
    ) -> Result<DeploymentResult> {
        self.deployment_schema()?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let raw: String = tx.query_row(
            "SELECT data FROM deployment_intents WHERE community=? AND owner=? AND operation_id=?",
            params![selected, keys.public_key().to_hex(), operation],
            |r| r.get(0),
        )?;
        let mut intent: Intent = serde_json::from_str(&raw)?;
        validate_materialization(selected, keys, &intent)?;
        let r = wire::open_result(keys, event, &intent.protected.authority)?;
        if let Some(raw) = &intent.result {
            let old: Event = serde_json::from_str(raw)?;
            let old_result = wire::open_result(keys, &old, &intent.protected.authority)?;
            ensure!(
                event.created_at >= old.created_at,
                "stale deployment result"
            );
            if matches!(
                old_result.outcome,
                buzz_core::host_deployment::Outcome::Applied
                    | buzz_core::host_deployment::Outcome::NeedsLogin
                    | buzz_core::host_deployment::Outcome::Cleaned
            ) {
                ensure!(old.id == event.id, "terminal deployment result changed");
            }
        }
        intent.result = Some(serde_json::to_string(event)?);
        tx.execute(
            "UPDATE deployment_intents SET data=? WHERE community=? AND owner=? AND operation_id=?",
            params![
                serde_json::to_string(&intent)?,
                selected,
                keys.public_key().to_hex(),
                operation
            ],
        )?;
        tx.commit()?;
        Ok(r)
    }
}
/// Lower-level owner transport. Native callers revalidate workspace before every
/// send through the supplied signer closure. No key is exposed to the WebView.
pub struct Management {
    ws: buzz_ws_client::NostrWsConnection,
    community: String,
    owner: PublicKey,
    relay: PublicKey,
}
impl Management {
    /// Negotiate the selected relay's default-off capability and authenticate with
    /// the current native signer; deployment commands never use HTTP transport.
    pub async fn connect_guarded(
        selected: &str,
        enabled: bool,
        signer: &impl Fn() -> Result<Keys>,
    ) -> Result<Self> {
        ensure!(enabled, "host deployment disabled");
        let owner = signer()?.public_key();
        let metadata = Endpoint::new(selected, owner)?.metadata().await?;
        ensure!(
            metadata["host_deployment"]
                == serde_json::json!({"versions":[1],"kind":43104,"admission":"host-scoped","reads":"req-v1"}),
            "host deployment unsupported"
        );
        let relay = registry::Registry::capability(&metadata)?;
        ensure!(signer()?.public_key() == owner, "active signer changed");
        let mut ws = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            buzz_ws_client::NostrWsConnection::connect(&selected.replacen("https://", "wss://", 1)),
        )
        .await??;
        ws.authenticate_with_signer(|challenge, url| {
            let keys = signer().map_err(|_| {
                buzz_ws_client::WsClientError::AuthFailed("active session changed".into())
            })?;
            if keys.public_key() != owner {
                return Err(buzz_ws_client::WsClientError::AuthFailed(
                    "active signer changed".into(),
                ));
            }
            buzz_ws_client::build_auth_event(challenge, url, &keys, None)
        })
        .await?;
        ensure!(signer()?.public_key() == owner, "active signer changed");
        Self::from_authenticated(true, selected, owner, relay, ws)
    }
    /// Bind an authenticated existing client only after explicit feature negotiation.
    pub fn from_authenticated(
        enabled: bool,
        community: &str,
        owner: PublicKey,
        relay: PublicKey,
        ws: buzz_ws_client::NostrWsConnection,
    ) -> Result<Self> {
        ensure!(
            enabled && buzz_core::host_registry::origin(community),
            "host deployment disabled or unsupported"
        );
        Ok(Self {
            ws,
            community: community.into(),
            owner,
            relay,
        })
    }
    /// Fetch private capability or operation evidence under current permissions.
    pub async fn query(
        &mut self,
        signer: &impl Fn() -> Result<Keys>,
        q: &wire::Query,
    ) -> Result<wire::Reply> {
        let keys = signer()?;
        session(&q.scope, &self.community, &keys)?;
        ensure!(keys.public_key() == self.owner, "active signer changed");
        let reply = self
            .ws
            .deployment_query(&keys, self.relay, q, |_| Ok(()))
            .await?;
        session(&q.scope, &self.community, &signer()?)?;
        Ok(reply)
    }
    /// Read a fresh typed discovery observation through current relay permissions.
    /// Historical encrypted receipts remain immutable and cannot authorize launch.
    pub async fn observation(
        &mut self,
        device: &mut Device,
        signer: &impl Fn() -> Result<Keys>,
        operation: &str,
    ) -> Result<Option<buzz_core::host_deployment::observation::Observation>> {
        let keys = signer()?;
        let intent = device
            .deployment_intent(&self.community, &keys, operation)?
            .context("missing intent")?;
        let request = &intent.protected.authority;
        let operation = self.operation(device, signer, operation).await?;
        ensure!(
            operation.current,
            "placement advanced; observation is historical"
        );
        let Some(body) = operation.body else {
            return Ok(None);
        };
        let Some(observation) = body.observation else {
            return Ok(None);
        };
        let Command::Deploy { config, .. } =
            wire::restore_command(&keys, request, &intent.protected.ciphertext)?
        else {
            bail!("not a preparation operation");
        };
        observation.current(&body.result, &config, now()?)?;
        Ok(Some(observation))
    }
    /// Send the exact journaled command only after main materialization/publication.
    pub async fn dispatch(
        &mut self,
        device: &mut Device,
        signer: &impl Fn() -> Result<Keys>,
        operation: &str,
    ) -> Result<()> {
        let keys = signer()?;
        let intent = device
            .deployment_intent(&self.community, &keys, operation)?
            .context("missing intent")?;
        device.restore_materialization(&self.community, &keys, &intent)?;
        ensure!(
            keys.public_key() == self.owner && intent.phase >= 2,
            "main materialization/publication pending"
        );
        session(
            &intent.protected.authority.scope,
            &self.community,
            &signer()?,
        )?;
        if intent.protected.authority.action == buzz_core::host_deployment::Action::Deploy {
            device.deployment_open(&self.community, &keys, &intent.protected.authority)?;
        }
        // Reply loss must be reconciled before interpreting command expiry.
        let evidence = self.operation(device, signer, operation).await?;
        ensure!(
            !evidence.admitted || evidence.current,
            "placement conflict; exact old intent retained"
        );
        if now()? >= intent.protected.authority.expires_at {
            ensure!(evidence.admitted && matches!(evidence.state.as_str(), "needs-login" | "applied" | "failed" | "cleanup-pending" | "cleaned"),
                "original deployment command expired; identity retained; explicit new operation required");
            return Ok(());
        }
        if intent.protected.authority.action == buzz_core::host_deployment::Action::Deploy {
            device.deployment_open(&self.community, &signer()?, &intent.protected.authority)?;
        }
        let ok = self
            .ws
            .send_event(serde_json::from_str(&intent.event)?)
            .await?;
        ensure!(ok.accepted, "deployment admission denied");
        let keys = signer()?;
        session(&intent.protected.authority.scope, &self.community, &keys)?;
        if intent.phase < 3 {
            device.deployment_advance(&self.community, &keys, operation, 3)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "deployment_tests.rs"]
mod tests;

fn restore_snapshot(keys: &Keys, intent: &Intent) -> Result<Value> {
    let plain = Zeroizing::new(nip44::decrypt(
        keys.secret_key(),
        &keys.public_key(),
        &intent.materialization,
    )?);
    let snapshot: Snapshot = buzz_core::host_registry::object(plain.as_bytes(), 48 * 1024)?;
    ensure!(
        snapshot.authority == intent.protected.authority,
        "materialization belongs to another operation"
    );
    Ok(snapshot.data)
}

mod cleanup;
pub use cleanup::CleanupPlan;
mod continuation;
pub use continuation::Operation;

fn validate_materialization(selected: &str, keys: &Keys, intent: &Intent) -> Result<Value> {
    let a = &intent.protected.authority;
    session(&a.scope, selected, keys)?;
    let (_, header) = wire::verify(intent.event.as_bytes(), selected, keys.public_key())?;
    ensure!(
        header
            == (wire::Header::Command {
                authority: a.clone()
            }),
        "intent changed"
    );
    let event: Event = serde_json::from_str(&intent.event)?;
    ensure!(
        event.content.as_bytes() == intent.protected.ciphertext,
        "ciphertext changed"
    );
    wire::restore_command(keys, a, &intent.protected.ciphertext)?;
    restore_snapshot(keys, intent)
}
