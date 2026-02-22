//! # DoraBridge Trait and Bridge Infrastructure
//!
//! This module defines the [`DoraBridge`] trait and [`BridgeState`] enum
//! for connecting MoFA widgets to Dora dataflows as dynamic nodes.
//!
//! ## Architecture
//!
//! Each widget type (audio player, chat, logs) has its own bridge that:
//! 1. Connects to Dora as a dynamic node
//! 2. Receives data from Dora inputs
//! 3. Pushes data to [`SharedDoraState`](crate::SharedDoraState) for UI consumption
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
//! │  Dora Dataflow  │────▶│  DoraBridge     │────▶│ SharedDoraState │
//! │  (TTS audio)    │     │  (impl trait)   │     │  (UI reads)     │
//! └─────────────────┘     └─────────────────┘     └─────────────────┘
//! ```
//!
//! ## Available Bridges
//!
//! | Bridge | Node ID | Purpose |
//! |--------|---------|---------|
//! | AudioPlayerBridge | `mofa-audio-player` | Receives TTS audio |
//! | PromptInputBridge | `mofa-prompt-input` | Receives chat messages |
//! | SystemLogBridge | `mofa-system-log` | Receives log entries |
//!
//! ## Connection States
//!
//! Bridges progress through these states:
//!
//! ```text
//! Disconnected → Connecting → Connected → Disconnecting → Disconnected
//!                    ↓                         ↓
//!                  Error ←───────────────────←─┘
//! ```

use crate::data::DoraData;
use crate::error::BridgeResult;

/// Connection state for a Dora bridge.
///
/// Represents the lifecycle of a bridge connection:
///
/// | State | Description |
/// |-------|-------------|
/// | `Disconnected` | Not connected to Dora |
/// | `Connecting` | Connection in progress |
/// | `Connected` | Ready to send/receive data |
/// | `Disconnecting` | Graceful shutdown in progress |
/// | `Error` | Connection failed or was lost |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BridgeState {
    /// Bridge is disconnected
    #[default]
    Disconnected,
    /// Bridge is connecting
    Connecting,
    /// Bridge is connected and ready
    Connected,
    /// Bridge is disconnecting
    Disconnecting,
    /// Bridge encountered an error
    Error,
}


/// Core trait for all Dora bridges.
///
/// Implement this trait to create a new widget bridge that connects to
/// Dora as a dynamic node and communicates with the UI via [`SharedDoraState`](crate::SharedDoraState).
///
/// # Implementation Notes
///
/// - Bridges run in worker threads (must be `Send + Sync`)
/// - Data is pushed to `SharedDoraState`, not returned directly
/// - Status updates go through `SharedDoraState::status`
///
/// # Example
///
/// ```rust,ignore
/// struct MyBridge {
///     state: BridgeState,
///     node_id: String,
///     shared_state: Arc<SharedDoraState>,
/// }
///
/// impl DoraBridge for MyBridge {
///     fn node_id(&self) -> &str { &self.node_id }
///     fn state(&self) -> BridgeState { self.state }
///     fn connect(&mut self) -> BridgeResult<()> {
///         self.state = BridgeState::Connected;
///         self.shared_state.add_bridge(self.node_id.clone());
///         Ok(())
///     }
///     // ... other methods
/// }
/// ```
pub trait DoraBridge: Send + Sync {
    /// Get the node ID for this bridge (e.g., "mofa-audio-player")
    fn node_id(&self) -> &str;

    /// Get current connection state
    fn state(&self) -> BridgeState;

    /// Connect to the dora dataflow as a dynamic node
    fn connect(&mut self) -> BridgeResult<()>;

    /// Disconnect from dora
    fn disconnect(&mut self) -> BridgeResult<()>;

    /// Check if connected
    fn is_connected(&self) -> bool {
        self.state() == BridgeState::Connected
    }

    /// Send data to a dora output
    fn send(&self, output_id: &str, data: DoraData) -> BridgeResult<()>;

    /// Get list of input IDs this bridge expects
    fn expected_inputs(&self) -> Vec<String>;

    /// Get list of output IDs this bridge provides
    fn expected_outputs(&self) -> Vec<String>;
}
