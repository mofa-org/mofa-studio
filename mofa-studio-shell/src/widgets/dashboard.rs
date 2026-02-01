//! Dashboard Widget - Main application layout container
//!
//! The Dashboard provides the base layer for MoFA Studio with:
//! - Header with logo, title, theme toggle, and user profile
//! - Content area with app pages (FM, Settings, etc.)
//! - Tab overlay for modal-like Profile/Settings tabs

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Import fonts and colors from shared theme
    use mofa_widgets::theme::FONT_REGULAR;
    use mofa_widgets::theme::FONT_MEDIUM;
    use mofa_widgets::theme::FONT_SEMIBOLD;
    use mofa_widgets::theme::FONT_BOLD;
    use mofa_widgets::theme::DARK_BG;
    use mofa_widgets::theme::PANEL_BG;
    use mofa_widgets::theme::ACCENT_INDIGO;
    use mofa_widgets::theme::TEXT_PRIMARY;
    use mofa_widgets::theme::TEXT_SECONDARY;
    use mofa_widgets::theme::HOVER_BG;
    use mofa_widgets::theme::TRANSPARENT;
    use mofa_widgets::theme::SLATE_50;
    use mofa_widgets::theme::SLATE_200;
    use mofa_widgets::theme::SLATE_400;
    use mofa_widgets::theme::SLATE_500;
    use mofa_widgets::theme::SLATE_700;
    use mofa_widgets::theme::SLATE_800;
    use mofa_widgets::theme::GRAY_300;
    use mofa_widgets::theme::GRAY_600;
    use mofa_widgets::theme::INDIGO_100;
    use mofa_widgets::theme::DARK_BG_DARK;
    use mofa_widgets::theme::PANEL_BG_DARK;
    use mofa_widgets::theme::TEXT_PRIMARY_DARK;
    use mofa_widgets::theme::TEXT_SECONDARY_DARK;

    use mofa_fm::screen::design::MoFaFMScreen;
    use mofa_settings::screen::SettingsScreen;
    use mofa_debate::screen::design::MoFaDebateScreen;
    use mofa_tts::screen::TTSScreen;
    use crate::widgets::tabs::TabWidget;
    use crate::widgets::tabs::TabBar;

    // Logo image
    MOFA_LOGO = dep("crate://self/resources/mofa-logo.png")

    pub Dashboard = {{Dashboard}} <View> {
        width: Fill, height: Fill
        flow: Overlay
        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn pixel(self) -> vec4 {
                return mix((DARK_BG), (DARK_BG_DARK), self.dark_mode);
            }
        }

        // Base layer - header + content area
        dashboard_base = <View> {
            width: Fill, height: Fill
            flow: Down

            // Header
            header = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 12
                align: {y: 0.5}
                padding: {left: 20, right: 20, top: 15, bottom: 15}
                show_bg: true
                draw_bg: {
                    instance dark_mode: 0.0
                    fn pixel(self) -> vec4 {
                        return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                    }
                }

                hamburger_placeholder = <View> {
                    width: 21, height: 21
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let cy = self.rect_size.y * 0.5;
                            let cx = self.rect_size.x * 0.5;
                            sdf.move_to(cx - 5.0, cy - 4.0);
                            sdf.line_to(cx + 5.0, cy - 4.0);
                            sdf.stroke((SLATE_500), 1.5);
                            sdf.move_to(cx - 5.0, cy);
                            sdf.line_to(cx + 5.0, cy);
                            sdf.stroke((SLATE_500), 1.5);
                            sdf.move_to(cx - 5.0, cy + 4.0);
                            sdf.line_to(cx + 5.0, cy + 4.0);
                            sdf.stroke((SLATE_500), 1.5);
                            return sdf.result;
                        }
                    }
                }

                logo = <Image> {
                    width: 40, height: 40
                    source: (MOFA_LOGO)
                }

                title = <Label> {
                    text: "MoFA Studio"
                    draw_text: {
                        color: (TEXT_PRIMARY)
                        text_style: <FONT_BOLD>{ font_size: 24.0 }
                    }
                }

                <View> { width: Fill, height: 1 }

                // Theme toggle button
                theme_toggle = <View> {
                    width: 36, height: 36
                    align: {x: 0.5, y: 0.5}
                    cursor: Hand
                    show_bg: true
                    draw_bg: {
                        instance hover: 0.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let cx = self.rect_size.x * 0.5;
                            let cy = self.rect_size.y * 0.5;
                            sdf.circle(cx, cy, 16.0);
                            sdf.fill(mix((TRANSPARENT), (HOVER_BG), self.hover));
                            return sdf.result;
                        }
                    }

                    // Sun icon (light mode) - amber color
                    sun_icon = <View> {
                        width: 20, height: 20
                        show_bg: true
                        draw_bg: {
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let c = self.rect_size * 0.5;
                                let amber = vec4(0.961, 0.624, 0.043, 1.0);  // AMBER_500 #f59f0b
                                // Sun circle
                                sdf.circle(c.x, c.y, 4.0);
                                sdf.fill(amber);
                                // Sun rays
                                let ray_len = 2.5;
                                let ray_dist = 6.5;
                                sdf.move_to(c.x, c.y - ray_dist);
                                sdf.line_to(c.x, c.y - ray_dist - ray_len);
                                sdf.stroke(amber, 1.5);
                                sdf.move_to(c.x, c.y + ray_dist);
                                sdf.line_to(c.x, c.y + ray_dist + ray_len);
                                sdf.stroke(amber, 1.5);
                                sdf.move_to(c.x - ray_dist, c.y);
                                sdf.line_to(c.x - ray_dist - ray_len, c.y);
                                sdf.stroke(amber, 1.5);
                                sdf.move_to(c.x + ray_dist, c.y);
                                sdf.line_to(c.x + ray_dist + ray_len, c.y);
                                sdf.stroke(amber, 1.5);
                                return sdf.result;
                            }
                        }
                    }

                    // Moon icon (dark mode - hidden by default) - indigo color
                    moon_icon = <View> {
                        width: 20, height: 20
                        visible: false
                        show_bg: true
                        draw_bg: {
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let c = self.rect_size * 0.5;
                                let indigo = vec4(0.388, 0.400, 0.945, 1.0);  // INDIGO_500 #6366f1
                                let white = vec4(1.0, 1.0, 1.0, 1.0);
                                sdf.circle(c.x, c.y, 6.0);
                                sdf.fill(indigo);
                                sdf.circle(c.x + 3.5, c.y - 2.5, 4.5);
                                sdf.fill(white);
                                return sdf.result;
                            }
                        }
                    }
                }

                user_profile_container = <View> {
                    width: Fit, height: Fill
                    flow: Right
                    align: {x: 0.5, y: 0.5}
                    spacing: 4
                    cursor: Hand

                    user_profile_btn = <View> {
                        width: 32, height: 32
                        padding: {left: 6, top: 8, right: 10, bottom: 8}
                        show_bg: true
                        draw_bg: {
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let cx = self.rect_size.x * 0.5;
                                let cy = self.rect_size.y * 0.5;
                                sdf.circle(cx, cy, 15.0);
                                sdf.fill((HOVER_BG));
                                return sdf.result;
                            }
                        }

                        <Icon> {
                            draw_icon: {
                                svg_file: dep("crate://self/resources/icons/user.svg")
                                fn get_color(self) -> vec4 { return (GRAY_600); }
                            }
                            icon_walk: {width: 16, height: 16}
                        }
                    }

                    dropdown_arrow = <View> {
                        width: 12, height: Fill
                        draw_bg: {
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let cx = self.rect_size.x * 0.5;
                                let cy = self.rect_size.y * 0.5;
                                sdf.move_to(cx - 4.0, cy - 2.0);
                                sdf.line_to(cx, cy + 2.0);
                                sdf.line_to(cx + 4.0, cy - 2.0);
                                sdf.stroke((SLATE_400), 1.5);
                                return sdf.result;
                            }
                        }
                    }
                }
            }

            // Content area
            content_area = <View> {
                width: Fill, height: Fill
                flow: Right
                padding: 20

                main_content = <View> {
                    width: Fill, height: Fill
                    flow: Down

                    // Hero title panel - displays current app name and metadata
                    hero_title_panel = <RoundedView> {
                        width: Fill, height: Fit
                        padding: {left: 16, right: 16, top: 12, bottom: 12}
                        margin: {bottom: 12}
                        show_bg: true
                        draw_bg: {
                            instance dark_mode: 0.0
                            border_radius: 8.0
                            fn get_color(self) -> vec4 {
                                return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                            }
                        }

                        <View> {
                            width: Fill, height: Fit
                            flow: Right
                            align: {y: 0.5}
                            spacing: 12

                            // App icon placeholder
                            app_icon = <View> {
                                width: 40, height: 40
                                show_bg: true
                                draw_bg: {
                                    instance dark_mode: 0.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        let c = self.rect_size * 0.5;
                                        sdf.circle(c.x, c.y, 18.0);
                                        let bg = mix((INDIGO_100), (SLATE_700), self.dark_mode);
                                        sdf.fill(bg);
                                        // Radio wave icon
                                        sdf.circle(c.x, c.y, 4.0);
                                        sdf.fill((ACCENT_INDIGO));
                                        sdf.circle(c.x, c.y, 8.0);
                                        sdf.stroke((ACCENT_INDIGO), 1.5);
                                        sdf.circle(c.x, c.y, 13.0);
                                        sdf.stroke((ACCENT_INDIGO), 1.5);
                                        return sdf.result;
                                    }
                                }
                            }

                            // App title and description
                            title_container = <View> {
                                width: Fill, height: Fit
                                flow: Down
                                spacing: 2

                                app_title = <Label> {
                                    text: "MoFA FM"
                                    draw_text: {
                                        instance dark_mode: 0.0
                                        text_style: <FONT_BOLD>{ font_size: 18.0 }
                                        fn get_color(self) -> vec4 {
                                            return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                        }
                                    }
                                }

                                app_description = <Label> {
                                    text: "AI-powered audio streaming and voice interface"
                                    draw_text: {
                                        instance dark_mode: 0.0
                                        text_style: <FONT_REGULAR>{ font_size: 12.0 }
                                        fn get_color(self) -> vec4 {
                                            return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                        }
                                    }
                                }
                            }

                        }
                    }

                    content = <View> {
                        width: Fill, height: Fill
                        flow: Overlay

                        fm_page = <MoFaFMScreen> {
                            width: Fill, height: Fill
                            visible: true
                        }

                        debate_page = <MoFaDebateScreen> {
                            width: Fill, height: Fill
                            visible: false
                        }

                        tts_page = <TTSScreen> {
                            width: Fill, height: Fill
                            visible: false
                        }

                        app_page = <View> {
                            width: Fill, height: Fill
                            flow: Down
                            spacing: 12
                            visible: false
                            align: {x: 0.5, y: 0.5}
                            show_bg: true
                            draw_bg: { color: (DARK_BG) }

                            <Label> {
                                text: "Demo App"
                                draw_text: {
                                    color: (SLATE_400)
                                    text_style: <FONT_SEMIBOLD>{ font_size: 18.0 }
                                }
                            }
                            <Label> {
                                text: "Select an app from the sidebar"
                                draw_text: {
                                    color: (GRAY_300)
                                    text_style: <FONT_REGULAR>{ font_size: 13.0 }
                                }
                            }
                        }

                        settings_page = <SettingsScreen> {
                            width: Fill, height: Fill
                            visible: false
                        }
                    }
                }
            }
        }

        // Tab overlay - modal layer for Profile/Settings
        tab_overlay = <View> {
            width: Fill, height: Fill
            flow: Down
            visible: false
            margin: {top: 70}
            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    return mix((DARK_BG), (DARK_BG_DARK), self.dark_mode);
                }
            }

            tab_bar = <TabBar> {
                profile_tab = <TabWidget> {
                    visible: false
                    tab_label = { text: "Profile" }
                }
                settings_tab = <TabWidget> {
                    visible: false
                    tab_label = { text: "Settings" }
                }
            }

            tab_content = <View> {
                width: Fill, height: Fill
                flow: Overlay
                padding: 20

                profile_page = <RoundedView> {
                    padding: 20
                    width: Fill, height: Fill
                    visible: false
                    show_bg: true
                    draw_bg: {
                        instance dark_mode: 0.0
                        border_radius: 8.0
                        fn get_color(self) -> vec4 {
                            return mix((PANEL_BG), (PANEL_BG_DARK), self.dark_mode);
                        }
                    }
                    padding: 24
                    flow: Down
                    spacing: 16

                    profile_title = <Label> {
                        text: "User Profile"
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: <FONT_BOLD>{ font_size: 20.0 }
                            fn get_color(self) -> vec4 {
                                return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                            }
                        }
                    }

                    profile_divider = <View> {
                        width: Fill, height: 1
                        show_bg: true
                        draw_bg: {
                            instance dark_mode: 0.0
                            fn pixel(self) -> vec4 {
                                return mix((SLATE_200), (SLATE_700), self.dark_mode);
                            }
                        }
                    }

                    profile_row = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 16
                        align: {y: 0.5}

                        profile_avatar = <View> {
                            width: 64, height: 64
                            show_bg: true
                            draw_bg: {
                                instance dark_mode: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let c = self.rect_size * 0.5;
                                    sdf.circle(c.x, c.y, 30.0);
                                    let bg = mix((INDIGO_100), (SLATE_700), self.dark_mode);
                                    sdf.fill(bg);
                                    return sdf.result;
                                }
                            }
                            align: {x: 0.5, y: 0.5}
                            <Icon> {
                                draw_icon: {
                                    svg_file: dep("crate://self/resources/icons/user.svg")
                                    fn get_color(self) -> vec4 { return (ACCENT_INDIGO); }
                                }
                                icon_walk: {width: 32, height: 32}
                            }
                        }

                        profile_info = <View> {
                            width: Fill, height: Fit
                            flow: Down
                            spacing: 4

                            profile_name = <Label> {
                                text: "Demo User"
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_SEMIBOLD>{ font_size: 16.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_PRIMARY), (TEXT_PRIMARY_DARK), self.dark_mode);
                                    }
                                }
                            }
                            profile_email = <Label> {
                                text: "demo@mofa.studio"
                                draw_text: {
                                    instance dark_mode: 0.0
                                    text_style: <FONT_REGULAR>{ font_size: 13.0 }
                                    fn get_color(self) -> vec4 {
                                        return mix((TEXT_SECONDARY), (TEXT_SECONDARY_DARK), self.dark_mode);
                                    }
                                }
                            }
                        }
                    }

                    profile_coming_soon = <Label> {
                        text: "Profile settings coming soon..."
                        margin: {top: 20}
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: <FONT_REGULAR>{ font_size: 13.0 }
                            fn get_color(self) -> vec4 {
                                return mix((SLATE_400), (SLATE_500), self.dark_mode);
                            }
                        }
                    }
                }

                settings_tab_page = <SettingsScreen> {
                    width: Fill, height: Fill
                    visible: false
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct Dashboard {
    #[deref]
    view: View,
}

impl Widget for Dashboard {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}
