use makepad_widgets::*;
use mofa_ui::widgets::chat_panel::{ChatMessage, ChatPanelWidgetExt};
use mofa_ui::widgets::chat_input::ChatInputWidgetExt;

const SYSTEM_PROMPT: &str = "You are an expert Makepad UI developer. \
    Generate high-quality Makepad .ds code based on natural language descriptions. \
    Focus on modern aesthetics, responsiveness, and proper use of Makepad widgets.";

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    
    use mofa_widgets::theme::*;
    use mofa_ui::widgets::chat_panel::ChatPanel;
    use mofa_ui::widgets::chat_input::ChatInput;

    pub MoFaUIGeneratorScreen = {{MoFaUIGeneratorScreen}} {
        width: Fill, height: Fill
        flow: Right
        spacing: 12
        padding: 12
        show_bg: true
        draw_bg: {
            color: (DARK_BG)
        }

        // Chat section on the left
        left_column = <View> {
            width: 450, height: Fill
            flow: Down
            spacing: 10

            chat_panel = <ChatPanel> {
                height: Fill
                empty_text: "Tell me what UI you want to build..."
            }
            chat_input = <ChatInput> {
                height: Fit
                placeholder: "e.g. Create a profile card with an avatar and stats"
            }
        }

        // Preview section on the right
        right_column = <RoundedView> {
            width: Fill, height: Fill
            show_bg: true
            draw_bg: {
                instance dark_mode: 1.0
                color: (PANEL_BG)
                border_radius: 8.0
            }
            padding: 20
            flow: Down
            spacing: 0

            header = <View> {
                width: Fill, height: Fit
                flow: Right
                align: {y: 0.5}
                padding: {bottom: 15}
                
                <Label> {
                    text: "UI Generator Output"
                    draw_text: {
                        text_style: <FONT_BOLD>{ font_size: 16.0 }
                        color: (TEXT_PRIMARY)
                    }
                }
                <Filler> {}
                
                tab_bar = <View> {
                    width: Fit, height: Fit
                    flow: Right
                    spacing: 5
                    
                    preview_tab = <Button> {
                        width: Fit, height: Fit
                        text: "Preview"
                    }
                    code_tab = <Button> {
                        width: Fit, height: Fit
                        text: "Code"
                    }
                }
            }

            divider = <View> {
                width: Fill, height: 1
                show_bg: true
                draw_bg: { color: (DIVIDER) }
            }

            content = <View> {
                width: Fill, height: Fill
                flow: Overlay
                padding: {top: 20}

                preview_view = <View> {
                    width: Fill, height: Fill
                    flow: Down
                    align: {x: 0.5, y: 0.5}
                    visible: true

                    placeholder_text = <Label> {
                        text: "Generated UI will be rendered here"
                        draw_text: {
                            color: (TEXT_SECONDARY)
                            text_style: <FONT_REGULAR>{ font_size: 14.0 }
                        }
                    }
                }

                code_view = <View> {
                    width: Fill, height: Fill
                    flow: Down
                    visible: false
                    
                    code_text = <TextInput> {
                        width: Fill, height: Fill
                        empty_message: "// Makepad DSL will appear here"
                        draw_text: {
                            text_style: <FONT_MONO>{ font_size: 11.0 }
                            color: (TEXT_PRIMARY)
                        }
                        show_bg: true
                        draw_bg: { color: (DARK_BG_DARK) }
                    }
                }
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct MoFaUIGeneratorScreen {
    #[deref]
    view: View,

    #[rust]
    messages: Vec<ChatMessage>,
    
    #[rust]
    generated_code: String,
}

impl Widget for MoFaUIGeneratorScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let actions = cx.capture_actions(|cx| self.view.handle_event(cx, event, scope));

        self.handle_actions(cx, &actions);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl MoFaUIGeneratorScreen {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // Handle Chat Input
        let chat_input = self.view.chat_input(id!(left_column.chat_input));
        if let Some(text) = chat_input.submitted(actions) {
            self.on_submit(cx, text);
        }

        // Handle Tabs
        if self.view.button(id!(right_column.header.tab_bar.preview_tab)).clicked(actions) {
            self.switch_tab(cx, true);
        }
        if self.view.button(id!(right_column.header.tab_bar.code_tab)).clicked(actions) {
            self.switch_tab(cx, false);
        }
    }

    fn on_submit(&mut self, cx: &mut Cx, text: String) {
        // Add user message
        self.messages.push(ChatMessage::new("You", text.clone()));
        
        // Mock generating code
        let mock_code = format!(
            "// Generated for: {}\n\nlive_design! {{\n    MyComponent = <View> {{\n        width: Fill, height: 200\n        show_bg: true\n        draw_bg: {{ color: #f00 }}\n        <Label> {{ text: \"Hello from AI!\" }}\n    }}\n}}", 
            text
        );
        self.generated_code = mock_code.clone();

        // Add AI response
        self.messages.push(ChatMessage::new("AI", "I've generated the Makepad UI code for you. You can check it in the 'Code' tab. Rendering..."));
        
        // Update components
        self.view.chat_panel(id!(left_column.chat_panel)).set_messages(cx, &self.messages);
        self.view.text_input(id!(right_column.content.code_view.code_text)).set_text(cx, &mock_code);
        
        // Show code tab by default when generated
        self.switch_tab(cx, false);
        
        self.view.redraw(cx);
    }

    fn switch_tab(&mut self, cx: &mut Cx, show_preview: bool) {
        self.view.view(id!(right_column.content.preview_view)).set_visible(show_preview);
        self.view.view(id!(right_column.content.code_view)).set_visible(!show_preview);
        self.view.redraw(cx);
    }
}
