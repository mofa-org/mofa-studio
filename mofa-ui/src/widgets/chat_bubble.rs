//! Chat Bubble Widget
//!
//! Role-aware chat message components for MoFA Studio conversations.
//! Implements the design requested in Issue #55.
//!
//! ## Widgets
//!
//! - [`ChatBubble`] — Single message bubble, styled by sender role
//! - [`ChatBubblePanel`] — Scrollable list of `ChatBubble`s with auto-scroll
//!
//! ## Role Styling
//!
//! | Role | Alignment | Color (light) | Color (dark) |
//! |------|-----------|---------------|--------------|
//! | User | right | blue (#3b82f6) | indigo (#6366f1) |
//! | Assistant | left | slate (#e2e8f0) | slate (#1e2a3a) |
//! | System | center | amber (#fef3c7) | amber (#78350f) |
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::widgets::chat_bubble::*;
//!
//!     panel = <ChatBubblePanel> {}
//! }
//!
//! // In Rust
//! let panel = self.view.chat_bubble_panel(id!(panel));
//! panel.set_messages(cx, &messages);
//! panel.apply_dark_mode(cx, 1.0);
//! ```
//!
//! ## Integration with Dora
//!
//! `ChatBubblePanel` is the visual counterpart to `ChatViewerBridge`. The bridge
//! pushes `ChatMessage` values into `SharedDoraState.chat`, and the app's poll
//! loop calls `panel.set_messages(cx, &messages)` when the state is dirty.
//!
//! ```rust,ignore
//! // In screen's poll_dora_state():
//! if let Some(messages) = shared_state.chat.read_if_dirty() {
//!     let ui_messages: Vec<UiChatMessage> = messages.iter().map(|m| m.into()).collect();
//!     self.view.chat_bubble_panel(id!(chat)).set_messages(cx, &ui_messages);
//! }
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Role indices (must match UiMessageRole enum order)
    ROLE_USER      = 0.0
    ROLE_ASSISTANT = 1.0
    ROLE_SYSTEM    = 2.0

    BUBBLE_RADIUS      = 10.0
    BUBBLE_PADDING_H   = 12.0
    BUBBLE_PADDING_V   = 8.0
    SENDER_FONT_SIZE   = 10.5
    CONTENT_FONT_SIZE  = 13.0
    TIMESTAMP_FONT_SIZE = 9.5

    // ── Single Chat Bubble ──────────────────────────────────────────────────
    //
    // A rounded rectangle containing:
    //  • sender name (small, muted)
    //  • message content (main text)
    //  • timestamp (small, bottom-right)
    //
    // Background color is driven by `role` (0=User, 1=Assistant, 2=System)
    // and `dark_mode` instance variables set from Rust.

    pub ChatBubble = {{ChatBubble}} {
        width: Fill, height: Fit
        flow: Down

        // Outer row for alignment (user = right, assistant = left, system = center)
        row = <View> {
            width: Fill, height: Fit
            flow: Right
            align: {x: 0.0, y: 0.0}

            spacer_left = <View> { width: 0, height: 1 }

            bubble_wrap = <View> {
                width: Fit, height: Fit
                flow: Down

                bubble = <View> {
                    width: Fit, height: Fit
                    flow: Down
                    padding: { left: (BUBBLE_PADDING_H), right: (BUBBLE_PADDING_H),
                               top: (BUBBLE_PADDING_V), bottom: (BUBBLE_PADDING_V) }
                    show_bg: true
                    draw_bg: {
                        instance role: 0.0        // 0=user, 1=assistant, 2=system
                        instance dark_mode: 0.0   // 0=light, 1=dark
                        border_radius: (BUBBLE_RADIUS)

                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                            // Light-mode palette
                            let user_light      = vec4(0.231, 0.510, 0.965, 1.0);  // blue-500
                            let assistant_light = vec4(0.886, 0.910, 0.941, 1.0);  // slate-200
                            let system_light    = vec4(0.996, 0.953, 0.780, 1.0);  // amber-100

                            // Dark-mode palette
                            let user_dark      = vec4(0.239, 0.302, 0.706, 1.0);  // indigo-600
                            let assistant_dark = vec4(0.122, 0.169, 0.239, 1.0);  // slate-800
                            let system_dark    = vec4(0.471, 0.212, 0.059, 1.0);  // amber-900

                            // Pick light/dark palette
                            let user_color      = mix(user_light, user_dark, self.dark_mode);
                            let assistant_color = mix(assistant_light, assistant_dark, self.dark_mode);
                            let system_color    = mix(system_light, system_dark, self.dark_mode);

                            // Select color by role
                            let color = mix(
                                mix(user_color, assistant_color, clamp(self.role, 0.0, 1.0)),
                                system_color,
                                clamp(self.role - 1.0, 0.0, 1.0)
                            );

                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            sdf.fill(color);
                            return sdf.result;
                        }
                    }

                    sender_label = <Label> {
                        width: Fit, height: Fit
                        draw_text: {
                            instance role: 0.0
                            instance dark_mode: 0.0
                            text_style: { font_size: (SENDER_FONT_SIZE) }
                            fn get_color(self) -> vec4 {
                                // User: white text; Assistant/System: muted gray
                                let white    = vec4(1.0, 1.0, 1.0, 0.85);
                                let muted_l  = vec4(0.376, 0.447, 0.529, 1.0);  // slate-500
                                let muted_d  = vec4(0.580, 0.639, 0.722, 1.0);  // slate-400
                                let muted    = mix(muted_l, muted_d, self.dark_mode);
                                let non_user = muted;
                                return mix(white, non_user, clamp(self.role, 0.0, 1.0));
                            }
                        }
                        text: "Sender"
                    }

                    content_label = <Label> {
                        width: 320, height: Fit
                        draw_text: {
                            instance role: 0.0
                            instance dark_mode: 0.0
                            text_style: { font_size: (CONTENT_FONT_SIZE) }
                            text_wrap: Word
                            fn get_color(self) -> vec4 {
                                let white    = vec4(1.0, 1.0, 1.0, 1.0);
                                let dark_l   = vec4(0.122, 0.161, 0.220, 1.0);  // slate-900
                                let dark_d   = vec4(0.878, 0.910, 0.941, 1.0);  // slate-200
                                let dark_text = mix(dark_l, dark_d, self.dark_mode);
                                return mix(white, dark_text, clamp(self.role, 0.0, 1.0));
                            }
                        }
                        text: "Message"
                    }

                    timestamp_label = <Label> {
                        width: Fit, height: Fit
                        margin: { top: 4 }
                        draw_text: {
                            instance role: 0.0
                            instance dark_mode: 0.0
                            text_style: { font_size: (TIMESTAMP_FONT_SIZE) }
                            fn get_color(self) -> vec4 {
                                let white_muted = vec4(1.0, 1.0, 1.0, 0.55);
                                let gray_l = vec4(0.580, 0.639, 0.722, 1.0);
                                let gray_d = vec4(0.439, 0.494, 0.561, 1.0);
                                let gray = mix(gray_l, gray_d, self.dark_mode);
                                return mix(white_muted, gray, clamp(self.role, 0.0, 1.0));
                            }
                        }
                        text: ""
                    }
                }
            }

            spacer_right = <View> { width: 0, height: 1 }
        }

        // Bottom margin between bubbles
        gap = <View> { width: Fill, height: 6 }
    }

    // ── Chat Bubble Panel ───────────────────────────────────────────────────
    //
    // Scrollable container for a dynamic list of ChatBubble widgets.
    // Uses PortalList for efficient rendering of large histories.

    pub ChatBubblePanel = {{ChatBubblePanel}} {
        width: Fill, height: Fill
        flow: Down
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            border_radius: 8.0
            border_size: 1.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                let bg    = mix(vec4(0.973, 0.980, 0.988, 1.0), vec4(0.082, 0.110, 0.157, 1.0), self.dark_mode);
                let border= mix(vec4(0.800, 0.820, 0.851, 1.0), vec4(0.180, 0.220, 0.290, 1.0), self.dark_mode);
                sdf.fill(bg);
                sdf.stroke(border, self.border_size);
                return sdf.result;
            }
        }

        header = <View> {
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
                text: "Conversation"
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 13.0 }
                    fn get_color(self) -> vec4 {
                        return mix(vec4(0.122, 0.161, 0.220, 1.0), vec4(0.878, 0.910, 0.941, 1.0), self.dark_mode);
                    }
                }
            }
            <Filler> {}
        }

        scroll_view = <ScrollYView> {
            width: Fill, height: Fill
            flow: Down
            padding: { left: 12, right: 12, top: 8, bottom: 8 }
            scroll_bars: <ScrollBars> {
                show_scroll_x: false
                show_scroll_y: true
            }

            list = <PortalList> {
                width: Fill, height: Fill
                flow: Down
                drag_scrolling: false
                ChatBubble = <ChatBubble> {}
            }
        }
    }
}

