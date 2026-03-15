//! Runtime Theme State for MoFA UI
//!
//! This module provides runtime theme management complementing the
//! static `live_design!` color constants in `mofa-widgets/src/theme.rs`.
//!
//! ## Architecture
//!
//! There are two theme systems that work together:
//!
//! 1. **Static Theme (mofa-widgets)**: Color constants (`SLATE_500`, `PANEL_BG`, etc.)
//!    defined in `live_design!` macros for use in widget definitions.
//!
//! 2. **Runtime Theme (this module)**: `MofaTheme` struct for managing:
//!    - Current dark mode state
//!    - Animated transitions between light/dark
//!    - Accent color customization (future)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use mofa_ui::MofaTheme;
//!
//! let mut theme = MofaTheme::default(); // Light mode
//!
//! // Toggle dark mode
//! theme.toggle();
//!
//! // Check state
//! assert!(theme.is_dark());
//!
//! // Get animation value for shaders
//! let dm = theme.dark_mode_anim; // 0.0 (light) to 1.0 (dark)
//!
//! // Apply to widget
//! widget.apply_over(cx, live! {
//!     draw_bg: { dark_mode: (dm) }
//! });
//! ```
//!
//! ## Animation Support
//!
//! For smooth theme transitions, use `update_animation`:
//!
//! ```rust,ignore
//! // On toggle
//! theme.toggle();
//! let anim_start = Cx::time_now();
//!
//! // In NextFrame handler
//! let elapsed = Cx::time_now() - anim_start;
//! if theme.update_animation(elapsed, 0.25) {
//!     // Animation in progress
//!     apply_theme(cx, theme.dark_mode_anim);
//!     cx.new_next_frame();
//! } else {
//!     // Animation complete
//! }
//! ```

/// Duration of theme transition animation in seconds
pub const THEME_TRANSITION_DURATION: f64 = 0.25;

/// Runtime theme state for MoFA UI applications.
///
/// Manages dark mode state and provides smooth animated transitions
/// between light and dark themes.
#[derive(Clone, Debug)]
pub struct MofaTheme {
    /// Whether dark mode is enabled
    pub dark_mode: bool,

    /// Animation value (0.0 = light, 1.0 = dark)
    /// Use this value in shader `dark_mode` instance variables.
    pub dark_mode_anim: f64,

    /// Accent color (future use)
    pub accent_color: ThemeColor,
}

impl MofaTheme {
    /// Create a new theme in light mode
    pub fn new() -> Self {
        Self {
            dark_mode: false,
            dark_mode_anim: 0.0,
            accent_color: ThemeColor::Blue,
        }
    }

    /// Create a theme with specified dark mode state
    pub fn with_dark_mode(dark: bool) -> Self {
        Self {
            dark_mode: dark,
            dark_mode_anim: if dark { 1.0 } else { 0.0 },
            accent_color: ThemeColor::Blue,
        }
    }

    /// Check if dark mode is enabled
    pub fn is_dark(&self) -> bool {
        self.dark_mode
    }

    /// Toggle dark mode
    ///
    /// Note: This only toggles the state. For animated transitions,
    /// use `start_animation()` and `update_animation()`.
    pub fn toggle(&mut self) {
        self.dark_mode = !self.dark_mode;
    }

    /// Set dark mode state immediately (no animation)
    pub fn set_dark_mode(&mut self, dark: bool) {
        self.dark_mode = dark;
        self.dark_mode_anim = if dark { 1.0 } else { 0.0 };
    }

    /// Update animation value based on elapsed time.
    ///
    /// Returns `true` if animation is still in progress, `false` when complete.
    ///
    /// # Arguments
    /// * `elapsed` - Time elapsed since animation started (seconds)
    /// * `duration` - Total animation duration (seconds)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Start animation
    /// let start_time = Cx::time_now();
    ///
    /// // In NextFrame handler
    /// let elapsed = Cx::time_now() - start_time;
    /// if theme.update_animation(elapsed, THEME_TRANSITION_DURATION) {
    ///     // Animation in progress - apply theme and request next frame
    ///     apply_theme(cx, theme.dark_mode_anim);
    ///     cx.new_next_frame();
    /// } else {
    ///     // Animation complete
    /// }
    /// ```
    pub fn update_animation(&mut self, elapsed: f64, duration: f64) -> bool {
        let target = if self.dark_mode { 1.0 } else { 0.0 };

        if elapsed >= duration {
            // Animation complete
            self.dark_mode_anim = target;
            false
        } else {
            // Ease-out cubic for smooth deceleration
            let t = (elapsed / duration).min(1.0);
            let ease_t = 1.0 - (1.0 - t).powi(3);

            // Interpolate from current to target
            let start = if self.dark_mode { 0.0 } else { 1.0 };
            self.dark_mode_anim = start + (target - start) * ease_t;

            true
        }
    }

