//! Provider Selector Widget
//!
//! A dropdown selector for AI providers with optional model selection.
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::widgets::provider_selector::*;
//!
//!     provider = <ProviderSelector> {
//!         label: "Provider"
//!     }
//! }
//! ```
//!
//! ## Handling Selection
//!
//! ```rust,ignore
//! let selector = self.view.provider_selector(id!(provider));
//! if let Some(provider_id) = selector.provider_changed(&actions) {
//!     // Handle provider selection change
//! }
//! if let Some(model) = selector.model_changed(&actions) {
//!     // Handle model selection change
//! }
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Light/Dark mode color constants
    TEXT_PRIMARY = vec4(0.067, 0.090, 0.125, 1.0)
    TEXT_PRIMARY_DARK = vec4(0.945, 0.961, 0.976, 1.0)
    TEXT_SECONDARY = vec4(0.392, 0.455, 0.545, 1.0)
    TEXT_SECONDARY_DARK = vec4(0.580, 0.639, 0.722, 1.0)
    SLATE_100 = vec4(0.945, 0.961, 0.976, 1.0)
    SLATE_300 = vec4(0.796, 0.835, 0.878, 1.0)
    SLATE_500 = vec4(0.392, 0.455, 0.545, 1.0)
    SLATE_600 = vec4(0.278, 0.337, 0.412, 1.0)
    SLATE_700 = vec4(0.204, 0.224, 0.275, 1.0)
    WHITE = vec4(1.0, 1.0, 1.0, 1.0)
    GRAY_100 = vec4(0.953, 0.957, 0.961, 1.0)

    /// Styled dropdown for provider/model selection
    SelectorDropDown = <DropDown> {
        width: Fill, height: 36
        draw_bg: {
            instance dark_mode: 0.0
            border_radius: 4.0
            border_size: 1.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                sdf.fill(bg);
                sdf.stroke(border, self.border_size);
                return sdf.result;
            }
        }
        draw_text: {
            instance dark_mode: 0.0
            text_style: { font_size: 12.0 }
            fn get_color(self) -> vec4 {
                return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
            }
        }
        popup_menu: {
            draw_bg: {
                instance dark_mode: 0.0
                border_size: 1.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                    let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                    let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                    sdf.fill(bg);
                    sdf.stroke(border, self.border_size);
                    return sdf.result;
                }
            }
            menu_item: {
                indent_width: 10.0
                padding: {left: 15, top: 8, bottom: 8, right: 15}
                draw_bg: {
                    instance dark_mode: 0.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                        let base = mix((WHITE), (SLATE_700), self.dark_mode);
                        let hover_color = mix((GRAY_100), (SLATE_600), self.dark_mode);
                        sdf.fill(mix(base, hover_color, self.hover));
                        return sdf.result;
                    }
                }
                draw_text: {
                    instance dark_mode: 0.0
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                    }
                }
            }
        }
    }

    /// Provider Selector Widget
    pub ProviderSelector = {{ProviderSelector}} {
        width: Fill, height: Fit
        flow: Down
        spacing: 12

        // Provider selection row
        provider_section = <View> {
            width: Fill, height: Fit
            flow: Down
            spacing: 6

            provider_label = <Label> {
                text: "Provider"
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 12.0 }
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                    }
                }
            }

            provider_dropdown = <SelectorDropDown> {
                labels: ["Select a provider"]
                selected_item: 0
            }
        }

        // Model selection row (optional, shown when provider has models)
        model_section = <View> {
            width: Fill, height: Fit
            flow: Down
            spacing: 6
            visible: false

            model_label = <Label> {
                text: "Model"
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 12.0 }
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                    }
                }
            }

            model_dropdown = <SelectorDropDown> {
                labels: ["Select a model"]
                selected_item: 0
            }
        }

        // Status/hint text
        status_section = <View> {
            width: Fill, height: Fit
            visible: false

            status_label = <Label> {
                text: ""
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 10.0 }
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                    }
                }
            }
        }
    }
}

/// Provider information
#[derive(Clone, Debug, Default)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub models: Vec<String>,
}

/// Actions emitted by ProviderSelector
#[derive(Clone, Debug, DefaultNone)]
pub enum ProviderSelectorAction {
    None,
    /// Provider selection changed
    ProviderChanged(String),
    /// Model selection changed
    ModelChanged(String),
}

#[derive(Live, LiveHook, Widget)]
pub struct ProviderSelector {
    #[deref]
    view: View,

