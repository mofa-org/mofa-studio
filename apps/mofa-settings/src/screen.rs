//! Settings Screen - Main settings interface

use crate::add_provider_modal::{AddProviderModalAction, AddProviderModalWidgetExt};
use crate::data::{Preferences, Provider, ProviderId};
use crate::provider_view::ProviderViewWidgetExt;
use crate::providers_panel::{ProvidersPanelAction, ProvidersPanelWidgetExt};
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::*;

    use crate::providers_panel::ProvidersPanel;
    use crate::provider_view::ProviderView;
    use crate::add_provider_modal::AddProviderModal;

    // Divider line with dark mode support
    VerticalDivider = <View> {
        width: 1, height: Fill
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((BORDER), (BORDER_DARK), self.dark_mode);
            }
        }
    }

    // Settings screen container
    pub SettingsScreen = {{SettingsScreen}} {
        width: Fill, height: Fill
        flow: Overlay
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((DARK_BG), (DARK_BG_DARK), self.dark_mode);
            }
        }

        // Main content
        content = <View> {
            width: Fill, height: Fill
            flow: Right

            // Left panel - provider list
            providers_panel = <ProvidersPanel> {}

            // Divider
            vertical_divider = <VerticalDivider> {}

            // Right panel - provider details
            provider_view = <ProviderView> {}
        }

        // Modal overlay (hidden by default)
        add_provider_modal = <AddProviderModal> {}
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct SettingsScreen {
    #[deref]
    view: View,

    #[rust]
    preferences: Option<Preferences>,

    #[rust]
    selected_provider_id: Option<ProviderId>,
}

