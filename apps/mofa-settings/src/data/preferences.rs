//! User preferences storage

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::providers::{get_supported_providers, Provider, ProviderId};

/// User preferences for the dashboard
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Preferences {
    pub providers: Vec<Provider>,
    pub default_chat_provider: Option<ProviderId>,
    pub default_tts_provider: Option<ProviderId>,
    pub default_asr_provider: Option<ProviderId>,
    #[serde(default)]
    pub audio_input_device: Option<String>,
    #[serde(default)]
    pub audio_output_device: Option<String>,
    /// Dark mode preference (true = dark, false = light)
    #[serde(default)]
    pub dark_mode: bool,
}

impl Preferences {
    /// Get the preferences file path
    pub fn get_preferences_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".dora")
            .join("dashboard")
            .join("preferences.json")
    }

    /// Load preferences from disk, or create defaults if not found
    pub fn load() -> Self {
        let path = Self::get_preferences_path();

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<Preferences>(&content) {
                        Ok(mut prefs) => {
                            // Merge with supported providers to ensure all are present
                            prefs.merge_with_supported_providers();
                            return prefs;
                        }
                        Err(e) => {
                            eprintln!("Failed to parse preferences: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read preferences file: {}", e);
                }
            }
        }

        // Return defaults with supported providers
        let mut prefs = Self::default();
        prefs.providers = get_supported_providers();
        prefs
    }

    /// Save preferences to disk
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_preferences_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;

        Ok(())
    }

    /// Merge loaded preferences with supported providers
    fn merge_with_supported_providers(&mut self) {
        let supported = get_supported_providers();

        for supported_provider in supported {
            if !self.providers.iter().any(|p| p.id == supported_provider.id) {
                self.providers.push(supported_provider);
            }
        }
    }

    /// Get a provider by ID
    pub fn get_provider(&self, id: &str) -> Option<&Provider> {
        self.providers.iter().find(|p| p.id == id)
    }

    /// Get a mutable provider by ID
    pub fn get_provider_mut(&mut self, id: &str) -> Option<&mut Provider> {
        self.providers.iter_mut().find(|p| p.id == id)
    }

    /// Update or insert a provider
    pub fn upsert_provider(&mut self, provider: Provider) {
        if let Some(existing) = self.providers.iter_mut().find(|p| p.id == provider.id) {
            *existing = provider;
        } else {
            self.providers.push(provider);
        }
    }

    /// Remove a custom provider
    pub fn remove_provider(&mut self, id: &str) -> bool {
        if let Some(pos) = self
            .providers
            .iter()
            .position(|p| p.id == id && p.is_custom)
        {
            self.providers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all enabled providers
    pub fn get_enabled_providers(&self) -> Vec<&Provider> {
        self.providers.iter().filter(|p| p.enabled).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::providers::ProviderType;

    fn create_test_provider(id: &str, is_custom: bool, enabled: bool) -> Provider {
        Provider {
            id: id.to_string(),
            name: format!("Test {}", id),
            url: format!("https://{}.example.com", id),
            api_key: None,
            provider_type: ProviderType::Custom,
            enabled,
            models: vec!["test-model".to_string()],
            is_custom,
            ..Default::default()
        }
    }

    #[test]
    fn test_preferences_default() {
        let prefs = Preferences::default();

        assert!(prefs.providers.is_empty());
        assert!(prefs.default_chat_provider.is_none());
        assert!(prefs.default_tts_provider.is_none());
        assert!(prefs.default_asr_provider.is_none());
        assert!(!prefs.dark_mode);
        assert!(prefs.audio_input_device.is_none());
        assert!(prefs.audio_output_device.is_none());
    }

    #[test]
    fn test_get_preferences_path() {
        let path = Preferences::get_preferences_path();

        // Should end with the expected path components
        assert!(path.ends_with(".dora/dashboard/preferences.json"));
    }

    #[test]
    fn test_get_provider() {
        let mut prefs = Preferences::default();
        prefs
            .providers
            .push(create_test_provider("provider1", false, true));
        prefs
            .providers
            .push(create_test_provider("provider2", true, false));

        // Found
        let found = prefs.get_provider("provider1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "provider1");

        // Not found
        assert!(prefs.get_provider("nonexistent").is_none());
    }

    #[test]
    fn test_get_provider_mut() {
        let mut prefs = Preferences::default();
        prefs
            .providers
            .push(create_test_provider("provider1", false, false));

        // Modify provider
        if let Some(provider) = prefs.get_provider_mut("provider1") {
            provider.enabled = true;
            provider.api_key = Some("secret".to_string());
        }

        let provider = prefs.get_provider("provider1").unwrap();
        assert!(provider.enabled);
        assert_eq!(provider.api_key, Some("secret".to_string()));
    }

    #[test]
    fn test_upsert_provider_insert() {
        let mut prefs = Preferences::default();
        assert!(prefs.providers.is_empty());

        let provider = create_test_provider("new_provider", true, true);
        prefs.upsert_provider(provider);

        assert_eq!(prefs.providers.len(), 1);
        assert_eq!(prefs.providers[0].id, "new_provider");
    }

    #[test]
    fn test_upsert_provider_update() {
        let mut prefs = Preferences::default();
        prefs
            .providers
            .push(create_test_provider("existing", false, false));

        // Update with new data
        let mut updated = create_test_provider("existing", false, true);
        updated.api_key = Some("new_key".to_string());
        prefs.upsert_provider(updated);

        // Should still be 1 provider, but updated
        assert_eq!(prefs.providers.len(), 1);
        assert!(prefs.providers[0].enabled);
        assert_eq!(prefs.providers[0].api_key, Some("new_key".to_string()));
    }

    #[test]
    fn test_remove_provider_custom() {
        let mut prefs = Preferences::default();
        prefs
            .providers
            .push(create_test_provider("custom1", true, false));
        prefs
            .providers
            .push(create_test_provider("builtin1", false, false));

        // Remove custom provider - should succeed
        assert!(prefs.remove_provider("custom1"));
        assert_eq!(prefs.providers.len(), 1);
        assert!(prefs.get_provider("custom1").is_none());
    }

    #[test]
    fn test_remove_provider_builtin() {
        let mut prefs = Preferences::default();
        prefs
            .providers
            .push(create_test_provider("builtin1", false, false));

        // Cannot remove non-custom provider
        assert!(!prefs.remove_provider("builtin1"));
        assert_eq!(prefs.providers.len(), 1);
    }

    #[test]
    fn test_remove_provider_nonexistent() {
        let mut prefs = Preferences::default();

        // Cannot remove nonexistent provider
        assert!(!prefs.remove_provider("nonexistent"));
    }

    #[test]
    fn test_get_enabled_providers() {
        let mut prefs = Preferences::default();
        prefs
            .providers
            .push(create_test_provider("enabled1", false, true));
        prefs
            .providers
            .push(create_test_provider("disabled1", false, false));
        prefs
            .providers
            .push(create_test_provider("enabled2", true, true));
        prefs
            .providers
            .push(create_test_provider("disabled2", true, false));

        let enabled = prefs.get_enabled_providers();
        assert_eq!(enabled.len(), 2);
        assert!(enabled.iter().any(|p| p.id == "enabled1"));
        assert!(enabled.iter().any(|p| p.id == "enabled2"));
    }

    #[test]
    fn test_merge_with_supported_providers() {
        let mut prefs = Preferences::default();
        // Start with one custom provider
        prefs
            .providers
            .push(create_test_provider("my_custom", true, true));

        // Merge should add all supported providers
        prefs.merge_with_supported_providers();

        // Should have custom + all supported (3)
        assert!(prefs.providers.len() >= 4);
        assert!(prefs.get_provider("my_custom").is_some());
        assert!(prefs.get_provider("openai").is_some());
        assert!(prefs.get_provider("deepseek").is_some());
        assert!(prefs.get_provider("alibaba_cloud").is_some());
    }

    #[test]
    fn test_merge_does_not_duplicate() {
        let mut prefs = Preferences::default();
        // Pre-populate with supported providers
        prefs.providers = get_supported_providers();
        let initial_count = prefs.providers.len();

        // Merge again - should not add duplicates
        prefs.merge_with_supported_providers();

        assert_eq!(prefs.providers.len(), initial_count);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut prefs = Preferences::default();
        prefs
            .providers
            .push(create_test_provider("test", true, true));
        prefs.default_chat_provider = Some("test".to_string());
        prefs.dark_mode = true;
        prefs.audio_input_device = Some("Microphone".to_string());
        prefs.audio_output_device = Some("Speakers".to_string());

        // Serialize
        let json = serde_json::to_string(&prefs).unwrap();

        // Deserialize
        let restored: Preferences = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.providers.len(), 1);
        assert_eq!(restored.providers[0].id, "test");
        assert_eq!(restored.default_chat_provider, Some("test".to_string()));
        assert!(restored.dark_mode);
        assert_eq!(restored.audio_input_device, Some("Microphone".to_string()));
        assert_eq!(restored.audio_output_device, Some("Speakers".to_string()));
    }

    #[test]
    fn test_deserialization_with_missing_optional_fields() {
        // JSON without new optional fields (backwards compatibility)
        let json = r#"{
            "providers": [],
            "default_chat_provider": null,
            "default_tts_provider": null,
            "default_asr_provider": null
        }"#;

        let prefs: Preferences = serde_json::from_str(json).unwrap();

        // New fields should use defaults
        assert!(!prefs.dark_mode);
        assert!(prefs.audio_input_device.is_none());
        assert!(prefs.audio_output_device.is_none());
    }
}
