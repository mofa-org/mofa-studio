//! Log Panel Widget
//!
//! A filterable log display panel with level/node filters and search.
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::widgets::log_panel::*;
//!
//!     logs = <LogPanel> {}
//! }
//! ```
//!
//! ## Adding Logs
//!
//! ```rust,ignore
//! let panel = self.view.log_panel(id!(logs));
//! panel.add_log(cx, "[INFO] [App] Starting...");
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Panel styling constants
    PANEL_RADIUS = 8.0

    /// Copy button with animated feedback
    LogCopyButton = <View> {
        width: 28, height: 24
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance copied: 0.0
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let c = self.rect_size * 0.5;

                let gray_light = vec4(0.886, 0.910, 0.941, 1.0);
                let blue_light = vec4(0.231, 0.510, 0.965, 1.0);
                let teal_light = vec4(0.078, 0.722, 0.651, 1.0);
                let green_light = vec4(0.133, 0.773, 0.373, 1.0);

                let gray_dark = vec4(0.334, 0.371, 0.451, 1.0);
                let purple_dark = vec4(0.639, 0.380, 0.957, 1.0);
                let cyan_dark = vec4(0.133, 0.831, 0.894, 1.0);
                let green_dark = vec4(0.290, 0.949, 0.424, 1.0);

                let gray = mix(gray_light, gray_dark, self.dark_mode);
                let c1 = mix(blue_light, purple_dark, self.dark_mode);
                let c2 = mix(teal_light, cyan_dark, self.dark_mode);
                let c3 = mix(green_light, green_dark, self.dark_mode);

                let t = self.copied;
                let bg_color = mix(
                    mix(mix(gray, c1, clamp(t * 3.0, 0.0, 1.0)),
                        c2, clamp((t - 0.33) * 3.0, 0.0, 1.0)),
                    c3, clamp((t - 0.66) * 3.0, 0.0, 1.0)
                );

                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                sdf.fill(bg_color);

                let icon_base = mix(vec4(0.294, 0.333, 0.388, 1.0), vec4(0.580, 0.639, 0.722, 1.0), self.dark_mode);
                let icon_color = mix(icon_base, vec4(1.0, 1.0, 1.0, 1.0), smoothstep(0.0, 0.3, self.copied));

                sdf.box(c.x - 4.0, c.y - 2.0, 8.0, 9.0, 1.0);
                sdf.stroke(icon_color, 1.2);
                sdf.box(c.x - 2.0, c.y - 5.0, 8.0, 9.0, 1.0);
                sdf.fill(bg_color);
                sdf.box(c.x - 2.0, c.y - 5.0, 8.0, 9.0, 1.0);
                sdf.stroke(icon_color, 1.2);

                return sdf.result;
            }
        }
    }

    /// Filter dropdown styling
    LogFilterDropDown = <DropDown> {
        height: 24
        popup_menu_position: BelowInput
        draw_bg: {
            color: (HOVER_BG)
            border_color: (SLATE_200)
            border_radius: 2.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 2.0);
                sdf.fill((HOVER_BG));
                let ax = self.rect_size.x - 12.0;
                let ay = self.rect_size.y * 0.5 - 2.0;
                sdf.move_to(ax - 3.0, ay);
                sdf.line_to(ax, ay + 4.0);
                sdf.line_to(ax + 3.0, ay);
                sdf.stroke((TEXT_PRIMARY), 1.5);
                return sdf.result;
            }
        }
        draw_text: {
            text_style: { font_size: 10.0 }
            fn get_color(self) -> vec4 {
                return (TEXT_PRIMARY);
            }
        }
        popup_menu: {
            draw_bg: {
                color: (WHITE)
                border_color: (BORDER)
                border_size: 1.0
                border_radius: 2.0
            }
            menu_item: {
                draw_bg: {
                    color: (WHITE)
                    color_hover: (GRAY_100)
                }
                draw_text: {
                    fn get_color(self) -> vec4 {
                        return mix(
                            mix((GRAY_700), (TEXT_PRIMARY), self.active),
                            (TEXT_PRIMARY),
                            self.hover
                        );
                    }
                }
            }
        }
    }

    /// Search icon
    SearchIcon = <View> {
        width: 20, height: 24
        align: {x: 0.5, y: 0.5}
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let c = self.rect_size * 0.5;
                sdf.circle(c.x - 2.0, c.y - 2.0, 5.0);
                sdf.stroke((GRAY_500), 1.5);
                sdf.move_to(c.x + 1.5, c.y + 1.5);
                sdf.line_to(c.x + 6.0, c.y + 6.0);
                sdf.stroke((GRAY_500), 1.5);
                return sdf.result;
            }
        }
    }

    /// Log panel widget - displays filterable log entries
    pub MofaLogPanel = {{MofaLogPanel}} {
        width: Fill, height: Fill
        flow: Down
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            border_radius: (PANEL_RADIUS)
            fn get_color(self) -> vec4 {
                return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
            }
        }

        // Header section
        header = <View> {
            width: Fill, height: Fit
            flow: Down
            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    return mix((SLATE_50), (SLATE_800), self.dark_mode);
                }
            }

            // Title row
            title_row = <View> {
                width: Fill, height: Fit
                padding: {left: 12, right: 12, top: 10, bottom: 6}
                title_label = <Label> {
                    text: "System Log"
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: { font_size: 13.0 }
                        fn get_color(self) -> vec4 {
                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                        }
                    }
                }
            }

            // Filter row
            filter_row = <View> {
                width: Fill, height: 32
                flow: Right
                align: {y: 0.5}
                padding: {left: 8, right: 8, bottom: 6}
                spacing: 6

                level_filter = <LogFilterDropDown> {
                    width: 70
                    labels: ["ALL", "DEBUG", "INFO", "WARN", "ERROR"]
                    values: [ALL, DEBUG, INFO, WARN, ERROR]
                }

                node_filter = <LogFilterDropDown> {
                    width: 85
                    labels: ["All Nodes", "ASR", "TTS", "LLM", "Bridge", "Monitor", "App"]
                    values: [ALL, ASR, TTS, LLM, BRIDGE, MONITOR, APP]
                }

                search_icon = <SearchIcon> {}

                search_input = <TextInput> {
                    width: Fill, height: 24
                    empty_text: "Search..."
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: 2.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                            sdf.fill(bg);
                            return sdf.result;
                        }
                    }
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: { font_size: 10.0 }
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

                copy_btn = <LogCopyButton> {}
            }
        }

        // Log content
        log_scroll = <ScrollYView> {
            width: Fill, height: Fill
            flow: Down
            scroll_bars: <ScrollBars> {
                show_scroll_x: false
                show_scroll_y: true
            }

            content_wrapper = <View> {
                width: Fill, height: Fit
                padding: {left: 12, right: 12, top: 8, bottom: 8}
                flow: Down

                content = <Label> {
                    width: Fill, height: Fit
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: { font_size: 10.0 }
                        wrap: Word
                        fn get_color(self) -> vec4 {
                            return mix((GRAY_600), (TEXT_PRIMARY_DARK), self.dark_mode);
                        }
                    }
                    text: ""
                }
            }
        }
    }
}