    /// Current dark mode value
    #[rust]
    dark_mode: f64,

    /// Available providers
    #[rust]
    providers: Vec<ProviderInfo>,

    /// Currently selected provider index
    #[rust]
    selected_provider_index: Option<usize>,

    /// Whether to show model selection
    #[live]
    show_models: bool,

    /// Provider label text
    #[live]
    provider_label: String,

    /// Model label text
    #[live]
    model_label: String,
}

impl Widget for ProviderSelector {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let actions = cx.capture_actions(|cx| self.view.handle_event(cx, event, scope));

        // Handle provider dropdown change
        let provider_dd = self
            .view
            .drop_down(ids!(provider_section.provider_dropdown));
        if let Some(idx) = provider_dd.changed(&actions) {
            if idx < self.providers.len() {
                self.selected_provider_index = Some(idx);
                let provider = &self.providers[idx];

                // Update model dropdown if provider has models
                if !provider.models.is_empty() && self.show_models {
                    self.view.view(ids!(model_section)).set_visible(cx, true);
                    self.view
                        .drop_down(ids!(model_section.model_dropdown))
                        .set_labels(cx, provider.models.clone());
                    self.view
                        .drop_down(ids!(model_section.model_dropdown))
                        .set_selected_item(cx, 0);
                } else {
                    self.view.view(ids!(model_section)).set_visible(cx, false);
                }

                cx.widget_action(
                    self.widget_uid(),
                    &scope.path,
                    ProviderSelectorAction::ProviderChanged(provider.id.clone()),
                );
                self.view.redraw(cx);
            }
        }

        // Handle model dropdown change
        let model_dd = self.view.drop_down(ids!(model_section.model_dropdown));
        if let Some(idx) = model_dd.changed(&actions) {
            if let Some(provider_idx) = self.selected_provider_index {
                if provider_idx < self.providers.len() {
                    let provider = &self.providers[provider_idx];
                    if idx < provider.models.len() {
                        cx.widget_action(
                            self.widget_uid(),
                            &scope.path,
                            ProviderSelectorAction::ModelChanged(provider.models[idx].clone()),
                        );
                    }
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl ProviderSelector {
    /// Set available providers
    pub fn set_providers(&mut self, cx: &mut Cx, providers: Vec<ProviderInfo>) {
        self.providers = providers.clone();

        let labels: Vec<String> = providers.iter().map(|p| p.name.clone()).collect();
        self.view
            .drop_down(ids!(provider_section.provider_dropdown))
            .set_labels(cx, labels);

        if !providers.is_empty() {
            self.selected_provider_index = Some(0);
            self.view
                .drop_down(ids!(provider_section.provider_dropdown))
                .set_selected_item(cx, 0);

            // Show models for first provider if available
            if !providers[0].models.is_empty() && self.show_models {
                self.view.view(ids!(model_section)).set_visible(cx, true);
                self.view
                    .drop_down(ids!(model_section.model_dropdown))
                    .set_labels(cx, providers[0].models.clone());
            }
        }

        self.view.redraw(cx);
    }

    /// Set selected provider by ID
    pub fn set_selected_provider(&mut self, cx: &mut Cx, provider_id: &str) {
        if let Some(idx) = self.providers.iter().position(|p| p.id == provider_id) {
            self.selected_provider_index = Some(idx);
            self.view
                .drop_down(ids!(provider_section.provider_dropdown))
                .set_selected_item(cx, idx);

            let provider = &self.providers[idx];
            if !provider.models.is_empty() && self.show_models {
                self.view.view(ids!(model_section)).set_visible(cx, true);
                self.view
                    .drop_down(ids!(model_section.model_dropdown))
                    .set_labels(cx, provider.models.clone());
            } else {
                self.view.view(ids!(model_section)).set_visible(cx, false);
            }

            self.view.redraw(cx);
        }
    }

    /// Set selected model by name
    pub fn set_selected_model(&mut self, cx: &mut Cx, model: &str) {
        if let Some(provider_idx) = self.selected_provider_index {
            if provider_idx < self.providers.len() {
                let provider = &self.providers[provider_idx];
                if let Some(idx) = provider.models.iter().position(|m| m == model) {
                    self.view
                        .drop_down(ids!(model_section.model_dropdown))
                        .set_selected_item(cx, idx);
                    self.view.redraw(cx);
                }
            }
        }
    }

    /// Get currently selected provider
    pub fn get_selected_provider(&self) -> Option<&ProviderInfo> {
        self.selected_provider_index
            .and_then(|idx| self.providers.get(idx))
    }

    /// Get currently selected model
    pub fn get_selected_model(&self) -> Option<String> {
        let provider_idx = self.selected_provider_index?;
        let provider = self.providers.get(provider_idx)?;
        let model_dd = self.view.drop_down(ids!(model_section.model_dropdown));
        let model_idx = model_dd.selected_item();
        provider.models.get(model_idx).cloned()
    }

    /// Set status text
    pub fn set_status(&mut self, cx: &mut Cx, status: Option<&str>) {
        match status {
            Some(text) => {
                self.view.view(ids!(status_section)).set_visible(cx, true);
                self.view
                    .label(ids!(status_section.status_label))
                    .set_text(cx, text);
            }
            None => {
                self.view.view(ids!(status_section)).set_visible(cx, false);
            }
        }
        self.view.redraw(cx);
    }

    /// Set provider label text
    pub fn set_provider_label(&mut self, cx: &mut Cx, label: &str) {
        self.provider_label = label.to_string();
        self.view
            .label(ids!(provider_section.provider_label))
            .set_text(cx, label);
    }

    /// Set model label text
    pub fn set_model_label(&mut self, cx: &mut Cx, label: &str) {
        self.model_label = label.to_string();
        self.view
            .label(ids!(model_section.model_label))
            .set_text(cx, label);
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&mut self, cx: &mut Cx, dark_mode: f64) {
        self.dark_mode = dark_mode;

        // Provider section
        self.view
            .label(ids!(provider_section.provider_label))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
        self.view
            .drop_down(ids!(provider_section.provider_dropdown))
            .apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                    draw_text: { dark_mode: (dark_mode) }
                    popup_menu: {
                        draw_bg: { dark_mode: (dark_mode) }
                        menu_item: {
                            draw_bg: { dark_mode: (dark_mode) }
                            draw_text: { dark_mode: (dark_mode) }
                        }
                    }
                },
            );

        // Model section
        self.view.label(ids!(model_section.model_label)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );
        self.view
            .drop_down(ids!(model_section.model_dropdown))
            .apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                    draw_text: { dark_mode: (dark_mode) }
                    popup_menu: {
                        draw_bg: { dark_mode: (dark_mode) }
                        menu_item: {
                            draw_bg: { dark_mode: (dark_mode) }
                            draw_text: { dark_mode: (dark_mode) }
                        }
                    }
                },
            );

        // Status section
        self.view
            .label(ids!(status_section.status_label))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

        self.view.redraw(cx);
    }
}

