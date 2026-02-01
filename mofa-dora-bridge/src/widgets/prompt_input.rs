//! Prompt input bridge
//!
//! Connects to dora as `mofa-prompt-input` dynamic node.
//! Sends user prompts to LLM nodes and receives:
//! - Text responses (streaming)
//! - Status updates

use crate::bridge::{BridgeState, DoraBridge};
use crate::data::{ChatMessage, ControlCommand, DoraData, EventMetadata, MessageRole};
use crate::error::{BridgeError, BridgeResult};
use crate::shared_state::SharedDoraState;
use arrow::array::Array;
use crossbeam_channel::{bounded, Receiver, Sender};
use dora_node_api::{
    dora_core::config::{DataId, NodeId},
    DoraNode, Event, IntoArrow, Parameter,
};
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread;
use tracing::{error, info, warn};

/// Prompt input bridge - sends prompts to dora, receives responses
///
/// Status updates (connected/disconnected/error) are communicated via SharedDoraState.
/// Chat messages are pushed directly to SharedDoraState.chat for UI consumption.
pub struct PromptInputBridge {
    /// Node ID (e.g., "mofa-prompt-input")
    node_id: String,
    /// Current state
    state: Arc<RwLock<BridgeState>>,
    /// Shared state for direct UI communication
    shared_state: Option<Arc<SharedDoraState>>,
    /// Prompt sender from widget
    prompt_sender: Sender<String>,
    /// Prompt receiver for dora
    prompt_receiver: Receiver<String>,
    /// Control command sender from widget
    control_sender: Sender<ControlCommand>,
    /// Control command receiver for dora
    control_receiver: Receiver<ControlCommand>,
    /// Stop signal
    stop_sender: Option<Sender<()>>,
    /// Worker thread handle
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl PromptInputBridge {
    /// Create a new prompt input bridge (legacy - without shared state)
    pub fn new(node_id: &str) -> Self {
        Self::with_shared_state(node_id, None)
    }

    /// Create a new prompt input bridge with shared state
    pub fn with_shared_state(node_id: &str, shared_state: Option<Arc<SharedDoraState>>) -> Self {
        let (prompt_tx, prompt_rx) = bounded(10);
        let (control_tx, control_rx) = bounded(10);

        Self {
            node_id: node_id.to_string(),
            state: Arc::new(RwLock::new(BridgeState::Disconnected)),
            shared_state,
            prompt_sender: prompt_tx,
            prompt_receiver: prompt_rx,
            control_sender: control_tx,
            control_receiver: control_rx,
            stop_sender: None,
            worker_handle: None,
        }
    }

    /// Send a prompt to dora (widget calls this)
    pub fn send_prompt(&self, prompt: impl Into<String>) -> BridgeResult<()> {
        self.prompt_sender
            .send(prompt.into())
            .map_err(|_| BridgeError::ChannelSendError)
    }

    /// Send a control command to dora (widget calls this)
    pub fn send_control(&self, command: ControlCommand) -> BridgeResult<()> {
        self.control_sender
            .send(command)
            .map_err(|_| BridgeError::ChannelSendError)
    }

    /// Run the dora event loop in background thread
    fn run_event_loop(
        node_id: String,
        state: Arc<RwLock<BridgeState>>,
        shared_state: Option<Arc<SharedDoraState>>,
        prompt_receiver: Receiver<String>,
        control_receiver: Receiver<ControlCommand>,
        stop_receiver: Receiver<()>,
    ) {
        info!("Starting prompt input bridge event loop for {}", node_id);

        // Initialize dora node
        let (mut node, mut events) =
            match DoraNode::init_from_node_id(NodeId::from(node_id.clone())) {
                Ok(n) => n,
                Err(e) => {
                    error!("Failed to init dora node {}: {}", node_id, e);
                    *state.write() = BridgeState::Error;
                    if let Some(ref ss) = shared_state {
                        ss.set_error(Some(format!("Init failed: {}", e)));
                    }
                    return;
                }
            };

        *state.write() = BridgeState::Connected;
        if let Some(ref ss) = shared_state {
            ss.add_bridge(node_id.clone());
        }

        // Event loop
        loop {
            // Check for stop signal
            if stop_receiver.try_recv().is_ok() {
                info!("Prompt input bridge received stop signal");
                break;
            }

            // Check for prompts to send
            while let Ok(prompt) = prompt_receiver.try_recv() {
                if let Err(e) = Self::send_prompt_to_dora(&mut node, &prompt) {
                    warn!("Failed to send prompt: {}", e);
                }
            }

            // Check for control commands to send
            while let Ok(cmd) = control_receiver.try_recv() {
                if let Err(e) = Self::send_control_to_dora(&mut node, &cmd) {
                    warn!("Failed to send control: {}", e);
                }
            }

            // Receive dora events with timeout
            match events.recv_timeout(std::time::Duration::from_millis(100)) {
                Some(event) => {
                    Self::handle_dora_event(event, shared_state.as_ref());
                }
                None => {
                    // Timeout or no event, continue
                }
            }
        }

        *state.write() = BridgeState::Disconnected;
        if let Some(ref ss) = shared_state {
            ss.remove_bridge(&node_id);
        }
        info!("Prompt input bridge event loop ended");
    }

    /// Handle a dora event
    fn handle_dora_event(event: Event, shared_state: Option<&Arc<SharedDoraState>>) {
        match event {
            Event::Input { id, data, metadata } => {
                let input_id = id.as_str();

                // Extract metadata (handle all parameter types like conference-dashboard)
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

                // Handle text inputs (responses from LLM)
                if input_id.contains("text") || input_id.contains("response") {
                    if let Some(text) = Self::extract_string(&data) {
                        let sender = Self::extract_sender(input_id);
                        let session_id = event_meta
                            .get("question_id")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        let session_status = event_meta
                            .get("session_status")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "unknown".to_string());

                        // LLM sends "ended" (not "complete") when streaming finishes
                        let is_complete = session_status == "ended" || session_status == "complete";

                        let msg = ChatMessage {
                            content: text,
                            sender,
                            role: MessageRole::Assistant,
                            timestamp: crate::data::current_timestamp(),
                            is_streaming: !is_complete,
                            session_id: Some(session_id),
                        };

                        // Push chat message to SharedDoraState for UI consumption
                        // ChatState.push() handles streaming consolidation internally
                        if let Some(ss) = shared_state {
                            ss.chat.push(msg);
                        }
                    }
                }
            }
            Event::Stop(_) => {
                info!("Received stop event from dora");
            }
            _ => {}
        }
    }

