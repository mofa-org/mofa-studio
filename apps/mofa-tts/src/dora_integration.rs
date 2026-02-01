//! Dora Integration for MoFA TTS
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
    /// Send a prompt to TTS (reusing PromptInputBridge for text storage/sending)
    SendPrompt { message: String },
    /// Send audio to ASR for transcription
    SendAudio {
        audio_samples: Vec<f32>,
        sample_rate: u32,
        language: String,
    },
}

/// Events sent from dora integration to UI
#[derive(Debug, Clone)]
pub enum DoraEvent {
    /// Dataflow started
    DataflowStarted { dataflow_id: String },
    /// Dataflow stopped
    DataflowStopped,
    /// Critical error occurred
    Error { message: String },
    /// ASR transcription result
    AsrTranscription { text: String, language: String },
}

/// Helper function to send data with retry logic
///
/// Retries up to 10 times with 150ms delay between attempts (1.5s total).
/// This is reduced from 20 retries to minimize thread blocking in the worker.
fn send_with_retry(
    bridge: &dyn mofa_dora_bridge::DoraBridge,
    output: &str,
    data: mofa_dora_bridge::DoraData,
) -> Result<(), String> {
    const MAX_RETRIES: u32 = 10;
    const RETRY_DELAY_MS: u64 = 150;

    for attempt in 1..=MAX_RETRIES {
        match bridge.send(output, data.clone()) {
            Ok(_) => {
                if attempt > 1 {
                    log::info!("Successfully sent data on attempt {}", attempt);
                }
                return Ok(());
            }
            Err(e) => {
                if attempt == MAX_RETRIES {
                    return Err(format!("Failed after {} retries: {}", MAX_RETRIES, e));
                }
                std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
            }
        }
    }
    Err("Retry exhausted".into())
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
    pub fn shared_dora_state(&self) -> &Arc<SharedDoraState> {
        &self.shared_dora_state
    }

    /// Send a command to the dora integration (non-blocking)
    pub fn send_command(&self, cmd: DoraCommand) -> bool {
        self.command_tx.try_send(cmd).is_ok()
    }

    /// Start a dataflow with optional environment variables
    pub fn start_dataflow(&self, dataflow_path: impl Into<PathBuf>) -> bool {
        let path = dataflow_path.into();
        self.send_command(DoraCommand::StartDataflow {
            dataflow_path: path,
            env_vars: std::collections::HashMap::new(),
        })
    }

    /// Stop the current dataflow gracefully
    pub fn stop_dataflow(&self) -> bool {
        self.send_command(DoraCommand::StopDataflow)
    }

    /// Send text to TTS
    pub fn send_prompt(&self, message: impl Into<String>) -> bool {
        self.send_command(DoraCommand::SendPrompt {
            message: message.into(),
        })
    }

    /// Send audio to ASR for transcription
    pub fn send_audio(&self, audio_samples: Vec<f32>, sample_rate: u32, language: String) -> bool {
        self.send_command(DoraCommand::SendAudio {
            audio_samples,
            sample_rate,
            language,
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
        let startup_grace_period = std::time::Duration::from_secs(10);

        loop {
            // Check for stop signal
            if stop_rx.try_recv().is_ok() {
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

                        // IMPORTANT: Stop any existing dispatcher first to avoid "Bridge already connected" errors
                        if let Some(mut old_disp) = dispatcher.take() {
                            log::warn!("Stopping existing dataflow before starting new one");
                            if let Err(e) = old_disp.stop() {
                                log::error!("Failed to stop existing dataflow: {}", e);
                            }
                            // Give bridges time to fully disconnect
                            std::thread::sleep(Duration::from_millis(500));
                        }

                        for (key, value) in &env_vars {
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
                                        log::info!("Dataflow started: {}", dataflow_id);

                                        // Log discovered MoFA nodes for debugging
                                        let mofa_nodes = disp.discover_mofa_nodes();
                                        log::info!("Discovered {} MoFA nodes:", mofa_nodes.len());
                                        for node in &mofa_nodes {
                                            log::info!(
                                                "  - {} (type: {:?})",
                                                node.id,
                                                node.node_type
                                            );
                                        }

                                        // Check which bridges are actually connected
                                        log::info!("Checking bridge connection status...");
                                        let bindings = disp.bindings();
                                        let mut connected_bridges = Vec::new();
                                        for binding in bindings {
                                            log::info!(
                                                "  Bridge {}: state={:?}",
                                                binding.node_id,
                                                binding.state
                                            );
                                            if binding.state == mofa_dora_bridge::BridgeState::Connected {
                                                connected_bridges.push(binding.node_id.clone());
                                            }
                                        }

                                        // Update shared state with connected bridges
                                        shared_state_for_dispatcher.status.set(
                                            mofa_dora_bridge::DoraStatus {
                                                active_bridges: connected_bridges,
                                                last_error: None,
                                            }
                                        );

                                        running.store(true, Ordering::Release);
                                        dataflow_start_time = Some(std::time::Instant::now());
                                        let _ = event_tx
                                            .send(DoraEvent::DataflowStarted { dataflow_id });
                                        dispatcher = Some(disp);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to start dataflow: {}", e);
                                        // Clear bridges on failure
                                        shared_state_for_dispatcher.status.set(
                                            mofa_dora_bridge::DoraStatus {
                                                active_bridges: Vec::new(),
                                                last_error: Some(e.to_string()),
                                            }
                                        );
                                        let _ = event_tx.send(DoraEvent::Error {
                                            message: format!("Failed to start dataflow: {}", e),
                                        });
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to create controller: {}", e);
                                // Clear bridges on failure
                                shared_state_for_dispatcher.status.set(
                                    mofa_dora_bridge::DoraStatus {
                                        active_bridges: Vec::new(),
                                        last_error: Some(e.to_string()),
                                    }
                                );
                                let _ = event_tx.send(DoraEvent::Error {
                                    message: format!("Failed to create controller: {}", e),
                                });
                            }
                        }
                    }

                    DoraCommand::StopDataflow => {
                        log::info!("Stopping dataflow");
                        if let Some(mut disp) = dispatcher.take() {
                            if let Err(e) = disp.stop() {
                                log::error!("Failed to stop dataflow: {}", e);
                            }
                            // Give bridges time to fully disconnect before allowing restart
                            log::debug!("Waiting for bridges to fully disconnect...");
                            std::thread::sleep(Duration::from_millis(300));
                        }

                        // Clear shared state
                        shared_state_for_dispatcher.status.set(
                            mofa_dora_bridge::DoraStatus {
                                active_bridges: Vec::new(),
                                last_error: None,
                            }
                        );

                        running.store(false, Ordering::Release);
                        dataflow_start_time = None;
                        let _ = event_tx.send(DoraEvent::DataflowStopped);
                    }

                    DoraCommand::SendPrompt { message } => {
                        if let Some(ref disp) = dispatcher {
                            // Try generic prompt input bridge or TTS-specific one if we define it in dataflow
                            if let Some(bridge) = disp
                                .get_bridge("mofa-prompt-input-tts")
                                .or_else(|| disp.get_bridge("mofa-prompt-input"))
                            {
                                log::info!("Sending text to TTS via bridge: {}", message);
                                if let Err(e) = send_with_retry(
                                    bridge,
                                    "prompt",
                                    mofa_dora_bridge::DoraData::Text(message.clone()),
                                ) {
                                    log::error!("Failed to send text: {}", e);
                                }
                            } else {
                                log::warn!("mofa-prompt-input bridge not found");
                            }
                        }
                    }

                    DoraCommand::SendAudio {
                        audio_samples,
                        sample_rate,
                        language,
                    } => {
                        if let Some(ref disp) = dispatcher {
                            if let Some(bridge) = disp.get_bridge("mofa-audio-input") {
                                log::info!(
                                    "Sending audio to ASR: {} samples at {}Hz, language: {}",
                                    audio_samples.len(),
                                    sample_rate,
                                    language
                                );

                                // Create audio data with metadata
                                let audio_data = mofa_dora_bridge::DoraData::Audio(
                                    mofa_dora_bridge::AudioData {
                                        samples: audio_samples,
                                        sample_rate,
                                        channels: 1,
                                        participant_id: None,
                                        question_id: None,
                                    },
                                );

                                if let Err(e) = send_with_retry(bridge, "audio", audio_data) {
                                    log::error!("Failed to send audio: {}", e);
                                    let _ = event_tx.send(DoraEvent::Error {
                                        message: format!("Failed to send audio to ASR: {}", e),
                                    });
                                }
                            } else {
                                log::warn!("mofa-audio-input bridge not found");
                                let _ = event_tx.send(DoraEvent::Error {
                                    message:
                                        "ASR not available (mofa-audio-input bridge not found)"
                                            .to_string(),
                                });
                            }
                        }
                    }
                }
            }

            // Periodic status check
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

            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        if let Some(mut disp) = dispatcher {
            let _ = disp.stop();
        }

        log::info!("Dora integration worker stopped");
    }
}

impl Drop for DoraIntegration {
    fn drop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
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
