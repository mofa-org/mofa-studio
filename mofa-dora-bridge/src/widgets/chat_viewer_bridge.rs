//! Chat Viewer Bridge
//!
//! Connects to the Dora dataflow as `mofa-chat-viewer` dynamic node.
//! Receives text or JSON chat messages from upstream nodes (ASR, LLM, etc.)
//! and routes them into `SharedDoraState.chat` for display in chat widgets.
//!
//! # Why This Bridge Exists
//!
//! `ChatViewerBridge` was the missing link between the Dora dataflow and the
//! chat display UI. Without it, any `mofa-chat-viewer` node entry in a
//! dataflow YAML was silently skipped (see `dispatcher.rs`), meaning chat
//! output from ASR/LLM nodes never reached the screen.
//!
//! # Supported Input Formats
//!
//! | Format | Example |
//! |--------|---------|
//! | Plain text | `"Hello, world!"` |
//! | JSON (full) | `{"content":"Hi","sender":"Bot","role":"assistant","is_streaming":false}` |
//! | JSON (partial) | `{"content":"streaming chunk...","is_streaming":true,"session_id":"s1"}` |
//!
//! Streaming consolidation is handled automatically by `ChatState` — multiple
//! chunks from the same sender/session are accumulated into a single message.
//!
//! # Data Flow
//!
//! ```text
//! Dora Node (ASR/LLM output)
//!      │
//!      │  Utf8 text or JSON
//!      ▼
//! mofa-chat-viewer   ← this bridge
//!      │
//!      │  ChatMessage (parsed)
//!      ▼
//! SharedDoraState.chat
//!      │
//!      │  read_if_dirty() on UI timer
//!      ▼
//! ChatPanel / ChatBubblePanel (widget)
//! ```

use crate::bridge::{BridgeState, DoraBridge};
use crate::data::{current_timestamp, ChatMessage, DoraData, EventMetadata, MessageRole};
use crate::error::{BridgeError, BridgeResult};
use crate::shared_state::SharedDoraState;
use arrow::array::Array;
use crossbeam_channel::{bounded, Receiver, Sender};
use dora_node_api::{dora_core::config::NodeId, DoraNode, Event, Parameter};
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread;
use tracing::{debug, error, info, warn};

