//! MoFA ASR Screen - Main screen for ASR testing and transcription
//!
//! Split into sub-modules following mofa-fm's pattern:
//! - `design.rs` - UI layout and styling (live_design! DSL)
//! - `log_panel.rs` - Log display, filtering

pub mod design;
mod log_panel;

use makepad_widgets::*;
use mofa_ui::{MofaHeroWidgetExt, MofaHeroAction, ConnectionStatus, AudioManager};
use mofa_ui::{LedMeterWidgetExt, MicButtonWidgetExt, AecButtonWidgetExt};
use mofa_settings::data::Preferences;
use crate::dora_integration::{AsrEngineId, DoraIntegration, DoraEvent};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use moly_kit::prelude::*;

/// Register live design for this module
pub fn live_design(cx: &mut Cx) {
    design::live_design(cx);
}

/// ASR Model selection
#[derive(Clone, Debug, PartialEq, Default)]
pub enum AsrModelSelection {
    #[default]
    Paraformer,
}

impl AsrModelSelection {
    /// All selections now use the unified dynamic dataflow
    pub fn dataflow_filename(&self) -> &'static str {
        "asr-dynamic.yml"
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AsrModelSelection::Paraformer => "Paraformer",
        }
    }
}

/// ASR Settings
#[derive(Clone, Debug)]
pub struct AsrSettings {
    pub model_selection: AsrModelSelection,
    pub min_audio_duration: f64,
    pub max_audio_duration: f64,
    pub warmup_enabled: bool,
}

