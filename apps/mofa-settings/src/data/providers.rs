//! Provider data models

use serde::{Deserialize, Serialize};

/// Unique identifier for a provider
pub type ProviderId = String;

/// Type of AI provider API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProviderType {
    #[default]
    OpenAi,
    DeepSeek,
    AlibabaCloud,
    Nvidia,
    Custom,
}

impl ProviderType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderType::OpenAi => "OpenAI",
            ProviderType::DeepSeek => "DeepSeek",
            ProviderType::AlibabaCloud => "Alibaba Cloud",
            ProviderType::Nvidia => "NVIDIA",
            ProviderType::Custom => "Custom",
        }
    }
}

/// Connection status of a provider
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ProviderConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl ProviderConnectionStatus {
    pub fn display_text(&self) -> &str {
        match self {
            ProviderConnectionStatus::Disconnected => "Disconnected",
            ProviderConnectionStatus::Connecting => "Connecting...",
            ProviderConnectionStatus::Connected => "Connected",
            ProviderConnectionStatus::Error(msg) => msg.as_str(),
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, ProviderConnectionStatus::Connected)
    }
}

/// A configured AI provider
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provider {
    pub id: ProviderId,
    pub name: String,
    pub url: String,
    pub api_key: Option<String>,
    pub provider_type: ProviderType,
    pub enabled: bool,
    pub models: Vec<String>,
    pub is_custom: bool,
    #[serde(skip)]
    pub connection_status: ProviderConnectionStatus,
}

impl Default for Provider {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            url: String::new(),
            api_key: None,
            provider_type: ProviderType::OpenAi,
            enabled: false,
            models: Vec::new(),
            is_custom: false,
            connection_status: ProviderConnectionStatus::Disconnected,
        }
    }
}

impl Provider {
    pub fn new_custom(name: String, url: String, provider_type: ProviderType) -> Self {
        let id = Self::generate_id(&name);
        Self {
            id,
            name,
            url,
            api_key: None,
            provider_type,
            enabled: false,
            models: Vec::new(),
            is_custom: true,
            connection_status: ProviderConnectionStatus::Disconnected,
        }
    }

    pub fn generate_id(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    }

    pub fn status_color(&self) -> &'static str {
        if !self.enabled {
            "#9ca3af" // Gray - disabled
        } else {
            match &self.connection_status {
                ProviderConnectionStatus::Connected => "#22c55e", // Green
                ProviderConnectionStatus::Connecting => "#f59e0b", // Yellow
                ProviderConnectionStatus::Disconnected => "#6b7280", // Gray
                ProviderConnectionStatus::Error(_) => "#ef4444",  // Red
            }
        }
    }
}

