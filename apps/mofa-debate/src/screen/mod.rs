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
pub mod design; // Public for Makepad live_design path resolution
mod dora_handlers;
mod log_panel;

use crate::dora_integration::{DoraCommand, DoraIntegration};
use makepad_widgets::*;
use mofa_ui::{
    MofaHeroAction, MofaHeroWidgetExt,
    MicButtonWidgetExt,
    AecButtonWidgetExt,
    LedMeterWidgetExt,
    AudioManager,
    log_bridge,
};
use mofa_widgets::participant_panel::ParticipantPanelWidgetExt;
use mofa_widgets::{StateChangeListener, TimerControl};
use std::collections::VecDeque;
use std::path::PathBuf;

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
pub struct MoFaDebateScreen {
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
    log_level_filter: usize, // 0=ALL, 1=DEBUG, 2=INFO, 3=WARN, 4=ERROR
    #[rust]
    log_node_filter: usize, // 0=ALL, 1=ASR, 2=TTS, 3=LLM, 4=Bridge, 5=Monitor, 6=App
    #[rust]
    log_entries: VecDeque<String>,

    // Dropdown width caching for popup menu sync
    #[rust]
    dropdown_widths_initialized: bool,
    #[rust]
    cached_input_dropdown_width: f64,
    #[rust]
    cached_output_dropdown_width: f64,

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
    copy_chat_flash_start: f64, // Absolute start time
    #[rust]
    copy_log_flash_active: bool,
    #[rust]
    copy_log_flash_start: f64, // Absolute start time
    #[rust]
    chat_messages: Vec<ChatMessageEntry>,
    #[rust]
    last_chat_count: usize,

    // Audio playback
    #[rust]
    audio_player: Option<std::sync::Arc<crate::audio_player::AudioPlayer>>,
    // Participant audio levels for decay animation (matches conference-dashboard)
    #[rust]
    participant_levels: [f64; 3], // 0=student1, 1=student2, 2=tutor

    // SharedDoraState tracking (for detecting changes)
    #[rust]
    connected_bridges: Vec<String>,
    #[rust]
    processed_dora_log_count: usize,
    #[rust]
    pending_prompts: Vec<String>,
}

