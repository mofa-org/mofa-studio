//! Dora Integration for MoFA ASR
//!
//! Manages the lifecycle of dora bridges and routes data between
//! the dora dataflow and MoFA ASR widgets.

use crossbeam_channel::{bounded, Receiver, Sender};
use mofa_dora_bridge::{
    controller::DataflowController, dispatcher::DynamicNodeDispatcher, SharedDoraState,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

/// ASR engine identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsrEngineId {
    Paraformer,
    Qwen3Asr,
}

impl AsrEngineId {
    /// Dora node ID for this engine
    pub fn node_id(&self) -> &'static str {
        match self {
            AsrEngineId::Paraformer => "mofa-asr-paraformer",
            AsrEngineId::Qwen3Asr => "mofa-asr-qwen3",
        }
    }

    /// Binary name for this engine
    pub fn binary_name(&self) -> &'static str {
        match self {
            AsrEngineId::Paraformer => "dora-funasr-mlx",
            AsrEngineId::Qwen3Asr => "dora-qwen3-asr-mlx",
        }
    }
}

/// Manages ASR engine child processes
struct AsrProcessManager {
    paraformer: Option<std::process::Child>,
    qwen3_asr: Option<std::process::Child>,
}

impl AsrProcessManager {
    fn new() -> Self {
        Self {
            paraformer: None,
            qwen3_asr: None,
        }
    }

    fn spawn_engine(&mut self, engine: AsrEngineId, env_vars: &std::collections::HashMap<String, String>) {
        // Kill existing process first
        self.kill_engine(engine);

        let binary = engine.binary_name();
        let node_id = engine.node_id();

        // Find binary: check workspace target/release, then target/debug, then PATH
        let bin_path = Self::find_binary(binary);
        log::info!(
            "Spawning ASR engine {:?}: {} --name {}",
            engine,
            bin_path.display(),
            node_id
        );

        let mut cmd = std::process::Command::new(&bin_path);
        cmd.arg("--name")
           .arg(node_id)
           .envs(env_vars)
           .stdout(std::process::Stdio::inherit())
           .stderr(std::process::Stdio::inherit());

        match cmd.spawn()
        {
            Ok(child) => {
                log::info!(
                    "ASR engine {:?} spawned (pid={})",
                    engine,
                    child.id()
                );
                match engine {
                    AsrEngineId::Paraformer => self.paraformer = Some(child),
                    AsrEngineId::Qwen3Asr => self.qwen3_asr = Some(child),
                }
            }
            Err(e) => {
                log::error!("Failed to spawn ASR engine {:?}: {}", engine, e);
            }
        }
    }

    fn kill_engine(&mut self, engine: AsrEngineId) {
        let child = match engine {
            AsrEngineId::Paraformer => &mut self.paraformer,
            AsrEngineId::Qwen3Asr => &mut self.qwen3_asr,
        };
        if let Some(ref mut c) = child {
            log::info!("Killing ASR engine {:?} (pid={})", engine, c.id());
            let _ = c.kill();
            let _ = c.wait();
        }
        *child = None;
    }

    fn kill_all(&mut self) {
        self.kill_engine(AsrEngineId::Paraformer);
        self.kill_engine(AsrEngineId::Qwen3Asr);
    }

    /// Find binary path - checks node-hub crate target dirs, workspace target dirs, then PATH
    fn find_binary(name: &str) -> PathBuf {
        if let Ok(cwd) = std::env::current_dir() {
            for base in [&cwd, &cwd.join(".."), &cwd.join("../..")]  {
                // Check node-hub/<crate>/target/{release,debug}/ (isolated workspace binaries)
                let node_hub_release = base.join("node-hub").join(name).join("target").join("release").join(name);
                if node_hub_release.exists() {
                    return node_hub_release;
                }
                let node_hub_debug = base.join("node-hub").join(name).join("target").join("debug").join(name);
                if node_hub_debug.exists() {
                    return node_hub_debug;
                }
                // Check workspace target/{release,debug}/
                let release = base.join("target").join("release").join(name);
                if release.exists() {
                    return release;
                }
                let debug = base.join("target").join("debug").join(name);
                if debug.exists() {
                    return debug;
                }
            }
        }
        // Fall back to PATH lookup
        PathBuf::from(name)
    }
}

