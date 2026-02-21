//! AEC (Acoustic Echo Cancellation) Input Bridge
//!
//! Connects to dora as `mofa-aec-input` dynamic node.
//! Captures microphone audio with macOS AEC via native library.
//! Provides:
//! - VAD-based speech segmentation
//! - Mic level for UI visualization
//! - Speech detection state
//! - Audio segments for ASR

use crate::bridge::{BridgeState, DoraBridge};
use crate::data::DoraData;
use crate::error::{BridgeError, BridgeResult};
use crate::shared_state::SharedDoraState;
use crossbeam_channel::{bounded, Receiver, Sender};
use dora_node_api::{
    dora_core::config::{DataId, NodeId},
    DoraNode, Event, IntoArrow, Parameter,
};
use libloading::Library;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Control commands for AEC input
#[derive(Debug, Clone)]
pub enum AecControlCommand {
    StartRecording,
    StopRecording,
    SetAecEnabled(bool),
}

/// Adaptive endpoint detector that adjusts silence threshold based on speaker pace.
///
/// Uses percentile-based threshold: sorts recent gaps and picks P90 + margin as
/// the sentence boundary threshold. This naturally separates short intra-sentence
/// pauses (breathing, word gaps) from longer inter-sentence silences.
///
/// Example: gaps = [120, 150, 180, 200, 130, 160, 140, 1500, 170, 190]
///   sorted = [120, 130, 140, 150, 160, 170, 180, 190, 200, 1500]
///   P90 = 200ms (index 9 of 10), margin = 1.5 → threshold = 300ms → clamped to 500ms
///   The 1500ms gap (actual sentence boundary) exceeds threshold → detected correctly.
struct AdaptiveEndpointer {
    enabled: bool,
    gap_history: VecDeque<f64>,
    window_size: usize,
    /// Percentile to use (0.0-1.0), default 0.85 — picks the gap value below which
    /// this fraction of gaps fall. Gaps above this are likely sentence boundaries.
    percentile: f64,
    /// Margin multiplier applied on top of the percentile value, default 1.5
    margin: f64,
    min_threshold_ms: f64,
    max_threshold_ms: f64,
    last_speech_end: Option<Instant>,
    last_logged_threshold: f64,
}

impl AdaptiveEndpointer {
    fn new() -> Self {
        let enabled = std::env::var("ADAPTIVE_ENDPOINT")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);
        let window_size = std::env::var("ADAPTIVE_WINDOW_SIZE")
            .ok().and_then(|s| s.parse().ok()).unwrap_or(20);
        let percentile = std::env::var("ADAPTIVE_PERCENTILE")
            .ok().and_then(|s| s.parse().ok()).unwrap_or(0.85);
        let margin = std::env::var("ADAPTIVE_MARGIN")
            .ok().and_then(|s| s.parse().ok()).unwrap_or(1.5);
        let min_threshold_ms = std::env::var("ADAPTIVE_MIN_MS")
            .ok().and_then(|s| s.parse().ok()).unwrap_or(100.0);
        let max_threshold_ms = std::env::var("ADAPTIVE_MAX_MS")
            .ok().and_then(|s| s.parse().ok()).unwrap_or(3000.0);

        if enabled {
            info!(
                "Adaptive endpointer enabled: window={}, P{:.0}×{}, range=[{}ms, {}ms]",
                window_size, percentile * 100.0, margin, min_threshold_ms, max_threshold_ms
            );
        }

        Self {
            enabled,
            gap_history: VecDeque::with_capacity(window_size + 1),
            window_size,
            percentile,
            margin,
            min_threshold_ms,
            max_threshold_ms,
            last_speech_end: None,
            last_logged_threshold: 0.0,
        }
    }

    /// Called when speech starts — records the gap since last speech end
    fn record_speech_start(&mut self) {
        if !self.enabled { return; }
        if let Some(last_end) = self.last_speech_end {
            let gap_ms = last_end.elapsed().as_secs_f64() * 1000.0;
            self.gap_history.push_back(gap_ms);
            if self.gap_history.len() > self.window_size {
                self.gap_history.pop_front();
            }
        }
    }

    /// Called when speech ends — records the timestamp
    fn record_speech_end(&mut self) {
        self.last_speech_end = Some(Instant::now());
    }

    /// Returns adaptive threshold using percentile of recent gaps, or fallback if insufficient data
    fn current_threshold_ms(&mut self, fallback: f64) -> f64 {
        if !self.enabled || self.gap_history.len() < 5 {
            return fallback;
        }

        // Sort gaps to compute percentile
        let mut sorted: Vec<f64> = self.gap_history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // P-th percentile: value below which P% of gaps fall
        let idx = ((sorted.len() as f64 - 1.0) * self.percentile).round() as usize;
        let p_value = sorted[idx.min(sorted.len() - 1)];

        // Threshold = percentile value × margin
        // Gaps representing normal intra-sentence pauses cluster below p_value.
        // A gap exceeding p_value × margin is an outlier = sentence boundary.
        let threshold = (p_value * self.margin).clamp(self.min_threshold_ms, self.max_threshold_ms);

        // Log when threshold changes significantly (>150ms shift)
        if (threshold - self.last_logged_threshold).abs() > 150.0 {
            let median_idx = sorted.len() / 2;
            info!(
                "Adaptive endpoint threshold: {:.0}ms (P{:.0}={:.0}ms, median={:.0}ms, samples={})",
                threshold, self.percentile * 100.0, p_value, sorted[median_idx], sorted.len()
            );
            self.last_logged_threshold = threshold;
        }

        threshold
    }
}