impl Widget for MoFaDebateScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Initialize audio and log bridge on first event
        if !self.audio_initialized {
            // Initialize log bridge to capture Rust logs
            log_bridge::init();
            self.init_audio(cx);
            self.audio_initialized = true;
        }

        // Handle audio timer for mic level updates, log polling, and buffer status
        if self.audio_timer.is_event(event).is_some() {
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

        // Handle NextFrame for smooth copy button fade animation
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
                    self.view
                        .view(ids!(
                            left_column
                                .chat_container
                                .chat_section
                                .chat_header
                                .copy_chat_btn
                        ))
                        .apply_over(cx, live! { draw_bg: { copied: 0.0 } });
                } else if elapsed >= fade_start {
                    // Fade out phase - smoothstep interpolation
                    let t = (elapsed - fade_start) / fade_duration;
                    // Smoothstep: 3t² - 2t³ for smooth ease-out
                    let smooth_t = t * t * (3.0 - 2.0 * t);
                    let copied = 1.0 - smooth_t;
                    self.view
                        .view(ids!(
                            left_column
                                .chat_container
                                .chat_section
                                .chat_header
                                .copy_chat_btn
                        ))
                        .apply_over(cx, live! { draw_bg: { copied: (copied) } });
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
                    self.view
                        .view(ids!(
                            log_section
                                .log_content_column
                                .log_header
                                .log_filter_row
                                .copy_log_btn
                        ))
                        .apply_over(cx, live! { draw_bg: { copied: 0.0 } });
                } else if elapsed >= fade_start {
                    // Fade out phase - smoothstep interpolation
                    let t = (elapsed - fade_start) / fade_duration;
                    // Smoothstep: 3t² - 2t³ for smooth ease-out
                    let smooth_t = t * t * (3.0 - 2.0 * t);
                    let copied = 1.0 - smooth_t;
                    self.view
                        .view(ids!(
                            log_section
                                .log_content_column
                                .log_header
                                .log_filter_row
                                .copy_log_btn
                        ))
                        .apply_over(cx, live! { draw_bg: { copied: (copied) } });
                }
                needs_redraw = true;
                if self.copy_log_flash_active {
                    cx.new_next_frame();
                }
            }

            if needs_redraw {
                self.view.redraw(cx);
            }
        }

        // Handle mic and AEC button actions (using shared mofa-ui widgets)

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

        // Handle actions
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => &[],
        };

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

        // Handle mic button click (using shared MicButton widget)
        let mic_btn = self.view.mic_button(ids!(audio_container.mic_container.mic_group.mic_mute_btn));
        if mic_btn.clicked(actions) {
            self.mic_muted = !self.mic_muted;
            mic_btn.set_muted(cx, self.mic_muted);
        }

        // Handle AEC button click (using shared AecButton widget)
        let aec_btn = self.view.aec_button(ids!(audio_container.aec_container.aec_group.aec_toggle_btn));
        if aec_btn.clicked(actions) {
            self.aec_enabled = !self.aec_enabled;
            aec_btn.set_enabled(cx, self.aec_enabled);
        }

        // Handle toggle log panel button
        if self
            .view
            .button(ids!(log_section.toggle_column.toggle_log_btn))
            .clicked(actions)
        {
            self.toggle_log_panel(cx);
        }

        // Handle input device selection
        if let Some(item) = self
            .view
            .drop_down(ids!(
                audio_container
                    .device_container
                    .device_selectors
                    .input_device_group
                    .input_device_dropdown
            ))
            .selected(actions)
        {
            if item < self.input_devices.len() {
                let device_name = self.input_devices[item].clone();
                self.select_input_device(cx, &device_name);
            }
        }

        // Handle output device selection
        if let Some(item) = self
            .view
            .drop_down(ids!(
                audio_container
                    .device_container
                    .device_selectors
                    .output_device_group
                    .output_device_dropdown
            ))
            .selected(actions)
        {
            if item < self.output_devices.len() {
                let device_name = self.output_devices[item].clone();
                self.select_output_device(&device_name);
            }
        }

        // Handle log level filter dropdown
        if let Some(selected) = self
            .view
            .drop_down(ids!(
                log_section
                    .log_content_column
                    .log_header
                    .log_filter_row
                    .level_filter
            ))
            .selected(actions)
        {
            self.log_level_filter = selected;
            self.update_log_display(cx);
        }

        // Handle log node filter dropdown
        if let Some(selected) = self
            .view
            .drop_down(ids!(
                log_section
                    .log_content_column
                    .log_header
                    .log_filter_row
                    .node_filter
            ))
            .selected(actions)
        {
            self.log_node_filter = selected;
            self.update_log_display(cx);
        }

        // Handle copy log button (manual click detection since it's a View)
        let copy_log_btn = self.view.view(ids!(
            log_section
                .log_content_column
                .log_header
                .log_filter_row
                .copy_log_btn
        ));
        match event.hits(cx, copy_log_btn.area()) {
            Hit::FingerUp(_) => {
                self.copy_logs_to_clipboard(cx);
                // Trigger copied feedback animation with NextFrame-based smooth fade
                self.view
                    .view(ids!(
                        log_section
                            .log_content_column
                            .log_header
                            .log_filter_row
                            .copy_log_btn
                    ))
                    .apply_over(cx, live! { draw_bg: { copied: 1.0 } });
                self.copy_log_flash_active = true;
                self.copy_log_flash_start = 0.0; // Sentinel: capture actual time on first NextFrame
                cx.new_next_frame();
                self.view.redraw(cx);
            }
            _ => {}
        }

        // Handle copy chat button (manual click detection since it's a View)
        let copy_chat_btn = self.view.view(ids!(
            left_column
                .chat_container
                .chat_section
                .chat_header
                .copy_chat_btn
        ));
        match event.hits(cx, copy_chat_btn.area()) {
            Hit::FingerUp(_) => {
                self.copy_chat_to_clipboard(cx);
                // Trigger copied feedback animation with NextFrame-based smooth fade
                self.view
                    .view(ids!(
                        left_column
                            .chat_container
                            .chat_section
                            .chat_header
                            .copy_chat_btn
                    ))
                    .apply_over(cx, live! { draw_bg: { copied: 1.0 } });
                self.copy_chat_flash_active = true;
                self.copy_chat_flash_start = 0.0; // Sentinel: capture actual time on first NextFrame
                cx.new_next_frame();
                self.view.redraw(cx);
            }
            _ => {}
        }

        // Handle log search text change
        if self
            .view
            .text_input(ids!(
                log_section
                    .log_content_column
                    .log_header
                    .log_filter_row
                    .log_search
            ))
            .changed(actions)
            .is_some()
        {
            self.update_log_display(cx);
        }

        // Handle Send button click
        if self
            .view
            .button(ids!(
                left_column
                    .prompt_container
                    .prompt_section
                    .prompt_row
                    .button_group
                    .send_prompt_btn
            ))
            .clicked(actions)
        {
            self.send_prompt(cx);
        }

        // Handle Reset button click
        if self
            .view
            .button(ids!(
                left_column
                    .prompt_container
                    .prompt_section
                    .prompt_row
                    .button_group
                    .reset_btn
            ))
            .clicked(actions)
        {
            self.reset_conversation(cx);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Update popup menu widths to match dropdown widths
        // This handles first-frame zero width and caches values for performance
        let input_dropdown = self.view.drop_down(ids!(
            audio_container
                .device_container
                .device_selectors
                .input_device_group
                .input_device_dropdown
        ));
        let input_width = input_dropdown.area().rect(cx).size.x;

        // Only update if width changed significantly (> 1px) to avoid unnecessary apply_over calls
        if input_width > 0.0 && (input_width - self.cached_input_dropdown_width).abs() > 1.0 {
            self.cached_input_dropdown_width = input_width;
            input_dropdown.apply_over(
                cx,
                live! {
                    popup_menu: { width: (input_width) }
                },
            );
        }

        let output_dropdown = self.view.drop_down(ids!(
            audio_container
                .device_container
                .device_selectors
                .output_device_group
                .output_device_dropdown
        ));
        let output_width = output_dropdown.area().rect(cx).size.x;

        // Only update if width changed significantly (> 1px)
        if output_width > 0.0 && (output_width - self.cached_output_dropdown_width).abs() > 1.0 {
            self.cached_output_dropdown_width = output_width;
            output_dropdown.apply_over(
                cx,
                live! {
                    popup_menu: { width: (output_width) }
                },
            );
        }

        // Force an extra redraw on first frame to ensure widths are properly captured
        // This fixes the issue where first click shows narrow popup (width=0 on first frame)
        if !self.dropdown_widths_initialized {
            self.dropdown_widths_initialized = true;
            self.view.redraw(cx);
        }

        self.view.draw_walk(cx, scope, walk)
    }
}

