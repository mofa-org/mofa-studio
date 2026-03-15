//! Dataflow Picker Widget
//!
//! A file picker widget for selecting YAML dataflow files.
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::widgets::dataflow_picker::*;
//!
//!     dataflow = <DataflowPicker> {
//!         label: "Dataflow"
//!     }
//! }
//! ```
//!
//! ## Handling Selection
//!
//! ```rust,ignore
//! let picker = self.view.dataflow_picker(id!(dataflow));
//! if let Some(path) = picker.changed(&actions) {
//!     // Handle dataflow file selection
//! }
//! ```

use makepad_widgets::*;
use std::path::PathBuf;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Light/Dark mode color constants
    PANEL_BG = vec4(0.976, 0.980, 0.984, 1.0)
    PANEL_BG_DARK = vec4(0.204, 0.224, 0.275, 1.0)
    BORDER = vec4(0.878, 0.906, 0.925, 1.0)
    TEXT_PRIMARY = vec4(0.067, 0.090, 0.125, 1.0)
    TEXT_PRIMARY_DARK = vec4(0.945, 0.961, 0.976, 1.0)
    TEXT_SECONDARY = vec4(0.392, 0.455, 0.545, 1.0)
    TEXT_SECONDARY_DARK = vec4(0.580, 0.639, 0.722, 1.0)
    SLATE_50 = vec4(0.976, 0.980, 0.984, 1.0)
    SLATE_100 = vec4(0.945, 0.961, 0.976, 1.0)
    SLATE_300 = vec4(0.796, 0.835, 0.878, 1.0)
    SLATE_500 = vec4(0.392, 0.455, 0.545, 1.0)
    SLATE_600 = vec4(0.278, 0.337, 0.412, 1.0)
    SLATE_700 = vec4(0.204, 0.224, 0.275, 1.0)
    WHITE = vec4(1.0, 1.0, 1.0, 1.0)
    ACCENT_BLUE = vec4(0.231, 0.510, 0.965, 1.0)

    /// Browse button style
    BrowseButton = <Button> {
        width: Fit, height: Fit
        padding: {left: 12, right: 12, top: 6, bottom: 6}
        text: "Browse"
        draw_text: {
            instance dark_mode: 0.0
            text_style: { font_size: 11.0 }
            fn get_color(self) -> vec4 {
                return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
            }
        }
        draw_bg: {
            instance dark_mode: 0.0
            instance hover: 0.0
            instance pressed: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                let light_base = (SLATE_100);
                let light_hover = (SLATE_300);
                let dark_base = (SLATE_600);
                let dark_hover = (SLATE_500);
                let base = mix(light_base, dark_base, self.dark_mode);
                let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                let pressed_factor = 0.9;
                let color = mix(base, hover_color, self.hover);
                let final_color = mix(color, vec4(color.x * pressed_factor, color.y * pressed_factor, color.z * pressed_factor, color.w), self.pressed);
                sdf.fill(final_color);
                return sdf.result;
            }
        }
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
    }

    /// Dataflow Picker Widget
    pub DataflowPicker = {{DataflowPicker}} {
        width: Fill, height: Fit
        flow: Down
        spacing: 8

        // Label row
        label_row = <View> {
            width: Fill, height: Fit
            flow: Right
            align: {y: 0.5}
            spacing: 8

            picker_label = <Label> {
                text: "Dataflow"
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 12.0 }
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                    }
                }
            }
        }

        // Path display and browse button
        path_row = <View> {
            width: Fill, height: Fit
            flow: Right
            spacing: 8
            align: {y: 0.5}

            // Path display container
            path_container = <RoundedView> {
                width: Fill, height: 36
                padding: {left: 12, right: 12, top: 8, bottom: 8}
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    border_radius: 4.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                        let bg = mix((SLATE_100), (SLATE_600), self.dark_mode);
                        let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                        sdf.fill(bg);
                        sdf.stroke(border, 1.0);
                        return sdf.result;
                    }
                }
                align: {y: 0.5}

                path_label = <Label> {
                    width: Fill
                    text: "No file selected"
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: { font_size: 11.0 }
                        fn get_color(self) -> vec4 {
                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                        }
                    }
                }
            }

            browse_btn = <BrowseButton> {}
        }

        // Optional file info row
        info_row = <View> {
            width: Fill, height: Fit
            visible: false

            info_label = <Label> {
                text: ""
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: { font_size: 10.0 }
                    fn get_color(self) -> vec4 {
                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                    }
                }
            }
        }
    }
}

