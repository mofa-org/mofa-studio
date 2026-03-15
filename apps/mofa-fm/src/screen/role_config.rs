//! Role configuration handling - load and save TOML config files
//!
//! Also supports updating VOICE_CHARACTER/VOICE_NAME in YAML dataflow files.

use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;

/// Available voice display names (shown in UI)
pub const VOICE_OPTIONS: &[&str] = &[
    "Zhao Daniu",
    "Chen Yifan",
    "Luo Xiang",
    "Doubao",
    "Yang Mi",
    "Ma Yun",
    "BYS",
    "Marc",
];

/// Map display name → VOICE_CHARACTER ID (used in voices.json / YAML env)
pub fn voice_display_to_id(display_name: &str) -> &str {
    match display_name {
        "Zhao Daniu" => "dnz",
        "Chen Yifan" => "yfc",
        "Luo Xiang" => "luoxiang",
        "Doubao" => "doubao",
        "Yang Mi" => "yangmi",
        "Ma Yun" => "mayun",
        "BYS" => "bys",
        "Marc" => "marc",
        _ => "doubao",
    }
}

/// Map VOICE_CHARACTER ID → display name (for UI)
pub fn voice_id_to_display(id: &str) -> &str {
    match id {
        "dnz" | "daniu" | "zhaodaniu" => "Zhao Daniu",
        "yfc" | "yifan" | "chenyifan" => "Chen Yifan",
        "luoxiang" | "luo" => "Luo Xiang",
        "doubao" | "default" => "Doubao",
        "yangmi" | "yang" => "Yang Mi",
        "mayun" | "ma" => "Ma Yun",
        "bys" => "BYS",
        "marc" | "fewshot" => "Marc",
        _ => "Doubao",
    }
}

/// Role configuration loaded from TOML file
#[derive(Debug, Clone, Default)]
pub struct RoleConfig {
    pub default_model: String,
    pub system_prompt: String,
    pub voice: String,
    pub models: Vec<String>,
    pub config_path: Option<PathBuf>,
}

/// Model definition from TOML
#[derive(Debug, Deserialize)]
struct TomlModel {
    id: String,
    #[allow(dead_code)]
    route: Option<TomlRoute>,
}

#[derive(Debug, Deserialize)]
struct TomlRoute {
    #[allow(dead_code)]
    provider: String,
    #[allow(dead_code)]
    model: String,
}

/// Partial TOML structure for reading
#[derive(Debug, Deserialize)]
struct TomlConfig {
    default_model: Option<String>,
    system_prompt: Option<String>,
    voice: Option<String>,
    models: Option<Vec<TomlModel>>,
}

impl RoleConfig {
    /// Load configuration from a TOML file
    pub fn load(path: &PathBuf) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let config: TomlConfig =
            toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {}", e))?;

        let models = config
            .models
            .map(|m| m.into_iter().map(|model| model.id).collect())
            .unwrap_or_default();