/// Pre-configured supported providers
pub fn get_supported_providers() -> Vec<Provider> {
    vec![
        Provider {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            url: "https://api.openai.com/v1".to_string(),
            api_key: None,
            provider_type: ProviderType::OpenAi,
            enabled: false,
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o1-mini".to_string(),
            ],
            is_custom: false,
            connection_status: ProviderConnectionStatus::Disconnected,
        },
        Provider {
            id: "deepseek".to_string(),
            name: "DeepSeek".to_string(),
            url: "https://api.deepseek.com/v1".to_string(),
            api_key: None,
            provider_type: ProviderType::DeepSeek,
            enabled: false,
            models: vec!["deepseek-chat".to_string(), "deepseek-reasoner".to_string()],
            is_custom: false,
            connection_status: ProviderConnectionStatus::Disconnected,
        },
        Provider {
            id: "alibaba_cloud".to_string(),
            name: "Alibaba Cloud (Qwen)".to_string(),
            url: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
            api_key: None,
            provider_type: ProviderType::AlibabaCloud,
            enabled: false,
            models: vec![
                "qwen-plus".to_string(),
                "qwen-turbo".to_string(),
                "qwen-max".to_string(),
            ],
            is_custom: false,
            connection_status: ProviderConnectionStatus::Disconnected,
        },
        Provider {
            id: "nvidia".to_string(),
            name: "NVIDIA".to_string(),
            url: "https://integrate.api.nvidia.com/v1".to_string(),
            api_key: None,
            provider_type: ProviderType::Nvidia,
            enabled: false,
            models: vec![
                "deepseek-ai/deepseek-r1".to_string(),
                "deepseek-ai/deepseek-v3.2".to_string(),
                "moonshotai/kimi-k2-thinking".to_string(),
                "minimaxai/minimax-m2".to_string(),
                "meta/llama-3.3-70b-instruct".to_string(),
            ],
            is_custom: false,
            connection_status: ProviderConnectionStatus::Disconnected,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_display_name() {
        assert_eq!(ProviderType::OpenAi.display_name(), "OpenAI");
        assert_eq!(ProviderType::DeepSeek.display_name(), "DeepSeek");
        assert_eq!(ProviderType::AlibabaCloud.display_name(), "Alibaba Cloud");
        assert_eq!(ProviderType::Nvidia.display_name(), "NVIDIA");
        assert_eq!(ProviderType::Custom.display_name(), "Custom");
    }

    #[test]
    fn test_provider_type_default() {
        let default = ProviderType::default();
        assert_eq!(default, ProviderType::OpenAi);
    }

    #[test]
    fn test_connection_status_display_text() {
        assert_eq!(
            ProviderConnectionStatus::Disconnected.display_text(),
            "Disconnected"
        );
        assert_eq!(
            ProviderConnectionStatus::Connecting.display_text(),
            "Connecting..."
        );
        assert_eq!(
            ProviderConnectionStatus::Connected.display_text(),
            "Connected"
        );
        assert_eq!(
            ProviderConnectionStatus::Error("API key invalid".to_string()).display_text(),
            "API key invalid"
        );
    }

    #[test]
    fn test_connection_status_is_connected() {
        assert!(!ProviderConnectionStatus::Disconnected.is_connected());
        assert!(!ProviderConnectionStatus::Connecting.is_connected());
        assert!(ProviderConnectionStatus::Connected.is_connected());
        assert!(!ProviderConnectionStatus::Error("error".to_string()).is_connected());
    }

    #[test]
    fn test_generate_id() {
        assert_eq!(Provider::generate_id("OpenAI"), "openai");
        assert_eq!(Provider::generate_id("Deep Seek"), "deep_seek");
        assert_eq!(
            Provider::generate_id("Alibaba Cloud (Qwen)"),
            "alibaba_cloud__qwen_"
        );
        assert_eq!(
            Provider::generate_id("My-Custom-Provider"),
            "my_custom_provider"
        );
        assert_eq!(Provider::generate_id("Test123"), "test123");
    }

    #[test]
    fn test_status_color_disabled() {
        let provider = Provider {
            enabled: false,
            connection_status: ProviderConnectionStatus::Connected,
            ..Default::default()
        };
        // Disabled providers are always gray regardless of connection status
        assert_eq!(provider.status_color(), "#9ca3af");
    }

    #[test]
    fn test_status_color_enabled() {
        let mut provider = Provider {
            enabled: true,
            ..Default::default()
        };

        provider.connection_status = ProviderConnectionStatus::Connected;
        assert_eq!(provider.status_color(), "#22c55e"); // Green

        provider.connection_status = ProviderConnectionStatus::Connecting;
        assert_eq!(provider.status_color(), "#f59e0b"); // Yellow

        provider.connection_status = ProviderConnectionStatus::Disconnected;
        assert_eq!(provider.status_color(), "#6b7280"); // Gray

        provider.connection_status = ProviderConnectionStatus::Error("error".to_string());
        assert_eq!(provider.status_color(), "#ef4444"); // Red
    }

    #[test]
    fn test_new_custom_provider() {
        let provider = Provider::new_custom(
            "My Provider".to_string(),
            "https://api.example.com".to_string(),
            ProviderType::Custom,
        );

        assert_eq!(provider.id, "my_provider");
        assert_eq!(provider.name, "My Provider");
        assert_eq!(provider.url, "https://api.example.com");
        assert!(provider.is_custom);
        assert!(!provider.enabled);
        assert!(provider.models.is_empty());
        assert_eq!(provider.provider_type, ProviderType::Custom);
    }

    #[test]
    fn test_get_supported_providers() {
        let providers = get_supported_providers();

        assert_eq!(providers.len(), 4);

        // Check OpenAI
        let openai = providers.iter().find(|p| p.id == "openai").unwrap();
        assert_eq!(openai.name, "OpenAI");
        assert_eq!(openai.provider_type, ProviderType::OpenAi);
        assert!(!openai.is_custom);
        assert!(openai.models.contains(&"gpt-4o".to_string()));

        // Check DeepSeek
        let deepseek = providers.iter().find(|p| p.id == "deepseek").unwrap();
        assert_eq!(deepseek.name, "DeepSeek");
        assert_eq!(deepseek.provider_type, ProviderType::DeepSeek);

        // Check Alibaba Cloud
        let alibaba = providers.iter().find(|p| p.id == "alibaba_cloud").unwrap();
        assert_eq!(alibaba.name, "Alibaba Cloud (Qwen)");
        assert_eq!(alibaba.provider_type, ProviderType::AlibabaCloud);

        // Check NVIDIA
        let nvidia = providers.iter().find(|p| p.id == "nvidia").unwrap();
        assert_eq!(nvidia.name, "NVIDIA");
        assert_eq!(nvidia.provider_type, ProviderType::Nvidia);
        assert_eq!(nvidia.url, "https://integrate.api.nvidia.com/v1");
        assert!(nvidia
            .models
            .contains(&"deepseek-ai/deepseek-r1".to_string()));
    }

    #[test]
    fn test_provider_default() {
        let provider = Provider::default();

        assert!(provider.id.is_empty());
        assert!(provider.name.is_empty());
        assert!(provider.url.is_empty());
        assert!(provider.api_key.is_none());
        assert_eq!(provider.provider_type, ProviderType::OpenAi);
        assert!(!provider.enabled);
        assert!(provider.models.is_empty());
        assert!(!provider.is_custom);
        assert_eq!(
            provider.connection_status,
            ProviderConnectionStatus::Disconnected
        );
    }
}
