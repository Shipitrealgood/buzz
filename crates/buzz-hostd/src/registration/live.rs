//! One verified invalidation watch, sharing the authenticated reconciliation
//! socket. Hints never apply state; all effects still pass through reconciliation.
use super::*;
use buzz_ws_client::{NostrWsConnection, RelayMessage};
use std::time::Duration;

pub(super) struct Live {
    sub: String,
    query: wire::Query,
    community: String,
    relay: PublicKey,
    high: Option<String>,
    ready: bool,
    dirty: bool,
}
impl Host {
    /// Subscribe before reading a snapshot. Exact pending writes must be flushed
    /// first: expired timely ack recovery is intentionally write-only admission.
    pub async fn subscribe_live(
        &mut self,
        ws: &mut NostrWsConnection,
        relay: PublicKey,
        g: &str,
    ) -> Result<()> {
        let query = wire::Query {
            v: 2,
            view: "live".into(),
            nonce: store::random()?,
            cursor: None,
            generation: Some(g.into()),
            event_id: None,
        };
        query.validate()?;
        let community = self
            .store
            .0
            .query_row("SELECT c FROM grants WHERE g=?", [g], |r| r.get(0))?;
        let sub = format!("live-{}", store::random()?);
        ws.send_raw(&serde_json::json!(["REQ",sub,{"kinds":[43103],"#p":[self.public_key()?.to_hex()],"host_registry":query}])).await?;
        *self.live.borrow_mut() = Some(Live {
            sub,
            query,
            community,
            relay,
            high: None,
            ready: false,
            dirty: false,
        });
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let message = ws.next_event(Duration::from_secs(5)).await?;
                self.observe_live(&message)?;
                if self.live.borrow().as_ref().is_some_and(|live| live.ready) {
                    return Ok::<_, anyhow::Error>(());
                }
            }
        })
        .await??;
        Ok(())
    }
    /// Verify and coalesce a live frame, including frames interleaved with finite
    /// pages. A CLOSED or invalid correlated reply invalidates this session.
    pub fn observe_live(&self, message: &RelayMessage) -> Result<()> {
        let keys = self.store.keys()?;
        let mut watch = self.live.borrow_mut();
        let Some(live) = &mut *watch else {
            return Ok(());
        };
        match message {
            RelayMessage::Event {
                subscription_id,
                event,
            } if subscription_id == &live.sub => {
                let reply =
                    wire::open_reply(&keys, live.relay, &live.community, &live.query, event)?;
                ensure!(reply.complete, "invalid live continuation");
                match reply.op.as_str() {
                    "registry-page" => ensure!(live.high.is_none(), "duplicate live page"),
                    "registry-change" => {
                        ensure!(live.high.is_some(), "change before live page");
                        ensure!(
                            reply.data["generation"].as_str() == live.query.generation.as_deref(),
                            "wrong live generation"
                        );
                        live.dirty |= live.high.as_ref() != Some(&reply.high_water);
                    }
                    _ => anyhow::bail!("invalid live operation"),
                }
                live.high = Some(reply.high_water);
            }
            RelayMessage::Eose { subscription_id } if subscription_id == &live.sub => {
                ensure!(live.high.is_some() && !live.ready, "invalid live EOSE");
                live.ready = true;
            }
            RelayMessage::Closed {
                subscription_id, ..
            } if subscription_id == &live.sub => {
                anyhow::bail!("registry live subscription closed")
            }
            _ => {}
        }
        Ok(())
    }
    /// Consume coalesced invalidations before reconciliation, never afterward:
    /// changes arriving during snapshot reads must remain scheduled.
    pub fn take_live_change(&mut self) -> bool {
        self.live
            .get_mut()
            .as_mut()
            .is_some_and(|live| std::mem::take(&mut live.dirty))
    }
    /// Whether this generation has exact local work, including an offline revoke.
    pub fn has_pending(&self, g: &str) -> Result<bool> {
        Ok(!self.pending_generation(Some(g))?.is_empty())
    }
}
