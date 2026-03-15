//! Log panel methods for MoFaDebateScreen
//!
//! Handles log display, filtering, and clipboard operations.

use makepad_widgets::*;
use mofa_ui::log_bridge;

use super::MoFaDebateScreen;

impl MoFaDebateScreen {
    /// Toggle log panel visibility
    pub(super) fn toggle_log_panel(&mut self, cx: &mut Cx) {
        self.log_panel_collapsed = !self.log_panel_collapsed;

        if self.log_panel_width == 0.0 {
            self.log_panel_width = 320.0;
        }

        if self.log_panel_collapsed {
            // Collapse: hide log content, show only toggle button
            self.view
                .view(ids!(log_section))
                .apply_over(cx, live! { width: Fit });
            self.view
                .view(ids!(log_section.log_content_column))
                .set_visible(cx, false);
            self.view
                .button(ids!(log_section.toggle_column.toggle_log_btn))
                .set_text(cx, "<");
            self.view
                .view(ids!(splitter))
                .apply_over(cx, live! { width: 0 });
        } else {
            // Expand: show log content at saved width
            let width = self.log_panel_width;
            self.view
                .view(ids!(log_section))
                .apply_over(cx, live! { width: (width) });
            self.view
                .view(ids!(log_section.log_content_column))
                .set_visible(cx, true);
            self.view
                .button(ids!(log_section.toggle_column.toggle_log_btn))
                .set_text(cx, ">");
            self.view
                .view(ids!(splitter))
                .apply_over(cx, live! { width: 16 });
        }

        self.view.redraw(cx);
    }

    /// Resize log panel via splitter drag
    pub(super) fn resize_log_panel(&mut self, cx: &mut Cx, abs_x: f64) {
        let container_rect = self.view.area().rect(cx);
        let padding = 16.0; // Match screen padding
        let new_log_width = (container_rect.pos.x + container_rect.size.x - abs_x - padding)
            .max(150.0) // Minimum log panel width
            .min(container_rect.size.x - 400.0); // Leave space for main content

        self.log_panel_width = new_log_width;

        self.view.view(ids!(log_section)).apply_over(
            cx,
            live! {
                width: (new_log_width)
            },
        );

        self.view.redraw(cx);
    }

    /// Update log display based on current filter and search
    pub(super) fn update_log_display(&mut self, cx: &mut Cx) {
        let search_text = self
            .view
            .text_input(ids!(
                log_section
                    .log_content_column
                    .log_header
                    .log_filter_row
                    .log_search
            ))
            .text()
            .to_lowercase();
        let level_filter = self.log_level_filter;
        let node_filter = self.log_node_filter;

        // Filter log entries
        let filtered_logs: Vec<&String> = self
            .log_entries
            .iter()
            .filter(|entry| {
                // Level filter: 0=ALL, 1=DEBUG, 2=INFO, 3=WARN, 4=ERROR
                let level_match = match level_filter {
                    0 => true, // ALL
                    1 => entry.contains("[DEBUG]"),
                    2 => entry.contains("[INFO]"),
                    3 => entry.contains("[WARN]"),
                    4 => entry.contains("[ERROR]"),
                    _ => true,
                };

                // Node filter: 0=ALL, 1=ASR, 2=TTS, 3=LLM, 4=Bridge, 5=Monitor, 6=App
                let node_match = match node_filter {
                    0 => true, // All Nodes
                    1 => entry.contains("[ASR]") || entry.to_lowercase().contains("asr"),
                    2 => entry.contains("[TTS]") || entry.to_lowercase().contains("tts"),
                    3 => entry.contains("[LLM]") || entry.to_lowercase().contains("llm"),
                    4 => entry.contains("[Bridge]") || entry.to_lowercase().contains("bridge"),
                    5 => entry.contains("[Monitor]") || entry.to_lowercase().contains("monitor"),
                    6 => entry.contains("[App]") || entry.to_lowercase().contains("app"),
                    _ => true,
                };

                // Search filter
                let search_match =
                    search_text.is_empty() || entry.to_lowercase().contains(&search_text);

                level_match && node_match && search_match
            })
            .collect();

        // Build display text (use double newlines for Markdown paragraph breaks)
        let log_text = if filtered_logs.is_empty() {
            "*No log entries*".to_string()
        } else {
            filtered_logs
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n\n")
        };

        // Update markdown display
        self.view
            .markdown(ids!(
                log_section
                    .log_content_column
                    .log_scroll
                    .log_content_wrapper
                    .log_content
            ))
            .set_text(cx, &log_text);
        self.view.redraw(cx);
    }

    /// Copy filtered logs to clipboard
    pub(super) fn copy_logs_to_clipboard(&mut self, cx: &mut Cx) {
        let search_text = self
            .view
            .text_input(ids!(
                log_section
                    .log_content_column
                    .log_header
                    .log_filter_row
                    .log_search
            ))
            .text()
            .to_lowercase();
        let level_filter = self.log_level_filter;
        let node_filter = self.log_node_filter;

        // Filter log entries (same as update_log_display)
        let filtered_logs: Vec<&String> = self
            .log_entries
            .iter()
            .filter(|entry| {
                let level_match = match level_filter {
                    0 => true,
                    1 => entry.contains("[DEBUG]"),
                    2 => entry.contains("[INFO]"),
                    3 => entry.contains("[WARN]"),
                    4 => entry.contains("[ERROR]"),
                    _ => true,
                };
                let node_match = match node_filter {
                    0 => true,
                    1 => entry.contains("[ASR]") || entry.to_lowercase().contains("asr"),
                    2 => entry.contains("[TTS]") || entry.to_lowercase().contains("tts"),
                    3 => entry.contains("[LLM]") || entry.to_lowercase().contains("llm"),
                    4 => entry.contains("[Bridge]") || entry.to_lowercase().contains("bridge"),
                    5 => entry.contains("[Monitor]") || entry.to_lowercase().contains("monitor"),
                    6 => entry.contains("[App]") || entry.to_lowercase().contains("app"),
                    _ => true,
                };
                let search_match =
                    search_text.is_empty() || entry.to_lowercase().contains(&search_text);
                level_match && node_match && search_match
            })
            .collect();

        let log_text = if filtered_logs.is_empty() {
            "No log entries".to_string()
        } else {
            // Use single newlines for clipboard (plain text, not Markdown)
            filtered_logs
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        };

        cx.copy_to_clipboard(&log_text);
    }

    /// Copy chat messages to clipboard
    pub(super) fn copy_chat_to_clipboard(&mut self, cx: &mut Cx) {
        let chat_text = if self.chat_messages.is_empty() {
            "No chat messages".to_string()
        } else {
            self.chat_messages
                .iter()
                .map(|msg| format!("[{}] {}", msg.sender, msg.content))
                .collect::<Vec<_>>()
                .join("\n\n")
        };

        cx.copy_to_clipboard(&chat_text);
    }

    /// Add a log entry
    pub(super) fn add_log(&mut self, cx: &mut Cx, entry: &str) {
        self.log_entries.push(entry.to_string());
        self.update_log_display(cx);
    }

    /// Poll Rust log messages and add them to the system log
    pub(super) fn poll_rust_logs(&mut self, cx: &mut Cx) {
        let logs = log_bridge::poll_logs();
        if logs.is_empty() {
            return;
        }

        for log_msg in logs {
            self.log_entries.push(log_msg.format());
        }

        // Only update display if we got new logs
        self.update_log_display(cx);
    }

    /// Clear all logs
    pub(super) fn clear_logs(&mut self, cx: &mut Cx) {
        self.log_entries.clear();
        self.update_log_display(cx);
    }
}
