//! Log panel methods for MoFaASRScreen
//!
//! Handles log display, filtering, and clipboard operations.
//! Following mofa-fm's log_panel.rs pattern with:
//! - Timestamp-based throttled updates (200ms)
//! - Plain Label for fast text rendering
//! - Maximum log entry limit to bound memory

use makepad_widgets::*;
use mofa_ui::log_bridge;
use std::time::{Duration, Instant};

use super::{MoFaASRScreen, AsrEngineId};

const MAX_LOG_ENTRIES: usize = 5000;
const MAX_DISPLAY_ENTRIES: usize = 200;
const LOG_UPDATE_THROTTLE: Duration = Duration::from_millis(200);

impl MoFaASRScreen {
    /// Toggle log panel visibility
    pub(super) fn toggle_log_panel(&mut self, cx: &mut Cx) {
        self.log_panel_collapsed = !self.log_panel_collapsed;

        if self.log_panel_width == 0.0 {
            self.log_panel_width = 320.0;
        }

        if self.log_panel_collapsed {
            self.view.view(ids!(log_section)).apply_over(cx, live!{ width: Fit });
            self.view.view(ids!(log_section.log_content_column)).set_visible(cx, false);
            self.view.button(ids!(log_section.toggle_column.toggle_log_btn)).set_text(cx, "<");
            self.view.view(ids!(splitter)).apply_over(cx, live!{ width: 0 });
        } else {
            let width = self.log_panel_width;
            self.view.view(ids!(log_section)).apply_over(cx, live!{ width: (width) });
            self.view.view(ids!(log_section.log_content_column)).set_visible(cx, true);
            self.view.button(ids!(log_section.toggle_column.toggle_log_btn)).set_text(cx, ">");
            self.view.view(ids!(splitter)).apply_over(cx, live!{ width: 16 });

            self.update_log_display_now(cx);
            self.log_display_dirty = false;
            self.last_log_update = Some(Instant::now());
        }

        self.view.redraw(cx);
    }

    /// Mark log display as dirty and update if enough time has passed
    pub(super) fn mark_log_dirty(&mut self, cx: &mut Cx) {
        self.log_display_dirty = true;

        if self.log_panel_collapsed {
            return;
        }

        let should_update = match self.last_log_update {
            None => true,
            Some(last) => last.elapsed() >= LOG_UPDATE_THROTTLE,
        };

        if should_update {
            self.log_display_dirty = false;
            self.last_log_update = Some(Instant::now());
            self.update_log_display_now(cx);
        }
    }

    /// Force immediate update of log display
    fn update_log_display_now(&mut self, cx: &mut Cx) {
        let search_text = self.view.text_input(ids!(log_section.log_content_column.log_header.log_filter_row.log_search)).text().to_lowercase();
        let level_filter = self.log_level_filter;
        let node_filter = self.log_node_filter;

        self.log_filter_cache = (level_filter, node_filter, search_text.clone());

        let filtered_logs: Vec<&str> = self.log_entries.iter()
            .filter_map(|entry| {
                let level_match = match level_filter {
                    0 => true, // ALL
                    1 => entry.contains("[DEBUG]"),
                    2 => entry.contains("[INFO]"),
                    3 => entry.contains("[WARN]"),
                    4 => entry.contains("[ERROR]"),
                    _ => true,
                };
                if !level_match { return None; }

                // Node filter: 0=ALL, 1=ASR, 2=Mic, 3=Bridge
                let node_match = match node_filter {
                    0 => true,
                    1 => entry.contains("[ASR]") || entry.contains("asr") || entry.contains("ASR"),
                    2 => entry.contains("[Mic]") || entry.contains("mic") || entry.contains("Mic"),
                    3 => entry.contains("[Bridge]") || entry.contains("bridge") || entry.contains("Bridge"),
                    _ => true,
                };
                if !node_match { return None; }

                if !search_text.is_empty() {
                    let entry_lower = entry.to_lowercase();
                    if !entry_lower.contains(&search_text) {
                        return None;
                    }
                }

                Some(entry.as_str())
            })
            .collect();

        let total_filtered = filtered_logs.len();
        let display_logs: Vec<&str> = if total_filtered > MAX_DISPLAY_ENTRIES {
            filtered_logs.into_iter().skip(total_filtered - MAX_DISPLAY_ENTRIES).collect()
        } else {
            filtered_logs
        };

        let log_text = if display_logs.is_empty() {
            "No log entries".to_string()
        } else if total_filtered > MAX_DISPLAY_ENTRIES {
            format!("... ({} older entries hidden) ...\n{}",
                total_filtered - MAX_DISPLAY_ENTRIES,
                display_logs.join("\n"))
        } else {
            display_logs.join("\n")
        };

        self.view.label(ids!(log_section.log_content_column.log_scroll.log_content_wrapper.log_content)).set_text(cx, &log_text);
        self.view.redraw(cx);
    }

