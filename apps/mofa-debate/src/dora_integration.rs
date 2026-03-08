//! Dora Integration for MoFA FM
//!
//! Manages the lifecycle of dora bridges and routes data between
//! the dora dataflow and MoFA widgets.

use crossbeam_channel::{bounded, Receiver, Sender};
use mofa_dora_bridge::{
    controller::DataflowController, dispatcher::DynamicNodeDispatcher, SharedDoraState,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Commands sent from UI to dora integration
#[derive(Debug, Clone)]
pub enum DoraCommand {
    /// Start the dataflow with optional environment variables
    StartDataflow {
        dataflow_path: PathBuf,
        env_vars: std::collections::HashMap<String, String>,
    },
    /// Stop the dataflow gracefully (default 15s grace period)
    StopDataflow,
    /// Stop the dataflow with custom grace duration (in seconds)
    StopDataflowWithGrace { grace_seconds: u64 },
    /// Force stop the dataflow immediately (0s grace period)
    ForceStopDataflow,
    /// Send a prompt to LLM
    SendPrompt { message: String },
    /// Send a control command
    SendControl { command: String },
    /// Update buffer status
    UpdateBufferStatus { fill_percentage: f64 },
}

/// Events sent from dora integration to UI
///
/// Note: All data (chat, audio, logs, status) is now handled via SharedDoraState.
/// DoraEvents are only used for control flow notifications.
#[derive(Debug, Clone)]
pub enum DoraEvent {
    /// Dataflow started
    DataflowStarted { dataflow_id: String },
    /// Dataflow stopped
    DataflowStopped,
    /// Critical error occurred
    Error { message: String },
}

/// Dora integration manager
pub struct DoraIntegration {
    /// Whether dataflow is currently running
    running: Arc<AtomicBool>,
    /// Shared state for direct Dora↔UI communication
    shared_dora_state: Arc<SharedDoraState>,
    /// Command sender (UI -> dora thread)
    command_tx: Sender<DoraCommand>,
    /// Event receiver (dora thread -> UI)
    event_rx: Receiver<DoraEvent>,
    /// Worker thread handle
    worker_handle: Option<thread::JoinHandle<()>>,
    /// Stop signal
    stop_tx: Option<Sender<()>>,
}

impl DoraIntegration {
    /// Create a new dora integration (not started)
    pub fn new() -> Self {
        let (command_tx, command_rx) = bounded(100);
        let (event_tx, event_rx) = bounded(100);
        let (stop_tx, stop_rx) = bounded(1);

        let running = Arc::new(AtomicBool::new(false));
        let running_clone = Arc::clone(&running);

        // Create shared state for direct Dora↔UI communication
        let shared_dora_state = SharedDoraState::new();
        let shared_dora_state_clone = Arc::clone(&shared_dora_state);

        // Spawn worker thread
        let handle = thread::spawn(move || {
            Self::run_worker(
                running_clone,
                shared_dora_state_clone,
                command_rx,
                event_tx,
                stop_rx,
            );
        });

        Self {
            running,
            shared_dora_state,
            command_tx,
            event_rx,
            worker_handle: Some(handle),
            stop_tx: Some(stop_tx),
        }
    }

    /// Get shared Dora state for direct UI polling
    ///
    /// This provides direct access to chat, audio, logs, and status
    /// without going through the event channel.
    pub fn shared_dora_state(&self) -> &Arc<SharedDoraState> {
        &self.shared_dora_state
    }

    /// Send a command to the dora integration
    pub fn send_command(&self, cmd: DoraCommand) -> bool {
        self.command_tx.send(cmd).is_ok()
    }

    /// Start a dataflow with optional environment variables
    pub fn start_dataflow(&self, dataflow_path: impl Into<PathBuf>) -> bool {
        self.start_dataflow_with_env(dataflow_path, std::collections::HashMap::new())
    }

    /// Start a dataflow with environment variables
    pub fn start_dataflow_with_env(
        &self,
        dataflow_path: impl Into<PathBuf>,
        env_vars: std::collections::HashMap<String, String>,
    ) -> bool {
        self.send_command(DoraCommand::StartDataflow {
            dataflow_path: dataflow_path.into(),
            env_vars,
        })
    }

    /// Stop the current dataflow gracefully (default 15s grace period)
    pub fn stop_dataflow(&self) -> bool {
        self.send_command(DoraCommand::StopDataflow)
    }

    /// Stop the dataflow with a custom grace duration
    ///
    /// After the grace duration, nodes that haven't stopped will be killed (SIGKILL).
    pub fn stop_dataflow_with_grace(&self, grace_seconds: u64) -> bool {
        self.send_command(DoraCommand::StopDataflowWithGrace { grace_seconds })
    }

    /// Force stop the dataflow immediately (0s grace period)
    ///
    /// This will immediately kill all nodes without waiting for graceful shutdown.
    pub fn force_stop_dataflow(&self) -> bool {
        self.send_command(DoraCommand::ForceStopDataflow)
    }

    /// Send a prompt to LLM
    pub fn send_prompt(&self, message: impl Into<String>) -> bool {
        self.send_command(DoraCommand::SendPrompt {
            message: message.into(),
        })
    }

    /// Send a control command (e.g., "reset", "cancel")
    pub fn send_control(&self, command: impl Into<String>) -> bool {
        self.send_command(DoraCommand::SendControl {
            command: command.into(),
        })
    }

    /// Poll for events (non-blocking)
    pub fn poll_events(&self) -> Vec<DoraEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Check if dataflow is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Worker thread main loop
    fn run_worker(
        running: Arc<AtomicBool>,
        shared_dora_state: Arc<SharedDoraState>,
        command_rx: Receiver<DoraCommand>,
        event_tx: Sender<DoraEvent>,
        stop_rx: Receiver<()>,
    ) {
        log::info!("Dora integration worker started");

        let mut dispatcher: Option<DynamicNodeDispatcher> = None;
        let shared_state_for_dispatcher = shared_dora_state;
        let mut last_status_check = std::time::Instant::now();
        let status_check_interval = std::time::Duration::from_secs(2);
        let mut dataflow_start_time: Option<std::time::Instant> = None;
        let startup_grace_period = std::time::Duration::from_secs(10); // Don't check status during startup

        loop {
            // Check for stop signal
            if stop_rx.try_recv().is_ok() {
                log::info!("Dora integration worker received stop signal");
                break;
            }

            // Process commands
            while let Ok(cmd) = command_rx.try_recv() {
                match cmd {
                    DoraCommand::StartDataflow {
                        dataflow_path,
                        env_vars,
                    } => {
                        log::info!("Starting dataflow: {:?}", dataflow_path);

                        // Pass environment variables only to the dataflow controller
                        for key in env_vars.keys() {
                            log::info!("Setting env var: {}=***", key);
                        }

                        match DataflowController::new(&dataflow_path) {
                            Ok(mut controller) => {
                                // Pass env vars to controller so they're explicitly added to dora start command
                                controller.set_envs(env_vars.clone());

                                // Create dispatcher with shared state for UI polling
                                let mut disp = DynamicNodeDispatcher::with_shared_state(
                                    controller,
                                    Arc::clone(&shared_state_for_dispatcher),
                                );

                                match disp.start() {
                                    Ok(dataflow_id) => {
                                        log::info!("Dataflow started: {}", dataflow_id);
                                        running.store(true, Ordering::Release);
                                        dataflow_start_time = Some(std::time::Instant::now());
                                        let _ = event_tx
                                            .send(DoraEvent::DataflowStarted { dataflow_id });
                                        dispatcher = Some(disp);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to start dataflow: {}", e);
                                        let _ = event_tx.send(DoraEvent::Error {
                                            message: format!("Failed to start dataflow: {}", e),
                                        });
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to create controller: {}", e);
                                let _ = event_tx.send(DoraEvent::Error {
                                    message: format!("Failed to create controller: {}", e),
                                });
                            }
                        }
                    }

                    DoraCommand::StopDataflow => {
                        log::info!("Stopping dataflow (graceful)");
                        if let Some(mut disp) = dispatcher.take() {
                            if let Err(e) = disp.stop() {
                                log::error!("Failed to stop dataflow: {}", e);
                            }
                        }
                        running.store(false, Ordering::Release);
                        dataflow_start_time = None;
                        let _ = event_tx.send(DoraEvent::DataflowStopped);
                    }

                    DoraCommand::StopDataflowWithGrace { grace_seconds } => {
                        log::info!("Stopping dataflow (grace: {}s)", grace_seconds);
                        if let Some(mut disp) = dispatcher.take() {
                            let duration = std::time::Duration::from_secs(grace_seconds);
                            if let Err(e) = disp.stop_with_grace_duration(duration) {
                                log::error!("Failed to stop dataflow: {}", e);
                            }
                        }
                        running.store(false, Ordering::Release);
                        dataflow_start_time = None;
                        let _ = event_tx.send(DoraEvent::DataflowStopped);
                    }

                    DoraCommand::ForceStopDataflow => {
                        log::info!("Force stopping dataflow (immediate kill)");
                        if let Some(mut disp) = dispatcher.take() {
                            if let Err(e) = disp.force_stop() {
                                log::error!("Failed to force stop dataflow: {}", e);
                            }
                        }
                        running.store(false, Ordering::Release);
                        dataflow_start_time = None;
                        let _ = event_tx.send(DoraEvent::DataflowStopped);
                    }

                    DoraCommand::SendPrompt { message } => {
                        // Helper to retry while bridges are connecting
                        let send_with_retry = |bridge: &dyn mofa_dora_bridge::DoraBridge,
                                               output: &str,
                                               data: mofa_dora_bridge::DoraData|
                         -> Result<(), String> {
                            let retries = 20;
                            for attempt in 1..=retries {
                                match bridge.send(output, data.clone()) {
                                    Ok(_) => return Ok(()),
                                    Err(e) => {
                                        if attempt == retries {
                                            return Err(e.to_string());
                                        }
                                        std::thread::sleep(Duration::from_millis(150));
                                    }
                                }
                            }
                            Err("retry exhausted".into())
                        };

                        if let Some(ref disp) = dispatcher {
                            if let Some(bridge) = disp
                                .get_bridge("mofa-prompt-input-debate")
                                .or_else(|| disp.get_bridge("mofa-prompt-input"))
                            {
                                log::info!("Sending prompt via bridge: {}", message);
                                if let Err(e) = send_with_retry(
                                    bridge,
                                    "prompt",
                                    mofa_dora_bridge::DoraData::Text(message.clone()),
                                ) {
                                    log::error!("Failed to send prompt: {}", e);
                                }
                            } else {
                                log::warn!("mofa-prompt-input bridge not found");
                            }
                        }
                    }

                    DoraCommand::SendControl { command } => {
                        // Helper to retry while bridges are connecting
                        let send_with_retry = |bridge: &dyn mofa_dora_bridge::DoraBridge,
                                               output: &str,
                                               data: mofa_dora_bridge::DoraData|
                         -> Result<(), String> {
                            let retries = 20;
                            for attempt in 1..=retries {
                                match bridge.send(output, data.clone()) {
                                    Ok(_) => return Ok(()),
                                    Err(e) => {
                                        if attempt == retries {
                                            return Err(e.to_string());
                                        }
                                        std::thread::sleep(Duration::from_millis(150));
                                    }
                                }
                            }
                            Err("retry exhausted".into())
                        };

                        if let Some(ref disp) = dispatcher {
                            if let Some(bridge) = disp
                                .get_bridge("mofa-prompt-input-debate")
                                .or_else(|| disp.get_bridge("mofa-prompt-input"))
                            {
                                log::info!("Sending control command: {}", command);
                                let ctrl = mofa_dora_bridge::ControlCommand::new(&command);
                                if let Err(e) = send_with_retry(
                                    bridge,
                                    "control",
                                    mofa_dora_bridge::DoraData::Control(ctrl),
                                ) {
                                    log::error!("Failed to send control: {}", e);
                                }
                            } else {
                                log::warn!("mofa-prompt-input bridge not found for control");
                            }
                        }
                    }

                    DoraCommand::UpdateBufferStatus { fill_percentage } => {
                        // Forward to audio player bridge for backpressure signaling to dora
                        if let Some(ref disp) = dispatcher {
                            if let Some(bridge) = disp
                                .get_bridge("mofa-audio-player-debate")
                                .or_else(|| disp.get_bridge("mofa-audio-player"))
                            {
                                if let Err(e) = bridge.send(
                                    "buffer_status",
                                    mofa_dora_bridge::DoraData::Json(serde_json::json!(
                                        fill_percentage
                                    )),
                                ) {
                                    log::debug!("Failed to send buffer status to bridge: {}", e);
                                }
                            }
                        }
                    }
                }
            }

            // Periodic status check - verify dataflow is actually running
            // Skip during startup grace period to avoid false positives
            let in_grace_period = dataflow_start_time
                .map(|t| t.elapsed() < startup_grace_period)
                .unwrap_or(false);

            if !in_grace_period && last_status_check.elapsed() >= status_check_interval {
                last_status_check = std::time::Instant::now();

                if let Some(ref disp) = dispatcher {
                    // Check if dataflow is still running via dora list
                    match disp.controller().read().get_status() {
                        Ok(status) => {
                            let was_running = running.load(Ordering::Acquire);
                            let is_running = status.state.is_running();

                            if was_running && !is_running {
                                // Dataflow stopped unexpectedly
                                log::warn!("Dataflow stopped unexpectedly");
                                running.store(false, Ordering::Release);
                                dataflow_start_time = None;
                                let _ = event_tx.send(DoraEvent::DataflowStopped);
                            }
                        }
                        Err(e) => {
                            log::debug!("Status check failed: {}", e);
                        }
                    }
                }
            }

            // Check SharedDoraState for critical errors (UI polls everything else directly)
            if let Some(status) = shared_state_for_dispatcher.status.read_if_dirty() {
                if let Some(error) = status.last_error {
                    log::error!("Bridge error: {}", error);
                    let _ = event_tx.send(DoraEvent::Error { message: error });
                }
            }

            // Small sleep to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Cleanup
        if let Some(mut disp) = dispatcher {
            let _ = disp.stop();
        }

        log::info!("Dora integration worker stopped");
    }
}

impl Drop for DoraIntegration {
    fn drop(&mut self) {
        // Send stop signal
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }

        // Wait for worker thread
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Default for DoraIntegration {
    fn default() -> Self {
        Self::new()
    }
}