impl ProviderSelectorRef {
    /// Set available providers
    pub fn set_providers(&self, cx: &mut Cx, providers: Vec<ProviderInfo>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_providers(cx, providers);
        }
    }

    /// Set selected provider by ID
    pub fn set_selected_provider(&self, cx: &mut Cx, provider_id: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_selected_provider(cx, provider_id);
        }
    }

    /// Set selected model by name
    pub fn set_selected_model(&self, cx: &mut Cx, model: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_selected_model(cx, model);
        }
    }

    /// Get currently selected provider ID
    pub fn get_selected_provider_id(&self) -> Option<String> {
        self.borrow()
            .and_then(|inner| inner.get_selected_provider().map(|p| p.id.clone()))
    }

    /// Get currently selected model
    pub fn get_selected_model(&self) -> Option<String> {
        self.borrow().and_then(|inner| inner.get_selected_model())
    }

    /// Set status text
    pub fn set_status(&self, cx: &mut Cx, status: Option<&str>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_status(cx, status);
        }
    }

    /// Set provider label text
    pub fn set_provider_label(&self, cx: &mut Cx, label: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_provider_label(cx, label);
        }
    }

    /// Set model label text
    pub fn set_model_label(&self, cx: &mut Cx, label: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_model_label(cx, label);
        }
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }

    /// Check if provider selection changed
    pub fn provider_changed(&self, actions: &Actions) -> Option<String> {
        if let ProviderSelectorAction::ProviderChanged(id) =
            actions.find_widget_action(self.widget_uid()).cast()
        {
            Some(id)
        } else {
            None
        }
    }

    /// Check if model selection changed
    pub fn model_changed(&self, actions: &Actions) -> Option<String> {
        if let ProviderSelectorAction::ModelChanged(model) =
            actions.find_widget_action(self.widget_uid()).cast()
        {
            Some(model)
        } else {
            None
        }
    }
}
