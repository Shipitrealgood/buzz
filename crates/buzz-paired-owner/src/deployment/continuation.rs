//! Reconciliation and continuation use only the existing protected intents.
use super::*;
use buzz_core::host_deployment::{observation, Action, ModelSelection, Outcome};

/// Sanitized current placement evidence. Receipts alone are historical.
#[derive(Serialize)]
pub struct Operation {
    /// Exact operation was found under current read permission.
    pub admitted: bool,
    /// Placement still belongs to this operation's admitted revision.
    pub current: bool,
    /// Relay's current admitted revision, including failed operations.
    pub revision: Option<u64>,
    /// Validated opaque native unit; never an executable path.
    pub unit: Option<String>,
    /// Relay application state, separate from conversational presence.
    pub state: String,
    /// Verified bounded host evidence, absent before a receipt.
    pub body: Option<observation::ResultBody>,
}
impl Device {
    /// Enumerate only this owner's existing bounded journal, validating every row.
    pub fn deployment_intents(&self, selected: &str, keys: &Keys) -> Result<Vec<Intent>> {
        self.deployment_schema()?;
        let mut stmt = self.db.prepare(
            "SELECT data FROM deployment_intents WHERE community=? AND owner=? LIMIT 1025",
        )?;
        let rows = stmt.query_map(params![selected, keys.public_key().to_hex()], |r| {
            r.get::<_, String>(0)
        })?;
        let mut intents = Vec::new();
        for raw in rows {
            let intent: Intent = serde_json::from_str(&raw?)?;
            self.restore_materialization(selected, keys, &intent)?;
            intents.push(intent);
        }
        ensure!(intents.len() <= 1024, "deployment intent capacity exceeded");
        intents.sort_by_key(|i| i.protected.authority.expected_revision);
        Ok(intents)
    }
}
impl Management {
    /// Reconcile exact journal bytes with the admitted operation and placement.
    pub async fn operation(
        &mut self,
        device: &mut Device,
        signer: &impl Fn() -> Result<Keys>,
        operation: &str,
    ) -> Result<Operation> {
        let keys = signer()?;
        let intent = device
            .deployment_intent(&self.community, &keys, operation)?
            .context("missing intent")?;
        device.restore_materialization(&self.community, &keys, &intent)?;
        let reply = self
            .query(
                signer,
                &wire::Query {
                    v: 1,
                    scope: intent.protected.authority.scope.clone(),
                    view: "operation".into(),
                    nonce: hex::encode(rand::random::<[u8; 32]>()),
                    operation_id: Some(operation.into()),
                },
            )
            .await?;
        let keys = signer()?;
        let evidence = reconcile(&keys, &intent, &reply.data)?;
        if let Some(raw) = reply.data["result"].as_str() {
            device.deployment_result(
                &self.community,
                &keys,
                operation,
                &serde_json::from_str(raw)?,
            )?;
        }
        if evidence.admitted && intent.phase == 2 {
            device.deployment_advance(&self.community, &keys, operation, 3)?;
        }
        Ok(evidence)
    }
    /// Create a new immutable intent only after exact current-operation reconciliation.
    /// Main supplies a keyless snapshot with only the explicitly selected model changed.
    pub async fn continue_deployment(
        &mut self,
        device: &mut Device,
        signer: &impl Fn() -> Result<Keys>,
        previous: &str,
        status: &Status,
        model: ModelSelection,
        snapshot: Value,
    ) -> Result<Intent> {
        let evidence = self.operation(device, signer, previous).await?;
        let keys = signer()?;
        let old = device
            .deployment_intent(&self.community, &keys, previous)?
            .context("missing intent")?;
        let a = &old.protected.authority;
        let Command::Deploy { config, identity } =
            wire::restore_command(&keys, a, &old.protected.ciphertext)?
        else {
            bail!("not a setup intent");
        };
        let revision = if evidence.admitted {
            ensure!(
                evidence.current,
                "placement conflict; another operation advanced it"
            );
            ensure!(
                matches!(evidence.state.as_str(), "needs-login" | "failed"),
                "operation is unresolved or no longer a setup"
            );
            evidence.revision.context("missing placement revision")?
        } else {
            ensure!(
                a.expected_revision == 0 && now()? >= a.expires_at,
                "operation admission unresolved; retry exact intent"
            );
            ensure!(
                matches!(model, ModelSelection::PendingLogin {}),
                "refresh preparation first"
            );
            0
        };
        status.permits(
            &a.scope,
            &config.catalog_revision,
            &config.runtime_id,
            now()?,
        )?;
        if let ModelSelection::Selected { id } = &model {
            let body = evidence
                .body
                .as_ref()
                .context("preparation observation unavailable")?;
            let observation = body.observation.as_ref().context("models unavailable")?;
            observation.current(&body.result, &config, now()?)?;
            ensure!(
                observation.login == observation::LoginState::Succeeded,
                "native login not complete"
            );
            let observation::Models::Available { catalog } = &observation.models else {
                bail!("models unavailable");
            };
            ensure!(
                catalog.models.iter().any(|m| &m.id == id),
                "selected model is no longer available"
            );
        }
        let prior = device.restore_materialization(&self.community, &keys, &old)?;
        let mut expected = prior.clone();
        expected["record"]["model"] = match &model {
            ModelSelection::Selected { id } => Value::String(id.clone()),
            ModelSelection::PendingLogin {} => prior["record"]["model"].clone(),
        };
        ensure!(snapshot == expected, "saved setup configuration changed");
        let mut next_config = *config;
        next_config.model = model;
        let authority = Authority {
            v: 1,
            scope: a.scope.clone(),
            agent: a.agent,
            action: Action::Deploy,
            operation_id: hex::encode(rand::random::<[u8; 32]>()),
            expected_revision: revision,
            expires_at: now()? + 600,
            content_sha256: String::new(),
        };
        device.retain_deployment(
            &self.community,
            &keys,
            authority,
            &Command::Deploy {
                config: Box::new(next_config),
                identity,
            },
            status,
            snapshot,
            Some(previous),
        )
    }
}
pub(super) fn reconcile(keys: &Keys, intent: &Intent, data: &Value) -> Result<Operation> {
    let a = &intent.protected.authority;
    if data["event"].is_null() {
        ensure!(
            data["state"] == "missing-or-superseded" && data["result"].is_null(),
            "invalid missing operation response"
        );
        return Ok(Operation {
            admitted: false,
            current: false,
            revision: None,
            unit: None,
            state: "unadmitted".into(),
            body: None,
        });
    }
    ensure!(
        data["event"].as_str() == Some(&intent.event),
        "admitted operation bytes changed"
    );
    let revision = data["revision"]
        .as_u64()
        .context("missing placement revision")?;
    let unit = data["unit_id"].as_str().context("missing unit")?;
    ensure!(buzz_core::host_registry::identifier(unit), "invalid unit");
    let state = data["state"].as_str().context("missing operation state")?;
    ensure!(
        matches!(
            state,
            "pending" | "failed" | "needs-login" | "applied" | "cleanup-pending" | "cleaned"
        ),
        "invalid operation state"
    );
    let body = data["result"]
        .as_str()
        .map(|raw| -> Result<_> {
            Ok(wire::open_result_body(
                keys,
                &serde_json::from_str(raw)?,
                a,
            )?)
        })
        .transpose()?;
    if let Some(body) = &body {
        let expected = match body.result.outcome {
            Outcome::NeedsLogin => "needs-login",
            Outcome::Applied => "applied",
            Outcome::Cleaned => "cleaned",
            Outcome::CleanupPending => "cleanup-pending",
            Outcome::Failed => "failed",
            Outcome::Pending => "pending",
        };
        ensure!(state == expected, "receipt and operation state disagree");
    }
    Ok(Operation {
        admitted: true,
        current: a.expected_revision.checked_add(1) == Some(revision),
        revision: Some(revision),
        unit: Some(unit.into()),
        state: state.into(),
        body,
    })
}
