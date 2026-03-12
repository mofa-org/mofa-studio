//! Add Provider Modal - Dialog for adding custom providers

use crate::data::{Provider, ProviderConnectionStatus, ProviderId, ProviderType};
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::*;

    // Modal text input with proper light/dark mode styling
    ModalTextInput = <TextInput> {
        width: Fill, height: 44
        padding: {left: 12, right: 12, top: 10, bottom: 10}

        draw_bg: {
            instance dark_mode: 0.0
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, self.radius);
                // Light mode: light gray bg, Dark mode: dark bg
                let light_bg = vec4(0.945, 0.961, 0.976, 1.0);  // SLATE_100
                let dark_bg = vec4(0.20, 0.22, 0.25, 1.0);       // Dark gray
                sdf.fill(mix(light_bg, dark_bg, self.dark_mode));
                // Border
                let light_border = vec4(0.82, 0.84, 0.86, 1.0);  // GRAY_300
                let dark_border = vec4(0.35, 0.38, 0.42, 1.0);   // Dark border
                sdf.stroke(mix(light_border, dark_border, self.dark_mode), 1.0);
                return sdf.result;
            }
        }

        draw_text: {
            instance dark_mode: 0.0
            text_style: <FONT_REGULAR>{ font_size: 13.0 }

            fn get_color(self) -> vec4 {
                // Light mode: dark text, Dark mode: light text
                let light_text = vec4(0.1, 0.1, 0.12, 1.0);      // Near black
                let dark_text = vec4(0.92, 0.93, 0.95, 1.0);     // Near white
                return mix(light_text, dark_text, self.dark_mode);
            }
        }

        draw_selection: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 1.0);
                // Visible blue selection highlight
                sdf.fill(vec4(0.26, 0.52, 0.96, 0.4));  // Blue with 40% opacity
                return sdf.result;
            }
        }
    }

    // Modal button - primary with hover, pressed, and save animations
    ModalPrimaryButton = <Button> {
        width: Fit, height: 40
        padding: {left: 20, right: 20, top: 10, bottom: 10}

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
            // Save success animation - pulse green then back to blue
            save = {
                default: off,
                off = {
                    from: {all: Forward {duration: 0.3}}
                    apply: { draw_bg: {save_progress: 0.0} }
                }
                on = {
                    from: {all: Forward {duration: 0.2}}
                    apply: { draw_bg: {save_progress: 1.0} }
                }
            }
        }

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance save_progress: 0.0
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);

                // Base blue color with hover/pressed states
                let base_color = mix(
                    mix((ACCENT_BLUE), (BLUE_600), self.hover),
                    (BLUE_700),
                    self.pressed
                );

                // Success green color for save animation
                let success_color = vec4(0.133, 0.773, 0.369, 1.0);  // #22C55E - Green 500

                // Mix between base and success based on save progress
                let color = mix(base_color, success_color, self.save_progress);

                // Add subtle scale effect during save
                let scale = 1.0 + self.save_progress * 0.02;
                let offset = (1.0 - scale) * 0.5;
                let w = self.rect_size.x * scale;
                let h = self.rect_size.y * scale;

                sdf.box(
                    offset * self.rect_size.x + 1.0,
                    offset * self.rect_size.y + 1.0,
                    w - 2.0,
                    h - 2.0,
                    self.radius
                );
                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
            color: (WHITE)

            fn get_color(self) -> vec4 {
                return (WHITE);
            }
        }

        text: "Add Provider"
    }

    // Modal button - secondary (cancel) with hover animation
    ModalSecondaryButton = <Button> {
        width: Fit, height: 40
        padding: {left: 20, right: 20, top: 10, bottom: 10}

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
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let color = mix(
                    mix((WHITE), (GRAY_50), self.hover),
                    (GRAY_100),
                    self.pressed
                );
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, self.radius);
                sdf.fill(color);
                sdf.stroke((GRAY_300), 1.0);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
            color: (GRAY_700)

            fn get_color(self) -> vec4 {
                return (GRAY_700);
            }
        }

        text: "Cancel"
    }

    // Add provider modal dialog
    pub AddProviderModal = {{AddProviderModal}} {
        width: Fill, height: Fill
        flow: Overlay
        visible: false

        // Overlay background
        overlay = <View> {
            width: Fill, height: Fill
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    return vec4(0.0, 0.0, 0.0, 0.5);
                }
            }
        }

        // Center the dialog
        dialog_container = <View> {
            width: Fill, height: Fill
            align: {x: 0.5, y: 0.5}

            // Modal dialog with dark mode support
            dialog = <View> {
                width: 480, height: Fit
                padding: 24
                flow: Down
                spacing: 20

                show_bg: true
                draw_bg: {
                    instance radius: 12.0
                    instance dark_mode: 0.0

                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.radius);
                        // Light: white, Dark: slate-800
                        let light_bg = (WHITE);
                        let dark_bg = vec4(0.122, 0.161, 0.231, 1.0);  // SLATE_800
                        sdf.fill(mix(light_bg, dark_bg, self.dark_mode));
                        return sdf.result;
                    }
                }

                // Header
                header = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    align: {y: 0.5}

                    header_label = <Label> {
                        text: "Add Custom Provider"
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: <FONT_BOLD>{ font_size: 16.0 }
                            fn get_color(self) -> vec4 {
                                let light = vec4(0.129, 0.145, 0.192, 1.0);  // SLATE_800
                                let dark = vec4(0.945, 0.961, 0.976, 1.0);   // SLATE_100
                                return mix(light, dark, self.dark_mode);
                            }
                        }
                    }
                }

                // Name field
                name_section = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 6

                    name_label = <Label> {
                        text: "Provider Name"
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                            fn get_color(self) -> vec4 {
                                let light = vec4(0.216, 0.255, 0.318, 1.0);  // GRAY_700
                                let dark = vec4(0.631, 0.663, 0.710, 1.0);   // GRAY_400
                                return mix(light, dark, self.dark_mode);
                            }
                        }
                    }

                    name_input = <ModalTextInput> {
                        empty_text: "My Custom Provider"
                    }
                }

                // API Host field
                host_section = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 6

                    host_label = <Label> {
                        text: "API Host"
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                            fn get_color(self) -> vec4 {
                                let light = vec4(0.216, 0.255, 0.318, 1.0);  // GRAY_700
                                let dark = vec4(0.631, 0.663, 0.710, 1.0);   // GRAY_400
                                return mix(light, dark, self.dark_mode);
                            }
                        }
                    }

                    host_input = <ModalTextInput> {
                        empty_text: "https://api.example.com/v1"
                    }
                }

                // API Key field (optional)
                key_section = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 6

                    key_label = <Label> {
                        text: "API Key (optional)"
                        draw_text: {
                            instance dark_mode: 0.0
                            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                            fn get_color(self) -> vec4 {
                                let light = vec4(0.216, 0.255, 0.318, 1.0);  // GRAY_700
                                let dark = vec4(0.631, 0.663, 0.710, 1.0);   // GRAY_400
                                return mix(light, dark, self.dark_mode);
                            }
                        }
                    }

                    key_input = <ModalTextInput> {
                        empty_text: "sk-..."
                        is_password: true
                    }
                }

                // Buttons
                actions = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    align: {x: 1.0, y: 0.5}
                    spacing: 12
                    margin: {top: 8}

                    cancel_button = <ModalSecondaryButton> {}
                    add_button = <ModalPrimaryButton> {}
                }
            }
        }
    }
}

