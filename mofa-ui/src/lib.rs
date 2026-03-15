//! # MoFA UI - Shared Component Library
//!
//! Reusable UI components, shell layouts, and infrastructure for MoFA Studio applications.
//!
//! ## Overview
//!
//! This crate provides:
//!
//! - **Widget Registry** - Dynamic widget discovery and registration
//! - **App Data** - Scope-based state injection for widgets
//! - **Theme** - Runtime dark mode management with animations
//! - **Traits** - Common widget interfaces for consistency
//! - **Widgets** - Reusable UI components (audio, chat, config)
//! - **Shell** - Application shell layouts
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use mofa_ui::{MofaAppData, MofaTheme, MofaWidgetRegistry};
//! use mofa_dora_bridge::SharedDoraState;
//! use std::sync::Arc;
//!
//! // 1. Create shared state
//! let dora_state = SharedDoraState::new();
//!
//! // 2. Create app data for scope injection
//! let app_data = MofaAppData::new(dora_state);
//!
//! // 3. Register widgets in live_design
//! impl LiveRegister for MyApp {
//!     fn live_register(cx: &mut Cx) {
//!         mofa_ui::live_design(cx);
//!         // ... your app widgets
//!     }
//! }
//!
//! // 4. Pass app_data through scope
//! impl AppMain for MyApp {
//!     fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
//!         self.ui.handle_event(cx, event, &mut Scope::with_data(&mut self.app_data));
//!     }
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                       mofa-ui                               │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │  Registry   │  │  App Data   │  │       Theme         │  │
//! │  │ (discover)  │  │ (inject)    │  │  (dark mode anim)   │  │
//! │  └─────────────┘  └─────────────┘  └─────────────────────┘  │
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │                       Traits                            ││
//! │  │  Themeable | DoraConnected | Maximizable | Clearable   ││
//! │  └─────────────────────────────────────────────────────────┘│
//! │  ┌─────────────────────────┐  ┌─────────────────────────┐  │
//! │  │        Widgets          │  │         Shell           │  │
//! │  │  AudioControls          │  │  MofaShell              │  │
//! │  │  ChatPanel              │  │  ShellSidebar           │  │
//! │  │  RoleEditor             │  │  StatusBar              │  │
//! │  └─────────────────────────┘  └─────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                           │
//!                           ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    mofa-dora-bridge                         │
//! │  SharedDoraState | ChatState | AudioState | MicState        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`registry`] | Widget discovery and registration |
//! | [`app_data`] | Scope-based state injection |
//! | [`theme`] | Runtime dark mode management |
//! | [`traits`] | Common widget interfaces |
//! | [`widgets`] | Reusable UI components |
//! | [`shell`] | Application shell layouts |
//!
//! ## Development Phases
//!
//! This crate is being developed incrementally:
//!
//! - **Phase 1** (Current): Foundation - Registry, AppData, Theme, Traits
//! - **Phase 2**: Audio widgets extraction from mofa-fm
//! - **Phase 3**: Chat/Log widgets extraction
//! - **Phase 4**: Config widgets extraction
//! - **Phase 5**: Shell components
//! - **Phase 6**: Validation with new app

pub mod registry;
pub mod app_data;
pub mod theme;
pub mod traits;
pub mod widgets;
pub mod shell;
pub mod system_monitor;
pub mod audio;
pub mod log_bridge;

// Re-export main types for convenience
pub use registry::{MofaWidgetRegistry, MofaWidgetDef, WidgetCategory, WidgetSize};
pub use app_data::{MofaAppData, AppConfig};
pub use theme::{MofaTheme, ThemeColor, ThemeListener, THEME_TRANSITION_DURATION};
pub use traits::{MofaWidget, Themeable, DoraConnected, Maximizable, Clearable, Animated, Focusable};

// Re-export shared infrastructure
pub use audio::{AudioManager, AudioDeviceInfo, MicLevelState};
pub use log_bridge::{LogMessage, init as log_bridge_init, poll_logs, receiver as log_receiver};

// Re-export widgets and their WidgetExt traits
pub use widgets::{
    // Audio widgets (Phase 2)
    LedMeter, LedMeterRef, LedMeterWidgetExt, LedColors,
    MicButton, MicButtonRef, MicButtonWidgetExt, MicButtonAction,
    AecButton, AecButtonRef, AecButtonWidgetExt, AecButtonAction,
    // Chat widgets (Phase 3)
    ChatBubble, ChatBubbleRef, ChatBubbleWidgetExt,
    ChatBubblePanel, ChatBubblePanelRef, ChatBubblePanelWidgetExt, ChatBubblePanelAction,
    UiChatMessage, UiMessageRole,
    ChatPanel, ChatPanelRef, ChatPanelWidgetExt, ChatPanelAction, ChatMessage,
    ChatInput, ChatInputRef, ChatInputWidgetExt, ChatInputAction,
    MofaLogPanel, MofaLogPanelRef, MofaLogPanelWidgetExt, LogPanelAction, LogLevel, LogNode,
    // Config widgets (Phase 4)
    RoleEditor, RoleEditorRef, RoleEditorWidgetExt, RoleEditorAction, RoleConfig,
    DataflowPicker, DataflowPickerRef, DataflowPickerWidgetExt, DataflowPickerAction,
    ProviderSelector, ProviderSelectorRef, ProviderSelectorWidgetExt, ProviderSelectorAction, ProviderInfo,
    // Hero widgets (Phase 5)
    MofaHero, MofaHeroRef, MofaHeroWidgetExt, MofaHeroAction, ConnectionStatus,
};

