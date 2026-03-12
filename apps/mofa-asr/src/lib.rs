//! MoFA ASR App - Dedicated ASR testing and transcription interface
//!
//! Supports two MLX-based ASR engines:
//! - Paraformer: Chinese only, ~60x real-time
//! - SenseVoice: Multi-language (zh/en/ja), ~3x real-time

pub mod dora_integration;
#[cfg(not(test))]
pub mod screen;

pub use dora_integration::{DoraCommand, DoraEvent, DoraIntegration};
// Re-export shared modules from mofa-ui
pub use mofa_ui::{
    // MofaHero widget
    ConnectionStatus, MofaHero, MofaHeroAction, MofaHeroRef, MofaHeroWidgetExt,
    // Audio infrastructure
    AudioManager, AudioDeviceInfo,
};
#[cfg(not(test))]
pub use screen::MoFaASRScreen;
#[cfg(not(test))]
pub use screen::MoFaASRScreenRef;
#[cfg(not(test))]
pub use screen::MoFaASRScreenWidgetRefExt;

use makepad_widgets::{Cx, live_id, LiveId};
use mofa_widgets::{AppInfo, MofaApp};

/// MoFA ASR app descriptor
pub struct MoFaASRApp;

impl MofaApp for MoFaASRApp {
    fn info() -> AppInfo {
        AppInfo {
            name: "MoFA ASR",
            id: "mofa-asr",
            description: "Speech recognition with MLX-based ASR engines",
            tab_id: Some(live_id!(mofa_asr_tab)),
            page_id: Some(live_id!(asr_page)),
            show_in_sidebar: true,
            ..Default::default()
        }
    }

    fn live_design(cx: &mut Cx) {
        #[cfg(not(test))]
        {
        // Shared widgets (LedMeter, MicButton, AecButton, ChatPanel, MofaLogPanel)
        // are registered centrally by mofa_ui::widgets::live_design(cx) in the shell.
        screen::live_design(cx);
        }

        #[cfg(test)]
        {
            let _ = cx;
        }
    }
}

/// Register all MoFA ASR widgets with Makepad
pub fn live_design(cx: &mut Cx) {
    MoFaASRApp::live_design(cx);
}
