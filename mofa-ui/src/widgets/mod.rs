//! Reusable UI Widgets for MoFA Applications
//!
//! This module contains extracted widgets for audio, chat, and configuration.
//!
//! ## Audio Widgets (Phase 2)
//!
//! - [`LedMeter`] - 5-LED horizontal level meter
//! - [`MicButton`] - Microphone toggle with on/off icons
//! - [`AecButton`] - AEC toggle with speaking indicator
//!
//! ## Chat Widgets (Phase 3)
//!
//! - [`ChatPanel`] - Message display with streaming support
//! - [`ChatInput`] - Text input with send button
//! - [`LogPanel`] - Filterable log display panel
//!
//! ## Config Widgets (Phase 4)
//!
//! - [`RoleEditor`] - Role configuration with model/voice/prompt editing
//! - [`DataflowPicker`] - YAML dataflow file selector
//! - [`ProviderSelector`] - AI provider and model selector
//!
//! ## Usage
//!
//! ```rust,ignore
//! use mofa_ui::widgets::*;
//!
//! live_design! {
//!     use mofa_ui::widgets::led_meter::*;
//!     use mofa_ui::widgets::mic_button::*;
//!     use mofa_ui::widgets::aec_button::*;
//!     use mofa_ui::widgets::chat_panel::*;
//!     use mofa_ui::widgets::chat_input::*;
//!     use mofa_ui::widgets::log_panel::*;
//!     use mofa_ui::widgets::role_editor::*;
//!     use mofa_ui::widgets::dataflow_picker::*;
//!     use mofa_ui::widgets::provider_selector::*;
//!
//!     MyApp = <View> {
//!         mic_btn = <MicButton> {}
//!         mic_level = <LedMeter> {}
//!         aec_btn = <AecButton> {}
//!         chat = <ChatPanel> {}
//!         prompt = <ChatInput> {}
//!         logs = <LogPanel> {}
//!         role = <RoleEditor> {}
//!         dataflow = <DataflowPicker> {}
//!         provider = <ProviderSelector> {}
//!     }
//! }
//! ```

// Phase 2 - Audio widgets
pub mod led_meter;
pub mod mic_button;
pub mod aec_button;

// Phase 3 - Chat widgets
pub mod chat_bubble;
pub mod chat_panel;
pub mod chat_input;
pub mod log_panel;

// Phase 4 - Config widgets
pub mod role_editor;
pub mod dataflow_picker;
pub mod provider_selector;

// Phase 5 - Hero widgets
pub mod mofa_hero;

// Re-export Phase 2 widgets
pub use led_meter::{LedMeter, LedMeterRef, LedMeterWidgetExt, LedColors};
pub use mic_button::{MicButton, MicButtonRef, MicButtonWidgetExt, MicButtonAction};
pub use aec_button::{AecButton, AecButtonRef, AecButtonWidgetExt, AecButtonAction};

// Re-export Phase 3 widgets
pub use chat_bubble::{
    ChatBubble, ChatBubbleRef, ChatBubbleWidgetExt,
    ChatBubblePanel, ChatBubblePanelRef, ChatBubblePanelWidgetExt, ChatBubblePanelAction,
    UiChatMessage, UiMessageRole,
};
pub use chat_panel::{ChatPanel, ChatPanelRef, ChatPanelWidgetExt, ChatPanelAction, ChatMessage};
pub use chat_input::{ChatInput, ChatInputRef, ChatInputWidgetExt, ChatInputAction};
pub use log_panel::{MofaLogPanel, MofaLogPanelRef, MofaLogPanelWidgetExt, LogPanelAction, LogLevel, LogNode};

// Re-export Phase 4 widgets
pub use role_editor::{RoleEditor, RoleEditorRef, RoleEditorWidgetExt, RoleEditorAction, RoleConfig};
pub use dataflow_picker::{DataflowPicker, DataflowPickerRef, DataflowPickerWidgetExt, DataflowPickerAction};
pub use provider_selector::{ProviderSelector, ProviderSelectorRef, ProviderSelectorWidgetExt, ProviderSelectorAction, ProviderInfo};

// Re-export Phase 5 widgets (Hero)
pub use mofa_hero::{MofaHero, MofaHeroRef, MofaHeroWidgetExt, MofaHeroAction, ConnectionStatus};

use makepad_widgets::Cx;

/// Register all widget live designs with Makepad.
///
/// Call this from mofa_ui::live_design().
///
/// NOTE: Currently disabled due to Makepad live_design parsing issues.
/// When `link::theme::*` is imported, the parser encounters "Unexpected token #" errors.
/// Apps should define inline widget versions in their own live_design blocks.
pub fn live_design(cx: &mut Cx) {
    // NOTE: Phase 2-4 widget live_design registrations are disabled due to Makepad
    // parser issues. When `link::theme::*` is imported, the parser encounters
    // "Unexpected token #" errors. Apps define inline widget DSL in their own
    // live_design blocks while importing Rust types from mofa-ui.
    //
    // Phase 2 - Audio widgets (inline in app design.rs)
    // led_meter::live_design(cx);
    // mic_button::live_design(cx);
    // aec_button::live_design(cx);
    //
    // Phase 3 - Chat widgets (inline in app design.rs)
    // chat_panel::live_design(cx);
    // chat_input::live_design(cx);
    // log_panel::live_design(cx);
    //
    // Phase 4 - Config widgets (inline in app design.rs)
    // role_editor::live_design(cx);
    // dataflow_picker::live_design(cx);
    // provider_selector::live_design(cx);

    // Phase 5 - Hero widgets (works because it doesn't import link::theme::*)
    mofa_hero::live_design(cx);
}
