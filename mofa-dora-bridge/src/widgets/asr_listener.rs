//! ASR Listener Bridge
//!
//! Listens to ASR node transcription output and writes it to SharedDoraState

use crate::bridge::{BridgeState, DoraBridge};
use crate::data::DoraData;
use crate::error::{BridgeError, BridgeResult};
use crate::shared_state::SharedDoraState;
use crossbeam_channel::{bounded, Receiver, Sender};
use dora_node_api::{
    dora_core::config::{DataId, NodeId},
    DoraNode, Event,
};
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread;
use tracing::{debug, error, info, warn};

/// ASR Listener Bridge - monitors ASR node transcription output
pub struct AsrListenerBridge {
    /// Node ID in the dataflow
    node_id: String,
    /// Connection state
    state: Arc<RwLock<BridgeState>>,
    /// Shared state for writing transcription results
    shared_state: Option<Arc<SharedDoraState>>,
    /// Stop signal
    stop_sender: Option<Sender<()>>,
    /// Worker thread handle
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl AsrListenerBridge {
    /// Create a new ASR listener bridge
    pub fn new(node_id: &str) -> Self {
        Self::with_shared_state(node_id, None)
    }

    /// Create with shared state
    pub fn with_shared_state(node_id: &str, shared_state: Option<Arc<SharedDoraState>>) -> Self {
        Self {
            node_id: node_id.to_string(),
            state: Arc::new(RwLock::new(BridgeState::Disconnected)),
            shared_state,
            stop_sender: None,
            worker_handle: None,
        }
    }

    /// Worker thread that listens to ASR transcription output
    fn run_event_loop(
        node_id: String,
        state: Arc<RwLock<BridgeState>>,
        shared_state: Option<Arc<SharedDoraState>>,
        stop_receiver: Receiver<()>,
    ) {
        info!("[AsrListener] Worker started for node: {}", node_id);

        // Connect to dora as dynamic node
        let (mut node, mut events) =
            match DoraNode::init_from_node_id(NodeId::from(node_id.clone())) {
                Ok(n) => n,
                Err(e) => {
                    error!("[AsrListener] Failed to init dora node: {}", e);
                    *state.write() = BridgeState::Error;
                    return;
                }
            };

        info!("[AsrListener] Connected to dora as: {}", node_id);
        *state.write() = BridgeState::Connected;

        // Add to active bridges
        if let Some(ref shared) = shared_state {
            shared.add_bridge(node_id.clone());
        }

        // Main event loop
        loop {
            // Check for stop signal
            if stop_receiver.try_recv().is_ok() {
                info!("[AsrListener] Received stop signal");
                break;
            }

            // Poll for dora events
            if let Some(event) = events.recv_timeout(std::time::Duration::from_millis(100)) {
                match event {
                    Event::Input { id, metadata, data } => {
                        // Check if this is a transcription output from ASR node
                        if id == DataId::from("transcription".to_owned()) {
                            debug!("[AsrListener] Received transcription event");

                            // Parse the data as text - use try_as_str for ArrowData
                            use arrow::array::Array;
                            if let Some(text_str) =
                                data.as_any().downcast_ref::<arrow::array::StringArray>()
                            {
                                if text_str.len() > 0 {
                                    if let Some(text) = text_str.value(0).to_string().into() {
                                        // Try to parse as JSON (ASR might send JSON)
                                        let (language, transcription) = if let Ok(json) =
                                            serde_json::from_str::<serde_json::Value>(&text)
                                        {
                                            let lang = json
                                                .get("language")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("auto")
                                                .to_string();
                                            let txt = json
                                                .get("text")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("")
                                                .to_string();
                                            (lang, txt)
                                        } else {
                                            // If not JSON, treat as plain text
                                            ("auto".to_string(), text)
                                        };

                                        info!(
                                            "[AsrListener] Transcription: language={}, text={}",
                                            language, transcription
                                        );

                                        // Write to shared state
                                        if let Some(ref state) = shared_state {
                                            state
                                                .asr_transcription
                                                .set(Some((language, transcription)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Event::Stop(_) => {
                        info!("[AsrListener] Received stop event from dora");
                        break;
                    }
                    Event::InputClosed { id } => {
                        debug!("[AsrListener] Input closed: {:?}", id);
                    }
                    Event::Error(e) => {
                        error!("[AsrListener] Dora error: {}", e);
                    }
                    _ => {}
                }
            }
        }

        info!("[AsrListener] Worker stopped");
        *state.write() = BridgeState::Disconnected;

        // Remove from active bridges
        if let Some(ref shared) = shared_state {
            shared.remove_bridge(&node_id);
        }
    }
}

impl DoraBridge for AsrListenerBridge {
    fn node_id(&self) -> &str {
        &self.node_id
    }

    fn state(&self) -> BridgeState {
        *self.state.read()
    }

    fn connect(&mut self) -> BridgeResult<()> {
        if self.is_connected() {
            return Ok(());
        }

        *self.state.write() = BridgeState::Connecting;

        let (stop_tx, stop_rx) = bounded(1);
        self.stop_sender = Some(stop_tx);

        let node_id = self.node_id.clone();
        let state = Arc::clone(&self.state);
        let shared_state = self.shared_state.clone();

        let handle = thread::Builder::new()
            .name(format!("asr-listener-{}", node_id))
            .spawn(move || {
                Self::run_event_loop(node_id, state, shared_state, stop_rx);
            })
            .map_err(|e| BridgeError::ThreadSpawnFailed(e.to_string()))?;

        self.worker_handle = Some(handle);

        // Wait for connection to succeed or fail
        for _ in 0..50 {
            // Wait up to 5 seconds
            std::thread::sleep(std::time::Duration::from_millis(100));
            match *self.state.read() {
                BridgeState::Connected => {
                    info!(
                        "[AsrListener] Bridge connected successfully: {}",
                        self.node_id
                    );
                    return Ok(());
                }
                BridgeState::Error => {
                    error!("[AsrListener] Bridge failed to connect: {}", self.node_id);
                    return Err(BridgeError::ConnectionFailed(format!(
                        "AsrListener {} failed to init dora node",
                        self.node_id
                    )));
                }
                _ => {}
            }
        }

        warn!(
            "[AsrListener] Bridge connection timeout for: {}",
            self.node_id
        );
        Err(BridgeError::ConnectionFailed(format!(
            "AsrListener {} connection timeout",
            self.node_id
        )))
    }

    fn disconnect(&mut self) -> BridgeResult<()> {
        if let Some(stop_tx) = self.stop_sender.take() {
            let _ = stop_tx.send(());
        }

        if let Some(handle) = self.worker_handle.take() {
            handle.join().map_err(|_| BridgeError::ThreadJoinFailed)?;
        }

        *self.state.write() = BridgeState::Disconnected;
        Ok(())
    }

    fn send(&self, _output: &str, _data: DoraData) -> BridgeResult<()> {
        // This bridge only listens, doesn't send
        Err(BridgeError::NotSupported(
            "AsrListenerBridge does not support sending data".to_string(),
        ))
    }

    fn expected_inputs(&self) -> Vec<String> {
        vec!["transcription".to_string()]
    }

    fn expected_outputs(&self) -> Vec<String> {
        vec![]
    }
}
