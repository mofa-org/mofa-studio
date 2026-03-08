use mofa_widgets::{MofaApp, AppInfo, PageId};
use makepad_widgets::*;

pub mod screen;

pub struct MoFaUIGeneratorApp;

impl MofaApp for MoFaUIGeneratorApp {
    fn info() -> AppInfo {
        AppInfo {
            name: "UI Generator",
            id: "mofa-ui-generator",
            description: "AI-powered Makepad UI Generator",
            tab_id: Some(live_id!(ui_generator_tab)),
            page_id: Some(live_id!(ui_generator_page)),
            show_in_sidebar: true,
        }
    }

    fn live_design(cx: &mut Cx) {
        crate::screen::live_design(cx);
    }
}