/// Chat viewer bridge — receives chat messages from Dora and surfaces them in the UI.
///
/// Status updates go through `SharedDoraState.status`.
/// Messages are pushed to `SharedDoraState.chat` for the UI to poll.
pub struct ChatViewerBridge {
    /// Node ID in the dataflow (e.g., `"mofa-chat-viewer"`)
    node_id: String,
    /// Current bridge state
    state: Arc<RwLock<BridgeState>>,
    /// Shared state for Dora ↔ UI communication
    shared_state: Option<Arc<SharedDoraState>>,
    /// Stop signal sender
    stop_sender: Option<Sender<()>>,
    /// Worker thread handle
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl ChatViewerBridge {
    /// Create without shared state (legacy / test use)
    pub fn new(node_id: &str) -> Self {
        Self::with_shared_state(node_id, None)
    }

    /// Create with shared state for UI communication
    pub fn with_shared_state(node_id: &str, shared_state: Option<Arc<SharedDoraState>>) -> Self {
        Self {
            node_id: node_id.to_string(),
            state: Arc::new(RwLock::new(BridgeState::Disconnected)),
            shared_state,
            stop_sender: None,
            worker_handle: None,
        }
    }

    /// Run the Dora event loop in a background thread.
    fn run_event_loop(
        node_id: String,
        state: Arc<RwLock<BridgeState>>,
        shared_state: Option<Arc<SharedDoraState>>,
        stop_receiver: Receiver<()>,
    ) {
        info!("Starting chat viewer bridge event loop for {}", node_id);

        let (_node, mut events) =
            match DoraNode::init_from_node_id(NodeId::from(node_id.clone())) {
                Ok(n) => n,
                Err(e) => {
                    error!("Failed to init dora node {}: {}", node_id, e);
                    *state.write() = BridgeState::Error;
                    if let Some(ref ss) = shared_state {
                        ss.set_error(Some(format!("ChatViewer init failed: {}", e)));
                    }
                    return;
                }
            };

        *state.write() = BridgeState::Connected;
        if let Some(ref ss) = shared_state {
            ss.add_bridge(node_id.clone());
        }

        loop {
            if stop_receiver.try_recv().is_ok() {
                info!("Chat viewer bridge received stop signal");
                break;
            }

            match events.recv_timeout(std::time::Duration::from_millis(100)) {
                Some(event) => {
                    Self::handle_dora_event(event, shared_state.as_ref(), &node_id);
                }
                None => {
                    // Timeout — normal, keep looping
                }
            }
        }

        *state.write() = BridgeState::Disconnected;
        if let Some(ref ss) = shared_state {
            ss.remove_bridge(&node_id);
        }
        info!("Chat viewer bridge event loop ended");
    }

    /// Handle a single Dora event.
    fn handle_dora_event(
        event: Event,
        shared_state: Option<&Arc<SharedDoraState>>,
        node_id: &str,
    ) {
        match event {
            Event::Input { id, data, metadata } => {
                let input_id = id.as_str();

                // Collect metadata parameters into EventMetadata
                let mut event_meta = EventMetadata::default();
                for (key, value) in metadata.parameters.iter() {
                    let string_value = match value {
                        Parameter::String(s) => s.clone(),
                        Parameter::Integer(i) => i.to_string(),
                        Parameter::Float(f) => f.to_string(),
                        Parameter::Bool(b) => b.to_string(),
                        Parameter::ListInt(l) => format!("{:?}", l),
                        Parameter::ListFloat(l) => format!("{:?}", l),
                        Parameter::ListString(l) => format!("{:?}", l),
                    };
                    event_meta.values.insert(key.clone(), string_value);
                }

                if let Some(text) = Self::extract_string(&data) {
                    let msg = Self::parse_message(&text, input_id, &event_meta);
                    debug!(
                        "ChatViewer [{}] {:?} from {}: {} chars",
                        node_id,
                        msg.role,
                        msg.sender,
                        msg.content.len()
                    );
                    if let Some(ss) = shared_state {
                        ss.chat.push(msg);
                    }
                }
            }
            Event::Stop(_) => {
                info!("Chat viewer bridge: received stop from dora");
            }
            _ => {}
        }
    }

    /// Parse a text payload into a `ChatMessage`.
    ///
    /// Tries JSON first (full or partial `ChatMessage` schema), falls back to
    /// treating the whole string as plain-text assistant content.
    fn parse_message(text: &str, source: &str, metadata: &EventMetadata) -> ChatMessage {
        // Attempt JSON parse
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            let content = json
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or(text)
                .to_string();

            let sender = json
                .get("sender")
                .and_then(|v| v.as_str())
                .or_else(|| metadata.participant_id())
                .unwrap_or(source)
                .to_string();

            let role = match json
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("assistant")
            {
                "user" => MessageRole::User,
                "system" => MessageRole::System,
                _ => MessageRole::Assistant,
            };

            let is_streaming = json
                .get("is_streaming")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let session_id = json
                .get("session_id")
                .and_then(|v| v.as_str())
                .or_else(|| metadata.get("session_id"))
                .map(|s| s.to_string());

            return ChatMessage {
                content,
                sender,
                role,
                timestamp: current_timestamp(),
                is_streaming,
                session_id,
            };
        }

        // Plain text fallback — infer metadata from Dora event parameters
        let role = match metadata.get("role") {
            Some("user") => MessageRole::User,
            Some("system") => MessageRole::System,
            _ => MessageRole::Assistant,
        };

        let sender = metadata
            .participant_id()
            .unwrap_or(source)
            .to_string();

        let is_streaming = metadata
            .get("is_streaming")
            .map(|s| s == "true")
            .unwrap_or(false);

        let session_id = metadata.get("session_id").map(|s| s.to_string());

        ChatMessage {
            content: text.to_string(),
            sender,
            role,
            timestamp: current_timestamp(),
            is_streaming,
            session_id,
        }
    }

    /// Extract a UTF-8 string from Arrow data.
    ///
    /// Supports Utf8, LargeUtf8, and UInt8 (raw bytes) column types.
    fn extract_string(data: &dora_node_api::ArrowData) -> Option<String> {
        match data.0.data_type() {
            arrow::datatypes::DataType::Utf8 => {
                let array = data
                    .0
                    .as_any()
                    .downcast_ref::<arrow::array::StringArray>()?;
                if array.len() > 0 {
                    return Some(array.value(0).to_string());
                }
            }
            arrow::datatypes::DataType::LargeUtf8 => {
                let array = data
                    .0
                    .as_any()
                    .downcast_ref::<arrow::array::LargeStringArray>()?;
                if array.len() > 0 {
                    return Some(array.value(0).to_string());
                }
            }
            arrow::datatypes::DataType::UInt8 => {
                let array = data
                    .0
                    .as_any()
                    .downcast_ref::<arrow::array::UInt8Array>()?;
                let bytes: Vec<u8> = array.values().to_vec();
                return String::from_utf8(bytes).ok();
            }
            dt => {
                warn!("ChatViewer: unsupported Arrow data type: {:?}", dt);
            }
        }
        None
    }
}

