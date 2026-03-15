//! Status Bar Widget
//!
//! A status bar for displaying connection status, notifications,
//! and other status information.
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::shell::status_bar::*;
//!
//!     status = <StatusBar> {}
//! }
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Color constants
    PANEL_BG = vec4(0.976, 0.980, 0.984, 1.0)
    PANEL_BG_DARK = vec4(0.118, 0.161, 0.231, 1.0)
    TEXT_SECONDARY = vec4(0.392, 0.455, 0.545, 1.0)
    TEXT_SECONDARY_DARK = vec4(0.580, 0.639, 0.722, 1.0)
    GREEN_500 = vec4(0.133, 0.773, 0.373, 1.0)
    AMBER_500 = vec4(0.961, 0.624, 0.043, 1.0)
    RED_500 = vec4(0.937, 0.267, 0.267, 1.0)
    SLATE_400 = vec4(0.580, 0.639, 0.702, 1.0)
    BORDER = vec4(0.878, 0.906, 0.925, 1.0)
    BORDER_DARK = vec4(0.278, 0.337, 0.412, 1.0)

    /// Connection status indicator dot
    StatusDot = <View> {
        width: 8, height: 8
        show_bg: true
        draw_bg: {
            instance status: 0.0  // 0=disconnected, 1=connecting, 2=connected
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let c = self.rect_size * 0.5;
                sdf.circle(c.x, c.y, 4.0);
                // Red for disconnected, amber for connecting, green for connected
                let disconnected = (RED_500);
                let connecting = (AMBER_500);
                let connected = (GREEN_500);
                let color = mix(
                    mix(disconnected, connecting, min(self.status, 1.0)),
                    connected,
                    max(self.status - 1.0, 0.0)
                );
                sdf.fill(color);
                return sdf.result;
            }
        }
    }

    /// Status Bar Widget
    pub StatusBar = {{StatusBar}} {
        width: Fill, height: 28
        flow: Right
        align: {y: 0.5}
        padding: {left: 16, right: 16}
        spacing: 12
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                let bg = mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                sdf.fill(bg);
                // Top border
                sdf.rect(0., 0., self.rect_size.x, 1.0);
                let border = mix((BORDER), (BORDER_DARK), self.dark_mode);
                sdf.fill(border);
                return sdf.result;
            }
        }

        // Left section - connection status
        left_section = <View> {
            width: Fit, height: Fill
            flow: Right
            align: {y: 0.5}
            spacing: 8

            status_dot = <StatusDot> {}

            status_text = <Label> {
                text: "Disconnected"
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 11.0 }
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                    }
                }
            }
        }

        // Center spacer
        <View> { width: Fill, height: 1 }

        // Right section - notifications and info
        right_section = <View> {
            width: Fit, height: Fill
            flow: Right
            align: {y: 0.5}
            spacing: 16

            info_text = <Label> {
                text: ""
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 11.0 }
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                    }
                }
            }
        }
    }
}

/// Connection status
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}

impl ConnectionStatus {
    fn as_f64(&self) -> f64 {
        match self {
            ConnectionStatus::Disconnected => 0.0,
            ConnectionStatus::Connecting => 1.0,
            ConnectionStatus::Connected => 2.0,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            ConnectionStatus::Disconnected => "Disconnected",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Connected => "Connected",
        }
    }
}

/// Actions emitted by StatusBar
#[derive(Clone, Debug, DefaultNone)]
pub enum StatusBarAction {
    None,
    /// Status indicator clicked
    StatusClicked,
}

#[derive(Live, LiveHook, Widget)]
pub struct StatusBar {
    #[deref]
    view: View,

    /// Current dark mode value
    #[rust]
    dark_mode: f64,

    /// Current connection status
    #[rust]
    status: ConnectionStatus,
}

impl Widget for StatusBar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Handle status click
        let status_section = self.view.view(ids!(left_section));
        match event.hits(cx, status_section.area()) {
            Hit::FingerUp(_) => {
                cx.widget_action(
                    self.widget_uid(),
                    &scope.path,
                    StatusBarAction::StatusClicked,
                );
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl StatusBar {
    /// Set connection status
    pub fn set_status(&mut self, cx: &mut Cx, status: ConnectionStatus) {
        self.status = status;

        self.view.view(ids!(left_section.status_dot)).apply_over(
            cx,
            live! {
                draw_bg: { status: (status.as_f64()) }
            },
        );

        self.view
            .label(ids!(left_section.status_text))
            .set_text(cx, status.label());
        self.view.redraw(cx);
    }

    /// Set info text (right section)
    pub fn set_info(&mut self, cx: &mut Cx, text: &str) {
        self.view
            .label(ids!(right_section.info_text))
            .set_text(cx, text);
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

        self.view.label(ids!(left_section.status_text)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );

        self.view.label(ids!(right_section.info_text)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );

        self.view.redraw(cx);
    }
}

impl StatusBarRef {
    /// Set connection status
    pub fn set_status(&self, cx: &mut Cx, status: ConnectionStatus) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_status(cx, status);
        }
    }

    /// Set info text
    pub fn set_info(&self, cx: &mut Cx, text: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_info(cx, text);
        }
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }

    /// Check if status was clicked
    pub fn status_clicked(&self, actions: &Actions) -> bool {
        matches!(
            actions.find_widget_action(self.widget_uid()).cast(),
            StatusBarAction::StatusClicked
        )
    }
}
