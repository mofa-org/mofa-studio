//! Dora integration handlers for MoFaDebateScreen
//!
//! Handles dataflow control, event processing, and participant panel updates.
//!
//! Simplified data flow architecture (all SharedDoraState):
//! - Chat: UI polls SharedDoraState.chat directly
//! - Audio: UI drains SharedDoraState.audio directly
//! - Logs: UI polls SharedDoraState.logs directly
//! - Status: UI polls SharedDoraState.status directly
//! - Control flow only: DoraEvent channel (DataflowStarted, DataflowStopped, Error)

use makepad_widgets::*;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::dora_integration::{DoraEvent, DoraIntegration};
use mofa_settings::data::Preferences;
use mofa_ui::{ConnectionStatus, MofaHeroWidgetExt};

use super::{ChatMessageEntry, MoFaDebateScreen};

impl MoFaDebateScreen {
    // =====================================================
    // Dora Integration Methods
    // =====================================================

    /// Initialize dora integration (lazy initialization)
    pub(super) fn init_dora(&mut self, cx: &mut Cx) {
        if self.dora_integration.is_some() {
            return;
        }

        ::log::info!("Initializing Dora integration");
        let integration = DoraIntegration::new();
        self.dora_integration = Some(integration);

        // Start timer to poll for dora events (100ms interval)
        self.dora_timer = cx.start_interval(0.1);

        // Look for default dataflow relative to current working directory
        // Check multiple possible locations
        let dataflow_path = std::env::current_dir().ok().and_then(|cwd| {
            // First try: apps/mofa-debate/dataflow/voice-chat.yml (when running from workspace root)
            let app_path = cwd
                .join("apps")
                .join("mofa-debate")
                .join("dataflow")
                .join("voice-chat.yml");
            if app_path.exists() {
                return Some(app_path);
            }
            // Second try: dataflow/voice-chat.yml (when running from app directory)
            let local_path = cwd.join("dataflow").join("voice-chat.yml");
            if local_path.exists() {
                return Some(local_path);
            }
            None
        });
        self.dataflow_path = dataflow_path;

        ::log::info!(
            "Dora integration initialized, dataflow: {:?}",
            self.dataflow_path
        );
    }

    /// Start a dataflow
    pub fn start_dataflow(&mut self, cx: &mut Cx, path: impl Into<PathBuf>) {
        self.init_dora(cx);

        let path = path.into();
        if let Some(ref dora) = self.dora_integration {
            if dora.start_dataflow(&path) {
                ::log::info!("Starting dataflow: {:?}", path);
                self.dataflow_path = Some(path);
                self.add_log(cx, &format!("[INFO] [App] Starting dataflow..."));
            } else {
                ::log::error!("Failed to start dataflow: {:?}", path);
                self.add_log(cx, &format!("[ERROR] [App] Failed to start dataflow"));
            }
        }
    }

    /// Stop the current dataflow
    pub fn stop_dataflow(&mut self, cx: &mut Cx) {
        if let Some(ref dora) = self.dora_integration {
            if dora.stop_dataflow() {
                ::log::info!("Stopping dataflow");
                self.add_log(cx, "[INFO] [App] Dataflow stopped");
            }
        }
    }