impl Widget for SettingsScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Initialize with default provider on first event
        if self.selected_provider_id.is_none() {
            // Load custom providers into the panel
            self.view
                .providers_panel(ids!(content.providers_panel))
                .load_providers(cx);

            let default_id = ProviderId::from("openai");
            self.selected_provider_id = Some(default_id.clone());
            self.load_provider_to_view(cx, &default_id);
        }

        // Extract actions for button clicks
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => return,
        };

        // Handle provider panel actions
        for action in actions {
            match action.as_widget_action().cast() {
                ProvidersPanelAction::Selected(id) => {
                    if self.selected_provider_id.as_ref() != Some(&id) {
                        self.selected_provider_id = Some(id.clone());
                        self.load_provider_to_view(cx, &id);
                    }
                }
                ProvidersPanelAction::AddProviderClicked => {
                    self.view
                        .add_provider_modal(ids!(add_provider_modal))
                        .show(cx);
                }
                _ => {}
            }
        }

        // Handle modal actions
        for action in actions {
            match action.as_widget_action().cast() {
                AddProviderModalAction::CancelClicked => {
                    self.view
                        .add_provider_modal(ids!(add_provider_modal))
                        .hide(cx);
                }
                AddProviderModalAction::AddClicked => {
                    let modal = self.view.add_provider_modal(ids!(add_provider_modal));
                    if let Some(provider) = modal.get_provider() {
                        // Play save animation
                        modal.play_save_animation(cx);
                        self.add_custom_provider(cx, provider);
                        modal.hide(cx);
                    }
                }
                _ => {}
            }
        }

        // Handle save button
        if self
            .view
            .button(ids!(content.provider_view.save_button))
            .clicked(actions)
        {
            self.save_current_provider(cx);
        }

        // Handle remove button
        if self
            .view
            .button(ids!(content.provider_view.remove_button))
            .clicked(actions)
        {
            self.remove_current_provider(cx);
        }

        // Handle sync button
        if self
            .view
            .button(ids!(content.provider_view.sync_button))
            .clicked(actions)
        {
            self.sync_models(cx);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl SettingsScreen {
    fn load_provider_to_view(&mut self, cx: &mut Cx, provider_id: &ProviderId) {
        // Load preferences if needed (limited scope to avoid borrow conflicts)
        if self.preferences.is_none() {
            self.preferences = Some(Preferences::load());
        }

        // Update provider item selection state
        // Note: Selection visual state is handled by ProvidersPanel internally
        // Here we just ensure our state is in sync

        // Find the provider and clone the data we need, or use defaults
        let (provider_name, provider_url, api_key, saved_models, has_api_key, is_custom) = {
            if let Some(prefs) = &self.preferences {
                if let Some(provider) = prefs
                    .providers
                    .iter()
                    .find(|p: &&Provider| &p.id == provider_id)
                {
                    (
                        provider.name.clone(),
                        provider.url.clone(),
                        provider.api_key.as_deref().unwrap_or("").to_string(),
                        provider.models.clone(),
                        provider
                            .api_key
                            .as_ref()
                            .map(|k| !k.is_empty())
                            .unwrap_or(false),
                        provider.is_custom,
                    )
                } else {
                    // Use default values for unknown provider
                    let name = match provider_id.as_str() {
                        "openai" => "OpenAI",
                        "deepseek" => "DeepSeek",
                        "alibaba_cloud" => "Alibaba Cloud (Qwen)",
                        "nvidia" => "NVIDIA",
                        _ => provider_id.as_str(),
                    };
                    let url = match provider_id.as_str() {
                        "openai" => "https://api.openai.com/v1",
                        "deepseek" => "https://api.deepseek.com",
                        "alibaba_cloud" => "https://dashscope.aliyuncs.com/compatible-mode/v1",
                        "nvidia" => "https://integrate.api.nvidia.com/v1",
                        _ => "",
                    };
                    (
                        name.to_string(),
                        url.to_string(),
                        "".to_string(),
                        Vec::new(),
                        false,
                        false,
                    )
                }
            } else {
                // No preferences loaded, use defaults
                let name = match provider_id.as_str() {
                    "openai" => "OpenAI",
                    "deepseek" => "DeepSeek",
                    "alibaba_cloud" => "Alibaba Cloud (Qwen)",
                    "nvidia" => "NVIDIA",
                    _ => provider_id.as_str(),
                };
                let url = match provider_id.as_str() {
                    "openai" => "https://api.openai.com/v1",
                    "deepseek" => "https://api.deepseek.com",
                    "alibaba_cloud" => "https://dashscope.aliyuncs.com/compatible-mode/v1",
                    "nvidia" => "https://integrate.api.nvidia.com/v1",
                    _ => "",
                };
                (
                    name.to_string(),
                    url.to_string(),
                    "".to_string(),
                    Vec::new(),
                    false,
                    false,
                )
            }
        };

        // Now use the cloned data to update the view (borrow is released)
        self.view
            .label(ids!(content.provider_view.provider_name))
            .set_text(cx, &provider_name);
        self.view
            .text_input(ids!(content.provider_view.api_host_input))
            .set_text(cx, &provider_url);
        self.view
            .text_input(ids!(content.provider_view.api_key_input))
            .set_text(cx, &api_key);

        // Models are now handled by ProviderView's PortalList - just update status labels
        // Update no models label and sync status
        self.view
            .label(ids!(content.provider_view.no_models_label))
            .set_visible(cx, saved_models.is_empty());
        if !saved_models.is_empty() {
            self.view
                .label(ids!(content.provider_view.sync_status))
                .set_text(cx, &format!("{} models", saved_models.len()));
        } else if has_api_key {
            self.view
                .label(ids!(content.provider_view.sync_status))
                .set_text(cx, "");
        } else {
            self.view
                .label(ids!(content.provider_view.sync_status))
                .set_text(cx, "Enter API key to sync models");
        }

        // Update sync button state based on API key
        if has_api_key {
            self.view
                .button(ids!(content.provider_view.sync_button))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { disabled: 0.0 }
                    },
                );
        } else {
            self.view
                .button(ids!(content.provider_view.sync_button))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { disabled: 1.0 }
                    },
                );
        }

        // Show content, hide empty state
        self.view
            .view(ids!(content.provider_view.content))
            .set_visible(cx, true);
        self.view
            .view(ids!(content.provider_view.empty_state))
            .set_visible(cx, false);
        self.view
            .button(ids!(content.provider_view.remove_button))
            .set_visible(cx, is_custom);

        self.view.redraw(cx);
    }

    fn save_current_provider(&mut self, _cx: &mut Cx) {
        if let Some(provider_id) = &self.selected_provider_id {
            let api_host = self
                .view
                .text_input(ids!(content.provider_view.api_host_input))
                .text();
            let api_key = {
                let key = self
                    .view
                    .text_input(ids!(content.provider_view.api_key_input))
                    .text();
                if key.is_empty() {
                    None
                } else {
                    Some(key)
                }
            };

            // Load preferences if needed and get mutable access
            if self.preferences.is_none() {
                self.preferences = Some(Preferences::load());
            }
            if let Some(prefs) = &mut self.preferences {
                if let Some(provider) = prefs.providers.iter_mut().find(|p| &p.id == provider_id) {
                    provider.url = api_host;
                    provider.api_key = api_key;

                    if let Err(e) = prefs.save() {
                        eprintln!("Failed to save preferences: {}", e);
                    } else {
                        ::log::info!("Saved provider settings for {}", provider_id.as_str());
                    }
                }
            }
        }
    }

    fn remove_current_provider(&mut self, cx: &mut Cx) {
        if let Some(provider_id) = self.selected_provider_id.clone() {
            // Load preferences if needed
            if self.preferences.is_none() {
                self.preferences = Some(Preferences::load());
            }
            if let Some(prefs) = &mut self.preferences {
                // Only remove custom providers
                if let Some(idx) = prefs
                    .providers
                    .iter()
                    .position(|p| p.id == provider_id && p.is_custom)
                {
                    prefs.providers.remove(idx);

                    if let Err(e) = prefs.save() {
                        eprintln!("Failed to save preferences: {}", e);
                    }

                    // Refresh the providers panel to remove the deleted provider
                    self.view
                        .providers_panel(ids!(content.providers_panel))
                        .refresh(cx);

                    // Clear the view and selection
                    self.selected_provider_id = None;
                    self.view
                        .view(ids!(content.provider_view.content))
                        .set_visible(cx, false);
                    self.view
                        .view(ids!(content.provider_view.empty_state))
                        .set_visible(cx, true);
                    self.view
                        .label(ids!(content.provider_view.provider_name))
                        .set_text(cx, "Select a Provider");
                    self.view.redraw(cx);
                }
            }
        }
    }

    fn add_custom_provider(&mut self, cx: &mut Cx, provider: Provider) {
        // Load preferences if needed
        if self.preferences.is_none() {
            self.preferences = Some(Preferences::load());
        }
        if let Some(prefs) = &mut self.preferences {
            let id = provider.id.clone();
            prefs.providers.push(provider);

            if let Err(e) = prefs.save() {
                eprintln!("Failed to save preferences: {}", e);
            }

            // Refresh the providers panel to show the new custom provider
            self.view
                .providers_panel(ids!(content.providers_panel))
                .refresh(cx);

            // Select the new provider
            self.selected_provider_id = Some(id.clone());
            self.view
                .providers_panel(ids!(content.providers_panel))
                .select_and_highlight(cx, &id);
            self.load_provider_to_view(cx, &id);
        }
    }

    fn sync_models(&mut self, cx: &mut Cx) {
        // Check if API key is provided
        let api_key = self
            .view
            .text_input(ids!(content.provider_view.api_key_input))
            .text();
        if api_key.is_empty() {
            self.view
                .label(ids!(content.provider_view.sync_status))
                .set_text(cx, "Enter API key to sync models");
            self.view.redraw(cx);
            return;
        }

        // Set syncing state
        self.view
            .label(ids!(content.provider_view.sync_status))
            .set_text(cx, "Syncing models...");
        self.view
            .button(ids!(content.provider_view.sync_button))
            .apply_over(
                cx,
                live! {
                    draw_bg: { disabled: 1.0 }
                },
            );
        self.view.redraw(cx);

        // For now, simulate fetching models based on provider
        let models = if let Some(provider_id) = &self.selected_provider_id {
            match provider_id.as_str() {
                "openai" => vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "gpt-4-turbo".to_string(),
                    "gpt-4".to_string(),
                    "gpt-3.5-turbo".to_string(),
                ],
                "deepseek" => vec!["deepseek-chat".to_string(), "deepseek-coder".to_string()],
                "alibaba_cloud" => vec![
                    "qwen-turbo".to_string(),
                    "qwen-plus".to_string(),
                    "qwen-max".to_string(),
                ],
                "nvidia" => vec![
                    "deepseek-ai/deepseek-r1".to_string(),
                    "deepseek-ai/deepseek-v3.2".to_string(),
                    "moonshotai/kimi-k2-thinking".to_string(),
                    "minimaxai/minimax-m2".to_string(),
                    "meta/llama-3.3-70b-instruct".to_string(),
                ],
                _ => vec!["custom-model".to_string()],
            }
        } else {
            vec![]
        };

        // Persist models to provider in preferences
        if let Some(provider_id) = &self.selected_provider_id {
            // Load preferences if needed
            if self.preferences.is_none() {
                self.preferences = Some(Preferences::load());
            }
            if let Some(prefs) = &mut self.preferences {
                if let Some(provider) = prefs.providers.iter_mut().find(|p| &p.id == provider_id) {
                    provider.models = models.clone();
                    if let Err(e) = prefs.save() {
                        eprintln!("Failed to save models: {}", e);
                    }
                }
            }
        }

        // Update the no_models_label visibility and sync status
        self.view
            .label(ids!(content.provider_view.no_models_label))
            .set_visible(cx, models.is_empty());

        // Update sync status
        if models.is_empty() {
            self.view
                .label(ids!(content.provider_view.sync_status))
                .set_text(cx, "No models found");
        } else {
            self.view
                .label(ids!(content.provider_view.sync_status))
                .set_text(cx, &format!("Found {} models", models.len()));
        }

        // Re-enable sync button
        self.view
            .button(ids!(content.provider_view.sync_button))
            .apply_over(
                cx,
                live! {
                    draw_bg: { disabled: 0.0 }
                },
            );

        // Reload the provider to view to refresh the model list
        if let Some(provider_id) = self.selected_provider_id.clone() {
            self.load_provider_to_view(cx, &provider_id);
        }
    }
}

impl SettingsScreenRef {
    /// Initialize the settings screen with preferences
    pub fn init(&self, cx: &mut Cx, preferences: Preferences) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.preferences = Some(preferences);
            inner.view.redraw(cx);
        }
    }

    /// Reload preferences from disk
    pub fn reload_preferences(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.preferences = Some(Preferences::load());
            inner.view.redraw(cx);
        }
    }

    /// Update dark mode for this screen
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            // Apply dark mode to screen background
            inner.view.apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );

            // Apply dark mode to vertical divider
            inner.view.view(ids!(content.vertical_divider)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );

            // Apply dark mode to providers panel
            inner
                .view
                .providers_panel(ids!(content.providers_panel))
                .update_dark_mode(cx, dark_mode);

            // Apply dark mode to provider view
            inner
                .view
                .provider_view(ids!(content.provider_view))
                .update_dark_mode(cx, dark_mode);

            // Apply dark mode to add provider modal
            inner
                .view
                .add_provider_modal(ids!(add_provider_modal))
                .update_dark_mode(cx, dark_mode);

            inner.view.redraw(cx);
        }
    }
}
