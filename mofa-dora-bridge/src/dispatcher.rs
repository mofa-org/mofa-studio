//! Dynamic node dispatcher
//!
//! Manages connections between MoFA widgets and their corresponding
//! dora dynamic nodes. Each widget type has its own bridge that
//! connects as a separate dynamic node.

use crate::bridge::{BridgeState, DoraBridge};
use crate::controller::DataflowController;
use crate::error::{BridgeError, BridgeResult};
use crate::parser::MofaNodeSpec;
use crate::shared_state::SharedDoraState;
use crate::widgets::{
    AecInputBridge, AsrListenerBridge, AudioInputBridge, AudioPlayerBridge, PromptInputBridge,
    SystemLogBridge,
};
use crate::MofaNodeType;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Binding between a widget and its dora node
#[derive(Debug, Clone)]
pub struct WidgetBinding {
    /// Widget identifier in the UI
    pub widget_id: String,
    /// MoFA node type
    pub node_type: MofaNodeType,
    /// Node ID in the dataflow
    pub node_id: String,
    /// Connection state
    pub state: BridgeState,
}

/// Dispatcher for managing dynamic node connections
///
/// Status updates and data are now communicated via SharedDoraState
/// instead of event channels.
pub struct DynamicNodeDispatcher {
    /// Dataflow controller
    controller: Arc<RwLock<DataflowController>>,
    /// Shared state for Dora↔UI communication
    shared_state: Arc<SharedDoraState>,
    /// Active bridges indexed by node ID
    bridges: HashMap<String, Box<dyn DoraBridge>>,
    /// Widget bindings
    bindings: Vec<WidgetBinding>,
}

impl DynamicNodeDispatcher {
    /// Create a new dispatcher with a dataflow controller
    pub fn new(controller: DataflowController) -> Self {
        Self::with_shared_state(controller, SharedDoraState::new())
    }

    /// Create a new dispatcher with an external shared state
    ///
    /// This allows the caller to hold a reference to the shared state
    /// for direct UI polling without going through channels.
    pub fn with_shared_state(
        controller: DataflowController,
        shared_state: Arc<SharedDoraState>,
    ) -> Self {
        Self {
            controller: Arc::new(RwLock::new(controller)),
            shared_state,
            bridges: HashMap::new(),
            bindings: Vec::new(),
        }
    }

    /// Get the dataflow controller
    pub fn controller(&self) -> &Arc<RwLock<DataflowController>> {
        &self.controller
    }

    /// Get the shared state for Dora↔UI communication
    pub fn shared_state(&self) -> &Arc<SharedDoraState> {
        &self.shared_state
    }

    /// Discover MoFA nodes from the parsed dataflow
    pub fn discover_mofa_nodes(&self) -> Vec<MofaNodeSpec> {
        self.controller
            .read()
            .parsed()
            .map(|p| p.mofa_nodes.clone())
            .unwrap_or_default()
    }

