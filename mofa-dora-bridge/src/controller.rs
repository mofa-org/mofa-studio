//! Dataflow lifecycle controller
//!
//! Manages the lifecycle of dora dataflows:
//! - Start dataflow with env configuration
//! - Stop dataflow and cleanup resources
//! - Monitor dataflow status

use crate::error::{BridgeError, BridgeResult};
use crate::parser::{DataflowParser, ParsedDataflow};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Dataflow state
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DataflowState {
    /// Dataflow is stopped
    #[default]
    Stopped,
    /// Dataflow is starting
    Starting,
    /// Dataflow is running
    Running {
        started_at: Instant,
        dataflow_id: String,
    },
    /// Dataflow is stopping
    Stopping,
    /// Dataflow encountered an error
    Error { message: String },
}


impl DataflowState {
    pub fn is_running(&self) -> bool {
        matches!(self, DataflowState::Running { .. })
    }

    pub fn is_stopped(&self) -> bool {
        matches!(self, DataflowState::Stopped)
    }
}

/// Controller for managing dataflow lifecycle
pub struct DataflowController {
    /// Path to the dataflow YAML file
    dataflow_path: PathBuf,
    /// Parsed dataflow information
    parsed: Option<ParsedDataflow>,
    /// Current state
    state: Arc<RwLock<DataflowState>>,
    /// Environment variables to apply
    env_vars: HashMap<String, String>,
    /// Dora daemon process (if we started it)
    daemon_process: Option<Child>,
}

impl DataflowController {
    /// Create a new controller for a dataflow
    pub fn new(dataflow_path: impl AsRef<Path>) -> BridgeResult<Self> {
        let original_path = dataflow_path.as_ref();
        // Canonicalize to avoid surprises when callers pass relative paths coming
        // from different working directories. If canonicalize fails (e.g. missing
        // file), fall back to the provided path so the parser can surface the error.
        let path = original_path
            .canonicalize()
            .unwrap_or_else(|_| original_path.to_path_buf());

        // Parse the dataflow
        let parsed = DataflowParser::parse(&path)?;

        Ok(Self {
            dataflow_path: path,
            parsed: Some(parsed),
            state: Arc::new(RwLock::new(DataflowState::Stopped)),
            env_vars: HashMap::new(),
            daemon_process: None,
        })
    }

    /// Get the parsed dataflow
    pub fn parsed(&self) -> Option<&ParsedDataflow> {
        self.parsed.as_ref()
    }

    /// Get current state
    pub fn state(&self) -> DataflowState {
        self.state.read().clone()
    }

