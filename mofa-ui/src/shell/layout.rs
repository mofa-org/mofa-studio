//! MoFA Shell Layout Widget
//!
//! The main application shell providing a standard layout with:
//! - Header with navigation and actions
//! - Optional sidebar (left/right)
//! - Main content area
//! - Optional status bar
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::shell::layout::*;
//!
//!     MyApp = <MofaShell> {
//!         header: { title: "My App" }
//!         content: <MyContent> {}
//!     }
//! }
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Color constants
    DARK_BG = vec4(0.933, 0.941, 0.953, 1.0)
    DARK_BG_DARK = vec4(0.067, 0.090, 0.125, 1.0)
    PANEL_BG = vec4(0.976, 0.980, 0.984, 1.0)
    PANEL_BG_DARK = vec4(0.118, 0.161, 0.231, 1.0)
    SLATE_50 = vec4(0.976, 0.980, 0.984, 1.0)
    SLATE_800 = vec4(0.118, 0.161, 0.231, 1.0)
    DIVIDER = vec4(0.878, 0.906, 0.925, 1.0)
    DIVIDER_DARK = vec4(0.278, 0.337, 0.412, 1.0)

    /// MoFA Shell - Main application layout
    pub MofaShell = {{MofaShell}} {
        width: Fill, height: Fill
        flow: Down
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((DARK_BG), (DARK_BG_DARK), self.dark_mode);
            }
        }

        // Header slot
        header_slot = <View> {
            width: Fill, height: Fit
            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                }
            }
        }

        // Main area (sidebar + content)
        main_area = <View> {
            width: Fill, height: Fill
            flow: Right

            // Left sidebar slot
            left_sidebar_slot = <View> {
                width: 0, height: Fill
                visible: false
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                        let bg = mix((SLATE_50), (SLATE_800), self.dark_mode);
                        sdf.fill(bg);
                        // Right border
                        sdf.rect(self.rect_size.x - 1.0, 0., 1.0, self.rect_size.y);
                        let border = mix((DIVIDER), (DIVIDER_DARK), self.dark_mode);
                        sdf.fill(border);
                        return sdf.result;
                    }
                }
            }

            // Content area
            content_slot = <View> {
                width: Fill, height: Fill
                flow: Down
                padding: 20
            }

            // Right sidebar slot (optional)
            right_sidebar_slot = <View> {
                width: 0, height: Fill
                visible: false
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                        let bg = mix((SLATE_50), (SLATE_800), self.dark_mode);
                        sdf.fill(bg);
                        // Left border
                        sdf.rect(0., 0., 1.0, self.rect_size.y);
                        let border = mix((DIVIDER), (DIVIDER_DARK), self.dark_mode);
                        sdf.fill(border);
                        return sdf.result;
                    }
                }
            }
        }

        // Status bar slot (optional, at bottom)
        status_bar_slot = <View> {
            width: Fill, height: 0
            visible: false
        }
    }
}

/// Actions emitted by MofaShell
#[derive(Clone, Debug, DefaultNone)]
pub enum MofaShellAction {
    None,
}

#[derive(Live, LiveHook, Widget)]
pub struct MofaShell {
    #[deref]
    view: View,

    /// Current dark mode value
    #[rust]
    dark_mode: f64,

    /// Left sidebar width (0 = hidden)
    #[live]
    left_sidebar_width: f64,

    /// Right sidebar width (0 = hidden)
    #[live]
    right_sidebar_width: f64,

    /// Whether status bar is visible
    #[live]
    show_status_bar: bool,

    /// Status bar height
    #[live]
    status_bar_height: f64,
}

impl Widget for MofaShell {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl MofaShell {
    /// Show/hide left sidebar with animation
    pub fn set_left_sidebar_width(&mut self, cx: &mut Cx, width: f64) {
        self.left_sidebar_width = width;
        let visible = width > 0.0;

        self.view
            .view(ids!(main_area.left_sidebar_slot))
            .set_visible(cx, visible);
        self.view
            .view(ids!(main_area.left_sidebar_slot))
            .apply_over(
                cx,
                live! {
                    width: (width)
                },
            );
        self.view.redraw(cx);
    }

    /// Show/hide right sidebar
    pub fn set_right_sidebar_width(&mut self, cx: &mut Cx, width: f64) {
        self.right_sidebar_width = width;
        let visible = width > 0.0;

        self.view
            .view(ids!(main_area.right_sidebar_slot))
            .set_visible(cx, visible);
        self.view
            .view(ids!(main_area.right_sidebar_slot))
            .apply_over(
                cx,
                live! {
                    width: (width)
                },
            );
        self.view.redraw(cx);
    }

    /// Show/hide status bar
    pub fn set_status_bar_visible(&mut self, cx: &mut Cx, visible: bool) {
        self.show_status_bar = visible;
        let height = if visible {
            self.status_bar_height.max(28.0)
        } else {
            0.0
        };

        self.view
            .view(ids!(status_bar_slot))
            .set_visible(cx, visible);
        self.view.view(ids!(status_bar_slot)).apply_over(
            cx,
            live! {
                height: (height)
            },
        );
        self.view.redraw(cx);
    }

    /// Set content padding
    pub fn set_content_padding(&mut self, cx: &mut Cx, padding: f64) {
        self.view.view(ids!(main_area.content_slot)).apply_over(
            cx,
            live! {
                padding: (padding)
            },
        );
        self.view.redraw(cx);
    }

    /// Apply dark mode to all shell elements
    pub fn apply_dark_mode(&mut self, cx: &mut Cx, dark_mode: f64) {
        self.dark_mode = dark_mode;

        // Main background
        self.view.apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );

        // Header slot
        self.view.view(ids!(header_slot)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );

        // Sidebars
        self.view
            .view(ids!(main_area.left_sidebar_slot))
            .apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );
        self.view
            .view(ids!(main_area.right_sidebar_slot))
            .apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );

        self.view.redraw(cx);
    }

    /// Get reference to header slot
    pub fn header_slot(&self) -> ViewRef {
        self.view.view(ids!(header_slot))
    }

    /// Get reference to content slot
    pub fn content_slot(&self) -> ViewRef {
        self.view.view(ids!(main_area.content_slot))
    }

    /// Get reference to left sidebar slot
    pub fn left_sidebar_slot(&self) -> ViewRef {
        self.view.view(ids!(main_area.left_sidebar_slot))
    }

    /// Get reference to right sidebar slot
    pub fn right_sidebar_slot(&self) -> ViewRef {
        self.view.view(ids!(main_area.right_sidebar_slot))
    }

    /// Get reference to status bar slot
    pub fn status_bar_slot(&self) -> ViewRef {
        self.view.view(ids!(status_bar_slot))
    }
}

impl MofaShellRef {
    /// Show/hide left sidebar with width
    pub fn set_left_sidebar_width(&self, cx: &mut Cx, width: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_left_sidebar_width(cx, width);
        }
    }

    /// Show/hide right sidebar with width
    pub fn set_right_sidebar_width(&self, cx: &mut Cx, width: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_right_sidebar_width(cx, width);
        }
    }

    /// Show/hide status bar
    pub fn set_status_bar_visible(&self, cx: &mut Cx, visible: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_status_bar_visible(cx, visible);
        }
    }

    /// Set content padding
    pub fn set_content_padding(&self, cx: &mut Cx, padding: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_content_padding(cx, padding);
        }
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }
}