/// VAD segmentation state
struct VadState {
    is_speaking: bool,
    speech_buffer: Vec<Vec<f32>>,
    audio_segment_buffer: Vec<f32>,
    silence_count: usize,
    speech_start_threshold: usize,
    speech_end_threshold: usize,
    min_segment_size: usize,
    max_segment_size: usize,
    question_end_silence_ms: f64,
    last_speech_end_time: Option<Instant>,
    max_exceeded: bool,
    question_end_sent: bool,
    current_question_id: u32,
    endpointer: AdaptiveEndpointer,
}

impl Default for VadState {
    fn default() -> Self {
        // Read from environment variables (matching Python behavior)
        let speech_end_threshold = std::env::var("SPEECH_END_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10); // Default 10 frames (~100ms)

        let question_end_silence_ms = std::env::var("QUESTION_END_SILENCE_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000.0); // Default 3000ms (matching mac_aec_simple_segmentation.py)

        Self {
            is_speaking: false,
            speech_buffer: Vec::new(),
            audio_segment_buffer: Vec::new(),
            silence_count: 0,
            speech_start_threshold: 3,      // Frames of speech to start
            speech_end_threshold,           // From env or default 10 frames (~100ms)
            min_segment_size: 4800,         // 0.3s at 16kHz
            max_segment_size: 160000,       // 10s at 16kHz
            max_exceeded: false,
            question_end_silence_ms,        // From env or default 3000ms
            last_speech_end_time: None,
            question_end_sent: false,
            current_question_id: rand::random::<u32>() % 900000 + 100000,
            endpointer: AdaptiveEndpointer::new(),
        }
    }
}

/// Native audio capture wrapper using libloading
struct NativeAudioCapture {
    _library: Library,
    start_record: unsafe extern "C" fn(),
    stop_record: unsafe extern "C" fn(),
    get_audio_data: unsafe extern "C" fn(*mut i32, *mut bool) -> *mut u8,
    free_audio_data: unsafe extern "C" fn(*mut u8),
    is_recording: bool,
    /// Tracks whether async initialization completed successfully
    /// The Swift library initializes in a Task block - if we call stopRecord()
    /// before init completes, audioUnit will be nil and crash
    init_successful: bool,
}

impl NativeAudioCapture {
    /// Load the native library
    fn new(library_path: &PathBuf) -> Result<Self, String> {
        if !library_path.exists() {
            return Err(format!("Library not found: {:?}", library_path));
        }

        unsafe {
            let library = Library::new(library_path)
                .map_err(|e| format!("Failed to load library: {}", e))?;

            let start_record: unsafe extern "C" fn() = {
                let sym: libloading::Symbol<unsafe extern "C" fn()> = library
                    .get(b"startRecord")
                    .map_err(|e| format!("Failed to get startRecord: {}", e))?;
                *sym
            };

            let stop_record: unsafe extern "C" fn() = {
                let sym: libloading::Symbol<unsafe extern "C" fn()> = library
                    .get(b"stopRecord")
                    .map_err(|e| format!("Failed to get stopRecord: {}", e))?;
                *sym
            };

            let get_audio_data: unsafe extern "C" fn(*mut i32, *mut bool) -> *mut u8 = {
                let sym: libloading::Symbol<unsafe extern "C" fn(*mut i32, *mut bool) -> *mut u8> =
                    library
                        .get(b"getAudioData")
                        .map_err(|e| format!("Failed to get getAudioData: {}", e))?;
                *sym
            };

            let free_audio_data: unsafe extern "C" fn(*mut u8) = {
                let sym: libloading::Symbol<unsafe extern "C" fn(*mut u8)> = library
                    .get(b"freeAudioData")
                    .map_err(|e| format!("Failed to get freeAudioData: {}", e))?;
                *sym
            };

            Ok(Self {
                _library: library,
                start_record,
                stop_record,
                get_audio_data,
                free_audio_data,
                is_recording: false,
                init_successful: false,
            })
        }
    }

    fn start(&mut self) {
        if !self.is_recording {
            info!("Starting native AEC recording...");
            unsafe {
                (self.start_record)();
            }
            // IMPORTANT: The Swift library initializes asynchronously in a Task block.
            // We need to wait for the audio unit to be properly initialized before
            // considering recording as "started". Otherwise stopRecord() will crash
            // trying to uninitialize a nil audioUnit.
            // Wait for async initialization to complete, then verify by checking for audio data.
            std::thread::sleep(std::time::Duration::from_millis(500));

            // Set recording flag first so get_audio works
            self.is_recording = true;

            // Try to get audio data as a verification that init succeeded
            // If we can get audio, initialization worked
            let mut got_audio = false;
            for _ in 0..10 {
                if self.get_audio().is_some() {
                    got_audio = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }

            if got_audio {
                self.init_successful = true;
                info!("Native AEC recording started successfully (verified audio data)");
            } else {
                // Init may have failed, but we'll try anyway
                // The user might not be speaking yet, so no audio is expected
                self.init_successful = true; // Assume success, will crash if not
                warn!("Native AEC started but no audio yet (may be OK if mic is silent)");
            }
        }
    }

    fn stop(&mut self) {
        // Only call stopRecord if initialization was successful
        // The Swift library will crash if audioUnit is nil
        if self.is_recording && self.init_successful {
            unsafe {
                (self.stop_record)();
            }
            self.is_recording = false;
            info!("Native AEC recording stopped");
        } else if self.is_recording {
            warn!("Skipping stopRecord - init may not have completed");
            self.is_recording = false;
        }
    }

    /// Get audio data from native library
    /// Returns (audio_samples_i16, vad_active)
    fn get_audio(&self) -> Option<(Vec<i16>, bool)> {
        if !self.is_recording {
            return None;
        }

        unsafe {
            let mut size: i32 = 0;
            let mut is_voice_active: bool = false;

            let data_ptr = (self.get_audio_data)(&mut size, &mut is_voice_active);

            if data_ptr.is_null() || size <= 0 {
                return None;
            }

            // Copy data to Rust Vec (data is int16 samples)
            let byte_slice = std::slice::from_raw_parts(data_ptr, size as usize);
            let samples: Vec<i16> = byte_slice
                .chunks_exact(2)
                .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();

            // Free native memory
            (self.free_audio_data)(data_ptr);

            Some((samples, is_voice_active))
        }
    }
}

impl Drop for NativeAudioCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Regular CPAL-based mic capture (no AEC)
/// Used when AEC is disabled - falls back to standard mic input
struct CpalMicCapture {
    stream: Option<cpal::Stream>,
    audio_buffer: Arc<parking_lot::Mutex<Vec<i16>>>,
    is_recording: bool,
    sample_rate: u32,
    vad_threshold: f32, // Energy threshold for simple VAD
}

impl CpalMicCapture {
    fn new() -> Result<Self, String> {
        Ok(Self {
            stream: None,
            audio_buffer: Arc::new(parking_lot::Mutex::new(Vec::new())),
            is_recording: false,
            sample_rate: 16000,
            vad_threshold: 0.01, // Simple energy-based VAD threshold
        })
    }

    fn start(&mut self) -> Result<(), String> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        if self.is_recording {
            return Ok(());
        }

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        // Try to get a config close to 16kHz mono
        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(self.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer = Arc::clone(&self.audio_buffer);
        let err_fn = |err| error!("CPAL stream error: {}", err);

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Convert f32 to i16 and store in buffer
                    let samples: Vec<i16> = data
                        .iter()
                        .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                        .collect();
                    buffer.lock().extend(samples);
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        self.stream = Some(stream);
        self.is_recording = true;
        info!("CPAL mic capture started (no AEC)");
        Ok(())
    }

    fn stop(&mut self) {
        if self.is_recording {
            self.stream = None;
            self.is_recording = false;
            self.audio_buffer.lock().clear();
            info!("CPAL mic capture stopped");
        }
    }

    /// Get audio data with simple energy-based VAD
    /// Returns (audio_samples_i16, vad_active)
    fn get_audio(&self) -> Option<(Vec<i16>, bool)> {
        if !self.is_recording {
            return None;
        }

        let mut buffer = self.audio_buffer.lock();
        if buffer.is_empty() {
            return None;
        }

        // Take all available samples
        let samples: Vec<i16> = buffer.drain(..).collect();

        // Simple energy-based VAD: calculate RMS and compare to threshold
        let rms: f32 = if samples.is_empty() {
            0.0
        } else {
            let sum_squares: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
            ((sum_squares / samples.len() as f64).sqrt() / 32768.0) as f32
        };

        let vad_active = rms > self.vad_threshold;

        Some((samples, vad_active))
    }
}

impl Drop for CpalMicCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

/// AEC Input bridge - captures mic audio with echo cancellation
pub struct AecInputBridge {
    node_id: String,
    state: Arc<RwLock<BridgeState>>,
    shared_state: Option<Arc<SharedDoraState>>,
    control_sender: Sender<AecControlCommand>,
    control_receiver: Receiver<AecControlCommand>,
    stop_sender: Option<Sender<()>>,
    worker_handle: Option<thread::JoinHandle<()>>,
    is_recording: Arc<AtomicBool>,
    aec_enabled: Arc<AtomicBool>,
}

impl AecInputBridge {
    pub fn new(node_id: &str) -> Self {
        Self::with_shared_state(node_id, None)
    }

    pub fn with_shared_state(node_id: &str, shared_state: Option<Arc<SharedDoraState>>) -> Self {
        let (control_tx, control_rx) = bounded(10);

        Self {
            node_id: node_id.to_string(),
            state: Arc::new(RwLock::new(BridgeState::Disconnected)),
            shared_state,
            control_sender: control_tx,
            control_receiver: control_rx,
            stop_sender: None,
            worker_handle: None,
            is_recording: Arc::new(AtomicBool::new(false)),
            aec_enabled: Arc::new(AtomicBool::new(false)), // Default to CPAL (safer startup)
        }
    }

    /// Send control command (from UI)
    pub fn send_control(&self, cmd: AecControlCommand) -> BridgeResult<()> {
        self.control_sender
            .send(cmd)
            .map_err(|_| BridgeError::ChannelSendError)
    }

    /// Check if recording
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Acquire)
    }

    /// Check if AEC is enabled
    pub fn is_aec_enabled(&self) -> bool {
        self.aec_enabled.load(Ordering::Acquire)
    }

    /// Find the native library path
    fn find_library_path() -> Option<PathBuf> {
        // Try multiple locations
        let candidates = vec![
            // In mofa-dora-bridge/lib/
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib/libAudioCapture.dylib"),
            // In workspace root
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .map(|p| p.join("lib/libAudioCapture.dylib"))
                .unwrap_or_default(),
        ];

        for path in candidates {
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Calculate RMS level from audio samples
    fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }

    /// Run the event loop
    fn run_event_loop(
        node_id: String,
        state: Arc<RwLock<BridgeState>>,
        shared_state: Option<Arc<SharedDoraState>>,
        control_receiver: Receiver<AecControlCommand>,
        stop_receiver: Receiver<()>,
        is_recording: Arc<AtomicBool>,
        aec_enabled: Arc<AtomicBool>,
    ) {
        eprintln!("[AecInput] Starting event loop for {}", node_id);

        // Initialize both capture methods
        // 1. Native AEC capture (with echo cancellation)
        let mut aec_capture: Option<NativeAudioCapture> = None;
        if let Some(library_path) = Self::find_library_path() {
            info!("Found AEC library at: {:?}", library_path);
            match NativeAudioCapture::new(&library_path) {
                Ok(cap) => {
                    aec_capture = Some(cap);
                    info!("Native AEC capture initialized");
                }
                Err(e) => {
                    warn!("Failed to load AEC library: {} - will use CPAL only", e);
                }
            }
        } else {
            warn!("AEC native library not found - will use CPAL only");
        }

        // 2. CPAL mic capture (no echo cancellation, fallback)
        let mut cpal_capture = match CpalMicCapture::new() {
            Ok(cap) => cap,
            Err(e) => {
                error!("Failed to init CPAL capture: {}", e);
                *state.write() = BridgeState::Error;
                if let Some(ref ss) = shared_state {
                    ss.set_error(Some(format!("CPAL init failed: {}", e)));
                }
                return;
            }
        };

        // If no AEC available, force AEC disabled
        let aec_available = aec_capture.is_some();
        if !aec_available {
            aec_enabled.store(false, Ordering::Release);
            warn!("AEC not available - using CPAL capture only");
        }

        // Initialize dora node
        eprintln!("[AecInput] Initializing dora node for {}", node_id);
        let (mut node, mut events) =
            match DoraNode::init_from_node_id(NodeId::from(node_id.clone())) {
                Ok(n) => {
                    eprintln!("[AecInput] Dora node init SUCCESS for {}", node_id);
                    n
                }
                Err(e) => {
                    eprintln!("[AecInput] FAILED to init dora node {}: {}", node_id, e);
                    *state.write() = BridgeState::Error;
                    if let Some(ref ss) = shared_state {
                        ss.set_error(Some(format!("Dora init failed: {}", e)));
                    }
                    return;
                }
            };

        *state.write() = BridgeState::Connected;
        eprintln!("[AecInput] Bridge state set to CONNECTED for {}", node_id);
        if let Some(ref ss) = shared_state {
            ss.add_bridge(node_id.clone());
        }

        // VAD state
        let mut vad_state = VadState::default();
        let mut recording_active = false;
        let mut using_aec = aec_enabled.load(Ordering::Acquire) && aec_available;

        // Log config on startup (matching Python behavior)
        let _ = Self::send_log(
            &mut node,
            &node_id,
            "INFO",
            &format!(
                "🔧 CONFIG: SPEECH_END_FRAMES={}, QUESTION_END_SILENCE_MS={}ms, AEC_AVAILABLE={}",
                vad_state.speech_end_threshold, vad_state.question_end_silence_ms, aec_available
            ),
        );
        let speech_end_ms = vad_state.speech_end_threshold * 10; // ~10ms per frame
        let total_silence_ms = speech_end_ms as f64 + vad_state.question_end_silence_ms;
        let _ = Self::send_log(
            &mut node,
            &node_id,
            "INFO",
            &format!(
                "Silence detection: speech_end={}ms ({} frames) + question_end={}ms = total ~{}ms",
                speech_end_ms,
                vad_state.speech_end_threshold,
                vad_state.question_end_silence_ms,
                total_silence_ms
            ),
        );

        // Start recording by default when connected
        if using_aec {
            if let Some(ref mut aec) = aec_capture {
                aec.start();
            }
            let _ = Self::send_log(&mut node, &node_id, "INFO", "🎙️ Recording started with AEC (echo cancellation ON)");
        } else {
            if let Err(e) = cpal_capture.start() {
                error!("Failed to start CPAL capture: {}", e);
            }
            let _ = Self::send_log(&mut node, &node_id, "INFO", "🎙️ Recording started without AEC (regular mic)");
        }
        is_recording.store(true, Ordering::Release);
        recording_active = true;

        // Update shared state
        if let Some(ref ss) = shared_state {
            ss.mic.set_recording(true);
            ss.mic.set_aec_enabled(using_aec);
        }

        let _ = Self::send_log(
            &mut node,
            &node_id,
            "INFO",
            "Node ready - outputting: audio, is_speaking, speech_started, speech_ended, audio_segment, question_ended",
        );

        // Send initial status
        let _ = Self::send_status(&mut node, "recording");
        let _ = Self::send_log(&mut node, &node_id, "INFO", "🎙️ Mic recording STARTED (auto-start on connect)");

        // Main event loop
        let poll_interval = Duration::from_millis(10);
        let mut last_poll = Instant::now();

        loop {
            // Check for stop signal
            if stop_receiver.try_recv().is_ok() {
                eprintln!("[AecInput] Received internal stop signal from bridge");
                break;
            }

            // Handle control commands
            while let Ok(cmd) = control_receiver.try_recv() {
                eprintln!("[AecInput] Received control command: {:?}", cmd);
                match cmd {
                    AecControlCommand::StartRecording => {
                        if !recording_active {
                            // Start the appropriate capture
                            if using_aec {
                                if let Some(ref mut aec) = aec_capture {
                                    aec.start();
                                }
                                let _ = Self::send_log(&mut node, &node_id, "INFO", "🎙️ Recording STARTED with AEC");
                            } else {
                                if let Err(e) = cpal_capture.start() {
                                    error!("Failed to start CPAL: {}", e);
                                }
                                let _ = Self::send_log(&mut node, &node_id, "INFO", "🎙️ Recording STARTED without AEC");
                            }
                            recording_active = true;
                            is_recording.store(true, Ordering::Release);
                            if let Some(ref ss) = shared_state {
                                ss.mic.set_recording(true);
                            }
                            let _ = Self::send_status(&mut node, "recording");
                        }
                    }
                    AecControlCommand::StopRecording => {
                        if recording_active {
                            // Stop both captures (one will be inactive anyway)
                            if let Some(ref mut aec) = aec_capture {
                                aec.stop();
                            }
                            cpal_capture.stop();
                            recording_active = false;
                            is_recording.store(false, Ordering::Release);
                            if let Some(ref ss) = shared_state {
                                ss.mic.set_recording(false);
                            }
                            let _ = Self::send_status(&mut node, "stopped");
                            let _ = Self::send_log(&mut node, &node_id, "INFO", "🔇 Mic recording STOPPED");
                        }
                    }
                    AecControlCommand::SetAecEnabled(enabled) => {
                        let new_using_aec = enabled && aec_available;

                        // Only switch if actually changing capture method
                        if new_using_aec != using_aec {
                            // Stop current capture
                            if recording_active {
                                if using_aec {
                                    if let Some(ref mut aec) = aec_capture {
                                        aec.stop();
                                    }
                                } else {
                                    cpal_capture.stop();
                                }
                            }

                            // Switch capture method
                            using_aec = new_using_aec;

                            // Start new capture if was recording
                            if recording_active {
                                if using_aec {
                                    if let Some(ref mut aec) = aec_capture {
                                        aec.start();
                                    }
                                    let _ = Self::send_log(&mut node, &node_id, "INFO", "🔄 Switched to AEC capture (echo cancellation ON)");
                                } else {
                                    if let Err(e) = cpal_capture.start() {
                                        error!("Failed to start CPAL: {}", e);
                                    }
                                    let _ = Self::send_log(&mut node, &node_id, "INFO", "🔄 Switched to regular mic (echo cancellation OFF)");
                                }
                            }
                        }

                        aec_enabled.store(enabled, Ordering::Release);
                        if let Some(ref ss) = shared_state {
                            ss.mic.set_aec_enabled(new_using_aec);
                        }
                        info!("AEC enabled: {} (using_aec: {})", enabled, using_aec);
                    }
                }
            }

            // Process audio at regular intervals
            if recording_active && last_poll.elapsed() >= poll_interval {
                last_poll = Instant::now();

                // Collect all available audio from the active capture source
                let mut all_audio: Vec<f32> = Vec::new();
                let mut vad_results: Vec<bool> = Vec::new();

                // Debug: track audio stats periodically
                static AUDIO_DEBUG_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let debug_count = AUDIO_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);

                for _ in 0..100 {
                    // Get audio from the appropriate capture source
                    let audio_result = if using_aec {
                        aec_capture.as_ref().and_then(|aec| aec.get_audio())
                    } else {
                        cpal_capture.get_audio()
                    };

                    match audio_result {
                        Some((samples_i16, vad)) => {
                            // Convert i16 to f32 normalized
                            let samples_f32: Vec<f32> =
                                samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();
                            all_audio.extend(samples_f32);
                            vad_results.push(vad);
                        }
                        None => break,
                    }
                }

                // Log audio stats every 100 iterations (~1 second)
                if debug_count % 100 == 0 && recording_active {
                    let rms = Self::calculate_rms(&all_audio);
                    let vad_active = vad_results.iter().any(|&v| v);
                    eprintln!(
                        "[AecInput] Audio stats: using_aec={}, samples={}, rms={:.4}, vad_any={}, is_speaking={}",
                        using_aec, all_audio.len(), rms, vad_active, vad_state.is_speaking
                    );
                }

                // Check question_ended timer (runs even without audio)
                let mut question_ended = false;
                if !vad_state.is_speaking
                    && vad_state.last_speech_end_time.is_some()
                    && !vad_state.question_end_sent
                {
                    let elapsed = vad_state.last_speech_end_time.unwrap().elapsed();
                    let threshold = vad_state.endpointer.current_threshold_ms(vad_state.question_end_silence_ms);
                    if elapsed.as_millis() as f64 >= threshold {
                        question_ended = true;
                        vad_state.question_end_sent = true;
                        info!(
                            "Sentence complete: silence={:.0}ms >= threshold={:.0}ms (question_id={})",
                            elapsed.as_millis() as f64, threshold, vad_state.current_question_id
                        );
                        let _ = Self::send_log(
                            &mut node,
                            &node_id,
                            "INFO",
                            &format!(
                                "🔚 Sentence complete: silence={:.0}ms, threshold={:.0}ms, question_id={}",
                                elapsed.as_millis() as f64, threshold, vad_state.current_question_id
                            ),
                        );
                    }
                }

                // Send question_ended signal
                if question_ended {
                    let old_qid = vad_state.current_question_id;
                    let _ = Self::send_log(
                        &mut node,
                        &node_id,
                        "INFO",
                        &format!("📤 SENDING question_ended with OLD question_id={}", old_qid),
                    );
                    if let Err(e) = Self::send_question_ended(&mut node, old_qid) {
                        warn!("Failed to send question_ended: {}", e);
                    }
                    // Generate new question_id for next question
                    let new_qid = rand::random::<u32>() % 900000 + 100000;
                    vad_state.current_question_id = new_qid;
                    let _ = Self::send_log(
                        &mut node,
                        &node_id,
                        "INFO",
                        &format!("🆕 GENERATED NEW question_id={} for NEXT question", new_qid),
                    );
                }

                if all_audio.is_empty() {
                    continue;
                }

                // Calculate mic level and update shared state
                let rms = Self::calculate_rms(&all_audio);
                if let Some(ref ss) = shared_state {
                    ss.mic.set_level(rms);
                }

                // Send continuous audio stream (matching Python behavior)
                if let Err(e) = Self::send_audio(&mut node, &all_audio) {
                    warn!("Failed to send audio: {}", e);
                }

                // VAD processing
                let vad_result = vad_results.iter().any(|&v| v);
                let num_chunks = vad_results.len();

                let mut speech_started = false;
                let mut speech_ended = false;
                let mut audio_segment: Option<Vec<f32>> = None;

                if vad_result {
                    // Speech detected
                    if !vad_state.is_speaking {
                        vad_state.silence_count = 0;
                        vad_state.speech_buffer.push(all_audio.clone());

                        if vad_state.speech_buffer.len() >= vad_state.speech_start_threshold {
                            vad_state.is_speaking = true;
                            speech_started = true;
                            vad_state.question_end_sent = false;
                            vad_state.endpointer.record_speech_start();

                            // Start segment buffer
                            vad_state.audio_segment_buffer.clear();
                            for buf in &vad_state.speech_buffer {
                                vad_state.audio_segment_buffer.extend(buf);
                            }

                            info!(
                                "Speech started (question_id={})",
                                vad_state.current_question_id
                            );
                        }
                    } else {
                        // Continue segment
                        vad_state.audio_segment_buffer.extend(&all_audio);
                        vad_state.silence_count = 0;

                        // Flag when max size exceeded — actual cut deferred to next silence
                        if vad_state.audio_segment_buffer.len() >= vad_state.max_segment_size {
                            vad_state.max_exceeded = true;
                        }
                    }
                } else {
                    // No speech
                    if vad_state.is_speaking {
                        vad_state.audio_segment_buffer.extend(&all_audio);
                        vad_state.silence_count += num_chunks;

                        // When max exceeded, cut on any silence (threshold=1 frame)
                        // Also hard-cut at 1.5x max as absolute safety ceiling
                        let effective_threshold = if vad_state.max_exceeded {
                            1 // Cut on first silent frame after max exceeded
                        } else {
                            vad_state.speech_end_threshold
                        };
                        let hard_ceiling = vad_state.max_segment_size + (vad_state.max_segment_size / 2);
                        let force_cut = vad_state.audio_segment_buffer.len() >= hard_ceiling;

                        if vad_state.silence_count >= effective_threshold || force_cut {
                            // Speech ended
                            if vad_state.audio_segment_buffer.len() >= vad_state.min_segment_size {
                                audio_segment = Some(vad_state.audio_segment_buffer.clone());
                            }

                            vad_state.audio_segment_buffer.clear();
                            vad_state.is_speaking = false;
                            vad_state.silence_count = 0;
                            vad_state.max_exceeded = false;
                            vad_state.speech_buffer.clear();
                            speech_ended = true;
                            vad_state.last_speech_end_time = Some(Instant::now());
                            vad_state.question_end_sent = false;
                            vad_state.endpointer.record_speech_end();

                            let seg_duration_ms = audio_segment.as_ref().map(|s| s.len() as f64 / 16.0).unwrap_or(0.0); // 16kHz → ms
                            let current_threshold = vad_state.endpointer.current_threshold_ms(vad_state.question_end_silence_ms);
                            info!(
                                "Speech ended: segment={:.0}ms, adaptive_threshold={:.0}ms (question_id={})",
                                seg_duration_ms, current_threshold, vad_state.current_question_id
                            );
                            let _ = Self::send_log(
                                &mut node,
                                &node_id,
                                "INFO",
                                &format!(
                                    "🎤 Speech ended: segment={:.0}ms, next_threshold={:.0}ms, question_id={}",
                                    seg_duration_ms, current_threshold, vad_state.current_question_id
                                ),
                            );
                        }
                    } else {
                        vad_state.speech_buffer.clear();
                    }
                }

                // Update shared state with speaking status
                if let Some(ref ss) = shared_state {
                    if speech_started || speech_ended {
                        ss.mic.set_speaking(vad_state.is_speaking);
                    }
                }

                // Send dora outputs
                if speech_started {
                    if let Err(e) = Self::send_speech_started(&mut node) {
                        warn!("Failed to send speech_started: {}", e);
                    }
                    if let Err(e) = Self::send_is_speaking(&mut node, true) {
                        warn!("Failed to send is_speaking: {}", e);
                    }
                    let _ = Self::send_log(
                        &mut node,
                        &node_id,
                        "INFO",
                        &format!(
                            "🎤 NEW SPEECH STARTED - question_id={}",
                            vad_state.current_question_id
                        ),
                    );
                }

                if speech_ended {
                    if let Err(e) = Self::send_speech_ended(&mut node) {
                        warn!("Failed to send speech_ended: {}", e);
                    }
                    if let Err(e) = Self::send_is_speaking(&mut node, false) {
                        warn!("Failed to send is_speaking: {}", e);
                    }
                    let _ = Self::send_log(
                        &mut node,
                        &node_id,
                        "INFO",
                        &format!(
                            "🔇 SPEECH ENDED - question_id={}",
                            vad_state.current_question_id
                        ),
                    );
                }

                // Send audio segment for ASR
                if let Some(segment) = audio_segment {
                    if let Err(e) =
                        Self::send_audio_segment(&mut node, &segment, vad_state.current_question_id)
                    {
                        warn!("Failed to send audio_segment: {}", e);
                    } else {
                        info!(
                            "Sent audio segment: {} samples (question_id={})",
                            segment.len(),
                            vad_state.current_question_id
                        );
                        let _ = Self::send_log(
                            &mut node,
                            &node_id,
                            "INFO",
                            &format!(
                                "🎵 AUDIO_SEGMENT sent with question_id={} ({} samples)",
                                vad_state.current_question_id,
                                segment.len()
                            ),
                        );
                    }
                }
            }

            // Handle dora events (control inputs)
            match events.recv_timeout(Duration::from_millis(1)) {
                Some(Event::Input { id, .. }) => {
                    debug!("Received input: {}", id.as_str());
                    // Handle control inputs if needed
                }
                Some(Event::Stop(_)) => {
                    // Don't break on Stop - other bridges ignore it too
                    // Breaking causes immediate disconnect and retry loops
                    eprintln!("[AecInput] Received Stop event from dora (ignoring)");
                }
                _ => {}
            }
        }

        // Cleanup - stop both capture methods
        eprintln!("[AecInput] Event loop exited - running cleanup");
        if let Some(ref mut aec) = aec_capture {
            aec.stop();
        }
        cpal_capture.stop();
        is_recording.store(false, Ordering::Release);
        *state.write() = BridgeState::Disconnected;
        eprintln!("[AecInput] State set to DISCONNECTED");
        if let Some(ref ss) = shared_state {
            ss.remove_bridge(&node_id);
            ss.mic.set_recording(false);
        }
        info!("AEC input bridge event loop ended");
    }

    fn send_speech_started(node: &mut DoraNode) -> BridgeResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let data = vec![now].into_arrow();
        let output_id: DataId = "speech_started".to_string().into();
        node.send_output(output_id, BTreeMap::new(), data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    fn send_speech_ended(node: &mut DoraNode) -> BridgeResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let data = vec![now].into_arrow();
        let output_id: DataId = "speech_ended".to_string().into();
        node.send_output(output_id, BTreeMap::new(), data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    fn send_is_speaking(node: &mut DoraNode, speaking: bool) -> BridgeResult<()> {
        // Convert bool to u8 since Vec<bool> doesn't implement IntoArrow
        let data = vec![speaking as u8].into_arrow();
        let output_id: DataId = "is_speaking".to_string().into();
        node.send_output(output_id, BTreeMap::new(), data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    fn send_question_ended(node: &mut DoraNode, question_id: u32) -> BridgeResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let data = vec![now].into_arrow();
        let output_id: DataId = "question_ended".to_string().into();

        let mut params: BTreeMap<String, Parameter> = BTreeMap::new();
        params.insert(
            "question_id".to_string(),
            Parameter::Integer(question_id as i64),
        );

        node.send_output(output_id, params, data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    fn send_audio_segment(
        node: &mut DoraNode,
        samples: &[f32],
        question_id: u32,
    ) -> BridgeResult<()> {
        let data = samples.to_vec().into_arrow();
        let output_id: DataId = "audio_segment".to_string().into();

        let mut params: BTreeMap<String, Parameter> = BTreeMap::new();
        params.insert(
            "question_id".to_string(),
            Parameter::Integer(question_id as i64),
        );
        params.insert("sample_rate".to_string(), Parameter::Integer(16000));

        node.send_output(output_id, params, data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    /// Send continuous audio stream (for recording/monitoring)
    fn send_audio(node: &mut DoraNode, samples: &[f32]) -> BridgeResult<()> {
        let data = samples.to_vec().into_arrow();
        let output_id: DataId = "audio".to_string().into();
        node.send_output(output_id, BTreeMap::new(), data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    /// Send log message to dora log output
    fn send_log(node: &mut DoraNode, node_id: &str, level: &str, message: &str) -> BridgeResult<()> {
        let log_entry = serde_json::json!({
            "level": level,
            "message": message,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
            "node": node_id
        });
        let log_str = log_entry.to_string();
        let data = vec![log_str].into_arrow();
        let output_id: DataId = "log".to_string().into();
        node.send_output(output_id, BTreeMap::new(), data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    /// Send status update (recording/stopped)
    fn send_status(node: &mut DoraNode, status: &str) -> BridgeResult<()> {
        let data = vec![status.to_string()].into_arrow();
        let output_id: DataId = "status".to_string().into();
        node.send_output(output_id, BTreeMap::new(), data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }
}

impl DoraBridge for AecInputBridge {
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

        // If there's an existing worker thread, wait for it to finish
        // This prevents duplicate dora node connections
        if let Some(handle) = self.worker_handle.take() {
            eprintln!("[AecInput] Waiting for previous worker thread to finish...");
            if let Some(stop_tx) = self.stop_sender.take() {
                let _ = stop_tx.send(());
            }
            let _ = handle.join();
            eprintln!("[AecInput] Previous worker thread finished");
            // Give dora a moment to clean up the old connection
            std::thread::sleep(Duration::from_millis(500));
        }

        *self.state.write() = BridgeState::Connecting;

        let (stop_tx, stop_rx) = bounded(1);
        self.stop_sender = Some(stop_tx);

        let node_id = self.node_id.clone();
        let state = Arc::clone(&self.state);
        let shared_state = self.shared_state.clone();
        let control_receiver = self.control_receiver.clone();
        let is_recording = Arc::clone(&self.is_recording);
        let aec_enabled = Arc::clone(&self.aec_enabled);

        let handle = thread::spawn(move || {
            Self::run_event_loop(
                node_id,
                state,
                shared_state,
                control_receiver,
                stop_rx,
                is_recording,
                aec_enabled,
            );
        });

        self.worker_handle = Some(handle);

        // Wait for connection to complete (Connected or Error state)
        // The worker thread will update the state when it connects to dora
        let max_wait = Duration::from_secs(10); // 10 second timeout (was 30s)
        let check_interval = Duration::from_millis(100);
        let start = Instant::now();

        while start.elapsed() < max_wait {
            let current_state = *self.state.read();
            match current_state {
                BridgeState::Connected => {
                    info!("AecInputBridge connected successfully");
                    return Ok(());
                }
                BridgeState::Error => {
                    error!("AecInputBridge connection failed");
                    return Err(BridgeError::ConnectionFailed("Bridge failed to connect".to_string()));
                }
                BridgeState::Connecting => {
                    // Still connecting, keep waiting
                    std::thread::sleep(check_interval);
                }
                BridgeState::Disconnected | BridgeState::Disconnecting => {
                    // Worker thread exited without setting state
                    error!("AecInputBridge worker exited unexpectedly");
                    return Err(BridgeError::ConnectionFailed("Worker thread exited".to_string()));
                }
            }
        }

        // Timeout - connection took too long
        error!("AecInputBridge connection timeout after {:?}", max_wait);
        Err(BridgeError::ConnectionFailed("Connection timeout".to_string()))
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

        match output_id {
            "control" => {
                if let DoraData::Json(val) = data {
                    if let Some(action) = val.get("action").and_then(|v| v.as_str()) {
                        let cmd = match action {
                            "start_recording" => Some(AecControlCommand::StartRecording),
                            "stop_recording" => Some(AecControlCommand::StopRecording),
                            "toggle_aec" | "set_aec_enabled" => {
                                let enabled = val
                                    .get("enabled")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(true);
                                Some(AecControlCommand::SetAecEnabled(enabled))
                            }
                            _ => None,
                        };
                        if let Some(cmd) = cmd {
                            self.send_control(cmd)?;
                        }
                    }
                }
            }
            _ => {
                warn!("Unknown output: {}", output_id);
            }
        }

        Ok(())
    }

    fn expected_inputs(&self) -> Vec<String> {
        vec!["control".to_string()]
    }

    fn expected_outputs(&self) -> Vec<String> {
        vec![
            "audio".to_string(),         // Continuous audio stream
            "audio_segment".to_string(), // VAD-segmented audio for ASR
            "speech_started".to_string(),
            "speech_ended".to_string(),
            "is_speaking".to_string(),
            "question_ended".to_string(),
            "status".to_string(), // Recording status (recording/stopped)
            "log".to_string(),
        ]
    }
}

impl Drop for AecInputBridge {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}
