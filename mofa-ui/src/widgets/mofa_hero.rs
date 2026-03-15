//! MofaHero Widget - System status bar with Dataflow, CPU, Memory, GPU, and VRAM panels
//!
//! A shared hero widget that displays system monitoring information and
//! provides dataflow start/stop controls.

use crate::system_monitor;
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use mofa_widgets::theme::*;

    // Local layout constants (colors imported from theme)
    HERO_RADIUS = 4.0

    // Dark mode colors
    use mofa_widgets::theme::PANEL_BG_DARK;
    use mofa_widgets::theme::TEXT_PRIMARY_DARK;
    use mofa_widgets::theme::TEXT_SECONDARY_DARK;

    // Icons
    ICO_START = dep("crate://self/resources/icons/start.svg")
    ICO_STOP = dep("crate://self/resources/icons/stop.svg")

    // Dataflow status button (Ready/Connected/Failed with color change) with hover animation
    DataflowButton = <Button> {
        width: Fill, height: 22
        text: "Connected"

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
            text_style: <FONT_SEMIBOLD>{ font_size: 10.0 }
            text_wrap: Word
            fn get_color(self) -> vec4 {
                return self.color;
            }
        }
        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance status: 0.0  // 0=ready(green), 1=connected(neon green, blinking), 2=failed(red)
            instance blink: 0.0   // blink phase for animation
            border_radius: 4.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let green = vec4(0.13, 0.77, 0.37, 1.0);        // Ready green
                let neon_green = vec4(0.0, 1.0, 0.5, 1.0);       // Connected neon green
                let red = vec4(0.95, 0.25, 0.25, 1.0);           // Failed red

                // Calculate color based on status
                let base_color = mix(
                    green,
                    red,
                    step(1.5, self.status)  // Switch to red when status > 1.5
                );

                // Apply blinking for connected state (status ~ 1.0)
                let blink_factor = smoothstep(
                    0.4 - 0.1 * sin(self.blink * 12.566),
                    0.6 + 0.1 * sin(self.blink * 12.566),
                    step(0.5, self.status) * (1.0 - step(1.5, self.status))
                );

                // Mix between base color and neon green based on blink
                let final_color = mix(base_color, neon_green, blink_factor * step(0.5, self.status) * (1.0 - step(1.5, self.status)));

                // Darken on hover/press
                let hover_darken = 0.85;
                let press_darken = 0.75;
                let darken = mix(1.0, mix(hover_darken, press_darken, self.pressed), self.hover);
                let color = final_color * darken;

                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.border_radius);
                sdf.fill(vec4(color.xyz, final_color.w));
                return sdf.result;
            }
        }
    }

    // Reusable status dot component
    StatusDot = <View> {
        width: 10, height: 10
        show_bg: true
        draw_bg: {
            instance status: 0.0  // 0=inactive(gray), 1=good(green), 2=warning(yellow), 3=critical(red)

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let center = self.rect_size * 0.5;
                let radius = min(center.x, center.y) - 0.5;
                sdf.circle(center.x, center.y, radius);

                let gray = vec4(0.62, 0.65, 0.69, 1.0);
                let green = vec4(0.13, 0.77, 0.37, 1.0);
                let yellow = vec4(0.95, 0.75, 0.2, 1.0);
                let red = vec4(0.95, 0.25, 0.25, 1.0);

                let color = mix(
                    mix(gray, green, step(0.5, self.status)),
                    mix(yellow, red, step(2.5, self.status)),
                    step(1.5, self.status)
                );
                sdf.fill(color);
                return sdf.result;
            }
        }
    }

    // Connection dot (for dataflow status - green when connected)
    ConnectionDot = <View> {
        width: 10, height: 10
        show_bg: true
        draw_bg: {
            color: (GRAY_400)

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let center = self.rect_size * 0.5;
                let radius = min(center.x, center.y) - 0.5;
                sdf.circle(center.x, center.y, radius);
                sdf.fill(self.color);
                return sdf.result;
            }
        }
    }

    // Reusable LED gauge component (10 segments)
    LedGauge = <View> {
        width: Fill, height: 16
        show_bg: true
        draw_bg: {
            instance fill_pct: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                let num_segs = 10.0;
                let gap = 2.0;
                let seg_width = (self.rect_size.x - gap * (num_segs + 1.0)) / num_segs;
                let seg_height = self.rect_size.y - 2.0;
                let active_segs = self.fill_pct * num_segs;
                let dim = vec4(0.90, 0.91, 0.93, 1.0);

                // Colors: blue -> green -> yellow -> red
                let c0 = vec4(0.23, 0.51, 0.97, 1.0);
                let c1 = vec4(0.20, 0.65, 0.90, 1.0);
                let c2 = vec4(0.20, 0.78, 0.75, 1.0);
                let c3 = vec4(0.20, 0.83, 0.55, 1.0);
                let c4 = vec4(0.30, 0.85, 0.40, 1.0);
                let c5 = vec4(0.55, 0.85, 0.30, 1.0);
                let c6 = vec4(0.80, 0.82, 0.22, 1.0);
                let c7 = vec4(0.95, 0.70, 0.20, 1.0);
                let c8 = vec4(0.95, 0.50, 0.20, 1.0);
                let c9 = vec4(0.95, 0.30, 0.20, 1.0);

                // Draw each segment manually
                let x0 = gap + 0.0 * (seg_width + gap);
                sdf.box(x0, 1.0, seg_width, seg_height, 1.5);
                sdf.fill(mix(dim, c0, step(0.5, active_segs)));

                let x1 = gap + 1.0 * (seg_width + gap);
                sdf.box(x1, 1.0, seg_width, seg_height, 1.5);
                sdf.fill(mix(dim, c1, step(1.5, active_segs)));

                let x2 = gap + 2.0 * (seg_width + gap);
                sdf.box(x2, 1.0, seg_width, seg_height, 1.5);
                sdf.fill(mix(dim, c2, step(2.5, active_segs)));

                let x3 = gap + 3.0 * (seg_width + gap);
                sdf.box(x3, 1.0, seg_width, seg_height, 1.5);
                sdf.fill(mix(dim, c3, step(3.5, active_segs)));

                let x4 = gap + 4.0 * (seg_width + gap);
                sdf.box(x4, 1.0, seg_width, seg_height, 1.5);
                sdf.fill(mix(dim, c4, step(4.5, active_segs)));

                let x5 = gap + 5.0 * (seg_width + gap);
                sdf.box(x5, 1.0, seg_width, seg_height, 1.5);
                sdf.fill(mix(dim, c5, step(5.5, active_segs)));

                let x6 = gap + 6.0 * (seg_width + gap);
                sdf.box(x6, 1.0, seg_width, seg_height, 1.5);
                sdf.fill(mix(dim, c6, step(6.5, active_segs)));

                let x7 = gap + 7.0 * (seg_width + gap);
                sdf.box(x7, 1.0, seg_width, seg_height, 1.5);
                sdf.fill(mix(dim, c7, step(7.5, active_segs)));

                let x8 = gap + 8.0 * (seg_width + gap);
                sdf.box(x8, 1.0, seg_width, seg_height, 1.5);
                sdf.fill(mix(dim, c8, step(8.5, active_segs)));

                let x9 = gap + 9.0 * (seg_width + gap);
                sdf.box(x9, 1.0, seg_width, seg_height, 1.5);
                sdf.fill(mix(dim, c9, step(9.5, active_segs)));

                return sdf.result;
            }
        }
    }

    // Status section template - equal width for all sections
    StatusSection = <RoundedView> {
        width: Fill, height: Fill
        padding: { left: 12, right: 12, top: 8, bottom: 8 }
        draw_bg: {
            instance dark_mode: 0.0
            border_radius: (HERO_RADIUS)
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let r = self.border_radius;
                let bg = mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, r);
                sdf.fill(bg);
                return sdf.result;
            }
        }
        flow: Down
        spacing: 4
        align: {x: 0.0, y: 0.0}
    }

    // Status label with dark mode support
    StatusLabel = <Label> {
        draw_text: {
            instance dark_mode: 0.0
            text_style: <FONT_MEDIUM>{ font_size: 10.0 }
            fn get_color(self) -> vec4 {
                return mix((GRAY_700), (TEXT_SECONDARY_DARK), self.dark_mode);
            }
        }
    }

    // Percentage label with dark mode support
    PctLabel = <Label> {
        draw_text: {
            instance dark_mode: 0.0
            text_style: <FONT_REGULAR>{ font_size: 10.0 }
            fn get_color(self) -> vec4 {
                return mix((GRAY_500), (TEXT_SECONDARY_DARK), self.dark_mode);
            }
        }
    }

    pub MofaHero = {{MofaHero}} {
        width: Fill, height: 72
        flow: Right
        spacing: 8

        // Action section (start/stop toggle) - matches conference-dashboard pattern
        action_section = <RoundedView> {
            width: Fill, height: Fill
            padding: { left: 12, right: 12, top: 8, bottom: 8 }
            draw_bg: {
                instance dark_mode: 0.0
                border_radius: (HERO_RADIUS)
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let r = self.border_radius;
                    let bg = mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                    sdf.box(0., 0., self.rect_size.x, self.rect_size.y, r);
                    sdf.fill(bg);
                    return sdf.result;
                }
            }
            flow: Down
            spacing: 4
            align: {x: 0.5, y: 0.5}

            // Start state (visible when not running)
            start_view = <View> {
                width: Fill, height: Fill
                flow: Down
                spacing: 4
                align: {x: 0.5, y: 0.5}
                cursor: Hand

                action_start_label = <StatusLabel> {
                    text: "Start MoFA"
                }

                start_btn = <View> {
                    width: 24, height: 20
                    align: {x: 0.5, y: 0.5}
                    <Icon> {
                        draw_icon: {
                            svg_file: (ICO_START)
                            fn get_color(self) -> vec4 {
                                return vec4(0.133, 0.773, 0.373, 1.0);  // Green #22c55e
                            }
                        }
                        icon_walk: {width: 20, height: 20}
                    }
                }
            }

            // Stop state (hidden by default, shown when running)
            stop_view = <View> {
                visible: false
                width: Fill, height: Fill
                flow: Down
                spacing: 4
                align: {x: 0.5, y: 0.5}
                cursor: Hand

                action_stop_label = <StatusLabel> {
                    text: "Stop MoFA"
                }

                stop_btn = <View> {
                    width: 24, height: 20
                    align: {x: 0.5, y: 0.5}
                    <Icon> {
                        draw_icon: {
                            svg_file: (ICO_STOP)
                            fn get_color(self) -> vec4 {
                                return vec4(0.937, 0.267, 0.267, 1.0);  // Red #ef4444
                            }
                        }
                        icon_walk: {width: 20, height: 20}
                    }
                }
            }
        }

        // Dataflow status section - matches conference-dashboard pattern
        connection_section = <StatusSection> {
            <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 6
                align: {x: 0.0, y: 0.5}

                connection_dot = <ConnectionDot> {}
                dataflow_label = <StatusLabel> {
                    text: "Dataflow"
                }
            }

            <View> {
                width: Fill
                height: Fill
                flow: Down
                align: {x: 0.0, y: 0.0}
                margin: {top: -6}

                dataflow_btn = <DataflowButton> {}
            }
        }

        // CPU section
        cpu_section = <StatusSection> {
            <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 6
                align: {x: 0.0, y: 0.5}

                cpu_dot = <StatusDot> {}
                cpu_label = <StatusLabel> {
                    text: "CPU"
                }
            }

            cpu_gauge = <LedGauge> {}

            cpu_pct = <PctLabel> {
                text: "0%"
            }
        }

        // Memory section
        memory_section = <StatusSection> {
            <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 6
                align: {x: 0.0, y: 0.5}

                memory_dot = <StatusDot> {}
                memory_label = <StatusLabel> {
                    text: "Memory"
                }
            }

            memory_gauge = <LedGauge> {}

            memory_pct = <PctLabel> {
                text: "0%"
            }
        }

        // GPU section
        gpu_section = <StatusSection> {
            <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 6
                align: {x: 0.0, y: 0.5}

                gpu_dot = <StatusDot> {}
                gpu_label = <StatusLabel> {
                    text: "GPU"
                }
            }

            gpu_gauge = <LedGauge> {}

            gpu_pct = <PctLabel> {
                text: "N/A"
            }
        }

        // VRAM section
        vram_section = <StatusSection> {
            <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 6
                align: {x: 0.0, y: 0.5}

                vram_dot = <StatusDot> {}
                vram_label = <StatusLabel> {
                    text: "VRAM"
                }
            }

            vram_gauge = <LedGauge> {}

            vram_pct = <PctLabel> {
                text: "N/A"
            }
        }
    }
}