impl Drop for AsrProcessManager {
    fn drop(&mut self) {
        self.kill_all();
    }
}

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
    /// Force stop the dataflow immediately (0s grace period)
    ForceStopDataflow,
    /// Connect an ASR engine (spawn child process)
    ConnectAsrEngine { 
        engine: AsrEngineId,
        env_vars: std::collections::HashMap<String, String>,
    },
    /// Disconnect an ASR engine (kill child process)
    DisconnectAsrEngine { engine: AsrEngineId },
    /// Start mic recording
    StartRecording,
    /// Stop mic recording
    StopRecording,
    /// Enable/disable AEC (echo cancellation)
    SetAecEnabled { enabled: bool },
}

/// Events sent from dora integration to UI
///
/// Note: All data (transcriptions, audio, logs, status) is handled via SharedDoraState.
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

/// Dora integration manager for ASR
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
    /// This provides direct access to transcriptions, audio, logs, and status
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

    /// Force stop the dataflow immediately (0s grace period)
    pub fn force_stop_dataflow(&self) -> bool {
        self.send_command(DoraCommand::ForceStopDataflow)
    }

    /// Start mic recording
    pub fn start_recording(&self) -> bool {
        self.send_command(DoraCommand::StartRecording)
    }

    /// Stop mic recording
    pub fn stop_recording(&self) -> bool {
        self.send_command(DoraCommand::StopRecording)
    }

    /// Enable/disable AEC (echo cancellation)
    pub fn set_aec_enabled(&self, enabled: bool) -> bool {
        self.send_command(DoraCommand::SetAecEnabled { enabled })
    }

    /// Connect an ASR engine (spawn child process)
    pub fn connect_asr_engine(&self, engine: AsrEngineId, env_vars: std::collections::HashMap<String, String>) -> bool {
        self.send_command(DoraCommand::ConnectAsrEngine { engine, env_vars })
    }

    /// Disconnect an ASR engine (kill child process)
    pub fn disconnect_asr_engine(&self, engine: AsrEngineId) -> bool {
        self.send_command(DoraCommand::DisconnectAsrEngine { engine })
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
        log::info!("ASR Dora integration worker started");

        let mut dispatcher: Option<DynamicNodeDispatcher> = None;
        let mut asr_processes = AsrProcessManager::new();
        let shared_state_for_dispatcher = shared_dora_state;
        let mut last_status_check = std::time::Instant::now();
        let status_check_interval = std::time::Duration::from_secs(2);
        let mut dataflow_start_time: Option<std::time::Instant> = None;
        let startup_grace_period = std::time::Duration::from_secs(10);

        loop {
            // Check for stop signal
            if stop_rx.try_recv().is_ok() {
                log::info!("ASR Dora integration worker received stop signal");
                break;
            }

            // Process commands
            while let Ok(cmd) = command_rx.try_recv() {
                match cmd {
                    DoraCommand::StartDataflow {
                        dataflow_path,
                        env_vars,
                    } => {
                        log::info!("Starting ASR dataflow: {:?}", dataflow_path);

                        // Set environment variables
                        for (key, value) in &env_vars {
                            log::info!("Setting env var: {}=***", key);
                            std::env::set_var(key, value);
                        }

                        match DataflowController::new(&dataflow_path) {
                            Ok(mut controller) => {
                                controller.set_envs(env_vars.clone());

                                let mut disp = DynamicNodeDispatcher::with_shared_state(
                                    controller,
                                    Arc::clone(&shared_state_for_dispatcher),
                                );

                                match disp.start() {
                                    Ok(dataflow_id) => {
                                        log::info!("ASR dataflow started: {}", dataflow_id);
                                        running.store(true, Ordering::Release);
                                        dataflow_start_time = Some(std::time::Instant::now());
                                        let _ = event_tx
                                            .send(DoraEvent::DataflowStarted { dataflow_id });
                                        dispatcher = Some(disp);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to start ASR dataflow: {}", e);
                                        let _ = event_tx.send(DoraEvent::Error {
                                            message: format!("Failed to start dataflow: {}", e),
                                        });
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to create ASR controller: {}", e);
                                let _ = event_tx.send(DoraEvent::Error {
                                    message: format!("Failed to create controller: {}", e),
                                });
                            }
                        }
                    }

                    DoraCommand::StopDataflow => {
                        log::info!("Stopping ASR dataflow (graceful)");
                        asr_processes.kill_all();
                        if let Some(mut disp) = dispatcher.take() {
                            if let Err(e) = disp.stop() {
                                log::error!("Failed to stop ASR dataflow: {}", e);
                            }
                        }
                        running.store(false, Ordering::Release);
                        dataflow_start_time = None;
                        let _ = event_tx.send(DoraEvent::DataflowStopped);
                    }

                    DoraCommand::ForceStopDataflow => {
                        log::info!("Force stopping ASR dataflow (immediate kill)");
                        asr_processes.kill_all();
                        if let Some(mut disp) = dispatcher.take() {
                            if let Err(e) = disp.force_stop() {
                                log::error!("Failed to force stop ASR dataflow: {}", e);
                            }
                        }
                        running.store(false, Ordering::Release);
                        dataflow_start_time = None;
                        let _ = event_tx.send(DoraEvent::DataflowStopped);
                    }

                    DoraCommand::ConnectAsrEngine { engine, env_vars } => {
                        log::info!("Connecting ASR engine: {:?}", engine);
                        asr_processes.spawn_engine(engine, &env_vars);
                    }

                    DoraCommand::DisconnectAsrEngine { engine } => {
                        log::info!("Disconnecting ASR engine: {:?}", engine);
                        asr_processes.kill_engine(engine);
                    }

                    DoraCommand::StartRecording => {
                        if let Some(ref disp) = dispatcher {
                            if let Some(bridge) = disp.get_bridge("mofa-mic-input") {
                                log::info!("Sending start_recording to mic bridge");
                                if let Err(e) = bridge.send(
                                    "control",
                                    mofa_dora_bridge::DoraData::Json(serde_json::json!({"action": "start_recording"})),
                                ) {
                                    log::error!("Failed to send start_recording: {}", e);
                                }
                            } else {
                                log::warn!("mofa-mic-input bridge not found");
                            }
                        }
                    }

                    DoraCommand::StopRecording => {
                        if let Some(ref disp) = dispatcher {
                            if let Some(bridge) = disp.get_bridge("mofa-mic-input") {
                                log::info!("Sending stop_recording to mic bridge");
                                if let Err(e) = bridge.send(
                                    "control",
                                    mofa_dora_bridge::DoraData::Json(serde_json::json!({"action": "stop_recording"})),
                                ) {
                                    log::error!("Failed to send stop_recording: {}", e);
                                }
                            } else {
                                log::warn!("mofa-mic-input bridge not found");
                            }
                        }
                    }

                    DoraCommand::SetAecEnabled { enabled } => {
                        if let Some(ref disp) = dispatcher {
                            if let Some(bridge) = disp.get_bridge("mofa-mic-input") {
                                log::info!("Setting AEC enabled: {}", enabled);
                                if let Err(e) = bridge.send(
                                    "control",
                                    mofa_dora_bridge::DoraData::Json(serde_json::json!({"action": "set_aec_enabled", "enabled": enabled})),
                                ) {
                                    log::error!("Failed to set AEC enabled: {}", e);
                                }
                            } else {
                                log::warn!("mofa-mic-input bridge not found");
                            }
                        }
                    }

                }
            }

            // Periodic status check - verify dataflow is actually running
            let in_grace_period = dataflow_start_time
                .map(|t| t.elapsed() < startup_grace_period)
                .unwrap_or(false);

            if !in_grace_period && last_status_check.elapsed() >= status_check_interval {
                last_status_check = std::time::Instant::now();

                if let Some(ref disp) = dispatcher {
                    match disp.controller().read().get_status() {
                        Ok(status) => {
                            let was_running = running.load(Ordering::Acquire);
                            let is_running = status.state.is_running();

                            if was_running && !is_running {
                                log::warn!("ASR dataflow stopped unexpectedly");
                                running.store(false, Ordering::Release);
                                dataflow_start_time = None;
                                let _ = event_tx.send(DoraEvent::DataflowStopped);
                            }
                        }
                        Err(e) => {
                            log::debug!("ASR status check failed: {}", e);
                        }
                    }
                }
            }

            // Check SharedDoraState for critical errors
            if let Some(status) = shared_state_for_dispatcher.status.read_if_dirty() {
                if let Some(error) = status.last_error {
                    log::error!("ASR bridge error: {}", error);
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

        log::info!("ASR Dora integration worker stopped");
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
