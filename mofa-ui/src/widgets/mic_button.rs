//! Microphone Toggle Button Widget
//!
//! A toggle button showing mic on/off icons with muted state visualization.
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::widgets::mic_button::*;
//!
//!     mic_btn = <MicButton> {}
//! }
//! ```
//!
//! ## Handling Clicks
//!
//! ```rust,ignore
//! // In handle_event
//! if self.view.mic_button(id!(mic_btn)).clicked(&actions) {
//!     let is_muted = self.view.mic_button(id!(mic_btn)).is_muted();
//!     // Toggle mic state
//! }
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    /// Microphone toggle button with on/off icons and recording indicator
    pub MicButton = {{MicButton}} {
        width: Fit
        height: Fit
        flow: Overlay
        cursor: Hand
        padding: 4

        // Background with recording indicator (pulsing when active)
        show_bg: true
        draw_bg: {
            instance recording: 0.0  // 1.0 = recording (not muted), 0.0 = muted

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);

                // Recording indicator: pulsing red dot in top-right when active
                let dot_x = self.rect_size.x - 6.0;
                let dot_y = 4.0;
                let dot_radius = 3.0;
                let dist = length(self.pos * self.rect_size - vec2(dot_x, dot_y));

                // Pulse animation (only when recording)
                let pulse = (sin(self.time * 4.0) * 0.3 + 0.7) * self.recording;
                let red = vec4(0.937, 0.267, 0.267, 1.0);

                // Draw pulsing red dot when recording
                if dist < dot_radius && self.recording > 0.5 {
                    sdf.fill(mix(red, vec4(1.0, 0.4, 0.4, 1.0), pulse));
                } else {
                    sdf.fill(vec4(0.0, 0.0, 0.0, 0.0));  // Transparent background
                }

                return sdf.result;
            }
        }

        mic_icon_on = <View> {
            width: Fit, height: Fit
            icon = <Icon> {
                draw_icon: {
                    instance dark_mode: 0.0
                    svg_file: dep("crate://self/resources/icons/mic.svg")
                    fn get_color(self) -> vec4 {
                        return mix(
                            vec4(0.392, 0.455, 0.545, 1.0),  // SLATE_500
                            vec4(1.0, 1.0, 1.0, 1.0),        // WHITE
                            self.dark_mode
                        );
                    }
                }
                icon_walk: {width: 20, height: 20}
            }
        }

        mic_icon_off = <View> {
            width: Fit, height: Fit
            visible: false
            <Icon> {
                draw_icon: {
                    svg_file: dep("crate://self/resources/icons/mic-off.svg")
                    fn get_color(self) -> vec4 {
                        return vec4(0.937, 0.267, 0.267, 1.0);  // ACCENT_RED
                    }
                }
                icon_walk: {width: 20, height: 20}
            }
        }
    }
}

/// Actions emitted by MicButton
#[derive(Clone, Debug, DefaultNone)]
pub enum MicButtonAction {
    None,
    /// Mic button was clicked
    Clicked,
    /// Mic state changed (contains new muted state)
    StateChanged(bool),
}

#[derive(Live, LiveHook, Widget)]
pub struct MicButton {
    #[deref]
    view: View,

    /// Whether the mic is muted
    #[rust]
    muted: bool,

    /// Whether actively recording (for blinking indicator)
    #[rust]
    recording: bool,

    /// Current dark mode value
    #[rust]
    dark_mode: f64,
}

impl Widget for MicButton {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Handle click
        match event.hits(cx, self.view.area()) {
            Hit::FingerUp(fe) if fe.is_over && fe.was_tap() => {
                cx.widget_action(self.widget_uid(), &scope.path, MicButtonAction::Clicked);
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl MicButton {
    /// Check if mic is muted
    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// Set muted state (only updates icon visibility, not recording indicator)
    pub fn set_muted(&mut self, cx: &mut Cx, muted: bool) {
        self.muted = muted;
        self.update_display(cx);
    }

    /// Toggle muted state
    pub fn toggle(&mut self, cx: &mut Cx) {
        self.muted = !self.muted;
        self.update_display(cx);
    }

    /// Set recording state (for external control, e.g., dora connection)
    pub fn set_recording(&mut self, cx: &mut Cx, recording: bool) {
        self.recording = recording;
        self.update_shader(cx);
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&mut self, cx: &mut Cx, dark_mode: f64) {
        self.dark_mode = dark_mode;
        self.view
            .view(ids!(mic_icon_on))
            .icon(ids!(icon))
            .apply_over(
                cx,
                live! {
                    draw_icon: { dark_mode: (dark_mode) }
                },
            );
        self.view.redraw(cx);
    }

    /// Update display (icon visibility and shader)
    fn update_display(&mut self, cx: &mut Cx) {
        self.view
            .view(ids!(mic_icon_on))
            .set_visible(cx, !self.muted);
        self.view
            .view(ids!(mic_icon_off))
            .set_visible(cx, self.muted);
        self.update_shader(cx);
    }

    /// Update shader instance variables
    fn update_shader(&mut self, cx: &mut Cx) {
        self.view.apply_over(
            cx,
            live! {
                draw_bg: {
                    recording: (if self.recording { 1.0 } else { 0.0 }),
                }
            },
        );
        self.view.redraw(cx);
    }
}

impl MicButtonRef {
    /// Check if mic is muted
    pub fn is_muted(&self) -> bool {
        self.borrow().map(|inner| inner.is_muted()).unwrap_or(false)
    }

    /// Set muted state
    pub fn set_muted(&self, cx: &mut Cx, muted: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_muted(cx, muted);
        }
    }

    /// Toggle muted state
    pub fn toggle(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.toggle(cx);
        }
    }

    /// Set recording state (for external control)
    pub fn set_recording(&self, cx: &mut Cx, recording: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_recording(cx, recording);
        }
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }

    /// Check if clicked in actions
    pub fn clicked(&self, actions: &Actions) -> bool {
        // Search all widget actions for MicButtonAction::Clicked
        actions
            .iter()
            .filter_map(|a| a.as_widget_action())
            .any(|a| matches!(a.cast(), MicButtonAction::Clicked))
    }
}
