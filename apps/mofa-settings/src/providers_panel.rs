//! Providers Panel - List of AI providers with collapsible custom providers section

use makepad_widgets::*;
use crate::data::{Provider, ProviderId, Preferences};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use mofa_widgets::theme::*;

    ICO_OPENAI = dep("crate://self/resources/icons/openai.svg")
    ICO_DEEPSEEK = dep("crate://self/resources/icons/deepseek.svg")
    ICO_CUSTOM = dep("crate://self/resources/icons/custom-model.svg")
    IMG_QWEN = dep("crate://self/resources/icons/qwen.png")

    // Provider item - clickable row with hover effect
    // Uses custom shader with instance hover variable for proper hover effects
    ProviderItem = <View> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}
        margin: 0
        show_bg: true
        draw_bg: {
            instance hover: 0.0
            instance selected: 0.0
            instance dark_mode: 0.0

            fn pixel(self) -> vec4 {
                // Light mode colors
                let light_normal = (WHITE);
                let light_hover = vec4(0.95, 0.96, 0.98, 1.0);  // SLATE_100
                let light_selected = vec4(0.86, 0.90, 0.98, 1.0);  // Blue-100

                // Dark mode colors
                let dark_normal = (SLATE_800);
                let dark_hover = vec4(0.2, 0.25, 0.33, 1.0);  // SLATE_700
                let dark_selected = vec4(0.12, 0.23, 0.37, 1.0);  // Blue-ish

                // Pick base colors based on dark mode
                let normal = mix(light_normal, dark_normal, self.dark_mode);
                let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                let selected_color = mix(light_selected, dark_selected, self.dark_mode);

                // Calculate final color: selected takes priority, then hover
                let base = mix(normal, hover_color, self.hover);
                return mix(base, selected_color, self.selected);
            }
        }
        cursor: Hand
        flow: Right
        align: {x: 0.0, y: 0.5}
    }

    // Custom provider item - View with hover/selected effects
    // Uses custom shader with instance variables (proven to work like SectionHeader)
    CustomProviderItem = <View> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}
        margin: 0
        show_bg: true
        cursor: Hand
        flow: Right
        align: {x: 0.0, y: 0.5}

        draw_bg: {
            instance hover: 0.0
            instance selected: 0.0
            instance dark_mode: 0.0

            fn pixel(self) -> vec4 {
                // Light mode colors
                let light_normal = (WHITE);
                let light_hover = #DAE6F9;  // Light blue hover
                let light_selected = #DBEAFE;  // Blue selected

                // Dark mode colors
                let dark_normal = (SLATE_800);
                let dark_hover = #334155;  // Slate-700
                let dark_selected = #1E3A5F;  // Blue-ish selected

                // Pick base colors based on dark mode
                let normal = mix(light_normal, dark_normal, self.dark_mode);
                let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                let selected_color = mix(light_selected, dark_selected, self.dark_mode);

                // Calculate final color: selected takes priority, then hover
                let base = mix(normal, hover_color, self.hover);
                let final_color = mix(base, selected_color, self.selected);

                return final_color;
            }
        }

        <Icon> {
            draw_icon: {
                svg_file: (ICO_CUSTOM)
                fn get_color(self) -> vec4 { return (SLATE_500); }
            }
            icon_walk: {width: 20, height: 20, margin: {right: 10}}
        }

        custom_label = <Label> {
            width: Fill
            draw_text: {
                text_style: <FONT_REGULAR>{ font_size: 12.0 }
                color: #383A40
            }
            text: "Custom Provider"
        }
    }

    // Provider label
    ProviderLabel = <Label> {
        draw_text: {
            color: (GRAY_700)
            text_style: <FONT_REGULAR>{ font_size: 12.0 }
        }
    }

    // Section header - matching sidebar ShowMoreContainer style
    // Entire header (including arrow) has hover effect
    SectionHeader = <View> {
        width: Fill, height: Fit
        cursor: Hand
        show_bg: true

        // Hover background covers entire header including arrow
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

        // Inner container for text and arrow
        header_content = <View> {
            width: Fill, height: Fit
            flow: Right
            align: {x: 0.0, y: 0.5}

            header_label = <Label> {
                width: Fill
                padding: {top: 12, bottom: 12, left: 10}
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: <FONT_REGULAR>{ font_size: 12.0 }
                    fn get_color(self) -> vec4 {
                        return mix((SLATE_800), (SLATE_200), self.dark_mode);
                    }
                }
            }

            // Arrow on right side
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
    }

    // Add provider button
    AddProviderButton = <View> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance hover: 0.0
            instance dark_mode: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let light_normal = (WHITE);
                let light_hover = (SLATE_100);
                let dark_normal = (SLATE_800);
                let dark_hover = (SLATE_700);
                let normal = mix(light_normal, dark_normal, self.dark_mode);
                let hover_color = mix(light_hover, dark_hover, self.dark_mode);
                let color = mix(normal, hover_color, self.hover);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 0.0);
                sdf.fill(color);

                // Top border
                let border = mix((BORDER), (BORDER_DARK), self.dark_mode);
                sdf.box(0.0, 0.0, self.rect_size.x, 1.0, 0.0);
                sdf.fill(border);

                return sdf.result;
            }
        }

        add_icon = <Label> {
            text: "+"
            draw_text: {
                instance dark_mode: 0.0
                text_style: <FONT_BOLD>{ font_size: 14.0 }
                fn get_color(self) -> vec4 {
                    return mix((ACCENT_BLUE), (ACCENT_BLUE_DARK), self.dark_mode);
                }
            }
            margin: {right: 8}
        }

        add_label = <Label> {
            text: "Add Custom Provider"
            draw_text: {
                instance dark_mode: 0.0
                text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                fn get_color(self) -> vec4 {
                    return mix((ACCENT_BLUE), (ACCENT_BLUE_DARK), self.dark_mode);
                }
            }
        }
    }

    // Providers panel - left side of settings
    pub ProvidersPanel = {{ProvidersPanel}} {
        width: 280, height: Fill
        flow: Down
        spacing: 0

        show_bg: true
        draw_bg: {
            instance dark_mode: 0.0
            fn get_color(self) -> vec4 {
                return mix((WHITE), (SLATE_800), self.dark_mode);
            }
        }

        // Header
        header = <View> {
            width: Fill, height: Fit
            padding: {left: 16, right: 16, top: 16, bottom: 12}

            header_label = <Label> {
                text: "Providers"
                draw_text: {
                    instance dark_mode: 0.0
                    text_style: <FONT_BOLD>{ font_size: 14.0 }
                    fn get_color(self) -> vec4 {
                        return mix((SLATE_800), (TEXT_PRIMARY_DARK), self.dark_mode);
                    }
                }
            }
        }

        // Scrollable provider list
        scroll_view = <ScrollYView> {
            width: Fill, height: Fill
            flow: Down
            spacing: 0
            scroll_bars: <ScrollBars> {
                show_scroll_x: false
                show_scroll_y: true
            }

            // Built-in providers list
            list_container = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 0

                openai_item = <ProviderItem> {
                    <Icon> {
                        draw_icon: {
                            svg_file: (ICO_OPENAI)
                            fn get_color(self) -> vec4 { return (SLATE_500); }
                        }
                        icon_walk: {width: 24, height: 24, margin: {right: 10}}
                    }
                    openai_label = <ProviderLabel> {
                        text: "OpenAI"
                    }
                }

                deepseek_item = <ProviderItem> {
                    <Icon> {
                        draw_icon: {
                            svg_file: (ICO_DEEPSEEK)
                            fn get_color(self) -> vec4 { return (SLATE_500); }
                        }
                        icon_walk: {width: 20, height: 20, margin: {right: 10}}
                    }
                    deepseek_label = <ProviderLabel> {
                        text: "DeepSeek"
                    }
                }

                alibaba_item = <ProviderItem> {
                    <Icon> {
                        draw_icon: {
                            svg_file: (ICO_DEEPSEEK)
                            fn get_color(self) -> vec4 { return (SLATE_500); }
                        }
                        icon_walk: {width: 20, height: 20, margin: {right: 10}}
                    }
                    alibaba_label = <ProviderLabel> {
                        text: "Alibaba Cloud (Qwen)"
                    }
                }

                nvidia_item = <ProviderItem> {
                    <Icon> {
                        draw_icon: {
                            svg_file: (ICO_DEEPSEEK)
                            fn get_color(self) -> vec4 { return (SLATE_500); }
                        }
                        icon_walk: {width: 20, height: 20, margin: {right: 10}}
                    }
                    nvidia_label = <ProviderLabel> {
                        text: "NVIDIA"
                    }
                }
            }

            // Custom providers section header (collapsible) - matching sidebar "Show More" style
            custom_header = <SectionHeader> {
                visible: false
                header_content = {
                    header_label = { text: "Show More" }
                    arrow_label = { text: ">" }
                }
            }

            // Collapsible custom providers list - static items like sidebar
            custom_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 0
                visible: true  // Start visible for area calculation

                // Static custom provider items (up to 10 slots)
                custom_provider_1 = <CustomProviderItem> {}
                custom_provider_2 = <CustomProviderItem> {}
                custom_provider_3 = <CustomProviderItem> {}
                custom_provider_4 = <CustomProviderItem> {}
                custom_provider_5 = <CustomProviderItem> {}
                custom_provider_6 = <CustomProviderItem> {}
                custom_provider_7 = <CustomProviderItem> {}
                custom_provider_8 = <CustomProviderItem> {}
                custom_provider_9 = <CustomProviderItem> {}
                custom_provider_10 = <CustomProviderItem> {}
            }

        }

        // Divider before add button
        add_divider = <View> {
            width: Fill, height: 1
            margin: {top: 8, bottom: 8}
            show_bg: true
            draw_bg: {
                instance dark_mode: 0.0
                fn pixel(self) -> vec4 {
                    return mix((BORDER), (BORDER_DARK), self.dark_mode);
                }
            }
        }

        // Add provider button fixed at bottom (outside scroll area)
        add_button = <AddProviderButton> {}
    }
}