    /// Get the target animation value (what dark_mode_anim should be when animation completes)
    pub fn target_value(&self) -> f64 {
        if self.dark_mode {
            1.0
        } else {
            0.0
        }
    }
}

impl Default for MofaTheme {
    fn default() -> Self {
        Self::new()
    }
}

/// Accent color options for theming
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeColor {
    Blue,
    Indigo,
    Green,
    Red,
    Amber,
    Custom(u32), // RGBA as u32
}

impl ThemeColor {
    /// Convert to RGB values (0.0-1.0)
    pub fn to_rgb(&self) -> (f32, f32, f32) {
        match self {
            ThemeColor::Blue => (0.231, 0.510, 0.965),   // #3b82f6
            ThemeColor::Indigo => (0.388, 0.400, 0.945), // #6366f1
            ThemeColor::Green => (0.063, 0.725, 0.506),  // #10b981
            ThemeColor::Red => (0.937, 0.267, 0.267),    // #ef4444
            ThemeColor::Amber => (0.961, 0.620, 0.043),  // #f59e0b
            ThemeColor::Custom(rgba) => {
                let r = ((rgba >> 24) & 0xFF) as f32 / 255.0;
                let g = ((rgba >> 16) & 0xFF) as f32 / 255.0;
                let b = ((rgba >> 8) & 0xFF) as f32 / 255.0;
                (r, g, b)
            }
        }
    }
}

impl Default for ThemeColor {
    fn default() -> Self {
        ThemeColor::Blue
    }
}

/// Trait for widgets that respond to theme changes
pub trait ThemeListener {
    /// Apply dark mode value to the widget
    ///
    /// # Arguments
    /// * `cx` - Makepad context for applying UI updates
    /// * `dark_mode` - Dark mode animation value (0.0 = light, 1.0 = dark)
    fn apply_dark_mode(&self, cx: &mut makepad_widgets::Cx, dark_mode: f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_default() {
        let theme = MofaTheme::default();
        assert!(!theme.is_dark());
        assert_eq!(theme.dark_mode_anim, 0.0);
    }

    #[test]
    fn test_theme_toggle() {
        let mut theme = MofaTheme::default();

        theme.toggle();
        assert!(theme.is_dark());

        theme.toggle();
        assert!(!theme.is_dark());
    }

    #[test]
    fn test_theme_set_dark_mode() {
        let mut theme = MofaTheme::default();

        theme.set_dark_mode(true);
        assert!(theme.is_dark());
        assert_eq!(theme.dark_mode_anim, 1.0);

        theme.set_dark_mode(false);
        assert!(!theme.is_dark());
        assert_eq!(theme.dark_mode_anim, 0.0);
    }

    #[test]
    fn test_theme_animation() {
        let mut theme = MofaTheme::default();
        theme.toggle(); // Switch to dark

        // Animation should be in progress at 50%
        let in_progress = theme.update_animation(0.125, 0.25);
        assert!(in_progress);
        assert!(theme.dark_mode_anim > 0.0);
        assert!(theme.dark_mode_anim < 1.0);

        // Animation should be complete at 100%
        let in_progress = theme.update_animation(0.25, 0.25);
        assert!(!in_progress);
        assert_eq!(theme.dark_mode_anim, 1.0);
    }

    #[test]
    fn test_theme_color_rgb() {
        let (r, g, b) = ThemeColor::Blue.to_rgb();
        assert!(r > 0.2 && r < 0.3);
        assert!(g > 0.5 && g < 0.6);
        assert!(b > 0.9 && b < 1.0);
    }
}
