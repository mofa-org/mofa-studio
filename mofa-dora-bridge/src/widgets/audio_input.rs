//! Audio Input Bridge
//!
//! Connects to dora as `mofa-audio-input` dynamic node.
//! Sends pre-recorded audio data (e.g., from file uploads or voice recording)
//! to the dora dataflow for processing by ASR or other audio nodes.
//!
//! This differs from AecInputBridge which captures live microphone audio with VAD.

use crate::bridge::{BridgeState, DoraBridge};
use crate::data::DoraData;
use crate::error::{BridgeError, BridgeResult};
use crate::shared_state::SharedDoraState;
use dora_node_api::{
    dora_core::config::{DataId, NodeId},
    DoraNode, IntoArrow,
};
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tracing::{error, info, warn};

/// Audio Input Bridge
///
/// Simple bridge for sending audio data directly to dora without VAD or segmentation.
/// Used for:
/// - Voice cloning: Send recorded audio to ASR for transcription
/// - Audio file uploads: Process pre-recorded audio
pub struct AudioInputBridge {
    /// Bridge identifier (node_id)
    id: String,
    /// Connection state
    state: Arc<RwLock<BridgeState>>,
    /// Dora node (stored after connection)
    node: Arc<RwLock<Option<DoraNode>>>,
    /// Shared state for UI communication
    shared_state: Arc<SharedDoraState>,
    /// Running flag
    running: Arc<AtomicBool>,
}

impl AudioInputBridge {
    /// Create a new audio input bridge
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            state: Arc::new(RwLock::new(BridgeState::Disconnected)),
            node: Arc::new(RwLock::new(None)),
            shared_state: SharedDoraState::new(),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a new audio input bridge with shared state
    pub fn with_shared_state(id: &str, shared_state: Arc<SharedDoraState>) -> Self {
        Self {
            id: id.to_string(),
            state: Arc::new(RwLock::new(BridgeState::Disconnected)),
            node: Arc::new(RwLock::new(None)),
            shared_state,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl DoraBridge for AudioInputBridge {
    fn node_id(&self) -> &str {
        &self.id
    }

    fn state(&self) -> BridgeState {
        *self.state.read()
    }

    fn is_connected(&self) -> bool {
        matches!(*self.state.read(), BridgeState::Connected)
    }

    fn connect(&mut self) -> BridgeResult<()> {
        if self.is_connected() {
            return Err(BridgeError::AlreadyConnected);
        }

        let state = self.state.clone();
        let node_arc = self.node.clone();
        let id = self.id.clone();
        let running = self.running.clone();

        // Set to connecting
        *state.write() = BridgeState::Connecting;

        // Spawn worker thread
        thread::Builder::new()
            .name(format!("audio-input-{}", id))
            .spawn(move || {
                info!("[AudioInputBridge] Initializing for node_id: {}", id);

                // Initialize dora node - returns (DoraNode, EventStream)
                let (node, _events) = match DoraNode::init_from_node_id(NodeId::from(id.clone())) {
                    Ok(n) => {
                        info!("[AudioInputBridge] Successfully connected to Dora");
                        n
                    }
                    Err(e) => {
                        error!("[AudioInputBridge] Failed to connect: {}", e);
                        *state.write() = BridgeState::Error;
                        return;
                    }
                };

                // Store the node
                *node_arc.write() = Some(node);
                *state.write() = BridgeState::Connected;
                running.store(true, Ordering::SeqCst);

                info!("[AudioInputBridge] Worker thread started - staying alive for send operations");

                // No event loop needed - audio sending happens synchronously via send() method
                // Just keep thread alive until disconnect() is called
                while running.load(Ordering::SeqCst) {
                    thread::sleep(std::time::Duration::from_millis(100));
                }

                info!("[AudioInputBridge] Worker thread exiting");
                *state.write() = BridgeState::Disconnected;
            })
            .map_err(|e| BridgeError::ThreadSpawnFailed(e.to_string()))?;

        // Wait for connection to establish (max 5 seconds)
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_secs(5) {
            match *self.state.read() {
                BridgeState::Connected => {
                    info!("[AudioInputBridge] Connection verified");
                    return Ok(());
                }
                BridgeState::Error => {
                    error!("[AudioInputBridge] Connection failed");
                    return Err(BridgeError::ConnectionFailed(
                        "Failed to initialize Dora node".to_string(),
                    ));
                }
                _ => {
                    thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }

        Err(BridgeError::Timeout(
            "Connection timeout after 5s".to_string(),
        ))
    }

    fn disconnect(&mut self) -> BridgeResult<()> {
        if !self.is_connected() {
            return Ok(());
        }

        info!("[AudioInputBridge] Disconnecting...");
        self.running.store(false, Ordering::SeqCst);

        // Wait for worker thread to stop
        thread::sleep(std::time::Duration::from_millis(200));

        *self.node.write() = None;
        *self.state.write() = BridgeState::Disconnected;

        Ok(())
    }

    fn send(&self, output_id: &str, data: DoraData) -> BridgeResult<()> {
        if !self.is_connected() {
            return Err(BridgeError::NotConnected);
        }

        match (output_id, data) {
            ("audio", DoraData::Audio(audio_data)) => {
                // Get node and send audio directly
                let mut node_guard = self.node.write();
                if let Some(ref mut node) = *node_guard {
                    info!(
                        "[AudioInputBridge] Sending audio: {} samples at {}Hz",
                        audio_data.samples.len(),
                        audio_data.sample_rate
                    );

                    // Convert f32 samples to Arrow format
                    let data = audio_data.samples.into_arrow();
                    let output_id: DataId = "audio".to_string().into();
                    node.send_output(output_id, BTreeMap::new(), data)
                        .map_err(|e| BridgeError::SendFailed(e.to_string()))?;

                    info!("[AudioInputBridge] Audio sent successfully");
                } else {
                    return Err(BridgeError::NotConnected);
                }
            }
            (output, data_type) => {
                warn!(
                    "[AudioInputBridge] Unknown output '{}' with data type {:?}",
                    output,
                    std::any::type_name_of_val(&data_type)
                );
                return Err(BridgeError::NotSupported(format!(
                    "Output '{}' not supported by AudioInputBridge",
                    output
                )));
            }
        }

        Ok(())
    }

    fn expected_inputs(&self) -> Vec<String> {
        vec![] // No inputs - this bridge only sends data
    }

    fn expected_outputs(&self) -> Vec<String> {
        vec!["audio".to_string()]
    }
}

impl Drop for AudioInputBridge {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}