/// Log level filter options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    All = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

/// Node filter options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogNode {
    All = 0,
    Asr = 1,
    Tts = 2,
    Llm = 3,
    Bridge = 4,
    Monitor = 5,
    App = 6,
}

/// Maximum entries to keep in memory
const MAX_LOG_ENTRIES: usize = 5000;
/// Maximum entries to display
const MAX_DISPLAY_ENTRIES: usize = 200;

/// Actions emitted by LogPanel
#[derive(Clone, Debug, DefaultNone)]
pub enum LogPanelAction {
    None,
    /// Copy button was clicked
    CopyClicked,
    /// Filter changed
    FilterChanged,
}

#[derive(Live, LiveHook, Widget)]
pub struct MofaLogPanel {
    #[deref]
    view: View,

    /// All log entries
    #[rust]
    entries: Vec<String>,

    /// Current level filter index
    #[rust]
    level_filter: usize,

    /// Current node filter index
    #[rust]
    node_filter: usize,

    /// Cached search text
    #[rust]
    search_cache: String,

    /// Dark mode value
    #[rust]
    dark_mode: f64,

    /// Display needs update
    #[rust]
    display_dirty: bool,
}

impl Widget for MofaLogPanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let actions = cx.capture_actions(|cx| self.view.handle_event(cx, event, scope));

        // Check level filter change
        let level_dd = self.view.drop_down(ids!(header.filter_row.level_filter));
        if let Some(idx) = level_dd.changed(&actions) {
            self.level_filter = idx;
            self.update_display(cx);
            cx.widget_action(
                self.widget_uid(),
                &scope.path,
                LogPanelAction::FilterChanged,
            );
        }

        // Check node filter change
        let node_dd = self.view.drop_down(ids!(header.filter_row.node_filter));
        if let Some(idx) = node_dd.changed(&actions) {
            self.node_filter = idx;
            self.update_display(cx);
            cx.widget_action(
                self.widget_uid(),
                &scope.path,
                LogPanelAction::FilterChanged,
            );
        }

        // Check search text change
        let search_text = self
            .view
            .text_input(ids!(header.filter_row.search_input))
            .text();
        if search_text != self.search_cache {
            self.search_cache = search_text;
            self.update_display(cx);
        }

        // Check copy button click
        let copy_btn = self.view.view(ids!(header.filter_row.copy_btn));
        match event.hits(cx, copy_btn.area()) {
            Hit::FingerUp(fe) if fe.was_tap() => {
                cx.widget_action(self.widget_uid(), &scope.path, LogPanelAction::CopyClicked);
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl MofaLogPanel {
    /// Add a log entry
    pub fn add_log(&mut self, cx: &mut Cx, entry: &str) {
        self.entries.push(entry.to_string());

        // Prune oldest entries if over limit
        if self.entries.len() > MAX_LOG_ENTRIES {
            let excess = self.entries.len() - MAX_LOG_ENTRIES;
            self.entries.drain(0..excess);
        }

        self.display_dirty = true;
    }

    /// Mark display as needing update (call this after batch adds)
    pub fn mark_dirty(&mut self) {
        self.display_dirty = true;
    }

    /// Update display if dirty
    pub fn update_if_dirty(&mut self, cx: &mut Cx) {
        if self.display_dirty {
            self.update_display(cx);
            self.display_dirty = false;
        }
    }

    /// Force update the display
    pub fn update_display(&mut self, cx: &mut Cx) {
        let search_lower = self.search_cache.to_lowercase();

        // Filter entries
        let filtered: Vec<&str> = self
            .entries
            .iter()
            .filter_map(|entry| {
                // Level filter
                let level_match = match self.level_filter {
                    0 => true,
                    1 => entry.contains("[DEBUG]"),
                    2 => entry.contains("[INFO]"),
                    3 => entry.contains("[WARN]"),
                    4 => entry.contains("[ERROR]"),
                    _ => true,
                };
                if !level_match {
                    return None;
                }

                // Node filter
                let node_match = match self.node_filter {
                    0 => true,
                    1 => entry.contains("[ASR]") || entry.to_lowercase().contains("asr"),
                    2 => entry.contains("[TTS]") || entry.to_lowercase().contains("tts"),
                    3 => entry.contains("[LLM]") || entry.to_lowercase().contains("llm"),
                    4 => entry.contains("[Bridge]") || entry.to_lowercase().contains("bridge"),
                    5 => entry.contains("[Monitor]") || entry.to_lowercase().contains("monitor"),
                    6 => entry.contains("[App]") || entry.to_lowercase().contains("app"),
                    _ => true,
                };
                if !node_match {
                    return None;
                }

                // Search filter
                if !search_lower.is_empty() {
                    if !entry.to_lowercase().contains(&search_lower) {
                        return None;
                    }
                }

                Some(entry.as_str())
            })
            .collect();

        // Limit display
        let total = filtered.len();
        let display: Vec<&str> = if total > MAX_DISPLAY_ENTRIES {
            filtered
                .into_iter()
                .skip(total - MAX_DISPLAY_ENTRIES)
                .collect()
        } else {
            filtered
        };

        // Build text
        let text = if display.is_empty() {
            "No log entries".to_string()
        } else if total > MAX_DISPLAY_ENTRIES {
            format!(
                "... ({} older entries hidden) ...\n{}",
                total - MAX_DISPLAY_ENTRIES,
                display.join("\n")
            )
        } else {
            display.join("\n")
        };

        self.view
            .label(ids!(log_scroll.content_wrapper.content))
            .set_text(cx, &text);
        self.view.redraw(cx);
    }

    /// Clear all logs
    pub fn clear(&mut self, cx: &mut Cx) {
        self.entries.clear();
        self.display_dirty = false;
        self.view
            .label(ids!(log_scroll.content_wrapper.content))
            .set_text(cx, "No log entries");
        self.view.redraw(cx);
    }

    /// Get filtered logs for copying
    pub fn get_filtered_logs(&self) -> String {
        let search_lower = self.search_cache.to_lowercase();

        let filtered: Vec<&str> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let level_match = match self.level_filter {
                    0 => true,
                    1 => entry.contains("[DEBUG]"),
                    2 => entry.contains("[INFO]"),
                    3 => entry.contains("[WARN]"),
                    4 => entry.contains("[ERROR]"),
                    _ => true,
                };
                if !level_match {
                    return None;
                }

                let node_match = match self.node_filter {
                    0 => true,
                    1 => entry.contains("[ASR]") || entry.to_lowercase().contains("asr"),
                    2 => entry.contains("[TTS]") || entry.to_lowercase().contains("tts"),
                    3 => entry.contains("[LLM]") || entry.to_lowercase().contains("llm"),
                    4 => entry.contains("[Bridge]") || entry.to_lowercase().contains("bridge"),
                    5 => entry.contains("[Monitor]") || entry.to_lowercase().contains("monitor"),
                    6 => entry.contains("[App]") || entry.to_lowercase().contains("app"),
                    _ => true,
                };
                if !node_match {
                    return None;
                }

                if !search_lower.is_empty() {
                    if !entry.to_lowercase().contains(&search_lower) {
                        return None;
                    }
                }

                Some(entry.as_str())
            })
            .collect();

        if filtered.is_empty() {
            "No log entries".to_string()
        } else {
            filtered.join("\n")
        }
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
        self.view
            .label(ids!(header.title_row.title_label))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
        self.view
            .text_input(ids!(header.filter_row.search_input))
            .apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
        self.view.view(ids!(header.filter_row.copy_btn)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );
        self.view
            .label(ids!(log_scroll.content_wrapper.content))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
        self.view.redraw(cx);
    }

    /// Set copy button animation state
    pub fn set_copy_flash(&mut self, cx: &mut Cx, value: f64) {
        self.view.view(ids!(header.filter_row.copy_btn)).apply_over(
            cx,
            live! {
                draw_bg: { copied: (value) }
            },
        );
        self.view.redraw(cx);
    }
}

