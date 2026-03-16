//! MoFA FM Screen - Main screen for AI-powered audio streaming
//!
//! This module is split into sub-modules for better organization:
//! - `design.rs` - UI layout and styling (live_design! DSL)
//! - `audio_controls.rs` - Audio device selection, mic monitoring
//! - `chat_panel.rs` - Chat display, prompt input
//! - `log_panel.rs` - Log display, filtering
//! - `dora_handlers.rs` - Dora event handling, dataflow control

mod audio_controls;
mod chat_panel;
pub mod design;  // Public for Makepad live_design path resolution
mod dora_handlers;
mod log_panel;
mod role_config;

use role_config::{RoleConfig, get_role_config_path, get_yaml_path, read_yaml_voice, VOICE_OPTIONS};

use makepad_widgets::*;
use mofa_ui::{MofaHeroWidgetExt, MofaHeroAction, AudioManager};
use mofa_ui::log_bridge;
use crate::dora_integration::{DoraIntegration, DoraCommand};
use mofa_widgets::participant_panel::ParticipantPanelWidgetExt;
use mofa_widgets::{StateChangeListener, TimerControl};
use mofa_ui::{LedMeterWidgetExt, MicButtonWidgetExt, AecButtonWidgetExt};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Data preloaded in background thread
#[derive(Default)]
struct PreloadedData {
    context_content: Option<String>,
    student1_config: Option<RoleConfig>,
    student2_config: Option<RoleConfig>,
    tutor_config: Option<RoleConfig>,
    loading_complete: bool,
}

/// Register live design for this module
pub fn live_design(cx: &mut Cx) {
    design::live_design(cx);
}

/// Chat message entry for display
#[derive(Clone, Debug)]
pub struct ChatMessageEntry {
    pub sender: String,
    pub content: String,
    pub timestamp: u64,
    pub is_streaming: bool,
    pub session_id: Option<String>,
}