    /// Create bridges for all discovered MoFA nodes
    pub fn create_bridges(&mut self) -> BridgeResult<()> {
        let mofa_nodes = self.discover_mofa_nodes();
        let shared_state = Some(self.shared_state.clone());

        for node_spec in mofa_nodes {
            let bridge: Box<dyn DoraBridge> = match node_spec.node_type {
                MofaNodeType::AudioPlayer => Box::new(AudioPlayerBridge::with_shared_state(
                    &node_spec.id,
                    shared_state.clone(),
                )),
                MofaNodeType::SystemLog => Box::new(SystemLogBridge::with_shared_state(
                    &node_spec.id,
                    shared_state.clone(),
                )),
                MofaNodeType::PromptInput => Box::new(PromptInputBridge::with_shared_state(
                    &node_spec.id,
                    shared_state.clone(),
                )),
                MofaNodeType::MicInput => Box::new(AecInputBridge::with_shared_state(
                    &node_spec.id,
                    shared_state.clone(),
                )),
                MofaNodeType::ChatViewer => {
                    // TODO: Implement ChatViewerBridge
                    continue;
                }
                MofaNodeType::ParticipantPanel => {
                    // ParticipantPanel functionality consolidated into AudioPlayerBridge
                    // No separate bridge needed - AudioPlayerBridge now handles LED visualization
                    info!("Skipping ParticipantPanel bridge - consolidated into AudioPlayerBridge");
                    continue;
                }
                MofaNodeType::AsrListener => Box::new(AsrListenerBridge::with_shared_state(
                    &node_spec.id,
                    shared_state.clone(),
                )),
                MofaNodeType::AudioInput => Box::new(AudioInputBridge::with_shared_state(
                    &node_spec.id,
                    shared_state
                        .clone()
                        .expect("AudioInputBridge requires shared state"),
                )),
            };

            self.bindings.push(WidgetBinding {
                widget_id: node_spec.id.clone(),
                node_type: node_spec.node_type,
                node_id: node_spec.id.clone(),
                state: BridgeState::Disconnected,
            });

            self.bridges.insert(node_spec.id, bridge);
        }

        info!("Created {} bridges with shared state", self.bridges.len());
        Ok(())
    }

    /// Connect all bridges to the dataflow
    pub fn connect_all(&mut self) -> BridgeResult<()> {
        // Ensure dataflow is running
        {
            let controller = self.controller.read();
            if !controller.state().is_running() {
                return Err(BridgeError::DataflowNotRunning);
            }
        }

        let mut errors = Vec::new();

        for (node_id, bridge) in &mut self.bridges {
            match bridge.connect() {
                Ok(()) => {
                    info!("Connected bridge: {}", node_id);
                    // Update binding state
                    if let Some(binding) = self.bindings.iter_mut().find(|b| &b.node_id == node_id)
                    {
                        binding.state = BridgeState::Connected;
                    }
                }
                Err(e) => {
                    error!("Failed to connect bridge {}: {}", node_id, e);
                    errors.push(format!("{}: {}", node_id, e));
                }
            }
        }

        if !errors.is_empty() {
            return Err(BridgeError::ConnectionFailed(errors.join("; ")));
        }

        Ok(())
    }

    /// Disconnect all bridges
    pub fn disconnect_all(&mut self) -> BridgeResult<()> {
        let mut errors = Vec::new();

        for (node_id, bridge) in &mut self.bridges {
            match bridge.disconnect() {
                Ok(()) => {
                    debug!("Disconnected bridge: {}", node_id);
                    if let Some(binding) = self.bindings.iter_mut().find(|b| &b.node_id == node_id)
                    {
                        binding.state = BridgeState::Disconnected;
                    }
                }
                Err(e) => {
                    error!("Failed to disconnect bridge {}: {}", node_id, e);
                    errors.push(format!("{}: {}", node_id, e));
                }
            }
        }

        if !errors.is_empty() {
            return Err(BridgeError::Unknown(errors.join("; ")));
        }

        Ok(())
    }

    /// Get a bridge by node ID
    pub fn get_bridge(&self, node_id: &str) -> Option<&dyn DoraBridge> {
        self.bridges.get(node_id).map(|b| b.as_ref())
    }

    /// Get a mutable bridge by node ID
    pub fn get_bridge_mut(&mut self, node_id: &str) -> Option<&mut Box<dyn DoraBridge>> {
        self.bridges.get_mut(node_id)
    }

    /// Get all bindings
    pub fn bindings(&self) -> &[WidgetBinding] {
        &self.bindings
    }

    /// Get binding for a specific node
    pub fn get_binding(&self, node_id: &str) -> Option<&WidgetBinding> {
        self.bindings.iter().find(|b| b.node_id == node_id)
    }