#[derive(Clone, Debug, DefaultNone)]
pub enum ProvidersPanelAction {
    None,
    Selected(ProviderId),
    AddProviderClicked,
}

#[derive(Live, LiveHook, Widget)]
pub struct ProvidersPanel {
    #[deref]
    view: View,

    #[rust]
    selected_provider_id: Option<ProviderId>,

    #[rust]
    dark_mode: bool,

    /// Custom providers loaded from preferences
    #[rust]
    custom_providers: Vec<Provider>,

    /// Whether custom providers section is expanded
    #[rust]
    custom_section_expanded: bool,
}

impl Widget for ProvidersPanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        let uid = self.widget_uid();

        // Provider items for hover and click handling
        let items = [
            ids!(scroll_view.list_container.openai_item),
            ids!(scroll_view.list_container.deepseek_item),
            ids!(scroll_view.list_container.alibaba_item),
            ids!(scroll_view.list_container.nvidia_item),
        ];

        // Handle hover effects for built-in providers
        for item_id in items.iter() {
            let item = self.view.view(item_id.clone());
            match event.hits(cx, item.area()) {
                Hit::FingerHoverIn(_) => {
                    let is_selected = self.is_builtin_item_selected(*item_id);
                    if !is_selected {
                        self.apply_item_hover(cx, *item_id, true);
                    }
                }
                Hit::FingerHoverOut(_) => {
                    let is_selected = self.is_builtin_item_selected(*item_id);
                    if !is_selected {
                        self.apply_item_hover(cx, *item_id, false);
                    }
                }
                _ => {}
            }
        }

        // Handle hover for custom header (target the header_bg view)
        let header_bg = self.view.view(ids!(scroll_view.custom_header));
        match event.hits(cx, header_bg.area()) {
            Hit::FingerHoverIn(_) => {
                self.view.view(ids!(scroll_view.custom_header)).apply_over(cx, live!{
                    draw_bg: { hover: 1.0 }
                });
                self.view.redraw(cx);
            }
            Hit::FingerHoverOut(_) => {
                self.view.view(ids!(scroll_view.custom_header)).apply_over(cx, live!{
                    draw_bg: { hover: 0.0 }
                });
                self.view.redraw(cx);
            }
            _ => {}
        }

        // Handle hover for add button
        let add_button = self.view.view(ids!(add_button));
        match event.hits(cx, add_button.area()) {
            Hit::FingerHoverIn(_) => {
                self.view.view(ids!(add_button)).apply_over(cx, live!{
                    draw_bg: { hover: 1.0 }
                });
                self.view.redraw(cx);
            }
            Hit::FingerHoverOut(_) => {
                self.view.view(ids!(add_button)).apply_over(cx, live!{
                    draw_bg: { hover: 0.0 }
                });
                self.view.redraw(cx);
            }
            _ => {}
        }

        // Handle hover for custom provider items (manual, like built-in providers)
        let custom_items = [
            ids!(scroll_view.custom_section.custom_provider_1),
            ids!(scroll_view.custom_section.custom_provider_2),
            ids!(scroll_view.custom_section.custom_provider_3),
            ids!(scroll_view.custom_section.custom_provider_4),
            ids!(scroll_view.custom_section.custom_provider_5),
            ids!(scroll_view.custom_section.custom_provider_6),
            ids!(scroll_view.custom_section.custom_provider_7),
            ids!(scroll_view.custom_section.custom_provider_8),
            ids!(scroll_view.custom_section.custom_provider_9),
            ids!(scroll_view.custom_section.custom_provider_10),
        ];

        for (i, path) in custom_items.iter().enumerate() {
            if i >= self.custom_providers.len() {
                break; // Skip hidden items
            }
            let item = self.view.view(*path);
            let area = item.area();
            match event.hits(cx, area) {
                Hit::FingerHoverIn(_) => {
                    let is_selected = self.selected_provider_id.as_ref() == Some(&self.custom_providers[i].id);
                    if !is_selected {
                        // Use instance variable for hover effect
                        self.view.view(*path).apply_over(cx, live!{
                            draw_bg: { hover: 1.0 }
                        });
                        self.view.redraw(cx);
                    }
                }
                Hit::FingerHoverOut(_) => {
                    let is_selected = self.selected_provider_id.as_ref() == Some(&self.custom_providers[i].id);
                    if !is_selected {
                        // Reset hover state
                        self.view.view(*path).apply_over(cx, live!{
                            draw_bg: { hover: 0.0 }
                        });
                        self.view.redraw(cx);
                    }
                }
                _ => {}
            }
        }

        // Extract actions - return early if not an Actions event
        let actions = match event {
            Event::Actions(actions) => actions.as_slice(),
            _ => return,
        };

        // Handle custom header click (expand/collapse) - match sidebar pattern
        if self.view.view(ids!(scroll_view.custom_header)).finger_up(actions).is_some() {
            self.custom_section_expanded = !self.custom_section_expanded;
            self.view.view(ids!(scroll_view.custom_section)).set_visible(cx, self.custom_section_expanded);

            // Update text and arrow like sidebar
            if self.custom_section_expanded {
                self.view.label(ids!(scroll_view.custom_header.header_content.header_label)).set_text(cx, "Show Less");
                self.view.label(ids!(scroll_view.custom_header.header_content.arrow_label)).set_text(cx, "^");
            } else {
                self.view.label(ids!(scroll_view.custom_header.header_content.header_label)).set_text(cx, "Show More");
                self.view.label(ids!(scroll_view.custom_header.header_content.arrow_label)).set_text(cx, ">");
            }

            self.view.redraw(cx);
        }

        // Handle add button click
        if self.view.view(ids!(add_button)).finger_up(actions).is_some() {
            cx.widget_action(uid, &scope.path, ProvidersPanelAction::AddProviderClicked);
        }

        // Handle provider item clicks
        let mut new_selection: Option<ProviderId> = None;

        if self.view.view(ids!(scroll_view.list_container.openai_item)).finger_up(actions).is_some() {
            new_selection = Some(ProviderId::from("openai"));
        }
        if self.view.view(ids!(scroll_view.list_container.deepseek_item)).finger_up(actions).is_some() {
            new_selection = Some(ProviderId::from("deepseek"));
        }
        if self.view.view(ids!(scroll_view.list_container.alibaba_item)).finger_up(actions).is_some() {
            new_selection = Some(ProviderId::from("alibaba_cloud"));
        }
        if self.view.view(ids!(scroll_view.list_container.nvidia_item)).finger_up(actions).is_some() {
            new_selection = Some(ProviderId::from("nvidia"));
        }

        if let Some(id) = new_selection {
            if self.selected_provider_id.as_ref() != Some(&id) {
                self.select_provider_internal(cx, &id);
                cx.widget_action(uid, &scope.path, ProvidersPanelAction::Selected(id));
            }
        }

        // Handle custom provider item clicks (View with finger_up, like built-in providers)
        macro_rules! handle_custom_provider_click {
            ($self:expr, $cx:expr, $actions:expr, $uid:expr, $scope:expr, $($idx:expr => $path:expr),+ $(,)?) => {
                $(
                    if $self.view.view($path).finger_up($actions).is_some() {
                        if $idx < $self.custom_providers.len() {
                            let provider_id = $self.custom_providers[$idx].id.clone();
                            if $self.selected_provider_id.as_ref() != Some(&provider_id) {
                                $self.select_provider_internal($cx, &provider_id);
                                $cx.widget_action($uid, &$scope.path, ProvidersPanelAction::Selected(provider_id));
                            }
                        }
                    }
                )+
            };
        }

        handle_custom_provider_click!(self, cx, actions, uid, scope,
            0 => ids!(scroll_view.custom_section.custom_provider_1),
            1 => ids!(scroll_view.custom_section.custom_provider_2),
            2 => ids!(scroll_view.custom_section.custom_provider_3),
            3 => ids!(scroll_view.custom_section.custom_provider_4),
            4 => ids!(scroll_view.custom_section.custom_provider_5),
            5 => ids!(scroll_view.custom_section.custom_provider_6),
            6 => ids!(scroll_view.custom_section.custom_provider_7),
            7 => ids!(scroll_view.custom_section.custom_provider_8),
            8 => ids!(scroll_view.custom_section.custom_provider_9),
            9 => ids!(scroll_view.custom_section.custom_provider_10),
        );
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Simple draw - no PortalList, just static buttons
        self.view.draw_walk(cx, scope, walk)
    }
}

