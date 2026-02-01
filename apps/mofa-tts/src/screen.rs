//! TTS Screen - Main TTS interface using GPT-SoVITS

use crate::audio_player::TTSPlayer;
use crate::dora_integration::DoraIntegration;
use crate::log_bridge;
use crate::mofa_hero::{ConnectionStatus, MofaHeroAction, MofaHeroWidgetExt};
use crate::voice_clone_modal::{VoiceCloneModalAction, VoiceCloneModalWidgetExt};
use crate::voice_data::TTSStatus;
use crate::voice_selector::{VoiceSelectorAction, VoiceSelectorWidgetExt};
use hound::WavReader;
use makepad_widgets::*;
use std::path::PathBuf;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::*;
    use mofa_ui::widgets::mofa_hero::MofaHero;
    use crate::voice_selector::VoiceSelector;
    use crate::voice_clone_modal::VoiceCloneModal;

    // Confirmation dialog for deleting voices
    ConfirmDeleteModal = <View> {
        width: Fill, height: Fill
        flow: Overlay
        align: {x: 0.5, y: 0.5}
        visible: false

        // Semi-transparent backdrop
        backdrop = <View> {
            width: Fill, height: Fill
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    return vec4(0.0, 0.0, 0.0, 0.5);
                }
            }
        }

        // Dialog content
        dialog = <RoundedView> {
            width: 400, height: Fit
            flow: Down
            spacing: 0

            draw_bg: {
                instance dark_mode: 0.0
                border_radius: 12.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                    let bg = mix((WHITE), (SLATE_800), self.dark_mode);
                    sdf.fill(bg);
                    return sdf.result;
                }
            }

            // Header
            header = <View> {
                width: Fill, height: Fit
                padding: {left: 24, right: 24, top: 20, bottom: 16}
                flow: Down
                spacing: 8

                title = <Label> {
                    width: Fill, height: Fit
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: <FONT_BOLD>{ font_size: 18.0 }
                        fn get_color(self) -> vec4 {
                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                        }
                    }
                    text: "Delete Voice?"
                }

                voice_name = <Label> {
                    width: Fill, height: Fit
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                        fn get_color(self) -> vec4 {
                            return mix((PRIMARY_600), (PRIMARY_400), self.dark_mode);
                        }
                    }
                    text: ""
                }

                message = <Label> {
                    width: Fill, height: Fit
                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: { font_size: 14.0 }
                        fn get_color(self) -> vec4 {
                            return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                        }
                    }
                    text: "This action cannot be undone."
                }
            }

            // Divider
            <View> {
                width: Fill, height: 1
                margin: {top: 8}
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    fn pixel(self) -> vec4 {
                        return mix((BORDER), (BORDER_DARK), self.dark_mode);
                    }
                }
            }

            // Footer with buttons
            footer = <View> {
                width: Fill, height: Fit
                padding: {left: 24, right: 24, top: 16, bottom: 20}
                flow: Right
                align: {x: 1.0, y: 0.5}
                spacing: 12

                cancel_btn = <Button> {
                    width: Fit, height: 36
                    padding: {left: 20, right: 20}
                    text: "Cancel"

                    draw_bg: {
                        instance dark_mode: 0.0
                        instance hover: 0.0
                        border_radius: 6.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let base = mix((SLATE_100), (SLATE_700), self.dark_mode);
                            let hover_color = mix((SLATE_200), (SLATE_600), self.dark_mode);
                            sdf.fill(mix(base, hover_color, self.hover));
                            sdf.stroke(mix((SLATE_300), (SLATE_500), self.dark_mode), 1.0);
                            return sdf.result;
                        }
                    }

                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                        fn get_color(self) -> vec4 {
                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                        }
                    }
                }

                confirm_btn = <Button> {
                    width: Fit, height: 36
                    padding: {left: 20, right: 20}
                    text: "Delete"

                    draw_bg: {
                        instance dark_mode: 0.0
                        instance hover: 0.0
                        border_radius: 6.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let base = mix((RED_500), (RED_600), self.dark_mode);
                            let hover_color = mix((RED_600), (RED_500), self.dark_mode);
                            sdf.fill(mix(base, hover_color, self.hover));
                            return sdf.result;
                        }
                    }

                    draw_text: {
                        text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                        fn get_color(self) -> vec4 {
                            return (WHITE);
                        }
                    }
                }
            }
        }
    }

    // Toast notification for success/error messages
    Toast = <RoundedView> {
        width: Fit, height: Fit
        padding: {left: 16, right: 16, top: 10, bottom: 10}
        visible: false

        draw_bg: {
            instance dark_mode: 0.0
            border_radius: 8.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                // Green success background
                let bg = mix(vec4(0.16, 0.65, 0.37, 0.95), vec4(0.13, 0.55, 0.32, 0.95), self.dark_mode);
                sdf.fill(bg);
                return sdf.result;
            }
        }

        toast_content = <View> {
            width: Fit, height: Fit
            flow: Right
            spacing: 8
            align: {y: 0.5}

            // Checkmark icon
            checkmark = <View> {
                width: 18, height: 18
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        // Draw checkmark
                        sdf.move_to(3.0, 9.0);
                        sdf.line_to(7.0, 13.0);
                        sdf.line_to(15.0, 5.0);
                        sdf.stroke((WHITE), 2.0);
                        return sdf.result;
                    }
                }
            }

            toast_label = <Label> {
                width: Fit, height: Fit
                draw_text: {
                    text_style: <FONT_SEMIBOLD>{ font_size: 12.0 }
                    fn get_color(self) -> vec4 {
                        return (WHITE);
                    }
                }
                text: "Downloaded successfully!"
            }
        }
    }

    // Layout constants
    SECTION_SPACING = 12.0
    PANEL_RADIUS = 6.0
    PANEL_PADDING = 14.0

    // Splitter handle for resizing panels
    Splitter = <View> {
        width: 12, height: Fill
        margin: { left: 6, right: 6 }
        align: {x: 0.5, y: 0.5}
        cursor: ColResize

        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            instance hover: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                // Draw subtle grip dots
                let dot_y_start = self.rect_size.y * 0.35;
                let dot_y_end = self.rect_size.y * 0.65;
                let dot_spacing = 8.0;
                let num_dots = (dot_y_end - dot_y_start) / dot_spacing;
                let color = mix(mix((SLATE_300), (SLATE_600), self.dark_mode), (PRIMARY_400), self.hover);

                // Draw vertical dots
                let y = dot_y_start;
                sdf.circle(6.0, y, 1.5);
                sdf.fill(color);
                sdf.circle(6.0, y + dot_spacing, 1.5);
                sdf.fill(color);
                sdf.circle(6.0, y + dot_spacing * 2.0, 1.5);
                sdf.fill(color);
                sdf.circle(6.0, y + dot_spacing * 3.0, 1.5);
                sdf.fill(color);
                sdf.circle(6.0, y + dot_spacing * 4.0, 1.5);
                sdf.fill(color);

                return sdf.result;
            }
        }
    }

    // Primary button style
    PrimaryButton = <Button> {
        width: Fit, height: 42
        padding: {left: 20, right: 20}

        draw_bg: {
            instance dark_mode: 0.0
            instance disabled: 0.0
            instance hover: 0.0
            instance pressed: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 8.0);

                let base = mix((PRIMARY_500), (PRIMARY_400), self.dark_mode);
                let hover_color = mix((PRIMARY_600), (PRIMARY_300), self.dark_mode);
                let pressed_color = mix((PRIMARY_700), (PRIMARY_500), self.dark_mode);
                let disabled_color = mix((GRAY_300), (GRAY_600), self.dark_mode);

                let color = mix(base, hover_color, self.hover);
                let color = mix(color, pressed_color, self.pressed);
                let color = mix(color, disabled_color, self.disabled);

                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
            fn get_color(self) -> vec4 {
                return (WHITE);
            }
        }
    }

    // Loading spinner for generate button
    GenerateSpinner = <View> {
        width: 20, height: 20
        visible: false

        show_bg: true
        draw_bg: {
            instance rotation: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let center = self.rect_size * 0.5;
                let radius = 7.0;
                let dot_radius = 2.0;
                let num_dots = 6.0;

                let mut result = vec4(0.0, 0.0, 0.0, 0.0);

                // Draw 6 rotating dots in a circle
                for i in 0..6 {
                    let angle = (float(i) / num_dots) * 6.28318530718 + self.rotation * 6.28318530718;
                    let dot_x = center.x + cos(angle) * radius;
                    let dot_y = center.y + sin(angle) * radius;

                    // Calculate opacity based on position (creates rotation effect)
                    let opacity = (float(i) / num_dots) * 0.7 + 0.3;

                    sdf.circle(dot_x, dot_y, dot_radius);
                    let dot_color = vec4(1.0, 1.0, 1.0, opacity);
                    result = mix(result, dot_color, sdf.fill_keep(dot_color).a);
                }

                return result;
            }
        }

        animator: {
            spin = {
                default: off,
                off = {
                    from: {all: Forward {duration: 0.0}}
                    apply: { draw_bg: { rotation: 0.0 } }
                }
                on = {
                    from: {all: Loop {duration: 0.8, end: 1.0}}
                    apply: {
                        draw_bg: { rotation: [{time: 0.0, value: 0.0}, {time: 1.0, value: 1.0}] }
                    }
                }
            }
        }
    }

    // Icon button (circular) for stop
    IconButton = <Button> {
        width: 36, height: 36
        padding: 0

        draw_bg: {
            instance dark_mode: 0.0
            instance hover: 0.0
            instance pressed: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let center = self.rect_size * 0.5;
                sdf.circle(center.x, center.y, 17.0);

                let base = mix((SLATE_100), (SLATE_700), self.dark_mode);
                let hover_color = mix((SLATE_200), (SLATE_600), self.dark_mode);
                let pressed_color = mix((SLATE_300), (SLATE_500), self.dark_mode);

                let color = mix(base, hover_color, self.hover);
                let color = mix(color, pressed_color, self.pressed);

                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            instance dark_mode: 0.0
            text_style: { font_size: 14.0 }
            fn get_color(self) -> vec4 {
                return mix((SLATE_600), (SLATE_300), self.dark_mode);
            }
        }
    }

    // Play button (primary color with play/pause icon)
    PlayButton = <Button> {
        width: 36, height: 36
        padding: 0
        margin: 0

        draw_bg: {
            instance dark_mode: 0.0
            instance hover: 0.0
            instance pressed: 0.0
            instance is_playing: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let center = self.rect_size * 0.5;
                sdf.circle(center.x, center.y, 17.0);

                let base = mix((PRIMARY_500), (PRIMARY_400), self.dark_mode);
                let hover_color = mix((PRIMARY_600), (PRIMARY_300), self.dark_mode);
                let pressed_color = mix((PRIMARY_700), (PRIMARY_500), self.dark_mode);

                let color = mix(base, hover_color, self.hover);
                let color = mix(color, pressed_color, self.pressed);

                sdf.fill(color);

                // Draw play or pause icon
                if self.is_playing > 0.5 {
                    // Pause icon (two vertical bars)
                    sdf.rect(13.0, 12.0, 3.0, 12.0);
                    sdf.fill((WHITE));
                    sdf.rect(20.0, 12.0, 3.0, 12.0);
                    sdf.fill((WHITE));
                } else {
                    // Play icon (triangle) - slightly offset right for optical center
                    sdf.move_to(14.0, 11.0);
                    sdf.line_to(26.0, 18.0);
                    sdf.line_to(14.0, 25.0);
                    sdf.close_path();
                    sdf.fill((WHITE));
                }

                return sdf.result;
            }
        }

        draw_text: {
            text_style: { font_size: 0.0 }
            fn get_color(self) -> vec4 {
                return vec4(0.0, 0.0, 0.0, 0.0);
            }
        }
    }

    // TTS Screen - main layout with bottom audio player bar
    pub TTSScreen = {{TTSScreen}} {
        width: Fill, height: Fill
        flow: Overlay
        spacing: 0
        padding: 0

        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((DARK_BG), (DARK_BG_DARK), self.dark_mode);
            }
        }

        // Content wrapper (contains main layout)
        content_wrapper = <View> {
            width: Fill, height: Fill
            flow: Down
            spacing: 0
            padding: 0

        // Main content area (fills remaining space)
        main_content = <View> {
            width: Fill, height: Fill
            flow: Right
            spacing: 0
            padding: { left: 20, right: 20, top: 16, bottom: 16 }

            // Left column - main content area (adaptive width)
            left_column = <View> {
                width: Fill, height: Fill
                flow: Down
                spacing: 12
                align: {y: 0.0}

                // System status bar (MofaHero)
                hero = <MofaHero> {
                    width: Fill
                }

                // Main content area - text input and voice selector
                content_area = <View> {
                    width: Fill, height: Fill
                    flow: Right
                    spacing: 12

                    // Text input section (fills space)
                    input_section = <RoundedView> {
                        width: Fill, height: Fill
                        flow: Down
                        show_bg: true
                        draw_bg: {
                            instance dark_mode: 0.0
                            border_radius: 6.0
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

                        // Header with title
                        header = <View> {
                            width: Fill, height: Fit
                            padding: {left: 16, right: 16, top: 14, bottom: 14}
                            align: {x: 0.0, y: 0.5}
                            show_bg: true
                            draw_bg: {
                                instance dark_mode: 0.0
                                fn pixel(self) -> vec4 {
                                    return mix((SLATE_50), (SLATE_800), self.dark_mode);
                                }
                            }

                            title = <Label> {
                                width: Fit, height: Fit
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                    }
                                }
                                text: "TTS (GPT-SoVITS)"
                            }
                        }

                        // Text input container
                        input_container = <View> {
                            width: Fill, height: Fill
                            flow: Down
                            padding: {left: 16, right: 16, top: 16, bottom: 12}

                            text_input = <TextInput> {
                                width: Fill, height: Fill
                                padding: {left: 14, right: 14, top: 12, bottom: 12}
                                empty_text: "Enter text to convert to speech..."
                                text: "复杂的问题背后也许没有统一的答案，选择站在正方还是反方，其实取决于你对一系列价值判断的回答。"

                                draw_bg: {
                                    instance dark_mode: 0.0
                                    border_radius: 8.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                        let bg = mix((WHITE), (SLATE_700), self.dark_mode);
                                        let border = mix((SLATE_200), (SLATE_600), self.dark_mode);
                                        sdf.fill(bg);
                                        sdf.stroke(border, 1.0);
                                        return sdf.result;
                                    }
                                }

                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: { font_size: 15.0, line_spacing: 1.6 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                    }
                                }

                                draw_cursor: {
                                    instance focus: 0.0
                                    uniform border_radius: 0.5
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius);
                                        sdf.fill(mix((PRIMARY_500), (PRIMARY_500), self.focus));
                                        return sdf.result;
                                    }
                                }

                                draw_selection: {
                                    instance focus: 0.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 1.0);
                                        sdf.fill(mix(vec4(0.23, 0.51, 0.97, 0.2), vec4(0.23, 0.51, 0.97, 0.35), self.focus));
                                        return sdf.result;
                                    }
                                }
                            }
                        }

                        // Bottom bar with character count and generate button
                        bottom_bar = <View> {
                            width: Fill, height: Fit
                            flow: Right
                            align: {x: 0.0, y: 0.5}
                            padding: {left: 16, right: 16, top: 4, bottom: 16}
                            spacing: 16

                            // Character count
                            char_count = <Label> {
                                width: Fit, height: Fit
                                align: {y: 0.5}
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: { font_size: 12.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_TERTIARY), (TEXT_TERTIARY_DARK), self.dark_mode);
                                    }
                                }
                                text: "45 / 5,000 characters"
                            }

                            <View> { width: Fill, height: 1 }

                            // Generate button with spinner
                            generate_section = <View> {
                                width: Fit, height: Fit
                                flow: Right
                                align: {y: 0.5}
                                spacing: 8

                                // Spinner on the left (hidden by default)
                                generate_spinner = <GenerateSpinner> {}

                                generate_btn = <PrimaryButton> {
                                    text: "Generate Speech"
                                    draw_bg: { disabled: 1.0 }
                                }
                            }
                        }
                    }

                    // Voice selector panel (fixed width)
                    controls_panel = <View> {
                        width: 280, height: Fill
                        flow: Down

                        // Voice selector (fills available space)
                        voice_section = <RoundedView> {
                            width: Fill, height: Fill
                            flow: Down
                            show_bg: true
                            draw_bg: {
                                instance dark_mode: 0.0
                                border_radius: 6.0
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

                            voice_selector = <VoiceSelector> {
                                height: Fill
                            }
                        }
                    }
                }
            }

            // Splitter handle for resizing
            splitter = <Splitter> {}

            // Right Panel: System Log
            log_section = <View> {
                width: 300, height: Fill
                flow: Right
                align: {y: 0.0}

                // Toggle button column
                toggle_column = <View> {
                    width: Fit, height: Fill
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        fn pixel(self) -> vec4 {
                            return mix((SLATE_100), (SLATE_800), self.dark_mode);
                        }
                    }
                    align: {x: 0.5, y: 0.0}
                    padding: {left: 4, right: 4, top: 12}

                    toggle_log_btn = <Button> {
                        width: Fit, height: Fit
                        padding: {left: 6, right: 6, top: 8, bottom: 8}
                        text: ">"

                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: <FONT_BOLD>{ font_size: 10.0 }
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

                // Log content panel with border
                log_content_column = <RoundedView> {
                    width: Fill, height: Fill
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: 6.0
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

                        log_title_row = <View> {
                            width: Fill, height: Fit
                            padding: {left: 14, right: 14, top: 12, bottom: 12}
                            flow: Right
                            align: {x: 0.0, y: 0.5}

                            log_title_label = <Label> {
                                text: "System Log"
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                    }
                                }
                            }

                            <View> { width: Fill, height: 1 }

                            clear_log_btn = <Button> {
                                width: Fit, height: 26
                                padding: {left: 10, right: 10}
                                text: "Clear"
                                draw_bg: {
                                    instance dark_mode: 0.0
                                    instance hover: 0.0
                                    border_radius: 4.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                                        let base = mix((SLATE_100), (SLATE_700), self.dark_mode);
                                        let hover_color = mix((SLATE_200), (SLATE_600), self.dark_mode);
                                        sdf.fill(mix(base, hover_color, self.hover));
                                        return sdf.result;
                                    }
                                }
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: { font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((SLATE_600), (SLATE_300), self.dark_mode);
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
                            padding: { left: 14, right: 14, top: 10, bottom: 10 }
                            flow: Down

                            log_content = <Markdown> {
                                width: Fill, height: Fit
                                font_size: 11.0
                                font_color: (GRAY_600)
                                paragraph_spacing: 6

                                draw_normal: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((SLATE_600), (TEXT_SECONDARY_DARK), self.dark_mode);
                                    }
                                }
                                draw_bold: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((SLATE_700), (TEXT_PRIMARY_DARK), self.dark_mode);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Bottom audio player bar (like minimax)
        audio_player_bar = <View> {
            width: Fill, height: 90
            flow: Right
            align: {x: 0.5, y: 0.5}
            padding: {left: 24, right: 24, top: 8, bottom: 8}
            spacing: 0

            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    // Top border
                    sdf.rect(0.0, 0.0, self.rect_size.x, 1.0);
                    let border = mix((BORDER), (SLATE_700), self.dark_mode);
                    sdf.fill(border);
                    // Background
                    sdf.rect(0.0, 1.0, self.rect_size.x, self.rect_size.y - 1.0);
                    let bg = mix((SURFACE), (SURFACE_DARK), self.dark_mode);
                    sdf.fill(bg);
                    return sdf.result;
                }
            }

            // Left: Voice info (fixed width for balance)
            voice_info = <View> {
                width: 220, height: Fill
                flow: Right
                align: {x: 0.0, y: 0.5}
                spacing: 12

                // Voice avatar
                voice_avatar = <RoundedView> {
                    width: 48, height: 48
                    align: {x: 0.5, y: 0.5}
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: 10.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                            let color = mix((PRIMARY_500), (PRIMARY_400), self.dark_mode);
                            sdf.fill(color);
                            return sdf.result;
                        }
                    }

                    avatar_initial = <Label> {
                        width: Fill, height: Fill
                        align: {x: 0.4, y: 0.7}
                        draw_text: {
                            text_style: <FONT_BOLD>{ font_size: 18.0 }
                            fn get_color(self) -> vec4 {
                                return (WHITE);
                            }
                        }
                        text: "豆"
                    }
                }

                // Voice name
                voice_name_container = <View> {
                    width: Fit, height: Fit
                    flow: Down
                    spacing: 2

                    current_voice_name = <Label> {
                        width: Fit, height: Fit
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                            fn get_color(self) -> vec4 {
                                return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                            }
                        }
                        text: "Doubao"
                    }

                    status_label = <Label> {
                        width: Fit, height: Fit
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: { font_size: 12.0 }
                            fn get_color(self) -> vec4 {
                                return mix((TEXT_TERTIARY), (TEXT_TERTIARY_DARK), self.dark_mode);
                            }
                        }
                        text: "Ready"
                    }
                }
            }

            // Center: Playback controls (fills space)
            playback_controls = <View> {
                width: Fill, height: Fill
                flow: Down
                align: {x: 0.5, y: 0.5}
                spacing: 6

                // Control buttons row - centered
                controls_row = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    align: {x: 0.5, y: 0.5}
                    spacing: 12

                    // Play/Pause button
                    play_btn = <PlayButton> {
                        text: ""
                    }

                    // Stop button
                    // stop_btn = <IconButton> {
                    //     text: "■"
                    // }
                }

                // Progress bar row - centered with max width constraint
                progress_row = <View> {
                    width: 350, height: Fit
                    flow: Right
                    align: {y: 0.5}
                    spacing: 8

                    current_time = <Label> {
                        width: Fit, height: Fit
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: { font_size: 11.0 }
                            fn get_color(self) -> vec4 {
                                return mix((TEXT_TERTIARY), (TEXT_TERTIARY_DARK), self.dark_mode);
                            }
                        }
                        text: "00:00"
                    }

                    // Progress bar container
                    progress_bar_container = <View> {
                        width: Fill, height: 4
                        margin: {top: 2}

                        // Progress bar
                        progress_bar = <View> {
                            width: Fill, height: Fill
                            show_bg: true
                            draw_bg: {
                                instance dark_mode: 0.0
                                instance progress: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    // Background track
                                    sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 2.0);
                                    let track_color = mix((GRAY_200), (GRAY_700), self.dark_mode);
                                    sdf.fill(track_color);
                                    // Progress fill
                                    let progress_width = self.rect_size.x * self.progress;
                                    sdf.box(0.0, 0.0, progress_width, self.rect_size.y, 2.0);
                                    sdf.fill((PRIMARY_500));
                                    return sdf.result;
                                }
                            }
                        }
                    }

                    total_time = <Label> {
                        width: Fit, height: Fit
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: { font_size: 11.0 }
                            fn get_color(self) -> vec4 {
                                return mix((TEXT_TERTIARY), (TEXT_TERTIARY_DARK), self.dark_mode);
                            }
                        }
                        text: "00:00"
                    }
                }
            }

            // Right: Download button (fixed width for balance)
            download_section = <View> {
                width: 140, height: Fill
                align: {x: 1.0, y: 0.5}

                download_btn = <Button> {
                    width: Fit, height: 40
                    padding: {left: 24, right: 24}
                    text: "Download"

                    draw_bg: {
                        instance dark_mode: 0.0
                        instance hover: 0.0
                        instance pressed: 0.0

                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 8.0);

                            // Light blue tint background
                            let base = mix(vec4(0.23, 0.51, 0.97, 0.08), vec4(0.23, 0.51, 0.97, 0.15), self.dark_mode);
                            let hover_color = mix(vec4(0.23, 0.51, 0.97, 0.15), vec4(0.23, 0.51, 0.97, 0.25), self.dark_mode);
                            let pressed_color = mix(vec4(0.23, 0.51, 0.97, 0.25), vec4(0.23, 0.51, 0.97, 0.35), self.dark_mode);
                            let border = mix((PRIMARY_400), (PRIMARY_300), self.dark_mode);

                            let color = mix(base, hover_color, self.hover);
                            let color = mix(color, pressed_color, self.pressed);

                            sdf.fill(color);
                            sdf.stroke(border, 1.5);
                            return sdf.result;
                        }
                    }

                    draw_text: {
                        instance dark_mode: 0.0
                        text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                        fn get_color(self) -> vec4 {
                            return mix((PRIMARY_600), (PRIMARY_300), self.dark_mode);
                        }
                    }
                }
            }
        }
        } // End content_wrapper

        // Voice clone modal (overlay)
        voice_clone_modal = <VoiceCloneModal> {}

        // Confirm delete modal (overlay)
        confirm_delete_modal = <ConfirmDeleteModal> {}

        // Toast notification (top center overlay)
        toast_overlay = <View> {
            width: Fill, height: Fill
            align: {x: 0.5, y: 0.0}
            padding: {top: 80}

            download_toast = <Toast> {}
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct TTSScreen {
    #[deref]
    view: View,

    #[rust]
    tts_status: TTSStatus,

    #[rust]
    audio_player: Option<TTSPlayer>,

    #[rust]
    dora: Option<DoraIntegration>,

    #[rust]
    update_timer: Timer,

    #[rust]
    dark_mode: f64,

    // UI Logic states
    #[rust]
    log_panel_width: f64,
    #[rust]
    log_panel_collapsed: bool,
    #[rust]
    splitter_dragging: bool,
    #[rust]
    log_entries: Vec<String>,
    #[rust]
    logs_initialized: bool,
    #[rust]
    audio_playing_time: f64,

    // Stored audio for playback/download (not auto-play)
    #[rust]
    stored_audio_samples: Vec<f32>,
    #[rust]
    stored_audio_sample_rate: u32,

    // Current voice name for display
    #[rust]
    current_voice_name: String,

    // Preview player for reference audio
    #[rust]
    preview_player: Option<TTSPlayer>,
    #[rust]
    preview_playing_voice_id: Option<String>,

    // Toast notification state
    #[rust]
    toast_timer: Timer,
    #[rust]
    toast_visible: bool,
    #[rust]
    toast_message: String,

    // Delete confirmation state
    #[rust]
    pending_delete_voice_id: Option<String>,
    #[rust]
    pending_delete_voice_name: Option<String>,
}

impl Widget for TTSScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        // IMPORTANT: Use capture_actions to consume child widget events and prevent
        // them from propagating to sibling screens (like mofa-debate)
        let actions = cx.capture_actions(|cx| {
            self.view.handle_event(cx, event, scope);
        });

        // Initialize audio player
        if self.audio_player.is_none() {
            self.audio_player = Some(TTSPlayer::new());
        }

        // Initialize log bridge and timer
        if !self.logs_initialized {
            log_bridge::init();
            self.logs_initialized = true;
            // Start timer for polling
            self.update_timer = cx.start_interval(0.1);
            // Initialize stored audio sample rate (PrimeSpeech uses 32000)
            self.stored_audio_sample_rate = 32000;
            // Initialize voice name
            self.current_voice_name = "Doubao".to_string();
            // Add initial log entries
            self.log_entries
                .push("[INFO] [tts] MoFA TTS initialized".to_string());
            self.log_entries
                .push("[INFO] [tts] Default voice: Doubao (GPT-SoVITS)".to_string());
            self.log_entries
                .push("[INFO] [tts] Click 'Start' to connect to MoFA bridge".to_string());
            // Update log display immediately
            self.update_log_display(cx);

            // Collapse log panel by default
            self.log_panel_collapsed = true;
            self.log_panel_width = 320.0;
            self.view
                .view(ids!(content_wrapper.main_content.log_section))
                .apply_over(cx, live! { width: Fit });
            self.view
                .view(ids!(
                    content_wrapper.main_content.log_section.log_content_column
                ))
                .set_visible(cx, false);
            self.view
                .button(ids!(
                    content_wrapper
                        .main_content
                        .log_section
                        .toggle_column
                        .toggle_log_btn
                ))
                .set_text(cx, "<");
            self.view
                .view(ids!(content_wrapper.main_content.splitter))
                .apply_over(cx, live! { width: 0 });
        }

        // Initialize Dora (lazy, now controlled by MofaHero)
        if self.dora.is_none() {
            let dora = DoraIntegration::new();
            self.dora = Some(dora);
        }

        // Pass SharedDoraState to voice clone modal
        if let Some(ref dora) = self.dora {
            let shared_state = dora.shared_dora_state().clone();
            self.view
                .voice_clone_modal(ids!(voice_clone_modal))
                .set_shared_dora_state(shared_state);
        }

        // Handle toast timer (auto-hide after delay)
        if self.toast_timer.is_event(event).is_some() {
            self.hide_toast(cx);
        }

        // Poll for audio and logs
        if self.update_timer.is_event(event).is_some() {
            // Poll Dora Audio - store audio samples instead of auto-playing
            if let Some(dora) = &self.dora {
                if dora.is_running() {
                    let shared = dora.shared_dora_state();
                    let chunks = shared.audio.drain();
                    if !chunks.is_empty() {
                        for audio in chunks {
                            self.stored_audio_samples.extend(&audio.samples);
                            self.stored_audio_sample_rate = audio.sample_rate;
                        }
                        // Transition to Ready state - user must click Play
                        if self.tts_status == TTSStatus::Generating {
                            let sample_count = self.stored_audio_samples.len();
                            let duration_secs = if self.stored_audio_sample_rate > 0 {
                                sample_count as f32 / self.stored_audio_sample_rate as f32
                            } else {
                                0.0
                            };
                            self.add_log(
                                cx,
                                &format!(
                                    "[INFO] [tts] Audio generated: {} samples, {:.1}s duration",
                                    sample_count, duration_secs
                                ),
                            );
                            self.tts_status = TTSStatus::Ready;
                            self.set_generate_button_loading(cx, false);
                            self.update_player_bar(cx);
                        }
                    }
                }
            }

            // Update playback progress and check if finished
            if self.tts_status == TTSStatus::Playing {
                if let Some(player) = &self.audio_player {
                    // Check if playback has actually finished (buffer empty)
                    if player.check_playback_finished() {
                        // Audio finished - reset to Ready state
                        self.tts_status = TTSStatus::Ready;
                        self.audio_playing_time = 0.0;
                        self.update_playback_progress(cx);
                        self.update_player_bar(cx);
                        self.add_log(cx, "[INFO] [tts] Playback completed");
                    } else if player.is_playing() {
                        // Still playing - update playback time and progress bar
                        self.audio_playing_time += 0.1;
                        self.update_playback_progress(cx);
                    }
                    // If paused (is_playing=false but not finished), do nothing - keep current time
                }
            }

            // Check if preview playback has finished
            if self.preview_playing_voice_id.is_some() {
                if let Some(player) = &self.preview_player {
                    if player.check_playback_finished() {
                        // Preview finished - reset preview state
                        self.preview_playing_voice_id = None;
                        let voice_selector = self.view.voice_selector(ids!(
                            content_wrapper
                                .main_content
                                .left_column
                                .content_area
                                .controls_panel
                                .voice_section
                                .voice_selector
                        ));
                        voice_selector.set_preview_playing(cx, None);
                        self.add_log(cx, "[INFO] [tts] Preview playback finished");
                    }
                }
            }

            // Poll Logs from log_bridge
            let logs = log_bridge::poll_logs();
            if !logs.is_empty() {
                for log_msg in logs {
                    self.log_entries.push(log_msg.format());
                }
                self.update_log_display(cx);
            }
        }

        // Handle MofaHero Actions (Start/Stop)
        // Note: actions are already captured at the beginning of handle_event
        for action in &actions {
            match action.as_widget_action().cast() {
                MofaHeroAction::StartClicked => {
                    self.start_dora(cx);
                }
                MofaHeroAction::StopClicked => {
                    self.stop_dora(cx);
                }
                MofaHeroAction::None => {}
            }
        }

        for action in &actions {

            // Handle voice selector actions
            match action.as_widget_action().cast() {
                VoiceSelectorAction::VoiceSelected(voice_id) => {
                    // Update voice name in player bar
                    self.current_voice_name = voice_id.clone();
                    self.view
                        .label(ids!(
                            content_wrapper
                                .audio_player_bar
                                .voice_info
                                .voice_name_container
                                .current_voice_name
                        ))
                        .set_text(cx, &voice_id);
                    // Update avatar initial
                    let initial = voice_id.chars().next().unwrap_or('?').to_string();
                    self.view
                        .label(ids!(
                            content_wrapper
                                .audio_player_bar
                                .voice_info
                                .voice_avatar
                                .avatar_initial
                        ))
                        .set_text(cx, &initial);
                    self.add_log(cx, &format!("[INFO] [tts] Voice selected: {}", voice_id));
                }
                VoiceSelectorAction::PreviewRequested(voice_id) => {
                    self.handle_preview_request(cx, &voice_id);
                }
                VoiceSelectorAction::RequestStartDora => {
                    // Show toast warning - user must start dora first
                    self.show_toast(
                        cx,
                        "Please click 'Start' button first to initialize the dataflow",
                    );
                    self.add_log(
                        cx,
                        "[WARN] [tts] Dora dataflow must be started before cloning voices",
                    );
                }
                VoiceSelectorAction::CloneVoiceClicked => {
                    // Show the voice clone modal
                    self.view
                        .voice_clone_modal(ids!(voice_clone_modal))
                        .show(cx);
                    self.add_log(cx, "[INFO] [tts] Opening voice clone dialog...");
                }
                VoiceSelectorAction::RequestDeleteConfirmation(voice_id, voice_name) => {
                    // Show confirmation dialog
                    self.pending_delete_voice_id = Some(voice_id);
                    self.pending_delete_voice_name = Some(voice_name.clone());

                    // Update dialog with voice name
                    self.view
                        .label(ids!(confirm_delete_modal.dialog.header.voice_name))
                        .set_text(cx, &format!("\"{}\"", voice_name));

                    // Update dark mode
                    self.update_delete_modal_dark_mode(cx);

                    // Show modal
                    self.view
                        .view(ids!(confirm_delete_modal))
                        .set_visible(cx, true);
                    self.view.redraw(cx);
                }
                VoiceSelectorAction::DeleteVoiceClicked(voice_id) => {
                    // Delete custom voice
                    let voice_selector = self.view.voice_selector(ids!(
                        content_wrapper
                            .main_content
                            .left_column
                            .content_area
                            .controls_panel
                            .voice_section
                            .voice_selector
                    ));
                    match voice_selector.delete_custom_voice(cx, &voice_id) {
                        Ok(_) => {
                            self.add_log(cx, &format!("[INFO] [tts] Deleted voice: {}", voice_id));
                        }
                        Err(e) => {
                            self.add_log(
                                cx,
                                &format!("[ERROR] [tts] Failed to delete voice: {}", e),
                            );
                        }
                    }
                }
                VoiceSelectorAction::None => {}
            }

            // Handle voice clone modal actions
            match action.as_widget_action().cast() {
                VoiceCloneModalAction::VoiceCreated(voice) => {
                    // Add the new voice to the selector
                    let voice_selector = self.view.voice_selector(ids!(
                        content_wrapper
                            .main_content
                            .left_column
                            .content_area
                            .controls_panel
                            .voice_section
                            .voice_selector
                    ));
                    voice_selector.add_custom_voice(cx, voice.clone());
                    self.add_log(
                        cx,
                        &format!("[INFO] [tts] Voice '{}' created successfully!", voice.name),
                    );

                    // Show toast notification
                    self.show_toast(
                        cx,
                        &format!("Custom voice '{}' created successfully!", voice.name),
                    );
                }
                VoiceCloneModalAction::SendAudioToAsr {
                    samples,
                    sample_rate,
                    language,
                    ..
                } => {
                    // Parent screen handles sending audio to ASR via dora
                    self.add_log(
                        cx,
                        &format!("[INFO] [tts] Sending {} samples to ASR...", samples.len()),
                    );

                    if let Some(ref dora) = self.dora {
                        if dora.is_running() {
                            if dora.send_audio(samples, sample_rate, language) {
                                self.add_log(cx, "[INFO] [tts] Audio sent to ASR successfully");
                            } else {
                                self.add_log(cx, "[ERROR] [tts] Failed to send audio to ASR");
                            }
                        } else {
                            self.add_log(
                                cx,
                                "[ERROR] [tts] Dataflow is not running - cannot send audio to ASR",
                            );
                        }
                    } else {
                        self.add_log(cx, "[ERROR] [tts] Dora integration not initialized");
                    }
                }
                VoiceCloneModalAction::Closed => {
                    // Modal closed, nothing to do
                }
                VoiceCloneModalAction::None => {}
            }

            // Handle confirm delete modal buttons
            if self
                .view
                .button(ids!(confirm_delete_modal.dialog.footer.confirm_btn))
                .clicked(&actions)
            {
                // User confirmed - proceed with deletion
                if let Some(voice_id) = self.pending_delete_voice_id.take() {
                    self.pending_delete_voice_name = None;

                    // Delete the voice
                    let voice_selector = self.view.voice_selector(ids!(
                        content_wrapper
                            .main_content
                            .left_column
                            .content_area
                            .controls_panel
                            .voice_section
                            .voice_selector
                    ));
                    match voice_selector.delete_custom_voice(cx, &voice_id) {
                        Ok(_) => {
                            self.add_log(
                                cx,
                                &format!("[INFO] [tts] Voice '{}' deleted successfully", voice_id),
                            );
                        }
                        Err(e) => {
                            self.add_log(
                                cx,
                                &format!("[ERROR] [tts] Failed to delete voice: {}", e),
                            );
                        }
                    }
                }
                // Hide modal
                self.view
                    .view(ids!(confirm_delete_modal))
                    .set_visible(cx, false);
                self.view.redraw(cx);
            }

            if self
                .view
                .button(ids!(confirm_delete_modal.dialog.footer.cancel_btn))
                .clicked(&actions)
            {
                // User cancelled - just hide dialog
                self.pending_delete_voice_id = None;
                self.pending_delete_voice_name = None;
                self.view
                    .view(ids!(confirm_delete_modal))
                    .set_visible(cx, false);
                self.view.redraw(cx);
            }

            // Click backdrop to close dialog
            if self
                .view
                .view(ids!(confirm_delete_modal.backdrop))
                .finger_up(&actions)
                .is_some()
            {
                self.pending_delete_voice_id = None;
                self.pending_delete_voice_name = None;
                self.view
                    .view(ids!(confirm_delete_modal))
                    .set_visible(cx, false);
                self.view.redraw(cx);
            }
        }

        // Handle text input changes
        if self
            .view
            .text_input(ids!(
                main_content
                    .left_column
                    .content_area
                    .input_section
                    .input_container
                    .text_input
            ))
            .changed(&actions)
            .is_some()
        {
            self.update_char_count(cx);
        }

        // Handle generate button
        if self
            .view
            .button(ids!(
                content_wrapper
                    .main_content
                    .left_column
                    .content_area
                    .input_section
                    .bottom_bar
                    .generate_section
                    .generate_btn
            ))
            .clicked(&actions)
        {
            // Check if dora is running and bridges are ready
            if let Some(ref dora) = self.dora {
                if !dora.is_running() {
                    // Not started yet
                    self.show_toast(
                        cx,
                        "Please click 'Start MoFA' button first to initialize the dataflow",
                    );
                } else {
                    // Check if bridges are ready (expected: 4 bridges)
                    let bridge_count = dora.shared_dora_state().status.read().active_bridges.len();
                    if bridge_count < 4 {
                        // Starting but not ready
                        self.show_toast(
                            cx,
                            &format!("Dataflow is starting ({}/4 bridges connected), please wait...", bridge_count),
                        );
                        self.add_log(cx, &format!("[WARN] [tts] Bridges not ready yet: {}/4", bridge_count));
                    } else {
                        // Ready to generate
                        self.generate_speech(cx);
                    }
                }
            } else {
                self.show_toast(cx, "Dora integration not initialized");
            }
        }

        // Handle play button in audio player bar
        if self
            .view
            .button(ids!(
                content_wrapper
                    .audio_player_bar
                    .playback_controls
                    .controls_row
                    .play_btn
            ))
            .clicked(&actions)
        {
            self.toggle_playback(cx);
        }

        // Handle stop button in audio player bar
        if self
            .view
            .button(ids!(
                content_wrapper
                    .audio_player_bar
                    .playback_controls
                    .controls_row
                    .stop_btn
            ))
            .clicked(&actions)
        {
            self.stop_playback(cx);
        }

        // Handle download button in audio player bar
        if self
            .view
            .button(ids!(
                content_wrapper
                    .audio_player_bar
                    .download_section
                    .download_btn
            ))
            .clicked(&actions)
        {
            self.download_audio(cx);
        }

        // Handle clear logs
        if self
            .view
            .button(ids!(
                main_content
                    .log_section
                    .log_content_column
                    .log_header
                    .log_title_row
                    .clear_log_btn
            ))
            .clicked(&actions)
        {
            self.log_entries.clear();
            self.update_log_display(cx);
        }

        // Handle toggle log panel button
        if self
            .view
            .button(ids!(
                content_wrapper
                    .main_content
                    .log_section
                    .toggle_column
                    .toggle_log_btn
            ))
            .clicked(&actions)
        {
            self.toggle_log_panel(cx);
        }

        // Handle splitter
        let splitter = self.view.view(ids!(content_wrapper.main_content.splitter));
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
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl TTSScreen {
    fn add_log(&mut self, cx: &mut Cx, message: &str) {
        self.log_entries.push(message.to_string());
        self.update_log_display(cx);
    }

    fn update_delete_modal_dark_mode(&mut self, cx: &mut Cx) {
        let dark_mode = self.dark_mode;
        self.view
            .view(ids!(confirm_delete_modal.dialog))
            .apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );
        self.view
            .label(ids!(confirm_delete_modal.dialog.header.title))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
        self.view
            .label(ids!(confirm_delete_modal.dialog.header.voice_name))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
        self.view
            .label(ids!(confirm_delete_modal.dialog.header.message))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
        self.view
            .button(ids!(confirm_delete_modal.dialog.footer.cancel_btn))
            .apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
        self.view
            .button(ids!(confirm_delete_modal.dialog.footer.confirm_btn))
            .apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );
    }

    fn update_char_count(&mut self, cx: &mut Cx) {
        let text = self
            .view
            .text_input(ids!(
                main_content
                    .left_column
                    .content_area
                    .input_section
                    .input_container
                    .text_input
            ))
            .text();
        let count = text.chars().count();
        let label = format!("{} / 5,000 characters", count);
        self.view
            .label(ids!(
                main_content
                    .left_column
                    .content_area
                    .input_section
                    .bottom_bar
                    .char_count
            ))
            .set_text(cx, &label);
    }

    fn set_generate_button_loading(&mut self, cx: &mut Cx, loading: bool) {
        // Update button text
        let button_text = if loading {
            "Generating..."
        } else {
            "Generate Speech"
        };
        self.view
            .button(ids!(
                content_wrapper
                    .main_content
                    .left_column
                    .content_area
                    .input_section
                    .bottom_bar
                    .generate_section
                    .generate_btn
            ))
            .set_text(cx, button_text);

        // Show/hide spinner
        self.view
            .view(ids!(
                content_wrapper
                    .main_content
                    .left_column
                    .content_area
                    .input_section
                    .bottom_bar
                    .generate_section
                    .generate_spinner
            ))
            .set_visible(cx, loading);

        // Start/stop spinner animation
        if loading {
            self.view
                .view(ids!(
                    content_wrapper
                        .main_content
                        .left_column
                        .content_area
                        .input_section
                        .bottom_bar
                        .generate_section
                        .generate_spinner
                ))
                .animator_play(cx, ids!(spin.on));
        } else {
            self.view
                .view(ids!(
                    content_wrapper
                        .main_content
                        .left_column
                        .content_area
                        .input_section
                        .bottom_bar
                        .generate_section
                        .generate_spinner
                ))
                .animator_play(cx, ids!(spin.off));
        }

        self.view.redraw(cx);
    }

    fn update_player_bar(&mut self, cx: &mut Cx) {
        // Update status label
        let status_text = match &self.tts_status {
            TTSStatus::Idle => "Ready",
            TTSStatus::Generating => "Generating...",
            TTSStatus::Playing => "Playing",
            TTSStatus::Ready => "Audio Ready",
            TTSStatus::Error(msg) => msg.as_str(),
        };
        self.view
            .label(ids!(
                content_wrapper
                    .audio_player_bar
                    .voice_info
                    .voice_name_container
                    .status_label
            ))
            .set_text(cx, status_text);

        // Update play button state
        let is_playing = self.tts_status == TTSStatus::Playing;
        self.view
            .button(ids!(
                content_wrapper
                    .audio_player_bar
                    .playback_controls
                    .controls_row
                    .play_btn
            ))
            .apply_over(
                cx,
                live! {
                    draw_bg: { is_playing: (if is_playing { 1.0 } else { 0.0 }) }
                },
            );

        // Update total time
        if !self.stored_audio_samples.is_empty() && self.stored_audio_sample_rate > 0 {
            let duration_secs =
                self.stored_audio_samples.len() as f32 / self.stored_audio_sample_rate as f32;
            let mins = (duration_secs / 60.0) as u32;
            let secs = (duration_secs % 60.0) as u32;
            let time_str = format!("{:02}:{:02}", mins, secs);
            self.view
                .label(ids!(
                    content_wrapper
                        .audio_player_bar
                        .playback_controls
                        .progress_row
                        .total_time
                ))
                .set_text(cx, &time_str);
        }

        self.view.redraw(cx);
    }

    fn update_playback_progress(&mut self, cx: &mut Cx) {
        // Calculate total duration and current position
        if self.stored_audio_samples.is_empty() || self.stored_audio_sample_rate == 0 {
            return;
        }

        let total_duration =
            self.stored_audio_samples.len() as f32 / self.stored_audio_sample_rate as f32;
        let current_time = self.audio_playing_time as f32;
        let progress = (current_time / total_duration).min(1.0).max(0.0);

        // Update current time label
        let mins = (current_time / 60.0) as u32;
        let secs = (current_time % 60.0) as u32;
        let time_str = format!("{:02}:{:02}", mins, secs);
        self.view
            .label(ids!(
                content_wrapper
                    .audio_player_bar
                    .playback_controls
                    .progress_row
                    .current_time
            ))
            .set_text(cx, &time_str);

        // Update progress bar
        self.view
            .view(ids!(
                content_wrapper
                    .audio_player_bar
                    .playback_controls
                    .progress_row
                    .progress_bar_container
                    .progress_bar
            ))
            .apply_over(
                cx,
                live! {
                    draw_bg: { progress: (progress as f64) }
                },
            );

        self.view.redraw(cx);
    }

    fn handle_preview_request(&mut self, cx: &mut Cx, voice_id: &str) {
        use crate::voice_data::VoiceSource;
        use crate::voice_persistence;

        // Get the voice selector to check preview audio path
        let voice_selector = self.view.voice_selector(ids!(
            content_wrapper
                .main_content
                .left_column
                .content_area
                .controls_panel
                .voice_section
                .voice_selector
        ));

        // Check if we're stopping a preview
        if self.preview_playing_voice_id.as_ref() == Some(&voice_id.to_string()) {
            // Stop preview
            if let Some(player) = &self.preview_player {
                player.stop();
            }
            self.preview_playing_voice_id = None;
            voice_selector.set_preview_playing(cx, None);
            self.add_log(cx, &format!("[INFO] [tts] Stopped preview: {}", voice_id));
            return;
        }

        // Stop any currently playing preview
        if let Some(player) = &self.preview_player {
            player.stop();
        }

        // Get voice info
        let voice = match voice_selector.get_voice(voice_id) {
            Some(v) => v,
            None => {
                self.add_log(cx, &format!("[ERROR] [tts] Voice not found: {}", voice_id));
                return;
            }
        };

        // Build the audio path based on voice source
        let audio_path = if voice.source == VoiceSource::Custom {
            // Custom voice: use reference_audio_path from persistence
            match voice_persistence::get_reference_audio_path(&voice) {
                Some(path) => path,
                None => {
                    self.add_log(
                        cx,
                        &format!(
                            "[ERROR] [tts] Custom voice has no reference audio: {}",
                            voice_id
                        ),
                    );
                    return;
                }
            }
        } else {
            // Built-in voice: use preview_audio from models directory
            let preview_file = match &voice.preview_audio {
                Some(f) => f.clone(),
                None => {
                    self.add_log(
                        cx,
                        &format!("[WARN] [tts] No preview audio for: {}", voice_id),
                    );
                    return;
                }
            };

            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(".dora")
                .join("models")
                .join("primespeech")
                .join("moyoyo")
                .join("ref_audios")
                .join(&preview_file)
        };

        if !audio_path.exists() {
            self.add_log(
                cx,
                &format!("[ERROR] [tts] Preview audio not found: {:?}", audio_path),
            );
            return;
        }

        // Load WAV file
        match self.load_wav_file(&audio_path) {
            Ok(samples) => {
                // Initialize preview player if needed
                if self.preview_player.is_none() {
                    self.preview_player = Some(TTSPlayer::new());
                }

                // Play the audio
                if let Some(player) = &self.preview_player {
                    player.write_audio(&samples);
                }

                self.preview_playing_voice_id = Some(voice_id.to_string());
                voice_selector.set_preview_playing(cx, Some(voice_id.to_string()));
                self.add_log(
                    cx,
                    &format!(
                        "[INFO] [tts] Playing preview: {} ({:.1}s)",
                        voice_id,
                        samples.len() as f32 / 32000.0
                    ),
                );
            }
            Err(e) => {
                self.add_log(cx, &format!("[ERROR] [tts] Failed to load preview: {}", e));
            }
        }
    }

    fn load_wav_file(&self, path: &PathBuf) -> Result<Vec<f32>, String> {
        let reader = WavReader::open(path).map_err(|e| format!("Failed to open WAV: {}", e))?;
        let spec = reader.spec();
        let sample_rate = spec.sample_rate;
        let channels = spec.channels as usize;

        // Read samples based on format
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                let bits = spec.bits_per_sample;
                let max_val = (1 << (bits - 1)) as f32;
                reader
                    .into_samples::<i32>()
                    .filter_map(Result::ok)
                    .map(|s| s as f32 / max_val)
                    .collect()
            }
            hound::SampleFormat::Float => reader
                .into_samples::<f32>()
                .filter_map(Result::ok)
                .collect(),
        };

        // Convert to mono if stereo
        let mono_samples: Vec<f32> = if channels > 1 {
            samples
                .chunks(channels)
                .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                .collect()
        } else {
            samples
        };

        // Resample to 32000 Hz if needed (TTSPlayer expects 32000 Hz)
        let target_rate = 32000;
        let resampled = if sample_rate != target_rate {
            let ratio = target_rate as f32 / sample_rate as f32;
            let new_len = (mono_samples.len() as f32 * ratio) as usize;
            let mut result = Vec::with_capacity(new_len);
            for i in 0..new_len {
                let src_idx = i as f32 / ratio;
                let idx = src_idx as usize;
                let frac = src_idx - idx as f32;
                let s1 = mono_samples.get(idx).copied().unwrap_or(0.0);
                let s2 = mono_samples.get(idx + 1).copied().unwrap_or(s1);
                result.push(s1 + (s2 - s1) * frac);
            }
            result
        } else {
            mono_samples
        };

        Ok(resampled)
    }

    fn show_toast(&mut self, cx: &mut Cx, message: &str) {
        self.toast_message = message.to_string();
        self.toast_visible = true;

        // Update toast label
        self.view
            .label(ids!(toast_overlay.download_toast.toast_content.toast_label))
            .set_text(cx, message);

        // Show toast
        self.view
            .view(ids!(toast_overlay.download_toast))
            .set_visible(cx, true);

        // Start timer to auto-hide after 3 seconds
        self.toast_timer = cx.start_timeout(3.0);

        self.view.redraw(cx);
    }

    fn hide_toast(&mut self, cx: &mut Cx) {
        self.toast_visible = false;
        self.view
            .view(ids!(toast_overlay.download_toast))
            .set_visible(cx, false);
        self.view.redraw(cx);
    }

    fn update_log_display(&mut self, cx: &mut Cx) {
        let log_text = if self.log_entries.is_empty() {
            "*No log entries*".to_string()
        } else {
            self.log_entries
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n\n")
        };

        self.view
            .markdown(ids!(
                main_content
                    .log_section
                    .log_content_column
                    .log_scroll
                    .log_content_wrapper
                    .log_content
            ))
            .set_text(cx, &log_text);
        self.view.redraw(cx);
    }

    fn toggle_log_panel(&mut self, cx: &mut Cx) {
        self.log_panel_collapsed = !self.log_panel_collapsed;

        if self.log_panel_width == 0.0 {
            self.log_panel_width = 320.0;
        }

        if self.log_panel_collapsed {
            self.view
                .view(ids!(content_wrapper.main_content.log_section))
                .apply_over(cx, live! { width: Fit });
            self.view
                .view(ids!(
                    content_wrapper.main_content.log_section.log_content_column
                ))
                .set_visible(cx, false);
            self.view
                .button(ids!(
                    content_wrapper
                        .main_content
                        .log_section
                        .toggle_column
                        .toggle_log_btn
                ))
                .set_text(cx, "<");
            self.view
                .view(ids!(content_wrapper.main_content.splitter))
                .apply_over(cx, live! { width: 0 });
        } else {
            let width = self.log_panel_width;
            self.view
                .view(ids!(content_wrapper.main_content.log_section))
                .apply_over(cx, live! { width: (width) });
            self.view
                .view(ids!(
                    content_wrapper.main_content.log_section.log_content_column
                ))
                .set_visible(cx, true);
            self.view
                .button(ids!(
                    content_wrapper
                        .main_content
                        .log_section
                        .toggle_column
                        .toggle_log_btn
                ))
                .set_text(cx, ">");
            self.view
                .view(ids!(content_wrapper.main_content.splitter))
                .apply_over(cx, live! { width: 16 });
        }

        self.view.redraw(cx);
    }

    fn resize_log_panel(&mut self, cx: &mut Cx, abs_x: f64) {
        let container_rect = self.view.area().rect(cx);
        let padding = 16.0;
        let new_log_width = (container_rect.pos.x + container_rect.size.x - abs_x - padding)
            .max(150.0)
            .min(container_rect.size.x - 400.0);

        self.log_panel_width = new_log_width;

        self.view
            .view(ids!(content_wrapper.main_content.log_section))
            .apply_over(cx, live! { width: (new_log_width) });

        self.view.redraw(cx);
    }

    fn start_dora(&mut self, cx: &mut Cx) {
        // Check if dora exists and is not running
        let should_start = self.dora.as_ref().map(|d| !d.is_running()).unwrap_or(false);

        if !should_start {
            return;
        }

        let dataflow_path = PathBuf::from("apps/mofa-tts/dataflow/tts.yml");
        if !dataflow_path.exists() {
            self.log_entries.push(
                "[ERROR] [tts] Dataflow file not found: apps/mofa-tts/dataflow/tts.yml".to_string(),
            );
            self.update_log_display(cx);
            self.view
                .mofa_hero(ids!(content_wrapper.main_content.left_column.hero))
                .set_connection_status(cx, ConnectionStatus::Failed);
            return;
        }

        self.log_entries
            .push("[INFO] [tts] Starting TTS dataflow...".to_string());
        self.update_log_display(cx);

        // Start dora
        if let Some(dora) = &mut self.dora {
            dora.start_dataflow(dataflow_path);
        }

        self.view
            .mofa_hero(ids!(content_wrapper.main_content.left_column.hero))
            .set_running(cx, true);
        self.view
            .mofa_hero(ids!(content_wrapper.main_content.left_column.hero))
            .set_connection_status(cx, ConnectionStatus::Connecting);

        self.log_entries
            .push("[INFO] [tts] Dataflow started, connecting...".to_string());
        self.update_log_display(cx);

        self.view
            .mofa_hero(ids!(content_wrapper.main_content.left_column.hero))
            .set_connection_status(cx, ConnectionStatus::Connected);

        self.log_entries
            .push("[INFO] [tts] Connected to MoFA bridge".to_string());
        self.update_log_display(cx);

        // Update voice selector dora running state
        let voice_selector = self.view.voice_selector(ids!(
            content_wrapper
                .main_content
                .left_column
                .content_area
                .controls_panel
                .voice_section
                .voice_selector
        ));
        voice_selector.set_dora_running(cx, true);

        // Enable generate button
        self.view
            .button(ids!(
                content_wrapper
                    .main_content
                    .left_column
                    .content_area
                    .input_section
                    .bottom_bar
                    .generate_section
                    .generate_btn
            ))
            .apply_over(
                cx,
                live! {
                    draw_bg: { disabled: 0.0 }
                },
            );
    }

    fn stop_dora(&mut self, cx: &mut Cx) {
        // Check if dora exists
        if self.dora.is_none() {
            return;
        }

        self.log_entries
            .push("[INFO] [tts] Stopping TTS dataflow...".to_string());
        self.update_log_display(cx);

        // Stop dora
        if let Some(dora) = &mut self.dora {
            dora.stop_dataflow();
        }

        self.view
            .mofa_hero(ids!(content_wrapper.main_content.left_column.hero))
            .set_running(cx, false);
        self.view
            .mofa_hero(ids!(content_wrapper.main_content.left_column.hero))
            .set_connection_status(cx, ConnectionStatus::Stopped);

        self.log_entries
            .push("[INFO] [tts] Dataflow stopped".to_string());
        self.update_log_display(cx);

        // Update voice selector dora running state
        let voice_selector = self.view.voice_selector(ids!(
            content_wrapper
                .main_content
                .left_column
                .content_area
                .controls_panel
                .voice_section
                .voice_selector
        ));
        voice_selector.set_dora_running(cx, false);

        // Disable generate button
        self.view
            .button(ids!(
                content_wrapper
                    .main_content
                    .left_column
                    .content_area
                    .input_section
                    .bottom_bar
                    .generate_section
                    .generate_btn
            ))
            .apply_over(
                cx,
                live! {
                    draw_bg: { disabled: 1.0 }
                },
            );
    }

    fn generate_speech(&mut self, cx: &mut Cx) {
        // Check if Dora is connected
        let is_running = self.dora.as_ref().map(|d| d.is_running()).unwrap_or(false);
        if !is_running {
            self.add_log(
                cx,
                "[WARN] [tts] Bridge not connected. Please start MoFA first.",
            );
            return;
        }

        let text = self
            .view
            .text_input(ids!(
                main_content
                    .left_column
                    .content_area
                    .input_section
                    .input_container
                    .text_input
            ))
            .text();
        if text.is_empty() {
            self.add_log(
                cx,
                "[WARN] [tts] Please enter some text to convert to speech.",
            );
            return;
        }

        let log_text = match text.char_indices().nth(50) {
            Some((idx, _)) => format!("{}...", &text[..idx]),
            None => text.clone(),
        };
        self.add_log(
            cx,
            &format!("[INFO] [tts] Generating speech for: '{}'", log_text),
        );

        let voice_selector = self.view.voice_selector(ids!(
            content_wrapper
                .main_content
                .left_column
                .content_area
                .controls_panel
                .voice_section
                .voice_selector
        ));

        let voice_id = voice_selector
            .selected_voice_id()
            .unwrap_or_else(|| "Luo Xiang".to_string());

        // Get full voice info to check if it's a custom voice
        let voice_info = voice_selector.get_voice(&voice_id);

        self.add_log(cx, &format!("[INFO] [tts] Using voice: {}", voice_id));

        // Clear previous audio
        self.stored_audio_samples.clear();
        self.stored_audio_sample_rate = 32000;

        self.tts_status = TTSStatus::Generating;
        self.set_generate_button_loading(cx, true);
        self.update_player_bar(cx);

        // For PrimeSpeech, encode voice selection in prompt using VOICE: prefix
        // The dora-primespeech node will parse this format
        // For custom voices, use extended format: VOICE:CUSTOM|<ref_audio_path>|<prompt_text>|<language>|<text>
        let prompt = if let Some(voice) = voice_info {
            if voice.source == crate::voice_data::VoiceSource::Custom {
                // Custom voice - need to send reference audio path and prompt text
                if let (Some(ref_audio), Some(prompt_text)) =
                    (&voice.reference_audio_path, &voice.prompt_text)
                {
                    // Get absolute path for reference audio
                    let ref_audio_path = crate::voice_persistence::get_reference_audio_path(&voice)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| ref_audio.clone());

                    self.add_log(
                        cx,
                        &format!("[INFO] [tts] Custom voice ref audio: {}", ref_audio_path),
                    );

                    // Extended format for custom voices
                    // VOICE:CUSTOM|<ref_audio_path>|<prompt_text>|<language>|<text_to_speak>
                    format!(
                        "VOICE:CUSTOM|{}|{}|{}|{}",
                        ref_audio_path, prompt_text, voice.language, text
                    )
                } else {
                    self.add_log(
                        cx,
                        "[WARN] [tts] Custom voice missing ref audio or prompt text, using default",
                    );
                    format!("VOICE:Doubao|{}", text)
                }
            } else {
                // Built-in voice - use simple format
                format!("VOICE:{}|{}", voice_id, text)
            }
        } else {
            // Voice not found, use default
            format!("VOICE:{}|{}", voice_id, text)
        };

        // Send prompt to dora
        let send_result = self
            .dora
            .as_ref()
            .map(|d| d.send_prompt(&prompt))
            .unwrap_or(false);

        if send_result {
            self.add_log(cx, "[INFO] [tts] Prompt sent to TTS engine");
        } else {
            self.add_log(cx, "[ERROR] [tts] Failed to send prompt to Dora");
            self.tts_status = TTSStatus::Error("Failed to send prompt".to_string());
            self.set_generate_button_loading(cx, false);
            self.update_player_bar(cx);
        }

        if let Some(player) = &self.audio_player {
            player.stop();
        }
    }

    fn toggle_playback(&mut self, cx: &mut Cx) {
        if self.tts_status == TTSStatus::Playing {
            // Pause
            if let Some(player) = &self.audio_player {
                player.pause();
            }
            self.tts_status = TTSStatus::Ready;
            self.add_log(cx, &format!("[INFO] [tts] Playback paused at {:.1}s", self.audio_playing_time));
        } else if !self.stored_audio_samples.is_empty() {
            // Check if we're resuming from a paused state or starting fresh
            let total_duration = self.stored_audio_samples.len() as f64 / self.stored_audio_sample_rate as f64;
            let is_resuming = self.audio_playing_time > 0.1
                && self.audio_playing_time < (total_duration - 0.1);

            if let Some(player) = &self.audio_player {
                // Always stop and clear buffer first to avoid audio overlap
                player.stop();

                if is_resuming {
                    // Resume from paused position - write remaining audio from current position
                    let current_sample_index = (self.audio_playing_time * self.stored_audio_sample_rate as f64) as usize;
                    if current_sample_index < self.stored_audio_samples.len() {
                        let remaining_samples = &self.stored_audio_samples[current_sample_index..];
                        player.write_audio(remaining_samples);
                        self.add_log(cx, &format!("[INFO] [tts] Resuming playback from {:.1}s", self.audio_playing_time));
                    }
                } else {
                    // Start from beginning
                    player.write_audio(&self.stored_audio_samples);
                    self.audio_playing_time = 0.0;
                    self.update_playback_progress(cx);
                    self.add_log(cx, "[INFO] [tts] Playing audio...");
                }
            }
            self.tts_status = TTSStatus::Playing;
        } else {
            self.add_log(cx, "[WARN] [tts] No audio to play");
        }
        self.update_player_bar(cx);
    }

    fn stop_playback(&mut self, cx: &mut Cx) {
        if let Some(player) = &self.audio_player {
            player.stop();
        }
        if self.tts_status == TTSStatus::Playing {
            self.tts_status = TTSStatus::Ready;
            self.add_log(cx, "[INFO] [tts] Playback stopped");
        }
        // Reset progress
        self.view
            .label(ids!(
                content_wrapper
                    .audio_player_bar
                    .playback_controls
                    .progress_row
                    .current_time
            ))
            .set_text(cx, "00:00");
        self.update_player_bar(cx);
    }

    fn download_audio(&mut self, cx: &mut Cx) {
        if self.stored_audio_samples.is_empty() {
            self.add_log(cx, "[WARN] [tts] No audio to download");
            return;
        }

        // Generate filename with timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let filename = format!("tts_output_{}.wav", timestamp);

        // Get downloads folder or current directory
        let download_path = if let Some(home) = dirs::home_dir() {
            let downloads = home.join("Downloads");
            if downloads.exists() {
                downloads.join(&filename)
            } else {
                PathBuf::from(&filename)
            }
        } else {
            PathBuf::from(&filename)
        };

        // Write WAV file
        match self.write_wav_file(&download_path) {
            Ok(_) => {
                self.add_log(
                    cx,
                    &format!("[INFO] [tts] Audio saved to: {}", download_path.display()),
                );
                // Show success toast
                self.show_toast(cx, "Downloaded successfully!");
            }
            Err(e) => {
                self.add_log(cx, &format!("[ERROR] [tts] Failed to save audio: {}", e));
            }
        }
    }

    fn write_wav_file(&self, path: &PathBuf) -> std::io::Result<()> {
        use std::io::Write;

        let sample_rate = self.stored_audio_sample_rate;
        let num_channels: u16 = 1;
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * (num_channels as u32) * (bits_per_sample as u32) / 8;
        let block_align: u16 = num_channels * bits_per_sample / 8;
        let data_size = (self.stored_audio_samples.len() * 2) as u32;
        let file_size = 36 + data_size;

        let mut file = std::fs::File::create(path)?;

        // RIFF header
        file.write_all(b"RIFF")?;
        file.write_all(&file_size.to_le_bytes())?;
        file.write_all(b"WAVE")?;

        // fmt chunk
        file.write_all(b"fmt ")?;
        file.write_all(&16u32.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?;
        file.write_all(&num_channels.to_le_bytes())?;
        file.write_all(&sample_rate.to_le_bytes())?;
        file.write_all(&byte_rate.to_le_bytes())?;
        file.write_all(&block_align.to_le_bytes())?;
        file.write_all(&bits_per_sample.to_le_bytes())?;

        // data chunk
        file.write_all(b"data")?;
        file.write_all(&data_size.to_le_bytes())?;

        // Convert f32 samples to i16 and write
        for &sample in &self.stored_audio_samples {
            let clamped = sample.max(-1.0).min(1.0);
            let i16_sample = (clamped * 32767.0) as i16;
            file.write_all(&i16_sample.to_le_bytes())?;
        }

        Ok(())
    }
}