    /// Poll for dora events and update UI
    ///
    /// All data is polled from SharedDoraState:
    /// - chat: streaming messages from LLM
    /// - audio: TTS audio chunks
    /// - logs: system log entries
    /// - status: bridge connection status
    ///
    /// DoraEvents are only used for control flow (DataflowStarted/Stopped, Error)
    pub(super) fn poll_dora_events(&mut self, cx: &mut Cx) {
        // =====================================================
        // Poll SharedDoraState for all data
        // =====================================================
        // Collect data first, then update UI (avoids borrow checker issues)
        let (chat_messages, audio_chunks, log_entries, status) =
            if let Some(ref dora) = self.dora_integration {
                let shared_state = dora.shared_dora_state();
                (
                    shared_state.chat.read_if_dirty(),
                    shared_state.audio.drain(),
                    shared_state.logs.read_if_dirty(),
                    shared_state.status.read_if_dirty(),
                )
            } else {
                (None, Vec::new(), None, None)
            };

        // Update chat display if new messages
        if let Some(messages) = chat_messages {
            // Keep user messages (sender == "You") from local state
            let user_messages: Vec<ChatMessageEntry> = self
                .chat_messages
                .iter()
                .filter(|m| m.sender == "You")
                .cloned()
                .collect();

            // Convert SharedDoraState messages to ChatMessageEntry
            let mut assistant_messages: Vec<ChatMessageEntry> = messages
                .into_iter()
                .map(|m| ChatMessageEntry {
                    sender: m.sender,
                    content: m.content,
                    timestamp: m.timestamp,
                    is_streaming: m.is_streaming,
                    session_id: m.session_id,
                })
                .collect();

            // Merge user and assistant messages, sorted by timestamp
            assistant_messages.extend(user_messages);
            assistant_messages.sort_by_key(|m| m.timestamp);

            self.chat_messages = assistant_messages;
            self.update_chat_display(cx);
        }

        // Forward audio chunks to player
        for chunk in audio_chunks {
            if let Some(ref player) = self.audio_player {
                player.write_audio_with_question(
                    &chunk.samples,
                    chunk.participant_id.clone(),
                    chunk.question_id.clone(),
                );
            }
        }

        // Process new log entries from SharedDoraState
        if let Some(entries) = log_entries {
            // Only process entries we haven't seen yet
            for entry in entries.into_iter().skip(self.processed_dora_log_count) {
                let level_str = format!("{:?}", entry.level).to_uppercase();
                let log_line = format!("[{}] [{}] {}", level_str, entry.node_id, entry.message);
                self.add_log(cx, &log_line);
                self.processed_dora_log_count += 1;
            }
        }

        // Process bridge status changes from SharedDoraState
        if let Some(dora_status) = status {
            // Log bridge connections/disconnections by comparing with tracked state
            for bridge in &dora_status.active_bridges {
                if !self.connected_bridges.contains(bridge) {
                    ::log::info!("Bridge connected: {}", bridge);
                    let display_name = Self::format_bridge_name(bridge);
                    self.add_log(
                        cx,
                        &format!(
                            "[INFO] [Bridge] {} connected to dora dataflow",
                            display_name
                        ),
                    );
                    self.connected_bridges.push(bridge.clone());
                }
            }
            // Check for disconnected bridges
            let disconnected: Vec<_> = self
                .connected_bridges
                .iter()
                .filter(|b| !dora_status.active_bridges.contains(b))
                .cloned()
                .collect();
            for bridge in disconnected {
                ::log::info!("Bridge disconnected: {}", bridge);
                let display_name = Self::format_bridge_name(&bridge);
                self.add_log(
                    cx,
                    &format!(
                        "[WARN] [Bridge] {} disconnected from dora dataflow",
                        display_name
                    ),
                );
                self.connected_bridges.retain(|b| b != &bridge);
            }
        }

        // =====================================================
        // Poll event channel for control flow events only
        // =====================================================
        let events = if let Some(ref dora) = self.dora_integration {
            dora.poll_events()
        } else {
            Vec::new()
        };

        for event in events {
            match event {
                DoraEvent::DataflowStarted { dataflow_id } => {
                    ::log::info!("Dataflow started: {}", dataflow_id);
                    self.add_log(
                        cx,
                        &format!("[INFO] [App] Dataflow started: {}", dataflow_id),
                    );
                    self.view
                        .mofa_hero(ids!(left_column.mofa_hero))
                        .set_connection_status(cx, ConnectionStatus::Connected);
                    // Clear tracking state on new dataflow
                    self.connected_bridges.clear();
                    self.processed_dora_log_count = 0;
                    // Flush any prompts that were queued while starting
                    let queued: Vec<String> = self.pending_prompts.drain(..).collect();
                    if let Some(ref dora) = self.dora_integration {
                        for p in queued.iter() {
                            dora.send_prompt(p);
                        }
                    }
                    for p in queued {
                        self.add_log(cx, &format!("[INFO] [App] Sent queued prompt: {}", p));
                    }
                }
                DoraEvent::DataflowStopped => {
                    ::log::info!("Dataflow stopped");
                    self.add_log(cx, "[INFO] [App] Dataflow stopped");
                    self.view
                        .mofa_hero(ids!(left_column.mofa_hero))
                        .set_running(cx, false);
                    self.view
                        .mofa_hero(ids!(left_column.mofa_hero))
                        .set_connection_status(cx, ConnectionStatus::Stopped);
                    // Clear tracking state on stop
                    self.connected_bridges.clear();
                    self.processed_dora_log_count = 0;
                    self.pending_prompts.clear();
                }
                DoraEvent::Error { message } => {
                    ::log::error!("Dora error: {}", message);
                    self.add_log(cx, &format!("[ERROR] [Dora] {}", message));
                    self.view
                        .mofa_hero(ids!(left_column.mofa_hero))
                        .set_connection_status(cx, ConnectionStatus::Failed);
                }
            }
        }

        // Update audio buffer level in audio panel (from audio player)
        // Extract all data first to avoid borrow conflicts with update_buffer_level
        let (buffer_pct, is_playing, active_idx, waveform_data) =
            if let Some(ref player) = self.audio_player {
                let pct = player.buffer_fill_percentage() / 100.0;
                (
                    Some(pct),
                    player.is_playing(),
                    player.current_participant_idx(),
                    player.get_waveform_data(),
                )
            } else {
                (None, false, None, Vec::new())
            };
        if let Some(pct) = buffer_pct {
            self.update_buffer_level(cx, pct);
        }

        {
            // Calculate band levels from waveform data (same as conference-dashboard)
            let band_levels: [f32; 8] = if waveform_data.is_empty() {
                [0.0f32; 8]
            } else {
                let samples = &waveform_data;
                let band_size = samples.len() / 8;
                let mut levels = [0.0f32; 8];
                let peak = samples
                    .iter()
                    .map(|s| s.abs())
                    .fold(0.0f32, |a, b| a.max(b));
                let norm_factor = if peak > 0.01 { 1.0 / peak } else { 1.0 };

                for i in 0..8 {
                    let start = i * band_size;
                    let end = ((i + 1) * band_size).min(samples.len());
                    if end > start {
                        let sum_sq: f32 = samples[start..end].iter().map(|s| s * s).sum();
                        let rms = (sum_sq / (end - start) as f32).sqrt();
                        levels[i] = (rms * norm_factor * 1.5).clamp(0.0, 1.0);
                    }
                }
                levels
            };

            // Update participant panels using direct apply_over (exactly like conference-dashboard)
            let panel_ids: [&[LiveId]; 3] = [
                ids!(
                    left_column
                        .participant_container
                        .participant_bar
                        .student1_panel
                ),
                ids!(
                    left_column
                        .participant_container
                        .participant_bar
                        .student2_panel
                ),
                ids!(
                    left_column
                        .participant_container
                        .participant_bar
                        .tutor_panel
                ),
            ];

            for (i, panel_id) in panel_ids.into_iter().enumerate() {
                let panel = self.view.view(panel_id);
                let is_current_audio_speaker = is_playing && active_idx == Some(i);

                // Calculate level with decay (matches conference-dashboard)
                let new_level = if is_current_audio_speaker && !waveform_data.is_empty() {
                    let samples = &waveform_data;
                    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
                    let rms = (sum_sq / samples.len() as f32).sqrt();
                    (rms * 2.0).clamp(0.0, 1.0) as f64
                } else {
                    self.participant_levels[i] * 0.85
                };
                self.participant_levels[i] = new_level;

                // Update waveform - exactly like conference-dashboard
                let active_val = if is_current_audio_speaker { 1.0 } else { 0.0 };
                panel.view(ids!(waveform)).apply_over(cx, live! {
                    draw_bg: {
                        level: (new_level),
                        active: (active_val),
                        band0: (if is_current_audio_speaker { band_levels[0] as f64 } else { 0.0 }),
                        band1: (if is_current_audio_speaker { band_levels[1] as f64 } else { 0.0 }),
                        band2: (if is_current_audio_speaker { band_levels[2] as f64 } else { 0.0 }),
                        band3: (if is_current_audio_speaker { band_levels[3] as f64 } else { 0.0 }),
                        band4: (if is_current_audio_speaker { band_levels[4] as f64 } else { 0.0 }),
                        band5: (if is_current_audio_speaker { band_levels[5] as f64 } else { 0.0 }),
                        band6: (if is_current_audio_speaker { band_levels[6] as f64 } else { 0.0 }),
                        band7: (if is_current_audio_speaker { band_levels[7] as f64 } else { 0.0 }),
                    }
                });
            }
        }
    }

