//! Audio control methods for MoFaFMScreen
//!
//! Handles audio device selection, mic monitoring, and level visualization.

use makepad_widgets::*;
use mofa_settings::data::Preferences;
use mofa_ui::LedMeterWidgetExt;

use super::MoFaFMScreen;

impl MoFaFMScreen {
    /// Initialize audio manager and populate device dropdowns
    pub(super) fn init_audio(&mut self, cx: &mut Cx) {
        let mut audio_manager = mofa_ui::AudioManager::new();

        // Load saved preferences
        let prefs = Preferences::load();

        // Get input devices
        let input_devices = audio_manager.get_input_devices();
        let input_labels: Vec<String> = input_devices.iter().map(|d| {
            if d.is_default {
                format!("{} (Default)", d.name)
            } else {
                d.name.clone()
            }
        }).collect();
        self.input_devices = input_devices.iter().map(|d| d.name.clone()).collect();

        // Get output devices
        let output_devices = audio_manager.get_output_devices();
        let output_labels: Vec<String> = output_devices.iter().map(|d| {
            if d.is_default {
                format!("{} (Default)", d.name)
            } else {
                d.name.clone()
            }
        }).collect();
        self.output_devices = output_devices.iter().map(|d| d.name.clone()).collect();

        // Populate input dropdown
        if !input_labels.is_empty() {
            let dropdown = self.view.drop_down(ids!(running_tab_content.audio_container.device_container.device_selectors.input_device_group.input_device_dropdown));
            dropdown.set_labels(cx, input_labels);
            // Restore saved selection or default to first
            let selected_idx = prefs.audio_input_device
                .as_ref()
                .and_then(|saved| self.input_devices.iter().position(|d| d == saved))
                .unwrap_or(0);
            dropdown.set_selected_item(cx, selected_idx);
        }

        // Populate output dropdown
        if !output_labels.is_empty() {
            let dropdown = self.view.drop_down(ids!(running_tab_content.audio_container.device_container.device_selectors.output_device_group.output_device_dropdown));
            dropdown.set_labels(cx, output_labels);
            // Restore saved selection or default to first
            let selected_idx = prefs.audio_output_device
                .as_ref()
                .and_then(|saved| self.output_devices.iter().position(|d| d == saved))
                .unwrap_or(0);
            dropdown.set_selected_item(cx, selected_idx);
        }

        // Start mic monitoring with saved device or default
        let input_device = prefs.audio_input_device.as_deref();
        match audio_manager.start_mic_monitoring(input_device) {
            Ok(()) => {
                ::log::info!("Mic monitoring started for device: {:?}", input_device);
            }
            Err(e) => {
                ::log::error!("Failed to start mic monitoring: {}", e);
            }
        }

        self.audio_manager = Some(audio_manager);

        // Initialize audio player for TTS playback (32kHz for PrimeSpeech)
        match crate::audio_player::create_audio_player(32000) {
            Ok(player) => {
                ::log::info!("Audio player initialized (32kHz)");
                self.audio_player = Some(player);
            }
            Err(e) => {
                ::log::error!("Failed to create audio player: {}", e);
            }
        }

        // Start timer for mic level updates (50ms for smooth visualization)
        self.audio_timer = cx.start_interval(0.05);
        ::log::info!("Audio timer started: {:?}", self.audio_timer);

        // Start dora timer for participant panel updates (needed for audio visualization)
        self.dora_timer = cx.start_interval(0.1);
        ::log::info!("Dora timer started: {:?}", self.dora_timer);

        // Also request next frame to ensure continuous updates
        cx.new_next_frame();

        // AEC enabled by default (blink animation is shader-driven, no timer needed)
        self.aec_enabled = true;

        // Initialize demo log entries
        self.init_demo_logs(cx);

        self.view.redraw(cx);
    }