impl Default for AsrSettings {
    fn default() -> Self {
        Self {
            model_selection: AsrModelSelection::Paraformer,
            min_audio_duration: 0.1,
            max_audio_duration: 30.0,
            warmup_enabled: true,
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct MoFaASRScreen {
    #[deref]
    view: View,

    #[rust]
    initialized: bool,

    #[rust]
    active_tab: usize,

    #[rust]
    audio_timer: Timer,
    #[rust]
    dora_timer: Timer,

    #[rust]
    settings: AsrSettings,

    // Per-engine chat controllers
    #[rust]
    paraformer_chat_controller: Option<Arc<Mutex<ChatController>>>,
    #[rust]
    paraformer_last_chat_count: usize,
    #[rust]
    qwen3_chat_controller: Option<Arc<Mutex<ChatController>>>,
    #[rust]
    qwen3_last_chat_count: usize,

    // Per-engine active state (ON/OFF toggle)
    #[rust]
    paraformer_active: bool,
    #[rust]
    qwen3_active: bool,

    // Maximized chat panel: None = all visible, Some(engine) = that panel maximized
    #[rust]
    maximized_chat: Option<AsrEngineId>,

    #[rust]
    dora_integration: Option<DoraIntegration>,

    #[rust]
    dataflow_path: Option<PathBuf>,

    // Audio state
    #[rust]
    audio_manager: Option<AudioManager>,
    #[rust]
    mic_muted: bool,
    #[rust]
    input_devices: Vec<String>,
    #[rust]
    output_devices: Vec<String>,

    // Log panel state (following mofa-fm pattern)
    #[rust]
    log_panel_collapsed: bool,
    #[rust]
    log_panel_width: f64,
    #[rust]
    log_level_filter: usize,
    #[rust]
    log_node_filter: usize,
    #[rust]
    log_entries: Vec<String>,
    #[rust]
    log_display_dirty: bool,
    #[rust]
    last_log_update: Option<std::time::Instant>,
    #[rust]
    log_filter_cache: (usize, usize, String),

    // Copy button flash state
    #[rust]
    copy_flash_engine: Option<AsrEngineId>,
    #[rust]
    copy_flash_start: Option<std::time::Instant>,
    #[rust]
    copy_log_flash_start: Option<std::time::Instant>,

    // Splitter drag state
    #[rust]
    splitter_dragging: bool,
    #[rust]
    splitter_drag_start_x: f64,
    #[rust]
    splitter_drag_start_width: f64,
}

impl Widget for MoFaASRScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        if !self.initialized {
            self.initialized = true;
            self.log_panel_collapsed = true; // Start collapsed like mofa-fm
            self.audio_timer = cx.start_interval(0.05);
            self.dora_timer = cx.start_interval(0.1);
            self.init_audio(cx);
            // Collapse log panel on startup
            self.view.view(ids!(log_section)).apply_over(cx, live!{ width: Fit });
            self.view.view(ids!(log_section.log_content_column)).set_visible(cx, false);
            self.view.button(ids!(log_section.toggle_column.toggle_log_btn)).set_text(cx, "<");
            self.view.view(ids!(splitter)).apply_over(cx, live!{ width: 0 });
            // ChatController is initialized in draw_walk (before first draw)
            ::log::info!("MoFaASRScreen initialized");
        }

        if self.audio_timer.is_event(event).is_some() {
            self.update_mic_level(cx);
        }

        if self.dora_timer.is_event(event).is_some() {
            self.poll_dora_events(cx);
            self.poll_rust_logs(cx);
        }

        // Handle splitter drag
        match event.hits(cx, self.view.view(ids!(splitter)).area()) {
            Hit::FingerDown(fd) => {
                if !self.log_panel_collapsed {
                    self.splitter_dragging = true;
                    self.splitter_drag_start_x = fd.abs.x;
                    self.splitter_drag_start_width = self.log_panel_width;
                }
            }
            Hit::FingerMove(fm) => {
                if self.splitter_dragging {
                    let delta = self.splitter_drag_start_x - fm.abs.x;
                    let new_width = (self.splitter_drag_start_width + delta).clamp(200.0, 800.0);
                    self.log_panel_width = new_width;
                    self.view.view(ids!(log_section)).apply_over(cx, live!{ width: (new_width) });
                    self.view.redraw(cx);
                }
            }
            Hit::FingerUp(_) => {
                self.splitter_dragging = false;
            }
            _ => {}
        }

        // Handle copy flash animations
        if let Event::NextFrame(_) = event {
            let mut needs_redraw = false;
            if let Some(start) = self.copy_flash_start {
                let elapsed = start.elapsed().as_secs_f64();
                if elapsed < 0.8 {
                    let t = if elapsed < 0.3 { 1.0 } else { 1.0 - ((elapsed - 0.3) / 0.5).min(1.0) };
                    if let Some(engine) = self.copy_flash_engine {
                        let btn_id = match engine {
                            AsrEngineId::Paraformer => ids!(left_column.transcription_tab_content.chat_container.paraformer_section.paraformer_header.paraformer_copy_btn),
                            AsrEngineId::Qwen3Asr => ids!(left_column.transcription_tab_content.chat_container.qwen3_section.qwen3_header.qwen3_copy_btn),
                        };
                        self.view.view(btn_id).apply_over(cx, live!{ draw_bg: { copied: (t) } });
                    }
                    cx.new_next_frame();
                    needs_redraw = true;
                } else {
                    if let Some(engine) = self.copy_flash_engine.take() {
                        let btn_id = match engine {
                            AsrEngineId::Paraformer => ids!(left_column.transcription_tab_content.chat_container.paraformer_section.paraformer_header.paraformer_copy_btn),
                            AsrEngineId::Qwen3Asr => ids!(left_column.transcription_tab_content.chat_container.qwen3_section.qwen3_header.qwen3_copy_btn),
                        };
                        self.view.view(btn_id).apply_over(cx, live!{ draw_bg: { copied: 0.0 } });
                        needs_redraw = true;
                    }
                    self.copy_flash_start = None;
                }
            }
            if let Some(start) = self.copy_log_flash_start {
                let elapsed = start.elapsed().as_secs_f64();
                if elapsed < 0.8 {
                    let t = if elapsed < 0.3 { 1.0 } else { 1.0 - ((elapsed - 0.3) / 0.5).min(1.0) };
                    self.view.view(ids!(log_section.log_content_column.log_header.log_filter_row.copy_log_btn))
                        .apply_over(cx, live!{ draw_bg: { copied: (t) } });
                    cx.new_next_frame();
                    needs_redraw = true;
                } else {
                    self.view.view(ids!(log_section.log_content_column.log_header.log_filter_row.copy_log_btn))
                        .apply_over(cx, live!{ draw_bg: { copied: 0.0 } });
                    self.copy_log_flash_start = None;
                    needs_redraw = true;
                }
            }
            if needs_redraw {
                self.view.redraw(cx);
            }
        }

        if let Event::Actions(actions) = event {
            self.handle_actions(cx, &actions);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Ensure per-engine ChatControllers are set before any draw
        if self.paraformer_chat_controller.is_none() {
            let controller = ChatController::new_arc();
            {
                let mut guard = controller.lock().expect("ChatController mutex poisoned");
                guard.dangerous_state_mut().bots.push(Bot {
                    id: BotId::new("asr"),
                    name: "Paraformer".to_string(),
                    avatar: EntityAvatar::Text("P".to_string()),
                    capabilities: BotCapabilities::new(),
                });
            }
            self.paraformer_chat_controller = Some(controller.clone());
            self.view.messages(ids!(paraformer_messages)).write().chat_controller = Some(controller);
        }
        if self.qwen3_chat_controller.is_none() {
            let controller = ChatController::new_arc();
            {
                let mut guard = controller.lock().expect("ChatController mutex poisoned");
                guard.dangerous_state_mut().bots.push(Bot {
                    id: BotId::new("asr"),
                    name: "Qwen3-ASR".to_string(),
                    avatar: EntityAvatar::Text("Q".to_string()),
                    capabilities: BotCapabilities::new(),
                });
            }
            self.qwen3_chat_controller = Some(controller.clone());
            self.view.messages(ids!(qwen3_messages)).write().chat_controller = Some(controller);
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

impl MoFaASRScreen {
    // ========================================================================
    // Action handling
    // ========================================================================

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // Handle MofaHero start/stop — scoped to this screen's hero widget only
        let hero_uid = self.view.mofa_hero(ids!(left_column.mofa_hero)).widget_uid();
        match actions.find_widget_action_cast::<MofaHeroAction>(hero_uid) {
            MofaHeroAction::StartClicked => {
                ::log::info!("ASR Start clicked");
                self.handle_start(cx);
            }
            MofaHeroAction::StopClicked => {
                ::log::info!("ASR Stop clicked");
                self.handle_stop(cx);
            }
            MofaHeroAction::None => {}
        }

        // Handle tab switching
        if self.view.view(ids!(left_column.tab_bar.transcription_tab)).finger_up(actions).is_some() {
            self.switch_tab(cx, 0);
        }
        if self.view.view(ids!(left_column.tab_bar.settings_tab)).finger_up(actions).is_some() {
            self.switch_tab(cx, 1);
        }

        // Handle mic button
        let mic_btn = self.view.mic_button(ids!(left_column.transcription_tab_content.audio_container.audio_controls_row.mic_container.mic_group.mic_mute_btn));
        if mic_btn.clicked(actions) {
            self.mic_muted = !self.mic_muted;
            mic_btn.set_muted(cx, self.mic_muted);
            if self.mic_muted {
                self.view.led_meter(ids!(left_column.transcription_tab_content.audio_container.audio_controls_row.mic_container.mic_group.mic_level_meter)).set_level(cx, 0.0);
            }
        }

        // Handle AEC button
        let aec_btn = self.view.aec_button(ids!(left_column.transcription_tab_content.audio_container.audio_controls_row.aec_container.aec_group.aec_toggle_btn));
        if aec_btn.clicked(actions) {
            let enabled = aec_btn.is_enabled();
            if let Some(ref dora) = self.dora_integration {
                dora.set_aec_enabled(enabled);
            }
        }

        // Handle Paraformer toggle
        if self.view.button(ids!(left_column.transcription_tab_content.chat_container.paraformer_section.paraformer_header.paraformer_toggle_btn)).clicked(actions) {
            self.toggle_engine(cx, AsrEngineId::Paraformer);
        }

        // Handle Qwen3-ASR toggle
        if self.view.button(ids!(left_column.transcription_tab_content.chat_container.qwen3_section.qwen3_header.qwen3_toggle_btn)).clicked(actions) {
            self.toggle_engine(cx, AsrEngineId::Qwen3Asr);
        }

        // Handle maximize buttons (View with Hit events, not Button)
        let sv_max = self.view.view(ids!(left_column.transcription_tab_content.chat_container.paraformer_section.paraformer_header.paraformer_maximize_btn));
        if sv_max.finger_up(actions).is_some() {
            self.toggle_maximize_chat(cx, AsrEngineId::Paraformer);
        }
        let q3_max = self.view.view(ids!(left_column.transcription_tab_content.chat_container.qwen3_section.qwen3_header.qwen3_maximize_btn));
        if q3_max.finger_up(actions).is_some() {
            self.toggle_maximize_chat(cx, AsrEngineId::Qwen3Asr);
        }

        // Handle log toggle button
        if self.view.button(ids!(log_section.toggle_column.toggle_log_btn)).clicked(actions) {
            self.toggle_log_panel(cx);
        }

        // Handle log copy button
        if self.view.view(ids!(log_section.log_content_column.log_header.log_filter_row.copy_log_btn)).finger_up(actions).is_some() {
            self.copy_logs_to_clipboard(cx);
            self.copy_log_flash_start = Some(std::time::Instant::now());
            cx.new_next_frame();
        }

        // Handle chat copy buttons
        if self.view.view(ids!(left_column.transcription_tab_content.chat_container.paraformer_section.paraformer_header.paraformer_copy_btn)).finger_up(actions).is_some() {
            self.copy_chat_to_clipboard(cx, AsrEngineId::Paraformer);
        }
        if self.view.view(ids!(left_column.transcription_tab_content.chat_container.qwen3_section.qwen3_header.qwen3_copy_btn)).finger_up(actions).is_some() {
            self.copy_chat_to_clipboard(cx, AsrEngineId::Qwen3Asr);
        }

        // Handle log level filter
        if let Some(selected) = self.view.drop_down(ids!(log_section.log_content_column.log_header.log_filter_row.level_filter)).selected(actions) {
            self.log_level_filter = selected;
            self.update_log_display(cx);
        }

        // Handle log node filter
        if let Some(selected) = self.view.drop_down(ids!(log_section.log_content_column.log_header.log_filter_row.node_filter)).selected(actions) {
            self.log_node_filter = selected;
            self.update_log_display(cx);
        }

        // Handle input device dropdown
        if let Some(item) = self.view.drop_down(ids!(left_column.transcription_tab_content.audio_container.device_container.device_selectors.input_device_group.input_device_dropdown)).selected(actions) {
            if item < self.input_devices.len() {
                let device_name = self.input_devices[item].clone();
                self.select_input_device(cx, &device_name);
            }
        }

        // Handle output device dropdown
        if let Some(item) = self.view.drop_down(ids!(left_column.transcription_tab_content.audio_container.device_container.device_selectors.output_device_group.output_device_dropdown)).selected(actions) {
            if item < self.output_devices.len() {
                let device_name = self.output_devices[item].clone();
                self.select_output_device(&device_name);
            }
        }
    }

    // ========================================================================
    // Chat display (moly-kit Messages widget)
    // ========================================================================

    /// Sync chat messages from SharedDoraState to a per-engine Messages widget
    fn sync_engine_chat(&mut self, cx: &mut Cx, messages: &[mofa_dora_bridge::data::ChatMessage], engine: AsrEngineId) {
        let controller = match engine {
            AsrEngineId::Paraformer => self.paraformer_chat_controller.clone(),
            AsrEngineId::Qwen3Asr => self.qwen3_chat_controller.clone(),
        };
        let controller = match controller {
            Some(c) => c,
            None => return,
        };

        let count = {
            let mut guard = controller.lock().expect("ChatController mutex poisoned");
            let state = guard.dangerous_state_mut();
            state.messages.clear();
            for msg in messages {
                let entity = match msg.role {
                    mofa_dora_bridge::data::MessageRole::User => EntityId::User,
                    _ => EntityId::Bot(BotId::new("asr")),
                };
                state.messages.push(Message {
                    from: entity,
                    content: MessageContent {
                        text: msg.content.clone(),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            }
            state.messages.len()
        };

        let (last_count, widget_id) = match engine {
            AsrEngineId::Paraformer => (&mut self.paraformer_last_chat_count, ids!(paraformer_messages)),
            AsrEngineId::Qwen3Asr => (&mut self.qwen3_last_chat_count, ids!(qwen3_messages)),
        };

        if count > *last_count {
            *last_count = count;
            let mut messages_ref = self.view.messages(widget_id);
            messages_ref.write().instant_scroll_to_bottom(cx);
        }

        self.view.redraw(cx);
    }


    // ========================================================================
    // Dora integration
    // ========================================================================

    fn handle_start(&mut self, cx: &mut Cx) {
        // Clear per-engine chat controllers
        if let Some(ref controller) = self.paraformer_chat_controller {
            controller.lock().expect("ChatController mutex poisoned").dangerous_state_mut().messages.clear();
        }
        if let Some(ref controller) = self.qwen3_chat_controller {
            controller.lock().expect("ChatController mutex poisoned").dangerous_state_mut().messages.clear();
        }
        self.paraformer_last_chat_count = 0;
        self.qwen3_last_chat_count = 0;

        self.init_dora(cx);

        let mut env_vars = self.load_api_keys_from_preferences();
        env_vars.insert("MIN_AUDIO_DURATION".to_string(), self.settings.min_audio_duration.to_string());
        env_vars.insert("MAX_AUDIO_DURATION".to_string(), self.settings.max_audio_duration.to_string());

        if self.settings.warmup_enabled {
            env_vars.insert("ASR_MLX_WARMUP".to_string(), "1".to_string());
        }

        let dataflow_path = self.dataflow_path.clone().unwrap_or_else(|| {
            let cwd = std::env::current_dir().unwrap_or_default();
            let filename = self.settings.model_selection.dataflow_filename();
            let app_path = cwd.join("apps").join("mofa-asr").join("dataflow").join(filename);
            if app_path.exists() { return app_path; }
            let local_path = cwd.join("dataflow").join(filename);
            if local_path.exists() { return local_path; }
            ::log::warn!("Dataflow file not found, using default path: {:?}", app_path);
            app_path
        });

        ::log::info!("Starting ASR dataflow: {:?}", dataflow_path);
        self.add_log(cx, &format!("[INFO] [App] Starting dataflow: {:?}", dataflow_path));

        if let Some(ref dora) = self.dora_integration {
            dora.start_dataflow_with_env(dataflow_path, env_vars);
        }

        self.view.mofa_hero(ids!(left_column.mofa_hero)).set_running(cx, true);
        self.view.mofa_hero(ids!(left_column.mofa_hero)).set_connection_status(cx, ConnectionStatus::Connecting);
        self.view.redraw(cx);
    }

    fn handle_stop(&mut self, cx: &mut Cx) {
        self.view.mofa_hero(ids!(left_column.mofa_hero)).set_running(cx, false);
        self.view.mofa_hero(ids!(left_column.mofa_hero)).set_connection_status(cx, ConnectionStatus::Stopping);
        self.add_log(cx, "[INFO] [App] Stopping dataflow");
        if let Some(ref dora) = self.dora_integration {
            dora.force_stop_dataflow();
        }
        // Reset toggle states
        self.paraformer_active = false;
        self.qwen3_active = false;
        self.update_toggle_ui(cx);
    }

    fn toggle_engine(&mut self, cx: &mut Cx, engine: AsrEngineId) {
        // Only allow toggle when dataflow is running
        if !self.dora_integration.as_ref().map_or(false, |d| d.is_running()) {
            ::log::warn!("Cannot toggle ASR engine: dataflow not running");
            return;
        }

        let is_active = match engine {
            AsrEngineId::Paraformer => &mut self.paraformer_active,
            AsrEngineId::Qwen3Asr => &mut self.qwen3_active,
        };

        *is_active = !*is_active;
        let now_active = *is_active;

        if let Some(ref dora) = self.dora_integration {
            if now_active {
                let env_vars = self.load_api_keys_from_preferences();
                dora.connect_asr_engine(engine, env_vars);
                self.add_log(cx, &format!("[INFO] [App] Starting {:?} engine", engine));
            } else {
                dora.disconnect_asr_engine(engine);
                self.add_log(cx, &format!("[INFO] [App] Stopping {:?} engine", engine));
            }
        }

        self.update_toggle_ui(cx);
    }

    fn update_toggle_ui(&mut self, cx: &mut Cx) {
        // Paraformer
        let (p_text, p_status) = if self.paraformer_active {
            ("OFF", "ON")
        } else {
            ("ON", "OFF")
        };
        self.view.button(ids!(left_column.transcription_tab_content.chat_container.paraformer_section.paraformer_header.paraformer_toggle_btn))
            .set_text(cx, p_text);
        self.view.label(ids!(left_column.transcription_tab_content.chat_container.paraformer_section.paraformer_header.paraformer_status))
            .set_text(cx, p_status);

        // Qwen3-ASR
        let (q_text, q_status) = if self.qwen3_active {
            ("OFF", "ON")
        } else {
            ("ON", "OFF")
        };
        self.view.button(ids!(left_column.transcription_tab_content.chat_container.qwen3_section.qwen3_header.qwen3_toggle_btn))
            .set_text(cx, q_text);
        self.view.label(ids!(left_column.transcription_tab_content.chat_container.qwen3_section.qwen3_header.qwen3_status))
            .set_text(cx, q_status);

        self.view.redraw(cx);
    }

    fn toggle_maximize_chat(&mut self, cx: &mut Cx, engine: AsrEngineId) {
        if self.maximized_chat == Some(engine) {
            // Restore: show all panels
            self.maximized_chat = None;
            self.view.view(ids!(left_column.transcription_tab_content.chat_container.paraformer_section)).set_visible(cx, true);
            self.view.view(ids!(left_column.transcription_tab_content.chat_container.qwen3_section)).set_visible(cx, true);
            // Reset maximize icons to expand state
            self.view.view(ids!(left_column.transcription_tab_content.chat_container.paraformer_section.paraformer_header.paraformer_maximize_btn))
                .apply_over(cx, live!{ draw_bg: { maximized: 0.0 } });
            self.view.view(ids!(left_column.transcription_tab_content.chat_container.qwen3_section.qwen3_header.qwen3_maximize_btn))
                .apply_over(cx, live!{ draw_bg: { maximized: 0.0 } });
        } else {
            // Maximize: hide other panels, show only selected
            self.maximized_chat = Some(engine);
            self.view.view(ids!(left_column.transcription_tab_content.chat_container.paraformer_section))
                .set_visible(cx, engine == AsrEngineId::Paraformer);
            self.view.view(ids!(left_column.transcription_tab_content.chat_container.qwen3_section))
                .set_visible(cx, engine == AsrEngineId::Qwen3Asr);
            // Update maximize icon to collapse state
            let btn_id = match engine {
                AsrEngineId::Paraformer => ids!(left_column.transcription_tab_content.chat_container.paraformer_section.paraformer_header.paraformer_maximize_btn),
                AsrEngineId::Qwen3Asr => ids!(left_column.transcription_tab_content.chat_container.qwen3_section.qwen3_header.qwen3_maximize_btn),
            };
            self.view.view(btn_id).apply_over(cx, live!{ draw_bg: { maximized: 1.0 } });
        }
        self.view.redraw(cx);
    }

    fn init_dora(&mut self, cx: &mut Cx) {
        if self.dora_integration.is_some() {
            return;
        }
        ::log::info!("Initializing ASR Dora integration");
        let integration = DoraIntegration::new();
        self.dora_integration = Some(integration);
        self.dora_timer = cx.start_interval(0.1);
    }

    fn poll_dora_events(&mut self, cx: &mut Cx) {
        let events: Vec<DoraEvent> = if let Some(ref dora) = self.dora_integration {
            dora.poll_events()
        } else {
            return;
        };

        for event in events {
            match event {
                DoraEvent::DataflowStarted { dataflow_id } => {
                    ::log::info!("ASR dataflow started: {}", dataflow_id);
                    self.add_log(cx, &format!("[INFO] [App] Dataflow started: {}", dataflow_id));
                    self.view.mofa_hero(ids!(left_column.mofa_hero)).set_connection_status(cx, ConnectionStatus::Connected);

                    // Stop CPAL mic monitoring to avoid dual capture with AEC
                    if let Some(ref mut manager) = self.audio_manager {
                        manager.stop_mic_monitoring();
                    }

                    // Enable AEC and start recording on dataflow start (like mofa-fm)
                    self.view.aec_button(ids!(left_column.transcription_tab_content.audio_container.audio_controls_row.aec_container.aec_group.aec_toggle_btn))
                        .set_enabled(cx, true);
                    if let Some(ref dora) = self.dora_integration {
                        dora.set_aec_enabled(true);
                        dora.start_recording();
                    }

                    self.view.redraw(cx);
                }
                DoraEvent::DataflowStopped => {
                    ::log::info!("ASR dataflow stopped");
                    self.add_log(cx, "[INFO] [App] Dataflow stopped");
                    self.view.mofa_hero(ids!(left_column.mofa_hero)).set_running(cx, false);
                    self.view.mofa_hero(ids!(left_column.mofa_hero)).set_connection_status(cx, ConnectionStatus::Stopped);
                    // Reset toggle states
                    self.paraformer_active = false;
                    self.update_toggle_ui(cx);
                    // Restart CPAL mic monitoring for level meter
                    if let Some(ref mut manager) = self.audio_manager {
                        let _ = manager.start_mic_monitoring(None);
                    }
                    self.view.redraw(cx);
                }
                DoraEvent::Error { message } => {
                    ::log::error!("ASR dora error: {}", message);
                    self.add_log(cx, &format!("[ERROR] [App] {}", message));
                    self.view.mofa_hero(ids!(left_column.mofa_hero)).set_running(cx, false);
                    self.view.mofa_hero(ids!(left_column.mofa_hero)).set_connection_status(cx, ConnectionStatus::Failed);
                    self.view.redraw(cx);
                }
            }
        }

        // Poll SharedDoraState for per-engine transcription data and logs
        let shared_state = self.dora_integration.as_ref().map(|d| d.shared_dora_state().clone());
        if let Some(state) = shared_state {
            // Sync per-engine chats
            if let Some(messages) = state.chat_paraformer.read_if_dirty() {
                self.sync_engine_chat(cx, &messages, AsrEngineId::Paraformer);
            }
            if let Some(messages) = state.chat_qwen3.read_if_dirty() {
                self.sync_engine_chat(cx, &messages, AsrEngineId::Qwen3Asr);
            }

            // Poll SharedDoraState for system logs
            if let Some(logs) = state.logs.read_if_dirty() {
                for log_entry in &logs {
                    let level_str = match log_entry.level {
                        mofa_dora_bridge::data::LogLevel::Debug => "DEBUG",
                        mofa_dora_bridge::data::LogLevel::Info => "INFO",
                        mofa_dora_bridge::data::LogLevel::Warning => "WARN",
                        mofa_dora_bridge::data::LogLevel::Error => "ERROR",
                    };
                    let formatted = format!("[{}] [{}] {}", level_str, log_entry.node_id, log_entry.message);
                    self.add_log(cx, &formatted);
                }
            }
        }
    }

    // ========================================================================
    // Audio controls
    // ========================================================================

    fn init_audio(&mut self, cx: &mut Cx) {
        let mut manager = AudioManager::new();

        let input_devices = manager.get_input_devices();
        let output_devices = manager.get_output_devices();

        let input_labels: Vec<String> = input_devices.iter().map(|d| {
            if d.is_default { format!("{} (Default)", d.name) } else { d.name.clone() }
        }).collect();
        let output_labels: Vec<String> = output_devices.iter().map(|d| {
            if d.is_default { format!("{} (Default)", d.name) } else { d.name.clone() }
        }).collect();

        self.input_devices = input_devices.iter().map(|d| d.name.clone()).collect();
        self.output_devices = output_devices.iter().map(|d| d.name.clone()).collect();

        self.view.drop_down(ids!(left_column.transcription_tab_content.audio_container.device_container.device_selectors.input_device_group.input_device_dropdown))
            .set_labels(cx, input_labels);
        self.view.drop_down(ids!(left_column.transcription_tab_content.audio_container.device_container.device_selectors.output_device_group.output_device_dropdown))
            .set_labels(cx, output_labels);

        if let Some(default_idx) = input_devices.iter().position(|d| d.is_default) {
            self.view.drop_down(ids!(left_column.transcription_tab_content.audio_container.device_container.device_selectors.input_device_group.input_device_dropdown))
                .set_selected_item(cx, default_idx);
        }
        if let Some(default_idx) = output_devices.iter().position(|d| d.is_default) {
            self.view.drop_down(ids!(left_column.transcription_tab_content.audio_container.device_container.device_selectors.output_device_group.output_device_dropdown))
                .set_selected_item(cx, default_idx);
        }

        if let Err(e) = manager.start_mic_monitoring(None) {
            ::log::warn!("Failed to start mic monitoring: {}", e);
        }

        self.audio_manager = Some(manager);
        self.view.redraw(cx);
    }

    fn update_mic_level(&mut self, cx: &mut Cx) {
        if self.mic_muted {
            return;
        }

        let level = if let Some(ref manager) = self.audio_manager {
            manager.get_mic_level()
        } else {
            0.0
        };

        self.view.led_meter(ids!(left_column.transcription_tab_content.audio_container.audio_controls_row.mic_container.mic_group.mic_level_meter)).set_level(cx, level);
    }

    fn select_input_device(&mut self, _cx: &mut Cx, device_name: &str) {
        ::log::info!("Selecting input device: {}", device_name);
        if let Some(ref mut manager) = self.audio_manager {
            if let Err(e) = manager.start_mic_monitoring(Some(device_name)) {
                ::log::error!("Failed to switch input device: {}", e);
            }
        }
    }

    fn select_output_device(&mut self, device_name: &str) {
        ::log::info!("Selecting output device: {}", device_name);
    }

    // ========================================================================
    // Tab switching
    // ========================================================================

    fn switch_tab(&mut self, cx: &mut Cx, tab: usize) {
        if self.active_tab == tab {
            return;
        }
        self.active_tab = tab;

        let transcription_selected = if tab == 0 { 1.0 } else { 0.0 };
        let settings_selected = if tab == 1 { 1.0 } else { 0.0 };

        self.view.view(ids!(left_column.tab_bar.transcription_tab)).apply_over(cx, live!{
            draw_bg: { selected: (transcription_selected) }
        });
        self.view.label(ids!(left_column.tab_bar.transcription_tab.tab_label)).apply_over(cx, live!{
            draw_text: { selected: (transcription_selected) }
        });

        self.view.view(ids!(left_column.tab_bar.settings_tab)).apply_over(cx, live!{
            draw_bg: { selected: (settings_selected) }
        });
        self.view.label(ids!(left_column.tab_bar.settings_tab.tab_label)).apply_over(cx, live!{
            draw_text: { selected: (settings_selected) }
        });

        self.view.view(ids!(left_column.transcription_tab_content)).set_visible(cx, tab == 0);
        self.view.view(ids!(left_column.settings_tab_content)).set_visible(cx, tab == 1);

        self.view.redraw(cx);
    }

    // ========================================================================
    // Dark mode
    // ========================================================================

    pub fn update_dark_mode(&mut self, cx: &mut Cx, dm: f64) {
        self.view.apply_over(cx, live!{
            draw_bg: { dark_mode: (dm) }
        });

        self.view.view(ids!(left_column.tab_bar)).apply_over(cx, live!{
            draw_bg: { dark_mode: (dm) }
        });
        self.view.view(ids!(left_column.tab_bar.transcription_tab)).apply_over(cx, live!{
            draw_bg: { dark_mode: (dm) }
        });
        self.view.label(ids!(left_column.tab_bar.transcription_tab.tab_label)).apply_over(cx, live!{
            draw_text: { dark_mode: (dm) }
        });
        self.view.view(ids!(left_column.tab_bar.settings_tab)).apply_over(cx, live!{
            draw_bg: { dark_mode: (dm) }
        });
        self.view.label(ids!(left_column.tab_bar.settings_tab.tab_label)).apply_over(cx, live!{
            draw_text: { dark_mode: (dm) }
        });

        self.view.redraw(cx);
    }

    /// Load API keys from preferences
    pub(super) fn load_api_keys_from_preferences(&self) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();

        let prefs = Preferences::load();

        for provider in &prefs.providers {
            if let Some(ref api_key) = provider.api_key {
                if !api_key.is_empty() {
                    let env_var_name = match provider.id.as_str() {
                        "openai" => "OPENAI_API_KEY".to_string(),
                        "deepseek" => "DEEPSEEK_API_KEY".to_string(),
                        "alibaba_cloud" => "ALIBABA_CLOUD_API_KEY".to_string(),
                        "nvidia" => "NVIDIA_API_KEY".to_string(),
                        id => format!("{}_API_KEY", id.to_uppercase().replace('-', "_")),
                    };
                    env_vars.insert(env_var_name, api_key.clone());
                }
            }
        }

        if let Some(provider) = prefs.get_provider("alibaba_cloud") {
            if let Some(ref api_key) = provider.api_key {
                if !api_key.is_empty() {
                    env_vars.insert("DASHSCOPE_API_KEY".to_string(), api_key.clone());
                }
            }
        }

        env_vars
    }
}

/// Extension methods for MoFaASRScreen widget reference
impl MoFaASRScreenRef {
    pub fn stop_timers(&self, cx: &mut Cx) {
        if let Some(inner) = self.borrow_mut() {
            cx.stop_timer(inner.audio_timer);
            cx.stop_timer(inner.dora_timer);
        }
    }

    pub fn start_timers(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.audio_timer = cx.start_interval(0.05);
            inner.dora_timer = cx.start_interval(0.1);
        }
    }

    pub fn update_dark_mode(&self, cx: &mut Cx, dm: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.update_dark_mode(cx, dm);
        }
    }
}
