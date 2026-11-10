//! Opt-in deployment over the registered Host's existing identity and journal.
use crate::{registration::Host, store, Catalog, Instance, ManagedDeployment};
use anyhow::{ensure, Context, Result};
use buzz_core::{
    host_deployment::{
        wire, Action, Authority, Availability, Capability, Command, DeploymentResult,
        ModelSelection, Outcome, Status,
    },
    paired_message::Scope,
};
use buzz_ws_client::NostrWsConnection;
use nostr::{Event, PublicKey};
use rusqlite::{params, OptionalExtension};
use std::{
    fs,
    io::Write,
    os::unix::fs::{FileTypeExt, MetadataExt},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Fixed helper adapter. Production uses only `installed`; integration can supply
/// a disposable installation of the same socket/helper implementation.
#[derive(Clone)]
pub struct Helper {
    staging: PathBuf,
    socket: PathBuf,
}
impl Helper {
    /// Refuse the shipped registration-only sandbox before any helper effect.
    pub fn installed(state_dir: &Path) -> Result<Self> {
        ensure!(state_dir == Path::new("/var/lib/buzz-hostd"), "deployment installation incompatible: existing host journal must be mounted at /var/lib/buzz-hostd; registration-only sandbox has no execution boundary");
        Self::at(state_dir, Path::new("/run/buzz-hostd-activate.sock"))
    }
    /// Bind an existing private staging directory and Unix helper socket.
    /// The production executable never accepts overrides for either path.
    pub fn at(staging: &Path, socket: &Path) -> Result<Self> {
        let dir = fs::symlink_metadata(staging)?;
        ensure!(
            staging.is_absolute()
                && staging.canonicalize()? == staging
                && dir.is_dir()
                && dir.uid() == rustix::process::geteuid().as_raw()
                && dir.mode() & 0o077 == 0,
            "unsafe deployment staging"
        );
        ensure!(
            fs::symlink_metadata(socket)?.file_type().is_socket(),
            "deployment installation incompatible: fixed helper socket unavailable"
        );
        Ok(Self {
            staging: staging.into(),
            socket: socket.into(),
        })
    }
    fn stage(&self, value: &Instance) -> Result<()> {
        let mut temp = tempfile::NamedTempFile::new_in(&self.staging)?;
        serde_json::to_writer(&mut temp, value)?;
        temp.flush()?;
        temp.as_file().sync_all()?;
        temp.persist(self.staging.join(format!("{}.cred", value.unit_id)))?;
        fs::File::open(&self.staging)?.sync_all()?;
        Ok(())
    }
    async fn action(&self, verb: &str, unit: &str) -> Result<String> {
        ensure!(
            matches!(verb, "prepare" | "start" | "cleanup") && crate::id(unit),
            "invalid fixed helper action"
        );
        tokio::time::timeout(Duration::from_secs(90), async {
            let mut socket = tokio::net::UnixStream::connect(&self.socket).await?;
            socket
                .write_all(format!("{verb} {unit}").as_bytes())
                .await?;
            socket.shutdown().await?;
            let mut output = Vec::new();
            socket.take(16).read_to_end(&mut output).await?;
            Ok::<_, anyhow::Error>(String::from_utf8(output)?)
        })
        .await?
    }
    fn remove_staged(&self, unit: &str) -> Result<()> {
        let path = self.staging.join(format!("{unit}.cred"));
        if path.try_exists()? {
            fs::remove_file(path)?;
            fs::File::open(&self.staging)?.sync_all()?;
        }
        Ok(())
    }
}
impl Host {
    fn deployment_schema(&self) -> Result<()> {
        self.store.0.execute_batch("CREATE TABLE IF NOT EXISTS deployment_units(agent TEXT PRIMARY KEY,generation TEXT NOT NULL,unit_id TEXT NOT NULL,operation TEXT NOT NULL,event TEXT NOT NULL,revision INTEGER NOT NULL,cleanup INTEGER NOT NULL DEFAULT 0,cleaned INTEGER NOT NULL DEFAULT 0,result TEXT,issue TEXT); CREATE TABLE IF NOT EXISTS deployment_attempts(nonce TEXT PRIMARY KEY,operation TEXT NOT NULL); CREATE TABLE IF NOT EXISTS deployment_status(generation TEXT PRIMARY KEY,event TEXT NOT NULL)")?;
        Ok(())
    }
    fn deployment_cursor(&self, key: &str) -> Result<String> {
        Ok(self
            .store
            .0
            .query_row("SELECT r FROM work_cursor WHERE g=?", [key], |r| r.get(0))
            .optional()?
            .unwrap_or_default())
    }
    fn advance_deployment_cursor(&self, key: &str, value: &str) -> Result<()> {
        self.store.0.execute(
            "INSERT INTO work_cursor VALUES(?,?) ON CONFLICT(g) DO UPDATE SET r=excluded.r",
            params![key, value],
        )?;
        Ok(())
    }
    fn deployment_scope(&self, g: &str) -> Result<Scope> {
        let (community, owner): (String, String) =
            self.store
                .0
                .query_row("SELECT c,owner FROM grants WHERE g=?", [g], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })?;
        Ok(Scope {
            community,
            owner: PublicKey::from_hex(&owner)?,
            host: self.public_key()?,
            generation: g.into(),
        })
    }
    async fn deployment_query(
        &self,
        ws: &mut NostrWsConnection,
        relay: PublicKey,
        scope: &Scope,
        view: &str,
        operation: Option<&str>,
    ) -> Result<wire::Reply> {
        let q = wire::Query {
            v: 1,
            scope: scope.clone(),
            view: view.into(),
            nonce: store::random()?,
            operation_id: operation.map(str::to_string),
        };
        Ok(ws
            .deployment_query(&self.store.keys()?, relay, &q, |m| {
                self.observe_live(m).map_err(|_| {
                    buzz_ws_client::WsClientError::UnexpectedMessage(
                        "registration watch unavailable".into(),
                    )
                })
            })
            .await?)
    }
    async fn deployment_permit(
        &mut self,
        ws: &mut NostrWsConnection,
        relay: PublicKey,
        a: &Authority,
        unit_id: &str,
    ) -> Result<()> {
        let r = self
            .deployment_query(ws, relay, &a.scope, "permit", Some(&a.operation_id))
            .await?;
        ensure!(
            r.data["authority"] == serde_json::to_value(a)?
                && r.data["unit_id"] == unit_id
                && r.data["revision"] == a.expected_revision + 1,
            "execution authority mismatch"
        );
        let tx = self.store.tx()?;
        let state: String = tx.query_row(
            "SELECT state FROM grants WHERE g=?",
            [&a.scope.generation],
            |r| r.get(0),
        )?;
        let fenced: bool = tx.query_row(
            "SELECT cleanup FROM deployment_units WHERE agent=?",
            [a.agent.to_hex()],
            |r| r.get(0),
        )?;
        ensure!(
            state == "active"
                && (!fenced || a.action == Action::Cleanup)
                && nostr::Timestamp::now().as_secs() < r.expires_at,
            "execution permit stale or locally revoked"
        );
        let count: i64 =
            tx.query_row("SELECT count(*) FROM deployment_attempts", [], |r| r.get(0))?;
        ensure!(
            count < 65536,
            "execution attempt retention capacity reached"
        );
        tx.execute(
            "INSERT INTO deployment_attempts VALUES(?,?)",
            params![r.query.nonce, a.operation_id],
        )?;
        tx.commit()?;
        Ok(())
    }
    /// Reconcile a finite batch through the actual journal and fixed helper.
    /// Caller must explicitly enable and provide a compatible execution boundary.
    pub async fn reconcile_deployments(
        &mut self,
        ws: &mut NostrWsConnection,
        relay: PublicKey,
        g: &str,
        catalog: &Catalog,
        revision: &str,
        helper: &Helper,
    ) -> Result<()> {
        self.deployment_schema()?;
        ensure!(
            self.store.0.path() == helper.staging.join("host.sqlite").to_str(),
            "helper staging must be the existing registered journal directory"
        );
        let scope = self.deployment_scope(g)?;
        if self.state(g)? != "active" {
            return self.cleanup_deployments(g, helper).await;
        }
        ensure!(
            catalog.community.as_ref() == Some(&scope.community) && catalog.current(revision)?,
            "catalog changed or wrong community"
        );
        let runtimes = catalog
            .runtimes
            .iter()
            .filter(|(_, r)| {
                r.credential_profile == "none"
                    && r.harness.verify().is_ok()
                    && r.adapter.verify().is_ok()
            })
            .map(|(id, _)| id.clone())
            .collect();
        let now = nostr::Timestamp::now().as_secs();
        let status = Status {
            scope: scope.clone(),
            availability: Availability::Current,
            capability: Capability::Supported {
                catalog_revision: revision.into(),
                runtimes,
            },
            expires_at: now + 120,
        };
        let previous: Option<String> = self
            .store
            .0
            .query_row(
                "SELECT event FROM deployment_status WHERE generation=?",
                [g],
                |r| r.get(0),
            )
            .optional()?;
        let previous = previous
            .map(|s| serde_json::from_str::<Event>(&s))
            .transpose()?;
        let event = match previous {
            Some(e) if e.created_at.as_secs() == now => e,
            _ => wire::status(&self.store.keys()?, &status, now)?,
        };
        self.store.0.execute("INSERT INTO deployment_status VALUES(?,?) ON CONFLICT(generation) DO UPDATE SET event=excluded.event",params![g,serde_json::to_string(&event)?])?;
        ensure!(
            ws.send_event(event).await?.accepted,
            "deployment status denied"
        );
        let cursor_key = format!("deployment-pending:{g}");
        let cursor = self.deployment_cursor(&cursor_key)?;
        let pending = self
            .deployment_query(
                ws,
                relay,
                &scope,
                "pending",
                (!cursor.is_empty()).then_some(cursor.as_str()),
            )
            .await?;
        let operations: Vec<String> = serde_json::from_value(pending.data["operations"].clone())?;
        ensure!(operations.len() <= 32, "oversized work batch");
        for op in operations.iter().take(8) {
            // Advance durably before awaits: a failed request or process restart
            // cannot pin the next finite pass to the same failing prefix.
            self.advance_deployment_cursor(&cursor_key, op)?;
            let row = self
                .deployment_query(ws, relay, &scope, "operation", Some(op))
                .await?;
            let raw = row.data["event"].as_str().context("missing command")?;
            let (event, header) = wire::verify(raw.as_bytes(), &scope.community, scope.owner)?;
            let wire::Header::Command { authority: a } = header else {
                anyhow::bail!("not a command");
            };
            let unit_id = row.data["unit_id"]
                .as_str()
                .context("missing stable unit")?;
            ensure!(
                crate::id(unit_id)
                    && a.scope == scope
                    && a.operation_id == *op
                    && row.data["revision"] == a.expected_revision + 1,
                "placement mismatch"
            );
            let command = wire::open_command(&self.store.keys()?, &event, &a);
            let valid_command = command.is_ok();
            let tx = self.store.tx()?;
            let old:Option<(String,String,u64,bool,Option<String>)>=tx.query_row("SELECT unit_id,event,revision,cleanup,result FROM deployment_units WHERE agent=?",[a.agent.to_hex()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
            let expired = nostr::Timestamp::now().as_secs() >= a.expires_at;
            let persisted_cleanup = old
                .as_ref()
                .is_some_and(|(_, saved, _, fenced, _)| saved == raw && *fenced);
            if old.is_none() {
                let count: i64 =
                    tx.query_row("SELECT count(*) FROM deployment_units", [], |r| r.get(0))?;
                ensure!(
                    count < 4096,
                    "deployment retention capacity reached; existing identities retained"
                );
            }
            if let Some((stable, saved, rev, fenced, result)) = &old {
                ensure!(
                    stable == unit_id && (*rev < a.expected_revision + 1 || saved == raw),
                    "local placement conflict"
                );
                ensure!(
                    !fenced || a.action == Action::Cleanup,
                    "cleanup tombstone prevents resurrection"
                );
                if saved == raw {
                    if let Some(result) = result {
                        let e: Event = serde_json::from_str(result)?;
                        let (_, h) = wire::verify(result.as_bytes(), &scope.community, scope.host)?;
                        if matches!(
                            h,
                            wire::Header::Result {
                                result: DeploymentResult {
                                    outcome: Outcome::Applied
                                        | Outcome::NeedsLogin
                                        | Outcome::Cleaned,
                                    ..
                                }
                            }
                        ) {
                            tx.commit()?;
                            ensure!(ws.send_event(e).await?.accepted, "result retry denied");
                            continue;
                        }
                    }
                }
            }
            tx.execute("INSERT INTO deployment_units(agent,generation,unit_id,operation,event,revision,cleanup) VALUES(?,?,?,?,?,?,?) ON CONFLICT(agent) DO UPDATE SET operation=excluded.operation,event=excluded.event,revision=excluded.revision,cleanup=max(cleanup,excluded.cleanup),cleaned=0,result=NULL,issue=NULL",params![a.agent.to_hex(),g,unit_id,op,raw,a.expected_revision+1,a.action==Action::Cleanup && valid_command && (!expired || persisted_cleanup)])?;
            tx.commit()?;
            let unit = crate::unit_id(&scope, unit_id);
            let applied = async {
                ensure!(
                    !expired || persisted_cleanup,
                    "original command expired; identity retained; explicit new operation required"
                );
                match command? {
                    Command::Cleanup {} => {
                        // A persisted restrictive intent needs no new grant to stop.
                        ensure!(
                            helper.action("cleanup", &unit).await? == "cleaned\n",
                            "cleanup unconfirmed"
                        );
                        helper.remove_staged(&unit)?;
                        Ok(Outcome::Cleaned)
                    }
                    Command::Deploy { config, identity } => {
                        let keys = nostr::Keys::parse(&identity.private_key_nsec)?;
                        let pending = matches!(config.model, ModelSelection::PendingLogin {});
                        let instance = Instance {
                            deployment: Some(ManagedDeployment {
                                operation_id: op.clone(),
                                revision: a.expected_revision + 1,
                                config: (*config).clone(),
                            }),
                            unit_id: unit.clone(),
                            scope: scope.clone(),
                            request_id: unit_id.into(),
                            runtime_id: config.runtime_id.clone(),
                            revision: revision.into(),
                            channels: vec![],
                            agent: a.agent.to_hex(),
                            secret: keys.secret_key().to_secret_hex(),
                            oa: identity.auth_tag.context("missing NIP-OA")?,
                            policy: config.policy.clone(),
                        };
                        instance.validate(catalog, revision)?;
                        helper.stage(&instance)?;
                        self.deployment_permit(ws, relay, &a, unit_id).await?;
                        ensure!(
                            helper.action("prepare", &unit).await? == "prepared\n",
                            "prepare unconfirmed"
                        );
                        if pending {
                            return Ok(Outcome::NeedsLogin);
                        }
                        self.deployment_permit(ws, relay, &a, unit_id).await?;
                        ensure!(
                            helper.action("start", &unit).await? == "active\n",
                            "start unconfirmed"
                        );
                        Ok(Outcome::Applied)
                    }
                }
            }
            .await;
            let (outcome, issue) = match applied {
                Ok(outcome) => (outcome, None),
                Err(e) => (
                    if a.action == Action::Cleanup
                        && valid_command
                        && (!expired || persisted_cleanup)
                    {
                        Outcome::CleanupPending
                    } else {
                        Outcome::Failed
                    },
                    Some(e.to_string()),
                ),
            };
            let applied_revision = matches!(
                outcome,
                Outcome::Applied | Outcome::NeedsLogin | Outcome::Cleaned
            )
            .then_some(a.expected_revision + 1);
            let cleaned = outcome == Outcome::Cleaned;
            let result = wire::result(
                &self.store.keys()?,
                &DeploymentResult {
                    request: a,
                    outcome,
                    applied_revision,
                },
                nostr::Timestamp::now().as_secs(),
            )?;
            // Bind the durable receipt by the accepted immutable operation.
            self.store.0.execute(
                "UPDATE deployment_units SET result=?,issue=?,cleaned=? WHERE generation=? AND operation=?",
                params![serde_json::to_string(&result)?, issue, cleaned, g, op],
            )?;
            ensure!(
                ws.send_event(result).await?.accepted,
                "result pending relay delivery"
            );
        }
        Ok(())
    }
    /// Retry only persisted explicit cleanup, independently of relay grants.
    /// Revocation alone does not terminate an already-running agent.
    pub async fn cleanup_deployments(&mut self, g: &str, helper: &Helper) -> Result<()> {
        self.deployment_schema()?;
        let cursor_key = format!("deployment-cleanup:{g}");
        let cursor = self.deployment_cursor(&cursor_key)?;
        let units: Vec<String> = self
            .store
            .0
            .prepare("SELECT unit_id FROM deployment_units WHERE generation=? AND cleanup=1 AND cleaned=0 ORDER BY (unit_id <= ?2),unit_id LIMIT 8")?
            .query_map(params![g, cursor], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let scope = self.deployment_scope(g)?;
        self.advance_deployment_cursor(&format!("deployment-generations:{}", scope.community), g)?;
        for id in units {
            self.advance_deployment_cursor(&cursor_key, &id)?;
            let unit = crate::unit_id(&scope, &id);
            let result = async {
                ensure!(
                    helper.action("cleanup", &unit).await? == "cleaned\n",
                    "local restrictive cleanup pending"
                );
                helper.remove_staged(&unit)
            }
            .await;
            // A failed helper attempt remains durable and does not skip later units.
            self.store.0.execute(
                "UPDATE deployment_units SET cleaned=?,issue=? WHERE generation=? AND unit_id=?",
                params![result.is_ok(), result.err().map(|e| e.to_string()), g, id],
            )?;
        }
        Ok(())
    }
    /// Pending restrictive work survives registration retirement and restarts.
    pub async fn retry_deployment_cleanup(
        &mut self,
        community: &str,
        helper: &Helper,
    ) -> Result<()> {
        for g in self.deployment_cleanup_generations(community)? {
            self.cleanup_deployments(&g, helper).await?;
        }
        Ok(())
    }
    /// Rotate a bounded set of restrictive generation jobs independently of AUTH.
    /// Cursor shares the existing journal, using a namespace outside grant IDs.
    pub fn deployment_cleanup_generations(&self, community: &str) -> Result<Vec<String>> {
        self.deployment_schema()?;
        let key = format!("deployment-generations:{community}");
        let cursor = self.deployment_cursor(&key)?;
        let generations: Vec<String> = self.store.0.prepare("SELECT DISTINCT d.generation FROM deployment_units d JOIN grants g ON g.g=d.generation WHERE g.c=?1 AND d.cleanup=1 AND d.cleaned=0 ORDER BY (d.generation <= ?2),d.generation LIMIT 4")?.query_map(params![community,cursor],|r|r.get(0))?.collect::<std::result::Result<_,_>>()?;
        Ok(generations)
    }
}
