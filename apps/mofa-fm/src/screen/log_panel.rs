//! Log panel methods for MoFaFMScreen
//!
//! Handles log display, filtering, and clipboard operations.
//! Optimized for performance with:
//! - Timestamp-based throttled updates (200ms) to avoid per-entry re-renders
//! - Plain Label instead of Markdown for faster text rendering
//! - Cached filter state to skip unnecessary re-filtering
//! - Maximum log entry limit to bound memory

use makepad_widgets::*;
use mofa_ui::log_bridge;
use std::time::{Duration, Instant};

use super::MoFaFMScreen;

/// Maximum number of log entries to keep in memory (oldest entries are pruned)
const MAX_LOG_ENTRIES: usize = 5000;

/// Maximum number of log entries to display (for performance)
/// Full history still searchable, but only recent entries rendered
const MAX_DISPLAY_ENTRIES: usize = 200;

/// Throttle interval for log display updates
const LOG_UPDATE_THROTTLE: Duration = Duration::from_millis(200);

impl MoFaFMScreen {
    /// Toggle log panel visibility
    pub(super) fn toggle_log_panel(&mut self, cx: &mut Cx) {
        ::log::info!(
            "toggle_log_panel called, collapsed={}",
            self.log_panel_collapsed
        );
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

            // Always update display when expanding (logs may have accumulated while collapsed)
            self.update_log_display_now(cx);
            self.log_display_dirty = false;
            self.last_log_update = Some(Instant::now());
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

    /// Mark log display as dirty and update if enough time has passed
    /// This is the core of the sliding window - checks time-based throttle
    pub(super) fn mark_log_dirty(&mut self, cx: &mut Cx) {
        self.log_display_dirty = true;

        // Only update if panel is visible
        if self.log_panel_collapsed {
            return;
        }

        // Check if enough time has passed since last update (sliding window throttle)
        let should_update = match self.last_log_update {
            None => true, // First update
            Some(last) => last.elapsed() >= LOG_UPDATE_THROTTLE,
        };

        if should_update {
            self.log_display_dirty = false;
            self.last_log_update = Some(Instant::now());
            self.update_log_display_now(cx);
        }
    }

    /// Force immediate update of log display (called by filter change or expand)
    fn update_log_display_now(&mut self, cx: &mut Cx) {
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

        // Update filter cache
        self.log_filter_cache = (level_filter, node_filter, search_text.clone());

        // Filter log entries with optimized matching
        let filtered_logs: Vec<&str> = self
            .log_entries
            .iter()
            .filter_map(|entry| {
                // Level filter: 0=ALL, 1=DEBUG, 2=INFO, 3=WARN, 4=ERROR
                let level_match = match level_filter {
                    0 => true, // ALL
                    1 => entry.contains("[DEBUG]"),
                    2 => entry.contains("[INFO]"),
                    3 => entry.contains("[WARN]"),
                    4 => entry.contains("[ERROR]"),
                    _ => true,
                };
                if !level_match {
                    return None;
                }

                // Node filter: 0=ALL, 1=ASR, 2=TTS, 3=LLM, 4=Bridge, 5=Monitor, 6=App
                // Use case-insensitive matching only when needed
                let node_match = match node_filter {
                    0 => true, // All Nodes
                    1 => entry.contains("[ASR]") || entry.contains("asr") || entry.contains("ASR"),
                    2 => entry.contains("[TTS]") || entry.contains("tts") || entry.contains("TTS"),
                    3 => entry.contains("[LLM]") || entry.contains("llm") || entry.contains("LLM"),
                    4 => {
                        entry.contains("[Bridge]")
                            || entry.contains("bridge")
                            || entry.contains("Bridge")
                    }
                    5 => {
                        entry.contains("[Monitor]")
                            || entry.contains("monitor")
                            || entry.contains("Monitor")
                    }
                    6 => entry.contains("[App]") || entry.contains("app") || entry.contains("App"),
                    _ => true,
                };
                if !node_match {
                    return None;
                }

                // Search filter - only do lowercase conversion if search is active
                if !search_text.is_empty() {
                    // Use contains with lowercase only for search (most expensive operation)
                    let entry_lower = entry.to_lowercase();
                    if !entry_lower.contains(&search_text) {
                        return None;
                    }
                }

                Some(entry.as_str())
            })
            .collect();

        // Limit display to last MAX_DISPLAY_ENTRIES for performance
        // (keeps UI responsive while full history remains searchable)
        let total_filtered = filtered_logs.len();
        let display_logs: Vec<&str> = if total_filtered > MAX_DISPLAY_ENTRIES {
            filtered_logs
                .into_iter()
                .skip(total_filtered - MAX_DISPLAY_ENTRIES)
                .collect()
        } else {
            filtered_logs
        };

        // Build display text (single newlines for plain Label)
        let log_text = if display_logs.is_empty() {
            "No log entries".to_string()
        } else if total_filtered > MAX_DISPLAY_ENTRIES {
            // Show indicator that older logs are hidden
            format!(
                "... ({} older entries hidden) ...\n{}",
                total_filtered - MAX_DISPLAY_ENTRIES,
                display_logs.join("\n")
            )
        } else {
            display_logs.join("\n")
        };

        // Update Label display (much faster than Markdown)
        self.view
            .label(ids!(
                log_section
                    .log_content_column
                    .log_scroll
                    .log_content_wrapper
                    .log_content
            ))
            .set_text(cx, &log_text);
        self.view.redraw(cx);
    }

    /// Update log display based on current filter and search
    /// This is the public API - it marks dirty and schedules throttled update
    pub(super) fn update_log_display(&mut self, cx: &mut Cx) {
        // Check if filter state changed (need immediate update)
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
        let current_filter = (self.log_level_filter, self.log_node_filter, search_text);

        if current_filter != self.log_filter_cache {
            // Filter changed - update immediately
            self.update_log_display_now(cx);
        } else {
            // Just new logs - throttle the update
            self.mark_log_dirty(cx);
        }
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

        // Filter log entries (same logic as update_log_display_now)
        let filtered_logs: Vec<&str> = self
            .log_entries
            .iter()
            .filter_map(|entry| {
                let level_match = match level_filter {
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

                let node_match = match node_filter {
                    0 => true,
                    1 => entry.contains("[ASR]") || entry.contains("asr") || entry.contains("ASR"),
                    2 => entry.contains("[TTS]") || entry.contains("tts") || entry.contains("TTS"),
                    3 => entry.contains("[LLM]") || entry.contains("llm") || entry.contains("LLM"),
                    4 => {
                        entry.contains("[Bridge]")
                            || entry.contains("bridge")
                            || entry.contains("Bridge")
                    }
                    5 => {
                        entry.contains("[Monitor]")
                            || entry.contains("monitor")
                            || entry.contains("Monitor")
                    }
                    6 => entry.contains("[App]") || entry.contains("app") || entry.contains("App"),
                    _ => true,
                };
                if !node_match {
                    return None;
                }

                if !search_text.is_empty() {
                    let entry_lower = entry.to_lowercase();
                    if !entry_lower.contains(&search_text) {
                        return None;
                    }
                }

                Some(entry.as_str())
            })
            .collect();

        let log_text = if filtered_logs.is_empty() {
            "No log entries".to_string()
        } else {
            filtered_logs.join("\n")
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

    /// Add a log entry (throttled - doesn't immediately update display)
    pub(super) fn add_log(&mut self, cx: &mut Cx, entry: &str) {
        self.log_entries.push(entry.to_string());

        // Prune oldest entries if over limit
        if self.log_entries.len() > MAX_LOG_ENTRIES {
            let excess = self.log_entries.len() - MAX_LOG_ENTRIES;
            self.log_entries.drain(0..excess);
        }

        // Mark dirty for throttled update (don't update immediately)
        self.mark_log_dirty(cx);
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

        // Prune oldest entries if over limit
        if self.log_entries.len() > MAX_LOG_ENTRIES {
            let excess = self.log_entries.len() - MAX_LOG_ENTRIES;
            self.log_entries.drain(0..excess);
        }

        // Mark dirty for throttled update (don't update immediately)
        self.mark_log_dirty(cx);
    }

    /// Clear all logs
    pub(super) fn clear_logs(&mut self, cx: &mut Cx) {
        self.log_entries.clear();
        self.log_display_dirty = false;
        // Immediate update for clear (user expects instant feedback)
        self.update_log_display_now(cx);
    }
}