impl ProvidersPanel {
    fn is_builtin_item_selected(&self, item_id: &[LiveId]) -> bool {
        match self.selected_provider_id.as_ref().map(|id| id.as_str()) {
            Some("openai") => item_id == ids!(scroll_view.list_container.openai_item),
            Some("deepseek") => item_id == ids!(scroll_view.list_container.deepseek_item),
            Some("alibaba_cloud") => item_id == ids!(scroll_view.list_container.alibaba_item),
            Some("nvidia") => item_id == ids!(scroll_view.list_container.nvidia_item),
            _ => false,
        }
    }

    fn apply_item_hover(&mut self, cx: &mut Cx, item_id: &[LiveId], hover: bool) {
        let hover_val = if hover { 1.0 } else { 0.0 };
        self.view.view(item_id).apply_over(cx, live!{
            draw_bg: { hover: (hover_val) }
        });
        self.view.redraw(cx);
    }

    fn select_provider_internal(&mut self, cx: &mut Cx, provider_id: &ProviderId) {
        // Reset all built-in items to unselected and clear hover state.
        let items = [
            ids!(scroll_view.list_container.openai_item),
            ids!(scroll_view.list_container.deepseek_item),
            ids!(scroll_view.list_container.alibaba_item),
            ids!(scroll_view.list_container.nvidia_item),
        ];

        for item_id in &items {
            self.view.view(item_id.clone()).apply_over(cx, live!{
                draw_bg: { selected: 0.0, hover: 0.0 }
            });
        }

        // Reset all custom provider items to unselected (use instance variables)
        let custom_items = [
            ids!(scroll_view.custom_section.custom_provider_1),
            ids!(scroll_view.custom_section.custom_provider_2),
            ids!(scroll_view.custom_section.custom_provider_3),
            ids!(scroll_view.custom_section.custom_provider_4),
            ids!(scroll_view.custom_section.custom_provider_5),
            ids!(scroll_view.custom_section.custom_provider_6),
            ids!(scroll_view.custom_section.custom_provider_7),
            ids!(scroll_view.custom_section.custom_provider_8),
            ids!(scroll_view.custom_section.custom_provider_9),
            ids!(scroll_view.custom_section.custom_provider_10),
        ];

        for path in &custom_items {
            self.view.view(*path).apply_over(cx, live!{
                draw_bg: { selected: 0.0, hover: 0.0 }
            });
        }

        // Apply selected color to the selected item
        let selected = provider_id.as_str();
        match selected {
            "openai" => {
                self.view.view(ids!(scroll_view.list_container.openai_item)).apply_over(cx, live!{
                    draw_bg: { selected: 1.0, hover: 0.0 }
                });
            }
            "deepseek" => {
                self.view.view(ids!(scroll_view.list_container.deepseek_item)).apply_over(cx, live!{
                    draw_bg: { selected: 1.0, hover: 0.0 }
                });
            }
            "alibaba_cloud" => {
                self.view.view(ids!(scroll_view.list_container.alibaba_item)).apply_over(cx, live!{
                    draw_bg: { selected: 1.0, hover: 0.0 }
                });
            }
            "nvidia" => {
                self.view.view(ids!(scroll_view.list_container.nvidia_item)).apply_over(cx, live!{
                    draw_bg: { selected: 1.0, hover: 0.0 }
                });
            }
            _ => {
                // Check if it's a custom provider - use instance variable for selected
                for (i, provider) in self.custom_providers.iter().enumerate() {
                    if provider.id == *provider_id && i < custom_items.len() {
                        self.view.view(custom_items[i]).apply_over(cx, live!{
                            draw_bg: { selected: 1.0, hover: 0.0 }
                        });
                        break;
                    }
                }
            }
        }

        self.selected_provider_id = Some(provider_id.clone());
        self.view.redraw(cx);
    }
}

