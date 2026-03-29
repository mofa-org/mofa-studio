//! # MoFA Widgets
//!
//! Shared reusable UI components for MoFA Studio applications.
//!
//! This crate provides the core widget library and plugin infrastructure for building
//! MoFA Studio apps using the [Makepad](https://github.com/makepad/makepad) UI framework.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use makepad_widgets::Cx;
//! use mofa_widgets::{MofaApp, AppInfo};
//!
//! // 1. Register widgets in your app's live_register
//! impl LiveRegister for MyApp {
//!     fn live_register(cx: &mut Cx) {
//!         mofa_widgets::live_design(cx);  // Register shared widgets
//!         // ... register your app's widgets
//!     }
//! }
//! ```
//!
//! ## Modules
//!
//! - [`theme`] - Color palette, fonts, and dark mode support
//! - [`app_trait`] - Plugin app interface (`MofaApp`, `AppRegistry`)
//! - [`participant_panel`] - User avatar with audio waveform
//! - [`waveform_view`] - Real-time audio waveform visualization
//! - [`log_panel`] - Scrollable Markdown log display
//! - [`led_gauge`] - LED-style bar gauge for levels
//! - [`chat_bubble`] - Chat message bubble with role support
//! - [`audio_player`] - Audio playback engine
//!
//! ## Theme System
//!
//! The theme module provides a centralized color system with dark mode support:
//!
//! ```rust,ignore
//! use mofa_widgets::theme::*;
//!
//! // In live_design! macro - use theme constants
//! live_design! {
//!     use mofa_widgets::theme::*;
//!
//!     MyWidget = <View> {
//!         draw_bg: { color: (PANEL_BG) }
//!         label = <Label> {
//!             draw_text: { color: (TEXT_PRIMARY) }
//!         }
//!     }
//! }
//! ```
//!
//! ## Plugin Apps
//!
//! Apps implement the [`MofaApp`] trait for standardized registration:
//!
//! ```rust,ignore
//! use mofa_widgets::{MofaApp, AppInfo};
//!
//! pub struct MyApp;
//!
//! impl MofaApp for MyApp {
//!     fn info() -> AppInfo {
//!         AppInfo {
//!             name: "My App",
//!             id: "my-app",
//!             description: "Description here",
//!         }
//!     }
//!
//!     fn live_design(cx: &mut Cx) {
//!         // Register app's widgets
//!     }
//! }
//! ```

pub mod app_trait;
pub mod audio_player;
pub mod chat_bubble;
pub mod led_gauge;
pub mod log_panel;
pub mod participant_panel;
pub mod theme;
pub mod waveform_view;

// Re-export app trait types for convenience
pub use app_trait::{AppInfo, AppRegistry, MofaApp, PageId, PageRouter, StateChangeListener, TimerControl, tab_clicked};

use makepad_widgets::Cx;

/// Register all shared widgets with Makepad.
///
/// This function must be called during app initialization, typically in `LiveRegister::live_register`.
///
/// **Important**: Theme is registered first as other widgets depend on its font and color definitions.
///
/// # Example
///
/// ```rust,ignore
/// impl LiveRegister for App {
///     fn live_register(cx: &mut Cx) {
///         mofa_widgets::live_design(cx);  // Register shared widgets first
///         my_app::live_design(cx);        // Then app-specific widgets
///     }
/// }
/// ```
///
/// # Registration Order
///
/// 1. `theme` - Fonts and base styles (required by all widgets)
/// 2. `waveform_view` - Audio visualization
/// 3. `participant_panel` - User panels with waveforms
/// 4. `log_panel` - Log display
/// 5. `led_gauge` - Level indicators
pub fn live_design(cx: &mut Cx) {
    // Theme provides fonts and base styles - must be first
    theme::live_design(cx);

    // Register widgets in dependency order
    waveform_view::live_design(cx);
    participant_panel::live_design(cx);
    log_panel::live_design(cx);
    led_gauge::live_design(cx);
}

// Re-export commonly used types
pub use audio_player::*;
pub use participant_panel::ParticipantPanel;