#[derive(Clone, Debug, DefaultNone)]
pub enum AddProviderModalAction {
    None,
    AddClicked,
    CancelClicked,
}

#[derive(Live, LiveHook, Widget)]
pub struct AddProviderModal {
    #[deref]
    view: View,

    #[rust]
    dark_mode: f64,
}

impl Widget for AddProviderModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        let uid = self.widget_uid();

        // Extract actions
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => return,
        };

        // Handle add button click
        if self
            .view
            .button(ids!(dialog_container.dialog.actions.add_button))
            .clicked(actions)
        {
            cx.widget_action(uid, &scope.path, AddProviderModalAction::AddClicked);
        }

        // Handle cancel button click
        if self
            .view
            .button(ids!(dialog_container.dialog.actions.cancel_button))
            .clicked(actions)
        {
            cx.widget_action(uid, &scope.path, AddProviderModalAction::CancelClicked);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl AddProviderModalRef {
    /// Show the modal
    pub fn show(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.view.set_visible(cx, true);

            // Clear inputs - using full paths
            inner
                .view
                .text_input(ids!(dialog_container.dialog.name_section.name_input))
                .set_text(cx, "");
            inner
                .view
                .text_input(ids!(dialog_container.dialog.host_section.host_input))
                .set_text(cx, "");
            inner
                .view
                .text_input(ids!(dialog_container.dialog.key_section.key_input))
                .set_text(cx, "");

            inner.view.redraw(cx);
        }
    }

    /// Hide the modal
    pub fn hide(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.view.set_visible(cx, false);
            inner.view.redraw(cx);
        }
    }

    /// Check if modal is visible
    pub fn is_visible(&self) -> bool {
        self.borrow()
            .map(|inner| inner.view.visible())
            .unwrap_or(false)
    }

    /// Get the form values and create a Provider
    pub fn get_provider(&self) -> Option<Provider> {
        self.borrow().and_then(|inner| {
            let name = inner
                .view
                .text_input(ids!(dialog_container.dialog.name_section.name_input))
                .text();
            let host = inner
                .view
                .text_input(ids!(dialog_container.dialog.host_section.host_input))
                .text();
            let key = {
                let k = inner
                    .view
                    .text_input(ids!(dialog_container.dialog.key_section.key_input))
                    .text();
                if k.is_empty() {
                    None
                } else {
                    Some(k)
                }
            };

            if name.is_empty() || host.is_empty() {
                return None;
            }

            // Generate a unique ID from the name
            let id = ProviderId::from(name.to_lowercase().replace(" ", "_").replace("-", "_"));

            Some(Provider {
                id,
                name,
                url: host,
                api_key: key,
                provider_type: ProviderType::Custom,
                enabled: true,
                models: vec![],
                is_custom: true,
                connection_status: ProviderConnectionStatus::Disconnected,
            })
        })
    }

    /// Update dark mode for all modal elements
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.dark_mode = dark_mode;

            // Dialog background
            inner.view.view(ids!(dialog_container.dialog)).apply_over(
                cx,
                live! {
                    draw_bg: { dark_mode: (dark_mode) }
                },
            );

            // Header label
            inner
                .view
                .label(ids!(dialog_container.dialog.header.header_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Field labels
            inner
                .view
                .label(ids!(dialog_container.dialog.name_section.name_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(dialog_container.dialog.host_section.host_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .label(ids!(dialog_container.dialog.key_section.key_label))
                .apply_over(
                    cx,
                    live! {
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );

            // Text inputs
            inner
                .view
                .text_input(ids!(dialog_container.dialog.name_section.name_input))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .text_input(ids!(dialog_container.dialog.host_section.host_input))
                .apply_over(
                    cx,
                    live! {
                        draw_bg: { dark_mode: (dark_mode) }
                        draw_text: { dark_mode: (dark_mode) }
                    },
                );
            inner
                .view
                .text_input(ids!(dialog_container.dialog.key_section.key_input))
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

    /// Play save animation on the Add button (flash green)
    pub fn play_save_animation(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            // Flash the button green by setting save_progress
            inner
                .view
                .button(ids!(dialog_container.dialog.actions.add_button))
                .apply_over(cx, live! { draw_bg: { save_progress: 1.0 } });
            inner.view.redraw(cx);
        }
    }
}