/// Actions emitted by MofaHero
#[derive(Clone, Debug, DefaultNone)]
pub enum MofaHeroAction {
    None,
    StartClicked,
    StopClicked,
}

#[derive(Live, LiveHook, Widget)]
pub struct MofaHero {
    #[deref]
    view: View,

    #[rust]
    is_running: bool,

    #[rust]
    cpu_usage: f64,

    #[rust]
    memory_usage: f64,

    #[rust]
    gpu_usage: f64,

    #[rust]
    vram_usage: f64,

    #[rust]
    connection_status: ConnectionStatus,

    #[rust]
    timer: Timer,

    #[rust]
    monitor_started: bool,

    #[rust]
    blink_phase: f64, // For blinking animation
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ConnectionStatus {
    Ready,
    Connecting,
    #[default]
    Connected,
    Stopping,
    Stopped,
    Failed,
}

impl Widget for MofaHero {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Start background system monitor on first event
        if !self.monitor_started {
            system_monitor::start_system_monitor();
            self.monitor_started = true;
            // Start timer for periodic UI updates (every 1 second)
            self.timer = cx.start_interval(1.0);
        }

        // Handle timer for system stats updates
        if self.timer.is_event(event).is_some() {
            self.update_system_stats(cx);
        }

        // Handle start/stop button clicks (using view containers to match conference-dashboard)
        let start_view = self.view.view(ids!(action_section.start_view));
        let stop_view = self.view.view(ids!(action_section.stop_view));

