//! # MofaApp Trait - Plugin App Interface
//!
//! This module defines the standard interface for apps that integrate with the MoFA Studio shell.
//!
//! ## Architecture
//!
//! Due to Makepad's compile-time `live_design!` macro requirements, widget types must
//! still be imported directly in the shell. This trait provides:
//!
//! - **Standardized metadata** - App name, ID, description via [`AppInfo`]
//! - **Consistent registration** - Widget registration via [`MofaApp::live_design`]
//! - **Timer lifecycle** - Resource management via [`TimerControl`]
//! - **Runtime queries** - App discovery via [`AppRegistry`]
//!
//! ## Usage in Shell
//!
//! ```rust,ignore
//! use mofa_widgets::{MofaApp, AppRegistry};
//! use mofa_fm::MoFaFMApp;
//! use mofa_settings::MoFaSettingsApp;
//!
//! // In App struct
//! #[rust]
//! app_registry: AppRegistry,
//!
//! // In LiveHook::after_new_from_doc
//! fn after_new_from_doc(&mut self, _cx: &mut Cx) {
//!     self.app_registry.register(MoFaFMApp::info());
//!     self.app_registry.register(MoFaSettingsApp::info());
//! }
//!
//! // In LiveRegister
//! fn live_register(cx: &mut Cx) {
//!     <MoFaFMApp as MofaApp>::live_design(cx);
//!     <MoFaSettingsApp as MofaApp>::live_design(cx);
//! }
//! ```
//!
//! ## Creating a New App
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
//!             description: "My awesome MoFA app",
//!         }
//!     }
//!
//!     fn live_design(cx: &mut Cx) {
//!         crate::screen::live_design(cx);
//!         crate::widgets::live_design(cx);
//!     }
//! }
//! ```

use makepad_widgets::{Cx, LiveId, Action, live_id, ButtonAction, WidgetActionCast};

/// Metadata about a registered app
#[derive(Clone, Debug)]
pub struct AppInfo {
    /// Display name shown in UI
    pub name: &'static str,
    /// Unique identifier for the app
    pub id: &'static str,
    /// Description of the app
    pub description: &'static str,
    /// LiveId for the sidebar tab button (for click detection)
    pub tab_id: Option<LiveId>,
    /// LiveId for the page view (for visibility control)
    pub page_id: Option<LiveId>,
    /// Whether this app is shown in the main sidebar (vs settings/system apps)
    pub show_in_sidebar: bool,
}

impl Default for AppInfo {
    fn default() -> Self {
        Self {
            name: "",
            id: "",
            description: "",
            tab_id: None,
            page_id: None,
            show_in_sidebar: true,
        }
    }
}

/// Page identifiers for routing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PageId {
    /// MoFA FM main app
    MofaFM,
    /// Debate app
    Debate,
    /// TTS app (GPT-SoVITS)
    TTS,
    /// Settings page
    Settings,
    /// Generic app page (for demo apps)
    App,
}

impl PageId {
    /// Get the LiveId for this page's tab button
    pub fn tab_live_id(&self) -> LiveId {
        match self {
            PageId::MofaFM => live_id!(mofa_fm_tab),
            PageId::Debate => live_id!(debate_tab),
            PageId::TTS => live_id!(tts_tab),
            PageId::Settings => live_id!(settings_tab),
            PageId::App => live_id!(app_tab),
        }
    }

    /// Get the LiveId for this page's view
    pub fn page_live_id(&self) -> LiveId {
        match self {
            PageId::MofaFM => live_id!(fm_page),
            PageId::Debate => live_id!(debate_page),
            PageId::TTS => live_id!(tts_page),
            PageId::Settings => live_id!(settings_page),
            PageId::App => live_id!(app_page),
        }
    }
}

/// Router for managing page visibility and navigation
///
/// Centralizes page switching logic to avoid repetitive visibility code.
#[derive(Default)]
pub struct PageRouter {
    /// Currently active page
    current_page: Option<PageId>,
    /// All registered pages
    pages: Vec<PageId>,
}

impl PageRouter {
    pub fn new() -> Self {
        Self {
            current_page: Some(PageId::MofaFM), // Default to FM
            pages: vec![
                PageId::MofaFM,
                PageId::Debate,
                PageId::TTS,
                PageId::Settings,
                PageId::App,
            ],
        }
    }

    /// Get the current active page
    pub fn current(&self) -> Option<PageId> {
        self.current_page
    }

    /// Navigate to a page, returns true if page changed
    pub fn navigate_to(&mut self, page: PageId) -> bool {
        if self.current_page == Some(page) {
            return false;
        }
        self.current_page = Some(page);
        true
    }

    /// Get all pages that should be hidden (all except current)
    pub fn pages_to_hide(&self) -> impl Iterator<Item = PageId> + '_ {
        self.pages.iter().copied().filter(move |p| Some(*p) != self.current_page)
    }

    /// Check if any registered tab was clicked in actions (uses path-based detection)
    /// Returns the PageId if a tab click was detected
    pub fn check_tab_click(&self, actions: &[Action]) -> Option<PageId> {
        for action in actions {
            if let Some(wa) = action.as_widget_action() {
                if let ButtonAction::Clicked(_) = wa.cast() {
                    // Check each page's tab_id against the action path
                    for page in &self.pages {
                        let tab_id = page.tab_live_id();
                        if wa.path.data.iter().any(|id| *id == tab_id) {
                            return Some(*page);
                        }
                    }
                }
            }
        }
        None
    }
}

