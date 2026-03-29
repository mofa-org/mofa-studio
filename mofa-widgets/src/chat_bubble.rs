//! # Chat Bubble Widget
//!
//! A reusable chat bubble component for displaying messages with role-based styling.
//! Supports user, assistant, and system message types with appropriate visual differentiation.
//!
//! ## Features
//!
//! - **Role Support**: User (blue), Assistant (gray), System (yellow)
//! - **Responsive Layout**: Bubbles adapt to content width with max constraints
//! - **Theme Integration**: Uses centralized color palette
//!
//! ## Usage
//!
//! ```rust,ignore
//! live_design! {
//!     use mofa_widgets::chat_bubble::ChatBubble;
//!
//!     ChatView = <View> {
//!         user_msg = <ChatBubble> {
//!             role: 0.0,
//!             text: "Hello, how are you?"
//!         }
//!         assistant_msg = <ChatBubble> {
//!             role: 1.0,
//!             text: "I'm doing well, thank you!"
//!         }
//!     }
//! }
//! ```
//!
//! ## Updating Content
//!
//! Set the role and text via instance variables:
//!
//! ```rust,ignore
//! // Update bubble content
//! self.view.view(ids!(bubble)).apply_over(cx, live!{
//!     role: 1.0,
//!     text: "New message content"
//! });
//! ```
//!
//! ## Roles
//!
//! - **0.0 (User)**: Blue background, white text
//! - **1.0 (Assistant)**: Gray background, dark text
//! - **2.0 (System)**: Yellow background, dark text

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    // Import colors from theme
    use crate::theme::ACCENT_BLUE;
    use crate::theme::GRAY_100;
    use crate::theme::ACCENT_YELLOW;
    use crate::theme::TEXT_PRIMARY;
    use crate::theme::TEXT_PRIMARY_DARK;

    pub ChatBubble = {{ChatBubble}} <View> {
        width: Fill,
        height: Fit,
        margin: {top: 4, bottom: 4}
        // role: 0 = user, 1 = assistant, 2 = system
        instance role: 0.0
        instance text: ""

        draw_bg: {
            role: (role)
            instance role: 0.0

            fn get_bg_color(self) -> vec4 {
                if self.role < 0.5 {
                    return (ACCENT_BLUE);
                }
                if self.role < 1.5 {
                    return (GRAY_100);
                }
                return (ACCENT_YELLOW);
            }

            fn pixel(self) -> vec4 {
                let bg_color = self.get_bg_color();
                return vec4(bg_color.r, bg_color.g, bg_color.b, bg_color.a);
            }
        }

        bubble_container = <View> {
            width: Fill,
            height: Fit,
            padding: 12,
            spacing: 0,

            align: {
                x: 0.0, // Left align by default
                y: 0.5
            }

            text_label = <Label> {
                width: Fill,
                height: Fit,
                draw_text: {
                    role: (role)
                    instance role: 0.0

                    fn get_text_color(self) -> vec4 {
                        if self.role < 0.5 {
                            return (TEXT_PRIMARY); // White text on blue
                        }
                        return (TEXT_PRIMARY_DARK);
                    }

                    text_style: {
                        font_size: 14.0,
                        line_spacing: 1.2,
                    }
                    color: (TEXT_PRIMARY_DARK)
                    wrap: Word

                    fn pixel(self) -> vec4 {
                        let color = self.get_text_color();
                        return vec4(color.r, color.g, color.b, color.a);
                    }
                }
                text: (text)
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct ChatBubble {
    #[deref]
    view: View,
}

impl Widget for ChatBubble {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}