use super::*;

#[tokio::test]
async fn strict_session_rejection_prevents_actual_prompt_and_default_off_continues() {
    for (strict, mode, desired, allowed) in [
        (true, "missing", "model-b", false),
        (false, "missing", "model-b", true),
        (true, "rejected", "model-b", false),
        (false, "rejected", "model-b", true),
        (true, "accepted", "model-b", true),
        (true, "current", "model-a", true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("requests");
        let options = if mode == "missing" {
            r#"[{"category":"model","configId":"model","currentValue":"model-a","options":[{"value":"model-a"}]}]"#
        } else {
            r#"[{"category":"model","configId":"model","currentValue":"model-a","options":[{"value":"model-a"},{"value":"model-b"}]}]"#
        };
        // Discovery previously contained model-b; a missing fresh session must still fail.
        let discovery = buzz_core::agent_models::normalize_agent_models(
            &serde_json::json!({"stable":{"configOptions":[{"category":"model","options":[{"value":"model-b"}]}]}}),
            None,
        );
        assert_eq!(discovery.models[0].id, "model-b");
        let switch = if mode == "rejected" {
            r#""error":{"code":-32602,"message":"rejected"}"#
        } else {
            r#""result":{}"#
        };
        let script = format!(
            r#"id=0
while IFS= read -r line; do
 printf '%s\n' "$line" >> '{}'
 case "$line" in
 *'session/new'*) printf '%s\n' '{{"jsonrpc":"2.0","id":'"$id"',"result":{{"sessionId":"s","configOptions":{options}}}}}' ;;
 *'session/set_config_option'*) printf '%s\n' '{{"jsonrpc":"2.0","id":'"$id"',{switch}}}' ;;
 *'session/prompt'*) printf '%s\n' '{{"jsonrpc":"2.0","id":'"$id"',"result":{{"stopReason":"end_turn"}}}}' ;;
 esac
 id=$((id+1))
done"#,
            log.display()
        );
        let acp = AcpClient::spawn("bash", &["-c".into(), script], &[], false)
            .await
            .unwrap();
        let mut agent = OwnedAgent {
            index: 0,
            acp,
            state: SessionState::default(),
            model_capabilities: None,
            desired_model: Some(desired.into()),
            strict_model: strict,
            model_overridden: false,
            desired_model_request_id: None,
            desired_model_pending_ack: false,
            startup_effort: None,
            agent_name: "strict-fixture".into(),
            goose_system_prompt_supported: None,
            protocol_version: 2,
        };
        let context = tests::make_prompt_context_no_owner();
        let mut session = None;
        let result = run_isolated_prompt(
            &mut agent,
            &context,
            "prompt-sentinel",
            Duration::from_secs(3),
            &mut session,
        )
        .await;
        assert_eq!(result.is_ok(), allowed, "{strict}/{mode}: {result:?}");
        agent.acp.shutdown().await;
        let log = std::fs::read_to_string(log).unwrap();
        assert_eq!(log.contains("session/prompt"), allowed, "{strict}/{mode}");
        if mode == "current" {
            assert!(!log.contains("session/set_config_option"));
        }
    }
}
