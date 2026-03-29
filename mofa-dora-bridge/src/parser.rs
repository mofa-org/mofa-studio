//! Dataflow YAML parser
//!
//! Parses dora dataflow YAML files to extract:
//! - Node definitions and connections
//! - MoFA dynamic nodes (mofa-xxx)
//! - Environment variable requirements
//! - Log sources for system log widget

use crate::data::LogLevel;
use crate::error::BridgeResult;
use crate::MofaNodeType;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Parsed dataflow with extracted information
#[derive(Debug, Clone)]
pub struct ParsedDataflow {
    /// Dataflow file path
    pub path: PathBuf,
    /// All nodes in the dataflow
    pub nodes: Vec<ParsedNode>,
    /// MoFA dynamic nodes discovered
    pub mofa_nodes: Vec<MofaNodeSpec>,
    /// Environment variable requirements
    pub env_requirements: Vec<EnvRequirement>,
    /// Log sources for system log widget
    pub log_sources: Vec<LogSource>,
    /// Raw YAML for reference
    pub raw_yaml: serde_yaml::Value,
}

/// Specification for a MoFA dynamic node
#[derive(Debug, Clone)]
pub struct MofaNodeSpec {
    /// Node ID (e.g., "mofa-audio-player")
    pub id: String,
    /// Node type
    pub node_type: MofaNodeType,
    /// Expected inputs
    pub inputs: Vec<InputDef>,
    /// Expected outputs
    pub outputs: Vec<String>,
}

/// Parsed node from dataflow
#[derive(Debug, Clone)]
pub struct ParsedNode {
    /// Node ID
    pub id: String,
    /// Node kind
    pub kind: NodeKind,
    /// Inputs with source connections
    pub inputs: Vec<InputDef>,
    /// Output IDs
    pub outputs: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Whether this is a dynamic node
    pub is_dynamic: bool,
}

/// Node kind
#[derive(Debug, Clone)]
pub enum NodeKind {
    /// Python operator
    Python { path: String },
    /// Rust operator
    Rust { path: String },
    /// Custom node
    Custom {
        source: String,
        args: Option<String>,
    },
    /// Dynamic node (connected at runtime)
    Dynamic,
}

/// Input definition with source
#[derive(Debug, Clone)]
pub struct InputDef {
    /// Input ID
    pub id: String,
    /// Source in format "node_id/output_id"
    pub source: String,
}

/// Environment variable requirement for UI configuration
#[derive(Debug, Clone)]
pub struct EnvRequirement {
    /// Variable name (e.g., "OPENAI_API_KEY")
    pub key: String,
    /// Human-readable description
    pub description: String,
    /// Whether this variable is required
    pub required: bool,
    /// Default value if not set
    pub default: Option<String>,
    /// Whether this is a secret (API key, password)
    pub secret: bool,
    /// Which nodes use this variable
    pub used_by: Vec<String>,
}

/// Log source for system log widget
#[derive(Debug, Clone)]
pub struct LogSource {
    /// Source node ID
    pub node_id: String,
    /// Output ID (e.g., "log", "status")
    pub output_id: String,
    /// Display name for the UI
    pub display_name: String,
    /// Default log level filter
    pub default_level: LogLevel,
}

/// Dataflow parser
pub struct DataflowParser;

