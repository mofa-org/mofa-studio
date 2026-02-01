//! Voice data definitions for TTS (GPT-SoVITS)

use serde::{Deserialize, Serialize};

/// Voice source - distinguishes between built-in and custom voices
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum VoiceSource {
    #[default]
    Builtin,
    Custom,
}

/// Voice information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Voice {
    /// Unique voice ID (matches VOICE_NAME in PrimeSpeech config)
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Voice style/category
    pub category: VoiceCategory,
    /// Language (zh, en)
    pub language: String,
    /// Preview audio file path (optional)
    pub preview_audio: Option<String>,
    /// Voice source (built-in or custom)
    #[serde(default)]
    pub source: VoiceSource,
    /// Reference audio path for custom voices (relative to custom_voices dir)
    #[serde(default)]
    pub reference_audio_path: Option<String>,
    /// Prompt/reference text for zero-shot cloning
    #[serde(default)]
    pub prompt_text: Option<String>,
    /// GPT model weights path (optional, uses default if not set)
    #[serde(default)]
    pub gpt_weights: Option<String>,
    /// SoVITS model weights path (optional, uses default if not set)
    #[serde(default)]
    pub sovits_weights: Option<String>,
    /// Creation timestamp (Unix epoch seconds)
    #[serde(default)]
    pub created_at: Option<u64>,
}

/// Voice category
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VoiceCategory {
    Male,
    Female,
    Character,
}

impl VoiceCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            VoiceCategory::Male => "Male",
            VoiceCategory::Female => "Female",
            VoiceCategory::Character => "Character",
        }
    }
}

/// Get built-in voices for PrimeSpeech (GPT-SoVITS)
/// These match the VOICE_CONFIGS in dora-primespeech/config.py
pub fn get_builtin_voices() -> Vec<Voice> {
    vec![
        // Chinese voices
        Voice {
            id: "Doubao".to_string(),
            name: "豆包 (Doubao)".to_string(),
            description: "Chinese - mixed style, natural and expressive".to_string(),
            category: VoiceCategory::Character,
            language: "zh".to_string(),
            preview_audio: Some("doubao_ref_mix_new.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Luo Xiang".to_string(),
            name: "罗翔 (Luo Xiang)".to_string(),
            description: "Chinese male - law professor, articulate and thoughtful".to_string(),
            category: VoiceCategory::Male,
            language: "zh".to_string(),
            preview_audio: Some("luoxiang_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Yang Mi".to_string(),
            name: "杨幂 (Yang Mi)".to_string(),
            description: "Chinese female - actress, sweet and charming".to_string(),
            category: VoiceCategory::Female,
            language: "zh".to_string(),
            preview_audio: Some("yangmi_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Zhou Jielun".to_string(),
            name: "周杰伦 (Zhou Jielun)".to_string(),
            description: "Chinese male - singer, unique and distinctive".to_string(),
            category: VoiceCategory::Male,
            language: "zh".to_string(),
            preview_audio: Some("zhoujielun_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Ma Yun".to_string(),
            name: "马云 (Ma Yun)".to_string(),
            description: "Chinese male - entrepreneur, confident speaker".to_string(),
            category: VoiceCategory::Male,
            language: "zh".to_string(),
            preview_audio: Some("mayun_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Chen Yifan".to_string(),
            name: "陈一凡 (Chen Yifan)".to_string(),
            description: "Chinese male - analyst, professional tone".to_string(),
            category: VoiceCategory::Male,
            language: "zh".to_string(),
            preview_audio: Some("yfc_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Zhao Daniu".to_string(),
            name: "赵大牛 (Zhao Daniu)".to_string(),
            description: "Chinese male - podcast host, engaging narrator".to_string(),
            category: VoiceCategory::Male,
            language: "zh".to_string(),
            preview_audio: Some("dnz_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "BYS".to_string(),
            name: "BYS".to_string(),
            description: "Chinese - casual and friendly".to_string(),
            category: VoiceCategory::Character,
            language: "zh".to_string(),
            preview_audio: Some("bys_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Ma Baoguo".to_string(),
            name: "马保国 (Ma Baoguo)".to_string(),
            description: "Chinese male - martial arts master, distinctive style".to_string(),
            category: VoiceCategory::Male,
            language: "zh".to_string(),
            preview_audio: Some("mabaoguo_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Shen Yi".to_string(),
            name: "沈逸 (Shen Yi)".to_string(),
            description: "Chinese male - professor, analytical tone".to_string(),
            category: VoiceCategory::Male,
            language: "zh".to_string(),
            preview_audio: Some("shenyi_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        // English voices
        Voice {
            id: "Maple".to_string(),
            name: "Maple".to_string(),
            description: "English female - storyteller, warm and gentle".to_string(),
            category: VoiceCategory::Female,
            language: "en".to_string(),
            preview_audio: Some("maple_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Cove".to_string(),
            name: "Cove".to_string(),
            description: "English male - commentator, clear and professional".to_string(),
            category: VoiceCategory::Male,
            language: "en".to_string(),
            preview_audio: Some("cove_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Ellen".to_string(),
            name: "Ellen".to_string(),
            description: "English female - talk show host, energetic".to_string(),
            category: VoiceCategory::Female,
            language: "en".to_string(),
            preview_audio: Some("ellen_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Juniper".to_string(),
            name: "Juniper".to_string(),
            description: "English female - narrator, calm and soothing".to_string(),
            category: VoiceCategory::Female,
            language: "en".to_string(),
            preview_audio: Some("juniper_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
        Voice {
            id: "Trump".to_string(),
            name: "Trump".to_string(),
            description: "English male - distinctive speaking style".to_string(),
            category: VoiceCategory::Male,
            language: "en".to_string(),
            preview_audio: Some("trump_ref.wav".to_string()),
            source: VoiceSource::Builtin,
            reference_audio_path: None,
            prompt_text: None,
            gpt_weights: None,
            sovits_weights: None,
            created_at: None,
        },
    ]
}

/// TTS generation status
#[derive(Clone, Debug, PartialEq)]
pub enum TTSStatus {
    Idle,
    Generating,
    Ready,
    Playing,
    Error(String),
}

impl Default for TTSStatus {
    fn default() -> Self {
        TTSStatus::Idle
    }
}

/// Voice cloning status
#[derive(Clone, Debug, PartialEq)]
pub enum CloningStatus {
    Idle,
    ValidatingAudio,
    CopyingFiles,
    SavingConfig,
    Completed,
    Error(String),
}

impl Default for CloningStatus {
    fn default() -> Self {
        CloningStatus::Idle
    }
}

impl Voice {
    /// Create a new custom voice
    pub fn new_custom(
        id: String,
        name: String,
        language: String,
        reference_audio_path: String,
        prompt_text: String,
    ) -> Self {
        Self {
            id,
            name: name.clone(),
            description: format!("Custom voice - {}", name),
            category: VoiceCategory::Character,
            language,
            preview_audio: Some(reference_audio_path.clone()),
            source: VoiceSource::Custom,
            reference_audio_path: Some(reference_audio_path),
            prompt_text: Some(prompt_text),
            gpt_weights: None,
            sovits_weights: None,
            created_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            ),
        }
    }

    /// Check if this is a custom voice
    pub fn is_custom(&self) -> bool {
        self.source == VoiceSource::Custom
    }
}
