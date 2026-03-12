//! AEC (Acoustic Echo Cancellation) Toggle Button Widget
//!
//! A toggle button with animated background indicating:
//! - Gray: AEC disabled
//! - Green pulsing: AEC enabled, not speaking
//! - Red pulsing (fast): Speaking detected
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::widgets::aec_button::*;
//!
//!     aec_btn = <AecButton> {}
//! }
//! ```
//!
//! ## Updating State
//!
//! ```rust,ignore
//! // Set enabled state
//! self.view.aec_button(id!(aec_btn)).set_enabled(cx, true);
//!
//! // Set speaking state (from VAD)
//! self.view.aec_button(id!(aec_btn)).set_speaking(cx, is_speaking);
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    /// AEC toggle button with animated speaking indicator
    pub AecButton = {{AecButton}} {
        width: Fit
        height: Fit
        padding: 6
        cursor: Hand
        show_bg: true

        draw_bg: {
            instance enabled: 0.0   // 0.0=off (muted), 1.0=on (recording) - matches Rust default
            instance speaking: 0.0  // 1.0=voice detected, 0.0=silent

            // VAD indicator: red when speaking, green when enabled but silent, gray when disabled
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);

                let red = vec4(0.9, 0.2, 0.2, 1.0);        // Speaking color
                let bright_red = vec4(1.0, 0.3, 0.3, 1.0); // Speaking pulse
                let green = vec4(0.133, 0.773, 0.373, 1.0); // Enabled, silent
                let bright_green = vec4(0.2, 0.9, 0.5, 1.0);
                let gray = vec4(0.667, 0.686, 0.725, 1.0);  // Disabled

                // Fast pulse when speaking (4x speed)
                let speak_pulse = step(0.0, sin(self.time * 8.0)) * self.speaking;
                // Slow pulse when enabled but not speaking
                let idle_pulse = step(0.0, sin(self.time * 2.0)) * self.enabled * (1.0 - self.speaking);

                // Base color: gray (disabled) -> green (enabled) -> red (speaking)
                let base = mix(gray, green, self.enabled);
                let base = mix(base, red, self.speaking * self.enabled);

                // Pulse color
                let pulse_color = mix(bright_green, bright_red, self.speaking);
                let col = mix(base, pulse_color, (speak_pulse + idle_pulse) * 0.5);

                sdf.fill(col);
                return sdf.result;
            }
        }

        align: {x: 0.5, y: 0.5}

        icon = <Icon> {
            draw_icon: {
                svg_file: dep("crate://self/resources/icons/aec.svg")
                fn get_color(self) -> vec4 {
                    return vec4(1.0, 1.0, 1.0, 1.0);  // WHITE
                }
            }
            icon_walk: {width: 20, height: 20}
        }
    }
}

/// Actions emitted by AecButton
#[derive(Clone, Debug, DefaultNone)]
pub enum AecButtonAction {
    None,
    /// Button was clicked
    Clicked,
    /// AEC enabled state changed
    EnabledChanged(bool),
}

#[derive(Live, LiveHook, Widget)]
pub struct AecButton {
    #[deref]
    view: View,

    /// Whether AEC is enabled
    #[rust]
    enabled: bool,

    /// Whether speaking is detected
    #[rust]
    speaking: bool,
}

impl Widget for AecButton {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Handle click - use self.widget_uid() to match what AecButtonRef::clicked() looks for
        match event.hits(cx, self.view.area()) {
            Hit::FingerUp(fe) if fe.is_over && fe.was_tap() => {
                cx.widget_action(self.widget_uid(), &scope.path, AecButtonAction::Clicked);
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl AecButton {
    /// Check if AEC is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set AEC enabled state
    pub fn set_enabled(&mut self, cx: &mut Cx, enabled: bool) {
        self.enabled = enabled;
        self.update_shader(cx);
    }

    /// Toggle AEC enabled state
    pub fn toggle(&mut self, cx: &mut Cx) {
        self.enabled = !self.enabled;
        self.update_shader(cx);
    }

    /// Check if speaking is detected
    pub fn is_speaking(&self) -> bool {
        self.speaking
    }

    /// Set speaking state (from VAD)
    pub fn set_speaking(&mut self, cx: &mut Cx, speaking: bool) {
        self.speaking = speaking;
        self.update_shader(cx);
    }

    /// Update shader instance variables
    fn update_shader(&mut self, cx: &mut Cx) {
        self.view.apply_over(
            cx,
            live! {
                draw_bg: {
                    enabled: (if self.enabled { 1.0 } else { 0.0 }),
                    speaking: (if self.speaking { 1.0 } else { 0.0 }),
                }
            },
        );
        self.view.redraw(cx);
    }
}

impl AecButtonRef {
    /// Check if AEC is enabled
    pub fn is_enabled(&self) -> bool {
        self.borrow()
            .map(|inner| inner.is_enabled())
            .unwrap_or(false)
    }

    /// Set AEC enabled state
    pub fn set_enabled(&self, cx: &mut Cx, enabled: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_enabled(cx, enabled);
        }
    }

    /// Toggle AEC enabled state
    pub fn toggle(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.toggle(cx);
        }
    }

    /// Check if speaking is detected
    pub fn is_speaking(&self) -> bool {
        self.borrow()
            .map(|inner| inner.is_speaking())
            .unwrap_or(false)
    }

    /// Set speaking state
    pub fn set_speaking(&self, cx: &mut Cx, speaking: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_speaking(cx, speaking);
        }
    }

    /// Check if clicked in actions
    pub fn clicked(&self, actions: &Actions) -> bool {
        // Search all widget actions for AecButtonAction::Clicked
        actions
            .iter()
            .filter_map(|a| a.as_widget_action())
            .any(|a| matches!(a.cast(), AecButtonAction::Clicked))
    }
}
