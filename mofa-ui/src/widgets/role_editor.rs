//! Role Editor Widget
//!
//! A comprehensive configuration editor for roles with model/voice selection
//! and system prompt editing.
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::widgets::role_editor::*;
//!
//!     role_config = <RoleEditor> {
//!         role_title: "Student 1"
//!     }
//! }
//! ```
//!
//! ## Handling Changes
//!
//! ```rust,ignore
//! let editor = self.view.role_editor(id!(role_config));
//! if editor.saved(&actions) {
//!     let config = editor.get_config();
//!     // Save config to file
//! }
//! if let Some(model) = editor.model_changed(&actions) {
//!     // Handle model change
//! }
//! if let Some(voice) = editor.voice_changed(&actions) {
//!     // Handle voice change
//! }
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Light/Dark mode color constants as vec4
    PANEL_BG = vec4(0.976, 0.980, 0.984, 1.0)       // slate-50 light
    PANEL_BG_DARK = vec4(0.204, 0.224, 0.275, 1.0)  // slate-700 dark
    BORDER = vec4(0.878, 0.906, 0.925, 1.0)         // slate-200
    TEXT_PRIMARY = vec4(0.067, 0.090, 0.125, 1.0)   // slate-900
    TEXT_PRIMARY_DARK = vec4(0.945, 0.961, 0.976, 1.0) // slate-100
    TEXT_SECONDARY = vec4(0.392, 0.455, 0.545, 1.0) // slate-500
    TEXT_SECONDARY_DARK = vec4(0.580, 0.639, 0.722, 1.0) // slate-400
    SLATE_50 = vec4(0.976, 0.980, 0.984, 1.0)
    SLATE_100 = vec4(0.945, 0.961, 0.976, 1.0)
    SLATE_300 = vec4(0.796, 0.835, 0.878, 1.0)
    SLATE_400 = vec4(0.580, 0.639, 0.702, 1.0)
    SLATE_500 = vec4(0.392, 0.455, 0.545, 1.0)
    SLATE_600 = vec4(0.278, 0.337, 0.412, 1.0)
    SLATE_700 = vec4(0.204, 0.224, 0.275, 1.0)
    WHITE = vec4(1.0, 1.0, 1.0, 1.0)
    GRAY_100 = vec4(0.953, 0.957, 0.961, 1.0)

    /// Styled dropdown for role config
    ConfigDropDown = <DropDown> {
        width: 200, height: Fit
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

    /// Save button style
    RoleSaveButton = <Button> {
        width: Fit, height: Fit
        padding: {left: 12, right: 12, top: 4, bottom: 4}
        text: "Save"
        draw_text: {
            instance dark_mode: 0.0
            text_style: { font_size: 11.0 }
            fn get_color(self) -> vec4 {
                return (WHITE);
            }
        }
        draw_bg: {
            instance dark_mode: 0.0
            instance hover: 0.0
            instance pressed: 0.0
            instance saved: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                // Blue when normal, green when saved
                let blue = vec4(0.231, 0.510, 0.965, 1.0);
                let green = vec4(0.133, 0.773, 0.373, 1.0);
                let base = mix(blue, green, self.saved);
                let hover_color = mix(vec4(0.369, 0.580, 0.976, 1.0), vec4(0.2, 0.85, 0.45, 1.0), self.saved);
                let pressed_color = mix(vec4(0.188, 0.420, 0.839, 1.0), vec4(0.1, 0.65, 0.3, 1.0), self.saved);
                let color = mix(mix(base, hover_color, self.hover), pressed_color, self.pressed);
                sdf.fill(color);
                return sdf.result;
            }
        }
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
    }

    /// Role configuration section label
    ConfigLabel = <Label> {
        width: 100
        draw_text: {
            instance dark_mode: 0.0
            text_style: { font_size: 12.0 }
            fn get_color(self) -> vec4 {
                return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
            }
        }
    }

    /// Role Editor Widget
    pub RoleEditor = {{RoleEditor}} {
        width: Fill, height: Fit
        padding: 16
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            instance opacity: 1.0
            border_radius: 8.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let bg = mix((SLATE_50), (SLATE_700), self.dark_mode);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                sdf.fill(vec4(bg.x, bg.y, bg.z, bg.w * self.opacity));
                return sdf.result;
            }
        }
        flow: Down
        spacing: 12

        // Header with title and save button
        header = <View> {
            width: Fill, height: Fit
            flow: Right
            spacing: 8
            align: {y: 0.5}

            role_title = <Label> {
                text: "Role"
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 13.0 }
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                    }
                }
            }

            <View> { width: Fill, height: 1 }

            save_btn = <RoleSaveButton> {}
        }

        // Model selection row
        model_row = <View> {
            width: Fill, height: Fit
            flow: Right
            spacing: 12
            align: {y: 0.5}

            model_label = <ConfigLabel> {
                text: "Model"
            }

            model_dropdown = <ConfigDropDown> {
                labels: ["gpt-4o", "gpt-4o-mini", "deepseek-chat"]
                selected_item: 0
            }
        }

        // Voice selection row
        voice_row = <View> {
            width: Fill, height: Fit
            flow: Right
            spacing: 12
            align: {y: 0.5}

            voice_label = <ConfigLabel> {
                text: "Voice"
            }

            voice_dropdown = <ConfigDropDown> {
                labels: ["Zhao Daniu", "Chen Yifan", "Luo Xiang", "Doubao", "Yang Mi", "Ma Yun", "Maple", "Cove", "Ellen", "Juniper"]
                selected_item: 0
            }
        }

        // System Prompt label
        prompt_label = <Label> {
            text: "System Prompt"
            draw_text: {
                instance dark_mode: 0.0
                text_style: { font_size: 12.0 }
                fn get_color(self) -> vec4 {
                    return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                }
            }
        }

        // System prompt editor container
        prompt_container = <RoundedView> {
            width: Fill, height: 120
            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                border_radius: 4.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                    let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                    let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                    sdf.fill(bg);
                    sdf.stroke(border, 1.0);
                    return sdf.result;
                }
            }

            prompt_scroll = <ScrollYView> {
                width: Fill, height: Fill
                scroll_bars: <ScrollBars> {
                    show_scroll_x: false
                    show_scroll_y: true
                    scroll_bar_y: {
                        bar_size: 8.0
                        bar_side_margin: 2.0
                        min_handle_size: 30.0
                        smoothing: 0.15
                        draw_bg: {
                            instance dark_mode: 0.0
                            uniform border_radius: 4.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                if self.is_vertical > 0.5 {
                                    sdf.box(
                                        1.,
                                        self.rect_size.y * self.norm_scroll,
                                        self.rect_size.x - 2.0,
                                        self.rect_size.y * self.norm_handle,
                                        self.border_radius
                                    );
                                } else {
                                    sdf.box(
                                        self.rect_size.x * self.norm_scroll,
                                        1.,
                                        self.rect_size.x * self.norm_handle,
                                        self.rect_size.y - 2.0,
                                        self.border_radius
                                    );
                                }
                                let base = mix(vec4(0.58, 0.64, 0.69, 1.0), vec4(0.39, 0.45, 0.53, 1.0), self.dark_mode);
                                let hover_color = mix(vec4(0.49, 0.55, 0.61, 1.0), vec4(0.49, 0.55, 0.61, 1.0), self.dark_mode);
                                sdf.fill(mix(base, hover_color, self.hover));
                                return sdf.result;
                            }
                        }
                    }
                }

                prompt_wrapper = <View> {
                    width: Fill, height: Fit
                    padding: 8

                    prompt_input = <TextInput> {
                        width: Fill, height: Fit
                        draw_bg: {
                            fn pixel(self) -> vec4 {
                                return vec4(0.0, 0.0, 0.0, 0.0);
                            }
                        }
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: { font_size: 11.0 }
                            fn get_color(self) -> vec4 {
                                return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                            }
                        }
                        draw_selection: {
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 1.0);
                                sdf.fill(vec4(0.26, 0.52, 0.96, 0.4));
                                return sdf.result;
                            }
                        }
                        draw_cursor: {
                            instance focus: 0.0
                            instance blink: 0.0
                            instance dark_mode: 0.0
                            uniform border_radius: 0.5
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius);
                                let cursor_color = mix(vec4(0.1, 0.1, 0.12, 1.0), vec4(0.9, 0.9, 0.95, 1.0), self.dark_mode);
                                sdf.fill(mix(vec4(0.0, 0.0, 0.0, 0.0), cursor_color, (1.0 - self.blink) * self.focus));
                                return sdf.result;
                            }
                        }
                        animator: {
                            blink = {
                                default: off
                                off = {
                                    from: {all: Forward {duration: 0.5}}
                                    apply: { draw_cursor: {blink: 0.0} }
                                }
                                on = {
                                    from: {all: Forward {duration: 0.5}}
                                    apply: { draw_cursor: {blink: 1.0} }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Role configuration data
#[derive(Clone, Debug, Default)]
pub struct RoleConfig {
    pub role_id: String,
    pub model: String,
    pub voice: String,
    pub system_prompt: String,
}

/// Actions emitted by RoleEditor
#[derive(Clone, Debug, DefaultNone)]
pub enum RoleEditorAction {
    None,
    /// Save button clicked, contains current config
    Saved(RoleConfig),
    /// Model selection changed
    ModelChanged(String),
    /// Voice selection changed
    VoiceChanged(String),
    /// System prompt text changed
    PromptChanged(String),
}

#[derive(Live, LiveHook, Widget)]
pub struct RoleEditor {
    #[deref]
    view: View,

    /// Current dark mode value
    #[rust]
    dark_mode: f64,

    /// Role identifier
    #[live]
    role_id: String,

    /// Available models (set programmatically)
    #[rust]
    models: Vec<String>,

    /// Available voices (set programmatically)
    #[rust]
    voices: Vec<String>,

    /// Whether save was recently triggered (for animation)
    #[rust]
    save_animation_active: bool,
}

impl Widget for RoleEditor {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let actions = cx.capture_actions(|cx| self.view.handle_event(cx, event, scope));

        // Handle save button click
        if self.view.button(ids!(header.save_btn)).clicked(&actions) {
            let config = self.get_config();
            cx.widget_action(
                self.widget_uid(),
                &scope.path,
                RoleEditorAction::Saved(config),
            );

            // Animate save button to green
            self.save_animation_active = true;
            self.view.button(ids!(header.save_btn)).apply_over(
                cx,
                live! {
                    draw_bg: { saved: 1.0 }
                    text: "Saved"
                },
            );
            self.view.redraw(cx);

            // Reset after delay (handled by timer or next event)
        }

        // Handle model dropdown change
        let model_dd = self.view.drop_down(ids!(model_row.model_dropdown));
        if let Some(idx) = model_dd.changed(&actions) {
            if idx < self.models.len() {
                let model = self.models[idx].clone();
                cx.widget_action(
                    self.widget_uid(),
                    &scope.path,
                    RoleEditorAction::ModelChanged(model),
                );
            }
        }

        // Handle voice dropdown change
        let voice_dd = self.view.drop_down(ids!(voice_row.voice_dropdown));
        if let Some(idx) = voice_dd.changed(&actions) {
            if idx < self.voices.len() {
                let voice = self.voices[idx].clone();
                cx.widget_action(
                    self.widget_uid(),
                    &scope.path,
                    RoleEditorAction::VoiceChanged(voice),
                );
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl RoleEditor {
    /// Get current configuration
    pub fn get_config(&self) -> RoleConfig {
        let model_dd = self.view.drop_down(ids!(model_row.model_dropdown));
        let voice_dd = self.view.drop_down(ids!(voice_row.voice_dropdown));
        let prompt_input = self.view.text_input(ids!(
            prompt_container.prompt_scroll.prompt_wrapper.prompt_input
        ));

        let model = self
            .models
            .get(model_dd.selected_item())
            .cloned()
            .unwrap_or_default();
        let voice = self
            .voices
            .get(voice_dd.selected_item())
            .cloned()
            .unwrap_or_default();

        RoleConfig {
            role_id: self.role_id.clone(),
            model,
            voice,
            system_prompt: prompt_input.text(),
        }
    }

    /// Set role title
    pub fn set_title(&mut self, cx: &mut Cx, title: &str) {
        self.view.label(ids!(header.role_title)).set_text(cx, title);
    }

    /// Set model options
    pub fn set_models(&mut self, cx: &mut Cx, models: &[String]) {
        self.models = models.to_vec();
        self.view
            .drop_down(ids!(model_row.model_dropdown))
            .set_labels(cx, models.to_vec());
    }

    /// Set voice options
    pub fn set_voices(&mut self, cx: &mut Cx, voices: &[String]) {
        self.voices = voices.to_vec();
        self.view
            .drop_down(ids!(voice_row.voice_dropdown))
            .set_labels(cx, voices.to_vec());
    }

    /// Set selected model by name
    pub fn set_model(&mut self, cx: &mut Cx, model: &str) {
        if let Some(idx) = self.models.iter().position(|l| l == model) {
            self.view
                .drop_down(ids!(model_row.model_dropdown))
                .set_selected_item(cx, idx);
        }
    }

    /// Set selected voice by name
    pub fn set_voice(&mut self, cx: &mut Cx, voice: &str) {
        if let Some(idx) = self.voices.iter().position(|l| l == voice) {
            self.view
                .drop_down(ids!(voice_row.voice_dropdown))
                .set_selected_item(cx, idx);
        }
    }

    /// Set system prompt text
    pub fn set_system_prompt(&mut self, cx: &mut Cx, prompt: &str) {
        self.view
            .text_input(ids!(
                prompt_container.prompt_scroll.prompt_wrapper.prompt_input
            ))
            .set_text(cx, prompt);
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&mut self, cx: &mut Cx, dark_mode: f64) {
        self.dark_mode = dark_mode;

        // Main container
        self.view.apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );

        // Header
        self.view.label(ids!(header.role_title)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );
        self.view.button(ids!(header.save_btn)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            },
        );

        // Model row
        self.view.label(ids!(model_row.model_label)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );
        self.view
            .drop_down(ids!(model_row.model_dropdown))
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

        // Voice row
        self.view.label(ids!(voice_row.voice_label)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );
        self.view
            .drop_down(ids!(voice_row.voice_dropdown))
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

        // Prompt label
        self.view.label(ids!(prompt_label)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );

        // Prompt container
        self.view.view(ids!(prompt_container)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );
        self.view
            .text_input(ids!(
                prompt_container.prompt_scroll.prompt_wrapper.prompt_input
            ))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                    draw_cursor: { dark_mode: (dark_mode) }
                },
            );

        self.view.redraw(cx);
    }

    /// Reset save button after animation
    pub fn reset_save_button(&mut self, cx: &mut Cx) {
        if self.save_animation_active {
            self.save_animation_active = false;
            self.view.button(ids!(header.save_btn)).apply_over(
                cx,
                live! {
                    draw_bg: { saved: 0.0 }
                    text: "Save"
                },
            );
            self.view.redraw(cx);
        }
    }
}

impl RoleEditorRef {
    /// Get current configuration
    pub fn get_config(&self) -> RoleConfig {
        self.borrow()
            .map(|inner| inner.get_config())
            .unwrap_or_default()
    }

    /// Set role title
    pub fn set_title(&self, cx: &mut Cx, title: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_title(cx, title);
        }
    }

    /// Set model options
    pub fn set_models(&self, cx: &mut Cx, models: &[String]) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_models(cx, models);
        }
    }

    /// Set voice options
    pub fn set_voices(&self, cx: &mut Cx, voices: &[String]) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_voices(cx, voices);
        }
    }

    /// Set selected model by name
    pub fn set_model(&self, cx: &mut Cx, model: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_model(cx, model);
        }
    }

    /// Set selected voice by name
    pub fn set_voice(&self, cx: &mut Cx, voice: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_voice(cx, voice);
        }
    }

    /// Set system prompt text
    pub fn set_system_prompt(&self, cx: &mut Cx, prompt: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_system_prompt(cx, prompt);
        }
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }

    /// Reset save button after animation
    pub fn reset_save_button(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.reset_save_button(cx);
        }
    }

    /// Check if saved action was triggered
    pub fn saved(&self, actions: &Actions) -> Option<RoleConfig> {
        if let RoleEditorAction::Saved(config) =
            actions.find_widget_action(self.widget_uid()).cast()
        {
            Some(config)
        } else {
            None
        }
    }

    /// Check if model changed
    pub fn model_changed(&self, actions: &Actions) -> Option<String> {
        if let RoleEditorAction::ModelChanged(model) =
            actions.find_widget_action(self.widget_uid()).cast()
        {
            Some(model)
        } else {
            None
        }
    }

    /// Check if voice changed
    pub fn voice_changed(&self, actions: &Actions) -> Option<String> {
        if let RoleEditorAction::VoiceChanged(voice) =
            actions.find_widget_action(self.widget_uid()).cast()
        {
            Some(voice)
        } else {
            None
        }
    }
}
