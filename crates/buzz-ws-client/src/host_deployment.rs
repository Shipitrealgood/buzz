//! Bounded authenticated private deployment query on the existing WS transport.
use crate::{NostrWsConnection, RelayMessage, WsClientError};
use buzz_core::host_deployment::wire::{self, Query, Reply};
use nostr::{Keys, PublicKey, Timestamp};
use std::time::{Duration, Instant};
impl NostrWsConnection {
    /// Fetch one nonce-bound encrypted response. Other frames remain observable
    /// by the existing registration live subscriber; no separate transport loop.
    pub async fn deployment_query(
        &mut self,
        keys: &Keys,
        relay: PublicKey,
        query: &Query,
        mut observe: impl FnMut(&RelayMessage) -> Result<(), WsClientError>,
    ) -> Result<Reply, WsClientError> {
        let invalid = || WsClientError::UnexpectedMessage("reply-invalid".into());
        query.validate().map_err(|_| invalid())?;
        let sub = format!("deployment-{}", query.nonce);
        self.send_raw(&serde_json::json!(["REQ",sub,{"kinds":[43104],"#p":[keys.public_key().to_hex()],"host_deployment":query}])).await?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut reply = None;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(WsClientError::Timeout);
            }
            match self.next_event(left).await? {
                RelayMessage::Event {
                    subscription_id,
                    event,
                } if subscription_id == sub => {
                    if reply.is_some() {
                        return Err(WsClientError::UnexpectedMessage("reply-duplicate".into()));
                    }
                    reply = Some(
                        wire::open_reply(keys, relay, query, &event, Timestamp::now().as_secs())
                            .map_err(|_| invalid())?,
                    );
                }
                RelayMessage::Eose { subscription_id } if subscription_id == sub => {
                    return reply
                        .ok_or_else(|| WsClientError::UnexpectedMessage("reply-missing".into()))
                }
                RelayMessage::Closed {
                    subscription_id, ..
                } if subscription_id == sub => {
                    return Err(WsClientError::UnexpectedMessage("read-closed".into()))
                }
                other => observe(&other)?,
            }
        }
    }
}
