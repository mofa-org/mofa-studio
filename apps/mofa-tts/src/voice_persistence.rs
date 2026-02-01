//! Voice persistence module for saving/loading custom voices
//!
//! Custom voices are stored in:
//! - Config: ~/.dora/primespeech/custom_voices.json
//! - Audio: ~/.dora/primespeech/custom_voices/{voice_id}/ref.wav

use crate::voice_data::{Voice, VoiceSource};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Custom voices configuration file format
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomVoicesConfig {
    /// Config version for future compatibility
    pub version: String,
    /// List of custom voices
    pub voices: Vec<Voice>,
}

impl Default for CustomVoicesConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            voices: Vec::new(),
        }
    }
}

/// Get the base directory for PrimeSpeech data
pub fn get_primespeech_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".dora").join("primespeech")
}

/// Get the custom voices config file path
pub fn get_config_path() -> PathBuf {
    get_primespeech_dir().join("custom_voices.json")
}

/// Get the custom voices audio directory
pub fn get_custom_voices_dir() -> PathBuf {
    get_primespeech_dir().join("custom_voices")
}

/// Get the directory for a specific custom voice
pub fn get_voice_dir(voice_id: &str) -> PathBuf {
    get_custom_voices_dir().join(voice_id)
}

/// Ensure all required directories exist
pub fn ensure_directories() -> std::io::Result<()> {
    let primespeech_dir = get_primespeech_dir();
    if !primespeech_dir.exists() {
        fs::create_dir_all(&primespeech_dir)?;
    }

    let custom_voices_dir = get_custom_voices_dir();
    if !custom_voices_dir.exists() {
        fs::create_dir_all(&custom_voices_dir)?;
    }

    Ok(())
}

/// Load custom voices from config file
pub fn load_custom_voices() -> Vec<Voice> {
    let config_path = get_config_path();

    if !config_path.exists() {
        return Vec::new();
    }

    match fs::read_to_string(&config_path) {
        Ok(content) => match serde_json::from_str::<CustomVoicesConfig>(&content) {
            Ok(config) => config.voices,
            Err(e) => {
                log::error!("Failed to parse custom voices config: {}", e);
                Vec::new()
            }
        },
        Err(e) => {
            log::error!("Failed to read custom voices config: {}", e);
            Vec::new()
        }
    }
}

/// Save custom voices to config file
pub fn save_custom_voices(voices: &[Voice]) -> Result<(), String> {
    ensure_directories().map_err(|e| format!("Failed to create directories: {}", e))?;

    let config = CustomVoicesConfig {
        version: "1.0".to_string(),
        voices: voices.to_vec(),
    };

    let config_path = get_config_path();
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, json).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

/// Add a new custom voice
pub fn add_custom_voice(voice: Voice) -> Result<(), String> {
    let mut voices = load_custom_voices();

    // Check for duplicate ID
    if voices.iter().any(|v| v.id == voice.id) {
        return Err(format!("Voice with ID '{}' already exists", voice.id));
    }

    voices.push(voice);
    save_custom_voices(&voices)
}

/// Remove a custom voice by ID
pub fn remove_custom_voice(voice_id: &str) -> Result<(), String> {
    let mut voices = load_custom_voices();
    let original_len = voices.len();

    voices.retain(|v| v.id != voice_id);

    if voices.len() == original_len {
        return Err(format!("Voice with ID '{}' not found", voice_id));
    }

    // Delete the voice directory
    let voice_dir = get_voice_dir(voice_id);
    if voice_dir.exists() {
        fs::remove_dir_all(&voice_dir)
            .map_err(|e| format!("Failed to delete voice directory: {}", e))?;
    }

    save_custom_voices(&voices)
}

/// Update a custom voice
pub fn update_custom_voice(voice: Voice) -> Result<(), String> {
    let mut voices = load_custom_voices();

    if let Some(existing) = voices.iter_mut().find(|v| v.id == voice.id) {
        *existing = voice;
        save_custom_voices(&voices)
    } else {
        Err(format!("Voice with ID '{}' not found", voice.id))
    }
}

/// Rename a custom voice
pub fn rename_custom_voice(voice_id: &str, new_name: &str) -> Result<(), String> {
    let mut voices = load_custom_voices();

    if let Some(voice) = voices.iter_mut().find(|v| v.id == voice_id) {
        voice.name = new_name.to_string();
        save_custom_voices(&voices)
    } else {
        Err(format!("Voice with ID '{}' not found", voice_id))
    }
}