        match event.hits(cx, start_view.area()) {
            Hit::FingerUp(_) => {
                cx.widget_action(self.widget_uid(), &scope.path, MofaHeroAction::StartClicked);
            }
            _ => {}
        }

        match event.hits(cx, stop_view.area()) {
            Hit::FingerUp(_) => {
                cx.widget_action(self.widget_uid(), &scope.path, MofaHeroAction::StopClicked);
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Update blink animation for connected state (60Hz blinking)
        let time = Cx::time_now();
        self.blink_phase = time * 6.0; // Scaled for smooth blinking

        // Apply blink value to button when connected
        if self.connection_status == ConnectionStatus::Connected {
            self.view
                .button(ids!(connection_section.dataflow_btn))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { blink: (self.blink_phase) }
                    },
                );
            // Request next frame for continuous animation
            cx.new_next_frame();
        }

        self.view.draw_walk(cx, scope, walk)
    }
}

impl MofaHero {
    /// Update system stats from background monitor
    fn update_system_stats(&mut self, cx: &mut Cx) {
        // Read values from background system monitor
        let cpu_usage = system_monitor::get_cpu_usage();
        let memory_usage = system_monitor::get_memory_usage();
        let gpu_usage = system_monitor::get_gpu_usage();
        let vram_usage = system_monitor::get_vram_usage();
        let gpu_available = system_monitor::is_gpu_available();

        // Update UI with the values
        self.set_cpu_usage_internal(cx, cpu_usage);
        self.set_memory_usage_internal(cx, memory_usage);
        self.set_gpu_usage_internal(cx, gpu_usage, gpu_available);
        self.set_vram_usage_internal(cx, vram_usage, gpu_available);
    }

