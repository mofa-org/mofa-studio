//! Shell Header Widget
//!
//! A customizable application header with hamburger menu, logo, title,
//! theme toggle, and action slots.
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::shell::header::*;
//!
//!     header = <ShellHeader> {
//!         title: "My App"
//!     }
//! }
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Color constants (vec4 to avoid hex parsing issues)
    PANEL_BG = vec4(0.976, 0.980, 0.984, 1.0)
    PANEL_BG_DARK = vec4(0.118, 0.161, 0.231, 1.0)
    TEXT_PRIMARY = vec4(0.067, 0.090, 0.125, 1.0)
    TEXT_PRIMARY_DARK = vec4(0.945, 0.961, 0.976, 1.0)
    SLATE_400 = vec4(0.580, 0.639, 0.702, 1.0)
    SLATE_500 = vec4(0.392, 0.455, 0.545, 1.0)
    GRAY_600 = vec4(0.294, 0.333, 0.388, 1.0)
    HOVER_BG = vec4(0.0, 0.0, 0.0, 0.05)
    TRANSPARENT = vec4(0.0, 0.0, 0.0, 0.0)
    AMBER_500 = vec4(0.961, 0.624, 0.043, 1.0)
    INDIGO_500 = vec4(0.388, 0.400, 0.945, 1.0)
    WHITE = vec4(1.0, 1.0, 1.0, 1.0)

    /// Hamburger menu icon (three horizontal lines)
    HamburgerIcon = <View> {
        width: 21, height: 21
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let cy = self.rect_size.y * 0.5;
                let cx = self.rect_size.x * 0.5;
                let color = mix((SLATE_500), (SLATE_400), self.dark_mode);
                sdf.move_to(cx - 5.0, cy - 4.0);
                sdf.line_to(cx + 5.0, cy - 4.0);
                sdf.stroke(color, 1.5);
                sdf.move_to(cx - 5.0, cy);
                sdf.line_to(cx + 5.0, cy);
                sdf.stroke(color, 1.5);
                sdf.move_to(cx - 5.0, cy + 4.0);
                sdf.line_to(cx + 5.0, cy + 4.0);
                sdf.stroke(color, 1.5);
                return sdf.result;
            }
        }
    }

    /// Theme toggle button with sun/moon icons
    ThemeToggle = <View> {
        width: 36, height: 36
        align: {x: 0.5, y: 0.5}
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance hover: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let cx = self.rect_size.x * 0.5;
                let cy = self.rect_size.y * 0.5;
                sdf.circle(cx, cy, 16.0);
                sdf.fill(mix((TRANSPARENT), (HOVER_BG), self.hover));
                return sdf.result;
            }
        }

        sun_icon = <View> {
            width: 20, height: 20
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let c = self.rect_size * 0.5;
                    let amber = (AMBER_500);
                    // Sun circle
                    sdf.circle(c.x, c.y, 4.0);
                    sdf.fill(amber);
                    // Sun rays
                    let ray_len = 2.5;
                    let ray_dist = 6.5;
                    sdf.move_to(c.x, c.y - ray_dist);
                    sdf.line_to(c.x, c.y - ray_dist - ray_len);
                    sdf.stroke(amber, 1.5);
                    sdf.move_to(c.x, c.y + ray_dist);
                    sdf.line_to(c.x, c.y + ray_dist + ray_len);
                    sdf.stroke(amber, 1.5);
                    sdf.move_to(c.x - ray_dist, c.y);
                    sdf.line_to(c.x - ray_dist - ray_len, c.y);
                    sdf.stroke(amber, 1.5);
                    sdf.move_to(c.x + ray_dist, c.y);
                    sdf.line_to(c.x + ray_dist + ray_len, c.y);
                    sdf.stroke(amber, 1.5);
                    return sdf.result;
                }
            }
        }

        moon_icon = <View> {
            width: 20, height: 20
            visible: false
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let c = self.rect_size * 0.5;
                    let indigo = (INDIGO_500);
                    sdf.circle(c.x, c.y, 6.0);
                    sdf.fill(indigo);
                    sdf.circle(c.x + 3.5, c.y - 2.5, 4.5);
                    sdf.fill((WHITE));
                    return sdf.result;
                }
            }
        }
    }

    /// User profile button with avatar and dropdown arrow
    UserProfileButton = <View> {
        width: Fit, height: Fill
        flow: Right
        align: {x: 0.5, y: 0.5}
        spacing: 4
        cursor: Hand

        avatar = <View> {
            width: 32, height: 32
            padding: {left: 6, top: 8, right: 10, bottom: 8}
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let cx = self.rect_size.x * 0.5;
                    let cy = self.rect_size.y * 0.5;
                    sdf.circle(cx, cy, 15.0);
                    sdf.fill((HOVER_BG));
                    return sdf.result;
                }
            }

            <Icon> {
                draw_icon: {
                    svg_file: dep("crate://makepad-widgets/resources/icons/Icon_User.svg")
                    fn get_color(self) -> vec4 { return (GRAY_600); }
                }
                icon_walk: {width: 16, height: 16}
            }
        }

        dropdown_arrow = <View> {
            width: 12, height: Fill
            draw_bg: {
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let cx = self.rect_size.x * 0.5;
                    let cy = self.rect_size.y * 0.5;
                    sdf.move_to(cx - 4.0, cy - 2.0);
                    sdf.line_to(cx, cy + 2.0);
                    sdf.line_to(cx + 4.0, cy - 2.0);
                    sdf.stroke((SLATE_400), 1.5);
                    return sdf.result;
                }
            }
        }
    }

    /// Shell Header Widget
    pub ShellHeader = {{ShellHeader}} {
        width: Fill, height: Fit
        flow: Right
        spacing: 12
        align: {y: 0.5}
        padding: {left: 20, right: 20, top: 15, bottom: 15}
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
            }
        }

        hamburger = <HamburgerIcon> {}

        logo_slot = <View> {
            width: 40, height: 40
        }

        title_label = <Label> {
            text: "MoFA Studio"
            draw_text: {
                instance dark_mode: 0.0
                text_style: { font_size: 24.0 }
                fn get_color(self) -> vec4 {
                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                }
            }
        }

        // Spacer
        <View> { width: Fill, height: 1 }

        // Action slots (right side)
        actions_slot = <View> {
            width: Fit, height: Fill
            flow: Right
            spacing: 8
            align: {y: 0.5}

            theme_toggle = <ThemeToggle> {}
            user_profile = <UserProfileButton> {}
        }
    }
}

