//! Qwen3-ASR engine using qwen3-asr-mlx
//!
//! This engine wraps the qwen3-asr-mlx library for multilingual ASR on Apple Silicon.
//! Supports 30+ languages including Chinese, English, Japanese, Korean, French, German, etc.

use eyre::{Context, Result};
use std::path::Path;

use qwen3_asr_mlx::Qwen3ASR;

/// Qwen3-ASR engine for multilingual speech recognition
pub struct Qwen3AsrEngine {
    model: Qwen3ASR,
    language: String,
}

impl Qwen3AsrEngine {
    /// Create a new Qwen3-ASR engine
    ///
    /// # Arguments
    /// * `model_dir` - Path to model directory containing config.json + safetensors weights
    /// * `language` - Default language for transcription (e.g., "Chinese", "English")
    pub fn new(model_dir: impl AsRef<Path>, language: &str) -> Result<Self> {
        let model_dir = model_dir.as_ref();

        log::info!("Loading Qwen3-ASR model from {:?}", model_dir);

        let model = Qwen3ASR::load(model_dir)
            .map_err(|e| eyre::eyre!("Failed to load Qwen3-ASR: {:?}", e))?;

        log::info!("Qwen3-ASR model loaded successfully (language: {})", language);

        Ok(Self {
            model,
            language: language.to_string(),
        })
    }

    /// Transcribe audio samples to text with a specific language
    ///
    /// # Arguments
    /// * `samples` - Audio samples as f32 slice (16kHz, mono, normalized to [-1, 1])
    /// * `language` - Language for transcription (e.g., "Chinese", "English")
    ///
    /// # Returns
    /// Transcribed text
    pub fn transcribe_with_language(&mut self, samples: &[f32], language: &str) -> Result<String> {
        self.model
            .transcribe_samples(samples, language)
            .map_err(|e| eyre::eyre!("Qwen3-ASR transcription failed: {:?}", e))
            .context("Transcription failed")
    }

    /// Get the configured language
    pub fn language(&self) -> &str {
        &self.language
    }
}
