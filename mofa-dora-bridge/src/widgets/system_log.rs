//! System log bridge
//!
//! Connects to dora as `mofa-system-log` dynamic node.
//! Receives logs from multiple nodes and provides:
//! - Aggregated log entries to the widget
//! - Per-source filtering capability

use crate::bridge::{BridgeState, DoraBridge};
use crate::data::{current_timestamp, DoraData, EventMetadata, LogEntry, LogLevel};
use crate::error::{BridgeError, BridgeResult};
use crate::shared_state::SharedDoraState;
use arrow::array::Array;
use crossbeam_channel::{bounded, Receiver, Sender};
use dora_node_api::{dora_core::config::NodeId, DoraNode, Event, Parameter};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use tracing::{debug, error, info, warn};

/// System log bridge - receives logs from multiple dora nodes
///
/// Status updates (connected/disconnected/error) are communicated via SharedDoraState.
/// Log entries are pushed directly to SharedDoraState.logs for UI consumption.
pub struct SystemLogBridge {
    /// Node ID (e.g., "mofa-system-log")
    node_id: String,
    /// Current state
    state: Arc<RwLock<BridgeState>>,
    /// Shared state for direct UI communication
    shared_state: Option<Arc<SharedDoraState>>,
    /// Known log sources
    log_sources: Arc<RwLock<HashSet<String>>>,
    /// Minimum log level filter
    min_level: Arc<RwLock<LogLevel>>,
    /// Stop signal
    stop_sender: Option<Sender<()>>,
    /// Worker thread handle
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl SystemLogBridge {
    /// Create a new system log bridge (legacy - without shared state)
    pub fn new(node_id: &str) -> Self {
        Self::with_shared_state(node_id, None)
    }

    /// Create a new system log bridge with shared state
    pub fn with_shared_state(node_id: &str, shared_state: Option<Arc<SharedDoraState>>) -> Self {
        Self {
            node_id: node_id.to_string(),
            state: Arc::new(RwLock::new(BridgeState::Disconnected)),
            shared_state,
            log_sources: Arc::new(RwLock::new(HashSet::new())),
            min_level: Arc::new(RwLock::new(LogLevel::Info)),
            stop_sender: None,
            worker_handle: None,
        }
    }

    /// Set minimum log level filter
    pub fn set_min_level(&self, level: LogLevel) {
        *self.min_level.write() = level;
    }

    /// Get known log sources
    pub fn log_sources(&self) -> Vec<String> {
        self.log_sources.read().iter().cloned().collect()
    }

    /// Run the dora event loop in background thread
    fn run_event_loop(
        node_id: String,
        state: Arc<RwLock<BridgeState>>,
        shared_state: Option<Arc<SharedDoraState>>,
        log_sources: Arc<RwLock<HashSet<String>>>,
        min_level: Arc<RwLock<LogLevel>>,
        stop_receiver: Receiver<()>,
    ) {
        info!("Starting system log bridge event loop for {}", node_id);

        // Initialize dora node
        let (_node, mut events) = match DoraNode::init_from_node_id(NodeId::from(node_id.clone())) {
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
                info!("System log bridge received stop signal");
                break;
            }

            // Receive dora events with timeout
            match events.recv_timeout(std::time::Duration::from_millis(100)) {
                Some(event) => {
                    Self::handle_dora_event(event, shared_state.as_ref(), &log_sources, &min_level);
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
        info!("System log bridge event loop ended");
    }

    /// Handle a dora event
    fn handle_dora_event(
        event: Event,
        shared_state: Option<&Arc<SharedDoraState>>,
        log_sources: &Arc<RwLock<HashSet<String>>>,
        min_level: &Arc<RwLock<LogLevel>>,
    ) {
        match event {
            Event::Input { id, data, metadata } => {
                let input_id = id.as_str();

                // Extract source node from input ID (e.g., "tts_log" -> "tts")
                let source_node = input_id
                    .strip_suffix("_log")
                    .or_else(|| input_id.strip_suffix("_status"))
                    .unwrap_or(input_id);

                // Track log source
                log_sources.write().insert(source_node.to_string());

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

                // Try to parse log entry
                if let Some(log_entry) = Self::extract_log_entry(&data, source_node, &event_meta) {
                    // Filter by level
                    let current_min = *min_level.read();
                    if log_entry.level >= current_min {
                        debug!(
                            "[{}] {}: {}",
                            log_entry.level, log_entry.node_id, log_entry.message
                        );

                        // Push log entry to SharedDoraState for UI consumption
                        if let Some(ss) = shared_state {
                            ss.logs.push(log_entry);
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

    /// Extract log entry from dora data
    fn extract_log_entry(
        data: &dora_node_api::ArrowData,
        source_node: &str,
        _metadata: &EventMetadata,
    ) -> Option<LogEntry> {
        // Try to extract string data
        let text = Self::extract_string(data)?;

        // Try to parse as JSON log
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            let level = json
                .get("level")
                .and_then(|l| l.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(LogLevel::Info);

            let message = json
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or(&text)
                .to_string();

            let node_id = json
                .get("node")
                .and_then(|n| n.as_str())
                .unwrap_or(source_node)
                .to_string();

            let timestamp = json
                .get("timestamp")
                .and_then(|t| t.as_u64())
                .unwrap_or_else(current_timestamp);

            return Some(LogEntry {
                level,
                message,
                node_id,
                timestamp,
                metadata: Default::default(),
            });
        }

        // Plain text log
        Some(LogEntry::new(LogLevel::Info, text, source_node))
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
                warn!("Unsupported log data type: {:?}", data.0.data_type());
            }
        }
        None
    }
}

impl DoraBridge for SystemLogBridge {
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
        let log_sources = Arc::clone(&self.log_sources);
        let min_level = Arc::clone(&self.min_level);

        let handle = thread::spawn(move || {
            Self::run_event_loop(
                node_id,
                state,
                shared_state,
                log_sources,
                min_level,
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

    fn send(&self, _output_id: &str, _data: DoraData) -> BridgeResult<()> {
        // System log bridge doesn't send outputs
        Ok(())
    }

    fn expected_inputs(&self) -> Vec<String> {
        // Will be populated dynamically based on dataflow
        vec!["log".to_string()]
    }

    fn expected_outputs(&self) -> Vec<String> {
        vec![]
    }
}

impl Drop for SystemLogBridge {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}
