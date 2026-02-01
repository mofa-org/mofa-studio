use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Import fonts and colors from shared theme (single source of truth)
    use mofa_widgets::theme::FONT_REGULAR;
    use mofa_widgets::theme::FONT_BOLD;
    use mofa_widgets::theme::SLATE_50;
    use mofa_widgets::theme::SLATE_200;
    use mofa_widgets::theme::SLATE_400;
    use mofa_widgets::theme::SLATE_500;
    use mofa_widgets::theme::SLATE_600;
    use mofa_widgets::theme::SLATE_700;
    use mofa_widgets::theme::SLATE_800;
    use mofa_widgets::theme::SLATE_900;
    use mofa_widgets::theme::BLUE_100;
    use mofa_widgets::theme::BLUE_900;
    use mofa_widgets::theme::DIVIDER;
    use mofa_widgets::theme::DIVIDER_DARK;
    use mofa_widgets::theme::AMBER_500;
    use mofa_widgets::theme::INDIGO_500;
    use mofa_widgets::theme::TEXT_PRIMARY_DARK;
    use mofa_widgets::theme::TEXT_SECONDARY_DARK;

    // Chevron icon for expand/collapse
    ChevronRight = <Icon> {
        draw_icon: {
            svg_file: dep("crate://makepad-widgets/resources/icons/arrow.svg")
            color: (SLATE_400)
        }
        icon_walk: {width: 10, height: 10}
    }

    // Chevron pointing down (rotated)
    ChevronDown = <Icon> {
        draw_icon: {
            svg_file: dep("crate://makepad-widgets/resources/icons/arrow.svg")
            color: (SLATE_400)
            fn get_rotation_z(self) -> f64 {
                return 90.0;
            }
        }
        icon_walk: {width: 10, height: 10}
    }

    // Custom sidebar button using Button instead of RadioButton - with dark mode and hover animation
    pub SidebarMenuButton = <Button> {
        width: Fill, height: Fit
        padding: {top: 12, bottom: 12, left: 12, right: 12}
        margin: 0
        align: {x: 0.0, y: 0.5}
        icon_walk: {width: 20, height: 20, margin: {right: 12}}

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

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance selected: 0.0
            instance dark_mode: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                // Light mode: SLATE_50 -> SLATE_200 (hover) -> BLUE_100 (selected)
                // Dark mode: SLATE_800 -> SLATE_700 (hover) -> BLUE_900 (selected)
                let light_normal = (SLATE_50);
                let light_hover = (SLATE_200);
                let light_selected = (BLUE_100);
                let dark_normal = (SLATE_800);
                let dark_hover = (SLATE_700);
                let dark_selected = (BLUE_900);
                let normal = mix(light_normal, dark_normal, self.dark_mode);
                let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                let selected_color = mix(light_selected, dark_selected, self.dark_mode);
                let color = mix(
                    mix(normal, hover_color, self.hover),
                    selected_color,
                    self.selected
                );
                sdf.box(2.0, 2.0, self.rect_size.x - 4.0, self.rect_size.y - 4.0, 6.0);
                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            instance dark_mode: 0.0
            text_style: <FONT_REGULAR>{ font_size: 12.0 }

            fn get_color(self) -> vec4 {
                return mix((SLATE_500), (SLATE_400), self.dark_mode);
            }
        }

        draw_icon: {
            fn get_color(self) -> vec4 {
                return (SLATE_500);
            }
        }
    }

    // Show More/Less button container with arrow on right
    ShowMoreContainer = <View> {
        width: Fill, height: Fit
        flow: Right
        align: {x: 0.0, y: 0.5}
        spacing: 4

        show_more_bg = <View> {
            width: Fill, height: Fit
            cursor: Hand
            show_bg: true
            draw_bg: {
                instance hover: 0.0
                instance dark_mode: 0.0

                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let light_normal = (SLATE_50);
                    let light_hover = (SLATE_200);
                    let dark_normal = (SLATE_800);
                    let dark_hover = (SLATE_700);
                    let normal = mix(light_normal, dark_normal, self.dark_mode);
                    let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                    let color = mix(normal, hover_color, self.hover);
                    sdf.box(2.0, 2.0, self.rect_size.x - 4.0, self.rect_size.y - 4.0, 6.0);
                    sdf.fill(color);
                    return sdf.result;
                }
            }

            show_more_text = <Label> {
                padding: {top: 12, bottom: 12, left: 10}
                text: "Show More"
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: <FONT_REGULAR>{ font_size: 12.0 }

                    fn get_color(self) -> vec4 {
                        return mix((SLATE_800), (SLATE_200), self.dark_mode);
                    }
                }
            }
        }

        arrow_label = <Label> {
            padding: {top: 12, bottom: 12, right: 10}
            text: ">"
            draw_text: {
                instance dark_mode: 0.0
                text_style: <FONT_REGULAR>{ font_size: 15.0 }

                fn get_color(self) -> vec4 {
                    return mix((SLATE_800), (SLATE_200), self.dark_mode);
                }
            }
        }
    }

    // Main sidebar container - with dark mode support
    // Height is Fit so sidebar adapts to content (compact when collapsed)
    pub Sidebar = {{Sidebar}} {
        width: Fill, height: Fill
        flow: Down
        spacing: 4.0
        padding: {top: 15, bottom: 15, left: 10, right: 10}
        margin: 0

        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                // Main rectangle with subtle rounded corners
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 4.0);
                let bg = mix((SLATE_50), (SLATE_800), self.dark_mode);
                sdf.fill(bg);

                return sdf.result;
            }
        }

        // Main content wrapper - expands to fill, leaving room for settings at bottom
        main_content = <View> {
            width: Fill, height: Fit
            flow: Down
            spacing: 4.0

            // Logo area (empty spacer)
            logo_area = <View> {
                width: Fill, height: 5
            }

            // Navigation buttons
            mofa_fm_tab = <SidebarMenuButton> {
                text: "MoFA FM"
                draw_icon: {
                    svg_file: dep("crate://self/resources/icons/fm.svg")
                }
            }

            debate_tab = <SidebarMenuButton> {
                text: "Debate"
                draw_icon: {
                    svg_file: dep("crate://self/resources/icons/app.svg")
                }
            }

            tts_tab = <SidebarMenuButton> {
                text: "MoFA TTS"
                draw_icon: {
                    svg_file: dep("crate://self/resources/icons/app.svg")
                }
            }

            // Apps container - height Fit so it adapts to content
            apps_wrapper = <View> {
                width: Fill, height: Fit
                flow: Down

                // ScrollYView - height Fit when collapsed (no scroll needed)
                // When expanded, height is set dynamically to enable scrolling
                apps_scroll = <ScrollYView> {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 4
                    scroll_bars: <ScrollBars> {
                        show_scroll_x: false
                        show_scroll_y: true
                    }

                    // First 4 apps - always visible
                    app1_btn = <SidebarMenuButton> { text: "App 1", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                    app2_btn = <SidebarMenuButton> { text: "App 2", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                    app3_btn = <SidebarMenuButton> { text: "App 3", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                    app4_btn = <SidebarMenuButton> { text: "App 4", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }

                    // Pinned app from "Show More" section - appears when an app from expanded section is selected
                    pinned_app_btn = <SidebarMenuButton> {
                        visible: false
                        text: ""
                        draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") }
                    }

                    // Show More button
                    show_more_btn = <ShowMoreContainer> {}

                    // Collapsible section for apps 5-20 (hidden by default)
                    more_apps_section = <View> {
                        width: Fill, height: Fit
                        flow: Down
                        spacing: 4
                        visible: false

                        app5_btn = <SidebarMenuButton> { text: "App 5", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app6_btn = <SidebarMenuButton> { text: "App 6", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app7_btn = <SidebarMenuButton> { text: "App 7", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app8_btn = <SidebarMenuButton> { text: "App 8", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app9_btn = <SidebarMenuButton> { text: "App 9", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app10_btn = <SidebarMenuButton> { text: "App 10", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app11_btn = <SidebarMenuButton> { text: "App 11", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app12_btn = <SidebarMenuButton> { text: "App 12", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app13_btn = <SidebarMenuButton> { text: "App 13", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app14_btn = <SidebarMenuButton> { text: "App 14", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app15_btn = <SidebarMenuButton> { text: "App 15", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app16_btn = <SidebarMenuButton> { text: "App 16", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app17_btn = <SidebarMenuButton> { text: "App 17", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app18_btn = <SidebarMenuButton> { text: "App 18", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app19_btn = <SidebarMenuButton> { text: "App 19", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                        app20_btn = <SidebarMenuButton> { text: "App 20", draw_icon: { svg_file: dep("crate://self/resources/icons/app.svg") } }
                    }
                }
            }
        }

        // Divider before settings
        settings_divider = <View> {
            width: Fill, height: 1
            margin: {top: 8, bottom: 8}
            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    return mix((DIVIDER), (DIVIDER_DARK), self.dark_mode);
                }
            }
        }

        settings_tab = <SidebarMenuButton> {
            text: "Settings"
            draw_icon: {
                svg_file: dep("crate://self/resources/icons/settings.svg")
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum SidebarSelection {
    MofaFM,
    Debate,
    TTS,
    App(usize), // 1-20
    Settings,
}

#[derive(Clone, Debug, DefaultNone)]
pub enum SidebarAction {
    None,
    ToggleTheme,
}

#[derive(Live, LiveHook, Widget)]
pub struct Sidebar {
    #[deref]
    view: View,

    #[live]
    expand_to_fill: bool, // When true, apps_scroll height is calculated dynamically

    #[rust]
    more_apps_visible: bool,

    #[rust]
    selection: Option<SidebarSelection>, // Track current selection

    #[rust]
    pinned_app_name: Option<String>, // Name of the pinned app from "Show More" section

    #[rust]
    max_scroll_height: f64, // Max height for apps_scroll when expanded (set by app.rs)

    #[rust]
    cached_sidebar_height: f64, // Cached sidebar height from last draw
}

impl Widget for Sidebar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Handle show more/less click
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => return,
        };

        // Check if show_more_bg view was clicked
        if self
            .view
            .view(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .show_more_btn
                    .show_more_bg
            ))
            .finger_up(&actions)
            .is_some()
        {
            self.more_apps_visible = !self.more_apps_visible;

            // Toggle visibility of more apps section
            self.view
                .view(ids!(
                    main_content.apps_wrapper.apps_scroll.more_apps_section
                ))
                .set_visible(cx, self.more_apps_visible);

            // Update text and arrow labels
            if self.more_apps_visible {
                self.view
                    .label(ids!(
                        main_content
                            .apps_wrapper
                            .apps_scroll
                            .show_more_btn
                            .show_more_text
                    ))
                    .set_text(cx, "Show Less");
                self.view
                    .label(ids!(
                        main_content
                            .apps_wrapper
                            .apps_scroll
                            .show_more_btn
                            .arrow_label
                    ))
                    .set_text(cx, "^");
                // When expanded, set scroll height
                // Use max_scroll_height if set, otherwise calculate based on available space
                let scroll_height = if self.max_scroll_height > 0.0 {
                    self.max_scroll_height
                } else if self.expand_to_fill && self.cached_sidebar_height > 0.0 {
                    // Pinned sidebar: calculate available height dynamically
                    // Reserved space: logo_area (5) + mofa_fm_tab (~48) + settings_divider (1+16 margin)
                    //                 + settings_tab (~48) + padding (top:15 + bottom:15) + spacing
                    let reserved_height = 5.0 + 48.0 + 17.0 + 48.0 + 30.0 + 20.0; // ~168px total
                    (self.cached_sidebar_height - reserved_height).max(200.0)
                } else if self.expand_to_fill {
                    // Fallback for pinned sidebar if no cached height yet
                    500.0
                } else {
                    // Overlay sidebar: use smaller fixed height
                    400.0
                };
                self.view
                    .view(ids!(main_content.apps_wrapper.apps_scroll))
                    .apply_over(
                        cx,
                        live! {
                            height: (scroll_height)
                        },
                    );
            } else {
                self.view
                    .label(ids!(
                        main_content
                            .apps_wrapper
                            .apps_scroll
                            .show_more_btn
                            .show_more_text
                    ))
                    .set_text(cx, "Show More");
                self.view
                    .label(ids!(
                        main_content
                            .apps_wrapper
                            .apps_scroll
                            .show_more_btn
                            .arrow_label
                    ))
                    .set_text(cx, ">");
                // When collapsed, reset scroll area to Fit (no scrolling needed)
                self.view
                    .view(ids!(main_content.apps_wrapper.apps_scroll))
                    .apply_over(
                        cx,
                        live! {
                            height: Fit
                        },
                    );
            }

            self.view.redraw(cx);
        }

        // Handle MoFA FM tab click
        if self
            .view
            .button(ids!(main_content.mofa_fm_tab))
            .clicked(actions)
        {
            self.handle_selection(cx, SidebarSelection::MofaFM);
        }

        // Handle Debate tab click
        if self
            .view
            .button(ids!(main_content.debate_tab))
            .clicked(actions)
        {
            self.handle_selection(cx, SidebarSelection::Debate);
        }

        // Handle TTS tab click
        if self
            .view
            .button(ids!(main_content.tts_tab))
            .clicked(actions)
        {
            self.handle_selection(cx, SidebarSelection::TTS);
        }

        // Handle Settings tab click
        if self.view.button(ids!(settings_tab)).clicked(actions) {
            self.handle_selection(cx, SidebarSelection::Settings);
        }

        // Handle pinned app button click (acts same as the original app)
        if self
            .view
            .button(ids!(main_content.apps_wrapper.apps_scroll.pinned_app_btn))
            .clicked(actions)
        {
            if let Some(SidebarSelection::App(app_idx)) = &self.selection {
                if *app_idx >= 5 {
                    // Re-select the same app (refresh selection state)
                    self.handle_selection(cx, SidebarSelection::App(*app_idx));
                }
            }
        }

        // Handle app button clicks using macro to reduce repetition
        macro_rules! handle_app_click {
            ($self:expr, $cx:expr, $actions:expr, $($idx:expr => $path:expr),+ $(,)?) => {
                $(
                    if $self.view.button($path).clicked($actions) {
                        $self.handle_selection($cx, SidebarSelection::App($idx));
                    }
                )+
            };
        }

        handle_app_click!(self, cx, actions,
            // Apps 1-4 are directly in apps_scroll
            1 => ids!(apps_scroll.app1_btn),
            2 => ids!(apps_scroll.app2_btn),
            3 => ids!(apps_scroll.app3_btn),
            4 => ids!(apps_scroll.app4_btn),
            // Apps 5-20 are in more_apps_section
            5 => ids!(apps_scroll.more_apps_section.app5_btn),
            6 => ids!(apps_scroll.more_apps_section.app6_btn),
            7 => ids!(apps_scroll.more_apps_section.app7_btn),
            8 => ids!(apps_scroll.more_apps_section.app8_btn),
            9 => ids!(apps_scroll.more_apps_section.app9_btn),
            10 => ids!(apps_scroll.more_apps_section.app10_btn),
            11 => ids!(apps_scroll.more_apps_section.app11_btn),
            12 => ids!(apps_scroll.more_apps_section.app12_btn),
            13 => ids!(apps_scroll.more_apps_section.app13_btn),
            14 => ids!(apps_scroll.more_apps_section.app14_btn),
            15 => ids!(apps_scroll.more_apps_section.app15_btn),
            16 => ids!(apps_scroll.more_apps_section.app16_btn),
            17 => ids!(apps_scroll.more_apps_section.app17_btn),
            18 => ids!(apps_scroll.more_apps_section.app18_btn),
            19 => ids!(apps_scroll.more_apps_section.app19_btn),
            20 => ids!(apps_scroll.more_apps_section.app20_btn),
        );
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let result = self.view.draw_walk(cx, scope, walk);

        // Cache the sidebar height for dynamic scroll calculation
        let rect = self.view.area().rect(cx);
        if rect.size.y > 0.0 {
            self.cached_sidebar_height = rect.size.y;
        }

        result
    }
}

impl Sidebar {
    fn handle_selection(&mut self, cx: &mut Cx, selection: SidebarSelection) {
        self.selection = Some(selection.clone());

        // Clear all selections first
        self.clear_all_selections(cx);

        // Apply selected state based on what was clicked
        match &selection {
            SidebarSelection::MofaFM => {
                self.view
                    .button(ids!(main_content.mofa_fm_tab))
                    .apply_over(cx, live! { draw_bg: { selected: 1.0 } });
                // Hide pinned app when MoFA FM is selected
                self.pinned_app_name = None;
                self.view
                    .button(ids!(main_content.apps_wrapper.apps_scroll.pinned_app_btn))
                    .set_visible(cx, false);
            }
            SidebarSelection::Debate => {
                self.view
                    .button(ids!(main_content.debate_tab))
                    .apply_over(cx, live! { draw_bg: { selected: 1.0 } });
                // Hide pinned app when Debate is selected
                self.pinned_app_name = None;
                self.view
                    .button(ids!(main_content.apps_wrapper.apps_scroll.pinned_app_btn))
                    .set_visible(cx, false);
            }
            SidebarSelection::TTS => {
                self.view
                    .button(ids!(main_content.tts_tab))
                    .apply_over(cx, live! { draw_bg: { selected: 1.0 } });
                // Hide pinned app when TTS is selected
                self.pinned_app_name = None;
                self.view
                    .button(ids!(main_content.apps_wrapper.apps_scroll.pinned_app_btn))
                    .set_visible(cx, false);
            }

            SidebarSelection::App(app_idx) => {
                self.set_app_button_selected(cx, *app_idx, true);

                // Handle pinned app display for "Show More" section apps (5-20)
                if *app_idx >= 5 {
                    let app_name = format!("App {}", app_idx);
                    self.pinned_app_name = Some(app_name.clone());

                    self.view
                        .button(ids!(main_content.apps_wrapper.apps_scroll.pinned_app_btn))
                        .set_text(cx, &app_name);
                    self.view
                        .button(ids!(main_content.apps_wrapper.apps_scroll.pinned_app_btn))
                        .set_visible(cx, true);
                    self.view
                        .button(ids!(main_content.apps_wrapper.apps_scroll.pinned_app_btn))
                        .apply_over(
                            cx,
                            live! {
                                draw_bg: { selected: 1.0 }
                            },
                        );
                } else {
                    self.pinned_app_name = None;
                    self.view
                        .button(ids!(main_content.apps_wrapper.apps_scroll.pinned_app_btn))
                        .set_visible(cx, false);
                }
            }
            SidebarSelection::Settings => {
                self.view
                    .button(ids!(settings_tab))
                    .apply_over(cx, live! { draw_bg: { selected: 1.0 } });
                // Hide pinned app when Settings is selected
                self.pinned_app_name = None;
                self.view
                    .button(ids!(main_content.apps_wrapper.apps_scroll.pinned_app_btn))
                    .set_visible(cx, false);
            }
        }

        self.view.redraw(cx);
    }

    fn clear_all_selections(&mut self, cx: &mut Cx) {
        // Macro to clear selection on multiple buttons
        macro_rules! clear_selection {
            ($self:expr, $cx:expr, $($path:expr),+ $(,)?) => {
                $( $self.view.button($path).apply_over($cx, live!{ draw_bg: { selected: 0.0 } }); )+
            };
        }

        // Clear MoFA FM, Debate, TTS, Settings, and pinned app
        clear_selection!(
            self,
            cx,
            ids!(main_content.mofa_fm_tab),
            ids!(main_content.debate_tab),
            ids!(main_content.tts_tab),
            ids!(settings_tab),
            ids!(main_content.apps_wrapper.apps_scroll.pinned_app_btn)
        );

        // Clear apps 1-4
        clear_selection!(
            self,
            cx,
            ids!(main_content.apps_wrapper.apps_scroll.app1_btn),
            ids!(main_content.apps_wrapper.apps_scroll.app2_btn),
            ids!(main_content.apps_wrapper.apps_scroll.app3_btn),
            ids!(main_content.apps_wrapper.apps_scroll.app4_btn)
        );

        // Clear apps 5-20
        clear_selection!(
            self,
            cx,
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app5_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app6_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app7_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app8_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app9_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app10_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app11_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app12_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app13_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app14_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app15_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app16_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app17_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app18_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app19_btn
            ),
            ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app20_btn
            )
        );
    }

    /// Get the button path for an app index (used by set_app_button_selected)
    fn get_app_button(&mut self, app_idx: usize) -> ButtonRef {
        match app_idx {
            1 => self
                .view
                .button(ids!(main_content.apps_wrapper.apps_scroll.app1_btn)),
            2 => self
                .view
                .button(ids!(main_content.apps_wrapper.apps_scroll.app2_btn)),
            3 => self
                .view
                .button(ids!(main_content.apps_wrapper.apps_scroll.app3_btn)),
            4 => self
                .view
                .button(ids!(main_content.apps_wrapper.apps_scroll.app4_btn)),
            5 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app5_btn
            )),
            6 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app6_btn
            )),
            7 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app7_btn
            )),
            8 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app8_btn
            )),
            9 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app9_btn
            )),
            10 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app10_btn
            )),
            11 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app11_btn
            )),
            12 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app12_btn
            )),
            13 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app13_btn
            )),
            14 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app14_btn
            )),
            15 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app15_btn
            )),
            16 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app16_btn
            )),
            17 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app17_btn
            )),
            18 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app18_btn
            )),
            19 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app19_btn
            )),
            20 => self.view.button(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .more_apps_section
                    .app20_btn
            )),
            _ => self
                .view
                .button(ids!(main_content.apps_wrapper.apps_scroll.app1_btn)), // fallback
        }
    }

    fn set_app_button_selected(&mut self, cx: &mut Cx, app_idx: usize, selected: bool) {
        let selected_val = if selected { 1.0 } else { 0.0 };
        self.get_app_button(app_idx)
            .apply_over(cx, live! { draw_bg: { selected: (selected_val) } });
    }
}

