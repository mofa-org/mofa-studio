//! Chat Panel Widget
//!
//! A complete chat display panel with header, scrollable content, and copy button.
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::widgets::chat_panel::*;
//!
//!     chat = <ChatPanel> {}
//! }
//! ```
//!
//! ## Updating Messages
//!
//! ```rust,ignore
//! let chat = self.view.chat_panel(id!(chat));
//! chat.set_messages(cx, &messages);
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Panel styling constants
    PANEL_RADIUS = 8.0
    PANEL_PADDING = 12.0

    /// Copy button with animated feedback
    CopyButton = <View> {
        width: 28, height: 24
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance copied: 0.0
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let c = self.rect_size * 0.5;

                // Light theme gradient colors
                let gray_light = vec4(0.886, 0.910, 0.941, 1.0);
                let blue_light = vec4(0.231, 0.510, 0.965, 1.0);
                let teal_light = vec4(0.078, 0.722, 0.651, 1.0);
                let green_light = vec4(0.133, 0.773, 0.373, 1.0);

                // Dark theme gradient colors
                let gray_dark = vec4(0.334, 0.371, 0.451, 1.0);
                let purple_dark = vec4(0.639, 0.380, 0.957, 1.0);
                let cyan_dark = vec4(0.133, 0.831, 0.894, 1.0);
                let green_dark = vec4(0.290, 0.949, 0.424, 1.0);

                // Select colors based on dark mode
                let gray = mix(gray_light, gray_dark, self.dark_mode);
                let c1 = mix(blue_light, purple_dark, self.dark_mode);
                let c2 = mix(teal_light, cyan_dark, self.dark_mode);
                let c3 = mix(green_light, green_dark, self.dark_mode);

                // Multi-stop gradient based on copied value
                let t = self.copied;
                let bg_color = mix(
                    mix(mix(gray, c1, clamp(t * 3.0, 0.0, 1.0)),
                        c2, clamp((t - 0.33) * 3.0, 0.0, 1.0)),
                    c3, clamp((t - 0.66) * 3.0, 0.0, 1.0)
                );

                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                sdf.fill(bg_color);

                // Icon color
                let icon_base = mix(vec4(0.294, 0.333, 0.388, 1.0), vec4(0.580, 0.639, 0.722, 1.0), self.dark_mode);
                let icon_color = mix(icon_base, vec4(1.0, 1.0, 1.0, 1.0), smoothstep(0.0, 0.3, self.copied));

                // Clipboard icon - back rectangle
                sdf.box(c.x - 4.0, c.y - 2.0, 8.0, 9.0, 1.0);
                sdf.stroke(icon_color, 1.2);

                // Clipboard icon - front rectangle
                sdf.box(c.x - 2.0, c.y - 5.0, 8.0, 9.0, 1.0);
                sdf.fill(bg_color);
                sdf.box(c.x - 2.0, c.y - 5.0, 8.0, 9.0, 1.0);
                sdf.stroke(icon_color, 1.2);

                return sdf.result;
            }
        }
    }

    /// Panel header with title and actions
    ChatPanelHeader = <View> {
        width: Fill, height: Fit
        flow: Right
        align: {y: 0.5}
        padding: {left: 12, right: 12, top: 10, bottom: 10}
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix(vec4(0.973, 0.980, 0.988, 1.0), vec4(0.118, 0.161, 0.231, 1.0), self.dark_mode);
            }
        }

        title = <Label> {
            text: "Chat History"
            draw_text: {
                instance dark_mode: 0.0
                text_style: { font_size: 13.0 }
                fn get_color(self) -> vec4 {
                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                }
            }
        }
        <Filler> {}
        copy_btn = <CopyButton> {}
    }

    /// Chat panel widget - displays chat messages with header and copy button
    pub ChatPanel = {{ChatPanel}} {
        width: Fill, height: Fill
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

        header = <ChatPanelHeader> {}

        chat_scroll = <ScrollYView> {
            width: Fill, height: Fill
            flow: Down
            scroll_bars: <ScrollBars> {
                show_scroll_x: false
                show_scroll_y: true
            }

            content_wrapper = <View> {
                width: Fill, height: Fit
                padding: (PANEL_PADDING)
                flow: Down

                content = <Markdown> {
                    width: Fill, height: Fit
                    font_size: 13.0
                    font_color: (TEXT_PRIMARY)
                    paragraph_spacing: 8

                    draw_normal: {
                        text_style: { font_size: 13.0 }
                    }
                    draw_bold: {
                        text_style: { font_size: 13.0 }
                    }
                }
            }
        }
    }
}

