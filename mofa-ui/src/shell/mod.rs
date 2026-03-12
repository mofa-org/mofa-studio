//! Reusable Shell Components for MoFA Applications
//!
//! This module contains shell layout components:
//!
//! ## Components
//!
//! - [`MofaShell`] - Main application shell layout
//! - [`ShellHeader`] - Application header with title and controls
//! - [`ShellSidebar`] - Collapsible sidebar container
//! - [`StatusBar`] - Connection status and notifications
//!
//! ## Architecture
//!
//! The shell provides a consistent layout structure that apps can customize:
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │  ShellHeader                                │
//! ├─────────┬───────────────────────┬───────────┤
//! │         │                       │           │
//! │ Sidebar │     Center Content    │  Sidebar  │
//! │ (Left)  │     (App-specific)    │  (Right)  │
//! │         │                       │           │
//! ├─────────┴───────────────────────┴───────────┤
//! │  StatusBar                                  │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use mofa_ui::shell::*;
//!
//! live_design! {
//!     use mofa_ui::shell::layout::*;
//!     use mofa_ui::shell::header::*;
//!     use mofa_ui::shell::sidebar::*;
//!     use mofa_ui::shell::status_bar::*;
//!
//!     MyApp = <MofaShell> {
//!         header_slot: <ShellHeader> { title: "My App" }
//!         content_slot: <MyAppContent> {}
//!         status_bar_slot: <StatusBar> {}
//!     }
//! }
//! ```

pub mod header;
pub mod layout;
pub mod sidebar;
pub mod status_bar;

// Re-export main types
pub use header::{ShellHeader, ShellHeaderAction, ShellHeaderRef, ShellHeaderWidgetExt};
pub use layout::{MofaShell, MofaShellAction, MofaShellRef, MofaShellWidgetExt};
pub use sidebar::{
    ShellSidebar, ShellSidebarAction, ShellSidebarRef, ShellSidebarWidgetExt, SidebarItem,
};
pub use status_bar::{
    ConnectionStatus, StatusBar, StatusBarAction, StatusBarRef, StatusBarWidgetExt,
};

use makepad_widgets::Cx;

/// Register all shell live designs with Makepad.
///
/// Call this from mofa_ui::live_design().
///
/// NOTE: Currently disabled due to Makepad live_design parsing issues.
/// Shell components are defined but not registered until the parsing issue is resolved.
pub fn live_design(_cx: &mut Cx) {
    // TODO: Investigate why Makepad's live parser fails with "Unexpected token #"
    // when parsing these components.
    //
    // For now, apps should define their own shell layouts inline
    // or use the Rust APIs directly.
    //
    // layout::live_design(cx);
    // header::live_design(cx);
    // sidebar::live_design(cx);
    // status_bar::live_design(cx);
}