impl MoFaDebateScreenRef {
    /// Update dark mode for this screen
    /// Delegates to StateChangeListener::on_dark_mode_change for consistency
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        self.on_dark_mode_change(cx, dark_mode);
    }
}

impl TimerControl for MoFaDebateScreenRef {
    /// Stop audio and dora timers - call this before hiding/removing the widget
    /// to prevent timer callbacks on inactive state
    /// Note: AEC blink animation is shader-driven and doesn't need stopping
    fn stop_timers(&self, cx: &mut Cx) {
        if let Some(inner) = self.borrow_mut() {
            cx.stop_timer(inner.audio_timer);
            cx.stop_timer(inner.dora_timer);
            ::log::debug!("MoFaDebateScreen timers stopped");
        }
    }

    /// Restart audio and dora timers - call this when the widget becomes visible again
    /// Note: AEC blink animation is shader-driven and auto-resumes
    fn start_timers(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.audio_timer = cx.start_interval(0.05); // 50ms for mic level
            inner.dora_timer = cx.start_interval(0.1); // 100ms for dora events
            ::log::debug!("MoFaDebateScreen timers started");
        }
    }
}

impl StateChangeListener for MoFaDebateScreenRef {
    fn on_dark_mode_change(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            // Apply dark mode to screen background
            inner.view.apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );

