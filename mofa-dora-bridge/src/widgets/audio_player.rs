//! Audio Player Bridge
//!
//! Connects to dora as `mofa-audio-player` dynamic node.
//! Receives audio from TTS nodes and provides:
//! - Audio samples to the widget for playback
//! - Buffer status output back to dora
//! - Participant audio levels for LED visualization
//!
//! # Human Speech Interrupt
//!
//! When a human starts speaking, the system needs to immediately stop AI audio playback.
//! This is handled through two mechanisms:
//!
//! ## 1. Instant Audio Mute (Force Mute)
//!
//! The audio callback runs on its own thread and reads from a circular buffer.
//! To achieve instant silencing without waiting for UI polling:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                     Instant Mute Flow                               │
//! │                                                                     │
//! │  1. Human speaks → mic-input sends speech_started                   │
//! │  2. Controller receives → sends reset to audio-player               │
//! │  3. Bridge receives reset → calls SharedDoraState.audio.signal_clear()
//! │  4. signal_clear() sets force_mute = true (atomic store)            │
//! │  5. Audio callback checks force_mute → outputs silence immediately  │
//! │                                                                     │
//! │  Latency: < 1ms (next audio callback frame)                         │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The `force_mute` flag is an `Arc<AtomicBool>` shared between:
//! - `AudioPlayer` (UI component) - creates and owns the flag
//! - `SharedDoraState.AudioState` - registered via `register_force_mute()`
//! - Audio callback thread - checks flag before each buffer read
//!
//! ## 2. Smart Reset (Question ID Filtering)
//!
//! After a reset, stale audio chunks (from the previous question) may still be
//! in-flight in the Dora pipeline. Playing these would cause brief "garbled" audio.
//!
//! Smart reset prevents this by filtering incoming audio by `question_id`:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                     Smart Reset Flow                                │
//! │                                                                     │
//! │  State: AI speaking (question_id=5)                                 │
//! │                                                                     │
//! │  1. Human interrupts                                                │
//! │  2. Controller sends reset with question_id=6 (new question)        │
//! │  3. Audio player:                                                   │
//! │     a. Clears buffer (instant silence via force_mute)               │
//! │     b. Sets filtering_mode = true                                   │
//! │     c. Sets reset_question_id = "6"                                 │
//! │                                                                     │
//! │  4. Stale audio arrives (question_id=5)                             │
//! │     → REJECTED (doesn't match reset_question_id)                    │
//! │                                                                     │
//! │  5. New audio arrives (question_id=6)                               │
//! │     → ACCEPTED, exits filtering_mode                                │
//! │     → Normal playback resumes                                       │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Reset Types
//!
//! | Reset Type | question_id | Behavior |
//! |------------|-------------|----------|
//! | Full Reset | None        | Clear buffer, no filtering |
//! | Smart Reset| Present     | Clear buffer + filter by question_id |
//!
//! # Comparison with Python Implementation
//!
//! This implementation matches the Python `audio_player.py` from the conference example:
//!
//! | Feature | Python | Rust |
//! |---------|--------|------|
//! | Instant mute | Direct buffer.reset() | force_mute AtomicBool |
//! | Filtering mode | filtering_mode bool | filtering_mode bool |
//! | Question ID tracking | reset_question_id | reset_question_id |
//! | Stale audio rejection | continue (skip) | return (skip) |
//!
//! The key difference is that Python's audio player IS the Dora node (direct event handling),
//! while Rust uses a bridge pattern with SharedDoraState for UI communication.
//! The `force_mute` mechanism compensates for this by providing direct atomic access
//! to the audio callback thread.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         Dora Dataflow                               │
//! │  ┌──────────┐    ┌────────────┐    ┌─────────────────┐              │
//! │  │ TTS Node │───▶│ audio_*    │───▶│ mofa-audio-player│             │
//! │  └──────────┘    └────────────┘    │ (this bridge)   │              │
//! │                                    └────────┬────────┘              │
//! │  ┌──────────┐    ┌────────────┐             │                       │
//! │  │Controller│───▶│ reset      │─────────────┘                       │
//! │  └──────────┘    └────────────┘                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//!                                      │
//!                                      ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                       SharedDoraState                               │
//! │  ┌──────────────────────────────────────────────────────────────┐   │
//! │  │ AudioState                                                   │   │
//! │  │  • chunks: RwLock<VecDeque<AudioData>>  (pending audio)      │   │
//! │  │  • should_clear: AtomicBool             (UI polling signal)  │   │
//! │  │  • force_mute_flag: Arc<AtomicBool>     (instant mute)       │   │
//! │  └──────────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────────┘
//!                                      │
//!                                      ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         UI (Makepad)                                │
//! │  ┌──────────────────────────────────────────────────────────────┐   │
//! │  │ AudioPlayer                                                  │   │
//! │  │  • force_mute: Arc<AtomicBool>  ←── shared with AudioState   │   │
//! │  │  • circular_buffer: CircularAudioBuffer                      │   │
//! │  │  • audio_callback: checks force_mute before reading          │   │
//! │  └──────────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use crate::bridge::{BridgeState, DoraBridge};
use crate::data::{AudioData, DoraData, EventMetadata};
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
use tracing::{debug, error, info, warn};