/// Actions emitted by ShellHeader
#[derive(Clone, Debug, DefaultNone)]
pub enum ShellHeaderAction {
    None,
    /// Hamburger menu clicked
    HamburgerClicked,
    /// Theme toggle clicked
    ThemeToggled,
    /// User profile clicked
    UserProfileClicked,
}

#[derive(Live, LiveHook, Widget)]
pub struct ShellHeader {
    #[deref]
    view: View,

    /// Current dark mode value
    #[rust]
    dark_mode: f64,

    /// Header title
    #[live]
    title: String,
}

impl Widget for ShellHeader {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Handle hamburger click
        let hamburger = self.view.view(ids!(hamburger));
        match event.hits(cx, hamburger.area()) {
            Hit::FingerUp(_) => {
                cx.widget_action(
                    self.widget_uid(),
                    &scope.path,
                    ShellHeaderAction::HamburgerClicked,
                );
            }
            _ => {}
        }

        // Handle theme toggle
        let theme_toggle = self.view.view(ids!(actions_slot.theme_toggle));
        match event.hits(cx, theme_toggle.area()) {
            Hit::FingerHoverIn(_) => {
                self.view.view(ids!(actions_slot.theme_toggle)).apply_over(
                    cx,
                    live! {
                        draw_bg: { hover: 1.0 }
                    },
                );
                self.view.redraw(cx);
            }
            Hit::FingerHoverOut(_) => {
                self.view.view(ids!(actions_slot.theme_toggle)).apply_over(
                    cx,
                    live! {
                        draw_bg: { hover: 0.0 }
                    },
                );
                self.view.redraw(cx);
            }
            Hit::FingerUp(_) => {
                cx.widget_action(
                    self.widget_uid(),
                    &scope.path,
                    ShellHeaderAction::ThemeToggled,
                );
            }
            _ => {}
        }

        // Handle user profile click
        let user_profile = self.view.view(ids!(actions_slot.user_profile));
        match event.hits(cx, user_profile.area()) {
            Hit::FingerUp(_) => {
                cx.widget_action(
                    self.widget_uid(),
                    &scope.path,
                    ShellHeaderAction::UserProfileClicked,
                );
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl ShellHeader {
    /// Set header title
    pub fn set_title(&mut self, cx: &mut Cx, title: &str) {
        self.title = title.to_string();
        self.view.label(ids!(title_label)).set_text(cx, title);
    }

    /// Set dark mode (for theme toggle icon)
    pub fn set_dark_mode(&mut self, cx: &mut Cx, is_dark: bool) {
        self.view
            .view(ids!(actions_slot.theme_toggle.sun_icon))
            .set_visible(cx, !is_dark);
        self.view
            .view(ids!(actions_slot.theme_toggle.moon_icon))
            .set_visible(cx, is_dark);
        self.view.redraw(cx);
    }

    /// Apply dark mode animation value
    pub fn apply_dark_mode(&mut self, cx: &mut Cx, dark_mode: f64) {
        self.dark_mode = dark_mode;

        self.view.apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );

        self.view.view(ids!(hamburger)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );

        self.view.label(ids!(title_label)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );

        self.view.redraw(cx);
    }
}

impl ShellHeaderRef {
    /// Set header title
    pub fn set_title(&self, cx: &mut Cx, title: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_title(cx, title);
        }
    }

    /// Set dark mode (for theme toggle icon)
    pub fn set_dark_mode(&self, cx: &mut Cx, is_dark: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_dark_mode(cx, is_dark);
        }
    }

    /// Apply dark mode animation value
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }

    /// Check if hamburger was clicked
    pub fn hamburger_clicked(&self, actions: &Actions) -> bool {
        matches!(
            actions.find_widget_action(self.widget_uid()).cast(),
            ShellHeaderAction::HamburgerClicked
        )
    }

    /// Check if theme toggle was clicked
    pub fn theme_toggled(&self, actions: &Actions) -> bool {
        matches!(
            actions.find_widget_action(self.widget_uid()).cast(),
            ShellHeaderAction::ThemeToggled
        )
    }

    /// Check if user profile was clicked
    pub fn user_profile_clicked(&self, actions: &Actions) -> bool {
        matches!(
            actions.find_widget_action(self.widget_uid()).cast(),
            ShellHeaderAction::UserProfileClicked
        )
    }
}
