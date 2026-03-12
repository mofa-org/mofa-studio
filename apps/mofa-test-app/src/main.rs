//! MoFA Test App - Validates mofa-ui components
//!
//! This app tests all the extracted widgets from mofa-ui.

use makepad_widgets::*;
use mofa_dora_bridge::SharedDoraState;
use mofa_ui::{
    ChatMessage, ConnectionStatus, LogLevel, MofaAppData, MofaTheme, ProviderInfo, RoleConfig,
};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::*;

    // Color constants
    DARK_BG = vec4(0.933, 0.941, 0.953, 1.0)
    DARK_BG_DARK = vec4(0.067, 0.090, 0.125, 1.0)
    PANEL_BG = vec4(0.976, 0.980, 0.984, 1.0)
    PANEL_BG_DARK = vec4(0.118, 0.161, 0.231, 1.0)
    TEXT_PRIMARY = vec4(0.067, 0.090, 0.125, 1.0)
    TEXT_PRIMARY_DARK = vec4(0.945, 0.961, 0.976, 1.0)
    TEXT_SECONDARY = vec4(0.392, 0.455, 0.545, 1.0)
    GREEN_500 = vec4(0.133, 0.773, 0.373, 1.0)

    // Simple LED for testing
    TestLed = <View> {
        width: 12, height: 12
        show_bg: true
        draw_bg: {
            instance active: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.circle(self.rect_size.x * 0.5, self.rect_size.y * 0.5, 5.0);
                let off = vec4(0.3, 0.3, 0.3, 1.0);
                let on = (GREEN_500);
                sdf.fill(mix(off, on, self.active));
                return sdf.result;
            }
        }
    }

    // Test button
    TestButton = <Button> {
        width: Fit, height: 36
        padding: {left: 16, right: 16}
        draw_text: {
            text_style: { font_size: 12.0 }
            fn get_color(self) -> vec4 {
                return vec4(1.0, 1.0, 1.0, 1.0);
            }
        }
        draw_bg: {
            instance hover: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, 4.0);
                let base = vec4(0.231, 0.510, 0.965, 1.0);
                let hover_color = vec4(0.369, 0.580, 0.976, 1.0);
                sdf.fill(mix(base, hover_color, self.hover));
                return sdf.result;
            }
        }
        animator: {
            hover = {
                default: off,
                off = { from: {all: Forward {duration: 0.15}} apply: { draw_bg: {hover: 0.0} } }
                on = { from: {all: Forward {duration: 0.15}} apply: { draw_bg: {hover: 1.0} } }
            }
        }
    }

    // Section panel
    TestSection = <RoundedView> {
        width: Fill, height: Fit
        padding: 16
        margin: {bottom: 16}
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            border_radius: 8.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                let bg = mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                sdf.fill(bg);
                return sdf.result;
            }
        }
        flow: Down
        spacing: 12
    }

    // Section title
    SectionTitle = <Label> {
        draw_text: {
            instance dark_mode: 0.0
            text_style: { font_size: 14.0 }
            fn get_color(self) -> vec4 {
                return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
            }
        }
    }

    App = {{App}} {
        ui: <Window> {
            window: { inner_size: vec2(900, 700) }
            pass: { clear_color: (DARK_BG) }

            body = <View> {
                width: Fill, height: Fill
                flow: Down
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    fn pixel(self) -> vec4 {
                        return mix((DARK_BG), (DARK_BG_DARK), self.dark_mode);
                    }
                }

                // Header
                header = <View> {
                    width: Fill, height: 60
                    padding: {left: 20, right: 20}
                    align: {y: 0.5}
                    flow: Right
                    spacing: 16
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        fn pixel(self) -> vec4 {
                            return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                        }
                    }

                    title = <Label> {
                        text: "MoFA UI Test App"
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: { font_size: 20.0 }
                            fn get_color(self) -> vec4 {
                                return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                            }
                        }
                    }

                    <View> { width: Fill, height: 1 }

                    theme_btn = <TestButton> { text: "Toggle Theme" }

                    status_indicator = <View> {
                        width: Fit, height: Fill
                        flow: Right
                        align: {y: 0.5}
                        spacing: 8

                        status_led = <TestLed> {}
                        status_label = <Label> {
                            text: "Tests: Ready"
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: { font_size: 11.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_SECONDARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                        }
                    }
                }

                // Content
                content = <ScrollYView> {
                    width: Fill, height: Fill
                    padding: 20
                    flow: Down
                    scroll_bars: <ScrollBars> {
                        show_scroll_x: false
                        show_scroll_y: true
                    }

                    // Test controls
                    controls_section = <TestSection> {
                        section_title = <SectionTitle> { text: "Test Controls" }

                        controls_row = <View> {
                            width: Fill, height: Fit
                            flow: Right
                            spacing: 12

                            run_all_btn = <TestButton> { text: "Run All Tests" }
                        }
                    }

                    // Results section
                    results_section = <TestSection> {
                        section_title = <SectionTitle> { text: "Test Results" }

                        results_text = <Label> {
                            width: Fill
                            text: "Click 'Run All Tests' to validate mofa-ui components."
                            draw_text: {
                                instance dark_mode: 0.0
                                text_style: { font_size: 12.0 }
                                fn get_color(self) -> vec4 {
                                    return mix((TEXT_SECONDARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    ui: WidgetRef,

    #[rust]
    app_data: MofaAppData,

    #[rust]
    theme: MofaTheme,

    #[rust]
    test_results: Vec<(String, bool, String)>,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        mofa_widgets::live_design(cx);
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.ui
            .handle_event(cx, event, &mut Scope::with_data(&mut self.app_data));

        // Get actions from event
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => return,
        };

        // Theme toggle
        if self.ui.button(ids!(header.theme_btn)).clicked(actions) {
            self.theme.toggle();
            self.app_data.set_dark_mode(!self.app_data.is_dark_mode());
            self.apply_dark_mode(cx);
        }

        // Run all tests
        if self
            .ui
            .button(ids!(content.controls_section.controls_row.run_all_btn))
            .clicked(actions)
        {
            self.run_all_tests(cx);
        }
    }
}

impl App {
    fn run_all_tests(&mut self, cx: &mut Cx) {
        self.test_results.clear();

        // Test 1: Registry
        self.test_registry_internal();

        // Test 2: Theme
        self.test_theme_internal();

        // Test 3: AppData
        self.test_app_data_internal();

        // Test 4: Widget types
        self.test_widget_types();

        // Test 5: Shell components
        self.test_shell_components();

        // Update UI with results
        self.update_results_display(cx);
    }

    fn test_registry_internal(&mut self) {
        let registry = mofa_ui::create_default_registry();

        // Check widget count
        let count = registry.len();
        let expected = 13;
        self.test_results.push((
            "Registry widget count".to_string(),
            count == expected,
            format!("Expected {}, got {}", expected, count),
        ));

        // Check audio widgets
        self.test_results.push((
            "Audio widgets registered".to_string(),
            registry.contains("led_meter")
                && registry.contains("mic_button")
                && registry.contains("aec_button"),
            "led_meter, mic_button, aec_button".to_string(),
        ));

        // Check chat widgets
        self.test_results.push((
            "Chat widgets registered".to_string(),
            registry.contains("chat_panel")
                && registry.contains("chat_input")
                && registry.contains("log_panel"),
            "chat_panel, chat_input, log_panel".to_string(),
        ));

        // Check config widgets
        self.test_results.push((
            "Config widgets registered".to_string(),
            registry.contains("role_editor")
                && registry.contains("dataflow_picker")
                && registry.contains("provider_selector"),
            "role_editor, dataflow_picker, provider_selector".to_string(),
        ));

        // Check shell components
        self.test_results.push((
            "Shell components registered".to_string(),
            registry.contains("mofa_shell")
                && registry.contains("shell_header")
                && registry.contains("shell_sidebar")
                && registry.contains("status_bar"),
            "mofa_shell, shell_header, shell_sidebar, status_bar".to_string(),
        ));
    }

    fn test_theme_internal(&mut self) {
        let mut theme = MofaTheme::default();

        // Test default state
        self.test_results.push((
            "Theme default is light".to_string(),
            !theme.is_dark(),
            format!("is_dark: {}", theme.is_dark()),
        ));

        // Test toggle
        theme.set_dark_mode(true);
        self.test_results.push((
            "Theme toggle to dark".to_string(),
            theme.is_dark(),
            format!("is_dark: {}", theme.is_dark()),
        ));

        // Test target value
        self.test_results.push((
            "Theme target value".to_string(),
            theme.target_value() >= 0.0 && theme.target_value() <= 1.0,
            format!("target_value: {}", theme.target_value()),
        ));
    }

    fn test_app_data_internal(&mut self) {
        let dora_state = SharedDoraState::new();
        let app_data = MofaAppData::new(dora_state);

        // Test default dark mode
        self.test_results.push((
            "AppData default dark mode".to_string(),
            !app_data.is_dark_mode(),
            format!("is_dark_mode: {}", app_data.is_dark_mode()),
        ));

        // Test dora state access
        self.test_results.push((
            "AppData dora state accessible".to_string(),
            true, // If we get here, it works
            "SharedDoraState created and accessible".to_string(),
        ));
    }

    fn test_widget_types(&mut self) {
        // Test that all widget types are accessible

        // Phase 2 - Audio
        let _: fn() -> mofa_ui::LedColors = || mofa_ui::LedColors::default();
        let _: mofa_ui::MicButtonAction = mofa_ui::MicButtonAction::None;
        let _: mofa_ui::AecButtonAction = mofa_ui::AecButtonAction::None;

        self.test_results.push((
            "Audio widget types exported".to_string(),
            true,
            "LedColors, MicButtonAction, AecButtonAction".to_string(),
        ));

        // Phase 3 - Chat
        let _msg = ChatMessage::new("user", "test message");
        let _: mofa_ui::ChatInputAction = mofa_ui::ChatInputAction::None;
        let _: LogLevel = LogLevel::Info;

        self.test_results.push((
            "Chat widget types exported".to_string(),
            true,
            "ChatMessage, ChatInputAction, LogLevel".to_string(),
        ));

        // Phase 4 - Config
        let _config = RoleConfig::default();
        let _: mofa_ui::DataflowPickerAction = mofa_ui::DataflowPickerAction::None;
        let _provider = ProviderInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            models: vec![],
        };

        self.test_results.push((
            "Config widget types exported".to_string(),
            true,
            "RoleConfig, DataflowPickerAction, ProviderInfo".to_string(),
        ));
    }

    fn test_shell_components(&mut self) {
        // Test shell component types
        let _: mofa_ui::MofaShellAction = mofa_ui::MofaShellAction::None;
        let _: mofa_ui::ShellHeaderAction = mofa_ui::ShellHeaderAction::None;
        let _: mofa_ui::ShellSidebarAction = mofa_ui::ShellSidebarAction::None;
        let _: mofa_ui::StatusBarAction = mofa_ui::StatusBarAction::None;
        let _: ConnectionStatus = ConnectionStatus::Connected;
        let _item = mofa_ui::SidebarItem {
            id: "test".to_string(),
            label: "Test".to_string(),
            icon_path: None,
        };

        self.test_results.push((
            "Shell component types exported".to_string(),
            true,
            "MofaShellAction, ShellHeaderAction, ShellSidebarAction, StatusBarAction, ConnectionStatus, SidebarItem".to_string(),
        ));
    }

    fn update_results_display(&mut self, cx: &mut Cx) {
        let mut results_text = String::new();
        let mut pass_count = 0;
        let mut fail_count = 0;

        for (name, passed, details) in &self.test_results {
            let status = if *passed { "PASS" } else { "FAIL" };
            if *passed {
                pass_count += 1;
            } else {
                fail_count += 1;
            }
            results_text.push_str(&format!("[{}] {}\n    {}\n\n", status, name, details));
        }

        let summary = format!(
            "=== SUMMARY: {} passed, {} failed ===\n\n",
            pass_count, fail_count
        );

        results_text = summary + &results_text;

        self.ui
            .label(ids!(content.results_section.results_text))
            .set_text(cx, &results_text);

        // Update status indicator
        let all_passed = fail_count == 0 && pass_count > 0;
        self.ui
            .view(ids!(header.status_indicator.status_led))
            .apply_over(
                cx,
                live! {
                    draw_bg: { active: (if all_passed { 1.0 } else { 0.0 }) }
                },
            );
        self.ui
            .label(ids!(header.status_indicator.status_label))
            .set_text(
                cx,
                &format!("Tests: {}/{} passed", pass_count, pass_count + fail_count),
            );

        self.ui.redraw(cx);
    }

    fn apply_dark_mode(&mut self, cx: &mut Cx) {
        let dark_mode = self.theme.target_value();

        // Apply to main views
        self.ui.view(ids!(body)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );

        self.ui.view(ids!(header)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );

        self.ui.label(ids!(header.title)).apply_over(
            cx,
            live! {
                draw_text: { dark_mode: (dark_mode) }
            },
        );

        self.ui
            .label(ids!(header.status_indicator.status_label))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

        // Apply to sections
        self.ui.view(ids!(content.controls_section)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );
        self.ui.view(ids!(content.results_section)).apply_over(
            cx,
            live! {
                draw_bg: { dark_mode: (dark_mode) }
            },
        );

        // Apply to labels
        self.ui
            .label(ids!(content.controls_section.section_title))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
        self.ui
            .label(ids!(content.results_section.section_title))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
        self.ui
            .label(ids!(content.results_section.results_text))
            .apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

        self.ui.redraw(cx);
    }
}

app_main!(App);

fn main() {
    app_main();
}
