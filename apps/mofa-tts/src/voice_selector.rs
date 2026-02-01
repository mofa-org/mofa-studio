//! Voice selector component - displays list of available voices

use crate::voice_data::{get_builtin_voices, Voice};
use crate::voice_persistence;
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::*;

    // Voice item in the list
    VoiceItem = <View> {
        width: Fill, height: Fit
        padding: {left: 12, right: 16, top: 10, bottom: 10}
        flow: Right
        align: {y: 0.5}
        spacing: 12
        cursor: Hand

        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            instance selected: 0.0
            instance hover: 0.0

            fn pixel(self) -> vec4 {
                let base = mix((SURFACE), (SURFACE_DARK), self.dark_mode);
                let selected_color = mix((PRIMARY_50), (PRIMARY_900), self.dark_mode);
                let hover_color = mix((SLATE_50), (SLATE_800), self.dark_mode);

                let color = mix(base, selected_color, self.selected);
                let color = mix(color, hover_color, self.hover * (1.0 - self.selected));
                return color;
            }
        }

        // Selection indicator - left edge bar
        selection_indicator = <View> {
            width: 3, height: 36
            show_bg: true
            draw_bg: {
                instance selected: 0.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 1.5);
                    let color = mix(vec4(0.0, 0.0, 0.0, 0.0), (PRIMARY_500), self.selected);
                    sdf.fill(color);
                    return sdf.result;
                }
            }
        }

        // Voice avatar - circular with initial
        avatar = <RoundedView> {
            width: 36, height: 36
            align: {x: 0.5, y: 0.5}
            draw_bg: {
                instance dark_mode: 0.0
                instance selected: 0.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    sdf.circle(18.0, 18.0, 18.0);
                    let base_color = mix((PRIMARY_500), (PRIMARY_400), self.dark_mode);
                    let selected_color = mix((PRIMARY_600), (PRIMARY_300), self.dark_mode);
                    let color = mix(base_color, selected_color, self.selected);
                    sdf.fill(color);
                    return sdf.result;
                }
            }

            // Initial letter
            initial = <Label> {
                width: Fill, height: Fill
                align: {x: 0.3, y: 0.6}
                draw_text: {
                    text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                    fn get_color(self) -> vec4 {
                        return (WHITE);
                    }
                }
                text: "L"
            }
        }

        // Voice info - name and description
        info = <View> {
            width: Fill, height: Fit
            flow: Down
            spacing: 2

            name = <Label> {
                width: Fill, height: Fit
                draw_text: {
                    instance dark_mode: 0.0
                    instance selected: 0.0
                    text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                    fn get_color(self) -> vec4 {
                        let base = mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                        let selected_color = mix((PRIMARY_600), (PRIMARY_300), self.dark_mode);
                        return mix(base, selected_color, self.selected);
                    }
                }
                text: "Voice Name"
            }

            description = <Label> {
                width: Fill, height: Fit
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 11.0 }
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_TERTIARY), (TEXT_TERTIARY_DARK), self.dark_mode);
                    }
                }
                text: "Voice description"
            }
        }

        // Preview button - plays reference audio sample
        preview_btn = <View> {
            width: 28, height: 28
            align: {x: 0.5, y: 0.5}
            cursor: Hand
            visible: true

            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                instance hover: 0.0
                instance playing: 0.0

                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    sdf.circle(14.0, 14.0, 14.0);
                    let base = mix((SLATE_100), (SLATE_700), self.dark_mode);
                    let hover_color = mix((PRIMARY_100), (PRIMARY_700), self.dark_mode);
                    let playing_color = mix((PRIMARY_200), (PRIMARY_600), self.dark_mode);
                    let color = mix(base, hover_color, self.hover);
                    let color = mix(color, playing_color, self.playing);
                    sdf.fill(color);

                    // Draw play triangle or stop square based on playing state
                    if self.playing > 0.5 {
                        // Stop icon (square)
                        sdf.rect(10.0, 10.0, 8.0, 8.0);
                        let icon_color = mix((PRIMARY_600), (PRIMARY_300), self.dark_mode);
                        sdf.fill(icon_color);
                    } else {
                        // Play icon (triangle) - centered
                        sdf.move_to(11.0, 9.0);
                        sdf.line_to(20.0, 14.0);
                        sdf.line_to(11.0, 19.0);
                        sdf.close_path();
                        let icon_color = mix((SLATE_500), (SLATE_400), self.dark_mode);
                        let icon_hover = mix((PRIMARY_600), (PRIMARY_300), self.dark_mode);
                        sdf.fill(mix(icon_color, icon_hover, self.hover));
                    }

                    return sdf.result;
                }
            }
        }

        // Delete button - only for custom voices
        delete_btn = <View> {
            width: 28, height: 28
            align: {x: 0.5, y: 0.5}
            cursor: Hand
            visible: false

            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                instance hover: 0.0

                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    sdf.circle(14.0, 14.0, 14.0);
                    let base = mix((SLATE_100), (SLATE_700), self.dark_mode);
                    let hover_color = mix((RED_100), (RED_700), self.dark_mode);
                    let color = mix(base, hover_color, self.hover);
                    sdf.fill(color);

                    // Draw X icon (delete)
                    let icon_color = mix((SLATE_500), (SLATE_400), self.dark_mode);
                    let icon_hover = mix((RED_600), (RED_300), self.dark_mode);
                    let line_color = mix(icon_color, icon_hover, self.hover);

                    // X lines
                    sdf.move_to(9.0, 9.0);
                    sdf.line_to(19.0, 19.0);
                    sdf.stroke(line_color, 1.5);

                    sdf.move_to(19.0, 9.0);
                    sdf.line_to(9.0, 19.0);
                    sdf.stroke(line_color, 1.5);

                    return sdf.result;
                }
            }
        }
    }

    // Voice selector panel
    pub VoiceSelector = {{VoiceSelector}} {
        width: Fill, height: Fill
        flow: Down

        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((SURFACE), (SURFACE_DARK), self.dark_mode);
            }
        }

        // Header with title and selected voice indicator
        header = <View> {
            width: Fill, height: Fit
            padding: {left: 16, right: 16, top: 14, bottom: 14}
            flow: Down
            spacing: 0
            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    return mix((SLATE_50), (SLATE_800), self.dark_mode);
                }
            }

            // First row: Title + Clone button
            title_row = <View> {
                width: Fill, height: Fit
                flow: Right
                align: {x: 0.0, y: 0.5}
                spacing: 8

                title = <Label> {
                    width: Fit, height: Fit
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                        fn get_color(self) -> vec4 {
                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                        }
                    }
                    text: "Select Voice"
                }

                <View> { width: Fill, height: 1 }

                // Clone voice button
                clone_voice_btn = <Button> {
                    width: Fit, height: 26
                    padding: {left: 10, right: 10}
                    text: "+ Clone"

                    draw_bg: {
                        instance dark_mode: 0.0
                        instance hover: 0.0
                        instance disabled: 0.0
                        border_radius: 4.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);

                            // Disabled state
                            if self.disabled > 0.5 {
                                let disabled_bg = mix((SLATE_100), (SLATE_700), self.dark_mode);
                                sdf.fill(disabled_bg);
                                sdf.stroke(mix((SLATE_300), (SLATE_600), self.dark_mode), 1.0);
                            } else {
                                // Normal state
                                let base = mix((PRIMARY_50), (PRIMARY_900), self.dark_mode);
                                let hover_color = mix((PRIMARY_100), (PRIMARY_800), self.dark_mode);
                                sdf.fill(mix(base, hover_color, self.hover));
                                sdf.stroke(mix((PRIMARY_300), (PRIMARY_600), self.dark_mode), 1.0);
                            }
                            return sdf.result;
                        }
                    }

                    draw_text: {
                        instance dark_mode: 0.0
                        instance disabled: 0.0
                        text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                        fn get_color(self) -> vec4 {
                            if self.disabled > 0.5 {
                                return mix((SLATE_400), (SLATE_500), self.dark_mode);
                            } else {
                                return mix((PRIMARY_600), (PRIMARY_300), self.dark_mode);
                            }
                        }
                    }
                }
            }

            // Second row: Selected voice badge (full width for long names)
            badge_row = <View> {
                width: Fill, height: Fit
                flow: Right
                align: {x: 1.0, y: 0.5}
                margin: {top: 6}

                // Selected voice badge
                selected_voice_badge = <RoundedView> {
                    width: Fit, height: Fit
                    padding: {left: 8, right: 8, top: 4, bottom: 4}
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: 4.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let bg = mix((PRIMARY_100), (PRIMARY_800), self.dark_mode);
                            sdf.fill(bg);
                            return sdf.result;
                        }
                    }

                    selected_voice_label = <Label> {
                        width: Fit, height: Fit
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                            fn get_color(self) -> vec4 {
                                return mix((PRIMARY_700), (PRIMARY_200), self.dark_mode);
                            }
                        }
                        text: "豆包 (Doubao)"
                    }
                }
            }
        }

        // Divider
        <View> {
            width: Fill, height: 1
            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    return mix((BORDER), (BORDER_DARK), self.dark_mode);
                }
            }
        }

        // Voice list with scrolling
        voice_list = <PortalList> {
            width: Fill, height: Fill
            flow: Down

            VoiceItem = <VoiceItem> {}
        }
    }
}