    /// Set the running state (shows start or stop view - matches conference-dashboard)
    pub fn set_running(&mut self, cx: &mut Cx, running: bool) {
        self.is_running = running;
        self.view
            .view(ids!(action_section.start_view))
            .set_visible(cx, !running);
        self.view
            .view(ids!(action_section.stop_view))
            .set_visible(cx, running);
        self.view.redraw(cx);
    }

    /// Internal GPU usage update
    fn set_gpu_usage_internal(&mut self, cx: &mut Cx, usage: f64, available: bool) {
        self.gpu_usage = usage.clamp(0.0, 1.0);

        if available {
            self.view.view(ids!(gpu_section.gpu_gauge)).apply_over(
                cx,
                live! {
                    draw_bg: { fill_pct: (self.gpu_usage) }
                },
            );

            let pct_text = format!("{}%", (self.gpu_usage * 100.0) as u32);
            self.view
                .label(ids!(gpu_section.gpu_pct))
                .set_text(cx, &pct_text);

            let status = if self.gpu_usage < 0.7 {
                1.0
            } else if self.gpu_usage < 0.9 {
                2.0
            } else {
                3.0
            };
            self.view.view(ids!(gpu_section.gpu_dot)).apply_over(
                cx,
                live! {
                    draw_bg: { status: (status) }
                },
            );
        } else {
            self.view
                .label(ids!(gpu_section.gpu_pct))
                .set_text(cx, "N/A");
            self.view.view(ids!(gpu_section.gpu_dot)).apply_over(
                cx,
                live! {
                    draw_bg: { status: 0.0 }
                },
            );
        }

        self.view.redraw(cx);
    }