    // =====================================================
    // Helper Methods
    // =====================================================

    /// Format bridge node ID to a display-friendly name
    /// e.g., "mofa-audio-player" -> "Audio Player"
    ///       "mofa-system-log" -> "System Log"
    ///       "mofa-prompt-input" -> "Prompt Input"
    pub(super) fn format_bridge_name(node_id: &str) -> String {
        // Remove "mofa-" prefix if present
        let name = node_id.strip_prefix("mofa-").unwrap_or(node_id);

        // Convert kebab-case to Title Case
        name.split('-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    // =====================================================
    // MoFA Start/Stop Handlers
    // =====================================================

    /// Handle MoFA start button click
    pub(super) fn handle_mofa_start(&mut self, cx: &mut Cx) {
        ::log::info!("MoFA Start clicked");

        // Clear chat window and system log
        self.chat_messages.clear();
        self.last_chat_count = 0;
        self.update_chat_display(cx);
        self.clear_logs(cx);

        // Initialize dora if not already done
        self.init_dora(cx);

        // Load API keys from preferences
        let env_vars = self.load_api_keys_from_preferences();

        // Log which keys are available
        let has_openai = env_vars.contains_key("OPENAI_API_KEY");
        let has_deepseek = env_vars.contains_key("DEEPSEEK_API_KEY");
        self.add_log(
            cx,
            &format!(
                "[INFO] [App] API Keys: OpenAI={}, DeepSeek={}",
                if has_openai { "✓" } else { "✗" },
                if has_deepseek { "✓" } else { "✗" }
            ),
        );

        // Find the dataflow file relative to current working directory
        let dataflow_path = self.dataflow_path.clone().unwrap_or_else(|| {
            let cwd = std::env::current_dir().unwrap_or_default();
            // First try: apps/mofa-debate/dataflow/voice-chat.yml (when running from workspace root)
            let app_path = cwd
                .join("apps")
                .join("mofa-debate")
                .join("dataflow")
                .join("voice-chat.yml");
            if app_path.exists() {
                return app_path;
            }
            // Fallback: dataflow/voice-chat.yml (when running from app directory)
            cwd.join("dataflow").join("voice-chat.yml")
        });

        if !dataflow_path.exists() {
            self.add_log(
                cx,
                &format!("[ERROR] [App] Dataflow not found: {:?}", dataflow_path),
            );
            self.view
                .mofa_hero(ids!(left_column.mofa_hero))
                .set_connection_status(cx, ConnectionStatus::Failed);
            return;
        }

        self.add_log(
            cx,
            &format!("[INFO] [App] Starting dataflow: {:?}", dataflow_path),
        );

        // Update UI state - show connecting
        self.view
            .mofa_hero(ids!(left_column.mofa_hero))
            .set_running(cx, true);
        self.view
            .mofa_hero(ids!(left_column.mofa_hero))
            .set_connection_status(cx, ConnectionStatus::Connecting);

        // Require a topic from the input box; if empty, fill a placeholder and proceed
        let mut kickoff_topic = self.current_prompt_text().trim().to_string();
        if kickoff_topic.is_empty() {
            kickoff_topic = "辩题待输入，请各方待定".to_string();
            // Write the placeholder back so the user sees it
            self.view
                .text_input(ids!(
                    left_column
                        .prompt_container
                        .prompt_section
                        .prompt_row
                        .prompt_input
                ))
                .set_text(cx, &kickoff_topic);
        }

        let mut kickoff_sent = false;
        let kickoff_prompt = format!(
            "Start a formal debate. Roles: Student1 = PRO, Student2 = CON, Tutor = Judge/Moderator. Debate topic: {}. Keep turns concise and alternate speakers. Tutor manages order and summarizes at the end.",
            kickoff_topic
        );

        if let Some(ref dora) = self.dora_integration {
            if !dora.start_dataflow_with_env(&dataflow_path, env_vars) {
                self.add_log(cx, "[ERROR] [App] Failed to send start command");
                self.view
                    .mofa_hero(ids!(left_column.mofa_hero))
                    .set_connection_status(cx, ConnectionStatus::Failed);
            } else {
                dora.send_prompt(kickoff_prompt.clone());
                kickoff_sent = true;
            }
        }

        if kickoff_sent {
            self.add_log(
                cx,
                &format!("[INFO] [App] Kickoff prompt: {}", kickoff_topic),
            );
        }

        self.dataflow_path = Some(dataflow_path);
    }

    /// Handle MoFA stop button click
    pub(super) fn handle_mofa_stop(&mut self, cx: &mut Cx) {
        ::log::info!("MoFA Stop clicked");

        self.add_log(cx, "[INFO] [App] Force stopping MoFA dataflow...");

        // Show "Stopping" state while stop is in progress
        self.view
            .mofa_hero(ids!(left_column.mofa_hero))
            .set_connection_status(cx, ConnectionStatus::Stopping);

        // Force stop dataflow immediately (0s grace period)
        // The actual status update will come from DoraEvent::DataflowStopped
        if let Some(ref dora) = self.dora_integration {
            dora.force_stop_dataflow();
        }

        // Note: Don't set Stopped here - wait for DoraEvent::DataflowStopped
        // to confirm the dataflow actually stopped
    }

    /// Load API keys from preferences using least-privilege rules.
    ///
    /// Rules:
    /// - provider must be enabled
    /// - provider must be on explicit allowlist
    /// - custom providers are not auto-exported
    pub(super) fn load_api_keys_from_preferences(&self) -> HashMap<String, String> {
        fn env_name_for_provider(provider_id: &str) -> Option<&'static str> {
            match provider_id {
                "openai" => Some("OPENAI_API_KEY"),
                "deepseek" => Some("DEEPSEEK_API_KEY"),
                "alibaba_cloud" => Some("ALIBABA_CLOUD_API_KEY"),
                "nvidia" => Some("NVIDIA_API_KEY"),
                _ => None,
            }
        }

        let mut env_vars = HashMap::new();

        // Load preferences
        let prefs = Preferences::load();

        for provider in prefs.providers.iter().filter(|p| p.enabled) {
            let Some(env_var_name) = env_name_for_provider(provider.id.as_str()) else {
                continue;
            };

            let Some(api_key) = provider.api_key.as_ref() else {
                continue;
            };

            let trimmed = api_key.trim();
            if trimmed.is_empty() {
                continue;
            }

            env_vars.insert(env_var_name.to_string(), trimmed.to_string());

            // Backward-compat alias only if Alibaba key is actively exported
            if provider.id == "alibaba_cloud" {
                env_vars.insert("DASHSCOPE_API_KEY".to_string(), trimmed.to_string());
            }
        }

        env_vars
    }

    /// Read the current prompt input text (used as debate topic)
    fn current_prompt_text(&self) -> String {
        self.view
            .text_input(ids!(
                left_column
                    .prompt_container
                    .prompt_section
                    .prompt_row
                    .prompt_input
            ))
            .text()
    }
}