impl DoraBridge for ChatViewerBridge {
    fn node_id(&self) -> &str {
        &self.node_id
    }

    fn state(&self) -> BridgeState {
        *self.state.read()
    }

    fn connect(&mut self) -> BridgeResult<()> {
        if self.is_connected() {
            return Err(BridgeError::AlreadyConnected);
        }

        *self.state.write() = BridgeState::Connecting;

        let (stop_tx, stop_rx) = bounded(1);
        self.stop_sender = Some(stop_tx);

        let node_id = self.node_id.clone();
        let state = Arc::clone(&self.state);
        let shared_state = self.shared_state.clone();

        let handle = thread::spawn(move || {
            Self::run_event_loop(node_id, state, shared_state, stop_rx);
        });

        self.worker_handle = Some(handle);

        // Brief pause to let the worker thread initialize
        std::thread::sleep(std::time::Duration::from_millis(200));

        Ok(())
    }

    fn disconnect(&mut self) -> BridgeResult<()> {
        if let Some(stop_tx) = self.stop_sender.take() {
            let _ = stop_tx.send(());
        }

        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }

        *self.state.write() = BridgeState::Disconnected;
        Ok(())
    }

    fn send(&self, _output_id: &str, _data: DoraData) -> BridgeResult<()> {
        // Chat viewer is receive-only; no outputs.
        Ok(())
    }

    fn expected_inputs(&self) -> Vec<String> {
        // Accept any of the common output IDs from ASR/LLM nodes
        vec![
            "chat".to_string(),
            "text".to_string(),
            "asr_output".to_string(),
            "llm_output".to_string(),
            "transcript".to_string(),
        ]
    }

    fn expected_outputs(&self) -> Vec<String> {
        vec![]
    }
}

impl Drop for ChatViewerBridge {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::MessageRole;

    #[test]
    fn test_parse_plain_text() {
        let meta = EventMetadata::default();
        let msg = ChatViewerBridge::parse_message("Hello world", "mofa-asr", &meta);
        assert_eq!(msg.content, "Hello world");
        assert_eq!(msg.sender, "mofa-asr");
        assert_eq!(msg.role, MessageRole::Assistant);
        assert!(!msg.is_streaming);
    }

    #[test]
    fn test_parse_json_full() {
        let meta = EventMetadata::default();
        let json = r#"{"content":"Hi there","sender":"Bot","role":"assistant","is_streaming":false}"#;
        let msg = ChatViewerBridge::parse_message(json, "node", &meta);
        assert_eq!(msg.content, "Hi there");
        assert_eq!(msg.sender, "Bot");
        assert_eq!(msg.role, MessageRole::Assistant);
        assert!(!msg.is_streaming);
    }

    #[test]
    fn test_parse_json_user_role() {
        let meta = EventMetadata::default();
        let json = r#"{"content":"What is Rust?","sender":"human","role":"user"}"#;
        let msg = ChatViewerBridge::parse_message(json, "node", &meta);
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.sender, "human");
    }

    #[test]
    fn test_parse_json_streaming() {
        let meta = EventMetadata::default();
        let json = r#"{"content":"chunk","sender":"LLM","role":"assistant","is_streaming":true,"session_id":"s42"}"#;
        let msg = ChatViewerBridge::parse_message(json, "node", &meta);
        assert!(msg.is_streaming);
        assert_eq!(msg.session_id, Some("s42".to_string()));
    }

    #[test]
    fn test_parse_metadata_role_override() {
        let mut meta = EventMetadata::default();
        meta.values.insert("role".to_string(), "user".to_string());
        let msg = ChatViewerBridge::parse_message("plain text", "input_node", &meta);
        assert_eq!(msg.role, MessageRole::User);
    }

    #[test]
    fn test_parse_system_role() {
        let meta = EventMetadata::default();
        let json = r#"{"content":"System notice","role":"system"}"#;
        let msg = ChatViewerBridge::parse_message(json, "node", &meta);
        assert_eq!(msg.role, MessageRole::System);
    }
}