            // Apply dark mode to chat section
            inner
                .view
                .view(ids!(left_column.chat_container.chat_section))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );

            // Apply dark mode to chat header and title
            inner
                .view
                .view(ids!(left_column.chat_container.chat_section.chat_header))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(
                    left_column
                        .chat_container
                        .chat_section
                        .chat_header
                        .chat_title
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Apply dark mode to copy chat button
            inner
                .view
                .view(ids!(
                    left_column
                        .chat_container
                        .chat_section
                        .chat_header
                        .copy_chat_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );

            // Apply dark mode to chat content Markdown
            let chat_markdown = inner.view.markdown(ids!(
                left_column
                    .chat_container
                    .chat_section
                    .chat_scroll
                    .chat_content_wrapper
                    .chat_content
            ));
            if dark_mode > 0.5 {
                let light_color = vec4(0.945, 0.961, 0.976, 1.0); // TEXT_PRIMARY_DARK (#f1f5f9)
                chat_markdown.apply_over(
                    cx,
                    live! {
                        font_color: (light_color)
                        draw_normal: { color: (light_color) }
                        draw_bold: { color: (light_color) }
                        draw_italic: { color: (light_color) }
                        draw_fixed: { color: (vec4(0.580, 0.639, 0.722, 1.0)) } // SLATE_400 for code
                    },
                );
            } else {
                let dark_color = vec4(0.122, 0.161, 0.216, 1.0); // TEXT_PRIMARY (#1f2937)
                chat_markdown.apply_over(
                    cx,
                    live! {
                        font_color: (dark_color)
                        draw_normal: { color: (dark_color) }
                        draw_bold: { color: (dark_color) }
                        draw_italic: { color: (dark_color) }
                        draw_fixed: { color: (vec4(0.420, 0.451, 0.502, 1.0)) } // GRAY_500 for code
                    },
                );
            }

            // Apply dark mode to audio control containers
            inner
                .view
                .view(ids!(left_column.audio_container.mic_container))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );
            // Apply dark mode to mic button (using shared MicButton widget)
            inner
                .view
                .mic_button(ids!(left_column.audio_container.mic_container.mic_group.mic_mute_btn))
                .apply_dark_mode(cx, dark_mode);

            // Apply dark mode to LED meters (using shared LedMeter widget)
            inner
                .view
                .led_meter(ids!(left_column.audio_container.mic_container.mic_group.mic_level_meter))
                .apply_dark_mode(cx, dark_mode);
            inner
                .view
                .led_meter(ids!(left_column.audio_container.buffer_container.buffer_group.buffer_meter))
                .apply_dark_mode(cx, dark_mode);
            inner
                .view
                .view(ids!(left_column.audio_container.aec_container))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .view(ids!(left_column.audio_container.buffer_container))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .view(ids!(left_column.audio_container.device_container))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );

            // Apply dark mode to device labels
            inner
                .view
                .label(ids!(
                    left_column
                        .audio_container
                        .device_container
                        .device_selectors
                        .input_device_group
                        .input_device_label
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(
                    left_column
                        .audio_container
                        .device_container
                        .device_selectors
                        .output_device_group
                        .output_device_label
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // NOTE: DropDown apply_over causes "target class not found" errors
            // TODO: Find alternative way to theme dropdowns

            // Apply dark mode to MofaHero
            inner
                .view
                .mofa_hero(ids!(left_column.mofa_hero))
                .update_dark_mode(cx, dark_mode);

            // Apply dark mode to participant panels
            inner
                .view
                .participant_panel(ids!(
                    left_column
                        .participant_container
                        .participant_bar
                        .student1_panel
                ))
                .update_dark_mode(cx, dark_mode);
            inner
                .view
                .participant_panel(ids!(
                    left_column
                        .participant_container
                        .participant_bar
                        .student2_panel
                ))
                .update_dark_mode(cx, dark_mode);
            inner
                .view
                .participant_panel(ids!(
                    left_column
                        .participant_container
                        .participant_bar
                        .tutor_panel
                ))
                .update_dark_mode(cx, dark_mode);

            // Apply dark mode to prompt section
            inner
                .view
                .view(ids!(left_column.prompt_container.prompt_section))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );
            // NOTE: TextInput apply_over causes "target class not found" errors
            inner
                .view
                .button(ids!(
                    left_column
                        .prompt_container
                        .prompt_section
                        .prompt_row
                        .button_group
                        .reset_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Apply dark mode to splitter
            inner.view.view(ids!(splitter)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );

            // Apply dark mode to log section - toggle column
            inner.view.view(ids!(log_section.toggle_column)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );
            inner
                .view
                .button(ids!(log_section.toggle_column.toggle_log_btn))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Apply dark mode to log section - log content column
            inner
                .view
                .view(ids!(log_section.log_content_column))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .view(ids!(log_section.log_content_column.log_header))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(
                    log_section
                        .log_content_column
                        .log_header
                        .log_title_row
                        .log_title_label
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Apply dark mode to log filter dropdowns
            inner
                .view
                .drop_down(ids!(
                    log_section
                        .log_content_column
                        .log_header
                        .log_filter_row
                        .level_filter
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .drop_down(ids!(
                    log_section
                        .log_content_column
                        .log_header
                        .log_filter_row
                        .node_filter
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Apply dark mode to search icon and search input
            inner
                .view
                .view(ids!(
                    log_section
                        .log_content_column
                        .log_header
                        .log_filter_row
                        .search_icon
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .text_input(ids!(
                    log_section
                        .log_content_column
                        .log_header
                        .log_filter_row
                        .log_search
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Apply dark mode to copy log button
            inner
                .view
                .view(ids!(
                    log_section
                        .log_content_column
                        .log_header
                        .log_filter_row
                        .copy_log_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                    },
                );

            // Apply dark mode to log content Markdown
            // Update dark_mode instance variable on each draw component (they have get_color shader functions)
            let log_markdown = inner.view.markdown(ids!(
                log_section
                    .log_content_column
                    .log_scroll
                    .log_content_wrapper
                    .log_content
            ));
            log_markdown.apply_over(
                cx,
                live! {
                    draw_normal: { dark_mode: (dark_mode) }
                    draw_bold: { dark_mode: (dark_mode) }
                    draw_fixed: { dark_mode: (dark_mode) }
                },
            );

            inner.view.redraw(cx);
        }
    }
}
