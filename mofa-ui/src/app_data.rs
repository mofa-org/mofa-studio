//! Shared App Data for Makepad Scope Injection
//!
//! This module provides `MofaAppData`, a container for all shared application state
//! that can be passed through Makepad's `Scope` mechanism to child widgets.
//!
//! ## Why Scope Injection?
//!
//! Makepad widgets need access to shared state (Dora bridge, theme, configuration)
//! but passing them through constructor parameters is not possible due to `live_design!`
//! macro constraints. Scope injection solves this by:
//!
//! 1. Creating a single `MofaAppData` instance in the root app
//! 2. Passing it through `Scope::with_data()` during event handling
//! 3. Widgets access it via `scope.data.get::<MofaAppData>()`
//!
//! ## Usage
//!
//! ### In App (Root)
//!
//! ```rust,ignore
//! use mofa_ui::{MofaAppData, MofaTheme};
//! use mofa_dora_bridge::SharedDoraState;
//!
//! struct MyApp {
//!     ui: WidgetRef,
//!     app_data: MofaAppData,
//! }
//!
//! impl AppMain for MyApp {
//!     fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
//!         // Pass app_data through scope to all child widgets
//!         self.ui.handle_event(cx, event, &mut Scope::with_data(&mut self.app_data));
//!     }
//! }
//! ```
//!
//! ### In Widget
//!
//! ```rust,ignore
//! impl Widget for MyWidget {
//!     fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
//!         // Access shared data from scope
//!         if let Some(data) = scope.data.get::<MofaAppData>() {
//!             let mic_level = data.dora_state().mic.level();
//!             let is_dark = data.theme().is_dark();
//!             // ...
//!         }
//!     }
//! }
//! ```

use crate::registry::MofaWidgetRegistry;
use crate::theme::MofaTheme;
use mofa_dora_bridge::SharedDoraState;
use std::sync::Arc;

/// Application configuration
#[derive(Clone, Debug, Default)]
pub struct AppConfig {
    /// Application name
    pub name: String,

    /// Application ID (for persistence)
    pub id: String,

    /// Whether debug mode is enabled
    pub debug: bool,

    /// Whether to auto-connect to Dora on startup
    pub auto_connect_dora: bool,
}

impl AppConfig {
    /// Create a new app config
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            debug: false,
            auto_connect_dora: false,
        }
    }

    /// Enable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Enable auto-connect to Dora
    pub fn with_auto_connect(mut self, auto_connect: bool) -> Self {
        self.auto_connect_dora = auto_connect;
        self
    }
}

/// Shared data passed through Makepad's Scope mechanism.
///
/// This container holds all the shared state that widgets need access to:
/// - Dora bridge state (mic levels, chat, audio, etc.)
/// - Theme settings (dark mode, colors)
/// - App configuration
/// - Widget registry
///
/// ## Thread Safety
///
/// `MofaAppData` is designed to be used from the UI thread only.
/// The `SharedDoraState` inside is thread-safe and can be shared
/// with Dora bridge threads via `Arc::clone()`.
pub struct MofaAppData {
    /// Dora bridge state (thread-safe, shareable)
    dora_state: Arc<SharedDoraState>,

    /// Current theme settings
    theme: MofaTheme,

    /// App-specific configuration
    config: AppConfig,

    /// Widget registry
    registry: Arc<MofaWidgetRegistry>,
}

impl MofaAppData {
    /// Create new app data with default settings
    pub fn new(dora_state: Arc<SharedDoraState>) -> Self {
        Self {
            dora_state,
            theme: MofaTheme::default(),
            config: AppConfig::default(),
            registry: Arc::new(MofaWidgetRegistry::new()),
        }
    }

    /// Create with custom configuration
    pub fn with_config(dora_state: Arc<SharedDoraState>, config: AppConfig) -> Self {
        Self {
            dora_state,
            theme: MofaTheme::default(),
            config,
            registry: Arc::new(MofaWidgetRegistry::new()),
        }
    }

    /// Create with all custom components
    pub fn with_all(
        dora_state: Arc<SharedDoraState>,
        theme: MofaTheme,
        config: AppConfig,
        registry: Arc<MofaWidgetRegistry>,
    ) -> Self {
        Self {
            dora_state,
            theme,
            config,
            registry,
        }
    }

    // --- Accessors ---

    /// Get shared Dora state (for cloning to bridge threads)
    pub fn dora_state(&self) -> &Arc<SharedDoraState> {
        &self.dora_state
    }

    /// Get current theme
    pub fn theme(&self) -> &MofaTheme {
        &self.theme
    }

    /// Get mutable theme
    pub fn theme_mut(&mut self) -> &mut MofaTheme {
        &mut self.theme
    }

    /// Get app configuration
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Get mutable app configuration
    pub fn config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }

    /// Get widget registry
    pub fn registry(&self) -> &Arc<MofaWidgetRegistry> {
        &self.registry
    }

    // --- Convenience Methods ---

    /// Check if dark mode is enabled
    pub fn is_dark_mode(&self) -> bool {
        self.theme.is_dark()
    }

    /// Get dark mode animation value (0.0 = light, 1.0 = dark)
    pub fn dark_mode_value(&self) -> f64 {
        self.theme.dark_mode_anim
    }

    /// Toggle dark mode
    pub fn toggle_dark_mode(&mut self) {
        self.theme.toggle();
    }

    /// Set dark mode
    pub fn set_dark_mode(&mut self, dark: bool) {
        self.theme.set_dark_mode(dark);
    }

    /// Check if Dora is connected (has active bridges)
    pub fn is_dora_connected(&self) -> bool {
        let status = self.dora_state.status.read();
        !status.active_bridges.is_empty()
    }

    /// Get active Dora bridge count
    pub fn active_bridge_count(&self) -> usize {
        let status = self.dora_state.status.read();
        status.active_bridges.len()
    }
}

impl Default for MofaAppData {
    fn default() -> Self {
        Self::new(SharedDoraState::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_data_default() {
        let data = MofaAppData::default();
        assert!(!data.is_dark_mode());
        assert!(!data.is_dora_connected());
    }

    #[test]
    fn test_app_data_with_config() {
        let config = AppConfig::new("test-app", "Test App")
            .with_debug(true)
            .with_auto_connect(true);

        let data = MofaAppData::with_config(SharedDoraState::new(), config);

        assert_eq!(data.config().id, "test-app");
        assert_eq!(data.config().name, "Test App");
        assert!(data.config().debug);
        assert!(data.config().auto_connect_dora);
    }

    #[test]
    fn test_dark_mode_toggle() {
        let mut data = MofaAppData::default();

        assert!(!data.is_dark_mode());
        data.toggle_dark_mode();
        assert!(data.is_dark_mode());
        data.toggle_dark_mode();
        assert!(!data.is_dark_mode());
    }

    #[test]
    fn test_dora_state_sharing() {
        let dora_state = SharedDoraState::new();
        let data = MofaAppData::new(Arc::clone(&dora_state));

        // Modify through dora_state directly
        dora_state.mic.set_level(0.5);

        // Should be visible through app_data
        assert_eq!(data.dora_state().mic.level(), 0.5);
    }
}