/// Copy reference audio file to custom voice directory
pub fn copy_reference_audio(voice_id: &str, source_path: &PathBuf) -> Result<String, String> {
    ensure_directories().map_err(|e| format!("Failed to create directories: {}", e))?;

    let voice_dir = get_voice_dir(voice_id);
    if !voice_dir.exists() {
        fs::create_dir_all(&voice_dir)
            .map_err(|e| format!("Failed to create voice directory: {}", e))?;
    }

    let dest_path = voice_dir.join("ref.wav");
    fs::copy(source_path, &dest_path).map_err(|e| format!("Failed to copy audio file: {}", e))?;

    // Return the relative path from custom_voices dir
    Ok(format!("{}/ref.wav", voice_id))
}

/// Validate audio file for voice cloning
///
/// Hard limits (rejected):
/// - Duration < 1 second: Too short for voice cloning
/// - Duration > 30 seconds: Too long, may degrade quality
///
/// Recommended range (warnings):
/// - Duration 3-10 seconds: Optimal for voice cloning quality
///
/// Returns AudioInfo with validation warnings, or Err if hard limits exceeded.
pub fn validate_audio_file(path: &PathBuf) -> Result<AudioInfo, String> {
    use hound::WavReader;

    if !path.exists() {
        return Err("File does not exist".to_string());
    }

    let reader = WavReader::open(path).map_err(|e| format!("Failed to open WAV file: {}", e))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels;
    let bits_per_sample = spec.bits_per_sample;

    // Calculate duration
    let num_samples = reader.len() as f32;
    let duration_secs = num_samples / (sample_rate as f32 * channels as f32);

    // Validate duration with hard limits and warnings
    // Hard limits: reject files < 1s or > 30s (unusable for voice cloning)
    // Recommended: 3-10 seconds for optimal quality
    const MIN_DURATION_HARD: f32 = 1.0;
    const MAX_DURATION_HARD: f32 = 30.0;
    const MIN_DURATION_RECOMMENDED: f32 = 3.0;
    const MAX_DURATION_RECOMMENDED: f32 = 10.0;

    // Hard limit validation - reject immediately
    if duration_secs < MIN_DURATION_HARD {
        return Err(format!(
            "Audio too short for voice cloning: {:.1}s (minimum: {}s)",
            duration_secs, MIN_DURATION_HARD
        ));
    }
    if duration_secs > MAX_DURATION_HARD {
        return Err(format!(
            "Audio too long for voice cloning: {:.1}s (maximum: {}s)",
            duration_secs, MAX_DURATION_HARD
        ));
    }

    // Soft limit validation - warn but allow
    let mut warnings = Vec::new();
    if duration_secs < MIN_DURATION_RECOMMENDED {
        warnings.push(format!(
            "Audio shorter than recommended ({:.1}s < {}s) - may affect cloning quality",
            duration_secs, MIN_DURATION_RECOMMENDED
        ));
    }
    if duration_secs > MAX_DURATION_RECOMMENDED {
        warnings.push(format!(
            "Audio longer than recommended ({:.1}s > {}s) - may affect cloning quality",
            duration_secs, MAX_DURATION_RECOMMENDED
        ));
    }

    Ok(AudioInfo {
        duration_secs,
        sample_rate,
        channels,
        bits_per_sample,
        warnings,
    })
}

/// Audio file information
#[derive(Clone, Debug)]
pub struct AudioInfo {
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub warnings: Vec<String>,
}

/// Generate a unique voice ID from a name
///
/// Combines sanitized name + timestamp + random suffix to ensure uniqueness
/// even when multiple voices are created in the same second.
pub fn generate_voice_id(name: &str) -> String {
    use rand::Rng;

    // Sanitize the name for use as an ID
    let base_id: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();

    let base_id = if base_id.is_empty() {
        "custom_voice".to_string()
    } else {
        base_id
    };

    // Add timestamp for temporal ordering
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Add random suffix (4 alphanumeric chars) to prevent collisions
    let random_suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(4)
        .map(char::from)
        .collect();

    format!("{}_{}_{}", base_id, timestamp, random_suffix.to_lowercase())
}

/// Get the full path to a custom voice's reference audio
pub fn get_reference_audio_path(voice: &Voice) -> Option<PathBuf> {
    if voice.source != VoiceSource::Custom {
        return None;
    }

    voice.reference_audio_path.as_ref().map(|rel_path| {
        get_custom_voices_dir().join(rel_path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_voice_id() {
        let id = generate_voice_id("My Custom Voice");
        assert!(id.starts_with("my_custom_voice_"));
    }

    #[test]
    fn test_voice_id_with_chinese() {
        let id = generate_voice_id("我的声音");
        assert!(id.starts_with("____")); // Chinese chars become underscores
    }
}