    /// Set environment variable for the dataflow
    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env_vars.insert(key.into(), value.into());
    }

    /// Set multiple environment variables
    pub fn set_envs(&mut self, vars: HashMap<String, String>) {
        self.env_vars.extend(vars);
    }

    /// Check if all required env vars are set
    pub fn check_env_requirements(&self) -> Vec<String> {
        let mut missing = Vec::new();
        if let Some(parsed) = &self.parsed {
            for req in &parsed.env_requirements {
                if req.required && !self.env_vars.contains_key(&req.key) && std::env::var(&req.key).is_err() {
                    missing.push(req.key.clone());
                }
            }
        }
        missing
    }

    /// Ensure dora daemon is running
    pub fn ensure_daemon(&mut self) -> BridgeResult<()> {
        // Check if daemon is already running by using `dora list`
        // If it succeeds, daemon is running
        let status = Command::new("dora")
            .arg("list")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match status {
            Ok(s) if s.success() => {
                debug!("Dora daemon already running");
                Ok(())
            }
            _ => {
                info!("Starting dora daemon...");
                let child = Command::new("dora")
                    .arg("up")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| {
                        BridgeError::StartFailed(format!("Failed to start daemon: {}", e))
                    })?;

                self.daemon_process = Some(child);

                // Wait for daemon to be ready
                std::thread::sleep(Duration::from_millis(1000));
                Ok(())
            }
        }
    }

    /// Start the dataflow
    pub fn start(&mut self) -> BridgeResult<String> {
        // Check current state
        {
            let state = self.state.read();
            if state.is_running() {
                return Err(BridgeError::DataflowAlreadyRunning);
            }
        }

        // Update state
        *self.state.write() = DataflowState::Starting;

        // Ensure daemon is running
        self.ensure_daemon()?;

        // Check env requirements
        let missing = self.check_env_requirements();
        if !missing.is_empty() {
            let msg = format!("Missing required env vars: {}", missing.join(", "));
            *self.state.write() = DataflowState::Error {
                message: msg.clone(),
            };
            return Err(BridgeError::StartFailed(msg));
        }

        // Build command - run from dataflow's directory with just the filename
        let dataflow_dir = self
            .dataflow_path
            .parent()
            .ok_or_else(|| BridgeError::StartFailed("Invalid dataflow path".to_string()))?;

        let mut cmd = Command::new("dora");
        cmd.arg("start")
            // Use the absolute path so dora always resolves node paths relative to
            // the actual dataflow file location.
            .arg(&self.dataflow_path)
            .arg("--detach")
            .current_dir(dataflow_dir);

        // Add environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        // Execute
        eprintln!("[Controller] Executing: dora start {:?} --detach", self.dataflow_path);
        info!("Starting dataflow: {:?}", self.dataflow_path);
        let output = cmd.output().map_err(|e| {
            eprintln!("[Controller] FAILED to execute dora: {}", e);
            BridgeError::StartFailed(format!("Failed to execute dora start: {}", e))
        })?;

        eprintln!("[Controller] dora start exit status: {:?}", output.status);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = format!("Dora start failed: {}", stderr);
            eprintln!("[Controller] {}", msg);
            error!("{}", msg);
            *self.state.write() = DataflowState::Error {
                message: msg.clone(),
            };
            return Err(BridgeError::StartFailed(msg));
        }

        // Parse dataflow ID from output (check both stdout and stderr - dora outputs to stderr)
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("[Controller] dora stdout: {}", stdout);
        eprintln!("[Controller] dora stderr: {}", stderr);

        let dataflow_id = Self::parse_dataflow_id(&stderr)
            .or_else(|| Self::parse_dataflow_id(&stdout))
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        eprintln!("[Controller] Dataflow started with ID: {}", dataflow_id);
        info!("Dataflow started with ID: {}", dataflow_id);

        // Update state
        *self.state.write() = DataflowState::Running {
            started_at: Instant::now(),
            dataflow_id: dataflow_id.clone(),
        };

        Ok(dataflow_id)
    }

    /// Stop the dataflow gracefully (default 15s grace period)
    pub fn stop(&mut self) -> BridgeResult<()> {
        self.stop_with_options(None)
    }

    /// Stop the dataflow with a custom grace duration
    ///
    /// After the grace duration, nodes that haven't stopped will be killed (SIGKILL).
    ///
    /// # Arguments
    /// * `grace_duration` - How long to wait before killing. None uses dora's default (15s).
    pub fn stop_with_grace_duration(&mut self, grace_duration: Duration) -> BridgeResult<()> {
        self.stop_with_options(Some(grace_duration))
    }

    /// Force stop the dataflow immediately (0s grace period)
    ///
    /// This will immediately kill all nodes without waiting for graceful shutdown.
    pub fn force_stop(&mut self) -> BridgeResult<()> {
        self.stop_with_options(Some(Duration::from_secs(0)))
    }

    /// Stop the dataflow with options
    fn stop_with_options(&mut self, grace_duration: Option<Duration>) -> BridgeResult<()> {
        let dataflow_id = {
            let state = self.state.read();
            match &*state {
                DataflowState::Running { dataflow_id, .. } => dataflow_id.clone(),
                DataflowState::Stopped => return Ok(()),
                _ => {
                    return Err(BridgeError::DataflowNotRunning);
                }
            }
        };

        *self.state.write() = DataflowState::Stopping;

        let grace_str = grace_duration
            .map(|d| format!("{}s", d.as_secs()))
            .unwrap_or_else(|| "default".to_string());
        info!("Stopping dataflow: {} (grace: {})", dataflow_id, grace_str);

        // Build dora stop command
        let mut cmd = Command::new("dora");
        cmd.arg("stop").arg(&dataflow_id);

        // Add grace duration if specified
        if let Some(duration) = grace_duration {
            cmd.arg("--grace-duration")
                .arg(format!("{}s", duration.as_secs()));
        }

        // Execute dora stop
        let output = cmd
            .output()
            .map_err(|e| BridgeError::StopFailed(format!("Failed to execute dora stop: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Dora stop warning: {}", stderr);
            // Continue anyway - the dataflow might already be stopped
        }

        *self.state.write() = DataflowState::Stopped;
        info!("Dataflow stopped");

        Ok(())
    }

    /// Get dataflow status
    pub fn get_status(&self) -> BridgeResult<DataflowStatus> {
        let state = self.state.read().clone();

        match state {
            DataflowState::Running {
                ref dataflow_id,
                ref started_at,
            } => {
                // Query dora for node status
                let output = Command::new("dora")
                    .arg("list")
                    .output()
                    .map_err(|e| BridgeError::Unknown(format!("Failed to query status: {}", e)))?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                let is_running = stdout.contains(dataflow_id);
                let uptime = started_at.elapsed();

                Ok(DataflowStatus {
                    state: if is_running {
                        DataflowState::Running {
                            dataflow_id: dataflow_id.clone(),
                            started_at: *started_at,
                        }
                    } else {
                        DataflowState::Stopped
                    },
                    uptime: Some(uptime),
                    node_count: self.parsed.as_ref().map(|p| p.nodes.len()).unwrap_or(0),
                    mofa_node_count: self
                        .parsed
                        .as_ref()
                        .map(|p| p.mofa_nodes.len())
                        .unwrap_or(0),
                })
            }
            other => Ok(DataflowStatus {
                state: other,
                uptime: None,
                node_count: self.parsed.as_ref().map(|p| p.nodes.len()).unwrap_or(0),
                mofa_node_count: self
                    .parsed
                    .as_ref()
                    .map(|p| p.mofa_nodes.len())
                    .unwrap_or(0),
            }),
        }
    }

    /// Parse dataflow ID from dora start output
    fn parse_dataflow_id(output: &str) -> Option<String> {
        // Look for UUID pattern in output
        for line in output.lines() {
            if let Some(id) = line
                .split_whitespace()
                .find(|s| s.len() == 36 && s.chars().filter(|c| *c == '-').count() == 4)
            {
                return Some(id.to_string());
            }
        }
        None
    }
}

impl Drop for DataflowController {
    fn drop(&mut self) {
        // Try to stop the dataflow if running
        if self.state.read().is_running() {
            if let Err(e) = self.stop() {
                error!("Failed to stop dataflow on drop: {}", e);
            }
        }

        // Kill daemon if we started it
        if let Some(mut daemon) = self.daemon_process.take() {
            let _ = daemon.kill();
        }
    }
}

/// Dataflow status information
#[derive(Debug, Clone)]
pub struct DataflowStatus {
    pub state: DataflowState,
    pub uptime: Option<Duration>,
    pub node_count: usize,
    pub mofa_node_count: usize,
}