/// Action emitted by voice selector
#[derive(Clone, Debug, DefaultNone)]
pub enum VoiceSelectorAction {
    None,
    VoiceSelected(String),                     // voice_id
    PreviewRequested(String),                  // voice_id
    CloneVoiceClicked,                         // Open clone modal
    RequestStartDora,                          // Request parent to show "please start dora" message
    RequestDeleteConfirmation(String, String), // (voice_id, voice_name) - Request parent to show delete confirmation
    DeleteVoiceClicked(String), // voice_id (custom voices only) - Actually delete the voice
}

#[derive(Live, LiveHook, Widget)]
pub struct VoiceSelector {
    #[deref]
    view: View,

    #[rust]
    voices: Vec<Voice>,

    #[rust]
    custom_voices: Vec<Voice>,

    #[rust]
    selected_voice_id: Option<String>,

    #[rust]
    dark_mode: f64,

    #[rust]
    initialized: bool,

    #[rust]
    preview_playing_voice_id: Option<String>,

    #[rust]
    hovered_preview_idx: Option<usize>,

    #[rust]
    hovered_delete_idx: Option<usize>,

    #[rust]
    dora_running: bool,

    /// Store drawn item areas for hit testing: (item_id, item_area, preview_btn_area, delete_btn_area)
    #[rust]
    item_areas: Vec<(usize, Area, Area, Area)>,
}

