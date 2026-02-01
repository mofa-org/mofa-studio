//! # MoFA Dora Bridge
//!
//! Communication layer between the MoFA Studio UI and the Dora dataflow runtime.
//! Provides thread-safe shared state, data types, and bridge infrastructure for
//! real-time voice chat applications.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                         DORA DATAFLOW (Worker Threads)                       │
//! ├─────────────────────┬─────────────────────┬─────────────────────────────────┤
//! │  PromptInputBridge  │  AudioPlayerBridge  │  SystemLogBridge                │
//! │                     │                     │                                 │
//! │  state.chat.push()  │  state.audio.push() │  state.logs.push()              │
//! └─────────┬───────────┴──────────┬──────────┴───────────────┬─────────────────┘
//!           │         Direct write (no channels)              │
//!           ▼                      ▼                          ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                     SharedDoraState (Arc<...>)                              │
//! │                                                                             │
//! │  chat: ChatState        audio: AudioState       logs: DirtyVec<LogEntry>   │
//! │  status: DirtyValue<DoraStatus>                                            │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!           │          Read on UI timer (single poll)         │
//!           ▼                      ▼                          ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                        MoFA Studio UI (Main Thread)                         │
//! │  poll_dora_state() - reads dirty data, updates widgets                      │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Components
//!
//! ### Shared State ([`SharedDoraState`])
//!
//! Thread-safe state container with dirty tracking for efficient UI updates:
//!
//! - [`ChatState`] - Chat messages with streaming consolidation
//! - [`AudioState`] - Ring buffer for audio chunks (consumed by audio player)
//! - [`DirtyVec`] - Generic dirty-trackable collection
//! - [`DirtyValue`] - Generic dirty-trackable single value
//!
//! ### Data Types ([`data`] module)
//!
//! - [`AudioData`] - Audio samples with metadata (participant_id, question_id)
//! - [`ChatMessage`] - Chat message with sender, role, streaming status
//! - [`LogEntry`] - Log entry with level, node_id, timestamp
//! - [`ControlCommand`] - Dataflow control commands (start, stop, reset)
//!
//! ### Bridge Infrastructure
//!
//! - [`DoraBridge`] trait - Interface for widget bridges
//! - [`BridgeState`] - Connection state (Disconnected, Connecting, Connected, Error)
//! - [`MofaNodeType`] - Enum of known widget node types
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use mofa_dora_bridge::{SharedDoraState, ChatMessage, AudioData};
//!
//! // Create shared state (typically done once at app startup)
//! let state = SharedDoraState::new();
//!
//! // === PRODUCER (Dora bridge thread) ===
//!
//! // Push chat message (with automatic streaming consolidation)
//! state.chat.push(ChatMessage {
//!     content: "Hello".into(),
//!     sender: "Tutor".into(),
//!     is_streaming: true,
//!     session_id: Some("session_1".into()),
//!     ..Default::default()
//! });
//!
//! // Push audio chunk
//! state.audio.push(AudioData {
//!     samples: vec![0.1, 0.2, 0.3],
//!     sample_rate: 32000,
//!     channels: 1,
//!     participant_id: Some("tutor".into()),
//!     question_id: Some("q1".into()),
//! });
//!
//! // === CONSUMER (UI thread on timer) ===
//!
//! // Read chat only if changed (dirty tracking)
//! if let Some(messages) = state.chat.read_if_dirty() {
//!     update_chat_ui(messages);
//! }
//!
//! // Drain audio chunks for playback
//! let chunks = state.audio.drain();
//! for chunk in chunks {
//!     audio_player.write_samples(&chunk.samples);
//! }
//! ```
//!
//! ## Design Principles
//!
//! 1. **No Channels for Data** - Direct shared memory with dirty tracking
//! 2. **Single Poll Point** - UI reads all state on one timer (no multiple poll loops)
//! 3. **Streaming Consolidation** - ChatState automatically accumulates streaming chunks
//! 4. **Lock-Free Reads** - AtomicBool for dirty flags, RwLock for data
//! 5. **Bounded Collections** - All collections have max sizes to prevent memory growth

pub mod bridge;
pub mod controller;
pub mod data;
pub mod dispatcher;
pub mod error;
pub mod parser;
pub mod shared_state;

// Widget-specific bridges
pub mod widgets;

// Re-exports
pub use bridge::{BridgeState, DoraBridge};
pub use controller::{DataflowController, DataflowState};
pub use data::{AudioData, ChatMessage, ControlCommand, DoraData, LogEntry};
pub use dispatcher::{DynamicNodeDispatcher, WidgetBinding};
pub use error::{BridgeError, BridgeResult};
pub use shared_state::{SharedDoraState, DoraStatus, ChatState, AudioState, DirtyVec, DirtyValue, MicState};
pub use widgets::AecControlCommand;
pub use parser::{DataflowParser, EnvRequirement, LogSource, ParsedDataflow, ParsedNode};

/// Prefix for MoFA built-in dynamic nodes in dataflow YAML
pub const MOFA_NODE_PREFIX: &str = "mofa-";

/// Known MoFA widget node types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MofaNodeType {
    /// Audio player widget - receives audio, plays through speaker
    AudioPlayer,
    /// System log widget - receives logs from multiple nodes
    SystemLog,
    /// Prompt input widget - sends user prompts to LLM
    PromptInput,
    /// Mic input widget - captures audio from microphone
    MicInput,
    /// Chat viewer widget - displays conversation
    ChatViewer,
    /// Participant panel widget - receives audio and calculates levels for visualization
    ParticipantPanel,
    /// ASR listener widget - receives transcription from ASR node
    AsrListener,
    /// Audio input widget - sends audio to ASR (for voice cloning)
    AudioInput,
}

impl MofaNodeType {
    /// Get the node ID for this widget type
    pub fn node_id(&self) -> &'static str {
        match self {
            MofaNodeType::AudioPlayer => "mofa-audio-player",
            MofaNodeType::SystemLog => "mofa-system-log",
            MofaNodeType::PromptInput => "mofa-prompt-input",
            MofaNodeType::MicInput => "mofa-mic-input",
            MofaNodeType::ChatViewer => "mofa-chat-viewer",
            MofaNodeType::ParticipantPanel => "mofa-participant-panel",
            MofaNodeType::AsrListener => "mofa-asr-listener",
            MofaNodeType::AudioInput => "mofa-audio-input",
        }
    }

    /// Parse node type from node ID
    pub fn from_node_id(node_id: &str) -> Option<Self> {
        match node_id {
            "mofa-audio-player" => Some(MofaNodeType::AudioPlayer),
            "mofa-system-log" => Some(MofaNodeType::SystemLog),
            "mofa-prompt-input" => Some(MofaNodeType::PromptInput),
            "mofa-mic-input" => Some(MofaNodeType::MicInput),
            "mofa-chat-viewer" => Some(MofaNodeType::ChatViewer),
            "mofa-participant-panel" => Some(MofaNodeType::ParticipantPanel),
            "mofa-asr-listener" => Some(MofaNodeType::AsrListener),
            "mofa-audio-input" => Some(MofaNodeType::AudioInput),
            _ => None,
        }
    }

    /// Check if a node ID is a MoFA widget node
    pub fn is_mofa_node(node_id: &str) -> bool {
        node_id.starts_with(MOFA_NODE_PREFIX)
    }
}