impl MofaLogPanelRef {
    /// Add a log entry
    pub fn add_log(&self, cx: &mut Cx, entry: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.add_log(cx, entry);
        }
    }

    /// Mark dirty for update
    pub fn mark_dirty(&self) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.mark_dirty();
        }
    }

    /// Update display if dirty
    pub fn update_if_dirty(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.update_if_dirty(cx);
        }
    }

    /// Force update display
    pub fn update_display(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.update_display(cx);
        }
    }

    /// Clear logs
    pub fn clear(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.clear(cx);
        }
    }

    /// Get filtered logs
    pub fn get_filtered_logs(&self) -> String {
        self.borrow()
            .map(|inner| inner.get_filtered_logs())
            .unwrap_or_default()
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }

    /// Set copy flash
    pub fn set_copy_flash(&self, cx: &mut Cx, value: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_copy_flash(cx, value);
        }
    }

    /// Check if copy was clicked
    pub fn copy_clicked(&self, actions: &Actions) -> bool {
        if let LogPanelAction::CopyClicked = actions.find_widget_action(self.widget_uid()).cast() {
            true
        } else {
            false
        }
    }

    /// Check if filter changed
    pub fn filter_changed(&self, actions: &Actions) -> bool {
        if let LogPanelAction::FilterChanged = actions.find_widget_action(self.widget_uid()).cast()
        {
            true
        } else {
            false
        }
    }
}