impl ChatMessageEntry {
    pub fn new(sender: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            sender: sender.into(),
            content: content.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            is_streaming: false,
            session_id: None,
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct MoFaFMScreen {
    #[deref]
    view: View,
    #[rust]
    log_panel_collapsed: bool,
    #[rust]
    log_panel_width: f64,
    #[rust]
    splitter_dragging: bool,
    #[rust]
    audio_manager: Option<AudioManager>,
    #[rust]
    audio_timer: Timer,
    #[rust]
    audio_initialized: bool,
    #[rust]
    input_devices: Vec<String>,
    #[rust]
    output_devices: Vec<String>,
    #[rust]
    log_level_filter: usize,  // 0=ALL, 1=DEBUG, 2=INFO, 3=WARN, 4=ERROR
    #[rust]
    log_node_filter: usize,   // 0=ALL, 1=ASR, 2=TTS, 3=LLM, 4=Bridge, 5=Monitor, 6=App
    #[rust]
    log_entries: VecDeque<String>,
    #[rust]
    log_display_dirty: bool,   // Flag to track if log display needs update
    #[rust]
    last_log_update: Option<std::time::Instant>,  // Timestamp of last log display update
    #[rust]
    log_filter_cache: (usize, usize, String),  // Cache: (level, node, search) to detect filter changes

    // AEC toggle state
    #[rust]
    aec_enabled: bool,
    // Note: AEC blink animation is now shader-driven (self.time), no timer needed

    // Mic mute state
    #[rust]
    mic_muted: bool,

    // Dora integration
    #[rust]
    dora_integration: Option<DoraIntegration>,
    #[rust]
    dataflow_path: Option<PathBuf>,
    #[rust]
    dora_timer: Timer,
    // NextFrame-based animation for copy buttons (smooth fade instead of timer reset)
    #[rust]
    copy_chat_flash_active: bool,
    #[rust]
    copy_chat_flash_start: f64,  // Absolute start time
    #[rust]
    copy_log_flash_active: bool,
    #[rust]
    copy_log_flash_start: f64,   // Absolute start time
    #[rust]
    chat_messages: Vec<ChatMessageEntry>,
    #[rust]
    last_chat_count: usize,

    // Audio playback
    #[rust]
    audio_player: Option<std::sync::Arc<crate::audio_player::AudioPlayer>>,
    // Participant audio levels for decay animation (matches conference-dashboard)
    #[rust]
    participant_levels: [f64; 3],  // 0=student1, 1=student2, 2=tutor

    // SharedDoraState tracking (for detecting changes)
    #[rust]
    connected_bridges: Vec<String>,
    #[rust]
    processed_dora_log_count: usize,

    // Tab state: 0 = Running, 1 = Settings
    #[rust]
    active_tab: usize,

    // Context content loaded from study-context.md
    #[rust]
    context_content: String,

    // Role configurations
    #[rust]
    student1_config: RoleConfig,
    #[rust]
    student2_config: RoleConfig,
    #[rust]
    tutor_config: RoleConfig,
    // Background preloading - configs loaded into memory at startup
    #[rust]
    configs_preloaded: bool,
    // Async preloaded data from background thread
    #[rust]
    async_preload: Option<Arc<Mutex<PreloadedData>>>,
    // Lazy UI population flags - track which TextInputs have been populated
    #[rust]
    context_ui_populated: bool,
    #[rust]
    student1_ui_populated: bool,
    #[rust]
    student2_ui_populated: bool,
    #[rust]
    tutor_ui_populated: bool,

    // Editor maximize state: None = normal, Some(id) = maximized editor
    #[rust]
    maximized_editor: Option<String>,
    // Maximize animation state
    #[rust]
    maximize_animation_active: bool,
    #[rust]
    maximize_animation_start: f64,
    #[rust]
    maximize_animation_target: Option<String>,
    #[rust]
    maximize_animation_expanding: bool,  // true = maximizing, false = restoring
    #[rust]
    saved_scroll_pos: f64,  // Save scroll position before maximize
    // Shader pre-compilation: hide Settings tab after first draw
    #[rust]
    shader_precompile_frame: usize,

    // Save button animation state
    #[rust]
    save_animation_timer: Timer,
    #[rust]
    save_animation_role: Option<String>,
}

impl Widget for MoFaFMScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let actions = cx.capture_actions(|cx| self.view.handle_event(cx, event, scope));

        // Initialize audio and log bridge on first event
        if !self.audio_initialized {
            log_bridge::init();
            self.init_audio(cx);
            self.audio_initialized = true;
            // Start async preloading in background thread
            self.start_async_preload();
            // Collapse log panel by default
            self.log_panel_collapsed = true;
            self.view.view(ids!(log_section)).apply_over(cx, live!{ width: Fit });
            self.view.view(ids!(log_section.log_content_column)).set_visible(cx, false);
            self.view.button(ids!(log_section.toggle_column.toggle_log_btn)).set_text(cx, "<");
            self.view.view(ids!(splitter)).apply_over(cx, live!{ width: 0 });
        }

        // Check if async preload completed - store data and trigger UI population
        if !self.configs_preloaded {
            let mut preload_ready = false;
            if let Some(ref preload) = self.async_preload {
                if let Ok(mut data) = preload.try_lock() {
                    if data.loading_complete {
                        if let Some(content) = data.context_content.take() {
                            self.context_content = content;
                        }
                        if let Some(config) = data.student1_config.take() {
                            self.student1_config = config;
                        }
                        if let Some(config) = data.student2_config.take() {
                            self.student2_config = config;
                        }
                        if let Some(config) = data.tutor_config.take() {
                            self.tutor_config = config;
                        }
                        preload_ready = true;
                    }
                }
            }
            if preload_ready {
                self.configs_preloaded = true;
                // Trigger UI population on next frame
                self.shader_precompile_frame = 1;
                cx.new_next_frame();
                ::log::info!("Async preload complete - triggering UI population");
            }
        }

        // Debug: Log every event type once
        static LOGGED_TIMER: AtomicBool = AtomicBool::new(false);
        if !LOGGED_TIMER.load(Ordering::Relaxed) {
            if let Event::Timer(_) = event {
                ::log::info!("Received Timer event");
                LOGGED_TIMER.store(true, Ordering::Relaxed);
            }
        }

        // Handle audio timer for mic level updates, log polling, and buffer status
        if self.audio_timer.is_event(event).is_some() {
            // Debug: log timer firing
            static TIMER_COUNT: AtomicU32 = AtomicU32::new(0);
            let count = TIMER_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            if count == 1 {
                ::log::info!("Audio timer first fire");
            }
            self.update_mic_level(cx);
            // Poll Rust logs (50ms interval is fine for log updates)
            self.poll_rust_logs(cx);
            // Send actual buffer fill percentage to dora for backpressure control
            // This replaces the bridge's estimation with the real value from AudioPlayer
            if let Some(ref player) = self.audio_player {
                let fill_percentage = player.buffer_fill_percentage();
                if let Some(ref dora) = self.dora_integration {
                    dora.send_command(DoraCommand::UpdateBufferStatus { fill_percentage });
                }
            }
        }

        // Handle dora timer for polling dora events
        if self.dora_timer.is_event(event).is_some() {
            self.poll_dora_events(cx);
        }

        // Handle save animation timer - reset saved indicator after timeout
        if self.save_animation_timer.is_event(event).is_some() {
            if let Some(ref role) = self.save_animation_role.take() {
                let save_btn_id = match role.as_str() {
                    "student1" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_header.student1_save_btn),
                    "student2" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_header.student2_save_btn),
                    "tutor" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_header.tutor_save_btn),
                    "context" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_header.context_save_btn),
                    _ => return,
                };
                self.view.button(save_btn_id).apply_over(cx, live! { draw_bg: { saved: 0.0 } });
                self.view.redraw(cx);
            }
        }

        // Note: Log updates are now handled by timestamp-based throttling in poll_rust_logs/mark_log_dirty
        // No separate timer needed - the audio_timer (50ms) drives log polling

        // Handle NextFrame for animations only (mic level handled by audio_timer)
        if let Event::NextFrame(nf) = event {
            let mut needs_redraw = false;
            let current_time = nf.time;

            // Copy chat button fade animation
            if self.copy_chat_flash_active {
                // Capture start time on first frame
                if self.copy_chat_flash_start == 0.0 {
                    self.copy_chat_flash_start = current_time;
                }
                let elapsed = current_time - self.copy_chat_flash_start;
                // Hold at full brightness for 0.3s, then fade out over 0.5s
                let fade_start = 0.3;
                let fade_duration = 0.5;
                let total_duration = fade_start + fade_duration;

                if elapsed >= total_duration {
                    // Animation complete
                    self.copy_chat_flash_active = false;
                    self.view.view(ids!(left_column.running_tab_content.chat_container.chat_section.chat_header.copy_chat_btn))
                        .apply_over(cx, live!{ draw_bg: { copied: 0.0 } });
                } else if elapsed >= fade_start {
                    // Fade out phase - smoothstep interpolation
                    let t = (elapsed - fade_start) / fade_duration;
                    // Smoothstep: 3t² - 2t³ for smooth ease-out
                    let smooth_t = t * t * (3.0 - 2.0 * t);
                    let copied = 1.0 - smooth_t;
                    self.view.view(ids!(left_column.running_tab_content.chat_container.chat_section.chat_header.copy_chat_btn))
                        .apply_over(cx, live!{ draw_bg: { copied: (copied) } });
                }
                needs_redraw = true;
                if self.copy_chat_flash_active {
                    cx.new_next_frame();
                }
            }

            // Copy log button fade animation
            if self.copy_log_flash_active {
                // Capture start time on first frame
                if self.copy_log_flash_start == 0.0 {
                    self.copy_log_flash_start = current_time;
                }
                let elapsed = current_time - self.copy_log_flash_start;
                // Hold at full brightness for 0.3s, then fade out over 0.5s
                let fade_start = 0.3;
                let fade_duration = 0.5;
                let total_duration = fade_start + fade_duration;

                if elapsed >= total_duration {
                    // Animation complete
                    self.copy_log_flash_active = false;
                    self.view.view(ids!(log_section.log_content_column.log_header.log_filter_row.copy_log_btn))
                        .apply_over(cx, live!{ draw_bg: { copied: 0.0 } });
                } else if elapsed >= fade_start {
                    // Fade out phase - smoothstep interpolation
                    let t = (elapsed - fade_start) / fade_duration;
                    // Smoothstep: 3t² - 2t³ for smooth ease-out
                    let smooth_t = t * t * (3.0 - 2.0 * t);
                    let copied = 1.0 - smooth_t;
                    self.view.view(ids!(log_section.log_content_column.log_header.log_filter_row.copy_log_btn))
                        .apply_over(cx, live!{ draw_bg: { copied: (copied) } });
                }
                needs_redraw = true;
                if self.copy_log_flash_active {
                    cx.new_next_frame();
                }
            }

            // Maximize editor animation
            if self.maximize_animation_active {
                // Capture start time on first frame
                if self.maximize_animation_start == 0.0 {
                    self.maximize_animation_start = current_time;
                    ::log::info!("Maximize animation started, target: {:?}, expanding: {}",
                        self.maximize_animation_target, self.maximize_animation_expanding);
                }
                let elapsed = current_time - self.maximize_animation_start;
                let duration = 0.4;  // 400ms animation for more visible effect

                if elapsed >= duration {
                    // Animation complete
                    self.maximize_animation_active = false;
                    let final_value = if self.maximize_animation_expanding { 1.0 } else { 0.0 };
                    ::log::info!("Maximize animation complete, final_value: {}", final_value);
                    self.apply_maximize_value(cx, final_value);
                    // Finalize visibility after animation
                    self.finalize_maximize_visibility(cx);
                } else {
                    // Animate - use ease-out cubic for smooth deceleration
                    let t = elapsed / duration;
                    let ease_t = 1.0 - (1.0 - t).powi(3);
                    let value = if self.maximize_animation_expanding {
                        ease_t
                    } else {
                        1.0 - ease_t
                    };
                    ::log::info!("Maximize animation frame: t={:.3}, value={:.3}", t, value);
                    self.apply_maximize_value(cx, value);
                    cx.new_next_frame();
                }
                needs_redraw = true;
            }

            // Populate TextInputs and force render at startup
            if self.shader_precompile_frame > 0 && self.configs_preloaded {
                match self.shader_precompile_frame {
                    1 => {
                        // Step 1: Populate content and show Settings tab
                        self.lazy_populate_editor(cx, "context");
                        self.lazy_populate_editor(cx, "student1");
                        self.lazy_populate_editor(cx, "student2");
                        self.lazy_populate_editor(cx, "tutor");
                        self.view.view(ids!(left_column.settings_tab_content)).set_visible(cx, true);
                        ::log::info!("Startup: Content loaded, Settings visible for pre-render");
                        self.shader_precompile_frame = 2;
                        cx.new_next_frame();
                        needs_redraw = true;
                    }
                    5 => {
                        // Step 2: After a few frames, hide Settings and show Running
                        self.view.view(ids!(left_column.settings_tab_content)).set_visible(cx, false);
                        self.view.view(ids!(left_column.running_tab_content)).set_visible(cx, true);
                        self.shader_precompile_frame = 0;
                        ::log::info!("Startup: Pre-render complete, Settings hidden");
                        needs_redraw = true;
                    }
                    _ => {
                        // Keep rendering for a few frames
                        self.shader_precompile_frame += 1;
                        cx.new_next_frame();
                    }
                }
            }

            if needs_redraw {
                self.view.redraw(cx);
            }
        }

        // Handle mic mute button click
        let mic_btn = self.view.mic_button(ids!(running_tab_content.audio_container.audio_controls_row.mic_container.mic_group.mic_mute_btn));
        if mic_btn.clicked(&actions) {
            self.mic_muted = !self.mic_muted;
            ::log::info!("Mic mute toggled: muted={}", self.mic_muted);
            mic_btn.set_muted(cx, self.mic_muted);

            // Recording indicator only shows when dora is running and not muted
            let is_dora_running = self.dora_integration.as_ref().map(|d| d.is_running()).unwrap_or(false);
            mic_btn.set_recording(cx, is_dora_running && !self.mic_muted);

            // Send start/stop recording command to AEC bridge
            if let Some(ref dora) = self.dora_integration {
                if self.mic_muted {
                    dora.stop_recording();
                } else {
                    dora.start_recording();
                }
            }
        }

        // Handle AEC toggle button click
        // AEC toggle switches between:
        // - ON: macOS VoiceProcessingIO with hardware echo cancellation
        // - OFF: Regular CPAL mic capture (no echo cancellation)
        // Note: This does NOT stop recording - only mic mute does that
        let aec_btn = self.view.aec_button(ids!(running_tab_content.audio_container.audio_controls_row.aec_container.aec_group.aec_toggle_btn));
        if aec_btn.clicked(&actions) {
            self.aec_enabled = !self.aec_enabled;
            ::log::info!("AEC toggled: enabled={}", self.aec_enabled);
            aec_btn.set_enabled(cx, self.aec_enabled);

            // Only switch capture method, don't start/stop recording
            if let Some(ref dora) = self.dora_integration {
                dora.set_aec_enabled(self.aec_enabled);
            }
        }

        // Handle tab clicks
        let running_tab = self.view.view(ids!(left_column.tab_bar.running_tab));
        let settings_tab = self.view.view(ids!(left_column.tab_bar.settings_tab));

        // Running tab hover
        match event.hits(cx, running_tab.area()) {
            Hit::FingerHoverIn(_) => {
                if self.active_tab != 0 {
                    self.view.view(ids!(left_column.tab_bar.running_tab))
                        .apply_over(cx, live!{ draw_bg: { hover: 1.0 } });
                    self.view.redraw(cx);
                }
            }
            Hit::FingerHoverOut(_) => {
                self.view.view(ids!(left_column.tab_bar.running_tab))
                    .apply_over(cx, live!{ draw_bg: { hover: 0.0 } });
                self.view.redraw(cx);
            }
            Hit::FingerUp(_) => {
                if self.active_tab != 0 {
                    self.switch_tab(cx, 0);
                }
            }
            _ => {}
        }

        // Settings tab hover
        match event.hits(cx, settings_tab.area()) {
            Hit::FingerHoverIn(_) => {
                if self.active_tab != 1 {
                    self.view.view(ids!(left_column.tab_bar.settings_tab))
                        .apply_over(cx, live!{ draw_bg: { hover: 1.0 } });
                    self.view.redraw(cx);
                }
            }
            Hit::FingerHoverOut(_) => {
                self.view.view(ids!(left_column.tab_bar.settings_tab))
                    .apply_over(cx, live!{ draw_bg: { hover: 0.0 } });
                self.view.redraw(cx);
            }
            Hit::FingerUp(_) => {
                if self.active_tab != 1 {
                    self.switch_tab(cx, 1);
                }
            }
            _ => {}
        }

        // Handle splitter drag
        let splitter = self.view.view(ids!(splitter));
        match event.hits(cx, splitter.area()) {
            Hit::FingerDown(_) => {
                self.splitter_dragging = true;
            }
            Hit::FingerMove(fm) => {
                if self.splitter_dragging {
                    self.resize_log_panel(cx, fm.abs.x);
                }
            }
            Hit::FingerUp(_) => {
                self.splitter_dragging = false;
            }
            _ => {}
        }

        // Handle MofaHero start/stop — scoped to this screen's hero widget only
        let hero_uid = self.view.mofa_hero(ids!(left_column.mofa_hero)).widget_uid();
        match actions.find_widget_action_cast::<MofaHeroAction>(hero_uid) {
            MofaHeroAction::StartClicked => {
                ::log::info!("Screen received StartClicked action");
                self.handle_mofa_start(cx);
            }
            MofaHeroAction::StopClicked => {
                ::log::info!("Screen received StopClicked action");
                self.handle_mofa_stop(cx);
            }
            MofaHeroAction::None => {}
        }

        // Handle toggle log panel button
        // Use event.hits pattern for log toggle button
        let log_toggle_btn = self.view.button(ids!(log_section.toggle_column.toggle_log_btn));
        match event.hits(cx, log_toggle_btn.area()) {
            Hit::FingerUp(_) => {
                ::log::info!("Log toggle button FingerUp!");
                self.toggle_log_panel(cx);
            }
            _ => {}
        }

        // Handle input device dropdown selection
        if let Some(item) = self.view.drop_down(ids!(running_tab_content.audio_container.device_container.device_selectors.input_device_group.input_device_dropdown)).selected(&actions) {
            if item < self.input_devices.len() {
                let device_name = self.input_devices[item].clone();
                self.select_input_device(cx, &device_name);
            }
        }

        // Handle output device dropdown selection
        if let Some(item) = self.view.drop_down(ids!(running_tab_content.audio_container.device_container.device_selectors.output_device_group.output_device_dropdown)).selected(&actions) {
            if item < self.output_devices.len() {
                let device_name = self.output_devices[item].clone();
                self.select_output_device(&device_name);
            }
        }

        // Handle log level filter dropdown
        if let Some(selected) = self.view.drop_down(ids!(log_section.log_content_column.log_header.log_filter_row.level_filter)).selected(&actions) {
            self.log_level_filter = selected;
            self.update_log_display(cx);
        }

        // Handle log node filter dropdown
        if let Some(selected) = self.view.drop_down(ids!(log_section.log_content_column.log_header.log_filter_row.node_filter)).selected(&actions) {
            self.log_node_filter = selected;
            self.update_log_display(cx);
        }

        // Handle copy log button (manual click detection since it's a View)
        let copy_log_btn = self.view.view(ids!(log_section.log_content_column.log_header.log_filter_row.copy_log_btn));
        match event.hits(cx, copy_log_btn.area()) {
            Hit::FingerUp(_) => {
                self.copy_logs_to_clipboard(cx);
                // Trigger copied feedback animation with NextFrame-based smooth fade
                self.view.view(ids!(log_section.log_content_column.log_header.log_filter_row.copy_log_btn))
                    .apply_over(cx, live!{ draw_bg: { copied: 1.0 } });
                self.copy_log_flash_active = true;
                self.copy_log_flash_start = 0.0;  // Sentinel: capture actual time on first NextFrame
                cx.new_next_frame();
                self.view.redraw(cx);
            }
            _ => {}
        }

        // Handle copy chat button (manual click detection since it's a View)
        let copy_chat_btn = self.view.view(ids!(left_column.running_tab_content.chat_container.chat_section.chat_header.copy_chat_btn));
        match event.hits(cx, copy_chat_btn.area()) {
            Hit::FingerUp(_) => {
                self.copy_chat_to_clipboard(cx);
                // Trigger copied feedback animation with NextFrame-based smooth fade
                self.view.view(ids!(left_column.running_tab_content.chat_container.chat_section.chat_header.copy_chat_btn))
                    .apply_over(cx, live!{ draw_bg: { copied: 1.0 } });
                self.copy_chat_flash_active = true;
                self.copy_chat_flash_start = 0.0;  // Sentinel: capture actual time on first NextFrame
                cx.new_next_frame();
                self.view.redraw(cx);
            }
            _ => {}
        }

        // Handle log search text change
        if self.view.text_input(ids!(log_section.log_content_column.log_header.log_filter_row.log_search)).changed(&actions).is_some() {
            self.update_log_display(cx);
        }

        // Handle Send button click
        if self.view.button(ids!(left_column.prompt_container.prompt_section.prompt_row.button_group.send_prompt_btn)).clicked(&actions) {
            self.send_prompt(cx);
        }

        // Handle Reset button click
        if self.view.button(ids!(left_column.prompt_container.prompt_section.prompt_row.button_group.reset_btn)).clicked(&actions) {
            self.reset_conversation(cx);
        }

        // Handle Context Save button click
        if self.view.button(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_header.context_save_btn)).clicked(&actions) {
            self.save_context(cx);
        }

        // Handle role save button clicks
        if self.view.button(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_header.student1_save_btn)).clicked(&actions) {
            self.save_role_config(cx, "student1");
        }
        if self.view.button(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_header.student2_save_btn)).clicked(&actions) {
            self.save_role_config(cx, "student2");
        }
        if self.view.button(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_header.tutor_save_btn)).clicked(&actions) {
            self.save_role_config(cx, "tutor");
        }

        // Handle maximize button clicks
        let context_max_btn = self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_header.context_maximize_btn));
        match event.hits(cx, context_max_btn.area()) {
            Hit::FingerUp(_) => { self.toggle_maximize(cx, "context"); }
            _ => {}
        }

        let student1_max_btn = self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_header.student1_maximize_btn));
        match event.hits(cx, student1_max_btn.area()) {
            Hit::FingerUp(_) => { self.toggle_maximize(cx, "student1"); }
            _ => {}
        }

        let student2_max_btn = self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_header.student2_maximize_btn));
        match event.hits(cx, student2_max_btn.area()) {
            Hit::FingerUp(_) => { self.toggle_maximize(cx, "student2"); }
            _ => {}
        }

        let tutor_max_btn = self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_header.tutor_maximize_btn));
        match event.hits(cx, tutor_max_btn.area()) {
            Hit::FingerUp(_) => { self.toggle_maximize(cx, "tutor"); }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

// Tab switching
impl MoFaFMScreen {
    fn switch_tab(&mut self, cx: &mut Cx, tab: usize) {
        self.active_tab = tab;

        // Update tab button styles
        let running_selected = if tab == 0 { 1.0 } else { 0.0 };
        let settings_selected = if tab == 1 { 1.0 } else { 0.0 };

        self.view.view(ids!(left_column.tab_bar.running_tab))
            .apply_over(cx, live!{ draw_bg: { selected: (running_selected), hover: 0.0 } });
        self.view.label(ids!(left_column.tab_bar.running_tab.tab_label))
            .apply_over(cx, live!{ draw_text: { selected: (running_selected) } });

        self.view.view(ids!(left_column.tab_bar.settings_tab))
            .apply_over(cx, live!{ draw_bg: { selected: (settings_selected), hover: 0.0 } });
        self.view.label(ids!(left_column.tab_bar.settings_tab.tab_label))
            .apply_over(cx, live!{ draw_text: { selected: (settings_selected) } });

        // Toggle visibility of tab content
        self.view.view(ids!(left_column.running_tab_content)).set_visible(cx, tab == 0);
        self.view.view(ids!(left_column.settings_tab_content)).set_visible(cx, tab == 1);

        // Content already populated at startup - just show/hide
        self.view.redraw(cx);
    }

    /// Populate a role's model dropdown with models and select the default
    fn populate_role_dropdown(&mut self, cx: &mut Cx, role: &str, models: &[String], selected: &str) {
        let dropdown_id = match role {
            "student1" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_model_row.student1_model_dropdown),
            "student2" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_model_row.student2_model_dropdown),
            "tutor" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_model_row.tutor_model_dropdown),
            _ => return,
        };

        let dropdown = self.view.drop_down(dropdown_id);

        // Find selected index
        let selected_idx = models.iter()
            .position(|m| m == selected)
            .unwrap_or(0);

        dropdown.set_labels(cx, models.to_vec());
        dropdown.set_selected_item(cx, selected_idx);
    }

    /// Populate a role's voice dropdown and select the current voice
    fn populate_voice_dropdown(&mut self, cx: &mut Cx, role: &str, selected_voice: &str) {
        let dropdown_id = match role {
            "student1" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_voice_row.student1_voice_dropdown),
            "student2" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_voice_row.student2_voice_dropdown),
            "tutor" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_voice_row.tutor_voice_dropdown),
            _ => return,
        };

        let dropdown = self.view.drop_down(dropdown_id);

        // Find selected index
        let selected_idx = VOICE_OPTIONS.iter()
            .position(|&v| v == selected_voice)
            .unwrap_or(0);

        dropdown.set_selected_item(cx, selected_idx);
    }

    /// Save a role's configuration
    fn save_role_config(&mut self, cx: &mut Cx, role: &str) {
        let (config, prompt_input_id) = match role {
            "student1" => (
                &mut self.student1_config,
                ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_prompt_container.student1_prompt_scroll.student1_prompt_wrapper.student1_prompt_input)
            ),
            "student2" => (
                &mut self.student2_config,
                ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_prompt_container.student2_prompt_scroll.student2_prompt_wrapper.student2_prompt_input)
            ),
            "tutor" => (
                &mut self.tutor_config,
                ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_prompt_container.tutor_prompt_scroll.tutor_prompt_wrapper.tutor_prompt_input)
            ),
            _ => return,
        };

        // Get current system prompt from text input
        let system_prompt = self.view.text_input(prompt_input_id).text();
        config.system_prompt = system_prompt;

        // Get selected model from dropdown
        let dropdown_id = match role {
            "student1" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_model_row.student1_model_dropdown),
            "student2" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_model_row.student2_model_dropdown),
            "tutor" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_model_row.tutor_model_dropdown),
            _ => return,
        };

        let dropdown = self.view.drop_down(dropdown_id);
        let selected_idx = dropdown.selected_item();
        if selected_idx < config.models.len() {
            config.default_model = config.models[selected_idx].clone();
        }

        // Get selected voice from dropdown
        let voice_dropdown_id = match role {
            "student1" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_voice_row.student1_voice_dropdown),
            "student2" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_voice_row.student2_voice_dropdown),
            "tutor" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_voice_row.tutor_voice_dropdown),
            _ => return,
        };

        let voice_dropdown = self.view.drop_down(voice_dropdown_id);
        let voice_idx = voice_dropdown.selected_item();
        if voice_idx < VOICE_OPTIONS.len() {
            config.voice = VOICE_OPTIONS[voice_idx].to_string();
        }

        // Save to TOML file
        match config.save() {
            Ok(_) => ::log::info!("Saved {} config to TOML", role),
            Err(e) => ::log::error!("Failed to save {} config: {}", role, e),
        }

        // Also update VOICE_NAME in YAML dataflow file
        // Find the YAML path - either from dataflow_path or search common locations
        let yaml_path = self.dataflow_path.clone().or_else(|| {
            let cwd = std::env::current_dir().ok()?;
            // First try: apps/mofa-fm/dataflow/voice-chat.yml (workspace root)
            let app_path = cwd.join("apps").join("mofa-fm").join("dataflow").join("voice-chat.yml");
            if app_path.exists() {
                return Some(app_path);
            }
            // Second try: dataflow/voice-chat.yml (run from app directory)
            let local_path = cwd.join("dataflow").join("voice-chat.yml");
            if local_path.exists() {
                return Some(local_path);
            }
            None
        });

        if let Some(ref yaml_path) = yaml_path {
            match crate::screen::role_config::update_yaml_voice(yaml_path, role, &config.voice) {
                Ok(true) => ::log::info!("Updated {} voice in YAML: {}", role, config.voice),
                Ok(false) => ::log::warn!("Node primespeech-{} not found in YAML", role),
                Err(e) => ::log::error!("Failed to update {} voice in YAML: {}", role, e),
            }
        } else {
            ::log::warn!("YAML dataflow file not found, voice not synced to YAML");
        }

        // Trigger save button animation (green flash)
        let save_btn_id = match role {
            "student1" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_header.student1_save_btn),
            "student2" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_header.student2_save_btn),
            "tutor" => ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_header.tutor_save_btn),
            _ => return,
        };
        self.view.button(save_btn_id).apply_over(cx, live! { draw_bg: { saved: 1.0 } });

        // Start timer to fade out the saved indicator
        self.save_animation_timer = cx.start_timeout(1.5);
        self.save_animation_role = Some(role.to_string());

        self.view.redraw(cx);
    }

    /// Lazy populate a TextInput only when user first interacts with it
    fn lazy_populate_editor(&mut self, cx: &mut Cx, editor: &str) {
        match editor {
            "context" if !self.context_ui_populated && !self.context_content.is_empty() => {
                let content = self.context_content.clone();
                self.view.text_input(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_input_container.context_input_scroll.context_input_wrapper.context_input))
                    .set_text(cx, &content);
                self.context_ui_populated = true;
                ::log::info!("Lazy loaded context UI ({} bytes)", content.len());
            }
            "student1" if !self.student1_ui_populated && !self.student1_config.system_prompt.is_empty() => {
                let models = self.student1_config.models.clone();
                let default_model = self.student1_config.default_model.clone();
                let voice = self.student1_config.voice.clone();
                let prompt = self.student1_config.system_prompt.clone();
                self.populate_role_dropdown(cx, "student1", &models, &default_model);
                self.populate_voice_dropdown(cx, "student1", &voice);
                self.view.text_input(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_prompt_container.student1_prompt_scroll.student1_prompt_wrapper.student1_prompt_input))
                    .set_text(cx, &prompt);
                self.student1_ui_populated = true;
                ::log::info!("Lazy loaded student1 UI");
            }
            "student2" if !self.student2_ui_populated && !self.student2_config.system_prompt.is_empty() => {
                let models = self.student2_config.models.clone();
                let default_model = self.student2_config.default_model.clone();
                let voice = self.student2_config.voice.clone();
                let prompt = self.student2_config.system_prompt.clone();
                self.populate_role_dropdown(cx, "student2", &models, &default_model);
                self.populate_voice_dropdown(cx, "student2", &voice);
                self.view.text_input(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_prompt_container.student2_prompt_scroll.student2_prompt_wrapper.student2_prompt_input))
                    .set_text(cx, &prompt);
                self.student2_ui_populated = true;
                ::log::info!("Lazy loaded student2 UI");
            }
            "tutor" if !self.tutor_ui_populated && !self.tutor_config.system_prompt.is_empty() => {
                let models = self.tutor_config.models.clone();
                let default_model = self.tutor_config.default_model.clone();
                let voice = self.tutor_config.voice.clone();
                let prompt = self.tutor_config.system_prompt.clone();
                self.populate_role_dropdown(cx, "tutor", &models, &default_model);
                self.populate_voice_dropdown(cx, "tutor", &voice);
                self.view.text_input(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_prompt_container.tutor_prompt_scroll.tutor_prompt_wrapper.tutor_prompt_input))
                    .set_text(cx, &prompt);
                self.tutor_ui_populated = true;
                ::log::info!("Lazy loaded tutor UI");
            }
            _ => {}
        }
    }

    /// Toggle maximize state for an editor - takes over entire mofa-fm page
    fn toggle_maximize(&mut self, cx: &mut Cx, editor: &str) {
        ::log::info!("toggle_maximize called for editor: {}", editor);

        // Lazy populate TextInput on first interaction
        self.lazy_populate_editor(cx, editor);

        let is_currently_maximized = self.maximized_editor.as_deref() == Some(editor);

        // Start animation
        self.maximize_animation_active = true;
        self.maximize_animation_start = 0.0;  // Will be captured on first NextFrame
        self.maximize_animation_target = Some(editor.to_string());
        self.maximize_animation_expanding = !is_currently_maximized;
        ::log::info!("Animation started: active={}, expanding={}", self.maximize_animation_active, self.maximize_animation_expanding);
        cx.new_next_frame();

        if is_currently_maximized {
            // Restore: show all UI elements, reset heights
            self.maximized_editor = None;

            // Show tab bar
            self.view.view(ids!(left_column.tab_bar)).set_visible(cx, true);

            // Show settings header and section headers
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.settings_header))
                .set_visible(cx, true);
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.dataflow_section))
                .set_visible(cx, true);
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.role_section_title))
                .set_visible(cx, true);

            // Show audio section
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.audio_section))
                .set_visible(cx, true);

            // Show all role config sections with opacity 0 for fade-in animation
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section))
                .apply_over(cx, live!{ draw_bg: { opacity: 0.0 } });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section))
                .set_visible(cx, true);
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config))
                .apply_over(cx, live!{ draw_bg: { opacity: 0.0 } });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config))
                .set_visible(cx, true);
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config))
                .apply_over(cx, live!{ draw_bg: { opacity: 0.0 } });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config))
                .set_visible(cx, true);
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config))
                .apply_over(cx, live!{ draw_bg: { opacity: 0.0 } });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config))
                .set_visible(cx, true);

            // Re-enable outer scroll bar
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll))
                .apply_over(cx, live!{ scroll_bars: { show_scroll_y: true } });

            // Reset container heights to Fit/normal
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content))
                .apply_over(cx, live!{ height: Fit });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section))
                .apply_over(cx, live!{ height: Fit });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section))
                .apply_over(cx, live!{ height: Fit });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_input_container))
                .apply_over(cx, live!{ height: 200 });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config))
                .apply_over(cx, live!{ height: Fit });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_prompt_container))
                .apply_over(cx, live!{ height: 120 });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config))
                .apply_over(cx, live!{ height: Fit });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_prompt_container))
                .apply_over(cx, live!{ height: 120 });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config))
                .apply_over(cx, live!{ height: Fit });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_prompt_container))
                .apply_over(cx, live!{ height: 120 });

            // Icon animation will handle the maximize button state
        } else {
            // Maximize: hide everything except the editor, take over entire page
            self.maximized_editor = Some(editor.to_string());

            // Hide tab bar
            self.view.view(ids!(left_column.tab_bar)).set_visible(cx, false);

            // Hide settings header and section headers
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.settings_header))
                .set_visible(cx, false);
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.dataflow_section))
                .set_visible(cx, false);
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.role_section_title))
                .set_visible(cx, false);

            // Hide audio section
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.audio_section))
                .set_visible(cx, false);

            // Don't hide role config sections immediately - let the animation fade them out
            // They will be hidden at the end of the animation by apply_maximize_value
            // Keep all sections visible for now so they can animate
            // Just set initial opacity for fade-out (sections not being maximized start at opacity 1)
            // The animation handler will animate opacity to 0

            // Disable outer scroll and let inner editor scroll handle everything
            // Hide the outer settings_scroll's scroll bar
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll))
                .apply_over(cx, live!{ scroll_bars: { show_scroll_y: false } });

            // Set containers to Fill so editor takes entire space
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content))
                .apply_over(cx, live!{ height: Fill });
            self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section))
                .apply_over(cx, live!{ height: Fill });

            match editor {
                "context" => {
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section))
                        .apply_over(cx, live!{ height: Fill });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_input_container))
                        .apply_over(cx, live!{ height: Fill });
                    // Icon animation will handle the maximize button state
                }
                "student1" => {
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config))
                        .apply_over(cx, live!{ height: Fill });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_prompt_container))
                        .apply_over(cx, live!{ height: Fill });
                    // Icon animation will handle the maximize button state
                }
                "student2" => {
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config))
                        .apply_over(cx, live!{ height: Fill });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_prompt_container))
                        .apply_over(cx, live!{ height: Fill });
                    // Icon animation will handle the maximize button state
                }
                "tutor" => {
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config))
                        .apply_over(cx, live!{ height: Fill });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_prompt_container))
                        .apply_over(cx, live!{ height: Fill });
                    // Icon animation will handle the maximize button state
                }
                _ => {}
            }
        }

        self.view.redraw(cx);
    }

    /// Apply animated maximize value to the target editor's maximize button and fade other sections
    fn apply_maximize_value(&mut self, cx: &mut Cx, value: f64) {
        // Fade opacity for sections that are being hidden (inverse of maximize value)
        let fade_opacity = 1.0 - value;

        // Highlight pulse effect: peaks at value=0.5, creating a flash during animation
        // Using parabolic curve: 4 * value * (1 - value) gives 0→1→0 as value goes 0→1
        let highlight = 4.0 * value * (1.0 - value);

        if let Some(ref editor) = self.maximize_animation_target {
            // Apply maximize button animation
            match editor.as_str() {
                "context" => {
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_header.context_maximize_btn))
                        .apply_over(cx, live!{ draw_bg: { maximized: (value) } });
                    // Highlight the maximized section
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section))
                        .apply_over(cx, live!{ draw_bg: { highlight: (highlight) } });
                    // Fade out other sections
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                }
                "student1" => {
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_header.student1_maximize_btn))
                        .apply_over(cx, live!{ draw_bg: { maximized: (value) } });
                    // Highlight the maximized section
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config))
                        .apply_over(cx, live!{ draw_bg: { highlight: (highlight) } });
                    // Fade out other sections
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                }
                "student2" => {
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_header.student2_maximize_btn))
                        .apply_over(cx, live!{ draw_bg: { maximized: (value) } });
                    // Highlight the maximized section
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config))
                        .apply_over(cx, live!{ draw_bg: { highlight: (highlight) } });
                    // Fade out other sections
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                }
                "tutor" => {
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_header.tutor_maximize_btn))
                        .apply_over(cx, live!{ draw_bg: { maximized: (value) } });
                    // Highlight the maximized section
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config))
                        .apply_over(cx, live!{ draw_bg: { highlight: (highlight) } });
                    // Fade out other sections
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                    self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config))
                        .apply_over(cx, live!{ draw_bg: { opacity: (fade_opacity), highlight: 0.0 } });
                }
                _ => {}
            }
        }
    }

    /// Finalize visibility after maximize animation completes
    fn finalize_maximize_visibility(&mut self, cx: &mut Cx) {
        if let Some(ref editor) = self.maximize_animation_target.clone() {
            if self.maximize_animation_expanding {
                // Animation was maximizing - now hide the other sections and reset highlight
                let show_context = editor == "context";
                let show_student1 = editor == "student1";
                let show_student2 = editor == "student2";
                let show_tutor = editor == "tutor";

                self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section))
                    .set_visible(cx, show_context);
                self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config))
                    .set_visible(cx, show_student1);
                self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config))
                    .set_visible(cx, show_student2);
                self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config))
                    .set_visible(cx, show_tutor);

                // Reset highlight on the maximized section
                match editor.as_str() {
                    "context" => self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section))
                        .apply_over(cx, live!{ draw_bg: { highlight: 0.0 } }),
                    "student1" => self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config))
                        .apply_over(cx, live!{ draw_bg: { highlight: 0.0 } }),
                    "student2" => self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config))
                        .apply_over(cx, live!{ draw_bg: { highlight: 0.0 } }),
                    "tutor" => self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config))
                        .apply_over(cx, live!{ draw_bg: { highlight: 0.0 } }),
                    _ => {}
                }

                ::log::info!("Maximize complete: hidden other sections");
            } else {
                // Animation was restoring - reset opacity and highlight for all sections
                self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section))
                    .apply_over(cx, live!{ draw_bg: { opacity: 1.0, highlight: 0.0 } });
                self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config))
                    .apply_over(cx, live!{ draw_bg: { opacity: 1.0, highlight: 0.0 } });
                self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config))
                    .apply_over(cx, live!{ draw_bg: { opacity: 1.0, highlight: 0.0 } });
                self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config))
                    .apply_over(cx, live!{ draw_bg: { opacity: 1.0, highlight: 0.0 } });

                // Scroll to show the section that was restored at its original position
                // Context is at the bottom, so scroll down to show it
                // Other sections are near the top, so default scroll is fine
                match editor.as_str() {
                    "context" => {
                        // Scroll to bottom to show context section
                        self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll))
                            .set_scroll_pos(cx, DVec2 { x: 0.0, y: 1e10 });
                    }
                    "tutor" => {
                        // Tutor is near the bottom, scroll down to show it
                        self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll))
                            .set_scroll_pos(cx, DVec2 { x: 0.0, y: 800.0 });
                    }
                    "student2" => {
                        // Student2 is in the middle, scroll to show it
                        self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll))
                            .set_scroll_pos(cx, DVec2 { x: 0.0, y: 400.0 });
                    }
                    _ => {
                        // Student1 is near the top, scroll to top
                        self.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll))
                            .set_scroll_pos(cx, DVec2 { x: 0.0, y: 0.0 });
                    }
                }

                ::log::info!("Restore complete: reset all section opacities to 1.0");
            }
        }
    }

    /// Start async preloading in a background thread
    fn start_async_preload(&mut self) {
        // Compute paths upfront (on main thread)
        let context_path = self.get_context_path();
        let student1_path = get_role_config_path(self.dataflow_path.as_ref(), "student1");
        let student2_path = get_role_config_path(self.dataflow_path.as_ref(), "student2");
        let tutor_path = get_role_config_path(self.dataflow_path.as_ref(), "tutor");
        let yaml_path = get_yaml_path(self.dataflow_path.as_ref());

        // Create shared state for background thread
        let preload = Arc::new(Mutex::new(PreloadedData::default()));
        self.async_preload = Some(preload.clone());

        // Spawn background thread for file I/O
        std::thread::spawn(move || {
            let mut data = PreloadedData::default();

            // Load context file
            if let Ok(content) = std::fs::read_to_string(&context_path) {
                ::log::info!("Async preloaded study-context.md ({} bytes)", content.len());
                data.context_content = Some(content);
            }

            // Load role configs, reading voice from YAML
            if let Ok(mut config) = RoleConfig::load(&student1_path) {
                // Override voice from YAML if available
                if let Some(ref yaml) = yaml_path {
                    if let Some(voice) = read_yaml_voice(yaml, "student1") {
                        config.voice = voice;
                    }
                }
                ::log::info!("Async preloaded student1 config");
                data.student1_config = Some(config);
            }
            if let Ok(mut config) = RoleConfig::load(&student2_path) {
                // Override voice from YAML if available
                if let Some(ref yaml) = yaml_path {
                    if let Some(voice) = read_yaml_voice(yaml, "student2") {
                        config.voice = voice;
                    }
                }
                ::log::info!("Async preloaded student2 config");
                data.student2_config = Some(config);
            }
            if let Ok(mut config) = RoleConfig::load(&tutor_path) {
                // Override voice from YAML if available
                if let Some(ref yaml) = yaml_path {
                    if let Some(voice) = read_yaml_voice(yaml, "tutor") {
                        config.voice = voice;
                    }
                }
                ::log::info!("Async preloaded tutor config");
                data.tutor_config = Some(config);
            }

            data.loading_complete = true;

            // Store results in shared state
            if let Ok(mut shared) = preload.lock() {
                *shared = data;
            }
        });
    }

    /// Preload all configs into memory at startup (no UI updates) - DEPRECATED, use start_async_preload
    #[allow(dead_code)]
    fn preload_configs(&mut self) {
        // Preload context file
        let context_path = self.get_context_path();
        if let Ok(content) = std::fs::read_to_string(&context_path) {
            ::log::info!("Preloaded study-context.md ({} bytes)", content.len());
            self.context_content = content;
        }

        // Preload role configs
        let student1_path = get_role_config_path(self.dataflow_path.as_ref(), "student1");
        if let Ok(config) = RoleConfig::load(&student1_path) {
            ::log::info!("Preloaded student1 config");
            self.student1_config = config;
        }

        let student2_path = get_role_config_path(self.dataflow_path.as_ref(), "student2");
        if let Ok(config) = RoleConfig::load(&student2_path) {
            ::log::info!("Preloaded student2 config");
            self.student2_config = config;
        }

        let tutor_path = get_role_config_path(self.dataflow_path.as_ref(), "tutor");
        if let Ok(config) = RoleConfig::load(&tutor_path) {
            ::log::info!("Preloaded tutor config");
            self.tutor_config = config;
        }
    }

    /// Save context editor content to study-context.md
    fn save_context(&mut self, cx: &mut Cx) {
        let context_path = self.get_context_path();
        let content = self.view.text_input(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_input_container.context_input_scroll.context_input_wrapper.context_input))
            .text();

        match std::fs::write(&context_path, &content) {
            Ok(_) => {
                self.context_content = content.clone();
                // Flash save button green
                self.view.button(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_header.context_save_btn))
                    .apply_over(cx, live! { draw_bg: { saved: 1.0 } });
                self.save_animation_timer = cx.start_timeout(1.5);
                self.save_animation_role = Some("context".to_string());
                ::log::info!("Saved study-context.md ({} bytes)", content.len());
            }
            Err(e) => {
                ::log::error!("Failed to save study-context.md: {}", e);
            }
        }
        self.view.redraw(cx);
    }

    /// Get the path to study-context.md in the dataflow directory
    fn get_context_path(&self) -> PathBuf {
        // Try to use the dataflow_path if set, otherwise search common locations
        if let Some(ref dataflow_path) = self.dataflow_path {
            // Get directory containing the dataflow yaml
            if let Some(parent) = dataflow_path.parent() {
                return parent.join("study-context.md");
            }
        }

        // Fallback: search common locations
        let cwd = std::env::current_dir().unwrap_or_default();

        // First try: apps/mofa-fm/dataflow/study-context.md (workspace root)
        let app_path = cwd.join("apps").join("mofa-fm").join("dataflow").join("study-context.md");
        if app_path.exists() {
            return app_path;
        }

        // Second try: dataflow/study-context.md (run from app directory)
        let local_path = cwd.join("dataflow").join("study-context.md");
        if local_path.exists() {
            return local_path;
        }

        // Default: assume workspace root structure
        app_path
    }
}