    /// Extract sender from input ID (e.g., "student1_text" -> "Student 1")
    fn extract_sender(input_id: &str) -> String {
        if input_id.contains("student1") || input_id.contains("llm1") {
            "Student 1".to_string()
        } else if input_id.contains("student2") || input_id.contains("llm2") {
            "Student 2".to_string()
        } else if input_id.contains("tutor") || input_id.contains("judge") {
            "Tutor".to_string()
        } else {
            "Assistant".to_string()
        }
    }

    /// Extract string from arrow data
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
                let array = data.0.as_any().downcast_ref::<arrow::array::UInt8Array>()?;
                let bytes: Vec<u8> = array.values().to_vec();
                return String::from_utf8(bytes).ok();
            }
            _ => {
                warn!("Unsupported text data type: {:?}", data.0.data_type());
            }
        }
        None
    }

    /// Send prompt to dora via control output
    /// The conference-controller expects JSON with "prompt" field
    fn send_prompt_to_dora(node: &mut DoraNode, prompt: &str) -> BridgeResult<()> {
        // Create JSON payload that conference-controller expects
        let payload = serde_json::json!({
            "prompt": prompt
        });

        info!("Sending prompt to dora: {}", prompt);
        let data = payload.to_string().into_arrow();
        let output_id: DataId = "control".to_string().into(); // Use control output
        node.send_output(output_id, Default::default(), data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    /// Send control command to dora
    fn send_control_to_dora(node: &mut DoraNode, cmd: &ControlCommand) -> BridgeResult<()> {
        let payload =
            serde_json::to_string(cmd).map_err(|e| BridgeError::SendFailed(e.to_string()))?;

        let data = payload.into_arrow();
        let output_id: DataId = "control".to_string().into();
        node.send_output(output_id, Default::default(), data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }
}

impl DoraBridge for PromptInputBridge {
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
        let prompt_receiver = self.prompt_receiver.clone();
        let control_receiver = self.control_receiver.clone();

        let handle = thread::spawn(move || {
            Self::run_event_loop(
                node_id,
                state,
                shared_state,
                prompt_receiver,
                control_receiver,
                stop_rx,
            );
        });

        self.worker_handle = Some(handle);

        // Wait briefly for connection
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

    fn send(&self, output_id: &str, data: DoraData) -> BridgeResult<()> {
        if !self.is_connected() {
            return Err(BridgeError::NotConnected);
        }

        match (output_id, data) {
            // Prompts are sent via the prompt channel, which sends to "control" output as JSON
            ("prompt", DoraData::Text(text)) | ("control", DoraData::Text(text)) => {
                info!("Queuing prompt for sending: {}", text);
                self.prompt_sender
                    .send(text)
                    .map_err(|_| BridgeError::ChannelSendError)?;
            }
            ("control", DoraData::Control(cmd)) => {
                self.control_sender
                    .send(cmd)
                    .map_err(|_| BridgeError::ChannelSendError)?;
            }
            // Audio data needs direct sending via node (not via channel)
            // For now, we reject it with a clear error
            ("audio", DoraData::Audio(_)) => {
                error!("PromptInputBridge cannot send audio data - audio output not supported by this bridge type");
                return Err(BridgeError::NotSupported(
                    "PromptInputBridge does not support audio output. Use a dedicated audio input bridge instead.".to_string()
                ));
            }
            (output, data_type) => {
                let data_type_name = match &data_type {
                    DoraData::Text(_) => "Text",
                    DoraData::Audio(_) => "Audio",
                    DoraData::Control(_) => "Control",
                    DoraData::Json(_) => "Json",
                    DoraData::Binary(_) => "Binary",
                    DoraData::Log(_) => "Log",
                    DoraData::Chat(_) => "Chat",
                    DoraData::Empty => "Empty",
                };
                warn!(
                    "Unknown output '{}' with data type {}",
                    output, data_type_name
                );
                return Err(BridgeError::NotSupported(
                    format!("Output '{}' not supported by PromptInputBridge", output)
                ));
            }
        }

        Ok(())
    }

    fn expected_inputs(&self) -> Vec<String> {
        vec![
            "text".to_string(),
            "student1_text".to_string(),
            "student2_text".to_string(),
            "tutor_text".to_string(),
        ]
    }

    fn expected_outputs(&self) -> Vec<String> {
        vec!["control".to_string()] // Prompts are sent via control output as JSON
    }
}

impl Drop for PromptInputBridge {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}