/// Actions emitted by DataflowPicker
#[derive(Clone, Debug, DefaultNone)]
pub enum DataflowPickerAction {
    None,
    /// User selected a new file
    Selected(PathBuf),
    /// Browse button clicked (for external file dialog handling)
    BrowseClicked,
}

#[derive(Live, LiveHook, Widget)]
pub struct DataflowPicker {
    #[deref]
    view: View,

    /// Current dark mode value
    #[rust]
    dark_mode: f64,

    /// Currently selected path
    #[rust]
    selected_path: Option<PathBuf>,

    /// Label text
    #[live]
    label: String,

    /// File extension filter (default: "yml")
    #[live]
    extension: String,
}

impl Widget for DataflowPicker {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let actions = cx.capture_actions(|cx| self.view.handle_event(cx, event, scope));

        // Handle browse button click
        if self
            .view
            .button(ids!(path_row.browse_btn))
            .clicked(&actions)
        {
            cx.widget_action(
                self.widget_uid(),
                &scope.path,
                DataflowPickerAction::BrowseClicked,
            );
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl DataflowPicker {
    /// Get currently selected path
    pub fn get_path(&self) -> Option<PathBuf> {
        self.selected_path.clone()
    }

    /// Set selected path
    pub fn set_path(&mut self, cx: &mut Cx, path: Option<PathBuf>) {
        self.selected_path = path.clone();

        let display_text = match &path {
            Some(p) => {
                // Show just the filename, or relative path if short
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.to_string_lossy().to_string())
            }
            None => "No file selected".to_string(),
        };

        self.view
            .label(ids!(path_row.path_container.path_label))
            .set_text(cx, &display_text);
        self.view.redraw(cx);
    }

    /// Set full path display (shows the complete path)
    pub fn set_path_display(&mut self, cx: &mut Cx, path: &str) {
        self.view
            .label(ids!(path_row.path_container.path_label))
            .set_text(cx, path);
        self.view.redraw(cx);
    }

    /// Set label text
    pub fn set_label(&mut self, cx: &mut Cx, label: &str) {
        self.label = label.to_string();
        self.view
            .label(ids!(label_row.picker_label))
            .set_text(cx, label);
    }

    /// Set info text (shows below the path)
    pub fn set_info(&mut self, cx: &mut Cx, info: Option<&str>) {
        match info {
            Some(text) => {
                self.view.view(ids!(info_row)).set_visible(cx, true);
                self.view
                    .label(ids!(info_row.info_label))
                    .set_text(cx, text);
            }
            None => {
                self.view.view(ids!(info_row)).set_visible(cx, false);
            }
        }
        self.view.redraw(cx);
    }

    /// Notify that a file was selected (triggers action)
    pub fn notify_selected(&mut self, cx: &mut Cx, scope: &mut Scope, path: PathBuf) {
        self.selected_path = Some(path.clone());
        cx.widget_action(
            self.widget_uid(),
            &scope.path,
            DataflowPickerAction::Selected(path),
        );
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&mut self, cx: &mut Cx, dark_mode: f64) {
        self.dark_mode = dark_mode;

        // Label
        self.view.label(ids!(label_row.picker_label)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );

        // Path container
        self.view.view(ids!(path_row.path_container)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );
        self.view
            .label(ids!(path_row.path_container.path_label))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

        // Browse button
        self.view.button(ids!(path_row.browse_btn)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            },
        );

        // Info label
        self.view.label(ids!(info_row.info_label)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );

        self.view.redraw(cx);
    }
}

impl DataflowPickerRef {
    /// Get currently selected path
    pub fn get_path(&self) -> Option<PathBuf> {
        self.borrow().and_then(|inner| inner.get_path())
    }

    /// Set selected path
    pub fn set_path(&self, cx: &mut Cx, path: Option<PathBuf>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_path(cx, path);
        }
    }

    /// Set full path display
    pub fn set_path_display(&self, cx: &mut Cx, path: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_path_display(cx, path);
        }
    }

    /// Set label text
    pub fn set_label(&self, cx: &mut Cx, label: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_label(cx, label);
        }
    }

    /// Set info text
    pub fn set_info(&self, cx: &mut Cx, info: Option<&str>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_info(cx, info);
        }
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }

    /// Check if file was selected
    pub fn selected(&self, actions: &Actions) -> Option<PathBuf> {
        if let DataflowPickerAction::Selected(path) =
            actions.find_widget_action(self.widget_uid()).cast()
        {
            Some(path)
        } else {
            None
        }
    }

    /// Check if browse button was clicked
    pub fn browse_clicked(&self, actions: &Actions) -> bool {
        matches!(
            actions.find_widget_action(self.widget_uid()).cast(),
            DataflowPickerAction::BrowseClicked
        )
    }
}