impl Widget for VoiceSelector {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Initialize voices on first event
        if !self.initialized {
            self.reload_voices();
            // Select first voice by default
            if let Some(first) = self.voices.first() {
                self.selected_voice_id = Some(first.id.clone());
            }
            self.initialized = true;
        }

        // Handle clone voice button click using event.hits (BEFORE Actions early return)
        let clone_btn = self.view.button(ids!(header.title_row.clone_voice_btn));
        match event.hits(cx, clone_btn.area()) {
            Hit::FingerUp(fe) if fe.was_tap() => {
                if self.dora_running {
                    cx.widget_action(
                        self.widget_uid(),
                        &scope.path,
                        VoiceSelectorAction::CloneVoiceClicked,
                    );
                } else {
                    cx.widget_action(
                        self.widget_uid(),
                        &scope.path,
                        VoiceSelectorAction::RequestStartDora,
                    );
                }
            }
            _ => {}
        }

        // Handle portal list item clicks using stored areas (BEFORE Actions early return)
        for (item_id, item_area, preview_area, delete_area) in self.item_areas.iter().cloned() {
            if item_id >= self.voices.len() {
                continue;
            }

            // Check delete button click first
            match event.hits(cx, delete_area) {
                Hit::FingerUp(fe) if fe.was_tap() => {
                    let voice = &self.voices[item_id];
                    if voice.is_custom() {
                        cx.widget_action(
                            self.widget_uid(),
                            &scope.path,
                            VoiceSelectorAction::RequestDeleteConfirmation(
                                voice.id.clone(),
                                voice.name.clone(),
                            ),
                        );
                        self.view.redraw(cx);
                        continue;
                    }
                }
                Hit::FingerHoverIn(_) => {
                    self.hovered_delete_idx = Some(item_id);
                    self.view.redraw(cx);
                }
                Hit::FingerHoverOut(_) => {
                    if self.hovered_delete_idx == Some(item_id) {
                        self.hovered_delete_idx = None;
                        self.view.redraw(cx);
                    }
                }
                _ => {}
            }

            // Check preview button click
            match event.hits(cx, preview_area) {
                Hit::FingerUp(fe) if fe.was_tap() => {
                    let voice_id = self.voices[item_id].id.clone();
                    // Toggle preview: if same voice is playing, stop it
                    if self.preview_playing_voice_id.as_ref() == Some(&voice_id) {
                        self.preview_playing_voice_id = None;
                    } else {
                        self.preview_playing_voice_id = Some(voice_id.clone());
                    }
                    cx.widget_action(
                        self.widget_uid(),
                        &scope.path,
                        VoiceSelectorAction::PreviewRequested(voice_id),
                    );
                    self.view.redraw(cx);
                    continue;
                }
                Hit::FingerHoverIn(_) => {
                    self.hovered_preview_idx = Some(item_id);
                    self.view.redraw(cx);
                }
                Hit::FingerHoverOut(_) => {
                    if self.hovered_preview_idx == Some(item_id) {
                        self.hovered_preview_idx = None;
                        self.view.redraw(cx);
                    }
                }
                _ => {}
            }

            // Check item click (for selection)
            match event.hits(cx, item_area) {
                Hit::FingerUp(fe) if fe.was_tap() => {
                    let voice_id = self.voices[item_id].id.clone();
                    let voice_name = self.voices[item_id].name.clone();
                    self.selected_voice_id = Some(voice_id.clone());

                    // Update selected voice label in header badge
                    self.view
                        .label(ids!(
                            header.badge_row.selected_voice_badge.selected_voice_label
                        ))
                        .set_text(cx, &voice_name);

                    cx.widget_action(
                        self.widget_uid(),
                        &scope.path,
                        VoiceSelectorAction::VoiceSelected(voice_id),
                    );
                    self.view.redraw(cx);
                }
                _ => {}
            }
        }

