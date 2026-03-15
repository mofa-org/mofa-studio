//! Chat Input Widget
//!
//! A prompt input field with send button.
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::widgets::chat_input::*;
//!
//!     prompt = <ChatInput> {}
//! }
//! ```
//!
//! ## Handling Submit
//!
//! ```rust,ignore
//! let input = self.view.chat_input(id!(prompt));
//! if input.submitted(&actions) {
//!     let text = input.text();
//!     // Handle the submitted text
//!     input.clear(cx);
//! }
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Panel styling constants
    PANEL_RADIUS = 8.0

    /// Send button with hover/pressed states
    SendButton = <Button> {
        width: 72, height: 36
        text: "Send"
        padding: {left: 16, right: 16}

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

        draw_text: {
            instance dark_mode: 0.0
            text_style: { font_size: 13.0 }
            fn get_color(self) -> vec4 {
                return vec4(1.0, 1.0, 1.0, 1.0);
            }
        }

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance dark_mode: 0.0
            border_radius: 6.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                // Blue button with hover/pressed states
                let base = vec4(0.231, 0.510, 0.965, 1.0);     // #3b82f6
                let hover_color = vec4(0.369, 0.580, 0.976, 1.0); // lighter
                let pressed_color = vec4(0.188, 0.420, 0.839, 1.0); // darker
                let color = mix(mix(base, hover_color, self.hover), pressed_color, self.pressed);
                sdf.fill(color);
                return sdf.result;
            }
        }
    }

    /// Chat input widget - text input with send button
    pub ChatInput = {{ChatInput}} {
        width: Fill, height: Fit
        flow: Down
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            border_radius: (PANEL_RADIUS)
            border_size: 1.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                let bg = mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                let border = mix((BORDER), (SLATE_600), self.dark_mode);
                sdf.fill(bg);
                sdf.stroke(border, self.border_size);
                return sdf.result;
            }
        }
        padding: 12

        input_row = <View> {
            width: Fill, height: Fit
            flow: Right
            spacing: 8
            align: {y: 0.5}

            text_input = <TextInput> {
                width: Fill, height: 36
                empty_text: "Type a message..."
                draw_bg: {
                    instance dark_mode: 0.0
                    border_radius: 6.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                        let bg = mix(vec4(0.969, 0.973, 0.980, 1.0), vec4(0.118, 0.161, 0.231, 1.0), self.dark_mode);
                        let border = mix((BORDER), (SLATE_600), self.dark_mode);
                        sdf.fill(bg);
                        sdf.stroke(border, 1.0);
                        return sdf.result;
                    }
                }
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 13.0 }
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                    }
                }
                draw_selection: {
                    color: (INDIGO_200)
                }
                draw_cursor: {
                    color: (ACCENT_BLUE)
                }
            }

            send_btn = <SendButton> {}
        }
    }
}

/// Actions emitted by ChatInput
#[derive(Clone, Debug, DefaultNone)]
pub enum ChatInputAction {
    None,
    /// User submitted input (clicked send or pressed Enter)
    Submitted(String),
}

#[derive(Live, LiveHook, Widget)]
pub struct ChatInput {
    #[deref]
    view: View,

    /// Current dark mode value
    #[rust]
    dark_mode: f64,

    /// Placeholder text
    #[live]
    placeholder: String,

    /// Default text to send if input is empty
    #[live]
    default_text: String,
}

impl Widget for ChatInput {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let actions = cx.capture_actions(|cx| self.view.handle_event(cx, event, scope));

        // Check for send button click
        if self.view.button(ids!(input_row.send_btn)).clicked(&actions) {
            self.submit(cx, scope);
        }

        // Check for Enter key in text input
        for action in actions.iter() {
            if let TextInputAction::Returned(..) = action.as_widget_action().cast() {
                self.submit(cx, scope);
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl ChatInput {
    /// Submit the current input
    fn submit(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let text = self.text();
        let submit_text = if text.is_empty() && !self.default_text.is_empty() {
            self.default_text.clone()
        } else {
            text
        };

        if !submit_text.is_empty() {
            cx.widget_action(
                self.widget_uid(),
                &scope.path,
                ChatInputAction::Submitted(submit_text),
            );
            self.clear(cx);
        }
    }

    /// Get current text
    pub fn text(&self) -> String {
        self.view.text_input(ids!(input_row.text_input)).text()
    }

    /// Set text
    pub fn set_text(&mut self, cx: &mut Cx, text: &str) {
        self.view
            .text_input(ids!(input_row.text_input))
            .set_text(cx, text);
    }

    /// Clear input
    pub fn clear(&mut self, cx: &mut Cx) {
        self.view
            .text_input(ids!(input_row.text_input))
            .set_text(cx, "");
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&mut self, cx: &mut Cx, dark_mode: f64) {
        self.dark_mode = dark_mode;
        self.view.apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );
        self.view.text_input(ids!(input_row.text_input)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            },
        );
        self.view.button(ids!(input_row.send_btn)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            },
        );
        self.view.redraw(cx);
    }
}

impl ChatInputRef {
    /// Get current text
    pub fn text(&self) -> String {
        self.borrow().map(|inner| inner.text()).unwrap_or_default()
    }

    /// Set text
    pub fn set_text(&self, cx: &mut Cx, text: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_text(cx, text);
        }
    }

    /// Clear input
    pub fn clear(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.clear(cx);
        }
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }

    /// Check if input was submitted, returns the submitted text
    pub fn submitted(&self, actions: &Actions) -> Option<String> {
        if let ChatInputAction::Submitted(text) =
            actions.find_widget_action(self.widget_uid()).cast()
        {
            Some(text)
        } else {
            None
        }
    }
}