        Ok(RoleConfig {
            default_model: config.default_model.unwrap_or_default(),
            system_prompt: config.system_prompt.unwrap_or_default(),
            voice: config.voice.unwrap_or_else(|| "Zhao Daniu".to_string()),
            models,
            config_path: Some(path.clone()),
        })
    }

    /// Save model, system prompt, and voice back to the TOML file
    /// Uses regex-based replacement to preserve file formatting and comments
    pub fn save(&self) -> Result<(), String> {
        let path = self
            .config_path
            .as_ref()
            .ok_or_else(|| "No config path set".to_string())?;

        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        // Update default_model using regex (single-line value)
        let model_re = Regex::new(r#"(?m)^default_model\s*=\s*"[^"]*""#)
            .map_err(|e| format!("Invalid regex: {}", e))?;
        let content = model_re.replace(
            &content,
            format!(r#"default_model = "{}""#, self.default_model),
        );

        // Update voice using regex (single-line value)
        let voice_re = Regex::new(r#"(?m)^voice\s*=\s*"[^"]*""#)
            .map_err(|e| format!("Invalid regex: {}", e))?;
        let content = voice_re.replace(&content, format!(r#"voice = "{}""#, self.voice));

        // Update system_prompt using regex (multiline triple-quoted string)
        // Match: system_prompt = """...""" (with newlines inside)
        let prompt_re = Regex::new(r#"(?ms)^system_prompt\s*=\s*""".*?""""#)
            .map_err(|e| format!("Invalid regex: {}", e))?;
        let content = prompt_re.replace(
            &content,
            format!(r#"system_prompt = """{}""""#, self.system_prompt),
        );

        std::fs::write(path, content.as_ref())
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        Ok(())
    }
}

/// Update VOICE_CHARACTER in a YAML dataflow file for a specific role
///
/// This finds the primespeech node for the role and updates its VOICE_CHARACTER env variable.
/// Uses regex replacement to preserve YAML formatting and comments.
///
/// # Arguments
/// * `yaml_path` - Path to the YAML dataflow file
/// * `role` - Role name (student1, student2, tutor)
/// * `voice` - Voice display name (e.g., "Zhao Daniu") — converted to ID automatically
///
/// # Returns
/// * `Ok(true)` - Voice was updated
/// * `Ok(false)` - Node not found (no update needed)
/// * `Err(String)` - Error occurred
pub fn update_yaml_voice(yaml_path: &PathBuf, role: &str, voice: &str) -> Result<bool, String> {
    let content = std::fs::read_to_string(yaml_path)
        .map_err(|e| format!("Failed to read YAML file: {}", e))?;

    // Convert display name to voice character ID
    let voice_id = voice_display_to_id(voice);

    // Map role to primespeech node id
    let node_id = match role {
        "student1" => "primespeech-student1",
        "student2" => "primespeech-student2",
        "tutor" => "primespeech-tutor",
        _ => return Err(format!("Unknown role: {}", role)),
    };

    // Find the node section boundaries
    let node_pattern = format!(r"(?m)^  - id: {}\s*$", regex::escape(node_id));
    let node_re = Regex::new(&node_pattern).map_err(|e| format!("Invalid regex: {}", e))?;

    let node_match = match node_re.find(&content) {
        Some(m) => m,
        None => return Ok(false), // Node not found
    };

    let node_start = node_match.start();

    // Find the end of this node (next "  - id:" or end of file)
    let next_node_re = Regex::new(r"(?m)^  - id: ").map_err(|e| format!("Invalid regex: {}", e))?;

    let node_end = next_node_re
        .find_iter(&content[node_match.end()..])
        .next()
        .map(|m| node_match.end() + m.start())
        .unwrap_or(content.len());

    // Extract the node section
    let node_section = &content[node_start..node_end];

    // Try VOICE_CHARACTER first (gpt-sovits-mlx), then fall back to VOICE_NAME (primespeech)
    let vc_re = Regex::new(r#"(?m)(^\s*VOICE_CHARACTER:\s*)["']?[^"'\n]*["']?\s*$"#)
        .map_err(|e| format!("Invalid regex: {}", e))?;
    let vn_re = Regex::new(r#"(?m)(^\s*VOICE_NAME:\s*)["']?[^"'\n]*["']?\s*$"#)
        .map_err(|e| format!("Invalid regex: {}", e))?;

    let (voice_re, value) = if vc_re.is_match(node_section) {
        (vc_re, voice_id)
    } else if vn_re.is_match(node_section) {
        (vn_re, voice)
    } else {
        return Err(format!(
            "VOICE_CHARACTER/VOICE_NAME not found in {} section",
            node_id
        ));
    };

    // Build replacement string: preserve prefix ($1) and add new quoted voice
    let replacement = format!("$1\"{}\"", value);
    let new_node_section = voice_re.replace(node_section, replacement.as_str());

    // Rebuild the full content
    let new_content = format!(
        "{}{}{}",
        &content[..node_start],
        new_node_section,
        &content[node_end..]
    );

    // Write back
    std::fs::write(yaml_path, new_content)
        .map_err(|e| format!("Failed to write YAML file: {}", e))?;

    Ok(true)
}

/// Read voice setting from a YAML dataflow file for a specific role
///
/// Tries VOICE_CHARACTER first (gpt-sovits-mlx format, returns ID converted to display name),
/// then falls back to VOICE_NAME (primespeech format, returns display name as-is).
/// Returns the voice display name if found, or None if not found.
pub fn read_yaml_voice(yaml_path: &PathBuf, role: &str) -> Option<String> {
    let content = std::fs::read_to_string(yaml_path).ok()?;

    // Map role to primespeech node id
    let node_id = match role {
        "student1" => "primespeech-student1",
        "student2" => "primespeech-student2",
        "tutor" => "primespeech-tutor",
        _ => return None,
    };

    // Find the node section
    let node_pattern = format!(r"(?m)^  - id: {}\s*$", regex::escape(node_id));
    let node_re = Regex::new(&node_pattern).ok()?;
    let node_match = node_re.find(&content)?;
    let node_start = node_match.start();

    // Find the end of this node (next "  - id:" or end of file)
    let next_node_re = Regex::new(r"(?m)^  - id: ").ok()?;
    let node_end = next_node_re
        .find_iter(&content[node_match.end()..])
        .next()
        .map(|m| node_match.end() + m.start())
        .unwrap_or(content.len());

    // Extract the node section
    let node_section = &content[node_start..node_end];

    // Try VOICE_CHARACTER first (gpt-sovits-mlx) — value is an ID, convert to display name
    let vc_re = Regex::new(r#"(?m)^\s*VOICE_CHARACTER:\s*["']?([^"'\n]+)["']?\s*$"#).ok()?;
    if let Some(caps) = vc_re.captures(node_section) {
        let id = caps.get(1)?.as_str().trim();
        return Some(voice_id_to_display(id).to_string());
    }

    // Fall back to VOICE_NAME (primespeech) — value is already a display name
    let vn_re = Regex::new(r#"(?m)^\s*VOICE_NAME:\s*["']?([^"'\n]+)["']?\s*$"#).ok()?;
    let caps = vn_re.captures(node_section)?;
    let voice = caps.get(1)?.as_str().trim().to_string();

    Some(voice)
}

/// Get the YAML dataflow path, searching common locations
pub fn get_yaml_path(dataflow_path: Option<&PathBuf>) -> Option<PathBuf> {
    // Try to use the provided dataflow_path
    if let Some(path) = dataflow_path {
        if path.exists() {
            return Some(path.clone());
        }
    }

    // Fallback: search common locations
    let cwd = std::env::current_dir().ok()?;

    // First try: apps/mofa-fm/dataflow/voice-chat.yml (workspace root)
    let app_path = cwd
        .join("apps")
        .join("mofa-fm")
        .join("dataflow")
        .join("voice-chat.yml");
    if app_path.exists() {
        return Some(app_path);
    }

    // Second try: dataflow/voice-chat.yml (run from app directory)
    let local_path = cwd.join("dataflow").join("voice-chat.yml");
    if local_path.exists() {
        return Some(local_path);
    }

    None
}

/// Get the config file path for a role
pub fn get_role_config_path(dataflow_path: Option<&PathBuf>, role: &str) -> PathBuf {
    let config_name = match role {
        "student1" => "study_config_student1.toml",
        "student2" => "study_config_student2.toml",
        "tutor" => "study_config_tutor.toml",
        _ => "study_config_student1.toml",
    };

    // Try to use the dataflow_path if set
    if let Some(dataflow_path) = dataflow_path {
        if let Some(parent) = dataflow_path.parent() {
            let config_path = parent.join(config_name);
            if config_path.exists() {
                return config_path;
            }
        }
    }

    // Fallback: search common locations
    let cwd = std::env::current_dir().unwrap_or_default();

    // First try: apps/mofa-fm/dataflow/ (workspace root)
    let app_path = cwd
        .join("apps")
        .join("mofa-fm")
        .join("dataflow")
        .join(config_name);
    if app_path.exists() {
        return app_path;
    }

    // Second try: dataflow/ (run from app directory)
    let local_path = cwd.join("dataflow").join(config_name);
    if local_path.exists() {
        return local_path;
    }

    // Default
    app_path
}