/// Role for a chat message, mirroring `mofa_dora_bridge::data::MessageRole`.
///
/// Defined here as a UI-level enum to avoid coupling mofa-ui to mofa-dora-bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UiMessageRole {
    User,
    #[default]
    Assistant,
    System,
}

impl UiMessageRole {
    /// Shader instance value used in `draw_bg.role`
    pub fn shader_value(self) -> f64 {
        match self {
            UiMessageRole::User => 0.0,
            UiMessageRole::Assistant => 1.0,
            UiMessageRole::System => 2.0,
        }
    }

    /// Whether this role's bubble aligns to the right
    pub fn is_right_aligned(self) -> bool {
        self == UiMessageRole::User
    }
}

/// A single chat message for display.
///
/// This is the UI-layer message type, decoupled from `mofa_dora_bridge::ChatMessage`
/// so that `mofa-ui` can be used independently of the bridge crate.
#[derive(Clone, Debug)]
pub struct UiChatMessage {
    /// Sender display name (e.g., "You", "Tutor", "System")
    pub sender: String,
    /// Message text content
    pub content: String,
    /// Unix timestamp in milliseconds (0 = no timestamp shown)
    pub timestamp_ms: u64,
    /// Message role for visual differentiation
    pub role: UiMessageRole,
    /// Whether this message is still streaming (shows ⌛ indicator)
    pub is_streaming: bool,
}