// NOTE: LED visualization (band levels) is calculated in screen.rs from output waveform
// This is more accurate since it reflects what's actually being played,
// not what's being received (which may be buffered ahead of playback)

/// Audio player bridge - receives audio from dora, provides to widget
///
/// Status updates (connected/disconnected/error) are communicated via SharedDoraState.
/// Audio data is pushed directly to SharedDoraState.audio for UI consumption.
pub struct AudioPlayerBridge {
    /// Node ID (e.g., "mofa-audio-player")
    node_id: String,
    /// Current state
    state: Arc<RwLock<BridgeState>>,
    /// Shared state for direct UI communication
    shared_state: Option<Arc<SharedDoraState>>,
    /// Buffer status sender from widget
    buffer_status_sender: Sender<f64>,
    /// Buffer status receiver for dora
    buffer_status_receiver: Receiver<f64>,
    /// Stop signal
    stop_sender: Option<Sender<()>>,
    /// Worker thread handle
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl AudioPlayerBridge {
    /// Create a new audio player bridge (legacy - without shared state)
    pub fn new(node_id: &str) -> Self {
        Self::with_shared_state(node_id, None)
    }

    /// Create a new audio player bridge with shared state
    pub fn with_shared_state(node_id: &str, shared_state: Option<Arc<SharedDoraState>>) -> Self {
        let (buffer_tx, buffer_rx) = bounded(10);

        Self {
            node_id: node_id.to_string(),
            state: Arc::new(RwLock::new(BridgeState::Disconnected)),
            shared_state,
            buffer_status_sender: buffer_tx,
            buffer_status_receiver: buffer_rx,
            stop_sender: None,
            worker_handle: None,
        }
    }

    /// Send buffer status back to dora (widget calls this)
    pub fn send_buffer_status(&self, fill_percentage: f64) -> BridgeResult<()> {
        self.buffer_status_sender
            .send(fill_percentage)
            .map_err(|_| BridgeError::ChannelSendError)
    }

    /// Run the dora event loop in background thread
    fn run_event_loop(
        node_id: String,
        state: Arc<RwLock<BridgeState>>,
        shared_state: Option<Arc<SharedDoraState>>,
        buffer_status_receiver: Receiver<f64>,
        stop_receiver: Receiver<()>,
    ) {
        info!("Starting audio player bridge event loop for {}", node_id);

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

        // Session tracking - track which question_ids we've sent session_start for
        // to avoid flooding the controller with duplicate signals
        let mut session_start_sent_for: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // Active participant tracking for LED visualization
        let mut active_participant: Option<String> = None;
        let mut active_switch_for: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // Smart reset state (matches Python audio_player.py)
        // When reset arrives with question_id, we enter filtering_mode
        // and reject audio chunks until we receive one with matching question_id
        let mut filtering_mode = false;
        let mut reset_question_id: Option<String> = None;

        // Event loop
        loop {
            // Check for stop signal
            if stop_receiver.try_recv().is_ok() {
                info!("Audio player bridge received stop signal");
                break;
            }

            // Forward buffer status from UI's AudioPlayer to dora
            // The actual buffer fill percentage comes from CircularAudioBuffer::fill_percentage()
            // in the UI layer, sent here via channel every 50ms
            while let Ok(status) = buffer_status_receiver.try_recv() {
                if let Err(e) = Self::send_buffer_status_to_dora(&mut node, status) {
                    warn!("Failed to send buffer status: {}", e);
                } else {
                    debug!("Buffer status: {:.1}%", status);
                }
            }

            // Receive dora events with timeout
            match events.recv_timeout(std::time::Duration::from_millis(100)) {
                Some(event) => {
                    Self::handle_dora_event(
                        event,
                        &mut node,
                        shared_state.as_ref(),
                        &mut session_start_sent_for,
                        &mut active_participant,
                        &mut active_switch_for,
                        &mut filtering_mode,
                        &mut reset_question_id,
                    );
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
        info!("Audio player bridge event loop ended");
    }

    /// Handle a dora event
    fn handle_dora_event(
        event: Event,
        node: &mut DoraNode,
        shared_state: Option<&Arc<SharedDoraState>>,
        session_start_sent_for: &mut std::collections::HashSet<String>,
        active_participant: &mut Option<String>,
        active_switch_for: &mut std::collections::HashSet<String>,
        filtering_mode: &mut bool,
        reset_question_id: &mut Option<String>,
    ) {
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

                // Handle reset input - immediately clear audio buffer (human speaking interrupt)
                // Smart reset: if question_id is provided, filter incoming audio until matching question_id arrives
                if input_id == "reset" {
                    // Extract command from data or metadata
                    let command = if let Some(cmd) = event_meta.get("command") {
                        cmd.to_string()
                    } else {
                        // Try to read from data (StringArray)
                        use arrow::array::AsArray;
                        data.as_string::<i32>()
                            .iter()
                            .filter_map(|s| s)
                            .next()
                            .map(|s| s.to_string())
                            .unwrap_or_default()
                    };

                    if command == "cancel" || command == "reset" {
                        // Extract question_id for smart reset
                        let new_question_id = event_meta.get("question_id").map(|s| s.to_string());

                        if let Some(ref qid) = new_question_id {
                            // Smart reset - clear buffer and enter filtering mode
                            info!("🔇 Audio player SMART RESET: clearing buffer, filtering for question_id={}", qid);

                            // Signal UI to clear its circular buffer (with force_mute)
                            if let Some(ss) = shared_state {
                                ss.audio.signal_clear();
                            }

                            // Enable filtering mode - reject audio until matching question_id arrives
                            *filtering_mode = true;
                            *reset_question_id = Some(qid.clone());

                            // Clear session tracking
                            session_start_sent_for.clear();
                            active_switch_for.clear();
                            *active_participant = None;
                        } else {
                            // Full reset - clear everything without filtering
                            info!("🔇 Audio player FULL RESET: clearing buffer (no question_id)");

                            // Signal UI to clear its circular buffer
                            if let Some(ss) = shared_state {
                                ss.audio.signal_clear();
                            }

                            // Disable filtering mode
                            *filtering_mode = false;
                            *reset_question_id = None;

                            // Clear session tracking
                            session_start_sent_for.clear();
                            active_switch_for.clear();
                            *active_participant = None;
                        }
                    }
                    return; // Don't process reset as audio
                }

                // Handle audio inputs
                if input_id.contains("audio") {
                    if let Some(audio_data) = Self::extract_audio(&data, &event_meta) {
                        let sample_count = audio_data.samples.len();

                        // Extract participant ID from input_id (e.g., "audio_student1" -> "student1")
                        let participant_id = input_id
                            .strip_prefix("audio_")
                            .unwrap_or("unknown")
                            .to_string();

                        // Get question_id from metadata
                        let question_id = event_meta.get("question_id");

                        // Smart reset filtering: reject stale audio until matching question_id arrives
                        if *filtering_mode {
                            let incoming_qid = question_id.map(|s| s.to_string());
                            let expected_qid = reset_question_id.as_ref();

                            match (&incoming_qid, expected_qid) {
                                (Some(incoming), Some(expected)) if incoming == expected => {
                                    // First chunk with matching question_id - exit filtering mode
                                    *filtering_mode = false;
                                    info!(
                                        "✅ Exiting filtering mode: received matching question_id={} from {}",
                                        incoming, participant_id
                                    );
                                }
                                (Some(incoming), Some(expected)) => {
                                    // Reject stale audio - question_id doesn't match
                                    debug!(
                                        "🚫 Filtering out stale audio from {} (question_id={}, expected={})",
                                        participant_id, incoming, expected
                                    );
                                    return; // Skip this audio chunk
                                }
                                _ => {
                                    // No question_id in audio or reset - assume new content, exit filtering
                                    *filtering_mode = false;
                                    debug!("Exiting filtering mode: no question_id available");
                                }
                            }
                        }

                        debug!(
                            "Received audio: {} samples, {}Hz from {}",
                            sample_count, audio_data.sample_rate, input_id
                        );

                        // Send session_start ONCE per question_id on FIRST audio chunk
                        // (matching conference-dashboard behavior: send on first audio OR when session_status="started")
                        // This marks when audio playback begins for a new LLM/TTS response
                        // The controller waits for this signal to advance to the next speaker
                        if let Some(qid) = question_id {
                            // Only send if we haven't sent for this question_id yet
                            if !session_start_sent_for.contains(qid) {
                                if let Err(e) =
                                    Self::send_session_start(node, input_id, &event_meta)
                                {
                                    warn!("Failed to send session_start: {}", e);
                                } else {
                                    info!(
                                        "Session started for question_id={} (first audio chunk)",
                                        qid
                                    );
                                    session_start_sent_for.insert(qid.to_string());

                                    // Keep the set size bounded (only track last 100 question_ids)
                                    if session_start_sent_for.len() > 100 {
                                        // Remove oldest entries (approximation)
                                        let to_remove: Vec<_> = session_start_sent_for
                                            .iter()
                                            .take(50)
                                            .cloned()
                                            .collect();
                                        for key in to_remove {
                                            session_start_sent_for.remove(&key);
                                        }
                                    }
                                }
                            }

                            // Switch active speaker ONCE per question_id (for logging)
                            if !active_switch_for.contains(qid) {
                                if active_participant.as_ref() != Some(&participant_id) {
                                    *active_participant = Some(participant_id.clone());
                                    debug!("Active speaker changed to: {}", participant_id);
                                }
                                active_switch_for.insert(qid.to_string());

                                // Keep the set size bounded
                                if active_switch_for.len() > 100 {
                                    let to_remove: Vec<_> =
                                        active_switch_for.iter().take(50).cloned().collect();
                                    for key in to_remove {
                                        active_switch_for.remove(&key);
                                    }
                                }
                            }
                        }

                        // NOTE: LED visualization is calculated in screen.rs from output waveform
                        // (more accurate since it reflects what's actually being played)
                        // The bridge only tracks active speaker for session management

                        // IMPORTANT: Override participant_id from input_id (more reliable than metadata)
                        // input_id is "audio_student1" -> participant_id is "student1"
                        // Also ensure question_id is set for smart reset support
                        let mut audio_data_with_participant = audio_data.clone();
                        audio_data_with_participant.participant_id = Some(participant_id.clone());
                        if let Some(qid) = question_id {
                            audio_data_with_participant.question_id = Some(qid.to_string());
                        }

                        // Push audio to SharedDoraState for UI consumption
                        // AudioState.push() uses a ring buffer internally
                        if let Some(ss) = shared_state {
                            ss.audio.push(audio_data_with_participant.clone());
                        }

                        // Send audio_complete signal back to text-segmenter
                        // This allows the next segment to be released
                        // CRITICAL: This must be sent for every audio chunk to keep the pipeline flowing
                        if let Err(e) = Self::send_audio_complete(node, input_id, &event_meta) {
                            warn!("Failed to send audio_complete: {}", e);
                        } else {
                            debug!(
                                "Sent audio_complete for {} (qid={:?})",
                                input_id,
                                event_meta.get("question_id")
                            );
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

    // NOTE: Audio level and band calculation removed - now done in screen.rs from output waveform
    // This is more accurate since it reflects what's actually being played,
    // not what's being received (which may be buffered ahead of playback)

    /// Send audio_complete signal to notify text-segmenter that audio was received
    /// Matches conference-dashboard's implementation for compatibility
    fn send_audio_complete(
        node: &mut DoraNode,
        input_id: &str,
        metadata: &EventMetadata,
    ) -> BridgeResult<()> {
        use std::collections::BTreeMap;

        // Extract participant from input_id (e.g., "audio_student1" -> "student1")
        let participant = input_id.strip_prefix("audio_").unwrap_or(input_id);

        // Build metadata with participant info (matching conference-dashboard format)
        let mut params: BTreeMap<String, Parameter> = BTreeMap::new();
        params.insert(
            "participant".to_string(),
            Parameter::String(participant.to_string()),
        );

        // Include question_id if present in incoming metadata
        if let Some(qid) = metadata.get("question_id") {
            params.insert(
                "question_id".to_string(),
                Parameter::String(qid.to_string()),
            );
        }

        // Include session_status if present in incoming metadata
        if let Some(status) = metadata.get("session_status") {
            params.insert(
                "session_status".to_string(),
                Parameter::String(status.to_string()),
            );
        }

        // Use vec!["received"] format to match conference-dashboard
        let data = vec!["received".to_string()].into_arrow();
        let output_id: DataId = "audio_complete".to_string().into();

        debug!(
            "Sending audio_complete for participant: {} (question_id={:?}, session_status={:?})",
            participant,
            metadata.get("question_id"),
            metadata.get("session_status")
        );

        node.send_output(output_id, params, data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    /// Send session_start signal to notify conference-controller that audio playback has begun
    /// This is critical for the controller to advance to the next speaker
    fn send_session_start(
        node: &mut DoraNode,
        input_id: &str,
        metadata: &EventMetadata,
    ) -> BridgeResult<()> {
        use std::collections::BTreeMap;

        // Extract participant from input_id (e.g., "audio_student1" -> "student1")
        let participant = input_id.strip_prefix("audio_").unwrap_or(input_id);

        // Build metadata (matching conference-dashboard format)
        let mut params: BTreeMap<String, Parameter> = BTreeMap::new();

        // Include question_id - REQUIRED by conference-controller
        if let Some(qid) = metadata.get("question_id") {
            params.insert(
                "question_id".to_string(),
                Parameter::String(qid.to_string()),
            );
        }

        params.insert(
            "participant".to_string(),
            Parameter::String(participant.to_string()),
        );

        params.insert(
            "source".to_string(),
            Parameter::String("mofa-audio-player".to_string()),
        );

        // Include session_status if present
        if let Some(status) = metadata.get("session_status") {
            params.insert(
                "session_status".to_string(),
                Parameter::String(status.to_string()),
            );
        }

        // Use vec!["audio_started"] format to match conference-dashboard
        let data = vec!["audio_started".to_string()].into_arrow();
        let output_id: DataId = "session_start".to_string().into();

        info!(
            "Sending session_start for participant: {} (question_id={:?})",
            participant,
            metadata.get("question_id")
        );

        node.send_output(output_id, params, data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    /// Extract audio data from dora arrow data
    /// Handles multiple formats: Float32, Float64, Int16, ListArray, LargeListArray
    fn extract_audio(
        data: &dora_node_api::ArrowData,
        metadata: &EventMetadata,
    ) -> Option<AudioData> {
        use arrow::array::{Float32Array, Float64Array, Int16Array, LargeListArray, ListArray};
        use arrow::datatypes::DataType;

        let array = &data.0;
        if array.is_empty() {
            return None;
        }

        // Try to extract f32 array
        let samples: Vec<f32> = match array.data_type() {
            DataType::Float32 => array
                .as_any()
                .downcast_ref::<Float32Array>()
                .map(|arr| arr.values().to_vec())?,
            DataType::Float64 => array
                .as_any()
                .downcast_ref::<Float64Array>()
                .map(|arr| arr.values().iter().map(|&x| x as f32).collect())?,
            DataType::Int16 => array
                .as_any()
                .downcast_ref::<Int16Array>()
                .map(|arr| arr.values().iter().map(|&x| x as f32 / 32768.0).collect())?,
            // Handle ListArray<Float32> - primespeech sends pa.array([audio_array])
            DataType::List(_) | DataType::LargeList(_) => {
                debug!("Audio data is ListArray, extracting inner array");

                // Try ListArray first
                if let Some(list_arr) = array.as_any().downcast_ref::<ListArray>() {
                    if list_arr.len() > 0 {
                        let first_value = list_arr.value(0);
                        if let Some(float_arr) = first_value.as_any().downcast_ref::<Float32Array>()
                        {
                            debug!("Extracted {} f32 samples from ListArray", float_arr.len());
                            float_arr.values().to_vec()
                        } else if let Some(float_arr) =
                            first_value.as_any().downcast_ref::<Float64Array>()
                        {
                            debug!("Extracted {} f64 samples from ListArray", float_arr.len());
                            float_arr.values().iter().map(|&v| v as f32).collect()
                        } else {
                            warn!(
                                "ListArray inner type not Float32/Float64: {:?}",
                                first_value.data_type()
                            );
                            return None;
                        }
                    } else {
                        return None;
                    }
                } else if let Some(list_arr) = array.as_any().downcast_ref::<LargeListArray>() {
                    if list_arr.len() > 0 {
                        let first_value = list_arr.value(0);
                        if let Some(float_arr) = first_value.as_any().downcast_ref::<Float32Array>()
                        {
                            debug!(
                                "Extracted {} f32 samples from LargeListArray",
                                float_arr.len()
                            );
                            float_arr.values().to_vec()
                        } else if let Some(float_arr) =
                            first_value.as_any().downcast_ref::<Float64Array>()
                        {
                            debug!(
                                "Extracted {} f64 samples from LargeListArray",
                                float_arr.len()
                            );
                            float_arr.values().iter().map(|&v| v as f32).collect()
                        } else {
                            warn!(
                                "LargeListArray inner type not Float32/Float64: {:?}",
                                first_value.data_type()
                            );
                            return None;
                        }
                    } else {
                        return None;
                    }
                } else {
                    warn!("Failed to extract audio from ListArray");
                    return None;
                }
            }
            dt => {
                warn!("Unsupported audio data type: {:?}", dt);
                return None;
            }
        };

        if samples.is_empty() {
            return None;
        }

        // Get sample rate from metadata or use default
        let sample_rate = metadata
            .get("sample_rate")
            .and_then(|s| s.parse().ok())
            .unwrap_or(32000);

        let participant_id = metadata.participant_id().map(|s| s.to_string());
        let question_id = metadata.get("question_id").map(|s| s.to_string());

        Some(AudioData {
            samples,
            sample_rate,
            channels: 1,
            participant_id,
            question_id,
        })
    }

    /// Send buffer status to dora
    fn send_buffer_status_to_dora(node: &mut DoraNode, status: f64) -> BridgeResult<()> {
        let data = vec![status].into_arrow();
        let output_id: DataId = "buffer_status".to_string().into();
        node.send_output(output_id, Default::default(), data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }
}

impl DoraBridge for AudioPlayerBridge {
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
        let buffer_receiver = self.buffer_status_receiver.clone();

        let handle = thread::spawn(move || {
            Self::run_event_loop(node_id, state, shared_state, buffer_receiver, stop_rx);
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
            ("buffer_status", DoraData::Json(val)) => {
                if let Some(status) = val.as_f64() {
                    // Use try_send to avoid blocking the worker thread if channel is full
                    // Buffer status updates are frequent and non-critical - dropping some is OK
                    if let Err(e) = self.buffer_status_sender.try_send(status) {
                        warn!("Failed to send buffer status update: {}", e);
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
        vec![
            "audio".to_string(),
            "audio_student1".to_string(),
            "audio_student2".to_string(),
            "audio_tutor".to_string(),
        ]
    }

    fn expected_outputs(&self) -> Vec<String> {
        vec![
            "buffer_status".to_string(),
            "status".to_string(),
            "session_start".to_string(),
            "audio_complete".to_string(),
            "log".to_string(),
        ]
    }
}

impl Drop for AudioPlayerBridge {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}
