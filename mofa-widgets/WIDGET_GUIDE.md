# MoFA Widgets User Guide

Shared reusable UI components for MoFA Studio applications built on the Makepad framework.

## Table of Contents

- [Getting Started](#getting-started)
- [Theme System](#theme-system)
- [WaveformView](#waveformview)
- [ParticipantPanel](#participantpanel)
- [LogPanel](#logpanel)
- [BufferGauge](#buffergauge)
- [ChatBubble](#chatbubble)
- [AudioPlayer](#audioplayer)
- [App Traits](#app-traits)

---

## Getting Started

### Registration

Register all widgets in your app's `LiveRegister`:

```rust
impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        mofa_widgets::live_design(cx);  // Register all mofa widgets
        // ... your app widgets
    }
}
```

### Import in live_design

```rust
live_design! {
    use mofa_widgets::theme::*;
    use mofa_widgets::waveform_view::WaveformView;
    use mofa_widgets::participant_panel::ParticipantPanel;
    use mofa_widgets::log_panel::LogPanel;
    use mofa_widgets::led_gauge::BufferGauge;
    use mofa_widgets::chat_bubble::ChatBubble;
}
```

---

## Theme System

Centralized color palette, fonts, and dark mode support.

### Usage

```rust
live_design! {
    use mofa_widgets::theme::*;

    MyWidget = <View> {
        draw_bg: { color: (PANEL_BG) }
        label = <Label> {
            draw_text: {
                color: (TEXT_PRIMARY)
                text_style: <FONT_REGULAR> { font_size: 12.0 }
            }
        }
    }
}
```

### Semantic Colors (Recommended)

| Constant | Light Mode | Description |
|----------|------------|-------------|
| `DARK_BG` | `#f5f7fa` | Main app background |
| `PANEL_BG` | `#ffffff` | Card/panel background |
| `TEXT_PRIMARY` | `#1f2937` | Main text color |
| `TEXT_SECONDARY` | `#6b7280` | Secondary text |
| `TEXT_MUTED` | `#9ca3af` | Disabled/muted text |
| `ACCENT_BLUE` | `#3b82f6` | Primary action |
| `ACCENT_GREEN` | `#10b981` | Success |
| `ACCENT_RED` | `#ef4444` | Error/danger |
| `BORDER` | `#e5e7eb` | Border color |
| `HOVER_BG` | `#f1f5f9` | Hover state |

### Dark Mode Variants

| Constant | Dark Mode | Description |
|----------|-----------|-------------|
| `DARK_BG_DARK` | `#0f172a` | Main background (dark) |
| `PANEL_BG_DARK` | `#1f293b` | Panel background (dark) |
| `TEXT_PRIMARY_DARK` | `#f1f5f9` | Main text (dark) |
| `TEXT_SECONDARY_DARK` | `#94a3b8` | Secondary text (dark) |
| `BORDER_DARK` | `#334155` | Border (dark) |

### Color Palettes

Full Tailwind-style palettes (50-900 shades):

- `SLATE_*` - Cool gray for backgrounds
- `GRAY_*` - Neutral gray for text
- `BLUE_*`, `INDIGO_*` - Primary colors
- `GREEN_*`, `RED_*`, `AMBER_*` - Status colors

### Fonts

```rust
// Four weights with Chinese + Emoji support
<FONT_REGULAR> { font_size: 12.0 }   // Normal text
<FONT_MEDIUM> { font_size: 12.0 }    // Slightly emphasized
<FONT_SEMIBOLD> { font_size: 14.0 }  // Headings
<FONT_BOLD> { font_size: 16.0 }      // Strong emphasis
```

### Dark Mode in Shaders

```rust
draw_bg: {
    instance dark_mode: 0.0  // 0.0 = light, 1.0 = dark
    fn pixel(self) -> vec4 {
        return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
    }
}
```

Update at runtime:

```rust
widget.apply_over(cx, live!{ draw_bg: { dark_mode: 1.0 } });
```

### Themeable Base Widgets

Pre-built widgets with dark mode support:

```rust
// Auto-themed panel
my_panel = <ThemeableView> {}

// Auto-themed rounded panel
my_card = <ThemeableRoundedView> {}
```

---

## WaveformView

8-band FFT-style frequency bar visualization with smooth animation.

### Features

- 8 rainbow-colored bars (red → orange → yellow → green → cyan → blue → purple → pink)
- Smooth attack/decay animation
- Dark background (SLATE_950)

### Usage

```rust
live_design! {
    use mofa_widgets::waveform_view::WaveformView;

    MyScreen = <View> {
        waveform = <WaveformView> {
            width: 200, height: 100
        }
    }
}
```

### Animation Setup

```rust
// In after_new_from_doc - start refresh timer
cx.start_interval(0.05);  // 20 FPS

// In handle_event - trigger animation frame
if let Event::Timer(_) = event {
    cx.request_next_frame();
}
```

### Updating Band Levels

```rust
self.view.view(ids!(waveform)).apply_over(cx, live!{
    draw_bg: {
        amplitude: 0.5,      // Global multiplier (0.0-1.0)
        band0: 0.3,          // Low frequencies
        band1: 0.5,
        band2: 0.8,
        band3: 0.6,
        band4: 0.5,
        band5: 0.4,
        band6: 0.3,
        band7: 0.2,          // High frequencies
    }
});
```

### Instance Variables

| Variable | Range | Description |
|----------|-------|-------------|
| `anim_time` | auto | Animation time (set by NextFrame) |
| `amplitude` | 0.0-1.0 | Global amplitude multiplier |
| `band0`-`band7` | 0.0-1.0 | Individual frequency band levels |

---

## ParticipantPanel

Composite widget showing participant status in voice chat applications.

### Features

- Status indicator (blue/green/red dot)
- Name label with dark mode support
- 8-band waveform with level bar background

### Usage

```rust
live_design! {
    use mofa_widgets::participant_panel::ParticipantPanel;

    MyScreen = <View> {
        student = <ParticipantPanel> {
            header = { name_label = { text: "Student 1" } }
        }
    }
}
```

### Status Indicator States

| Value | Color | Meaning |
|-------|-------|---------|
| `0.0` | Blue | Waiting/idle |
| `1.0` | Green | Speaking |
| `2.0` | Red | Error |

```rust
// Set status to "speaking"
self.view.view(ids!(participant.header.indicator)).apply_over(cx, live!{
    draw_bg: { status: 1.0 }
});
```

### Updating Waveform

```rust
self.view.view(ids!(participant.waveform)).apply_over(cx, live!{
    draw_bg: {
        active: 1.0,     // Enable waveform (0=hidden, 1=visible)
        level: 0.5,      // Background level bar (0.0-1.0)
        band0: 0.3,
        band1: 0.5,
        band2: 0.8,
        band3: 0.6,
        band4: 0.5,
        band5: 0.4,
        band6: 0.3,
        band7: 0.2,
    }
});
```

### Dark Mode

```rust
use mofa_widgets::participant_panel::ParticipantPanelWidgetExt;

self.ui.participant_panel(ids!(my_participant))
    .update_dark_mode(cx, 1.0);  // 0.0=light, 1.0=dark
```

### Instance Variables

| Variable | Widget | Range | Description |
|----------|--------|-------|-------------|
| `status` | StatusIndicator | 0/1/2 | Blue/Green/Red |
| `dark_mode` | ParticipantPanel | 0.0-1.0 | Theme |
| `level` | ParticipantWaveform | 0.0-1.0 | Level bar fill |
| `active` | ParticipantWaveform | 0/1 | Show/hide bars |
| `band0`-`band7` | ParticipantWaveform | 0.0-1.0 | Band levels |

---

## LogPanel

Scrollable panel for system logs with Markdown rendering.

### Features

- Markdown support (bold, italic, code, etc.)
- Auto-scroll via ScrollYView
- Customizable font size and colors

### Usage

```rust
live_design! {
    use mofa_widgets::log_panel::LogPanel;

    MyScreen = <View> {
        log = <LogPanel> {
            width: Fill, height: 200
        }
    }
}
```

### Updating Content

```rust
// Get markdown widget and set text
let markdown = self.view.markdown(ids!(log.log_scroll.log_content));
markdown.set_text("**Status**: Connected\n\n`12:34:56` Message received");

// For appending, maintain a buffer
self.log_buffer.push_str(&format!("\n{}", new_message));
markdown.set_text(&self.log_buffer);
```

### Dark Mode

```rust
self.view.widget(ids!(log.log_scroll.log_content)).apply_over(cx, live!{
    draw_normal: { color: (vec4(0.95, 0.96, 0.98, 1.0)) }
    draw_bold: { color: (vec4(0.95, 0.96, 0.98, 1.0)) }
});
```

### Widget Structure

```
LogPanel
└── log_scroll (ScrollYView)
    └── log_content (Markdown)
```

---

## BufferGauge

Horizontal bar gauge showing fill level with color change at threshold.

### Features

- Dynamic color (green below 80%, red above)
- Rounded corners
- Subtle border

### Usage

```rust
live_design! {
    use mofa_widgets::led_gauge::BufferGauge;

    MyScreen = <View> {
        buffer = <BufferGauge> {
            width: Fill, height: 40
        }
    }
}
```

### Updating Fill Level

```rust
// Set fill percentage (0.0 = empty, 1.0 = full)
self.view.view(ids!(buffer)).apply_over(cx, live!{
    draw_bg: { fill_pct: 0.65 }  // 65% full
});
self.view.redraw(cx);
```

### Color Behavior

| Fill Level | Color | Meaning |
|------------|-------|---------|
| 0-80% | Green | Normal/safe |
| 80-100% | Red | Warning/critical |

### Custom Colors

Override `get_fill_color` in a derived widget:

```rust
live_design! {
    CustomGauge = <BufferGauge> {
        draw_bg: {
            fn get_fill_color(self, pct: float) -> vec4 {
                if pct > 0.5 {
                    return vec4(0.95, 0.85, 0.2, 1.0);  // Yellow
                } else {
                    return vec4(0.2, 0.8, 0.4, 1.0);    // Green
                }
            }
        }
    }
}
```

---

## ChatBubble

Reusable chat message bubble with role-based styling for user, assistant, and system messages.

### Features

- Role-specific colors and text colors
- Automatic text wrapping
- Configurable padding and margins

### Usage

```rust
live_design! {
    use mofa_widgets::chat_bubble::ChatBubble;

    ChatView = <View> {
        user_bubble = <ChatBubble> {
            role: 0.0,
            text: "Hello, how can I help you?"
        }
        assistant_bubble = <ChatBubble> {
            role: 1.0,
            text: "I'm here to assist with your questions."
        }
        system_bubble = <ChatBubble> {
            role: 2.0,
            text: "System message: Connection established"
        }
    }
}
```

### Updating Content

Set role and text dynamically:

```rust
// Update bubble
self.view.view(ids!(bubble)).apply_over(cx, live!{
    role: 1.0,
    text: "New response"
});
```

### Roles

| Role | Background | Text Color | Description |
|------|------------|------------|-------------|
| User | Blue | White | User messages |
| Assistant | Gray | Dark | AI responses |
| System | Yellow | Dark | System notifications |

---

## AudioPlayer

Thread-safe circular buffer audio playback using cpal.

### Features

- Circular buffer with configurable size (default 60 seconds)
- Configurable sample rate (32kHz for PrimeSpeech, 24kHz for Kokoro)
- Buffer status reporting for backpressure control
- Segment tracking for multi-participant audio

### Usage

```rust
use mofa_widgets::{AudioPlayer, AudioPlayerRef, create_audio_player};

// Create player (32kHz sample rate)
let player: AudioPlayerRef = create_audio_player(32000)?;

// Write audio samples
player.write_audio(&samples, Some(question_id), Some(participant_idx));

// Check buffer status
let fill_pct = player.buffer_fill_percentage();
let seconds = player.buffer_seconds();
let is_playing = player.is_playing();

// Control playback
player.pause();
player.resume();
player.reset();

// Get current playback info
let question_id = player.current_question_id();
let participant = player.current_participant_idx();  // 0=student1, 1=student2, 2=tutor

// Get waveform for visualization
let waveform = player.get_waveform_data(512);
```

### API Reference

| Method | Description |
|--------|-------------|
| `new(sample_rate)` | Create player with sample rate |
| `write_audio(samples, qid, participant)` | Add samples to buffer |
| `buffer_fill_percentage()` | Get fill % (0-100) |
| `buffer_seconds()` | Get available seconds |
| `is_playing()` | Check if playing |
| `pause()` / `resume()` | Control playback |
| `reset()` | Clear buffer (for new question) |
| `current_question_id()` | Get current question ID |
| `current_participant_idx()` | Get current participant |
| `get_waveform_data(n)` | Get n samples for visualization |

---

## App Traits

Interfaces for plugin apps integrating with MoFA Studio shell.

### MofaApp Trait

Standard interface for apps:

```rust
use mofa_widgets::{MofaApp, AppInfo};

pub struct MyApp;

impl MofaApp for MyApp {
    fn info() -> AppInfo {
        AppInfo {
            name: "My App",
            id: "my-app",
            description: "My awesome app",
        }
    }

    fn live_design(cx: &mut Cx) {
        crate::screen::live_design(cx);
    }
}
```

### TimerControl Trait

For widgets with timer-based animations:

```rust
use mofa_widgets::TimerControl;

impl TimerControl for MyWidgetRef {
    fn stop_timers(&self, cx: &mut Cx) {
        if let Some(inner) = self.borrow_mut() {
            cx.stop_timer(inner.my_timer);
        }
    }

    fn start_timers(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.my_timer = cx.start_interval(0.05);
        }
    }
}
```

**Why use TimerControl?**

- Prevents resource waste on hidden widgets
- Avoids stale timer callbacks
- Makepad doesn't auto-cleanup timers

### StateChangeListener Trait

For widgets responding to global state changes:

```rust
use mofa_widgets::StateChangeListener;

impl StateChangeListener for MyWidgetRef {
    fn on_dark_mode_change(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.view.apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
        }
    }
}
```

### AppRegistry

Runtime registry for app metadata:

```rust
use mofa_widgets::AppRegistry;

// In App struct
#[rust]
app_registry: AppRegistry,

// Register apps
self.app_registry.register(MyApp::info());

// Query apps
let apps = self.app_registry.apps();
let app = self.app_registry.find_by_id("my-app");
```

---

## NextFrame Animation Pattern

For smooth animations without timers, use the `NextFrame` event pattern. This is used for copy button feedback animations.

### Basic Pattern

```rust
// State variables
#[rust]
flash_active: bool,
#[rust]
flash_start: f64,  // Absolute start time (0.0 = capture on first frame)

// Click handler - trigger animation
Hit::FingerUp(_) => {
    self.flash_active = true;
    self.flash_start = 0.0;  // Sentinel value
    cx.new_next_frame();
    self.view.redraw(cx);
}

// NextFrame handler - animate
if let Event::NextFrame(nf) = event {
    if self.flash_active {
        // Capture start time on first frame
        if self.flash_start == 0.0 {
            self.flash_start = nf.time;
        }
        let elapsed = nf.time - self.flash_start;

        // Animation logic here...

        if self.flash_active {
            cx.new_next_frame();  // Continue animation
        }
    }
}
```

### Color Gradient Animation

Multi-stop color gradient with smoothstep fade:

```rust
draw_bg: {
    instance copied: 0.0
    instance dark_mode: 0.0
    fn pixel(self) -> vec4 {
        // Light theme: Green → Teal → Blue → Gray
        let gray_light = (BORDER);
        let blue_light = vec4(0.231, 0.510, 0.965, 1.0);
        let teal_light = vec4(0.078, 0.722, 0.651, 1.0);
        let green_light = vec4(0.133, 0.773, 0.373, 1.0);

        // Dark theme: Bright Green → Cyan → Purple → Slate
        let gray_dark = vec4(0.334, 0.371, 0.451, 1.0);
        let purple_dark = vec4(0.639, 0.380, 0.957, 1.0);
        let cyan_dark = vec4(0.133, 0.831, 0.894, 1.0);
        let green_dark = vec4(0.290, 0.949, 0.424, 1.0);

        // Select colors based on dark mode
        let gray = mix(gray_light, gray_dark, self.dark_mode);
        let c1 = mix(blue_light, purple_dark, self.dark_mode);
        let c2 = mix(teal_light, cyan_dark, self.dark_mode);
        let c3 = mix(green_light, green_dark, self.dark_mode);

        // Multi-stop gradient: copied 1.0→0.66→0.33→0.0
        let t = self.copied;
        let bg_color = mix(
            mix(mix(gray, c1, clamp(t * 3.0, 0.0, 1.0)),
                c2, clamp((t - 0.33) * 3.0, 0.0, 1.0)),
            c3, clamp((t - 0.66) * 3.0, 0.0, 1.0)
        );
        // ... draw with bg_color
    }
}
```

### Smoothstep Fade

```rust
// Hold for 0.3s, then fade over 0.5s
let fade_start = 0.3;
let fade_duration = 0.5;

if elapsed >= fade_start + fade_duration {
    // Animation complete
    self.flash_active = false;
} else if elapsed >= fade_start {
    // Smoothstep: 3t² - 2t³ for smooth ease-out
    let t = (elapsed - fade_start) / fade_duration;
    let smooth_t = t * t * (3.0 - 2.0 * t);
    let value = 1.0 - smooth_t;
    // Apply interpolated value to shader instance
}
```

### Key Points

- `nf.time` is **absolute time**, not delta - track start time separately
- Use `cx.new_next_frame()` to request next frame (not `request_next_frame`)
- Animation is self-terminating: stops requesting frames when complete
- Use `smoothstep()` or manual `3t² - 2t³` for smooth easing

---

## Important Notes

### Hex Colors in Shaders

Theme constants like `(ACCENT_BLUE)` work in `live_design!{}` properties but **NOT** inside shader `fn pixel()` functions. Use `vec4()` literals for shader code:

```rust
// In live_design property - OK
draw_bg: { color: (ACCENT_BLUE) }

// In shader function - use vec4()
fn pixel(self) -> vec4 {
    let blue = vec4(0.231, 0.510, 0.965, 1.0);  // #3b82f6
    return blue;
}
```

### Lexer Issues

Some hex values are adjusted to avoid Rust lexer conflicts:

- `#1e293b` → `#1f293b` (because `1e` looks like scientific notation)
- `#4ade80` → `#4adf80` (because `de` is a digit sequence)

### Widget Ref Extension Pattern

Access widget refs using the generated `*WidgetExt` traits:

```rust
use mofa_widgets::participant_panel::ParticipantPanelWidgetExt;

// Get widget ref
let panel = self.ui.participant_panel(ids!(my_panel));
panel.update_dark_mode(cx, 1.0);
```