/// Chat message entry for display
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: u64,
    pub is_streaming: bool,
}

impl ChatMessage {
    pub fn new(sender: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            sender: sender.into(),
            content: content.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            is_streaming: false,
        }
    }

    /// Format Unix timestamp (milliseconds) to HH:MM:SS
    pub fn format_timestamp(timestamp_ms: u64) -> String {
        let total_secs = timestamp_ms / 1000;
        let secs_in_day = total_secs % 86400;
        let hours = secs_in_day / 3600;
        let minutes = (secs_in_day % 3600) / 60;
        let seconds = secs_in_day % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }
}

/// Actions emitted by ChatPanel
#[derive(Clone, Debug, DefaultNone)]
pub enum ChatPanelAction {
    None,
    /// Copy button was clicked
    CopyClicked,
}

#[derive(Live, LiveHook, Widget)]
pub struct ChatPanel {
    #[deref]
    view: View,

    /// Current dark mode value
    #[rust]
    dark_mode: f64,

    /// Last message count (for auto-scroll)
    #[rust]
    last_message_count: usize,

    /// Empty state text
    #[live]
    empty_text: String,
}

impl Widget for ChatPanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Handle copy button click
        let copy_btn = self.view.view(ids!(header.copy_btn));
        match event.hits(cx, copy_btn.area()) {
            Hit::FingerUp(fe) if fe.was_tap() => {
                cx.widget_action(self.widget_uid(), &scope.path, ChatPanelAction::CopyClicked);
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl ChatPanel {
    /// Set messages and update display
    pub fn set_messages(&mut self, cx: &mut Cx, messages: &[ChatMessage]) {
        let text = if messages.is_empty() {
            if self.empty_text.is_empty() {
                "Waiting for conversation...".to_string()
            } else {
                self.empty_text.clone()
            }
        } else {
            messages
                .iter()
                .map(|msg| {
                    let timestamp = ChatMessage::format_timestamp(msg.timestamp);
                    let streaming = if msg.is_streaming { " ⌛" } else { "" };
                    format!(
                        "**{}**{} ({}):  \n{}",
                        msg.sender, streaming, timestamp, msg.content
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n---\n\n")
        };

        self.view
            .markdown(ids!(chat_scroll.content_wrapper.content))
            .set_text(cx, &text);

        // Auto-scroll to bottom on new messages
        if messages.len() > self.last_message_count {
            self.view
                .view(ids!(chat_scroll))
                .set_scroll_pos(cx, DVec2 { x: 0.0, y: 1e10 });
            self.last_message_count = messages.len();
        }

        self.view.redraw(cx);
    }

    /// Clear all messages
    pub fn clear(&mut self, cx: &mut Cx) {
        self.last_message_count = 0;
        let empty = if self.empty_text.is_empty() {
            "Waiting for conversation..."
        } else {
            &self.empty_text
        };
        self.view
            .markdown(ids!(chat_scroll.content_wrapper.content))
            .set_text(cx, empty);
        self.view.redraw(cx);
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
        self.view.view(ids!(header)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );
        self.view.label(ids!(header.title)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );
        self.view.view(ids!(header.copy_btn)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );
        self.view.redraw(cx);
    }

    /// Set copy button animation state (for feedback)
    pub fn set_copy_flash(&mut self, cx: &mut Cx, value: f64) {
        self.view.view(ids!(header.copy_btn)).apply_over(
            cx,
            live! {
                draw_bg: { copied: (value) }
            },
        );
        self.view.redraw(cx);
    }

    /// Get the displayed text for copying
    pub fn get_text_for_copy(&self, messages: &[ChatMessage]) -> String {
        if messages.is_empty() {
            "No chat messages".to_string()
        } else {
            messages
                .iter()
                .map(|msg| format!("[{}] {}", msg.sender, msg.content))
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    }
}

impl ChatPanelRef {
    /// Set messages
    pub fn set_messages(&self, cx: &mut Cx, messages: &[ChatMessage]) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_messages(cx, messages);
        }
    }

    /// Clear messages
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

    /// Set copy flash animation
    pub fn set_copy_flash(&self, cx: &mut Cx, value: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_copy_flash(cx, value);
        }
    }

    /// Check if copy was clicked
    pub fn copy_clicked(&self, actions: &Actions) -> bool {
        if let ChatPanelAction::CopyClicked = actions.find_widget_action(self.widget_uid()).cast() {
            true
        } else {
            false
        }
    }
}