    /// Initialize log entries with a startup message
    pub(super) fn init_demo_logs(&mut self, cx: &mut Cx) {
        // Start with empty logs - real logs will come from log_bridge
        self.log_entries = std::collections::VecDeque::from(vec![
            "[INFO] [App] MoFA FM initialized".to_string(),
            "[INFO] [App] System log ready - Rust logs will appear here".to_string(),
        ]);

        // Update the log display
        self.update_log_display(cx);
    }

    /// Update mic level LEDs based on current audio input
    pub(super) fn update_mic_level(&mut self, cx: &mut Cx) {
        // Don't show mic level if muted
        if self.mic_muted {
            self.view.led_meter(ids!(running_tab_content.audio_container.audio_controls_row.mic_container.mic_group.mic_level_meter))
                .set_level(cx, 0.0);
            return;
        }

        let level = if let Some(ref audio_manager) = self.audio_manager {
            audio_manager.get_mic_level()
        } else {
            return;
        };

        // Use the LedMeter widget from mofa-ui
        self.view.led_meter(ids!(running_tab_content.audio_container.audio_controls_row.mic_container.mic_group.mic_level_meter))
            .set_level(cx, level);
    }

    /// Update mic level LEDs from dora shared state (AEC input bridge)
    /// This is called from poll_dora_events when mic level changes
    pub(super) fn update_mic_level_from_dora(&mut self, cx: &mut Cx, level: f32) {
        // Don't show mic level if muted
        let display_level = if self.mic_muted { 0.0 } else { level };

        // Use the LedMeter widget from mofa-ui
        self.view.led_meter(ids!(running_tab_content.audio_container.audio_controls_row.mic_container.mic_group.mic_level_meter))
            .set_level(cx, display_level);
    }

    /// Update buffer level LEDs based on audio buffer fill percentage
    pub(super) fn update_buffer_level(&mut self, cx: &mut Cx, level: f64) {
        // Use the LedMeter widget from mofa-ui with blue colors
        let meter = self.view.led_meter(ids!(running_tab_content.audio_container.audio_controls_row.buffer_container.buffer_group.buffer_meter));

        // Set colors based on level thresholds
        let colors = if level >= 0.95 {
            mofa_ui::LedColors::uniform(0.937, 0.267, 0.267) // Red - critical
        } else if level >= 0.8 {
            mofa_ui::LedColors::uniform(0.918, 0.702, 0.031) // Yellow - warning
        } else {
            mofa_ui::LedColors::blue() // Blue - normal
        };

        meter.set_colors(colors);
        meter.set_level(cx, level as f32);

        // Update percentage label
        let pct_text = format!("{}%", (level * 100.0) as u32);
        self.view.label(ids!(running_tab_content.audio_container.audio_controls_row.buffer_container.buffer_group.buffer_pct)).set_text(cx, &pct_text);
    }

    /// Select input device for mic monitoring
    pub(super) fn select_input_device(&mut self, cx: &mut Cx, device_name: &str) {
        ::log::info!("select_input_device: {}", device_name);
        if let Some(ref mut audio_manager) = self.audio_manager {
            if let Err(e) = audio_manager.set_input_device(device_name) {
                ::log::error!("Failed to set input device '{}': {}", device_name, e);
            }
        }

        // Save preference
        let mut prefs = Preferences::load();
        prefs.audio_input_device = Some(device_name.to_string());
        if let Err(e) = prefs.save() {
            eprintln!("Failed to save audio input preference: {}", e);
        }

        self.view.redraw(cx);
    }

    /// Select output device
    pub(super) fn select_output_device(&mut self, device_name: &str) {
        if let Some(ref mut audio_manager) = self.audio_manager {
            audio_manager.set_output_device(device_name);
        }

        // Save preference
        let mut prefs = Preferences::load();
        prefs.audio_output_device = Some(device_name.to_string());
        if let Err(e) = prefs.save() {
            eprintln!("Failed to save audio output preference: {}", e);
        }
    }
}
