//! MoFA FM Screen UI Design
//!
//! Contains the live_design! DSL block defining the UI layout and styling.

use makepad_widgets::*;

// Import widget types from mofa-ui for Rust code (WidgetExt traits)
// Note: Live design uses inline definitions due to Makepad parser limitations
use mofa_ui::{AecButton, LedMeter, MicButton};

use super::MoFaFMScreen;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::*;
    use mofa_widgets::participant_panel::ParticipantPanel;
    use mofa_widgets::log_panel::LogPanel;
    use mofa_ui::widgets::mofa_hero::MofaHero;

    // Local layout constants (colors imported from theme)
    SECTION_SPACING = 12.0
    PANEL_RADIUS = 4.0
    PANEL_PADDING = 12.0

    // Individual LED component for level meters
    // Note: Inline definition required due to Makepad parser issues with shared widgets
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

    // 5-LED horizontal level meter
    LedMeter = {{LedMeter}} {
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

    // Microphone toggle button with on/off icons and recording indicator
    MicButton = {{MicButton}} {
        width: Fit
        height: Fit
        flow: Overlay
        cursor: Hand
        padding: 4

        // Background with recording indicator (pulsing when active)
        show_bg: true
        draw_bg: {
            instance recording: 0.0  // 1.0 = recording (not muted), 0.0 = muted

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);

                // Recording indicator: pulsing red dot in top-right when active
                let dot_x = self.rect_size.x - 6.0;
                let dot_y = 4.0;
                let dot_radius = 3.0;
                let dist = length(self.pos * self.rect_size - vec2(dot_x, dot_y));

                // Pulse animation (only when recording)
                let pulse = (sin(self.time * 4.0) * 0.3 + 0.7) * self.recording;
                let red = vec4(0.937, 0.267, 0.267, 1.0);

                // Draw pulsing red dot when recording
                if dist < dot_radius && self.recording > 0.5 {
                    sdf.fill(mix(red, vec4(1.0, 0.4, 0.4, 1.0), pulse));
                } else {
                    sdf.fill(vec4(0.0, 0.0, 0.0, 0.0));  // Transparent background
                }

                return sdf.result;
            }
        }

        mic_icon_on = <View> {
            width: Fit, height: Fit
            icon = <Icon> {
                draw_icon: {
                    instance dark_mode: 0.0
                    svg_file: dep("crate://self/resources/icons/mic.svg")
                    fn get_color(self) -> vec4 {
                        return mix(
                            vec4(0.392, 0.455, 0.545, 1.0),  // SLATE_500
                            vec4(1.0, 1.0, 1.0, 1.0),        // WHITE
                            self.dark_mode
                        );
                    }
                }
                icon_walk: {width: 20, height: 20}
            }
        }

        mic_icon_off = <View> {
            width: Fit, height: Fit
            visible: false
            <Icon> {
                draw_icon: {
                    svg_file: dep("crate://self/resources/icons/mic-off.svg")
                    fn get_color(self) -> vec4 {
                        return vec4(0.937, 0.267, 0.267, 1.0);  // ACCENT_RED
                    }
                }
                icon_walk: {width: 20, height: 20}
            }
        }
    }

    // AEC toggle button with animated speaking indicator
    AecButton = {{AecButton}} {
        width: Fit
        height: Fit
        padding: 6
        cursor: Hand
        show_bg: true

        draw_bg: {
            instance enabled: 0.0   // 0.0=off (muted), 1.0=on (recording) - matches Rust default
            instance speaking: 0.0  // 1.0=voice detected, 0.0=silent

            // VAD indicator: red when speaking, green when enabled but silent, gray when disabled
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);

                let red = vec4(0.9, 0.2, 0.2, 1.0);        // Speaking color
                let bright_red = vec4(1.0, 0.3, 0.3, 1.0); // Speaking pulse
                let green = vec4(0.133, 0.773, 0.373, 1.0); // Enabled, silent
                let bright_green = vec4(0.2, 0.9, 0.5, 1.0);
                let gray = vec4(0.667, 0.686, 0.725, 1.0);  // Disabled

                // Fast pulse when speaking (4x speed)
                let speak_pulse = step(0.0, sin(self.time * 8.0)) * self.speaking;
                // Slow pulse when enabled but not speaking
                let idle_pulse = step(0.0, sin(self.time * 2.0)) * self.enabled * (1.0 - self.speaking);

                // Base color: gray (disabled) -> green (enabled) -> red (speaking)
                let base = mix(gray, green, self.enabled);
                let base = mix(base, red, self.speaking * self.enabled);

                // Pulse color
                let pulse_color = mix(bright_green, bright_red, self.speaking);
                let col = mix(base, pulse_color, (speak_pulse + idle_pulse) * 0.5);

                sdf.fill(col);
                return sdf.result;
            }
        }

        align: {x: 0.5, y: 0.5}

        icon = <Icon> {
            draw_icon: {
                svg_file: dep("crate://self/resources/icons/aec.svg")
                fn get_color(self) -> vec4 {
                    return vec4(1.0, 1.0, 1.0, 1.0);  // WHITE
                }
            }
            icon_walk: {width: 20, height: 20}
        }
    }

    // Tab button style
    TabButton = <View> {
        width: Fit, height: Fit
        padding: {left: 16, right: 16, top: 10, bottom: 10}
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            instance selected: 0.0
            instance hover: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 6.0);
                // Light: transparent -> slate-100 (hover) -> white (selected)
                // Dark: transparent -> slate-700 (hover) -> slate-600 (selected)
                let light_normal = vec4(0.0, 0.0, 0.0, 0.0);
                let light_hover = (SLATE_100);
                let light_selected = (WHITE);
                let dark_normal = vec4(0.0, 0.0, 0.0, 0.0);
                let dark_hover = (SLATE_700);
                let dark_selected = (SLATE_600);
                let normal = mix(light_normal, dark_normal, self.dark_mode);
                let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                let selected_color = mix(light_selected, dark_selected, self.dark_mode);
                let base = mix(normal, hover_color, self.hover * (1.0 - self.selected));
                let color = mix(base, selected_color, self.selected);
                sdf.fill(color);
                return sdf.result;
            }
        }

        tab_label = <Label> {
            draw_text: {
                instance dark_mode: 0.0
                instance selected: 0.0
                text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                fn get_color(self) -> vec4 {
                    let light_normal = (SLATE_500);
                    let light_selected = (SLATE_900);
                    let dark_normal = (SLATE_400);
                    let dark_selected = (WHITE);
                    let normal = mix(light_normal, dark_normal, self.dark_mode);
                    let selected = mix(light_selected, dark_selected, self.dark_mode);
                    return mix(normal, selected, self.selected);
                }
            }
        }
    }

    // Reusable panel header style with dark mode support
    PanelHeader = <View> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}
        align: {y: 0.5}
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((SLATE_50), (SLATE_800), self.dark_mode);
            }
        }
    }

    // Reusable vertical divider
    VerticalDivider = <View> {
        width: 1, height: Fill
        margin: {top: 4, bottom: 4}
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((DIVIDER), (DIVIDER_DARK), self.dark_mode);
            }
        }
    }

    // MoFA FM Screen - adaptive horizontal layout with left content and right log panel
    pub MoFaFMScreen = {{MoFaFMScreen}} {
        width: Fill, height: Fill
        flow: Right
        spacing: 0
        padding: { left: 16, right: 16, top: 16, bottom: 16 }
        align: {y: 0.0}
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((DARK_BG), (DARK_BG_DARK), self.dark_mode);
            }
        }

        // Left column - main content area (adaptive width)
        left_column = <View> {
            width: Fill, height: Fill
            flow: Down
            spacing: (SECTION_SPACING)
            align: {y: 0.0}

            // System status bar (self-contained widget)
            mofa_hero = <MofaHero> {
                width: Fill
            }

            // Tab bar for Running/Settings
            tab_bar = <RoundedView> {
                width: Fill, height: Fit
                padding: 4
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    border_radius: 8.0
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                        let bg = mix((SLATE_100), (SLATE_800), self.dark_mode);
                        sdf.fill(bg);
                        return sdf.result;
                    }
                }
                flow: Right
                spacing: 4

                running_tab = <TabButton> {
                    draw_bg: { selected: 1.0 }
                    tab_label = { text: "Running", draw_text: { selected: 1.0 } }
                }

                settings_tab = <TabButton> {
                    tab_label = { text: "Settings" }
                }
            }

            // Running tab content - visible by default
            running_tab_content = <View> {
                width: Fill, height: Fill
                flow: Down
                spacing: (SECTION_SPACING)

            // Participant status cards container
            participant_container = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 8

                participant_bar = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    spacing: (SECTION_SPACING)

                    student1_panel = <ParticipantPanel> {
                        width: Fill, height: Fit
                        header = { name_label = { text: "Student 1" } }
                    }
                    student2_panel = <ParticipantPanel> {
                        width: Fill, height: Fit
                        header = { name_label = { text: "Student 2" } }
                    }
                    tutor_panel = <ParticipantPanel> {
                        width: Fill, height: Fit
                        header = { name_label = { text: "Tutor" } }
                    }
                }
            }

            // Chat window container (fills remaining space)
            chat_container = <View> {
                width: Fill, height: Fill
                flow: Down

                chat_section = <RoundedView> {
                    width: Fill, height: Fill
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: (PANEL_RADIUS)
                        border_size: 1.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let bg = mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                            let border = mix((BORDER), (SLATE_600), self.dark_mode);
                            sdf.fill(bg);
                            sdf.stroke(border, self.border_size);
                            return sdf.result;
                        }
                    }
                    flow: Down

                    // Chat header with copy button
                    chat_header = <PanelHeader> {
                        chat_title = <Label> {
                            text: "Chat History"
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                        }
                        <Filler> {}
                        // Copy to clipboard button
                        copy_chat_btn = <View> {
                            width: 28, height: 24
                            cursor: Hand
                            show_bg: true
                            draw_bg: {
                                instance copied: 0.0
                                instance dark_mode: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let c = self.rect_size * 0.5;

                                    // Light theme: Green → Teal → Blue → Gray
                                    let gray_light = (BORDER);
                                    let blue_light = vec4(0.231, 0.510, 0.965, 1.0);   // #3b82f6
                                    let teal_light = vec4(0.078, 0.722, 0.651, 1.0);   // #14b8a6
                                    let green_light = vec4(0.133, 0.773, 0.373, 1.0);  // #22c55f

                                    // Dark theme: Bright Green → Cyan → Purple → Slate
                                    let gray_dark = vec4(0.334, 0.371, 0.451, 1.0);    // #555e73 (slate-600)
                                    let purple_dark = vec4(0.639, 0.380, 0.957, 1.0);  // #a361f4
                                    let cyan_dark = vec4(0.133, 0.831, 0.894, 1.0);    // #22d4e4
                                    let green_dark = vec4(0.290, 0.949, 0.424, 1.0);   // #4af26c

                                    // Select colors based on dark mode
                                    let gray = mix(gray_light, gray_dark, self.dark_mode);
                                    let c1 = mix(blue_light, purple_dark, self.dark_mode);
                                    let c2 = mix(teal_light, cyan_dark, self.dark_mode);
                                    let c3 = mix(green_light, green_dark, self.dark_mode);

                                    // Multi-stop gradient based on copied value
                                    let t = self.copied;
                                    let bg_color = mix(
                                        mix(mix(gray, c1, clamp(t * 3.0, 0.0, 1.0)),
                                            c2, clamp((t - 0.33) * 3.0, 0.0, 1.0)),
                                        c3, clamp((t - 0.66) * 3.0, 0.0, 1.0)
                                    );

                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                    sdf.fill(bg_color);

                                    // Icon color - white when active, gray otherwise
                                    let icon_base = mix((GRAY_600), vec4(0.580, 0.639, 0.722, 1.0), self.dark_mode);
                                    let icon_color = mix(icon_base, vec4(1.0, 1.0, 1.0, 1.0), smoothstep(0.0, 0.3, self.copied));

                                    // Clipboard icon - back rectangle
                                    sdf.box(c.x - 4.0, c.y - 2.0, 8.0, 9.0, 1.0);
                                    sdf.stroke(icon_color, 1.2);

                                    // Clipboard icon - front rectangle (overlapping)
                                    sdf.box(c.x - 2.0, c.y - 5.0, 8.0, 9.0, 1.0);
                                    sdf.fill(bg_color);
                                    sdf.box(c.x - 2.0, c.y - 5.0, 8.0, 9.0, 1.0);
                                    sdf.stroke(icon_color, 1.2);

                                    return sdf.result;
                                }
                            }
                        }
                    }

                    // Chat messages area (scrollable, fills space)
                    chat_scroll = <ScrollYView> {
                        width: Fill, height: Fill
                        flow: Down
                        scroll_bars: <ScrollBars> {
                            show_scroll_x: false
                            show_scroll_y: true
                        }

                        chat_content_wrapper = <View> {
                            width: Fill, height: Fit
                            padding: (PANEL_PADDING)
                            flow: Down

                            chat_content = <Markdown> {
                                width: Fill, height: Fit
                                font_size: 13.0
                                font_color: (TEXT_PRIMARY)
                                paragraph_spacing: 8

                                draw_normal: {
                                    text_style: <FONT_REGULAR>{ font_size: 13.0 }
                                }
                                draw_bold: {
                                    text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                                }
                            }
                        }
                    }
                }
            }

            // Audio control panel container - side by side: controls and device selection
            audio_container = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: (SECTION_SPACING)

                // Left side: mic, AEC, and buffer controls
                audio_controls_row = <View> {
                    width: Fit, height: Fit
                    flow: Right
                    spacing: (SECTION_SPACING)

                    // Mic level meter container
                    mic_container = <RoundedView> {
                    width: Fit, height: Fit
                    padding: (PANEL_PADDING)
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: (PANEL_RADIUS)
                        border_size: 1.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let bg = mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                            let border = mix((BORDER), (SLATE_600), self.dark_mode);
                            sdf.fill(bg);
                            sdf.stroke(border, self.border_size);
                            return sdf.result;
                        }
                    }

                    mic_group = <View> {
                        width: Fit, height: Fit
                        flow: Right
                        spacing: 10
                        align: {y: 0.5}

                        mic_mute_btn = <MicButton> {}

                        mic_level_meter = <LedMeter> {}
                    }
                }

                // AEC toggle container
                aec_container = <RoundedView> {
                    width: Fit, height: Fit
                    padding: (PANEL_PADDING)
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: (PANEL_RADIUS)
                        border_size: 1.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let bg = mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                            let border = mix((BORDER), (SLATE_600), self.dark_mode);
                            sdf.fill(bg);
                            sdf.stroke(border, self.border_size);
                            return sdf.result;
                        }
                    }

                    aec_group = <View> {
                        width: Fit, height: Fit
                        flow: Right
                        spacing: 8
                        align: {y: 0.5}

                        aec_toggle_btn = <AecButton> {}
                    }
                }

                // Audio buffer indicator container
                buffer_container = <RoundedView> {
                    width: Fit, height: Fit
                    padding: (PANEL_PADDING)
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: (PANEL_RADIUS)
                        border_size: 1.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let bg = mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                            let border = mix((BORDER), (SLATE_600), self.dark_mode);
                            sdf.fill(bg);
                            sdf.stroke(border, self.border_size);
                            return sdf.result;
                        }
                    }

                    buffer_group = <View> {
                        width: Fit, height: Fit
                        flow: Right
                        spacing: 8
                        align: {y: 0.5}

                        buffer_label = <Label> {
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_MEDIUM>{ font_size: 10.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((GRAY_700), (TEXT_SECONDARY_DARK), self.dark_mode);
                                }
                            }
                            text: "Buffer"
                        }

                        buffer_meter = <LedMeter> {}

                        buffer_pct = <Label> {
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_REGULAR>{ font_size: 10.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((GRAY_500), (TEXT_SECONDARY_DARK), self.dark_mode);
                                }
                            }
                            text: "0%"
                        }
                    }
                }
                } // Close audio_controls_row

                // Right side: Device selectors container (fills remaining space)
                device_container = <RoundedView> {
                    width: Fill, height: Fit
                    padding: (PANEL_PADDING)
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: (PANEL_RADIUS)
                        border_size: 1.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let bg = mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                            let border = mix((BORDER), (SLATE_600), self.dark_mode);
                            sdf.fill(bg);
                            sdf.stroke(border, self.border_size);
                            return sdf.result;
                        }
                    }

                    device_selectors = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 16
                        align: {y: 0.5}

                        // Input device group (fills available space)
                        input_device_group = <View> {
                            width: Fill, height: Fit
                            flow: Right
                            spacing: 8
                            align: {y: 0.5}

                            input_device_label = <Label> {
                                width: 90  // Fixed width for alignment with output label
                                text: "Microphone:"
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                    }
                                }
                            }

                            input_device_dropdown = <DropDown> {
                                width: Fill, height: Fit
                                padding: {left: 10, right: 10, top: 6, bottom: 6}
                                popup_menu_position: BelowInput
                                // Labels will be set at runtime by init_audio()
                                labels: []
                                values: []
                                selected_item: 0
                                draw_bg: {
                                    instance dark_mode: 0.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 3.0);
                                        let bg = mix((SLATE_100), (SLATE_700), self.dark_mode);
                                        sdf.fill(bg);
                                        return sdf.result;
                                    }
                                }
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        let light = mix((SLATE_500), (TEXT_PRIMARY), self.focus);
                                        let dark = mix((SLATE_300), (TEXT_PRIMARY_DARK), self.focus);
                                        return mix(light, dark, self.dark_mode);
                                    }
                                }
                                popup_menu: {
                                    width: 250
                                    draw_bg: {
                                        instance dark_mode: 0.0
                                        border_size: 1.0
                                        fn pixel(self) -> vec4 {
                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 2.0);
                                            let bg = mix((WHITE), (SLATE_800), self.dark_mode);
                                            let border = mix((BORDER), (SLATE_600), self.dark_mode);
                                            sdf.fill(bg);
                                            sdf.stroke(border, self.border_size);
                                            return sdf.result;
                                        }
                                    }
                                    menu_item: {
                                        width: Fill
                                        draw_bg: {
                                            instance dark_mode: 0.0
                                            fn pixel(self) -> vec4 {
                                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                                let base = mix((WHITE), (SLATE_800), self.dark_mode);
                                                let hover_color = mix((GRAY_100), (SLATE_700), self.dark_mode);
                                                sdf.fill(mix(base, hover_color, self.hover));
                                                return sdf.result;
                                            }
                                        }
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            fn get_color(self) -> vec4 {
                                                let light_base = mix((GRAY_700), (TEXT_PRIMARY), self.active);
                                                let dark_base = mix((SLATE_300), (TEXT_PRIMARY_DARK), self.active);
                                                let base = mix(light_base, dark_base, self.dark_mode);
                                                let light_hover = (TEXT_PRIMARY);
                                                let dark_hover = (TEXT_PRIMARY_DARK);
                                                let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                                                return mix(base, hover_color, self.hover);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        <VerticalDivider> {}

                        // Output device group (fills available space)
                        output_device_group = <View> {
                            width: Fill, height: Fit
                            flow: Right
                            spacing: 8
                            align: {y: 0.5}

                            output_device_label = <Label> {
                                width: 90  // Fixed width for alignment with input label
                                text: "Speaker:"
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                    }
                                }
                            }

                            output_device_dropdown = <DropDown> {
                                width: Fill, height: Fit
                                padding: {left: 10, right: 10, top: 6, bottom: 6}
                                popup_menu_position: BelowInput
                                // Labels will be set at runtime by init_audio()
                                labels: []
                                values: []
                                selected_item: 0
                                draw_bg: {
                                    instance dark_mode: 0.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 3.0);
                                        let bg = mix((SLATE_100), (SLATE_700), self.dark_mode);
                                        sdf.fill(bg);
                                        return sdf.result;
                                    }
                                }
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        let light = mix((SLATE_500), (TEXT_PRIMARY), self.focus);
                                        let dark = mix((SLATE_300), (TEXT_PRIMARY_DARK), self.focus);
                                        return mix(light, dark, self.dark_mode);
                                    }
                                }
                                popup_menu: {
                                    width: 250
                                    draw_bg: {
                                        instance dark_mode: 0.0
                                        border_size: 1.0
                                        fn pixel(self) -> vec4 {
                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 2.0);
                                            let bg = mix((WHITE), (SLATE_800), self.dark_mode);
                                            let border = mix((BORDER), (SLATE_600), self.dark_mode);
                                            sdf.fill(bg);
                                            sdf.stroke(border, self.border_size);
                                            return sdf.result;
                                        }
                                    }
                                    menu_item: {
                                        width: Fill
                                        draw_bg: {
                                            instance dark_mode: 0.0
                                            fn pixel(self) -> vec4 {
                                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                                let base = mix((WHITE), (SLATE_800), self.dark_mode);
                                                let hover_color = mix((GRAY_100), (SLATE_700), self.dark_mode);
                                                sdf.fill(mix(base, hover_color, self.hover));
                                                return sdf.result;
                                            }
                                        }
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            fn get_color(self) -> vec4 {
                                                let light_base = mix((GRAY_700), (TEXT_PRIMARY), self.active);
                                                let dark_base = mix((SLATE_300), (TEXT_PRIMARY_DARK), self.active);
                                                let base = mix(light_base, dark_base, self.dark_mode);
                                                let light_hover = (TEXT_PRIMARY);
                                                let dark_hover = (TEXT_PRIMARY_DARK);
                                                let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                                                return mix(base, hover_color, self.hover);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Prompt input area container
            prompt_container = <View> {
                width: Fill, height: Fit
                flow: Down

                prompt_section = <RoundedView> {
                    width: Fill, height: Fit
                    padding: (PANEL_PADDING)
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: (PANEL_RADIUS)
                        fn get_color(self) -> vec4 {
                            return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                        }
                    }
                    flow: Down
                    spacing: 8

                    prompt_row = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 12
                        align: {y: 0.5}

                        prompt_input = <TextInput> {
                            width: Fill, height: Fit
                            padding: {left: 12, right: 12, top: 10, bottom: 10}
                            empty_text: "Enter prompt to send..."
                            draw_bg: {
                                instance dark_mode: 0.0
                                border_radius: 4.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                    let bg = mix((SLATE_200), (SLATE_700), self.dark_mode);
                                    sdf.fill(bg);
                                    return sdf.result;
                                }
                            }
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                            draw_selection: {
                                color: (INDIGO_200)
                            }
                            draw_cursor: {
                                color: (ACCENT_BLUE)
                            }
                        }

                        button_group = <View> {
                            width: Fit, height: Fit
                            flow: Right
                            spacing: 8

                            send_prompt_btn = <Button> {
                                width: Fit, height: 35
                                padding: {left: 16, right: 16}
                                text: "Send"

                                animator: {
                                    hover = {
                                        default: off,
                                        off = {
                                            from: {all: Forward {duration: 0.15}}
                                            apply: { draw_bg: {hover: 0.0} }
                                        }
                                        on = {
                                            from: {all: Forward {duration: 0.15}}
                                            apply: { draw_bg: {hover: 1.0} }
                                        }
                                    }
                                    pressed = {
                                        default: off,
                                        off = {
                                            from: {all: Forward {duration: 0.1}}
                                            apply: { draw_bg: {pressed: 0.0} }
                                        }
                                        on = {
                                            from: {all: Forward {duration: 0.1}}
                                            apply: { draw_bg: {pressed: 1.0} }
                                        }
                                    }
                                }

                                draw_text: {
                                    color: (WHITE)
                                    text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                                }
                                draw_bg: {
                                    instance hover: 0.0
                                    instance pressed: 0.0
                                    border_radius: 4.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        let color = mix(
                                            mix((ACCENT_BLUE), (BLUE_600), self.hover),
                                            (BLUE_700),
                                            self.pressed
                                        );
                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                        sdf.fill(color);
                                        return sdf.result;
                                    }
                                }
                            }

                            reset_btn = <Button> {
                                width: Fit, height: 35
                                padding: {left: 16, right: 16}
                                text: "Reset"

                                animator: {
                                    hover = {
                                        default: off,
                                        off = {
                                            from: {all: Forward {duration: 0.15}}
                                            apply: { draw_bg: {hover: 0.0} }
                                        }
                                        on = {
                                            from: {all: Forward {duration: 0.15}}
                                            apply: { draw_bg: {hover: 1.0} }
                                        }
                                    }
                                    pressed = {
                                        default: off,
                                        off = {
                                            from: {all: Forward {duration: 0.1}}
                                            apply: { draw_bg: {pressed: 0.0} }
                                        }
                                        on = {
                                            from: {all: Forward {duration: 0.1}}
                                            apply: { draw_bg: {pressed: 1.0} }
                                        }
                                    }
                                }

                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((GRAY_700), (SLATE_300), self.dark_mode);
                                    }
                                }
                                draw_bg: {
                                    instance hover: 0.0
                                    instance pressed: 0.0
                                    instance dark_mode: 0.0
                                    border_radius: 4.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                        let base = mix((HOVER_BG), (SLATE_600), self.dark_mode);
                                        let hover_color = mix((SLATE_200), (SLATE_500), self.dark_mode);
                                        let pressed_color = mix((SLATE_300), (SLATE_400), self.dark_mode);
                                        let color = mix(mix(base, hover_color, self.hover), pressed_color, self.pressed);
                                        sdf.fill(color);
                                        return sdf.result;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            } // end running_tab_content

            // Settings tab content - hidden by default
            settings_tab_content = <View> {
                width: Fill, height: Fill
                visible: false
                flow: Down
                spacing: (SECTION_SPACING)

                // Settings panel
                settings_panel = <RoundedView> {
                    width: Fill, height: Fill
                    padding: 20
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: (PANEL_RADIUS)
                        border_size: 1.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let bg = mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                            let border = mix((BORDER), (SLATE_600), self.dark_mode);
                            sdf.fill(bg);
                            sdf.stroke(border, self.border_size);
                            return sdf.result;
                        }
                    }
                    flow: Down
                    spacing: 20

                    // Settings header
                    settings_header = <View> {
                        width: Fill, height: Fit
                        flow: Down
                        spacing: 4

                        settings_title = <Label> {
                            text: "MoFA FM Settings"
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_SEMIBOLD>{ font_size: 18.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                        }

                        settings_subtitle = <Label> {
                            text: "Configure voice chat settings and participant options"
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_REGULAR>{ font_size: 12.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                }
                            }
                        }
                    }

                    // Divider
                    <View> {
                        width: Fill, height: 1
                        show_bg: true
                        draw_bg: {
                            instance dark_mode: 0.0
                            fn pixel(self) -> vec4 {
                                return mix((DIVIDER), (DIVIDER_DARK), self.dark_mode);
                            }
                        }
                    }

                    // Settings content - scrollable
                    settings_scroll = <ScrollYView> {
                        width: Fill, height: Fill
                        flow: Down
                        scroll_bars: <ScrollBars> {
                            show_scroll_x: false
                            show_scroll_y: true
                            scroll_bar_y: {
                                bar_size: 8.0
                                bar_side_margin: 2.0
                                min_handle_size: 30.0
                                smoothing: 0.15
                                draw_bg: {
                                    instance dark_mode: 0.0
                                    uniform border_radius: 4.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        if self.is_vertical > 0.5 {
                                            sdf.box(
                                                1.,
                                                self.rect_size.y * self.norm_scroll,
                                                self.rect_size.x - 2.0,
                                                self.rect_size.y * self.norm_handle,
                                                self.border_radius
                                            );
                                        } else {
                                            sdf.box(
                                                self.rect_size.x * self.norm_scroll,
                                                1.,
                                                self.rect_size.x * self.norm_handle,
                                                self.rect_size.y - 2.0,
                                                self.border_radius
                                            );
                                        }
                                        let base = mix(vec4(0.58, 0.64, 0.69, 1.0), vec4(0.39, 0.45, 0.53, 1.0), self.dark_mode);
                                        let hover_color = mix(vec4(0.49, 0.55, 0.61, 1.0), vec4(0.49, 0.55, 0.61, 1.0), self.dark_mode);
                                        sdf.fill(mix(base, hover_color, self.hover));
                                        return sdf.result;
                                    }
                                }
                            }
                        }

                        settings_content = <View> {
                            width: Fill, height: Fit
                            flow: Down
                            spacing: 24
                            padding: {right: 12}

                            // Dataflow Configuration Section
                            dataflow_section = <View> {
                                width: Fill, height: Fit
                                flow: Down
                                spacing: 12

                                dataflow_section_title = <Label> {
                                    text: "Dataflow Configuration"
                                    draw_text: {
                                        instance dark_mode: 0.0
                                        text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                                        fn get_color(self) -> vec4 {
                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                        }
                                    }
                                }

                                // Dataflow path row
                                dataflow_path_row = <View> {
                                    width: Fill, height: Fit
                                    flow: Right
                                    spacing: 12
                                    align: {y: 0.5}

                                    dataflow_path_label = <Label> {
                                        width: 120
                                        text: "Dataflow Path"
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                            fn get_color(self) -> vec4 {
                                                return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                            }
                                        }
                                    }

                                    dataflow_path_value = <Label> {
                                        width: Fill
                                        text: "apps/mofa-fm/dataflow/voice-chat.yml"
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            text_style: <FONT_REGULAR>{ font_size: 12.0 }
                                            fn get_color(self) -> vec4 {
                                                return mix((GRAY_600), (SLATE_400), self.dark_mode);
                                            }
                                        }
                                    }
                                }
                            }

                            // Role Configuration Section
                            role_section = <View> {
                                width: Fill, height: Fit
                                flow: Down
                                spacing: 16

                                role_section_title = <Label> {
                                    text: "Role Configuration"
                                    draw_text: {
                                        instance dark_mode: 0.0
                                        text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                                        fn get_color(self) -> vec4 {
                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                        }
                                    }
                                }

                                // Student 1 Configuration
                                student1_config = <RoundedView> {
                                    width: Fill, height: Fit
                                    padding: 16
                                    show_bg: true
                                    draw_bg: {
                                        instance dark_mode: 0.0
                                        instance opacity: 1.0
                                        instance highlight: 0.0
                                        border_radius: 8.0
                                        fn pixel(self) -> vec4 {
                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                            let bg = mix((SLATE_50), (SLATE_700), self.dark_mode);
                                            // Scale effect: shrink from center when highlight > 0
                                            let scale = 1.0 - self.highlight * 0.05;
                                            let offset_x = self.rect_size.x * (1.0 - scale) * 0.5;
                                            let offset_y = self.rect_size.y * (1.0 - scale) * 0.5;
                                            let w = self.rect_size.x * scale;
                                            let h = self.rect_size.y * scale;
                                            sdf.box(offset_x, offset_y, w, h, self.border_radius * scale);
                                            sdf.fill(vec4(bg.x, bg.y, bg.z, bg.w * self.opacity));
                                            return sdf.result;
                                        }
                                    }
                                    flow: Down
                                    spacing: 12

                                    // Header row with name and status
                                    student1_header = <View> {
                                        width: Fill, height: Fit
                                        flow: Right
                                        spacing: 8
                                        align: {y: 0.5}

                                        student1_name = <Label> {
                                            text: "Student 1 (大牛)"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                }
                                            }
                                        }

                                        <View> { width: Fill, height: 1 }

                                        student1_save_btn = <Button> {
                                            width: Fit, height: Fit
                                            padding: {left: 12, right: 12, top: 4, bottom: 4}
                                            text: "Save"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                                fn get_color(self) -> vec4 {
                                                    return (WHITE);
                                                }
                                            }
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                instance hover: 0.0
                                                instance pressed: 0.0
                                                instance saved: 0.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                                    let base = mix((ACCENT_BLUE), (BLUE_600), self.dark_mode);
                                                    let hover_color = mix((BLUE_600), (BLUE_500), self.dark_mode);
                                                    let pressed_color = mix((BLUE_700), (BLUE_400), self.dark_mode);
                                                    let saved_color = vec4(0.2, 0.8, 0.4, 1.0);
                                                    let normal_color = mix(mix(base, hover_color, self.hover), pressed_color, self.pressed);
                                                    let color = mix(normal_color, saved_color, self.saved);
                                                    sdf.fill(color);
                                                    return sdf.result;
                                                }
                                            }
                                        }

                                        student1_maximize_btn = <View> {
                                            width: 20, height: 20
                                            margin: {left: 8}
                                            cursor: Hand
                                            show_bg: true
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                instance maximized: 0.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    let color = mix(vec4(0.4, 0.45, 0.5, 1.0), vec4(0.7, 0.75, 0.8, 1.0), self.dark_mode);
                                                    let w = self.rect_size.x;
                                                    let h = self.rect_size.y;
                                                    let t = self.maximized;
                                                    // Interpolate between expand (t=0) and collapse (t=1) icons
                                                    // Top-right arrow: L-shape corner moves from inner to outer
                                                    let tr_corner_x = mix(0.71, 0.83, t);
                                                    let tr_corner_y = mix(0.29, 0.17, t);
                                                    let tr_end_x = mix(0.62, 0.58, t);
                                                    let tr_end_y = mix(0.38, 0.42, t);
                                                    sdf.move_to(w * mix(0.58, 0.58, t), h * mix(0.29, 0.26, t));
                                                    sdf.line_to(w * mix(0.71, 0.58, t), h * mix(0.29, 0.42, t));
                                                    sdf.move_to(w * mix(0.71, 0.58, t), h * mix(0.29, 0.42, t));
                                                    sdf.line_to(w * mix(0.71, 0.74, t), h * mix(0.42, 0.42, t));
                                                    // Diagonal line
                                                    sdf.move_to(w * mix(0.46, 0.83, t), h * mix(0.54, 0.17, t));
                                                    sdf.line_to(w * tr_end_x, h * tr_end_y);
                                                    // Bottom-left arrow: L-shape corner moves from inner to outer
                                                    let bl_corner_x = mix(0.29, 0.17, t);
                                                    let bl_corner_y = mix(0.71, 0.83, t);
                                                    let bl_end_x = mix(0.38, 0.42, t);
                                                    let bl_end_y = mix(0.62, 0.58, t);
                                                    sdf.move_to(w * mix(0.29, 0.26, t), h * mix(0.58, 0.58, t));
                                                    sdf.line_to(w * mix(0.29, 0.42, t), h * mix(0.71, 0.58, t));
                                                    sdf.move_to(w * mix(0.29, 0.42, t), h * mix(0.71, 0.58, t));
                                                    sdf.line_to(w * mix(0.42, 0.42, t), h * mix(0.71, 0.74, t));
                                                    // Diagonal line
                                                    sdf.move_to(w * mix(0.54, 0.17, t), h * mix(0.46, 0.83, t));
                                                    sdf.line_to(w * bl_end_x, h * bl_end_y);
                                                    sdf.stroke(color, 1.2);
                                                    return sdf.result;
                                                }
                                            }
                                        }
                                    }

                                    // Model selection row
                                    student1_model_row = <View> {
                                        width: Fill, height: Fit
                                        flow: Right
                                        spacing: 12
                                        align: {y: 0.5}

                                        student1_model_label = <Label> {
                                            width: 100
                                            text: "Model"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                                }
                                            }
                                        }

                                        student1_model_dropdown = <DropDown> {
                                            width: 200, height: Fit
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                border_radius: 4.0
                                                border_size: 1.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                                    let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                                                    let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                    sdf.fill(bg);
                                                    sdf.stroke(border, self.border_size);
                                                    return sdf.result;
                                                }
                                            }
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_REGULAR>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                }
                                            }
                                            popup_menu: {
                                                draw_bg: {
                                                    instance dark_mode: 0.0
                                                    border_size: 1.0
                                                    fn pixel(self) -> vec4 {
                                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                                        let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                                                        let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                        sdf.fill(bg);
                                                        sdf.stroke(border, self.border_size);
                                                        return sdf.result;
                                                    }
                                                }
                                                menu_item: {
                                                    indent_width: 10.0
                                                    padding: {left: 15, top: 8, bottom: 8, right: 15}
                                                    draw_bg: {
                                                        instance dark_mode: 0.0
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                                            let base = mix((WHITE), (SLATE_700), self.dark_mode);
                                                            let hover_color = mix((GRAY_100), (SLATE_600), self.dark_mode);
                                                            sdf.fill(mix(base, hover_color, self.hover));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    draw_text: {
                                                        instance dark_mode: 0.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                        }
                                                    }
                                                }
                                            }
                                            labels: ["gpt-4o", "gpt-4o-mini", "deepseek-chat"]
                                            selected_item: 0
                                        }
                                    }

                                    // Voice selection row
                                    student1_voice_row = <View> {
                                        width: Fill, height: Fit
                                        flow: Right
                                        spacing: 12
                                        align: {y: 0.5}

                                        student1_voice_label = <Label> {
                                            width: 100
                                            text: "Voice"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                                }
                                            }
                                        }

                                        student1_voice_dropdown = <DropDown> {
                                            width: 200, height: Fit
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                border_radius: 4.0
                                                border_size: 1.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                                    let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                                                    let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                    sdf.fill(bg);
                                                    sdf.stroke(border, self.border_size);
                                                    return sdf.result;
                                                }
                                            }
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_REGULAR>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                }
                                            }
                                            popup_menu: {
                                                draw_bg: {
                                                    instance dark_mode: 0.0
                                                    border_size: 1.0
                                                    fn pixel(self) -> vec4 {
                                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                                        let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                                                        let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                        sdf.fill(bg);
                                                        sdf.stroke(border, self.border_size);
                                                        return sdf.result;
                                                    }
                                                }
                                                menu_item: {
                                                    indent_width: 10.0
                                                    padding: {left: 15, top: 8, bottom: 8, right: 15}
                                                    draw_bg: {
                                                        instance dark_mode: 0.0
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                                            let base = mix((WHITE), (SLATE_700), self.dark_mode);
                                                            let hover_color = mix((GRAY_100), (SLATE_600), self.dark_mode);
                                                            sdf.fill(mix(base, hover_color, self.hover));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    draw_text: {
                                                        instance dark_mode: 0.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                        }
                                                    }
                                                }
                                            }
                                            labels: ["Zhao Daniu", "Chen Yifan", "Luo Xiang", "Doubao", "Yang Mi", "Ma Yun", "Maple", "Cove", "Ellen", "Juniper"]
                                            selected_item: 0
                                        }
                                    }

                                    // System Prompt label
                                    student1_prompt_label = <Label> {
                                        text: "System Prompt"
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                            fn get_color(self) -> vec4 {
                                                return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                            }
                                        }
                                    }

                                    student1_prompt_container = <RoundedView> {
                                        width: Fill, height: 120
                                        show_bg: true
                                        draw_bg: {
                                            instance dark_mode: 0.0
                                            border_radius: 4.0
                                            fn pixel(self) -> vec4 {
                                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                                let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                                                let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                sdf.fill(bg);
                                                sdf.stroke(border, 1.0);
                                                return sdf.result;
                                            }
                                        }

                                        student1_prompt_scroll = <ScrollYView> {
                                            width: Fill, height: Fill
                                            scroll_bars: <ScrollBars> {
                                                show_scroll_x: false
                                                show_scroll_y: true
                                                scroll_bar_y: {
                                                    bar_size: 8.0
                                                    bar_side_margin: 2.0
                                                    min_handle_size: 30.0
                                                    smoothing: 0.15
                                                    draw_bg: {
                                                        instance dark_mode: 0.0
                                                        uniform border_radius: 4.0
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            if self.is_vertical > 0.5 {
                                                                sdf.box(
                                                                    1.,
                                                                    self.rect_size.y * self.norm_scroll,
                                                                    self.rect_size.x - 2.0,
                                                                    self.rect_size.y * self.norm_handle,
                                                                    self.border_radius
                                                                );
                                                            } else {
                                                                sdf.box(
                                                                    self.rect_size.x * self.norm_scroll,
                                                                    1.,
                                                                    self.rect_size.x * self.norm_handle,
                                                                    self.rect_size.y - 2.0,
                                                                    self.border_radius
                                                                );
                                                            }
                                                            // Light: SLATE_400, Dark: SLATE_500
                                                            let base = mix(vec4(0.58, 0.64, 0.69, 1.0), vec4(0.39, 0.45, 0.53, 1.0), self.dark_mode);
                                                            // Hover: slightly lighter
                                                            let hover_color = mix(vec4(0.49, 0.55, 0.61, 1.0), vec4(0.49, 0.55, 0.61, 1.0), self.dark_mode);
                                                            sdf.fill(mix(base, hover_color, self.hover));
                                                            return sdf.result;
                                                        }
                                                    }
                                                }
                                            }

                                            student1_prompt_wrapper = <View> {
                                                width: Fill, height: Fit
                                                padding: 8

                                                student1_prompt_input = <TextInput> {
                                                    width: Fill, height: Fit
                                                    draw_bg: {
                                                        fn pixel(self) -> vec4 {
                                                            return vec4(0.0, 0.0, 0.0, 0.0);
                                                        }
                                                    }
                                                    draw_text: {
                                                        instance dark_mode: 0.0
                                                        text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                                        fn get_color(self) -> vec4 {
                                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                        }
                                                    }
                                                    draw_selection: {
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 1.0);
                                                            sdf.fill(vec4(0.26, 0.52, 0.96, 0.4));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    draw_cursor: {
                                                        instance focus: 0.0
                                                        instance blink: 0.0
                                                        instance dark_mode: 0.0
                                                        uniform border_radius: 0.5
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius);
                                                            let cursor_color = mix(vec4(0.1, 0.1, 0.12, 1.0), vec4(0.9, 0.9, 0.95, 1.0), self.dark_mode);
                                                            sdf.fill(mix(vec4(0.0, 0.0, 0.0, 0.0), cursor_color, (1.0 - self.blink) * self.focus));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    animator: {
                                                        blink = {
                                                            default: off
                                                            off = {
                                                                from: {all: Forward {duration: 0.5}}
                                                                apply: { draw_cursor: {blink: 0.0} }
                                                            }
                                                            on = {
                                                                from: {all: Forward {duration: 0.5}}
                                                                apply: { draw_cursor: {blink: 1.0} }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                }

                                // Student 2 Configuration
                                student2_config = <RoundedView> {
                                    width: Fill, height: Fit
                                    padding: 16
                                    show_bg: true
                                    draw_bg: {
                                        instance dark_mode: 0.0
                                        instance opacity: 1.0
                                        instance highlight: 0.0
                                        border_radius: 8.0
                                        fn pixel(self) -> vec4 {
                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                            let bg = mix((SLATE_50), (SLATE_700), self.dark_mode);
                                            // Scale effect: shrink from center when highlight > 0
                                            let scale = 1.0 - self.highlight * 0.05;
                                            let offset_x = self.rect_size.x * (1.0 - scale) * 0.5;
                                            let offset_y = self.rect_size.y * (1.0 - scale) * 0.5;
                                            let w = self.rect_size.x * scale;
                                            let h = self.rect_size.y * scale;
                                            sdf.box(offset_x, offset_y, w, h, self.border_radius * scale);
                                            sdf.fill(vec4(bg.x, bg.y, bg.z, bg.w * self.opacity));
                                            return sdf.result;
                                        }
                                    }
                                    flow: Down
                                    spacing: 12

                                    student2_header = <View> {
                                        width: Fill, height: Fit
                                        flow: Right
                                        spacing: 8
                                        align: {y: 0.5}

                                        student2_name = <Label> {
                                            text: "Student 2 (亦菲)"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                }
                                            }
                                        }

                                        <View> { width: Fill, height: 1 }

                                        student2_save_btn = <Button> {
                                            width: Fit, height: Fit
                                            padding: {left: 12, right: 12, top: 4, bottom: 4}
                                            text: "Save"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                                fn get_color(self) -> vec4 {
                                                    return (WHITE);
                                                }
                                            }
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                instance hover: 0.0
                                                instance pressed: 0.0
                                                instance saved: 0.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                                    let base = mix((ACCENT_BLUE), (BLUE_600), self.dark_mode);
                                                    let hover_color = mix((BLUE_600), (BLUE_500), self.dark_mode);
                                                    let pressed_color = mix((BLUE_700), (BLUE_400), self.dark_mode);
                                                    let saved_color = vec4(0.2, 0.8, 0.4, 1.0);
                                                    let normal_color = mix(mix(base, hover_color, self.hover), pressed_color, self.pressed);
                                                    let color = mix(normal_color, saved_color, self.saved);
                                                    sdf.fill(color);
                                                    return sdf.result;
                                                }
                                            }
                                        }

                                        student2_maximize_btn = <View> {
                                            width: 20, height: 20
                                            margin: {left: 8}
                                            cursor: Hand
                                            show_bg: true
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                instance maximized: 0.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    let color = mix(vec4(0.4, 0.45, 0.5, 1.0), vec4(0.7, 0.75, 0.8, 1.0), self.dark_mode);
                                                    let w = self.rect_size.x;
                                                    let h = self.rect_size.y;
                                                    let t = self.maximized;
                                                    // Interpolate between expand (t=0) and collapse (t=1) icons
                                                    // Top-right arrow: L-shape corner moves from inner to outer
                                                    let tr_corner_x = mix(0.71, 0.83, t);
                                                    let tr_corner_y = mix(0.29, 0.17, t);
                                                    let tr_end_x = mix(0.62, 0.58, t);
                                                    let tr_end_y = mix(0.38, 0.42, t);
                                                    sdf.move_to(w * mix(0.58, 0.58, t), h * mix(0.29, 0.26, t));
                                                    sdf.line_to(w * mix(0.71, 0.58, t), h * mix(0.29, 0.42, t));
                                                    sdf.move_to(w * mix(0.71, 0.58, t), h * mix(0.29, 0.42, t));
                                                    sdf.line_to(w * mix(0.71, 0.74, t), h * mix(0.42, 0.42, t));
                                                    // Diagonal line
                                                    sdf.move_to(w * mix(0.46, 0.83, t), h * mix(0.54, 0.17, t));
                                                    sdf.line_to(w * tr_end_x, h * tr_end_y);
                                                    // Bottom-left arrow: L-shape corner moves from inner to outer
                                                    let bl_corner_x = mix(0.29, 0.17, t);
                                                    let bl_corner_y = mix(0.71, 0.83, t);
                                                    let bl_end_x = mix(0.38, 0.42, t);
                                                    let bl_end_y = mix(0.62, 0.58, t);
                                                    sdf.move_to(w * mix(0.29, 0.26, t), h * mix(0.58, 0.58, t));
                                                    sdf.line_to(w * mix(0.29, 0.42, t), h * mix(0.71, 0.58, t));
                                                    sdf.move_to(w * mix(0.29, 0.42, t), h * mix(0.71, 0.58, t));
                                                    sdf.line_to(w * mix(0.42, 0.42, t), h * mix(0.71, 0.74, t));
                                                    // Diagonal line
                                                    sdf.move_to(w * mix(0.54, 0.17, t), h * mix(0.46, 0.83, t));
                                                    sdf.line_to(w * bl_end_x, h * bl_end_y);
                                                    sdf.stroke(color, 1.2);
                                                    return sdf.result;
                                                }
                                            }
                                        }
                                    }

                                    student2_model_row = <View> {
                                        width: Fill, height: Fit
                                        flow: Right
                                        spacing: 12
                                        align: {y: 0.5}

                                        student2_model_label = <Label> {
                                            width: 100
                                            text: "Model"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                                }
                                            }
                                        }

                                        student2_model_dropdown = <DropDown> {
                                            width: 200, height: Fit
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                border_radius: 4.0
                                                border_size: 1.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                                    let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                                                    let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                    sdf.fill(bg);
                                                    sdf.stroke(border, self.border_size);
                                                    return sdf.result;
                                                }
                                            }
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_REGULAR>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                }
                                            }
                                            popup_menu: {
                                                draw_bg: {
                                                    instance dark_mode: 0.0
                                                    border_size: 1.0
                                                    fn pixel(self) -> vec4 {
                                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                                        let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                                                        let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                        sdf.fill(bg);
                                                        sdf.stroke(border, self.border_size);
                                                        return sdf.result;
                                                    }
                                                }
                                                menu_item: {
                                                    indent_width: 10.0
                                                    padding: {left: 15, top: 8, bottom: 8, right: 15}
                                                    draw_bg: {
                                                        instance dark_mode: 0.0
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                                            let base = mix((WHITE), (SLATE_700), self.dark_mode);
                                                            let hover_color = mix((GRAY_100), (SLATE_600), self.dark_mode);
                                                            sdf.fill(mix(base, hover_color, self.hover));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    draw_text: {
                                                        instance dark_mode: 0.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                        }
                                                    }
                                                }
                                            }
                                            labels: ["gpt-4o", "gpt-4o-mini", "deepseek-chat"]
                                            selected_item: 0
                                        }
                                    }

                                    student2_voice_row = <View> {
                                        width: Fill, height: Fit
                                        flow: Right
                                        spacing: 12
                                        align: {y: 0.5}

                                        student2_voice_label = <Label> {
                                            width: 100
                                            text: "Voice"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                                }
                                            }
                                        }

                                        student2_voice_dropdown = <DropDown> {
                                            width: 200, height: Fit
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                border_radius: 4.0
                                                border_size: 1.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                                    let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                                                    let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                    sdf.fill(bg);
                                                    sdf.stroke(border, self.border_size);
                                                    return sdf.result;
                                                }
                                            }
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_REGULAR>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                }
                                            }
                                            popup_menu: {
                                                draw_bg: {
                                                    instance dark_mode: 0.0
                                                    border_size: 1.0
                                                    fn pixel(self) -> vec4 {
                                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                                        let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                                                        let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                        sdf.fill(bg);
                                                        sdf.stroke(border, self.border_size);
                                                        return sdf.result;
                                                    }
                                                }
                                                menu_item: {
                                                    indent_width: 10.0
                                                    padding: {left: 15, top: 8, bottom: 8, right: 15}
                                                    draw_bg: {
                                                        instance dark_mode: 0.0
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                                            let base = mix((WHITE), (SLATE_700), self.dark_mode);
                                                            let hover_color = mix((GRAY_100), (SLATE_600), self.dark_mode);
                                                            sdf.fill(mix(base, hover_color, self.hover));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    draw_text: {
                                                        instance dark_mode: 0.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                        }
                                                    }
                                                }
                                            }
                                            labels: ["Zhao Daniu", "Chen Yifan", "Luo Xiang", "Doubao", "Yang Mi", "Ma Yun", "Maple", "Cove", "Ellen", "Juniper"]
                                            selected_item: 4
                                        }
                                    }

                                    // System Prompt label
                                    student2_prompt_label = <Label> {
                                        text: "System Prompt"
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                            fn get_color(self) -> vec4 {
                                                return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                            }
                                        }
                                    }

                                    student2_prompt_container = <RoundedView> {
                                        width: Fill, height: 120
                                        show_bg: true
                                        draw_bg: {
                                            instance dark_mode: 0.0
                                            border_radius: 4.0
                                            fn pixel(self) -> vec4 {
                                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                                let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                                                let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                sdf.fill(bg);
                                                sdf.stroke(border, 1.0);
                                                return sdf.result;
                                            }
                                        }

                                        student2_prompt_scroll = <ScrollYView> {
                                            width: Fill, height: Fill
                                            scroll_bars: <ScrollBars> {
                                                show_scroll_x: false
                                                show_scroll_y: true
                                                scroll_bar_y: {
                                                    bar_size: 8.0
                                                    bar_side_margin: 2.0
                                                    min_handle_size: 30.0
                                                    smoothing: 0.15
                                                    draw_bg: {
                                                        instance dark_mode: 0.0
                                                        uniform border_radius: 4.0
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            if self.is_vertical > 0.5 {
                                                                sdf.box(
                                                                    1.,
                                                                    self.rect_size.y * self.norm_scroll,
                                                                    self.rect_size.x - 2.0,
                                                                    self.rect_size.y * self.norm_handle,
                                                                    self.border_radius
                                                                );
                                                            } else {
                                                                sdf.box(
                                                                    self.rect_size.x * self.norm_scroll,
                                                                    1.,
                                                                    self.rect_size.x * self.norm_handle,
                                                                    self.rect_size.y - 2.0,
                                                                    self.border_radius
                                                                );
                                                            }
                                                            let base = mix(vec4(0.58, 0.64, 0.69, 1.0), vec4(0.39, 0.45, 0.53, 1.0), self.dark_mode);
                                                            let hover_color = mix(vec4(0.49, 0.55, 0.61, 1.0), vec4(0.49, 0.55, 0.61, 1.0), self.dark_mode);
                                                            sdf.fill(mix(base, hover_color, self.hover));
                                                            return sdf.result;
                                                        }
                                                    }
                                                }
                                            }

                                            student2_prompt_wrapper = <View> {
                                                width: Fill, height: Fit
                                                padding: 8

                                                student2_prompt_input = <TextInput> {
                                                    width: Fill, height: Fit
                                                    draw_bg: {
                                                        fn pixel(self) -> vec4 {
                                                            return vec4(0.0, 0.0, 0.0, 0.0);
                                                        }
                                                    }
                                                    draw_text: {
                                                        instance dark_mode: 0.0
                                                        text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                                        fn get_color(self) -> vec4 {
                                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                        }
                                                    }
                                                    draw_selection: {
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 1.0);
                                                            sdf.fill(vec4(0.26, 0.52, 0.96, 0.4));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    draw_cursor: {
                                                        instance focus: 0.0
                                                        instance blink: 0.0
                                                        instance dark_mode: 0.0
                                                        uniform border_radius: 0.5
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius);
                                                            let cursor_color = mix(vec4(0.1, 0.1, 0.12, 1.0), vec4(0.9, 0.9, 0.95, 1.0), self.dark_mode);
                                                            sdf.fill(mix(vec4(0.0, 0.0, 0.0, 0.0), cursor_color, (1.0 - self.blink) * self.focus));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    animator: {
                                                        blink = {
                                                            default: off
                                                            off = {
                                                                from: {all: Forward {duration: 0.5}}
                                                                apply: { draw_cursor: {blink: 0.0} }
                                                            }
                                                            on = {
                                                                from: {all: Forward {duration: 0.5}}
                                                                apply: { draw_cursor: {blink: 1.0} }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                }

                                // Tutor Configuration
                                tutor_config = <RoundedView> {
                                    width: Fill, height: Fit
                                    padding: 16
                                    show_bg: true
                                    draw_bg: {
                                        instance dark_mode: 0.0
                                        instance opacity: 1.0
                                        instance highlight: 0.0
                                        border_radius: 8.0
                                        fn pixel(self) -> vec4 {
                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                            let bg = mix((SLATE_50), (SLATE_700), self.dark_mode);
                                            // Scale effect: shrink from center when highlight > 0
                                            let scale = 1.0 - self.highlight * 0.05;
                                            let offset_x = self.rect_size.x * (1.0 - scale) * 0.5;
                                            let offset_y = self.rect_size.y * (1.0 - scale) * 0.5;
                                            let w = self.rect_size.x * scale;
                                            let h = self.rect_size.y * scale;
                                            sdf.box(offset_x, offset_y, w, h, self.border_radius * scale);
                                            sdf.fill(vec4(bg.x, bg.y, bg.z, bg.w * self.opacity));
                                            return sdf.result;
                                        }
                                    }
                                    flow: Down
                                    spacing: 12

                                    tutor_header = <View> {
                                        width: Fill, height: Fit
                                        flow: Right
                                        spacing: 8
                                        align: {y: 0.5}

                                        tutor_name = <Label> {
                                            text: "Tutor (孙老师)"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                }
                                            }
                                        }

                                        <View> { width: Fill, height: 1 }

                                        tutor_save_btn = <Button> {
                                            width: Fit, height: Fit
                                            padding: {left: 12, right: 12, top: 4, bottom: 4}
                                            text: "Save"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                                fn get_color(self) -> vec4 {
                                                    return (WHITE);
                                                }
                                            }
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                instance hover: 0.0
                                                instance pressed: 0.0
                                                instance saved: 0.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                                    let base = mix((ACCENT_BLUE), (BLUE_600), self.dark_mode);
                                                    let hover_color = mix((BLUE_600), (BLUE_500), self.dark_mode);
                                                    let pressed_color = mix((BLUE_700), (BLUE_400), self.dark_mode);
                                                    let saved_color = vec4(0.2, 0.8, 0.4, 1.0);
                                                    let normal_color = mix(mix(base, hover_color, self.hover), pressed_color, self.pressed);
                                                    let color = mix(normal_color, saved_color, self.saved);
                                                    sdf.fill(color);
                                                    return sdf.result;
                                                }
                                            }
                                        }

                                        tutor_maximize_btn = <View> {
                                            width: 20, height: 20
                                            margin: {left: 8}
                                            cursor: Hand
                                            show_bg: true
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                instance maximized: 0.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    let color = mix(vec4(0.4, 0.45, 0.5, 1.0), vec4(0.7, 0.75, 0.8, 1.0), self.dark_mode);
                                                    let w = self.rect_size.x;
                                                    let h = self.rect_size.y;
                                                    let t = self.maximized;
                                                    // Interpolate between expand (t=0) and collapse (t=1) icons
                                                    // Top-right arrow: L-shape corner moves from inner to outer
                                                    let tr_corner_x = mix(0.71, 0.83, t);
                                                    let tr_corner_y = mix(0.29, 0.17, t);
                                                    let tr_end_x = mix(0.62, 0.58, t);
                                                    let tr_end_y = mix(0.38, 0.42, t);
                                                    sdf.move_to(w * mix(0.58, 0.58, t), h * mix(0.29, 0.26, t));
                                                    sdf.line_to(w * mix(0.71, 0.58, t), h * mix(0.29, 0.42, t));
                                                    sdf.move_to(w * mix(0.71, 0.58, t), h * mix(0.29, 0.42, t));
                                                    sdf.line_to(w * mix(0.71, 0.74, t), h * mix(0.42, 0.42, t));
                                                    // Diagonal line
                                                    sdf.move_to(w * mix(0.46, 0.83, t), h * mix(0.54, 0.17, t));
                                                    sdf.line_to(w * tr_end_x, h * tr_end_y);
                                                    // Bottom-left arrow: L-shape corner moves from inner to outer
                                                    let bl_corner_x = mix(0.29, 0.17, t);
                                                    let bl_corner_y = mix(0.71, 0.83, t);
                                                    let bl_end_x = mix(0.38, 0.42, t);
                                                    let bl_end_y = mix(0.62, 0.58, t);
                                                    sdf.move_to(w * mix(0.29, 0.26, t), h * mix(0.58, 0.58, t));
                                                    sdf.line_to(w * mix(0.29, 0.42, t), h * mix(0.71, 0.58, t));
                                                    sdf.move_to(w * mix(0.29, 0.42, t), h * mix(0.71, 0.58, t));
                                                    sdf.line_to(w * mix(0.42, 0.42, t), h * mix(0.71, 0.74, t));
                                                    // Diagonal line
                                                    sdf.move_to(w * mix(0.54, 0.17, t), h * mix(0.46, 0.83, t));
                                                    sdf.line_to(w * bl_end_x, h * bl_end_y);
                                                    sdf.stroke(color, 1.2);
                                                    return sdf.result;
                                                }
                                            }
                                        }
                                    }

                                    tutor_model_row = <View> {
                                        width: Fill, height: Fit
                                        flow: Right
                                        spacing: 12
                                        align: {y: 0.5}

                                        tutor_model_label = <Label> {
                                            width: 100
                                            text: "Model"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                                }
                                            }
                                        }

                                        tutor_model_dropdown = <DropDown> {
                                            width: 200, height: Fit
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                border_radius: 4.0
                                                border_size: 1.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                                    let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                                                    let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                    sdf.fill(bg);
                                                    sdf.stroke(border, self.border_size);
                                                    return sdf.result;
                                                }
                                            }
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_REGULAR>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                }
                                            }
                                            popup_menu: {
                                                draw_bg: {
                                                    instance dark_mode: 0.0
                                                    border_size: 1.0
                                                    fn pixel(self) -> vec4 {
                                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                                        let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                                                        let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                        sdf.fill(bg);
                                                        sdf.stroke(border, self.border_size);
                                                        return sdf.result;
                                                    }
                                                }
                                                menu_item: {
                                                    indent_width: 10.0
                                                    padding: {left: 15, top: 8, bottom: 8, right: 15}
                                                    draw_bg: {
                                                        instance dark_mode: 0.0
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                                            let base = mix((WHITE), (SLATE_700), self.dark_mode);
                                                            let hover_color = mix((GRAY_100), (SLATE_600), self.dark_mode);
                                                            sdf.fill(mix(base, hover_color, self.hover));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    draw_text: {
                                                        instance dark_mode: 0.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                        }
                                                    }
                                                }
                                            }
                                            labels: ["gpt-4o", "gpt-4o-mini", "deepseek-chat"]
                                            selected_item: 0
                                        }
                                    }

                                    tutor_voice_row = <View> {
                                        width: Fill, height: Fit
                                        flow: Right
                                        spacing: 12
                                        align: {y: 0.5}

                                        tutor_voice_label = <Label> {
                                            width: 100
                                            text: "Voice"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                                }
                                            }
                                        }

                                        tutor_voice_dropdown = <DropDown> {
                                            width: 200, height: Fit
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                border_radius: 4.0
                                                border_size: 1.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                                    let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                                                    let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                    sdf.fill(bg);
                                                    sdf.stroke(border, self.border_size);
                                                    return sdf.result;
                                                }
                                            }
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_REGULAR>{ font_size: 12.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                }
                                            }
                                            popup_menu: {
                                                draw_bg: {
                                                    instance dark_mode: 0.0
                                                    border_size: 1.0
                                                    fn pixel(self) -> vec4 {
                                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                                        let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                                                        let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                        sdf.fill(bg);
                                                        sdf.stroke(border, self.border_size);
                                                        return sdf.result;
                                                    }
                                                }
                                                menu_item: {
                                                    indent_width: 10.0
                                                    padding: {left: 15, top: 8, bottom: 8, right: 15}
                                                    draw_bg: {
                                                        instance dark_mode: 0.0
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                                            let base = mix((WHITE), (SLATE_700), self.dark_mode);
                                                            let hover_color = mix((GRAY_100), (SLATE_600), self.dark_mode);
                                                            sdf.fill(mix(base, hover_color, self.hover));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    draw_text: {
                                                        instance dark_mode: 0.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                        }
                                                    }
                                                }
                                            }
                                            labels: ["Zhao Daniu", "Chen Yifan", "Luo Xiang", "Doubao", "Yang Mi", "Ma Yun", "Maple", "Cove", "Ellen", "Juniper"]
                                            selected_item: 0
                                        }
                                    }

                                    // System Prompt label
                                    tutor_prompt_label = <Label> {
                                        text: "System Prompt"
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                            fn get_color(self) -> vec4 {
                                                return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                            }
                                        }
                                    }

                                    tutor_prompt_container = <RoundedView> {
                                        width: Fill, height: 120
                                        show_bg: true
                                        draw_bg: {
                                            instance dark_mode: 0.0
                                            border_radius: 4.0
                                            fn pixel(self) -> vec4 {
                                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                                let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                                                let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                sdf.fill(bg);
                                                sdf.stroke(border, 1.0);
                                                return sdf.result;
                                            }
                                        }

                                        tutor_prompt_scroll = <ScrollYView> {
                                            width: Fill, height: Fill
                                            scroll_bars: <ScrollBars> {
                                                show_scroll_x: false
                                                show_scroll_y: true
                                                scroll_bar_y: {
                                                    bar_size: 8.0
                                                    bar_side_margin: 2.0
                                                    min_handle_size: 30.0
                                                    smoothing: 0.15
                                                    draw_bg: {
                                                        instance dark_mode: 0.0
                                                        uniform border_radius: 4.0
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            if self.is_vertical > 0.5 {
                                                                sdf.box(
                                                                    1.,
                                                                    self.rect_size.y * self.norm_scroll,
                                                                    self.rect_size.x - 2.0,
                                                                    self.rect_size.y * self.norm_handle,
                                                                    self.border_radius
                                                                );
                                                            } else {
                                                                sdf.box(
                                                                    self.rect_size.x * self.norm_scroll,
                                                                    1.,
                                                                    self.rect_size.x * self.norm_handle,
                                                                    self.rect_size.y - 2.0,
                                                                    self.border_radius
                                                                );
                                                            }
                                                            let base = mix(vec4(0.58, 0.64, 0.69, 1.0), vec4(0.39, 0.45, 0.53, 1.0), self.dark_mode);
                                                            let hover_color = mix(vec4(0.49, 0.55, 0.61, 1.0), vec4(0.49, 0.55, 0.61, 1.0), self.dark_mode);
                                                            sdf.fill(mix(base, hover_color, self.hover));
                                                            return sdf.result;
                                                        }
                                                    }
                                                }
                                            }

                                            tutor_prompt_wrapper = <View> {
                                                width: Fill, height: Fit
                                                padding: 8

                                                tutor_prompt_input = <TextInput> {
                                                    width: Fill, height: Fit
                                                    draw_bg: {
                                                        fn pixel(self) -> vec4 {
                                                            return vec4(0.0, 0.0, 0.0, 0.0);
                                                        }
                                                    }
                                                    draw_text: {
                                                        instance dark_mode: 0.0
                                                        text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                                        fn get_color(self) -> vec4 {
                                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                        }
                                                    }
                                                    draw_selection: {
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 1.0);
                                                            sdf.fill(vec4(0.26, 0.52, 0.96, 0.4));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    draw_cursor: {
                                                        instance focus: 0.0
                                                        instance blink: 0.0
                                                        instance dark_mode: 0.0
                                                        uniform border_radius: 0.5
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius);
                                                            let cursor_color = mix(vec4(0.1, 0.1, 0.12, 1.0), vec4(0.9, 0.9, 0.95, 1.0), self.dark_mode);
                                                            sdf.fill(mix(vec4(0.0, 0.0, 0.0, 0.0), cursor_color, (1.0 - self.blink) * self.focus));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    animator: {
                                                        blink = {
                                                            default: off
                                                            off = {
                                                                from: {all: Forward {duration: 0.5}}
                                                                apply: { draw_cursor: {blink: 0.0} }
                                                            }
                                                            on = {
                                                                from: {all: Forward {duration: 0.5}}
                                                                apply: { draw_cursor: {blink: 1.0} }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                }

                                // Shared System Context Section
                                context_section = <RoundedView> {
                                    width: Fill, height: Fit
                                    padding: 16
                                    show_bg: true
                                    draw_bg: {
                                        instance dark_mode: 0.0
                                        instance opacity: 1.0
                                        instance highlight: 0.0
                                        border_radius: 8.0
                                        fn pixel(self) -> vec4 {
                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                            let bg = mix((SLATE_50), (SLATE_700), self.dark_mode);
                                            // Scale effect: shrink from center when highlight > 0
                                            let scale = 1.0 - self.highlight * 0.05;
                                            let offset_x = self.rect_size.x * (1.0 - scale) * 0.5;
                                            let offset_y = self.rect_size.y * (1.0 - scale) * 0.5;
                                            let w = self.rect_size.x * scale;
                                            let h = self.rect_size.y * scale;
                                            sdf.box(offset_x, offset_y, w, h, self.border_radius * scale);
                                            sdf.fill(vec4(bg.x, bg.y, bg.z, bg.w * self.opacity));
                                            return sdf.result;
                                        }
                                    }
                                    flow: Down
                                    spacing: 12

                                    context_header = <View> {
                                        width: Fill, height: Fit
                                        flow: Right
                                        spacing: 8
                                        align: {y: 0.5}

                                        context_title = <Label> {
                                            text: "Shared System Context (study-context.md)"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                                                fn get_color(self) -> vec4 {
                                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                }
                                            }
                                        }

                                        <View> { width: Fill, height: 1 }

                                        context_save_btn = <Button> {
                                            width: Fit, height: Fit
                                            padding: {left: 12, right: 12, top: 4, bottom: 4}
                                            text: "Save"
                                            draw_text: {
                                                instance dark_mode: 0.0
                                                text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                                fn get_color(self) -> vec4 {
                                                    return (WHITE);
                                                }
                                            }
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                instance hover: 0.0
                                                instance pressed: 0.0
                                                instance saved: 0.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                                    let base = mix((ACCENT_BLUE), (BLUE_600), self.dark_mode);
                                                    let hover_color = mix((BLUE_600), (BLUE_500), self.dark_mode);
                                                    let pressed_color = mix((BLUE_700), (BLUE_400), self.dark_mode);
                                                    let saved_color = vec4(0.2, 0.8, 0.4, 1.0);
                                                    let normal_color = mix(mix(base, hover_color, self.hover), pressed_color, self.pressed);
                                                    let color = mix(normal_color, saved_color, self.saved);
                                                    sdf.fill(color);
                                                    return sdf.result;
                                                }
                                            }
                                        }

                                        context_maximize_btn = <View> {
                                            width: 20, height: 20
                                            margin: {left: 8}
                                            cursor: Hand
                                            show_bg: true
                                            draw_bg: {
                                                instance dark_mode: 0.0
                                                instance maximized: 0.0
                                                fn pixel(self) -> vec4 {
                                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                    let color = mix(vec4(0.4, 0.45, 0.5, 1.0), vec4(0.7, 0.75, 0.8, 1.0), self.dark_mode);
                                                    let w = self.rect_size.x;
                                                    let h = self.rect_size.y;
                                                    let t = self.maximized;
                                                    // Interpolate between expand (t=0) and collapse (t=1) icons
                                                    // Top-right arrow: L-shape corner moves from inner to outer
                                                    let tr_corner_x = mix(0.71, 0.83, t);
                                                    let tr_corner_y = mix(0.29, 0.17, t);
                                                    let tr_end_x = mix(0.62, 0.58, t);
                                                    let tr_end_y = mix(0.38, 0.42, t);
                                                    sdf.move_to(w * mix(0.58, 0.58, t), h * mix(0.29, 0.26, t));
                                                    sdf.line_to(w * mix(0.71, 0.58, t), h * mix(0.29, 0.42, t));
                                                    sdf.move_to(w * mix(0.71, 0.58, t), h * mix(0.29, 0.42, t));
                                                    sdf.line_to(w * mix(0.71, 0.74, t), h * mix(0.42, 0.42, t));
                                                    // Diagonal line
                                                    sdf.move_to(w * mix(0.46, 0.83, t), h * mix(0.54, 0.17, t));
                                                    sdf.line_to(w * tr_end_x, h * tr_end_y);
                                                    // Bottom-left arrow: L-shape corner moves from inner to outer
                                                    let bl_corner_x = mix(0.29, 0.17, t);
                                                    let bl_corner_y = mix(0.71, 0.83, t);
                                                    let bl_end_x = mix(0.38, 0.42, t);
                                                    let bl_end_y = mix(0.62, 0.58, t);
                                                    sdf.move_to(w * mix(0.29, 0.26, t), h * mix(0.58, 0.58, t));
                                                    sdf.line_to(w * mix(0.29, 0.42, t), h * mix(0.71, 0.58, t));
                                                    sdf.move_to(w * mix(0.29, 0.42, t), h * mix(0.71, 0.58, t));
                                                    sdf.line_to(w * mix(0.42, 0.42, t), h * mix(0.71, 0.74, t));
                                                    // Diagonal line
                                                    sdf.move_to(w * mix(0.54, 0.17, t), h * mix(0.46, 0.83, t));
                                                    sdf.line_to(w * bl_end_x, h * bl_end_y);
                                                    sdf.stroke(color, 1.2);
                                                    return sdf.result;
                                                }
                                            }
                                        }
                                    }

                                    context_input_container = <RoundedView> {
                                        width: Fill, height: 200
                                        show_bg: true
                                        draw_bg: {
                                            instance dark_mode: 0.0
                                            border_radius: 4.0
                                            fn pixel(self) -> vec4 {
                                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                                let bg = mix((WHITE), (SLATE_600), self.dark_mode);
                                                let border = mix((SLATE_300), (SLATE_500), self.dark_mode);
                                                sdf.fill(bg);
                                                sdf.stroke(border, 1.0);
                                                return sdf.result;
                                            }
                                        }

                                        context_input_scroll = <ScrollYView> {
                                            width: Fill, height: Fill
                                            scroll_bars: <ScrollBars> {
                                                show_scroll_x: false
                                                show_scroll_y: true
                                                scroll_bar_y: {
                                                    bar_size: 8.0
                                                    bar_side_margin: 2.0
                                                    min_handle_size: 30.0
                                                    smoothing: 0.15
                                                    draw_bg: {
                                                        instance dark_mode: 0.0
                                                        uniform border_radius: 4.0
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            if self.is_vertical > 0.5 {
                                                                sdf.box(
                                                                    1.,
                                                                    self.rect_size.y * self.norm_scroll,
                                                                    self.rect_size.x - 2.0,
                                                                    self.rect_size.y * self.norm_handle,
                                                                    self.border_radius
                                                                );
                                                            } else {
                                                                sdf.box(
                                                                    self.rect_size.x * self.norm_scroll,
                                                                    1.,
                                                                    self.rect_size.x * self.norm_handle,
                                                                    self.rect_size.y - 2.0,
                                                                    self.border_radius
                                                                );
                                                            }
                                                            let base = mix(vec4(0.58, 0.64, 0.69, 1.0), vec4(0.39, 0.45, 0.53, 1.0), self.dark_mode);
                                                            let hover_color = mix(vec4(0.49, 0.55, 0.61, 1.0), vec4(0.49, 0.55, 0.61, 1.0), self.dark_mode);
                                                            sdf.fill(mix(base, hover_color, self.hover));
                                                            return sdf.result;
                                                        }
                                                    }
                                                }
                                            }

                                            context_input_wrapper = <View> {
                                                width: Fill, height: Fit
                                                padding: 8

                                                context_input = <TextInput> {
                                                    width: Fill, height: Fit
                                                    draw_bg: {
                                                        fn pixel(self) -> vec4 {
                                                            return vec4(0.0, 0.0, 0.0, 0.0);
                                                        }
                                                    }
                                                    draw_text: {
                                                        instance dark_mode: 0.0
                                                        text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                                        fn get_color(self) -> vec4 {
                                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                                        }
                                                    }
                                                    draw_selection: {
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 1.0);
                                                            sdf.fill(vec4(0.26, 0.52, 0.96, 0.4));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    draw_cursor: {
                                                        instance focus: 0.0
                                                        instance blink: 0.0
                                                        instance dark_mode: 0.0
                                                        uniform border_radius: 0.5
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius);
                                                            let cursor_color = mix(vec4(0.1, 0.1, 0.12, 1.0), vec4(0.9, 0.9, 0.95, 1.0), self.dark_mode);
                                                            sdf.fill(mix(vec4(0.0, 0.0, 0.0, 0.0), cursor_color, (1.0 - self.blink) * self.focus));
                                                            return sdf.result;
                                                        }
                                                    }
                                                    animator: {
                                                        blink = {
                                                            default: off
                                                            off = {
                                                                from: {all: Forward {duration: 0.5}}
                                                                apply: { draw_cursor: {blink: 0.0} }
                                                            }
                                                            on = {
                                                                from: {all: Forward {duration: 0.5}}
                                                                apply: { draw_cursor: {blink: 1.0} }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                }
                            }

                            // Audio Settings Section
                            audio_section = <View> {
                                width: Fill, height: Fit
                                flow: Down
                                spacing: 12

                                audio_section_title = <Label> {
                                    text: "Audio Settings"
                                    draw_text: {
                                        instance dark_mode: 0.0
                                        text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                                        fn get_color(self) -> vec4 {
                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                        }
                                    }
                                }

                                // Sample rate row
                                sample_rate_row = <View> {
                                    width: Fill, height: Fit
                                    flow: Right
                                    spacing: 12
                                    align: {y: 0.5}

                                    sample_rate_label = <Label> {
                                        width: 120
                                        text: "Sample Rate"
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                            fn get_color(self) -> vec4 {
                                                return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                            }
                                        }
                                    }

                                    sample_rate_value = <Label> {
                                        text: "32000 Hz"
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            text_style: <FONT_REGULAR>{ font_size: 12.0 }
                                            fn get_color(self) -> vec4 {
                                                return mix((GRAY_600), (SLATE_400), self.dark_mode);
                                            }
                                        }
                                    }
                                }

                                // Buffer size row
                                buffer_size_row = <View> {
                                    width: Fill, height: Fit
                                    flow: Right
                                    spacing: 12
                                    align: {y: 0.5}

                                    buffer_size_label = <Label> {
                                        width: 120
                                        text: "Buffer Size"
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                            fn get_color(self) -> vec4 {
                                                return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                            }
                                        }
                                    }

                                    buffer_size_value = <Label> {
                                        text: "5 seconds"
                                        draw_text: {
                                            instance dark_mode: 0.0
                                            text_style: <FONT_REGULAR>{ font_size: 12.0 }
                                            fn get_color(self) -> vec4 {
                                                return mix((GRAY_600), (SLATE_400), self.dark_mode);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Splitter - draggable handle with padding
        splitter = <View> {
            width: 16, height: Fill
            margin: { left: 8, right: 8 }
            align: {y: 0.0}
            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    // Draw thin line in center
                    sdf.rect(7.0, 16.0, 2.0, self.rect_size.y - 32.0);
                    let color = mix((SLATE_300), (SLATE_600), self.dark_mode);
                    sdf.fill(color);
                    return sdf.result;
                }
            }
            cursor: ColResize
        }

        // System Log panel - adaptive width, top-aligned
        log_section = <View> {
            width: 320, height: Fill
            flow: Right
            align: {y: 0.0}

            // Toggle button column
            toggle_column = <View> {
                width: Fit, height: Fill
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    fn pixel(self) -> vec4 {
                        return mix((SLATE_50), (SLATE_800), self.dark_mode);
                    }
                }
                align: {x: 0.5, y: 0.0}
                padding: {left: 4, right: 4, top: 8}

                toggle_log_btn = <Button> {
                    width: Fit, height: Fit
                    padding: {left: 8, right: 8, top: 6, bottom: 6}
                    text: ">"

                    animator: {
                        hover = {
                            default: off,
                            off = {
                                from: {all: Forward {duration: 0.15}}
                                apply: { draw_bg: {hover: 0.0} }
                            }
                            on = {
                                from: {all: Forward {duration: 0.15}}
                                apply: { draw_bg: {hover: 1.0} }
                            }
                        }
                        pressed = {
                            default: off,
                            off = {
                                from: {all: Forward {duration: 0.1}}
                                apply: { draw_bg: {pressed: 0.0} }
                            }
                            on = {
                                from: {all: Forward {duration: 0.1}}
                                apply: { draw_bg: {pressed: 1.0} }
                            }
                        }
                    }

                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: <FONT_BOLD>{ font_size: 11.0 }
                        fn get_color(self) -> vec4 {
                            return mix((SLATE_500), (SLATE_400), self.dark_mode);
                        }
                    }
                    draw_bg: {
                        instance hover: 0.0
                        instance pressed: 0.0
                        instance dark_mode: 0.0
                        border_radius: 4.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let base = mix((SLATE_200), (SLATE_600), self.dark_mode);
                            let hover_color = mix((SLATE_300), (SLATE_500), self.dark_mode);
                            let pressed_color = mix((SLATE_400), (SLATE_400), self.dark_mode);
                            let color = mix(mix(base, hover_color, self.hover), pressed_color, self.pressed);
                            sdf.fill(color);
                            return sdf.result;
                        }
                    }
                }
            }

            // Log content panel
            log_content_column = <RoundedView> {
                width: Fill, height: Fill
                draw_bg: {
                    instance dark_mode: 0.0
                    border_radius: (PANEL_RADIUS)
                    fn get_color(self) -> vec4 {
                        return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                    }
                }
                flow: Down

                log_header = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        fn pixel(self) -> vec4 {
                            return mix((SLATE_50), (SLATE_800), self.dark_mode);
                        }
                    }

                    // Title row
                    log_title_row = <View> {
                        width: Fill, height: Fit
                        padding: {left: 12, right: 12, top: 10, bottom: 6}
                        log_title_label = <Label> {
                            text: "System Log"
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                        }
                    }

                    // Filter row
                    log_filter_row = <View> {
                        width: Fill, height: 32
                        flow: Right
                        align: {y: 0.5}
                        padding: {left: 8, right: 8, bottom: 6}
                        spacing: 6

                        // Level filter dropdown
                        level_filter = <DropDown> {
                            width: 70, height: 24
                            popup_menu_position: BelowInput
                            draw_bg: {
                                instance dark_mode: 0.0
                                border_radius: 2.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 2.0);
                                    let bg = mix((HOVER_BG), (SLATE_700), self.dark_mode);
                                    sdf.fill(bg);
                                    // Down arrow on right side
                                    let ax = self.rect_size.x - 12.0;
                                    let ay = self.rect_size.y * 0.5 - 2.0;
                                    sdf.move_to(ax - 3.0, ay);
                                    sdf.line_to(ax, ay + 4.0);
                                    sdf.line_to(ax + 3.0, ay);
                                    let arrow_color = mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                    sdf.stroke(arrow_color, 1.5);
                                    return sdf.result;
                                }
                            }
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_MEDIUM>{ font_size: 10.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                            popup_menu: {
                                draw_bg: {
                                    instance dark_mode: 0.0
                                    border_size: 1.0
                                    border_radius: 2.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 2.0);
                                        let bg = mix((WHITE), (SLATE_800), self.dark_mode);
                                        let border = mix((BORDER), (SLATE_600), self.dark_mode);
                                        sdf.fill(bg);
                                        sdf.stroke(border, 1.0);
                                        return sdf.result;
                                    }
                                }
                                menu_item: {
                                    draw_bg: {
                                        instance dark_mode: 0.0
                                        fn pixel(self) -> vec4 {
                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                            sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                            let base = mix((WHITE), (SLATE_800), self.dark_mode);
                                            let hover_color = mix((GRAY_100), (SLATE_700), self.dark_mode);
                                            sdf.fill(mix(base, hover_color, self.hover));
                                            return sdf.result;
                                        }
                                    }
                                    draw_text: {
                                        instance dark_mode: 0.0
                                        fn get_color(self) -> vec4 {
                                            let light_base = mix((GRAY_700), (TEXT_PRIMARY), self.active);
                                            let dark_base = mix((SLATE_300), (TEXT_PRIMARY_DARK), self.active);
                                            let base = mix(light_base, dark_base, self.dark_mode);
                                            let light_hover = (TEXT_PRIMARY);
                                            let dark_hover = (TEXT_PRIMARY_DARK);
                                            let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                                            return mix(base, hover_color, self.hover);
                                        }
                                    }
                                }
                            }
                            labels: ["ALL", "DEBUG", "INFO", "WARN", "ERROR"]
                            values: [ALL, DEBUG, INFO, WARN, ERROR]
                        }

                        // Node filter dropdown
                        node_filter = <DropDown> {
                            width: 85, height: 24
                            popup_menu_position: BelowInput
                            draw_bg: {
                                instance dark_mode: 0.0
                                border_radius: 2.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 2.0);
                                    let bg = mix((HOVER_BG), (SLATE_700), self.dark_mode);
                                    sdf.fill(bg);
                                    // Down arrow on right side
                                    let ax = self.rect_size.x - 12.0;
                                    let ay = self.rect_size.y * 0.5 - 2.0;
                                    sdf.move_to(ax - 3.0, ay);
                                    sdf.line_to(ax, ay + 4.0);
                                    sdf.line_to(ax + 3.0, ay);
                                    let arrow_color = mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                    sdf.stroke(arrow_color, 1.5);
                                    return sdf.result;
                                }
                            }
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_MEDIUM>{ font_size: 10.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                            popup_menu: {
                                draw_bg: {
                                    instance dark_mode: 0.0
                                    border_size: 1.0
                                    border_radius: 2.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 2.0);
                                        let bg = mix((WHITE), (SLATE_800), self.dark_mode);
                                        let border = mix((BORDER), (SLATE_600), self.dark_mode);
                                        sdf.fill(bg);
                                        sdf.stroke(border, 1.0);
                                        return sdf.result;
                                    }
                                }
                                menu_item: {
                                    draw_bg: {
                                        instance dark_mode: 0.0
                                        fn pixel(self) -> vec4 {
                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                            sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                                            let base = mix((WHITE), (SLATE_800), self.dark_mode);
                                            let hover_color = mix((GRAY_100), (SLATE_700), self.dark_mode);
                                            sdf.fill(mix(base, hover_color, self.hover));
                                            return sdf.result;
                                        }
                                    }
                                    draw_text: {
                                        instance dark_mode: 0.0
                                        fn get_color(self) -> vec4 {
                                            let light_base = mix((GRAY_700), (TEXT_PRIMARY), self.active);
                                            let dark_base = mix((SLATE_300), (TEXT_PRIMARY_DARK), self.active);
                                            let base = mix(light_base, dark_base, self.dark_mode);
                                            let light_hover = (TEXT_PRIMARY);
                                            let dark_hover = (TEXT_PRIMARY_DARK);
                                            let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                                            return mix(base, hover_color, self.hover);
                                        }
                                    }
                                }
                            }
                            labels: ["All Nodes", "ASR", "TTS", "LLM", "Bridge", "Monitor", "App"]
                            values: [ALL, ASR, TTS, LLM, BRIDGE, MONITOR, APP]
                        }

                        // Search icon
                        search_icon = <View> {
                            width: 20, height: 24
                            align: {x: 0.5, y: 0.5}
                            show_bg: true
                            draw_bg: {
                                instance dark_mode: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let c = self.rect_size * 0.5;
                                    let icon_color = mix((GRAY_500), (SLATE_400), self.dark_mode);
                                    // Magnifying glass circle
                                    sdf.circle(c.x - 2.0, c.y - 2.0, 5.0);
                                    sdf.stroke(icon_color, 1.5);
                                    // Handle
                                    sdf.move_to(c.x + 1.5, c.y + 1.5);
                                    sdf.line_to(c.x + 6.0, c.y + 6.0);
                                    sdf.stroke(icon_color, 1.5);
                                    return sdf.result;
                                }
                            }
                        }

                        // Search field
                        log_search = <TextInput> {
                            width: Fill, height: 24
                            empty_text: "Search..."
                            draw_bg: {
                                instance dark_mode: 0.0
                                border_radius: 2.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                    let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                                    sdf.fill(bg);
                                    return sdf.result;
                                }
                            }
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_REGULAR>{ font_size: 10.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                            draw_selection: {
                                color: (INDIGO_200)
                            }
                            draw_cursor: {
                                color: (ACCENT_BLUE)
                            }
                        }

                        // Copy to clipboard button
                        copy_log_btn = <View> {
                            width: 28, height: 24
                            cursor: Hand
                            show_bg: true
                            draw_bg: {
                                instance copied: 0.0
                                instance dark_mode: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let c = self.rect_size * 0.5;

                                    // Light theme: Green → Teal → Blue → Gray
                                    let gray_light = (BORDER);
                                    let blue_light = vec4(0.231, 0.510, 0.965, 1.0);   // #3b82f6
                                    let teal_light = vec4(0.078, 0.722, 0.651, 1.0);   // #14b8a6
                                    let green_light = vec4(0.133, 0.773, 0.373, 1.0);  // #22c55f

                                    // Dark theme: Bright Green → Cyan → Purple → Slate
                                    let gray_dark = vec4(0.334, 0.371, 0.451, 1.0);    // #555e73 (slate-600)
                                    let purple_dark = vec4(0.639, 0.380, 0.957, 1.0);  // #a361f4
                                    let cyan_dark = vec4(0.133, 0.831, 0.894, 1.0);    // #22d4e4
                                    let green_dark = vec4(0.290, 0.949, 0.424, 1.0);   // #4af26c

                                    // Select colors based on dark mode
                                    let gray = mix(gray_light, gray_dark, self.dark_mode);
                                    let c1 = mix(blue_light, purple_dark, self.dark_mode);
                                    let c2 = mix(teal_light, cyan_dark, self.dark_mode);
                                    let c3 = mix(green_light, green_dark, self.dark_mode);

                                    // Multi-stop gradient based on copied value
                                    let t = self.copied;
                                    let bg_color = mix(
                                        mix(mix(gray, c1, clamp(t * 3.0, 0.0, 1.0)),
                                            c2, clamp((t - 0.33) * 3.0, 0.0, 1.0)),
                                        c3, clamp((t - 0.66) * 3.0, 0.0, 1.0)
                                    );

                                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                                    sdf.fill(bg_color);

                                    // Icon color - white when active, gray otherwise
                                    let icon_base = mix((GRAY_600), vec4(0.580, 0.639, 0.722, 1.0), self.dark_mode);
                                    let icon_color = mix(icon_base, vec4(1.0, 1.0, 1.0, 1.0), smoothstep(0.0, 0.3, self.copied));

                                    // Clipboard icon - back rectangle
                                    sdf.box(c.x - 4.0, c.y - 2.0, 8.0, 9.0, 1.0);
                                    sdf.stroke(icon_color, 1.2);

                                    // Clipboard icon - front rectangle (overlapping)
                                    sdf.box(c.x - 2.0, c.y - 5.0, 8.0, 9.0, 1.0);
                                    sdf.fill(bg_color);
                                    sdf.box(c.x - 2.0, c.y - 5.0, 8.0, 9.0, 1.0);
                                    sdf.stroke(icon_color, 1.2);

                                    return sdf.result;
                                }
                            }
                        }
                    }
                }

                log_scroll = <ScrollYView> {
                    width: Fill, height: Fill
                    flow: Down
                    scroll_bars: <ScrollBars> {
                        show_scroll_x: false
                        show_scroll_y: true
                    }

                    log_content_wrapper = <View> {
                        width: Fill, height: Fit
                        padding: { left: 12, right: 12, top: 8, bottom: 8 }
                        flow: Down

                        // Use Label instead of Markdown for much faster rendering
                        log_content = <Label> {
                            width: Fill, height: Fit
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: <FONT_REGULAR>{ font_size: 10.0 }
                                wrap: Word
                                fn get_color(self) -> vec4 {
                                    return mix((GRAY_600), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                            text: ""
                        }
                    }
                }
            }
        }
    }
}
