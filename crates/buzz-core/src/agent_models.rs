//! Pure ACP model projection shared by native clients and hosted discovery.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Response from `get_agent_models` — normalized model info for the frontend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelsResponse {
    /// ACP implementation name.
    pub agent_name: String,
    /// ACP implementation version.
    pub agent_version: String,
    /// Unified model list (merged from both ACP paths, deduplicated by ID).
    pub models: Vec<AgentModelInfo>,
    /// The agent's default model for a fresh session.
    pub agent_default_model: Option<String>,
    /// The user's persisted model selection (from ManagedAgentRecord.model).
    pub selected_model: Option<String>,
    /// Whether this agent supports model switching.
    pub supports_switching: bool,
}

/// A single model available from an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelInfo {
    /// Canonical ID used for persistence and round-tripping.
    pub id: String,
    /// Optional native display label.
    pub name: Option<String>,
    /// Optional native model description.
    pub description: Option<String>,
}

/// Normalize raw `buzz-acp models --json` output into a typed DTO for the frontend.
///
/// Merges models from both ACP paths (stable configOptions + unstable SessionModelState),
/// deduplicates by ID (stable takes precedence), and returns a unified list.
pub fn normalize_agent_models(
    raw: &serde_json::Value,
    persisted_model: Option<String>,
) -> AgentModelsResponse {
    let agent_name = raw["agent"]["name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let agent_version = raw["agent"]["version"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let mut models: Vec<AgentModelInfo> = Vec::new();
    let mut agent_default_model: Option<String> = None;
    let mut seen_ids: HashSet<String> = HashSet::new();

    // 1. Stable configOptions (preferred). Only entries with category "model"
    //    are model options — the CLI pre-filters, but we're defensive here.
    if let Some(config_options) = raw["stable"]["configOptions"].as_array() {
        for opt in config_options {
            if opt.get("category").and_then(|c| c.as_str()) != Some("model") {
                continue;
            }
            if agent_default_model.is_none() {
                agent_default_model = opt
                    .get("currentValue")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            if let Some(options) = opt.get("options").and_then(|v| v.as_array()) {
                for o in options {
                    if let Some(value) = o.get("value").and_then(|v| v.as_str()) {
                        if seen_ids.insert(value.to_string()) {
                            models.push(AgentModelInfo {
                                id: value.to_string(),
                                name: o.get("name").and_then(|v| v.as_str()).map(str::to_string),
                                description: o
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .map(str::to_string),
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. Unstable availableModels (fallback — skip duplicates from stable).
    if let Some(unstable) = raw.get("unstable") {
        if agent_default_model.is_none() {
            agent_default_model = unstable["currentModelId"].as_str().map(str::to_string);
        }
        if let Some(available) = unstable["availableModels"].as_array() {
            for m in available {
                if let Some(id) = m.get("modelId").and_then(|v| v.as_str()) {
                    if seen_ids.insert(id.to_string()) {
                        models.push(AgentModelInfo {
                            id: id.to_string(),
                            name: m.get("name").and_then(|v| v.as_str()).map(str::to_string),
                            description: m
                                .get("description")
                                .and_then(|v| v.as_str())
                                .map(str::to_string),
                        });
                    }
                }
            }
        }
    }

    let supports_switching = !models.is_empty();

    AgentModelsResponse {
        agent_name,
        agent_version,
        models,
        agent_default_model,
        selected_model: persisted_model,
        supports_switching,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn claude_code_models_raw() -> serde_json::Value {
        serde_json::json!({
            "agent": { "name": "claude-agent-acp", "version": "0.36.1" },
            "stable": { "configOptions": [{
                "id": "model",
                "name": "Model",
                "category": "model",
                "type": "select",
                "currentValue": "opus[1m]",
                "options": [
                    { "value": "default", "name": "Default (recommended)",
                      "description": "Use the default model (currently claude-opus-5-5[1m])" },
                    { "value": "opus[1m]", "name": "Opus",
                      "description": "Opus with 1M context · Best for everyday, complex tasks" },
                    { "value": "haiku", "name": "Haiku",
                      "description": "Haiku 4.5 · Fastest for quick answers · $1/$5 per Mtok" }
                ]
            }]}
        })
    }

    #[test]
    fn claude_code_models_keep_adapter_name_description_and_current_value() {
        let response = normalize_agent_models(&claude_code_models_raw(), None);

        let haiku = response.models.iter().find(|m| m.id == "haiku").unwrap();
        assert_eq!(haiku.name.as_deref(), Some("Haiku"));
        assert_eq!(
            haiku.description.as_deref(),
            Some("Haiku 4.5 · Fastest for quick answers · $1/$5 per Mtok")
        );
        assert_eq!(response.agent_default_model.as_deref(), Some("opus[1m]"));
    }

    #[test]
    fn stable_current_value_outranks_unstable_current_model_id() {
        let mut raw = claude_code_models_raw();
        raw["unstable"] = serde_json::json!({ "currentModelId": "haiku", "availableModels": [] });

        let response = normalize_agent_models(&raw, None);

        assert_eq!(response.agent_default_model.as_deref(), Some("opus[1m]"));
    }
}