    /// Internal VRAM usage update
    fn set_vram_usage_internal(&mut self, cx: &mut Cx, usage: f64, available: bool) {
        self.vram_usage = usage.clamp(0.0, 1.0);

        if available {
            self.view.view(ids!(vram_section.vram_gauge)).apply_over(
                cx,
                live! {
                    draw_bg: { fill_pct: (self.vram_usage) }
                },
            );

            let pct_text = format!("{}%", (self.vram_usage * 100.0) as u32);
            self.view
                .label(ids!(vram_section.vram_pct))
                .set_text(cx, &pct_text);

            let status = if self.vram_usage < 0.7 {
                1.0
            } else if self.vram_usage < 0.9 {
                2.0
            } else {
                3.0
            };
            self.view.view(ids!(vram_section.vram_dot)).apply_over(
                cx,
                live! {
                    draw_bg: { status: (status) }
                },
            );
        } else {
            self.view
                .label(ids!(vram_section.vram_pct))
                .set_text(cx, "N/A");
            self.view.view(ids!(vram_section.vram_dot)).apply_over(
                cx,
                live! {
                    draw_bg: { status: 0.0 }
                },
            );
        }

        self.view.redraw(cx);
    }

    /// Internal CPU update (doesn't trigger external notification)
    fn set_cpu_usage_internal(&mut self, cx: &mut Cx, usage: f64) {
        self.cpu_usage = usage.clamp(0.0, 1.0);

        self.view.view(ids!(cpu_section.cpu_gauge)).apply_over(
            cx,
            live! {
                draw_bg: { fill_pct: (self.cpu_usage) }
            },
        );

        let pct_text = format!("{}%", (self.cpu_usage * 100.0) as u32);
        self.view
            .label(ids!(cpu_section.cpu_pct))
            .set_text(cx, &pct_text);

        let status = if self.cpu_usage < 0.7 {
            1.0
        } else if self.cpu_usage < 0.9 {
            2.0
        } else {
            3.0
        };
        self.view.view(ids!(cpu_section.cpu_dot)).apply_over(
            cx,
            live! {
                draw_bg: { status: (status) }
            },
        );

        self.view.redraw(cx);
    }

    /// Set CPU usage (0.0 - 1.0)
    pub fn set_cpu_usage(&mut self, cx: &mut Cx, usage: f64) {
        self.set_cpu_usage_internal(cx, usage);
    }