// Re-export shell components (Phase 5)
pub use shell::{
    MofaShell, MofaShellRef, MofaShellWidgetExt, MofaShellAction,
    ShellHeader, ShellHeaderRef, ShellHeaderWidgetExt, ShellHeaderAction,
    ShellSidebar, ShellSidebarRef, ShellSidebarWidgetExt, ShellSidebarAction,
    StatusBar, StatusBarRef, StatusBarWidgetExt, StatusBarAction,
    SidebarItem,
};

use makepad_widgets::Cx;

/// Register all mofa-ui widgets and components with Makepad.
///
/// Call this in your app's `LiveRegister::live_register` implementation
/// before registering app-specific widgets.
///
/// # Example
///
/// ```rust,ignore
/// impl LiveRegister for MyApp {
///     fn live_register(cx: &mut Cx) {
///         mofa_ui::live_design(cx);  // Register mofa-ui first
///         my_app::live_design(cx);   // Then app-specific widgets
///     }
/// }
/// ```
pub fn live_design(cx: &mut Cx) {
    // NOTE: mofa_widgets::live_design(cx) must be called BEFORE this function
    // by the app (e.g., in App::live_register). mofa-ui widgets use theme
    // colors via `use mofa_widgets::theme::*` in their live_design! blocks.

    // Register mofa-ui widgets
    widgets::live_design(cx);
    // Register shell components (Phase 5)
    shell::live_design(cx);
}

/// Create a default widget registry with standard mofa-ui widgets.
///
/// # Example
///
/// ```rust,ignore
/// let registry = mofa_ui::create_default_registry();
///
/// // Access widget definitions
/// for widget in registry.all() {
///     println!("{}: {}", widget.id, widget.title);
/// }
/// ```
pub fn create_default_registry() -> MofaWidgetRegistry {
    let mut registry = MofaWidgetRegistry::new();

    // Register audio widgets (Phase 2)
    registry.register(
        MofaWidgetDef::new("led_meter", "LED Meter", WidgetCategory::Audio)
            .description("5-LED horizontal level meter for audio visualization")
    );
    registry.register(
        MofaWidgetDef::new("mic_button", "Mic Button", WidgetCategory::Audio)
            .description("Microphone toggle button with on/off icons")
    );
    registry.register(
        MofaWidgetDef::new("aec_button", "AEC Button", WidgetCategory::Audio)
            .requires_dora(true)
            .description("AEC toggle with animated speaking indicator")
    );

    // Register chat widgets (Phase 3)
    registry.register(
        MofaWidgetDef::new("chat_bubble", "Chat Bubble", WidgetCategory::Chat)
            .description("Role-aware chat message bubble (User/Assistant/System)")
    );
    registry.register(
        MofaWidgetDef::new("chat_bubble_panel", "Chat Bubble Panel", WidgetCategory::Chat)
            .requires_dora(true)
            .description("Scrollable role-aware conversation panel with auto-scroll")
    );
    registry.register(
        MofaWidgetDef::new("chat_panel", "Chat Panel", WidgetCategory::Chat)
            .description("Chat message display with markdown support")
    );
    registry.register(
        MofaWidgetDef::new("chat_input", "Chat Input", WidgetCategory::Chat)
            .description("Text input with send button for chat")
    );
    registry.register(
        MofaWidgetDef::new("log_panel", "Log Panel", WidgetCategory::Debug)
            .description("Filterable log display with search")
    );

    // Register config widgets (Phase 4)
    registry.register(
        MofaWidgetDef::new("role_editor", "Role Editor", WidgetCategory::Config)
            .description("Role configuration with model/voice/prompt editing")
    );
    registry.register(
        MofaWidgetDef::new("dataflow_picker", "Dataflow Picker", WidgetCategory::Config)
            .description("YAML dataflow file selector")
    );
    registry.register(
        MofaWidgetDef::new("provider_selector", "Provider Selector", WidgetCategory::Config)
            .description("AI provider and model selector")
    );

    // Register shell components (Phase 5)
    registry.register(
        MofaWidgetDef::new("mofa_shell", "MoFA Shell", WidgetCategory::Shell)
            .description("Main application shell layout with header, sidebar, content")
    );
    registry.register(
        MofaWidgetDef::new("shell_header", "Shell Header", WidgetCategory::Shell)
            .description("Application header with navigation and theme toggle")
    );
    registry.register(
        MofaWidgetDef::new("shell_sidebar", "Shell Sidebar", WidgetCategory::Shell)
            .description("Collapsible navigation sidebar")
    );
    registry.register(
        MofaWidgetDef::new("status_bar", "Status Bar", WidgetCategory::Shell)
            .description("Connection status and notifications bar")
    );

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_registry() {
        let registry = create_default_registry();
        // Should have audio, chat, config, and shell widgets registered
        assert_eq!(registry.len(), 15);
        // Audio widgets (Phase 2)
        assert!(registry.contains("led_meter"));
        assert!(registry.contains("mic_button"));
        assert!(registry.contains("aec_button"));
        // Chat widgets (Phase 3)
        assert!(registry.contains("chat_bubble"));
        assert!(registry.contains("chat_bubble_panel"));
        assert!(registry.contains("chat_panel"));
        assert!(registry.contains("chat_input"));
        assert!(registry.contains("log_panel"));
        // Config widgets (Phase 4)
        assert!(registry.contains("role_editor"));
        assert!(registry.contains("dataflow_picker"));
        assert!(registry.contains("provider_selector"));
        // Shell components (Phase 5)
        assert!(registry.contains("mofa_shell"));
        assert!(registry.contains("shell_header"));
        assert!(registry.contains("shell_sidebar"));
        assert!(registry.contains("status_bar"));
    }

    #[test]
    fn test_re_exports() {
        // Verify re-exports work
        let _registry = MofaWidgetRegistry::new();
        let _theme = MofaTheme::default();
        let _config = AppConfig::default();
    }
}