        // Extract actions from event - now only for button actions that use .clicked()
        let _actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => return,
        };

        // Note: All click handling is now done above using event.hits()
        // The items_with_actions pattern doesn't work well for View-based list items
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Initialize if needed (in case draw happens before handle_event)
        if !self.initialized {
            self.reload_voices();
            if let Some(first) = self.voices.first() {
                self.selected_voice_id = Some(first.id.clone());
            }
            self.initialized = true;
        }

        // Update clone button disabled state based on dora_running
        let disabled_val = if self.dora_running { 0.0 } else { 1.0 };
        self.view
            .button(ids!(header.title_row.clone_voice_btn))
            .apply_over(
                cx,
                live! {
                    draw_bg: { disabled: (disabled_val) }
                    draw_text: { disabled: (disabled_val) }
                },
            );

        // Clear item areas before redrawing
        self.item_areas.clear();

        // Draw portal list items using borrow pattern
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = item.as_portal_list().borrow_mut() {
                list.set_item_range(cx, 0, self.voices.len());

                while let Some(item_id) = list.next_visible_item(cx) {
                    if item_id < self.voices.len() {
                        let voice = &self.voices[item_id];
                        let item = list.item(cx, item_id, live_id!(VoiceItem));

                        // Set voice data
                        let initial = voice.name.chars().next().unwrap_or('?').to_string();
                        item.label(ids!(avatar.initial)).set_text(cx, &initial);
                        item.label(ids!(info.name)).set_text(cx, &voice.name);
                        item.label(ids!(info.description))
                            .set_text(cx, &voice.description);

                        // Set selection state
                        let is_selected = self.selected_voice_id.as_ref() == Some(&voice.id);
                        let selected_val = if is_selected { 1.0 } else { 0.0 };

                        // Apply selection and dark mode to item background
                        item.apply_over(
                            cx,
                            live! {
                                draw_bg: { selected: (selected_val), dark_mode: (self.dark_mode) }
                            },
                        );

                        // Apply selection indicator
                        item.view(ids!(selection_indicator)).apply_over(
                            cx,
                            live! {
                                draw_bg: { selected: (selected_val) }
                            },
                        );

                        // Apply dark mode and selection to avatar
                        item.view(ids!(avatar)).apply_over(
                            cx,
                            live! {
                                draw_bg: { dark_mode: (self.dark_mode), selected: (selected_val) }
                            },
                        );

                        // Apply dark mode and selection to name label
                        item.label(ids!(info.name)).apply_over(
                            cx,
                            live! {
                                draw_text: { dark_mode: (self.dark_mode), selected: (selected_val) }
                            },
                        );

                        // Apply dark mode to description
                        item.label(ids!(info.description)).apply_over(
                            cx,
                            live! {
                                draw_text: { dark_mode: (self.dark_mode) }
                            },
                        );

                        // Apply preview button state
                        let is_playing = self.preview_playing_voice_id.as_ref() == Some(&voice.id);
                        let playing_val = if is_playing { 1.0 } else { 0.0 };
                        let is_hovered = self.hovered_preview_idx == Some(item_id);
                        let hover_val = if is_hovered { 1.0 } else { 0.0 };
                        item.view(ids!(preview_btn)).apply_over(cx, live! {
                            draw_bg: { dark_mode: (self.dark_mode), playing: (playing_val), hover: (hover_val) }
                        });

                        // Show delete button only for custom voices
                        let is_custom = voice.is_custom();
                        item.view(ids!(delete_btn)).set_visible(cx, is_custom);
                        if is_custom {
                            let delete_hover = if self.hovered_delete_idx == Some(item_id) {
                                1.0
                            } else {
                                0.0
                            };
                            item.view(ids!(delete_btn)).apply_over(
                                cx,
                                live! {
                                    draw_bg: { dark_mode: (self.dark_mode), hover: (delete_hover) }
                                },
                            );
                        }

                        item.draw_all(cx, scope);

                        // Store item areas for hit testing in handle_event
                        let item_area = item.area();
                        let preview_area = item.view(ids!(preview_btn)).area();
                        let delete_area = item.view(ids!(delete_btn)).area();
                        self.item_areas.push((item_id, item_area, preview_area, delete_area));
                    }
                }
            }
        }
        DrawStep::done()
    }
}