impl MoFaFMScreenRef {
    /// Update dark mode for this screen
    /// Delegates to StateChangeListener::on_dark_mode_change for consistency
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        self.on_dark_mode_change(cx, dark_mode);
    }
}

impl TimerControl for MoFaFMScreenRef {
    /// Stop audio and dora timers - call this before hiding/removing the widget
    /// to prevent timer callbacks on inactive state
    /// Note: AEC blink animation is shader-driven and doesn't need stopping
    fn stop_timers(&self, cx: &mut Cx) {
        if let Some(inner) = self.borrow_mut() {
            cx.stop_timer(inner.audio_timer);
            cx.stop_timer(inner.dora_timer);
            ::log::debug!("MoFaFMScreen timers stopped");
        }
    }

    /// Restart audio and dora timers - call this when the widget becomes visible again
    /// Note: AEC blink animation is shader-driven and auto-resumes
    fn start_timers(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.audio_timer = cx.start_interval(0.05);  // 50ms for mic level
            inner.dora_timer = cx.start_interval(0.1);    // 100ms for dora events
            ::log::debug!("MoFaFMScreen timers started");
        }
    }
}

impl StateChangeListener for MoFaFMScreenRef {
    fn on_dark_mode_change(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            // Apply dark mode to screen background
            inner.view.apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to chat section
            inner.view.view(ids!(left_column.running_tab_content.chat_container.chat_section)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to chat header and title
            inner.view.view(ids!(left_column.running_tab_content.chat_container.chat_section.chat_header)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.running_tab_content.chat_container.chat_section.chat_header.chat_title)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to copy chat button
            inner.view.view(ids!(left_column.running_tab_content.chat_container.chat_section.chat_header.copy_chat_btn)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to chat content Markdown
            let chat_markdown = inner.view.markdown(ids!(left_column.running_tab_content.chat_container.chat_section.chat_scroll.chat_content_wrapper.chat_content));
            if dark_mode > 0.5 {
                let light_color = vec4(0.945, 0.961, 0.976, 1.0); // TEXT_PRIMARY_DARK (#f1f5f9)
                chat_markdown.apply_over(cx, live!{
                    font_color: (light_color)
                    draw_normal: { color: (light_color) }
                    draw_bold: { color: (light_color) }
                    draw_italic: { color: (light_color) }
                    draw_fixed: { color: (vec4(0.580, 0.639, 0.722, 1.0)) } // SLATE_400 for code
                });
            } else {
                let dark_color = vec4(0.122, 0.161, 0.216, 1.0); // TEXT_PRIMARY (#1f2937)
                chat_markdown.apply_over(cx, live!{
                    font_color: (dark_color)
                    draw_normal: { color: (dark_color) }
                    draw_bold: { color: (dark_color) }
                    draw_italic: { color: (dark_color) }
                    draw_fixed: { color: (vec4(0.420, 0.451, 0.502, 1.0)) } // GRAY_500 for code
                });
            }

            // Apply dark mode to tab bar
            inner.view.view(ids!(left_column.tab_bar)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(left_column.tab_bar.running_tab)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.tab_bar.running_tab.tab_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(left_column.tab_bar.settings_tab)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.tab_bar.settings_tab.tab_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to audio control containers
            inner.view.view(ids!(running_tab_content.audio_container.audio_controls_row.mic_container)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            // Apply dark mode to mic button and level meter
            inner.view.mic_button(ids!(running_tab_content.audio_container.audio_controls_row.mic_container.mic_group.mic_mute_btn))
                .apply_dark_mode(cx, dark_mode);
            inner.view.led_meter(ids!(running_tab_content.audio_container.audio_controls_row.mic_container.mic_group.mic_level_meter))
                .apply_dark_mode(cx, dark_mode);
            inner.view.view(ids!(running_tab_content.audio_container.audio_controls_row.aec_container)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(running_tab_content.audio_container.audio_controls_row.buffer_container)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            // Apply dark mode to buffer level meter
            inner.view.led_meter(ids!(running_tab_content.audio_container.audio_controls_row.buffer_container.buffer_group.buffer_meter))
                .apply_dark_mode(cx, dark_mode);
            inner.view.view(ids!(running_tab_content.audio_container.device_container)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to device dropdowns
            inner.view.drop_down(ids!(running_tab_content.audio_container.device_container.device_selectors.input_device_group.input_device_dropdown)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.drop_down(ids!(running_tab_content.audio_container.device_container.device_selectors.output_device_group.output_device_dropdown)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
                draw_bg: { dark_mode: (dark_mode) }
            });
            // Apply dark mode to device labels
            inner.view.label(ids!(running_tab_content.audio_container.device_container.device_selectors.input_device_group.input_device_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(running_tab_content.audio_container.device_container.device_selectors.output_device_group.output_device_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to MofaHero
            inner.view.mofa_hero(ids!(left_column.mofa_hero)).update_dark_mode(cx, dark_mode);

            // Apply dark mode to participant panels
            inner.view.participant_panel(ids!(left_column.running_tab_content.participant_container.participant_bar.student1_panel)).update_dark_mode(cx, dark_mode);
            inner.view.participant_panel(ids!(left_column.running_tab_content.participant_container.participant_bar.student2_panel)).update_dark_mode(cx, dark_mode);
            inner.view.participant_panel(ids!(left_column.running_tab_content.participant_container.participant_bar.tutor_panel)).update_dark_mode(cx, dark_mode);

            // Apply dark mode to prompt section
            inner.view.view(ids!(left_column.running_tab_content.prompt_container.prompt_section)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            // NOTE: TextInput apply_over causes "target class not found" errors
            inner.view.button(ids!(left_column.running_tab_content.prompt_container.prompt_section.prompt_row.button_group.reset_btn)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to settings tab content
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            // Settings header labels
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_header.settings_title)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_header.settings_subtitle)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            // Dataflow section labels
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.dataflow_section.dataflow_section_title)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.dataflow_section.dataflow_path_row.dataflow_path_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.dataflow_section.dataflow_path_row.dataflow_path_value)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            // Role section - title
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.role_section_title)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            // Role section - student1 config
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_header.student1_name)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_model_row.student1_model_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.drop_down(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_model_row.student1_model_dropdown)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_voice_row.student1_voice_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.drop_down(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_voice_row.student1_voice_dropdown)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_prompt_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_prompt_container)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.text_input(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_prompt_container.student1_prompt_scroll.student1_prompt_wrapper.student1_prompt_input)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
                draw_cursor: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student1_config.student1_header.student1_maximize_btn)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            // Role section - student2 config
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_header.student2_name)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_model_row.student2_model_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.drop_down(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_model_row.student2_model_dropdown)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_voice_row.student2_voice_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.drop_down(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_voice_row.student2_voice_dropdown)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_prompt_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_prompt_container)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.text_input(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_prompt_container.student2_prompt_scroll.student2_prompt_wrapper.student2_prompt_input)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
                draw_cursor: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.student2_config.student2_header.student2_maximize_btn)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            // Role section - tutor config
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_header.tutor_name)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_model_row.tutor_model_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.drop_down(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_model_row.tutor_model_dropdown)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_voice_row.tutor_voice_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.drop_down(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_voice_row.tutor_voice_dropdown)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_prompt_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_prompt_container)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.text_input(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_prompt_container.tutor_prompt_scroll.tutor_prompt_wrapper.tutor_prompt_input)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
                draw_cursor: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.tutor_config.tutor_header.tutor_maximize_btn)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            // Role section - shared context
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_header.context_title)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_input_container)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.text_input(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_input_container.context_input_scroll.context_input_wrapper.context_input)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
                draw_cursor: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.role_section.context_section.context_header.context_maximize_btn)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            // Audio section labels
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.audio_section.audio_section_title)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.audio_section.sample_rate_row.sample_rate_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.audio_section.sample_rate_row.sample_rate_value)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.audio_section.buffer_size_row.buffer_size_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(left_column.settings_tab_content.settings_panel.settings_scroll.settings_content.audio_section.buffer_size_row.buffer_size_value)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to splitter
            inner.view.view(ids!(splitter)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to log section - toggle column
            inner.view.view(ids!(log_section.toggle_column)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.button(ids!(log_section.toggle_column.toggle_log_btn)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to log section - log content column
            inner.view.view(ids!(log_section.log_content_column)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.view(ids!(log_section.log_content_column.log_header)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(log_section.log_content_column.log_header.log_title_row.log_title_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to log filter dropdowns
            inner.view.drop_down(ids!(log_section.log_content_column.log_header.log_filter_row.level_filter)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.drop_down(ids!(log_section.log_content_column.log_header.log_filter_row.node_filter)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to search icon and search input
            inner.view.view(ids!(log_section.log_content_column.log_header.log_filter_row.search_icon)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.text_input(ids!(log_section.log_content_column.log_header.log_filter_row.log_search)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
                draw_text: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to copy log button
            inner.view.view(ids!(log_section.log_content_column.log_header.log_filter_row.copy_log_btn)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Apply dark mode to log content Label
            inner.view.label(ids!(log_section.log_content_column.log_scroll.log_content_wrapper.log_content)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            inner.view.redraw(cx);
        }
    }
}
