//! Shell Sidebar Widget
//!
//! A collapsible sidebar for navigation with app list and settings.
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::shell::sidebar::*;
//!
//!     sidebar = <ShellSidebar> {}
//! }
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Color constants
    SLATE_50 = vec4(0.976, 0.980, 0.984, 1.0)
    SLATE_200 = vec4(0.878, 0.906, 0.925, 1.0)
    SLATE_400 = vec4(0.580, 0.639, 0.702, 1.0)
    SLATE_500 = vec4(0.392, 0.455, 0.545, 1.0)
    SLATE_700 = vec4(0.204, 0.224, 0.275, 1.0)
    SLATE_800 = vec4(0.118, 0.161, 0.231, 1.0)
    BLUE_100 = vec4(0.859, 0.906, 0.996, 1.0)
    BLUE_900 = vec4(0.118, 0.161, 0.353, 1.0)
    DIVIDER = vec4(0.878, 0.906, 0.925, 1.0)
    DIVIDER_DARK = vec4(0.278, 0.337, 0.412, 1.0)

    /// Sidebar menu button with selection and hover states
    pub SidebarButton = <Button> {
        width: Fill, height: Fit
        padding: {top: 12, bottom: 12, left: 12, right: 12}
        margin: 0
        align: {x: 0.0, y: 0.5}
        icon_walk: {width: 20, height: 20, margin: {right: 12}}

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

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance selected: 0.0
            instance dark_mode: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let light_normal = (SLATE_50);
                let light_hover = (SLATE_200);
                let light_selected = (BLUE_100);
                let dark_normal = (SLATE_800);
                let dark_hover = (SLATE_700);
                let dark_selected = (BLUE_900);
                let normal = mix(light_normal, dark_normal, self.dark_mode);
                let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                let selected_color = mix(light_selected, dark_selected, self.dark_mode);
                let color = mix(
                    mix(normal, hover_color, self.hover),
                    selected_color,
                    self.selected
                );
                sdf.box(2.0, 2.0, self.rect_size.x - 4.0, self.rect_size.y - 4.0, 6.0);
                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            instance dark_mode: 0.0
            text_style: { font_size: 12.0 }

            fn get_color(self) -> vec4 {
                return mix((SLATE_500), (SLATE_400), self.dark_mode);
            }
        }

        draw_icon: {
            fn get_color(self) -> vec4 {
                return (SLATE_500);
            }
        }
    }

    /// Sidebar divider line
    SidebarDivider = <View> {
        width: Fill, height: 1
        margin: {top: 8, bottom: 8}
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((DIVIDER), (DIVIDER_DARK), self.dark_mode);
            }
        }
    }

    /// Shell Sidebar Widget
    pub ShellSidebar = {{ShellSidebar}} {
        width: Fill, height: Fill
        flow: Down
        spacing: 4.0
        padding: {top: 15, bottom: 15, left: 10, right: 10}
        margin: 0

        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 4.0);
                let bg = mix((SLATE_50), (SLATE_800), self.dark_mode);
                sdf.fill(bg);
                return sdf.result;
            }
        }

        // Main navigation slot
        nav_slot = <View> {
            width: Fill, height: Fit
            flow: Down
            spacing: 4.0
        }

        // Spacer to push settings to bottom
        <View> { width: Fill, height: Fill }

        // Bottom divider
        bottom_divider = <SidebarDivider> {}

        // Settings slot (bottom of sidebar)
        settings_slot = <View> {
            width: Fill, height: Fit
            flow: Down
            spacing: 4.0
        }
    }
}

/// Sidebar item definition
#[derive(Clone, Debug)]
pub struct SidebarItem {
    pub id: String,
    pub label: String,
    pub icon_path: Option<String>,
}

/// Actions emitted by ShellSidebar
#[derive(Clone, Debug, DefaultNone)]
pub enum ShellSidebarAction {
    None,
    /// A navigation item was selected
    ItemSelected(String),
}

#[derive(Live, LiveHook, Widget)]
pub struct ShellSidebar {
    #[deref]
    view: View,

    /// Current dark mode value
    #[rust]
    dark_mode: f64,

    /// Currently selected item ID
    #[rust]
    selected_id: Option<String>,

    /// Registered navigation items (reserved for future sidebar population)
    #[rust]
    _nav_items: Vec<SidebarItem>,

    /// Registered settings items (reserved for future sidebar population)
    #[rust]
    _settings_items: Vec<SidebarItem>,
}

impl Widget for ShellSidebar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl ShellSidebar {
    /// Set selected item by ID
    pub fn set_selected(&mut self, _cx: &mut Cx, id: Option<&str>) {
        self.selected_id = id.map(|s| s.to_string());
        // Note: Button selection state would need to be applied
        // via apply_over for each button in the slot
    }

    /// Get currently selected item ID
    pub fn get_selected(&self) -> Option<&str> {
        self.selected_id.as_deref()
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&mut self, cx: &mut Cx, dark_mode: f64) {
        self.dark_mode = dark_mode;

        self.view.apply_over(cx, live!{
            draw_bg: { dark_mode: (dark_mode) }
        });

        self.view.view(ids!(bottom_divider)).apply_over(cx, live!{
            draw_bg: { dark_mode: (dark_mode) }
        });

        self.view.redraw(cx);
    }
}

impl ShellSidebarRef {
    /// Set selected item by ID
    pub fn set_selected(&self, cx: &mut Cx, id: Option<&str>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_selected(cx, id);
        }
    }

    /// Get currently selected item ID
    pub fn get_selected(&self) -> Option<String> {
        self.borrow().and_then(|inner| inner.get_selected().map(|s| s.to_string()))
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }

    /// Check if an item was selected
    pub fn item_selected(&self, actions: &Actions) -> Option<String> {
        if let ShellSidebarAction::ItemSelected(id) = actions.find_widget_action(self.widget_uid()).cast() {
            Some(id)
        } else {
            None
        }
    }
}