impl VoiceSelector {
    /// Reload all voices (built-in + custom)
    fn reload_voices(&mut self) {
        self.voices = get_builtin_voices();
        self.custom_voices = voice_persistence::load_custom_voices();
        // Append custom voices to the main list
        self.voices.extend(self.custom_voices.clone());
    }
}

impl VoiceSelectorRef {
    /// Get currently selected voice
    pub fn selected_voice(&self) -> Option<Voice> {
        if let Some(inner) = self.borrow() {
            if let Some(voice_id) = &inner.selected_voice_id {
                return inner.voices.iter().find(|v| &v.id == voice_id).cloned();
            }
        }
        None
    }

    /// Get selected voice ID
    pub fn selected_voice_id(&self) -> Option<String> {
        self.borrow()
            .and_then(|inner| inner.selected_voice_id.clone())
    }

    /// Get voice by ID
    pub fn get_voice(&self, voice_id: &str) -> Option<Voice> {
        if let Some(inner) = self.borrow() {
            return inner.voices.iter().find(|v| v.id == voice_id).cloned();
        }
        None
    }

    /// Set preview playing state
    pub fn set_preview_playing(&self, cx: &mut Cx, voice_id: Option<String>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.preview_playing_voice_id = voice_id;
            inner.view.redraw(cx);
        }
    }

    /// Check if preview is playing for a voice
    pub fn is_preview_playing(&self, voice_id: &str) -> bool {
        if let Some(inner) = self.borrow() {
            return inner.preview_playing_voice_id.as_ref() == Some(&voice_id.to_string());
        }
        false
    }

    /// Reload voices from storage
    pub fn reload_voices(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.reload_voices();
            inner.view.redraw(cx);
        }
    }

    /// Add a newly created custom voice
    pub fn add_custom_voice(&self, cx: &mut Cx, voice: Voice) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.custom_voices.push(voice.clone());
            inner.voices.push(voice);
            inner.view.redraw(cx);
        }
    }

    /// Delete a custom voice by ID
    pub fn delete_custom_voice(&self, cx: &mut Cx, voice_id: &str) -> Result<(), String> {
        // First remove from persistence
        voice_persistence::remove_custom_voice(voice_id)?;

        // Then update internal state
        if let Some(mut inner) = self.borrow_mut() {
            inner.custom_voices.retain(|v| v.id != voice_id);
            inner.voices.retain(|v| v.id != voice_id);

            // If the deleted voice was selected, select another
            if inner.selected_voice_id.as_ref() == Some(&voice_id.to_string()) {
                inner.selected_voice_id = inner.voices.first().map(|v| v.id.clone());
            }

            inner.view.redraw(cx);
        }

        Ok(())
    }

    /// Set dora running state
    pub fn set_dora_running(&self, cx: &mut Cx, is_running: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.dora_running = is_running;
            inner.view.redraw(cx);
        }
    }

    /// Update dark mode
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.dark_mode = dark_mode;

            inner.view.apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );

            // Header background
            inner.view.view(ids!(header)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );

            // Header title
            inner.view.label(ids!(header.title_row.title)).apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

            // Clone button
            inner
                .view
                .button(ids!(header.title_row.clone_voice_btn))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Selected voice badge
            inner
                .view
                .view(ids!(header.badge_row.selected_voice_badge))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );

            // Selected voice label
            inner
                .view
                .label(ids!(
                    header.badge_row.selected_voice_badge.selected_voice_label
                ))
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
