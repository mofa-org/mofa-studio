//! Widget-specific bridge implementations
//!
//! Each widget type has its own bridge that connects to dora as a dynamic node:
//! - `mofa-audio-player`: Receives audio, forwards to UI for playback
//! - `mofa-system-log`: Receives logs from multiple nodes
//! - `mofa-prompt-input`: Sends user prompts to LLM
//! - `mofa-aec-input`: Captures mic audio with AEC, sends to ASR
//! - `mofa-audio-input`: Sends pre-recorded audio to ASR
//!
//! Note: LED visualization is calculated in screen.rs from output waveform
//! (more accurate since it reflects what's actually being played)

mod aec_input;
mod asr_listener;
mod audio_input;
mod audio_player;
mod prompt_input;
mod system_log;

pub use aec_input::{AecControlCommand, AecInputBridge};
pub use asr_listener::AsrListenerBridge;
pub use audio_input::AudioInputBridge;
pub use audio_player::AudioPlayerBridge;
pub use prompt_input::PromptInputBridge;
pub use system_log::SystemLogBridge;