impl SidebarRef {
    /// Set the maximum scroll height for the apps list when expanded
    /// This should be called by app.rs when window size changes
    pub fn set_max_scroll_height(&self, max_height: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.max_scroll_height = max_height;
        }
    }

    /// Collapse the "Show More" section when sidebar is hidden
    pub fn collapse_show_more(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            if inner.more_apps_visible {
                inner.more_apps_visible = false;

                // Hide the more apps section
                inner
                    .view
                    .view(ids!(
                        main_content.apps_wrapper.apps_scroll.more_apps_section
                    ))
                    .set_visible(cx, false);

                // Update text and arrow labels
                inner
                    .view
                    .label(ids!(
                        main_content
                            .apps_wrapper
                            .apps_scroll
                            .show_more_btn
                            .show_more_text
                    ))
                    .set_text(cx, "Show More");
                inner
                    .view
                    .label(ids!(
                        main_content
                            .apps_wrapper
                            .apps_scroll
                            .show_more_btn
                            .arrow_label
                    ))
                    .set_text(cx, ">");

                // Reset scroll height to Fit
                inner
                    .view
                    .view(ids!(main_content.apps_wrapper.apps_scroll))
                    .apply_over(
                        cx,
                        live! {
                            height: Fit
                        },
                    );

                inner.view.redraw(cx);
            }
        }
    }

    /// Restore the selection visual state (call when sidebar becomes visible)
    pub fn restore_selection_state(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            // First clear all selections
            inner.clear_all_selections(cx);

            // Then restore based on current selection
            if let Some(selection) = inner.selection.clone() {
                match selection {
                    SidebarSelection::MofaFM => {
                        inner
                            .view
                            .button(ids!(main_content.mofa_fm_tab))
                            .apply_over(cx, live! { draw_bg: { selected: 1.0 } });
                    }
                    SidebarSelection::Debate => {
                        inner
                            .view
                            .button(ids!(main_content.debate_tab))
                            .apply_over(cx, live! { draw_bg: { selected: 1.0 } });
                    }
                    SidebarSelection::TTS => {
                        inner
                            .view
                            .button(ids!(main_content.tts_tab))
                            .apply_over(cx, live! { draw_bg: { selected: 1.0 } });
                    }

                    SidebarSelection::App(app_idx) => {
                        inner.set_app_button_selected(cx, app_idx, true);

                        // Restore pinned app for Show More apps (5-20)
                        if app_idx >= 5 {
                            let app_name = format!("App {}", app_idx);
                            inner
                                .view
                                .button(ids!(apps_scroll.pinned_app_btn))
                                .set_text(cx, &app_name);
                            inner
                                .view
                                .button(ids!(apps_scroll.pinned_app_btn))
                                .set_visible(cx, true);
                            inner
                                .view
                                .button(ids!(apps_scroll.pinned_app_btn))
                                .apply_over(
                                    cx,
                                    live! {
                                        draw_bg: { selected: 1.0 }
                                    },
                                );
                        }
                    }
                    SidebarSelection::Settings => {
                        inner
                            .view
                            .button(ids!(settings_tab))
                            .apply_over(cx, live! { draw_bg: { selected: 1.0 } });
                    }
                }
            }
            inner.view.redraw(cx);
        }
    }

    /// Update dark mode for this widget
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            // Sidebar background
            inner.view.apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );

            // MoFA FM tab
            inner
                .view
                .button(ids!(main_content.mofa_fm_tab))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Debate tab
            inner.view.button(ids!(main_content.debate_tab)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

            // TTS tab
            inner.view.button(ids!(main_content.tts_tab)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

            // Settings divider
            inner.view.view(ids!(settings_divider)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );

            // Settings tab
            inner.view.button(ids!(settings_tab)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                    draw_text: { dark_mode: (dark_mode) }
                },
            );

            // App buttons (1-4) in apps_wrapper.apps_scroll
            inner
                .view
                .button(ids!(main_content.apps_wrapper.apps_scroll.app1_btn))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(main_content.apps_wrapper.apps_scroll.app2_btn))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(main_content.apps_wrapper.apps_scroll.app3_btn))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(main_content.apps_wrapper.apps_scroll.app4_btn))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(main_content.apps_wrapper.apps_scroll.pinned_app_btn))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Show more button components - use show_more_bg as the parent for accessing nested widgets
            let show_more_bg = inner.view.view(ids!(
                main_content
                    .apps_wrapper
                    .apps_scroll
                    .show_more_btn
                    .show_more_bg
            ));
            show_more_bg.apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );
            show_more_bg.label(ids!(show_more_text)).apply_over(
                cx,
                live! {
                    draw_text: { dark_mode: (dark_mode) }
                },
            );
            inner
                .view
                .label(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .show_more_btn
                        .arrow_label
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // App buttons (5-20) in more_apps_section - always update so they're correct when expanded
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app5_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app6_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app7_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app8_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app9_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app10_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app11_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app12_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app13_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app14_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app15_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app16_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app17_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app18_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app19_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .button(ids!(
                    main_content
                        .apps_wrapper
                        .apps_scroll
                        .more_apps_section
                        .app20_btn
                ))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            inner.view.redraw(cx);
        }
    }
}

// Navigation uses button clicks, handled in app.rs