    /// Update log display with throttling
    pub(super) fn update_log_display(&mut self, cx: &mut Cx) {
        let search_text = self.view.text_input(ids!(log_section.log_content_column.log_header.log_filter_row.log_search)).text().to_lowercase();
        let current_filter = (self.log_level_filter, self.log_node_filter, search_text);

        if current_filter != self.log_filter_cache {
            self.update_log_display_now(cx);
        } else {
            self.mark_log_dirty(cx);
        }
    }

    /// Copy chat messages from a specific engine to clipboard
    pub(super) fn copy_chat_to_clipboard(&mut self, cx: &mut Cx, engine: AsrEngineId) {
        let controller = match engine {
            AsrEngineId::Paraformer => self.paraformer_chat_controller.clone(),
            AsrEngineId::Qwen3Asr => self.qwen3_chat_controller.clone(),
        };
        if let Some(ctrl) = controller {
            let mut guard = ctrl.lock().expect("ChatController mutex poisoned");
            let state = guard.dangerous_state_mut();
            let text: Vec<String> = state.messages.iter().map(|msg| {
                let role = if msg.from == moly_kit::prelude::EntityId::User { "User" } else { "ASR" };
                format!("[{}] {}", role, msg.content.text)
            }).collect();
            let clipboard_text = if text.is_empty() {
                "No messages".to_string()
            } else {
                text.join("\n")
            };
            cx.copy_to_clipboard(&clipboard_text);
        }
        // Start copy flash animation
        self.copy_flash_engine = Some(engine);
        self.copy_flash_start = Some(std::time::Instant::now());
        cx.new_next_frame();
    }

    /// Copy filtered logs to clipboard
    pub(super) fn copy_logs_to_clipboard(&mut self, cx: &mut Cx) {
        let search_text = self.view.text_input(ids!(log_section.log_content_column.log_header.log_filter_row.log_search)).text().to_lowercase();
        let level_filter = self.log_level_filter;
        let node_filter = self.log_node_filter;

        let filtered_logs: Vec<&str> = self.log_entries.iter()
            .filter_map(|entry| {
                let level_match = match level_filter {
                    0 => true,
                    1 => entry.contains("[DEBUG]"),
                    2 => entry.contains("[INFO]"),
                    3 => entry.contains("[WARN]"),
                    4 => entry.contains("[ERROR]"),
                    _ => true,
                };
                if !level_match { return None; }

                let node_match = match node_filter {
                    0 => true,
                    1 => entry.contains("[ASR]") || entry.contains("asr") || entry.contains("ASR"),
                    2 => entry.contains("[Mic]") || entry.contains("mic") || entry.contains("Mic"),
                    3 => entry.contains("[Bridge]") || entry.contains("bridge") || entry.contains("Bridge"),
                    _ => true,
                };
                if !node_match { return None; }

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

    /// Add a log entry (throttled)
    pub(super) fn add_log(&mut self, cx: &mut Cx, entry: &str) {
        self.log_entries.push(entry.to_string());

        if self.log_entries.len() > MAX_LOG_ENTRIES {
            let excess = self.log_entries.len() - MAX_LOG_ENTRIES;
            self.log_entries.drain(0..excess);
        }

        self.mark_log_dirty(cx);
    }

    /// Poll Rust log messages
    pub(super) fn poll_rust_logs(&mut self, cx: &mut Cx) {
        let logs = log_bridge::poll_logs();
        if logs.is_empty() {
            return;
        }

        for log_msg in logs {
            self.log_entries.push(log_msg.format());
        }

        if self.log_entries.len() > MAX_LOG_ENTRIES {
            let excess = self.log_entries.len() - MAX_LOG_ENTRIES;
            self.log_entries.drain(0..excess);
        }

        self.mark_log_dirty(cx);
    }
}
