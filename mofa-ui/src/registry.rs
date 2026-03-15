//! Widget Registry for MoFA UI Components
//!
//! Provides a registry for registering and discovering reusable UI widgets.
//! Widgets can be categorized and queried at runtime for dynamic composition.

use std::collections::HashMap;

/// Category of a widget for organization and filtering
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WidgetCategory {
    /// Audio-related widgets (mic, speaker, VU meter)
    Audio,
    /// Chat and messaging widgets
    Chat,
    /// Configuration and settings widgets
    Config,
    /// Debug and diagnostic widgets (logs, status)
    Debug,
    /// Navigation widgets (tabs, sidebars)
    Navigation,
    /// Shell layout components (header, sidebar, status bar)
    Shell,
    /// Custom app-specific widgets
    Custom(String),
}

impl WidgetCategory {
    /// Get display name for the category
    pub fn display_name(&self) -> &str {
        match self {
            WidgetCategory::Audio => "Audio",
            WidgetCategory::Chat => "Chat",
            WidgetCategory::Config => "Configuration",
            WidgetCategory::Debug => "Debug",
            WidgetCategory::Navigation => "Navigation",
            WidgetCategory::Shell => "Shell",
            WidgetCategory::Custom(name) => name,
        }
    }
}

/// Size hints for widget layout
#[derive(Clone, Debug)]
pub struct WidgetSize {
    /// Minimum width in pixels
    pub min_width: f64,
    /// Minimum height in pixels
    pub min_height: f64,
    /// Preferred width in pixels
    pub preferred_width: f64,
    /// Preferred height in pixels
    pub preferred_height: f64,
}

impl Default for WidgetSize {
    fn default() -> Self {
        Self {
            min_width: 100.0,
            min_height: 50.0,
            preferred_width: 300.0,
            preferred_height: 200.0,
        }
    }
}

/// Definition of a registerable widget
#[derive(Clone, Debug)]
pub struct MofaWidgetDef {
    /// Unique identifier (e.g., "audio_controls", "chat_panel")
    pub id: String,

    /// Display name for UI
    pub title: String,

    /// Category for organization
    pub category: WidgetCategory,

    /// Whether this widget requires a Dora connection
    pub requires_dora: bool,

    /// Whether the widget can be maximized
    pub maximizable: bool,

    /// Default size hints
    pub default_size: WidgetSize,

    /// Description of the widget's functionality
    pub description: String,
}

impl MofaWidgetDef {
    /// Create a new widget definition with minimal required fields
    pub fn new(id: impl Into<String>, title: impl Into<String>, category: WidgetCategory) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            category,
            requires_dora: false,
            maximizable: false,
            default_size: WidgetSize::default(),
            description: String::new(),
        }
    }

    /// Set whether this widget requires Dora connection
    pub fn requires_dora(mut self, requires: bool) -> Self {
        self.requires_dora = requires;
        self
    }

    /// Set whether this widget can be maximized
    pub fn maximizable(mut self, can_maximize: bool) -> Self {
        self.maximizable = can_maximize;
        self
    }

    /// Set the default size hints
    pub fn default_size(mut self, size: WidgetSize) -> Self {
        self.default_size = size;
        self
    }

    /// Set the description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}

/// Registry for all available widgets
///
/// The registry maintains a collection of widget definitions that can be
/// queried by ID or category. This enables dynamic composition of UIs
/// based on available widgets.
pub struct MofaWidgetRegistry {
    definitions: HashMap<String, MofaWidgetDef>,
    order: Vec<String>,
}

impl MofaWidgetRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Register a widget definition
    ///
    /// If a widget with the same ID already exists, it will be replaced.
    pub fn register(&mut self, def: MofaWidgetDef) {
        let id = def.id.clone();
        if !self.definitions.contains_key(&id) {
            self.order.push(id.clone());
        }
        self.definitions.insert(id, def);
    }

    /// Get a widget definition by ID
    pub fn get(&self, id: &str) -> Option<&MofaWidgetDef> {
        self.definitions.get(id)
    }

    /// Get all widget definitions in registration order
    pub fn all(&self) -> Vec<&MofaWidgetDef> {
        self.order
            .iter()
            .filter_map(|id| self.definitions.get(id))
            .collect()
    }

    /// Get widget definitions by category
    pub fn by_category(&self, category: &WidgetCategory) -> Vec<&MofaWidgetDef> {
        self.order
            .iter()
            .filter_map(|id| self.definitions.get(id))
            .filter(|def| &def.category == category)
            .collect()
    }

    /// Get widget definitions that require Dora connection
    pub fn dora_widgets(&self) -> Vec<&MofaWidgetDef> {
        self.order
            .iter()
            .filter_map(|id| self.definitions.get(id))
            .filter(|def| def.requires_dora)
            .collect()
    }

    /// Get widget definitions that can be maximized
    pub fn maximizable_widgets(&self) -> Vec<&MofaWidgetDef> {
        self.order
            .iter()
            .filter_map(|id| self.definitions.get(id))
            .filter(|def| def.maximizable)
            .collect()
    }

    /// Check if a widget is registered
    pub fn contains(&self, id: &str) -> bool {
        self.definitions.contains_key(id)
    }

    /// Number of registered widgets
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Get all unique categories in the registry
    pub fn categories(&self) -> Vec<&WidgetCategory> {
        let mut seen = std::collections::HashSet::new();
        self.order
            .iter()
            .filter_map(|id| self.definitions.get(id))
            .filter_map(|def| {
                if seen.insert(&def.category) {
                    Some(&def.category)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for MofaWidgetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_def_builder() {
        let def = MofaWidgetDef::new("test", "Test Widget", WidgetCategory::Audio)
            .requires_dora(true)
            .maximizable(true)
            .description("A test widget");

        assert_eq!(def.id, "test");
        assert_eq!(def.title, "Test Widget");
        assert!(def.requires_dora);
        assert!(def.maximizable);
        assert_eq!(def.description, "A test widget");
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = MofaWidgetRegistry::new();
        let def = MofaWidgetDef::new("audio_controls", "Audio Controls", WidgetCategory::Audio);

        registry.register(def);

        assert!(registry.contains("audio_controls"));
        assert_eq!(registry.len(), 1);

        let retrieved = registry.get("audio_controls").unwrap();
        assert_eq!(retrieved.title, "Audio Controls");
    }

    #[test]
    fn test_registry_by_category() {
        let mut registry = MofaWidgetRegistry::new();

        registry.register(MofaWidgetDef::new("mic", "Mic", WidgetCategory::Audio));
        registry.register(MofaWidgetDef::new(
            "speaker",
            "Speaker",
            WidgetCategory::Audio,
        ));
        registry.register(MofaWidgetDef::new("chat", "Chat", WidgetCategory::Chat));

        let audio = registry.by_category(&WidgetCategory::Audio);
        assert_eq!(audio.len(), 2);

        let chat = registry.by_category(&WidgetCategory::Chat);
        assert_eq!(chat.len(), 1);
    }

    #[test]
    fn test_registry_order_preserved() {
        let mut registry = MofaWidgetRegistry::new();

        registry.register(MofaWidgetDef::new("first", "First", WidgetCategory::Audio));
        registry.register(MofaWidgetDef::new("second", "Second", WidgetCategory::Chat));
        registry.register(MofaWidgetDef::new("third", "Third", WidgetCategory::Config));

        let all = registry.all();
        assert_eq!(all[0].id, "first");
        assert_eq!(all[1].id, "second");
        assert_eq!(all[2].id, "third");
    }
}
