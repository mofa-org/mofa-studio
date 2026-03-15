//! LED Level Meter Widget
//!
//! A 5-LED horizontal level meter for audio visualization.
//! Supports configurable colors with automatic thresholds.
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_ui::widgets::led_meter::*;
//!
//!     mic_meter = <LedMeter> {
//!         // Default: green, green, yellow, orange, red
//!     }
//! }
//! ```
//!
//! ## Updating Level
//!
//! ```rust,ignore
//! // Set level (0.0 to 1.0)
//! let meter = self.view.led_meter(id!(mic_meter));
//! meter.set_level(cx, 0.6);
//! ```

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Individual LED component
    Led = <RoundedView> {
        width: 8
        height: 14
        show_bg: true
        draw_bg: {
            instance active: 0.0
            instance dark_mode: 0.0
            instance color_r: 0.133
            instance color_g: 0.773
            instance color_b: 0.373
            border_radius: 2.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);

                let on_color = vec4(self.color_r, self.color_g, self.color_b, 1.0);
                let off_color = mix(
                    vec4(0.886, 0.910, 0.941, 1.0),  // LED_OFF light
                    vec4(0.278, 0.337, 0.412, 1.0),  // LED_OFF dark
                    self.dark_mode
                );

                sdf.fill(mix(off_color, on_color, self.active));
                return sdf.result;
            }
        }
    }

    /// 5-LED horizontal level meter
    pub LedMeter = {{LedMeter}} {
        width: Fit
        height: Fit
        flow: Right
        spacing: 3
        align: {y: 0.5}
        padding: {top: 2, bottom: 2}

        led_1 = <Led> {}
        led_2 = <Led> {}
        led_3 = <Led> {}
        led_4 = <Led> {}
        led_5 = <Led> {}
    }
}

/// LED color configuration for the meter
#[derive(Clone, Copy, Debug)]
pub struct LedColors {
    /// RGB for LED 1 (lowest level)
    pub led_1: (f32, f32, f32),
    /// RGB for LED 2
    pub led_2: (f32, f32, f32),
    /// RGB for LED 3
    pub led_3: (f32, f32, f32),
    /// RGB for LED 4
    pub led_4: (f32, f32, f32),
    /// RGB for LED 5 (highest level)
    pub led_5: (f32, f32, f32),
}

impl Default for LedColors {
    fn default() -> Self {
        // Default: green, green, yellow, orange, red
        Self {
            led_1: (0.133, 0.773, 0.373), // Green
            led_2: (0.133, 0.773, 0.373), // Green
            led_3: (0.918, 0.702, 0.031), // Yellow
            led_4: (0.976, 0.451, 0.086), // Orange
            led_5: (0.937, 0.267, 0.267), // Red
        }
    }
}

impl LedColors {
    /// All LEDs the same color (e.g., for buffer level)
    pub fn uniform(r: f32, g: f32, b: f32) -> Self {
        Self {
            led_1: (r, g, b),
            led_2: (r, g, b),
            led_3: (r, g, b),
            led_4: (r, g, b),
            led_5: (r, g, b),
        }
    }

    /// Blue uniform color (for buffer indicators)
    pub fn blue() -> Self {
        Self::uniform(0.23, 0.51, 0.97)
    }

    /// Get color for LED index (0-4)
    pub fn get(&self, index: usize) -> (f32, f32, f32) {
        match index {
            0 => self.led_1,
            1 => self.led_2,
            2 => self.led_3,
            3 => self.led_4,
            _ => self.led_5,
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct LedMeter {
    #[deref]
    view: View,

    /// Current level (0.0 to 1.0)
    #[rust]
    level: f32,

    /// LED colors configuration
    #[rust]
    colors: LedColors,

    /// Current dark mode value
    #[rust]
    dark_mode: f64,
}

impl Widget for LedMeter {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl LedMeter {
    /// Set the level (0.0 to 1.0) and update LED display
    pub fn set_level(&mut self, cx: &mut Cx, level: f32) {
        self.level = level.clamp(0.0, 1.0);
        self.update_leds(cx);
    }

    /// Set LED colors configuration
    pub fn set_colors(&mut self, colors: LedColors) {
        self.colors = colors;
    }

    /// Apply dark mode to the meter
    pub fn apply_dark_mode(&mut self, cx: &mut Cx, dark_mode: f64) {
        self.dark_mode = dark_mode;
        self.update_leds(cx);
    }

    /// Update LED states based on current level
    fn update_leds(&mut self, cx: &mut Cx) {
        // Map level to active LED count (with amplification for visibility)
        let scaled_level = (self.level * 3.0).min(1.0);
        let active_count = (scaled_level * 5.0).ceil() as usize;

        // Apply to each LED
        for i in 0..5 {
            let is_active = i < active_count;
            let (r, g, b) = self.colors.get(i);
            let active_val = if is_active { 1.0 } else { 0.0 };

            let led_view = match i {
                0 => self.view.view(ids!(led_1)),
                1 => self.view.view(ids!(led_2)),
                2 => self.view.view(ids!(led_3)),
                3 => self.view.view(ids!(led_4)),
                _ => self.view.view(ids!(led_5)),
            };

            led_view.apply_over(
                cx,
                live! {
                    draw_bg: {
                        active: (active_val),
                        dark_mode: (self.dark_mode),
                        color_r: (r as f64),
                        color_g: (g as f64),
                        color_b: (b as f64),
                    }
                },
            );
        }

        self.view.redraw(cx);
    }
}

impl LedMeterRef {
    /// Set the level (0.0 to 1.0)
    pub fn set_level(&self, cx: &mut Cx, level: f32) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_level(cx, level);
        }
    }

    /// Set LED colors
    pub fn set_colors(&self, colors: LedColors) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_colors(colors);
        }
    }

    /// Apply dark mode
    pub fn apply_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.apply_dark_mode(cx, dark_mode);
        }
    }

    /// Get current level
    pub fn level(&self) -> f32 {
        self.borrow().map(|inner| inner.level).unwrap_or(0.0)
    }
}