impl ProvidersPanelRef {
    /// Get the currently selected provider ID
    pub fn selected_provider_id(&self) -> Option<ProviderId> {
        self.borrow().and_then(|inner| inner.selected_provider_id.clone())
    }

    /// Set the selected provider
    pub fn select_provider(&self, cx: &mut Cx, provider_id: &ProviderId) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.select_provider_internal(cx, provider_id);
        }
    }

    /// Map item name to provider ID
    pub fn item_to_provider_id(name: &str) -> Option<ProviderId> {
        match name {
            "openai_item" => Some(ProviderId::from("openai")),
            "deepseek_item" => Some(ProviderId::from("deepseek")),
            "alibaba_item" => Some(ProviderId::from("alibaba_cloud")),
            "nvidia_item" => Some(ProviderId::from("nvidia")),
            _ => None,
        }
    }

    /// Map button name to provider ID (legacy alias)
    pub fn button_to_provider_id(name: &str) -> Option<ProviderId> {
        Self::item_to_provider_id(name)
    }

    /// Update dark mode for this widget
    pub fn update_dark_mode(&self, cx: &mut Cx, dark_mode: f64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.dark_mode = dark_mode > 0.5;
            let is_dark = inner.dark_mode;

            // Panel background
            inner.view.apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Header label
            inner.view.label(ids!(header.header_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // Color constants
            let dark_text = vec4(0.95, 0.96, 0.98, 1.0);
            let light_text = vec4(0.22, 0.25, 0.32, 1.0);

            let selected = inner.selected_provider_id.as_ref().map(|id| id.as_str());

            // Update built-in provider items
            let items = [
                ("openai", ids!(scroll_view.list_container.openai_item), ids!(scroll_view.list_container.openai_item.openai_label)),
                ("deepseek", ids!(scroll_view.list_container.deepseek_item), ids!(scroll_view.list_container.deepseek_item.deepseek_label)),
                ("alibaba_cloud", ids!(scroll_view.list_container.alibaba_item), ids!(scroll_view.list_container.alibaba_item.alibaba_label)),
                ("nvidia", ids!(scroll_view.list_container.nvidia_item), ids!(scroll_view.list_container.nvidia_item.nvidia_label)),
            ];

            for (provider_name, item_path, label_path) in items {
                let is_selected = selected == Some(provider_name);
                let text_color = if is_dark { dark_text } else { light_text };
                let selected_val = if is_selected { 1.0 } else { 0.0 };

                inner.view.view(item_path).apply_over(cx, live!{
                    draw_bg: {
                        dark_mode: (dark_mode),
                        selected: (selected_val),
                        hover: 0.0
                    }
                });
                inner.view.label(label_path).apply_over(cx, live!{ draw_text: { color: (text_color) } });
            }

            // Custom header - matching sidebar style
            inner.view.view(ids!(scroll_view.custom_header)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(scroll_view.custom_header.header_content.header_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(scroll_view.custom_header.header_content.arrow_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            // Custom provider items (static, like sidebar) - use instance variables
            let custom_item_paths = [
                (ids!(scroll_view.custom_section.custom_provider_1), ids!(scroll_view.custom_section.custom_provider_1.custom_label)),
                (ids!(scroll_view.custom_section.custom_provider_2), ids!(scroll_view.custom_section.custom_provider_2.custom_label)),
                (ids!(scroll_view.custom_section.custom_provider_3), ids!(scroll_view.custom_section.custom_provider_3.custom_label)),
                (ids!(scroll_view.custom_section.custom_provider_4), ids!(scroll_view.custom_section.custom_provider_4.custom_label)),
                (ids!(scroll_view.custom_section.custom_provider_5), ids!(scroll_view.custom_section.custom_provider_5.custom_label)),
                (ids!(scroll_view.custom_section.custom_provider_6), ids!(scroll_view.custom_section.custom_provider_6.custom_label)),
                (ids!(scroll_view.custom_section.custom_provider_7), ids!(scroll_view.custom_section.custom_provider_7.custom_label)),
                (ids!(scroll_view.custom_section.custom_provider_8), ids!(scroll_view.custom_section.custom_provider_8.custom_label)),
                (ids!(scroll_view.custom_section.custom_provider_9), ids!(scroll_view.custom_section.custom_provider_9.custom_label)),
                (ids!(scroll_view.custom_section.custom_provider_10), ids!(scroll_view.custom_section.custom_provider_10.custom_label)),
            ];

            let custom_text_color = if is_dark { dark_text } else { light_text };

            for (i, (item_path, label_path)) in custom_item_paths.iter().enumerate() {
                let is_selected_custom = inner
                    .custom_providers
                    .get(i)
                    .map(|provider| selected == Some(provider.id.as_str()))
                    .unwrap_or(false);
                let selected_val = if is_selected_custom { 1.0 } else { 0.0 };

                inner.view.view(item_path).apply_over(cx, live!{
                    draw_bg: {
                        dark_mode: (dark_mode),
                        selected: (selected_val),
                        hover: 0.0
                    }
                });
                inner.view.label(label_path).apply_over(cx, live!{
                    draw_text: { color: (custom_text_color) }
                });
            }

            // Add divider
            inner.view.view(ids!(add_divider)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });

            // Add button
            inner.view.view(ids!(add_button)).apply_over(cx, live!{
                draw_bg: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(add_button.add_icon)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });
            inner.view.label(ids!(add_button.add_label)).apply_over(cx, live!{
                draw_text: { dark_mode: (dark_mode) }
            });

            inner.view.redraw(cx);
        }
    }

    /// Load custom providers from preferences
    pub fn load_providers(&self, cx: &mut Cx) {
        let needs_redraw = {
            if let Some(mut inner) = self.borrow_mut() {
                let prefs = Preferences::load();

                // Filter to only custom providers
                inner.custom_providers = prefs.providers
                    .into_iter()
                    .filter(|p| p.is_custom)
                    .collect();

                let count = inner.custom_providers.len();
                ::log::info!("Loaded {} custom providers", count);

                // Show/hide custom header based on whether we have custom providers
                let has_custom = count > 0;
                inner.view.view(ids!(scroll_view.custom_header)).set_visible(cx, has_custom);

                // Configure static custom provider items (show/hide and set text)
                let item_paths = [
                    (ids!(scroll_view.custom_section.custom_provider_1), ids!(scroll_view.custom_section.custom_provider_1.custom_label)),
                    (ids!(scroll_view.custom_section.custom_provider_2), ids!(scroll_view.custom_section.custom_provider_2.custom_label)),
                    (ids!(scroll_view.custom_section.custom_provider_3), ids!(scroll_view.custom_section.custom_provider_3.custom_label)),
                    (ids!(scroll_view.custom_section.custom_provider_4), ids!(scroll_view.custom_section.custom_provider_4.custom_label)),
                    (ids!(scroll_view.custom_section.custom_provider_5), ids!(scroll_view.custom_section.custom_provider_5.custom_label)),
                    (ids!(scroll_view.custom_section.custom_provider_6), ids!(scroll_view.custom_section.custom_provider_6.custom_label)),
                    (ids!(scroll_view.custom_section.custom_provider_7), ids!(scroll_view.custom_section.custom_provider_7.custom_label)),
                    (ids!(scroll_view.custom_section.custom_provider_8), ids!(scroll_view.custom_section.custom_provider_8.custom_label)),
                    (ids!(scroll_view.custom_section.custom_provider_9), ids!(scroll_view.custom_section.custom_provider_9.custom_label)),
                    (ids!(scroll_view.custom_section.custom_provider_10), ids!(scroll_view.custom_section.custom_provider_10.custom_label)),
                ];

                for (i, (item_path, label_path)) in item_paths.iter().enumerate() {
                    let item = inner.view.view(*item_path);
                    if i < count {
                        // Show and configure this item
                        item.set_visible(cx, true);
                        inner.view.label(*label_path).set_text(cx, &inner.custom_providers[i].name);
                    } else {
                        // Hide unused items
                        item.set_visible(cx, false);
                    }
                }

                // If there are custom providers and section is expanded, show it
                if has_custom && inner.custom_section_expanded {
                    inner.view.view(ids!(scroll_view.custom_section)).set_visible(cx, true);
                }

                // Auto-expand if there are custom providers
                if has_custom && !inner.custom_section_expanded {
                    inner.custom_section_expanded = true;
                    inner.view.view(ids!(scroll_view.custom_section)).set_visible(cx, true);
                    inner.view.label(ids!(scroll_view.custom_header.header_content.header_label)).set_text(cx, "Show Less");
                    inner.view.label(ids!(scroll_view.custom_header.header_content.arrow_label)).set_text(cx, "^");
                }

                true
            } else {
                false
            }
        };

        if needs_redraw {
            cx.redraw_all();
        }
    }

    /// Refresh the provider list (reload from preferences)
    pub fn refresh(&self, cx: &mut Cx) {
        self.load_providers(cx);
    }

    /// Select a provider by ID and update visual state
    pub fn select_and_highlight(&self, cx: &mut Cx, provider_id: &ProviderId) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.select_provider_internal(cx, provider_id);
        }
    }
}
