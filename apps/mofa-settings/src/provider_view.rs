//! Provider View - Right panel for provider configuration

use crate::data::{Provider, ProviderConnectionStatus, ProviderId};
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::*;

    // TextInput for settings fields with proper light/dark mode styling
    SettingsTextInput = <TextInput> {
        width: Fill, height: 44
        padding: {left: 12, right: 12, top: 10, bottom: 10}

        draw_bg: {
            instance dark_mode: 0.0
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, self.radius);
                // Light mode: light gray bg, Dark mode: dark bg
                let light_bg = vec4(0.945, 0.961, 0.976, 1.0);  // SLATE_100
                let dark_bg = vec4(0.20, 0.22, 0.25, 1.0);       // Dark gray
                sdf.fill(mix(light_bg, dark_bg, self.dark_mode));
                // Border
                let light_border = vec4(0.82, 0.84, 0.86, 1.0);  // GRAY_300
                let dark_border = vec4(0.35, 0.38, 0.42, 1.0);   // Dark border
                sdf.stroke(mix(light_border, dark_border, self.dark_mode), 1.0);
                return sdf.result;
            }
        }

        draw_text: {
            instance dark_mode: 0.0
            text_style: <FONT_REGULAR>{ font_size: 13.0 }

            fn get_color(self) -> vec4 {
                // Light mode: dark text, Dark mode: light text
                let light_text = vec4(0.1, 0.1, 0.12, 1.0);      // Near black
                let dark_text = vec4(0.92, 0.93, 0.95, 1.0);     // Near white
                return mix(light_text, dark_text, self.dark_mode);
            }
        }

        draw_selection: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 1.0);
                // Visible blue selection highlight
                sdf.fill(vec4(0.26, 0.52, 0.96, 0.4));  // Blue with 40% opacity
                return sdf.result;
            }
        }
    }

    // Settings label with dark mode
    SettingsLabel = <Label> {
        draw_text: {
            instance dark_mode: 0.0
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
            fn get_color(self) -> vec4 {
                return mix((GRAY_700), (TEXT_PRIMARY_DARK), self.dark_mode);
            }
        }
    }

    // Settings hint label with dark mode
    SettingsHint = <Label> {
        draw_text: {
            instance dark_mode: 0.0
            text_style: <FONT_REGULAR>{ font_size: 10.0 }
            fn get_color(self) -> vec4 {
                return mix((GRAY_500), (TEXT_SECONDARY_DARK), self.dark_mode);
            }
        }
    }

    // Save button style with hover animation
    SaveButton = <Button> {
        width: Fit, height: 40
        padding: {left: 20, right: 20, top: 10, bottom: 10}

        animator: {
            hover = {
                default: off,
                off = {
                    from: {all: Forward {duration: 0.15}}
                    apply: {
                        draw_bg: {hover: 0.0}
                    }
                }
                on = {
                    from: {all: Forward {duration: 0.15}}
                    apply: {
                        draw_bg: {hover: 1.0}
                    }
                }
            }
            pressed = {
                default: off,
                off = {
                    from: {all: Forward {duration: 0.1}}
                    apply: {
                        draw_bg: {pressed: 0.0}
                    }
                }
                on = {
                    from: {all: Forward {duration: 0.1}}
                    apply: {
                        draw_bg: {pressed: 1.0}
                    }
                }
            }
        }

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                // Base color transitions: normal -> hover -> pressed
                let base_color = mix((ACCENT_BLUE), (BLUE_600), self.hover);
                let color = mix(base_color, (BLUE_700), self.pressed);

                // Slight scale effect on hover (glow simulation via larger box)
                let glow = self.hover * 2.0;

                sdf.box(
                    1.0 - glow,
                    1.0 - glow,
                    self.rect_size.x - 2.0 + glow * 2.0,
                    self.rect_size.y - 2.0 + glow * 2.0,
                    self.radius
                );
                sdf.fill(color);

                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
            color: (WHITE)

            fn get_color(self) -> vec4 {
                return (WHITE);
            }
        }

        text: "Save"
    }

    // Remove button style (for custom providers) with hover animation
    RemoveButton = <Button> {
        width: Fit, height: 40
        padding: {left: 20, right: 20, top: 10, bottom: 10}

        animator: {
            hover = {
                default: off,
                off = {
                    from: {all: Forward {duration: 0.15}}
                    apply: { draw_bg: {hover: 0.0} }
                }
                on = {
                    from: {all: Forward {duration: 0.15}}
                    apply: { draw_bg: {hover: 1.0} }
                }
            }
            pressed = {
                default: off,
                off = {
                    from: {all: Forward {duration: 0.1}}
                    apply: { draw_bg: {pressed: 0.0} }
                }
                on = {
                    from: {all: Forward {duration: 0.1}}
                    apply: { draw_bg: {pressed: 1.0} }
                }
            }
        }

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let color = mix(
                    mix((RED_50), (RED_100), self.hover),
                    (RED_200),
                    self.pressed
                );
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, self.radius);
                sdf.fill(color);
                sdf.stroke((ACCENT_RED), 1.0);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
            color: (ACCENT_RED)

            fn get_color(self) -> vec4 {
                return (ACCENT_RED);
            }
        }

        text: "Remove"
    }

    // Sync button - active state with hover animation
    SyncButton = <Button> {
        width: Fit, height: 32
        padding: {left: 12, right: 12, top: 6, bottom: 6}

        animator: {
            hover = {
                default: off,
                off = {
                    from: {all: Forward {duration: 0.15}}
                    apply: { draw_bg: {hover: 0.0} }
                }
                on = {
                    from: {all: Forward {duration: 0.15}}
                    apply: { draw_bg: {hover: 1.0} }
                }
            }
            pressed = {
                default: off,
                off = {
                    from: {all: Forward {duration: 0.1}}
                    apply: { draw_bg: {pressed: 0.0} }
                }
                on = {
                    from: {all: Forward {duration: 0.1}}
                    apply: { draw_bg: {pressed: 1.0} }
                }
            }
        }

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance disabled: 0.0
            instance radius: 4.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let active_color = mix(
                    mix((EMERALD_500), (EMERALD_600), self.hover),
                    (EMERALD_700),
                    self.pressed
                );
                let disabled_color = (GRAY_300);
                let color = mix(active_color, disabled_color, self.disabled);
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, self.radius);
                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 10.0 }
            color: (WHITE)

            fn get_color(self) -> vec4 {
                return (WHITE);
            }
        }

        text: "Sync Models"
    }

    // Model radio item
    ModelRadioItem = <View> {
        width: Fill, height: Fit
        flow: Right
        spacing: 8
        padding: {top: 6, bottom: 6}
        align: {y: 0.5}
        cursor: Hand

        radio_circle = <View> {
            width: 16, height: 16
            show_bg: true
            draw_bg: {
                instance selected: 0.0

                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let center = self.rect_size * 0.5;
                    let radius = min(center.x, center.y) - 1.0;

                    // Outer circle
                    sdf.circle(center.x, center.y, radius);
                    sdf.stroke(mix((GRAY_400), (ACCENT_BLUE), self.selected), 1.5);

                    // Inner dot when selected
                    if self.selected > 0.5 {
                        sdf.circle(center.x, center.y, radius * 0.5);
                        sdf.fill((ACCENT_BLUE));
                    }

                    return sdf.result;
                }
            }
        }

        model_name = <Label> {
            text: "model-name"
            draw_text: {
                color: (GRAY_700)
                text_style: <FONT_REGULAR>{ font_size: 11.0 }
            }
        }
    }

    // Provider view - right panel showing provider details
    pub ProviderView = {{ProviderView}} {
        width: Fill, height: Fill
        flow: Down
        padding: 30
        spacing: 24

        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn get_color(self) -> vec4 {
                return mix((SLATE_50), (SLATE_900), self.dark_mode);
            }
        }

        // Header
        header = <View> {
            width: Fill, height: Fit
            flow: Right
            align: {y: 0.5}
            spacing: 12

            provider_name = <Label> {
                text: "Select a Provider"
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: <FONT_BOLD>{ font_size: 20.0 }
                    fn get_color(self) -> vec4 {
                        return mix((SLATE_800), (TEXT_PRIMARY_DARK), self.dark_mode);
                    }
                }
            }

            // Status indicator
            status_label = <Label> {
                text: ""
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: <FONT_REGULAR>{ font_size: 11.0 }
                    fn get_color(self) -> vec4 {
                        return mix((GRAY_500), (TEXT_SECONDARY_DARK), self.dark_mode);
                    }
                }
            }
        }

        // Content area
        content = <View> {
            width: Fill, height: Fill
            flow: Down
            spacing: 20

            // Provider Name field (for custom providers)
            name_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 6
                visible: false

                name_label = <SettingsLabel> {
                    text: "Provider Name"
                }

                provider_name_input = <SettingsTextInput> {
                    empty_text: "My Custom Provider"
                }

                name_hint = <SettingsHint> {
                    text: "A display name for this provider"
                }
            }

            // API Host field
            host_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 6

                host_label = <SettingsLabel> {
                    text: "API Host"
                }

                api_host_input = <SettingsTextInput> {
                    empty_text: "https://api.example.com/v1"
                }

                host_hint = <SettingsHint> {
                    text: "The base URL for API requests"
                }
            }

            // API Key field
            key_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 6

                key_label = <SettingsLabel> {
                    text: "API Key"
                }

                api_key_input = <SettingsTextInput> {
                    empty_text: "sk-..."
                    is_password: true
                }

                key_hint = <SettingsHint> {
                    text: "Your API key (stored locally)"
                }
            }

            // Available models with sync button
            models_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 8

                // Header row with label and sync button
                models_header = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    align: {y: 0.5}
                    spacing: 12

                    models_label = <SettingsLabel> {
                        text: "Available Models"
                    }

                    <View> { width: Fill, height: 1 }

                    sync_button = <SyncButton> {}
                }

                // Sync status message
                sync_status = <SettingsHint> {
                    width: Fill
                    text: ""
                }

                // Models list container (scrollable)
                models_list_container = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 0
                    padding: {top: 4}

                    // Placeholder when no models
                    no_models_label = <SettingsHint> {
                        width: Fill
                        text: "Click 'Sync Models' to fetch available models"
                    }

                    // Dynamic model list using PortalList
                    models_list = <PortalList> {
                        width: Fill, height: Fit
                        flow: Down

                        model_item = <ModelRadioItem> {}
                    }
                }
            }

            // Spacer
            <View> { width: Fill, height: Fill }

            // Action buttons
            actions = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 12

                save_button = <SaveButton> {}

                remove_button = <RemoveButton> {
                    visible: false
                }
            }
        }

        // Empty state - shown when no provider selected
        empty_state = <View> {
            width: Fill, height: Fill
            align: {x: 0.5, y: 0.5}

            <View> {
                width: Fit, height: Fit
                flow: Down
                align: {x: 0.5, y: 0.5}
                spacing: 8

                empty_title = <Label> {
                    text: "Select a Provider"
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                        fn get_color(self) -> vec4 {
                            return mix((SLATE_400), (SLATE_500), self.dark_mode);
                        }
                    }
                }

                empty_subtitle = <Label> {
                    text: "Choose a provider from the list to configure"
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: <FONT_REGULAR>{ font_size: 11.0 }
                        fn get_color(self) -> vec4 {
                            return mix((SLATE_300), (SLATE_600), self.dark_mode);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct ProviderView {
    #[deref]
    view: View,

    #[rust]
    current_provider_id: Option<ProviderId>,

    #[rust]
    show_content: bool,

    #[rust]
    available_models: Vec<String>,

    #[rust]
    selected_model: Option<String>,

    #[rust]
    is_syncing: bool,

    #[rust]
    selected_model_index: Option<usize>,
}

impl Widget for ProviderView {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Draw the PortalList with model items
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = item.as_portal_list().borrow_mut() {
                // Set the range based on number of models
                list.set_item_range(cx, 0, self.available_models.len());

                while let Some(item_id) = list.next_visible_item(cx) {
                    if item_id < self.available_models.len() {
                        let item = list.item(cx, item_id, live_id!(model_item));
                        let model_name = &self.available_models[item_id];
                        let is_selected = self.selected_model_index == Some(item_id);

                        // Set the model name label
                        item.label(ids!(model_name)).set_text(cx, model_name);

                        // Set the radio button selected state
                        let selected_val = if is_selected { 1.0 } else { 0.0 };
                        item.view(ids!(radio_circle)).apply_over(
                            cx,
                            live! {
                                draw_bg: { selected: (selected_val) }
                            },
                        );

                        item.draw_all(cx, scope);
                    }
                }
            }
        }
        DrawStep::done()
    }
}

impl ProviderViewRef {
    /// Load a provider's details into the view
    pub fn load_provider(&self, cx: &mut Cx, provider: &Provider) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.current_provider_id = Some(provider.id.clone());
            inner.show_content = true;
            inner.available_models = provider.models.clone();
            inner.selected_model = provider.models.first().cloned();
            inner.selected_model_index = if provider.models.is_empty() {
                None
            } else {
                Some(0)
            };

            // Update header
            inner
                .view
                .label(ids!(provider_name))
                .set_text(cx, &provider.name);

            // Show content, hide empty state
            inner.view.view(ids!(content)).set_visible(cx, true);
            inner.view.view(ids!(empty_state)).set_visible(cx, false);

            // Show name section for custom providers, allow editing name
            inner
                .view
                .view(ids!(content.name_section))
                .set_visible(cx, provider.is_custom);
            if provider.is_custom {
                inner
                    .view
                    .text_input(ids!(provider_name_input))
                    .set_text(cx, &provider.name);
            }

            // Set input values
            inner
                .view
                .text_input(ids!(api_host_input))
                .set_text(cx, &provider.url);
            inner
                .view
                .text_input(ids!(api_key_input))
                .set_text(cx, provider.api_key.as_deref().unwrap_or(""));

            // Update sync button state based on API key
            let has_api_key = provider
                .api_key
                .as_ref()
                .map(|k| !k.is_empty())
                .unwrap_or(false);
            inner.view.button(ids!(sync_button)).apply_over(
                cx,
                live! {
                    draw_bg: { disabled: (if has_api_key { 0.0 } else { 1.0 }) }
                },
            );

            // Update sync status text
            if !has_api_key {
                inner
                    .view
                    .label(ids!(sync_status))
                    .set_text(cx, "Enter API key to sync models");
            } else {
                inner.view.label(ids!(sync_status)).set_text(cx, "");
            }

            // Show models if available
            Self::display_models_internal(&mut inner, cx, &provider.models);

            // Show/hide remove button for custom providers
            inner
                .view
                .button(ids!(remove_button))
                .set_visible(cx, provider.is_custom);

            // Update status
            let status_text = match &provider.connection_status {
                ProviderConnectionStatus::Disconnected => "",
                ProviderConnectionStatus::Connecting => "Connecting...",
                ProviderConnectionStatus::Connected => "Connected",
                ProviderConnectionStatus::Error(_) => "Error",
            };
            inner
                .view
                .label(ids!(status_label))
                .set_text(cx, status_text);

            inner.view.redraw(cx);
        }
    }

    /// Internal helper to display models
    fn display_models_internal(
        inner: &mut std::cell::RefMut<ProviderView>,
        cx: &mut Cx,
        models: &[String],
    ) {
        // Show/hide no models label based on whether we have models
        inner
            .view
            .label(ids!(no_models_label))
            .set_visible(cx, models.is_empty());

        // Store models in the struct - the PortalList will render them in draw_walk
        inner.available_models = models.to_vec();

        // Set first model as selected if any exist and no selection yet
        if !models.is_empty() && inner.selected_model_index.is_none() {
            inner.selected_model_index = Some(0);
            inner.selected_model = Some(models[0].clone());
        }

        // Trigger redraw to update the PortalList
        inner.view.redraw(cx);
    }

    /// Display fetched models
    pub fn display_models(&self, cx: &mut Cx, models: Vec<String>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.is_syncing = false;

            // Reset selection when new models are loaded
            inner.selected_model_index = if models.is_empty() { None } else { Some(0) };
            inner.selected_model = models.first().cloned();

            Self::display_models_internal(&mut inner, cx, &models);

            if models.is_empty() {
                inner
                    .view
                    .label(ids!(sync_status))
                    .set_text(cx, "No models found");
            } else {
                inner
                    .view
                    .label(ids!(sync_status))
                    .set_text(cx, &format!("Found {} models", models.len()));
            }

            inner.view.redraw(cx);
        }
    }

    /// Clear the view (show empty state)
    pub fn clear(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.current_provider_id = None;
            inner.show_content = false;
            inner.available_models.clear();
            inner.selected_model = None;
            inner.selected_model_index = None;

            // Hide content, show empty state
            inner.view.view(ids!(content)).set_visible(cx, false);
            inner.view.view(ids!(empty_state)).set_visible(cx, true);

            inner
                .view
                .label(ids!(provider_name))
                .set_text(cx, "Select a Provider");
            inner.view.label(ids!(status_label)).set_text(cx, "");

            inner.view.redraw(cx);
        }
    }

    /// Get the current provider ID being edited
    pub fn current_provider_id(&self) -> Option<ProviderId> {
        self.borrow()
            .and_then(|inner| inner.current_provider_id.clone())
    }

    /// Get the current form values (name, api_host, api_key)
    pub fn get_form_values(&self) -> Option<(Option<String>, String, Option<String>)> {
        self.borrow().map(|inner| {
            let name = {
                let n = inner.view.text_input(ids!(provider_name_input)).text();
                if n.is_empty() {
                    None
                } else {
                    Some(n)
                }
            };
            let api_host = inner.view.text_input(ids!(api_host_input)).text();
            let api_key = {
                let key = inner.view.text_input(ids!(api_key_input)).text();
                if key.is_empty() {
                    None
                } else {
                    Some(key)
                }
            };

            (name, api_host, api_key)
        })
    }

    /// Update dark mode for this widget
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            // Main container background
            inner.view.apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );

            // Header labels
            inner.view.label(ids!(header.provider_name)).apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
            inner.view.label(ids!(header.status_label)).apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

            // Name section (for custom providers)
            inner
                .view
                .label(ids!(content.name_section.name_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(content.name_section.name_hint))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .text_input(ids!(content.name_section.provider_name_input))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Host section
            inner
                .view
                .label(ids!(content.host_section.host_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(content.host_section.host_hint))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .text_input(ids!(content.host_section.api_host_input))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Key section
            inner
                .view
                .label(ids!(content.key_section.key_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(content.key_section.key_hint))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .text_input(ids!(content.key_section.api_key_input))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Models section
            inner
                .view
                .label(ids!(content.models_section.models_header.models_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(content.models_section.sync_status))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(
                    content.models_section.models_list_container.no_models_label
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Empty state labels
            inner.view.label(ids!(empty_state.empty_title)).apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
            inner
                .view
                .label(ids!(empty_state.empty_subtitle))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            inner.view.redraw(cx);
        }
    }
}