impl UiChatMessage {
    /// Create a user message
    pub fn user(sender: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            sender: sender.into(),
            content: content.into(),
            timestamp_ms: current_timestamp_ms(),
            role: UiMessageRole::User,
            is_streaming: false,
        }
    }

    /// Create an assistant message
    pub fn assistant(sender: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            sender: sender.into(),
            content: content.into(),
            timestamp_ms: current_timestamp_ms(),
            role: UiMessageRole::Assistant,
            is_streaming: false,
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            sender: "System".into(),
            content: content.into(),
            timestamp_ms: current_timestamp_ms(),
            role: UiMessageRole::System,
            is_streaming: false,
        }
    }

    /// Format timestamp for display (HH:MM)
    pub fn format_time(&self) -> String {
        if self.timestamp_ms == 0 {
            return String::new();
        }
        let secs = self.timestamp_ms / 1000;
        let secs_in_day = secs % 86400;
        let h = secs_in_day / 3600;
        let m = (secs_in_day % 3600) / 60;
        format!("{:02}:{:02}", h, m)
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ── ChatBubble widget ────────────────────────────────────────────────────────

#[derive(Live, LiveHook, Widget)]
pub struct ChatBubble {
    #[deref]
    view: View,
    #[rust]
    dark_mode: f64,
    #[rust]
    role: UiMessageRole,
}

impl Widget for ChatBubble {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl ChatBubble {
    /// Populate the bubble with message data and trigger a redraw.
    pub fn set_message(&mut self, cx: &mut Cx, msg: &UiChatMessage) {
        self.role = msg.role;
        let role_val = msg.role.shader_value();
        let dark = self.dark_mode;

        // Sender label
        let sender_text = if msg.is_streaming {
            format!("{} ⌛", msg.sender)
        } else {
            msg.sender.clone()
        };
        self.view
            .label(ids!(row.bubble_wrap.bubble.sender_label))
            .set_text(cx, &sender_text);
        self.view
            .label(ids!(row.bubble_wrap.bubble.sender_label))
            .apply_over(
                cx,
                live! { draw_text: { role: (role_val), dark_mode: (dark) } },
            );

        // Content label
        self.view
            .label(ids!(row.bubble_wrap.bubble.content_label))
            .set_text(cx, &msg.content);
        self.view
            .label(ids!(row.bubble_wrap.bubble.content_label))
            .apply_over(
                cx,
                live! { draw_text: { role: (role_val), dark_mode: (dark) } },
            );

        // Timestamp label
        let ts = msg.format_time();
        self.view
            .label(ids!(row.bubble_wrap.bubble.timestamp_label))
            .set_text(cx, &ts);
        self.view
            .label(ids!(row.bubble_wrap.bubble.timestamp_label))
            .apply_over(
                cx,
                live! { draw_text: { role: (role_val), dark_mode: (dark) } },
            );

        // Bubble background
        self.view.view(ids!(row.bubble_wrap.bubble)).apply_over(
            cx,
            live! { draw_bg: { role: (role_val), dark_mode: (dark) } },
        );

        // Alignment: user → right (push spacer_left), assistant/system → left
        let (left_fill, right_fill): (f64, f64) = if msg.role.is_right_aligned() {
            (1.0, 0.0)
        } else {
            (0.0, 0.0)
        };
        self.view
            .view(ids!(row.spacer_left))
            .apply_over(cx, live! { width: (left_fill * 80.0) });
        self.view
            .view(ids!(row.spacer_right))
            .apply_over(cx, live! { width: (right_fill * 80.0) });

        self.view.redraw(cx);
    }

    /// Apply dark/light mode toggle
    pub fn apply_dark_mode(&mut self, cx: &mut Cx, dark_mode: f64) {
        self.dark_mode = dark_mode;
        self.view.redraw(cx);
    }
}

impl ChatBubbleRef {
    pub fn set_message(&self, cx: &mut Cx, msg: &UiChatMessage) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_message(cx, msg);
        }
    }
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }
}