impl TTSScreenRef {
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.dark_mode = dark_mode;
            inner
                .view
                .apply_over(cx, live! { draw_bg: { dark_mode: (dark_mode) } });

            // Apply dark mode to MofaHero
            inner
                .view
                .mofa_hero(ids!(content_wrapper.main_content.left_column.hero))
                .apply_over(cx, live! { draw_bg: { dark_mode: (dark_mode) } });

            // Apply dark mode to voice selector
            inner
                .view
                .voice_selector(ids!(
                    content_wrapper
                        .main_content
                        .left_column
                        .content_area
                        .controls_panel
                        .voice_section
                        .voice_selector
                ))
                .update_dark_mode(cx, dark_mode);

            // Apply dark mode to log markdown
            let log_markdown = inner.view.markdown(ids!(
                content_wrapper
                    .main_content
                    .log_section
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
                },
            );

            // Apply dark mode to audio player bar
            inner
                .view
                .view(ids!(content_wrapper.audio_player_bar))
                .apply_over(cx, live! { draw_bg: { dark_mode: (dark_mode) } });

            // Apply dark mode to voice clone modal
            inner
                .view
                .voice_clone_modal(ids!(voice_clone_modal))
                .update_dark_mode(cx, dark_mode);

            inner.view.redraw(cx);
        }
    }
}