    /// Start the dataflow and connect all bridges
    pub fn start(&mut self) -> BridgeResult<String> {
        // Start the dataflow
        let dataflow_id = {
            let mut controller = self.controller.write();
            controller.start()?
        };

        // Wait for dataflow to initialize and register dynamic nodes
        // This is necessary because `dora start --detach` returns immediately
        info!("Waiting for dataflow to initialize...");
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Create bridges if not already created
        if self.bridges.is_empty() {
            self.create_bridges()?;
        }

        info!("Connecting {} bridges to dora...", self.bridges.len());

        const MAX_CONNECT_ATTEMPTS: usize = 15;
        let connect_retry_delay = std::time::Duration::from_secs(2);

        let mut last_err: Option<BridgeError> = None;
        for attempt in 1..=MAX_CONNECT_ATTEMPTS {
            match self.connect_all() {
                Ok(()) => {
                    info!("All bridges connected after {} attempt(s)", attempt);
                    last_err = None;
                    break;
                }
                Err(e) => {
                    warn!(
                        "Bridge connection attempt {} failed: {}. Retrying...",
                        attempt, e
                    );
                    last_err = Some(e);
                    std::thread::sleep(connect_retry_delay);
                }
            }
        }

        if let Some(err) = last_err {
            error!(
                "Failed to connect bridges after {} attempts: {}",
                MAX_CONNECT_ATTEMPTS, err
            );
            return Err(err);
        }

        Ok(dataflow_id)
    }

    /// Stop the dataflow and disconnect all bridges (graceful, default 15s)
    pub fn stop(&mut self) -> BridgeResult<()> {
        // Disconnect bridges first
        self.disconnect_all()?;

        // Stop the dataflow
        let mut controller = self.controller.write();
        controller.stop()?;

        Ok(())
    }

    /// Stop the dataflow with a custom grace duration
    ///
    /// After the grace duration, nodes that haven't stopped will be killed (SIGKILL).
    pub fn stop_with_grace_duration(
        &mut self,
        grace_duration: std::time::Duration,
    ) -> BridgeResult<()> {
        // Disconnect bridges first
        self.disconnect_all()?;

        // Stop the dataflow with grace duration
        let mut controller = self.controller.write();
        controller.stop_with_grace_duration(grace_duration)?;

        Ok(())
    }

    /// Force stop the dataflow immediately (0s grace period)
    ///
    /// This will immediately kill all nodes without waiting for graceful shutdown.
    pub fn force_stop(&mut self) -> BridgeResult<()> {
        // Disconnect bridges first
        self.disconnect_all()?;

        // Force stop the dataflow
        let mut controller = self.controller.write();
        controller.force_stop()?;

        Ok(())
    }

    /// Check if the dispatcher is running
    pub fn is_running(&self) -> bool {
        self.controller.read().state().is_running()
    }
}

impl Drop for DynamicNodeDispatcher {
    fn drop(&mut self) {
        // Disconnect all bridges
        let _ = self.disconnect_all();
    }
}

/// Builder for creating dispatchers with custom configuration
pub struct DispatcherBuilder {
    controller: Option<DataflowController>,
    auto_connect: bool,
}

impl DispatcherBuilder {
    pub fn new() -> Self {
        Self {
            controller: None,
            auto_connect: false,
        }
    }

    pub fn with_controller(mut self, controller: DataflowController) -> Self {
        self.controller = Some(controller);
        self
    }

    pub fn auto_connect(mut self, auto: bool) -> Self {
        self.auto_connect = auto;
        self
    }

    pub fn build(self) -> BridgeResult<DynamicNodeDispatcher> {
        let controller = self
            .controller
            .ok_or_else(|| BridgeError::Unknown("No controller provided".to_string()))?;

        let mut dispatcher = DynamicNodeDispatcher::new(controller);

        if self.auto_connect {
            dispatcher.start()?;
        }

        Ok(dispatcher)
    }
}

impl Default for DispatcherBuilder {
    fn default() -> Self {
        Self::new()
    }
}