// ── ChatBubblePanel widget ───────────────────────────────────────────────────

/// Actions emitted by `ChatBubblePanel`
#[derive(Clone, Debug, DefaultNone)]
pub enum ChatBubblePanelAction {
    None,
}

#[derive(Live, LiveHook, Widget)]
pub struct ChatBubblePanel {
    #[deref]
    view: View,
    #[rust]
    messages: Vec<UiChatMessage>,
    #[rust]
    dark_mode: f64,
}

impl Widget for ChatBubblePanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let dark = self.dark_mode;
        // Clone required: self.view.draw_walk() takes &mut self, so we can't
        // hold a borrow on self.messages across the loop at the same time.
        let messages = self.messages.clone();

        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = item.as_portal_list().borrow_mut() {
                list.set_item_range(cx, 0, messages.len());

                while let Some(item_id) = list.next_visible_item(cx) {
                    if item_id < messages.len() {
                        let widget = list.item(cx, item_id, live_id!(ChatBubble));
                        widget.as_chat_bubble().set_message(cx, &messages[item_id]);
                        widget.as_chat_bubble().apply_dark_mode(cx, dark);
                        widget.draw_all(cx, scope);
                    }
                }
            }
        }

        DrawStep::done()
    }
}

impl ChatBubblePanel {
    /// Replace the displayed messages and trigger a redraw.
    ///
    /// Call this from the screen's poll loop whenever `SharedDoraState.chat`
    /// returns dirty data.
    pub fn set_messages(&mut self, cx: &mut Cx, messages: &[UiChatMessage]) {
        let prev_count = self.messages.len();
        self.messages = messages.to_vec();

        // Auto-scroll to bottom when new messages arrive
        let count = self.messages.len();
        if count > prev_count && count > 0 {
            self.view
                .portal_list(ids!(scroll_view.list))
                .set_first_id_and_scroll(count.saturating_sub(1), 0.0);
        }

        self.view.redraw(cx);
    }

    /// Clear all messages.
    pub fn clear(&mut self, cx: &mut Cx) {
        self.messages.clear();
        self.view.redraw(cx);
    }

    /// Apply dark/light mode.
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
        self.view.redraw(cx);
    }

    /// Number of messages currently displayed.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

impl ChatBubblePanelRef {
    pub fn set_messages(&self, cx: &mut Cx, messages: &[UiChatMessage]) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_messages(cx, messages);
        }
    }

    pub fn clear(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.clear(cx);
        }
    }

    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }

    pub fn message_count(&self) -> usize {
        self.borrow()
            .map(|inner| inner.message_count())
            .unwrap_or(0)
    }
}