impl DataflowParser {
    /// Parse a dataflow YAML file
    pub fn parse(path: impl AsRef<Path>) -> BridgeResult<ParsedDataflow> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        Self::parse_string(&content, path.to_path_buf())
    }

    /// Parse dataflow from YAML string
    pub fn parse_string(yaml: &str, path: PathBuf) -> BridgeResult<ParsedDataflow> {
        let raw_yaml: serde_yaml::Value = serde_yaml::from_str(yaml)?;

        let mut nodes = Vec::new();
        let mut mofa_nodes = Vec::new();
        let mut env_requirements = Vec::new();
        let mut log_sources = Vec::new();

        // Parse nodes array
        if let Some(nodes_array) = raw_yaml.get("nodes").and_then(|n| n.as_sequence()) {
            for node_value in nodes_array {
                if let Some(parsed) = Self::parse_node(node_value) {
                    // Check if this is a MoFA node
                    if let Some(mofa_type) = MofaNodeType::from_node_id(&parsed.id) {
                        mofa_nodes.push(MofaNodeSpec {
                            id: parsed.id.clone(),
                            node_type: mofa_type,
                            inputs: parsed.inputs.clone(),
                            outputs: parsed.outputs.clone(),
                        });
                    }

                    // Extract log sources
                    for output in &parsed.outputs {
                        if output.ends_with("_log")
                            || output == "log"
                            || output.ends_with("_status")
                        {
                            log_sources.push(LogSource {
                                node_id: parsed.id.clone(),
                                output_id: output.clone(),
                                display_name: Self::format_display_name(&parsed.id),
                                default_level: LogLevel::Info,
                            });
                        }
                    }

                    // Extract env requirements
                    for (key, value) in &parsed.env {
                        Self::add_env_requirement(
                            &mut env_requirements,
                            key.clone(),
                            value.clone(),
                            parsed.id.clone(),
                        );
                    }

                    nodes.push(parsed);
                }
            }
        }

        Ok(ParsedDataflow {
            path,
            nodes,
            mofa_nodes,
            env_requirements,
            log_sources,
            raw_yaml,
        })
    }

    /// Parse a single node from YAML value
    fn parse_node(value: &serde_yaml::Value) -> Option<ParsedNode> {
        let id = value.get("id")?.as_str()?.to_string();

        // Determine node kind
        let kind = if let Some(op) = value.get("operator") {
            if let Some(python) = op.get("python").and_then(|p| p.as_str()) {
                NodeKind::Python {
                    path: python.to_string(),
                }
            } else if let Some(rust) = op.get("rust").and_then(|r| r.as_str()) {
                NodeKind::Rust {
                    path: rust.to_string(),
                }
            } else {
                NodeKind::Dynamic
            }
        } else if let Some(custom) = value.get("custom") {
            let source = custom
                .get("source")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let args = custom
                .get("args")
                .and_then(|a| a.as_str())
                .map(|s| s.to_string());
            NodeKind::Custom { source, args }
        } else if value.get("path").and_then(|p| p.as_str()) == Some("dynamic") {
            NodeKind::Dynamic
        } else {
            return None;
        };

        // Check if dynamic
        let is_dynamic = value.get("path").and_then(|p| p.as_str()) == Some("dynamic");

        // Parse inputs
        let mut inputs = Vec::new();
        if let Some(inputs_map) = value.get("inputs").and_then(|i| i.as_mapping()) {
            for (key, val) in inputs_map {
                if let Some(id) = key.as_str() {
                    // Handle both string format and nested mapping format
                    let source = if let Some(source_str) = val.as_str() {
                        // Simple string format: "node/output"
                        Some(source_str.to_string())
                    } else if let Some(mapping) = val.as_mapping() {
                        // Nested format: { source: "node/output", queue_size: ... }
                        mapping
                            .get("source")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string())
                    } else {
                        None
                    };

                    if let Some(source) = source {
                        inputs.push(InputDef {
                            id: id.to_string(),
                            source,
                        });
                    }
                }
            }
        }

        // Parse outputs
        let mut outputs = Vec::new();
        if let Some(outputs_seq) = value.get("outputs").and_then(|o| o.as_sequence()) {
            for out in outputs_seq {
                if let Some(out_str) = out.as_str() {
                    outputs.push(out_str.to_string());
                }
            }
        }

        // Parse env
        let mut env = HashMap::new();
        if let Some(env_map) = value.get("env").and_then(|e| e.as_mapping()) {
            for (key, val) in env_map {
                if let Some(key_str) = key.as_str() {
                    let val_str = match val {
                        serde_yaml::Value::String(s) => s.clone(),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        serde_yaml::Value::Number(n) => n.to_string(),
                        _ => continue,
                    };
                    env.insert(key_str.to_string(), val_str);
                }
            }
        }

        Some(ParsedNode {
            id,
            kind,
            inputs,
            outputs,
            env,
            is_dynamic,
        })
    }

    /// Format node ID as display name
    fn format_display_name(node_id: &str) -> String {
        node_id
            .replace('_', " ")
            .replace('-', " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Add or update env requirement
    fn add_env_requirement(
        requirements: &mut Vec<EnvRequirement>,
        key: String,
        value: String,
        node_id: String,
    ) {
        // Check if this is a secret key
        let secret = key.to_uppercase().contains("API_KEY")
            || key.to_uppercase().contains("SECRET")
            || key.to_uppercase().contains("PASSWORD")
            || key.to_uppercase().contains("TOKEN");

        // Parse placeholder syntax: ${VAR}, ${VAR:-default}, or $VAR
        let (is_placeholder, has_default, default_value) =
            if value.starts_with("${") && value.ends_with("}") {
                // ${VAR} or ${VAR:-default}
                let inner = &value[2..value.len() - 1];
                if let Some(pos) = inner.find(":-") {
                    // ${VAR:-default} - has a default value
                    let default = inner[pos + 2..].to_string();
                    (true, true, Some(default))
                } else {
                    // ${VAR} - no default, required
                    (true, false, None)
                }
            } else if value.starts_with("$") {
                // $VAR - no default, required
                (true, false, None)
            } else {
                // Literal value
                (false, false, Some(value.clone()))
            };

        // Only required if it's a placeholder WITHOUT a default
        let required = is_placeholder && !has_default;

        // Find existing or create new
        if let Some(existing) = requirements.iter_mut().find(|r| r.key == key) {
            existing.used_by.push(node_id);
        } else {
            requirements.push(EnvRequirement {
                key,
                description: String::new(),
                required,
                default: default_value,
                secret,
                used_by: vec![node_id],
            });
        }
    }
}

impl ParsedDataflow {
    /// Get all MoFA node IDs
    pub fn mofa_node_ids(&self) -> Vec<&str> {
        self.mofa_nodes.iter().map(|n| n.id.as_str()).collect()
    }

    /// Get MoFA node spec by ID
    pub fn get_mofa_node(&self, id: &str) -> Option<&MofaNodeSpec> {
        self.mofa_nodes.iter().find(|n| n.id == id)
    }

    /// Get node by ID
    pub fn get_node(&self, id: &str) -> Option<&ParsedNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all nodes that send to a MoFA node
    pub fn get_sources_for(&self, mofa_node_id: &str) -> Vec<(&ParsedNode, &str)> {
        let mut sources = Vec::new();
        if let Some(mofa_node) = self.get_mofa_node(mofa_node_id) {
            for input in &mofa_node.inputs {
                // Parse source "node_id/output_id"
                let parts: Vec<&str> = input.source.split('/').collect();
                if parts.len() == 2 {
                    if let Some(source_node) = self.get_node(parts[0]) {
                        sources.push((source_node, parts[1]));
                    }
                }
            }
        }
        sources
    }

    /// Get required env vars that are not set
    pub fn get_missing_env_vars(&self) -> Vec<&EnvRequirement> {
        self.env_requirements
            .iter()
            .filter(|r| r.required && std::env::var(&r.key).is_err())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mofa_nodes() {
        let yaml = r#"
nodes:
  - id: tts
    operator:
      python: ../../node-hub/dora-primespeech
    outputs:
      - audio
      - log

  - id: mofa-audio-player
    path: dynamic
    inputs:
      audio: tts/audio
    outputs:
      - buffer_status

  - id: mofa-system-log
    path: dynamic
    inputs:
      tts_log: tts/log
"#;

        let parsed = DataflowParser::parse_string(yaml, PathBuf::from("test.yml"))
            .expect("DataflowParser failed to parse a valid test YAML; check parser changes");

        assert_eq!(parsed.mofa_nodes.len(), 2);
        assert_eq!(parsed.mofa_nodes[0].id, "mofa-audio-player");
        assert_eq!(parsed.mofa_nodes[1].id, "mofa-system-log");

        // log_sources includes: tts/log and mofa-audio-player/buffer_status
        assert_eq!(parsed.log_sources.len(), 2);
        assert_eq!(parsed.log_sources[0].node_id, "tts");
        assert_eq!(parsed.log_sources[0].output_id, "log");
        assert_eq!(parsed.log_sources[1].node_id, "mofa-audio-player");
        assert_eq!(parsed.log_sources[1].output_id, "buffer_status");
    }
}