    /// Internal memory update
    fn set_memory_usage_internal(&mut self, cx: &mut Cx, usage: f64) {
        self.memory_usage = usage.clamp(0.0, 1.0);

        self.view
            .view(ids!(memory_section.memory_gauge))
            .apply_over(
                cx,
                live! {
                    draw_bg: { fill_pct: (self.memory_usage) }
                },
            );

        let pct_text = format!("{}%", (self.memory_usage * 100.0) as u32);
        self.view
            .label(ids!(memory_section.memory_pct))
            .set_text(cx, &pct_text);

        let status = if self.memory_usage < 0.7 {
            1.0
        } else if self.memory_usage < 0.9 {
            2.0
        } else {
            3.0
        };
        self.view.view(ids!(memory_section.memory_dot)).apply_over(
            cx,
            live! {
                draw_bg: { status: (status) }
            },
        );

        self.view.redraw(cx);
    }

    /// Set memory usage (0.0 - 1.0)
    pub fn set_memory_usage(&mut self, cx: &mut Cx, usage: f64) {
        self.set_memory_usage_internal(cx, usage);
    }

    /// Set connection status
    pub fn set_connection_status(&mut self, cx: &mut Cx, status: ConnectionStatus) {
        self.connection_status = status.clone();

        let (status_val, text, dot_color) = match status {
            ConnectionStatus::Ready => (0.0, "Ready", (0.13, 0.77, 0.37)), // Green
            ConnectionStatus::Connecting => (0.5, "Connecting", (0.8, 0.8, 0.0)), // Yellow
            ConnectionStatus::Connected => (1.0, "Connected", (0.0, 1.0, 0.5)), // Neon green
            ConnectionStatus::Stopping => (0.5, "Stopping", (0.8, 0.6, 0.0)), // Orange
            ConnectionStatus::Stopped => (0.0, "Stopped", (0.5, 0.5, 0.5)), // Gray
            ConnectionStatus::Failed => (2.0, "Failed", (0.95, 0.25, 0.25)), // Red
        };

        self.view
            .button(ids!(connection_section.dataflow_btn))
            .set_text(cx, text);
        self.view
            .button(ids!(connection_section.dataflow_btn))
            .apply_over(
                cx,
                live! {
                    draw_bg: { status: (status_val) }
                },
            );
        self.view
            .view(ids!(connection_section.connection_dot))
            .apply_over(
                cx,
                live! {
                    draw_bg: { color: (vec4(dot_color.0, dot_color.1, dot_color.2, 1.0)) }
                },
            );

        self.view.redraw(cx);
    }

    /// Get the current running state
    pub fn is_running(&self) -> bool {
        self.is_running
    }
}

impl MofaHeroRef {
    pub fn set_running(&self, cx: &mut Cx, running: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_running(cx, running);
        }
    }

    pub fn set_cpu_usage(&self, cx: &mut Cx, usage: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_cpu_usage(cx, usage);
        }
    }

    pub fn set_memory_usage(&self, cx: &mut Cx, usage: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_memory_usage(cx, usage);
        }
    }

    pub fn set_connection_status(&self, cx: &mut Cx, status: ConnectionStatus) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_connection_status(cx, status);
        }
    }

    /// Update dark mode for this widget
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            // Action section
            inner.view.view(ids!(action_section)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );
            inner
                .view
                .label(ids!(action_section.start_view.action_start_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(action_section.stop_view.action_stop_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Connection section
            inner.view.view(ids!(connection_section)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );
            inner
                .view
                .label(ids!(connection_section.dataflow_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // CPU section
            inner.view.view(ids!(cpu_section)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );
            inner.view.label(ids!(cpu_section.cpu_label)).apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
            inner.view.label(ids!(cpu_section.cpu_pct)).apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

            // Memory section
            inner.view.view(ids!(memory_section)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );
            inner
                .view
                .label(ids!(memory_section.memory_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(memory_section.memory_pct))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // GPU section
            inner.view.view(ids!(gpu_section)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );
            inner.view.label(ids!(gpu_section.gpu_label)).apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
            inner.view.label(ids!(gpu_section.gpu_pct)).apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

            // VRAM section
            inner.view.view(ids!(vram_section)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );
            inner.view.label(ids!(vram_section.vram_label)).apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
            inner.view.label(ids!(vram_section.vram_pct)).apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

            inner.view.redraw(cx);
        }
    }
}