/// Helper to check if a specific tab was clicked using path-based detection
/// This avoids WidgetUid mismatch issues with nested widgets
pub fn tab_clicked(actions: &[Action], tab_id: LiveId) -> bool {
    actions.iter().filter_map(|a| a.as_widget_action()).any(|wa| {
            if let ButtonAction::Clicked(_) = wa.cast() {
                wa.path.data.iter().any(|id| *id == tab_id)
            } else {
                false
            }
        })
}

/// Trait for apps that integrate with MoFA Studio shell
///
/// # Example
/// ```ignore
/// impl MofaApp for MoFaFMApp {
///     fn info() -> AppInfo {
///         AppInfo {
///             name: "MoFA FM",
///             id: "mofa-fm",
///             description: "AI-powered audio streaming",
///         }
///     }
///
///     fn live_design(cx: &mut Cx) {
///         screen::live_design(cx);
///     }
/// }
/// ```
pub trait MofaApp {
    /// Returns metadata about this app
    fn info() -> AppInfo
    where
        Self: Sized;

    /// Register this app's widgets with Makepad
    fn live_design(cx: &mut Cx);
}

/// Trait for apps with timer-based animations that need lifecycle control
///
/// Apps implementing this trait should stop their timers when hidden
/// and restart them when shown, to prevent resource waste.
pub trait TimerControl {
    /// Stop all timers (call when app becomes hidden)
    fn stop_timers(&self, cx: &mut Cx);

    /// Start/restart timers (call when app becomes visible)
    fn start_timers(&self, cx: &mut Cx);
}

/// Registry of all installed apps
///
/// Note: Due to Makepad's architecture, apps must still be imported at compile time.
/// This registry provides metadata for runtime queries (e.g., sidebar generation).
pub struct AppRegistry {
    apps: Vec<AppInfo>,
}

impl AppRegistry {
    /// Create a new empty registry
    pub const fn new() -> Self {
        Self { apps: Vec::new() }
    }

    /// Register an app in the registry
    pub fn register(&mut self, info: AppInfo) {
        self.apps.push(info);
    }

    /// Get all registered apps
    pub fn apps(&self) -> &[AppInfo] {
        &self.apps
    }

    /// Find an app by ID
    pub fn find_by_id(&self, id: &str) -> Option<&AppInfo> {
        self.apps.iter().find(|app| app.id == id)
    }

    /// Number of registered apps
    pub fn len(&self) -> usize {
        self.apps.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }
}

impl Default for AppRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for widgets that respond to global state changes
///
/// Apps implement this trait to receive notifications when global state
/// changes (e.g., dark mode toggle, provider configuration updates).
///
/// # Example
/// ```ignore
/// impl StateChangeListener for MyScreenRef {
///     fn on_dark_mode_change(&self, cx: &mut Cx, dark_mode: f64) {
///         if let Some(mut inner) = self.borrow_mut() {
///             inner.view.apply_over(cx, live!{
///                 draw_bg: { dark_mode: (dark_mode) }
///             });
///         }
///     }
/// }
/// ```
pub trait StateChangeListener {
    /// Called when dark mode setting changes
    ///
    /// # Arguments
    /// * `cx` - Makepad context for applying UI updates
    /// * `dark_mode` - Dark mode value (0.0 = light, 1.0 = dark)
    fn on_dark_mode_change(&self, cx: &mut Cx, dark_mode: f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app_info(id: &'static str) -> AppInfo {
        AppInfo {
            name: "Test App",
            id,
            description: "A test app for unit tests",
            ..Default::default()
        }
    }

    #[test]
    fn test_app_info_fields() {
        let info = AppInfo {
            name: "MoFA FM",
            id: "mofa-fm",
            description: "AI-powered audio streaming",
            ..Default::default()
        };

        assert_eq!(info.name, "MoFA FM");
        assert_eq!(info.id, "mofa-fm");
        assert_eq!(info.description, "AI-powered audio streaming");
    }

    #[test]
    fn test_app_info_clone() {
        let info = create_test_app_info("test-app");
        let cloned = info.clone();

        assert_eq!(cloned.name, info.name);
        assert_eq!(cloned.id, info.id);
        assert_eq!(cloned.description, info.description);
    }

    #[test]
    fn test_app_registry_new() {
        let registry = AppRegistry::new();

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_app_registry_default() {
        let registry = AppRegistry::default();

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_app_registry_register() {
        let mut registry = AppRegistry::new();

        registry.register(create_test_app_info("app1"));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        registry.register(create_test_app_info("app2"));
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_app_registry_apps() {
        let mut registry = AppRegistry::new();
        registry.register(create_test_app_info("app1"));
        registry.register(create_test_app_info("app2"));

        let apps = registry.apps();
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].id, "app1");
        assert_eq!(apps[1].id, "app2");
    }

    #[test]
    fn test_app_registry_find_by_id() {
        let mut registry = AppRegistry::new();
        registry.register(AppInfo {
            name: "First App",
            id: "first",
            description: "The first app",
            ..Default::default()
        });
        registry.register(AppInfo {
            name: "Second App",
            id: "second",
            description: "The second app",
            ..Default::default()
        });

        // Found
        let found = registry.find_by_id("first");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "First App");

        let found = registry.find_by_id("second");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Second App");

        // Not found
        assert!(registry.find_by_id("nonexistent").is_none());
    }

    #[test]
    fn test_app_registry_find_by_id_empty() {
        let registry = AppRegistry::new();

        assert!(registry.find_by_id("any").is_none());
    }

    #[test]
    fn test_app_registry_len_and_is_empty() {
        let mut registry = AppRegistry::new();

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        registry.register(create_test_app_info("app1"));
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);

        registry.register(create_test_app_info("app2"));
        registry.register(create_test_app_info("app3"));
        assert_eq!(registry.len(), 3);
    }
}
