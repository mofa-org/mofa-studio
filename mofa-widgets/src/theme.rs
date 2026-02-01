//! # Theme System
//!
//! Centralized color palette, fonts, and dark mode support for MoFA Studio.
//!
//! ## Usage
//!
//! Import the theme in your `live_design!` macro:
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_widgets::theme::*;
//!
//!     MyWidget = <View> {
//!         draw_bg: { color: (PANEL_BG) }
//!         label = <Label> {
//!             draw_text: {
//!                 color: (TEXT_PRIMARY)
//!                 text_style: <FONT_REGULAR> { font_size: 12.0 }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Color Categories
//!
//! ### Semantic Colors (Recommended)
//! Use these for consistent theming:
//! - `DARK_BG` - Main app background
//! - `PANEL_BG` / `PANEL_BG_DARK` - Card/panel backgrounds
//! - `TEXT_PRIMARY` / `TEXT_PRIMARY_DARK` - Main text
//! - `TEXT_SECONDARY` / `TEXT_SECONDARY_DARK` - Muted text
//! - `ACCENT_BLUE`, `ACCENT_GREEN`, `ACCENT_RED` - Action colors
//! - `BORDER` / `BORDER_DARK` - Borders and dividers
//! - `HOVER_BG` / `HOVER_BG_DARK` - Hover states
//!
//! ### Color Palettes
//! Full Tailwind-style palettes (50-900 shades):
//! - `SLATE_*` - Cool gray for backgrounds
//! - `GRAY_*` - Neutral gray for text
//! - `BLUE_*`, `INDIGO_*` - Primary colors
//! - `GREEN_*`, `RED_*`, `AMBER_*` - Status colors
//!
//! ## Dark Mode
//!
//! Widgets support dark mode via shader instance variables:
//!
//! ```rust,ignore
//! draw_bg: {
//!     instance dark_mode: 0.0  // 0.0 = light, 1.0 = dark
//!     fn pixel(self) -> vec4 {
//!         return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
//!     }
//! }
//! ```
//!
//! Update at runtime via `apply_over`:
//! ```rust,ignore
//! widget.apply_over(cx, live!{ draw_bg: { dark_mode: 1.0 } });
//! ```
//!
//! ## Fonts
//!
//! Four font weights with Chinese and Emoji support:
//! - `FONT_REGULAR` - Normal text
//! - `FONT_MEDIUM` - Slightly emphasized
//! - `FONT_SEMIBOLD` - Headings
//! - `FONT_BOLD` - Strong emphasis
//!
//! ## Important Notes
//!
//! - **Hex colors in shaders**: Theme constants like `(ACCENT_BLUE)` work in
//!   `live_design!{}` properties but NOT inside shader `fn pixel()` functions.
//!   Use `vec4()` literals for shader code.
//!
//! - **Lexer issues**: Some hex values are adjusted to avoid Rust lexer conflicts
//!   (e.g., `#1e293b` → `#1f293b` because `1e` looks like scientific notation).

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Font definitions with Chinese and Emoji support
    pub FONT_REGULAR = {
        font_family: {
            latin = font("crate://self/resources/Manrope-Regular.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_regular/resources/LXGWWenKaiRegular.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }
    pub FONT_MEDIUM = {
        font_family: {
            latin = font("crate://self/resources/Manrope-Medium.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_regular/resources/LXGWWenKaiRegular.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }
    pub FONT_SEMIBOLD = {
        font_family: {
            latin = font("crate://self/resources/Manrope-SemiBold.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_bold/resources/LXGWWenKaiBold.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }
    pub FONT_BOLD = {
        font_family: {
            latin = font("crate://self/resources/Manrope-Bold.ttf", 0.0, 0.0),
            chinese = font("crate://makepad-widgets/fonts/chinese_bold/resources/LXGWWenKaiBold.ttf", 0.0, 0.0),
            emoji = font("crate://makepad_fonts_emoji/resources/NotoColorEmoji.ttf", 0.0, 0.0),
        }
    }

    // ========================================================================
    // COLOR PALETTE
    // Based on Tailwind CSS color system for consistency
    // ========================================================================

    // --- Semantic Colors (use these first) ---
    pub DARK_BG = #f5f7fa          // Main background
    pub PANEL_BG = #ffffff         // Card/panel background
    pub ACCENT_BLUE = #3b82f6      // Primary action color
    pub ACCENT_GREEN = #10b981     // Success/positive
    pub ACCENT_RED = #ef4444       // Error/danger
    pub ACCENT_YELLOW = #f59f0b    // Warning (adjusted from #f59e0b)
    pub ACCENT_INDIGO = #6366f1    // Secondary accent
    pub TEXT_PRIMARY = #1f2937     // Main text (gray-800)
    pub TEXT_SECONDARY = #6b7280   // Secondary text (gray-500)
    pub TEXT_TERTIARY = #9ca3af    // Tertiary text (gray-400)
    pub TEXT_MUTED = #9ca3af       // Muted/disabled text (gray-400)
    pub DIVIDER = #e2e8f0          // Divider lines (slate-200)
    pub BORDER = #e5e7eb           // Border color (gray-200)
    pub HOVER_BG = #f1f5f9         // Hover background (slate-100)
    pub SURFACE = #ffffff           // Surface background (white)
    pub SURFACE_HOVER = #f8fafc     // Surface hover (slate-50)

    // --- Primary Color Palette (based on Blue) ---
    pub PRIMARY_50 = #eff6ff
    pub PRIMARY_100 = #dbebfe       // Adjusted from #dbeafe
    pub PRIMARY_200 = #bfdbfe
    pub PRIMARY_300 = #93c5fd
    pub PRIMARY_400 = #60a5fa
    pub PRIMARY_500 = #3b82f6
    pub PRIMARY_600 = #2565fb       // Adjusted to avoid digit+e pattern
    pub PRIMARY_700 = #1d4fd8       // Adjusted from #1d4ed8
    pub PRIMARY_800 = #1f40af       // Adjusted from #1e40af
    pub PRIMARY_900 = #1f3a8a       // Adjusted from #1e3a8a

    // --- White ---
    pub WHITE = #ffffff

    // --- Slate (cool gray, used for backgrounds) ---
    pub SLATE_50 = #f8fafc
    pub SLATE_100 = #f1f5f9
    pub SLATE_200 = #e2e8f0
    pub SLATE_300 = #cbd5e1
    pub SLATE_400 = #94a3b8
    pub SLATE_500 = #64748b
    pub SLATE_600 = #475569
    pub SLATE_700 = #334155
    pub SLATE_800 = #1f293b        // Adjusted from #1e293b (lexer issue with 1e)
    pub SLATE_900 = #0f172a
    pub SLATE_950 = #0d1117        // Extra dark (waveform background)

    // --- Gray (neutral gray, used for text/icons) ---
    pub GRAY_50 = #f9fafb
    pub GRAY_100 = #f3f4f6
    pub GRAY_200 = #e5e7eb
    pub GRAY_300 = #d1d5db
    pub GRAY_400 = #9ca3af
    pub GRAY_500 = #6b7280
    pub GRAY_600 = #4b5563
    pub GRAY_700 = #374151
    pub GRAY_800 = #1f2937
    pub GRAY_900 = #111827

    // --- Blue (primary actions) ---
    pub BLUE_50 = #eff6ff
    pub BLUE_100 = #dbeafe
    pub BLUE_200 = #bfdbfe
    pub BLUE_300 = #93c5fd
    pub BLUE_400 = #60a5fa
    pub BLUE_500 = #3b82f6
    pub BLUE_600 = #2565fb      // Adjusted to avoid digit+e pattern
    pub BLUE_700 = #1d4fd8      // Adjusted from #1d4ed8 (lexer issue with 4e)
    pub BLUE_800 = #1f40af      // Adjusted from #1e40af (lexer issue with 1e)
    pub BLUE_900 = #1f3a8a      // Adjusted from #1e3a8a (lexer issue with 1e)

    // --- Indigo (secondary accent) ---
    pub INDIGO_50 = #eef2ff
    pub INDIGO_100 = #e1e7ff      // Adjusted from #e0e7ff (lexer issue with 0e)
    pub INDIGO_200 = #c7d2ff      // Adjusted from #c7d2fe (lexer issue with fe)
    pub INDIGO_300 = #a5b4fc
    pub INDIGO_400 = #818cf8
    pub INDIGO_500 = #6366f1
    pub INDIGO_600 = #4f47e5      // Adjusted from #4f46e5 (lexer issue with 6e)
    pub INDIGO_700 = #4338ca
    pub INDIGO_800 = #3730a3
    pub INDIGO_900 = #312f81      // Adjusted from #312e81 (lexer issue with 2e)

    // --- Green (success) ---
    pub GREEN_50 = #f0fdf4
    pub GREEN_100 = #dcfcf7      // Adjusted from #dcfce7 (lexer issue with ce)
    pub GREEN_200 = #bbf7d0
    pub GREEN_300 = #88ffac      // Adjusted to avoid digit+e pattern
    pub GREEN_400 = #4adf80      // Adjusted from #4ade80 (lexer issue with de)
    pub GREEN_500 = #22c55f      // Adjusted from #22c55e (lexer issue with 5e)
    pub GREEN_600 = #16a34a
    pub GREEN_700 = #15803d
    pub GREEN_800 = #166534
    pub GREEN_900 = #14532d

    // --- Emerald (alternate green) ---
    pub EMERALD_500 = #10b981
    pub EMERALD_600 = #059669
    pub EMERALD_700 = #047857

    // --- Red (error/danger) ---
    pub RED_50 = #fff2f2        // Adjusted from #fef2f2 (lexer issue with ef)
    pub RED_100 = #fff2f2       // Adjusted from #fee2e2 (lexer issue with ee)
    pub RED_200 = #ffcaca       // Adjusted from #fecaca (lexer issue with ec)
    pub RED_300 = #fca5a5
    pub RED_400 = #f87171
    pub RED_500 = #ef4444
    pub RED_600 = #dc2626
    pub RED_700 = #b91c1c
    pub RED_800 = #991b1b
    pub RED_900 = #7f1d1d

    // --- Yellow/Amber (warning) ---
    pub YELLOW_500 = #eab308
    pub AMBER_500 = #f59f0b        // Adjusted from #f59e0b (lexer issue with 9e)

    // --- Orange ---
    pub ORANGE_500 = #f97316

    // --- Transparent ---
    pub TRANSPARENT = #00000000

    // ========================================================================
    // DARK THEME VARIANTS
    // Use with mix(LIGHT_COLOR, DARK_COLOR, dark_mode) in shaders
    // ========================================================================

    // --- Dark Theme Semantic Colors ---
    pub DARK_BG_DARK = #0f172a         // Main background (dark)
    pub PANEL_BG_DARK = #1f293b        // Card/panel background (dark) - adjusted from #1e293b
    pub TEXT_PRIMARY_DARK = #f1f5f9    // Main text (dark)
    pub TEXT_SECONDARY_DARK = #94a3b8  // Secondary text (dark)
    pub TEXT_TERTIARY_DARK = #64748b   // Tertiary text (dark)
    pub TEXT_MUTED_DARK = #64748b      // Muted text (dark)
    pub DIVIDER_DARK = #475569         // Divider lines (dark)
    pub BORDER_DARK = #334155          // Border color (dark)
    pub HOVER_BG_DARK = #334155        // Hover background (dark)
    pub ACCENT_BLUE_DARK = #60a5fa     // Primary action (brighter for dark mode)
    pub SURFACE_DARK = #1f293b         // Surface background (dark) - adjusted from #1e293b
    pub SURFACE_HOVER_DARK = #334155   // Surface hover (dark)

    // ========================================================================
    // THEMEABLE WIDGET BASE
    // Base widget with dark_mode instance for theme switching
    // ========================================================================

    pub ThemeableView = <View> {
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0

            fn get_bg_color(self) -> vec4 {
                return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
            }

            fn pixel(self) -> vec4 {
                return self.get_bg_color();
            }
        }
    }

    pub ThemeableRoundedView = <RoundedView> {
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            border_radius: 4.0

            fn get_bg_color(self) -> vec4 {
                return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
            }
        }
    }
}

// live_design function is generated by the live_design! macro above
