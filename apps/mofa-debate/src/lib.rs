//! MoFA Debate App - Multi-agent debate platform

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
pub use screen::MoFaDebateScreen;
pub use screen::MoFaDebateScreenWidgetRefExt; // Export WidgetRefExt for timer control

use makepad_widgets::{live_id, Cx, LiveId};
use mofa_widgets::{AppInfo, MofaApp};

/// MoFA Debate app descriptor
pub struct MoFaDebateApp;

impl MofaApp for MoFaDebateApp {
    fn info() -> AppInfo {
        AppInfo {
            name: "MoFA Debate",
            id: "mofa-debate",
            description: "AI-powered audio streaming and voice interface",
            tab_id: Some(live_id!(debate_tab)),
            page_id: Some(live_id!(debate_page)),
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
    MoFaDebateApp::live_design(cx);
}
