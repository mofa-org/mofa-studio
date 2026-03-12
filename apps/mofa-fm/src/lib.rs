//! MoFA FM App - AI-powered audio streaming and voice interface

pub mod audio_player;
pub mod dora_integration;
pub mod screen;

pub use dora_integration::{DoraCommand, DoraEvent, DoraIntegration};
// Re-export shared modules from mofa-ui
pub use mofa_ui::{
    AudioDeviceInfo,
    // Audio infrastructure
    AudioManager,
    // MofaHero widget
    ConnectionStatus,
    MofaHero,
    MofaHeroAction,
    MofaHeroRef,
    MofaHeroWidgetExt,
};
pub use screen::MoFaFMScreen;
pub use screen::MoFaFMScreenWidgetRefExt; // Export WidgetRefExt for timer control

use makepad_widgets::{live_id, Cx, LiveId};
use mofa_widgets::{AppInfo, MofaApp};

/// MoFA FM app descriptor
pub struct MoFaFMApp;

impl MofaApp for MoFaFMApp {
    fn info() -> AppInfo {
        AppInfo {
            name: "MoFA FM",
            id: "mofa-fm",
            description: "AI-powered audio streaming and voice interface",
            tab_id: Some(live_id!(mofa_fm_tab)),
            page_id: Some(live_id!(fm_page)),
            show_in_sidebar: true,
            ..Default::default()
        }
    }

    fn live_design(cx: &mut Cx) {
        // Note: mofa_ui::live_design(cx) is called by mofa-studio-shell
        // Apps only need to register their own screen widgets
        screen::live_design(cx);
    }
}

/// Register all MoFA FM widgets with Makepad
/// (Kept for backwards compatibility - calls DoraApp::live_design)
pub fn live_design(cx: &mut Cx) {
    MoFaFMApp::live_design(cx);
}
