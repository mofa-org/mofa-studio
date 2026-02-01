//! MoFA TTS App - Text to Speech using GPT-SoVITS with voice cloning

// Local modules
pub mod audio_player; // Keep local: simplified TTS-specific version
pub mod dora_integration;
pub mod screen;
pub mod voice_clone_modal;
pub mod voice_data;
pub mod voice_persistence;
pub mod voice_selector;

// Re-export shared components from mofa-ui
pub use mofa_ui::log_bridge;
pub use mofa_ui::system_monitor;
pub use mofa_ui::widgets::mofa_hero::{self, ConnectionStatus, MofaHero, MofaHeroAction};

pub use screen::TTSScreenRef;
pub use screen::TTSScreenWidgetRefExt;

use makepad_widgets::Cx;
use mofa_widgets::{AppInfo, MofaApp};

/// MoFA TTS app descriptor
pub struct MoFaTTSApp;

impl MofaApp for MoFaTTSApp {
    fn info() -> AppInfo {
        AppInfo {
            name: "TTS",
            id: "mofa-tts",
            description: "GPT-SoVITS Text to Speech with voice cloning",
            ..Default::default()
        }
    }

    fn live_design(cx: &mut Cx) {
        // Note: Shared components (mofa_hero, system_monitor, log_bridge) are already
        // registered by mofa_ui::live_design(cx) in the shell, so we only register
        // app-specific components here.

        voice_selector::live_design(cx);
        voice_clone_modal::live_design(cx);
        screen::live_design(cx);
    }
}

/// Register all TTS widgets with Makepad
pub fn live_design(cx: &mut Cx) {
    MoFaTTSApp::live_design(cx);
}
