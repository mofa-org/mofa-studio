//! Audio Player Module - Circular buffer audio playback using cpal
//!
//! Adapted from conference-dashboard for mofa-fm.
//!
//! # Force Mute for Instant Audio Interrupt
//!
//! When a human starts speaking, the AI audio must stop immediately (< 1ms latency).
//! This is achieved through a shared `force_mute: Arc<AtomicBool>` flag:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                     Force Mute Architecture                         │
//! │                                                                     │
//! │  AudioPlayer                                                        │
//! │    │                                                                │
//! │    ├── force_mute: Arc<AtomicBool>  ←─┐                             │
//! │    │                                  │ Shared via                  │
//! │    └── audio_callback() ─────────────┤ register_force_mute()        │
//! │          │                            │                             │
//! │          │ checks force_mute          │                             │
//! │          │ before reading buffer      ▼                             │
//! │          │                    SharedDoraState.AudioState            │
//! │          │                      │                                   │
//! │          ▼                      │ signal_clear() sets               │
//! │    if force_mute == true:       │ force_mute = true                 │
//! │      output silence             │                                   │
//! │    else:                        ▼                                   │
//! │      read from buffer    AudioPlayerBridge (Dora event loop)        │
//! │                                 │                                   │
//! │                                 │ receives reset input              │
//! │                                 │ from controller                   │
//! │                                 ▼                                   │
//! │                          Human speaks → speech_started → reset      │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Setup
//!
//! The UI must register the force_mute flag with SharedDoraState after creating
//! the AudioPlayer:
//!
//! ```rust,ignore
//! // In UI initialization (e.g., init_dora):
//! if let Some(ref player) = self.audio_player {
//!     integration.shared_dora_state().audio.register_force_mute(
//!         player.force_mute_flag()
//!     );
//! }
//! ```
//!
//! ## Audio Callback Behavior
//!
//! The cpal audio callback checks `force_mute` FIRST before reading the buffer:
//!
//! ```rust,ignore
//! move |data: &mut [f32], _| {
//!     // Check force_mute first - instant silencing for human interrupt
//!     if force_mute_clone.load(Ordering::Acquire) {
//!         for sample in data.iter_mut() {
//!             *sample = 0.0;  // Output silence
//!         }
//!         return;
//!     }
//!     // Normal buffer read...
//! }
//! ```
//!
//! ## Reset Clears force_mute
//!
//! The `AudioCommand::Reset` handler clears `force_mute` after resetting the buffer,
//! allowing playback to resume when new audio arrives.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Segment tracking for knowing which participant and question owns audio in the buffer
#[derive(Clone, Debug)]
struct AudioSegment {
    participant_id: Option<String>,
    question_id: Option<String>,
    samples_remaining: usize,
}

/// Commands sent to the audio thread
enum AudioCommand {
    Write(Vec<f32>, Option<String>, Option<String>), // samples, participant_id, question_id
    Reset,
    SmartReset(String), // Keep only segments with this question_id
    Pause,
    Resume,
    Stop,
}

/// Circular audio buffer for thread-safe audio streaming
struct CircularAudioBuffer {
    buffer: Vec<f32>,
    write_pos: usize,
    read_pos: usize,
    available_samples: usize,
    buffer_size: usize,
    segments: VecDeque<AudioSegment>,
    current_playing_participant: Option<String>,
}

impl CircularAudioBuffer {
    fn new(size_seconds: f32, sample_rate: u32) -> Self {
        let buffer_size = (size_seconds * sample_rate as f32) as usize;
        Self {
            buffer: vec![0.0; buffer_size],
            write_pos: 0,
            read_pos: 0,
            available_samples: 0,
            buffer_size,
            segments: VecDeque::new(),
            current_playing_participant: None,
        }
    }

    fn write_with_participant(
        &mut self,
        samples: &[f32],
        participant_id: Option<String>,
        question_id: Option<String>,
    ) -> usize {
        let mut written = 0;
        for &sample in samples {
            if self.available_samples < self.buffer_size {
                self.buffer[self.write_pos] = sample;
                self.write_pos = (self.write_pos + 1) % self.buffer_size;
                self.available_samples += 1;
                written += 1;
            } else {
                // Buffer full - overwrite oldest
                self.buffer[self.write_pos] = sample;
                self.write_pos = (self.write_pos + 1) % self.buffer_size;
                self.read_pos = (self.read_pos + 1) % self.buffer_size;
                if let Some(front) = self.segments.front_mut() {
                    if front.samples_remaining > 0 {
                        front.samples_remaining -= 1;
                    }
                    if front.samples_remaining == 0 {
                        self.segments.pop_front();
                    }
                }
                written += 1;
            }
        }

        if written > 0 {
            // Try to merge with last segment if same participant AND question
            if let Some(last) = self.segments.back_mut() {
                if last.participant_id == participant_id && last.question_id == question_id {
                    last.samples_remaining += written;
                } else {
                    self.segments.push_back(AudioSegment {
                        participant_id,
                        question_id,
                        samples_remaining: written,
                    });
                }
            } else {
                self.segments.push_back(AudioSegment {
                    participant_id,
                    question_id,
                    samples_remaining: written,
                });
            }
        }

        written
    }

    fn read(&mut self, output: &mut [f32]) -> usize {
        let mut read_count = 0;
        for sample in output.iter_mut() {
            if self.available_samples > 0 {
                *sample = self.buffer[self.read_pos];
                self.read_pos = (self.read_pos + 1) % self.buffer_size;
                self.available_samples -= 1;
                read_count += 1;

                if let Some(front) = self.segments.front_mut() {
                    self.current_playing_participant = front.participant_id.clone();
                    if front.samples_remaining > 0 {
                        front.samples_remaining -= 1;
                    }
                    if front.samples_remaining == 0 {
                        self.segments.pop_front();
                    }
                }
            } else {
                *sample = 0.0;
            }
        }
        read_count
    }

    fn current_participant(&self) -> Option<String> {
        self.current_playing_participant.clone()
    }

    fn fill_percentage(&self) -> f64 {
        (self.available_samples as f64 / self.buffer_size as f64) * 100.0
    }

    fn available_seconds(&self, sample_rate: u32) -> f64 {
        self.available_samples as f64 / sample_rate as f64
    }

    fn reset(&mut self) {
        self.write_pos = 0;
        self.read_pos = 0;
        self.available_samples = 0;
        self.segments.clear();
        self.current_playing_participant = None;
    }

    /// Smart reset - only keep segments with the specified question_id
    /// This prevents playing stale audio from previous questions after a reset
    fn smart_reset(&mut self, active_question_id: &str) {
        // Count samples to discard (segments with wrong question_id)
        let mut samples_to_discard = 0;
        let mut new_segments = VecDeque::new();

        for segment in &self.segments {
            if let Some(ref qid) = segment.question_id {
                if qid == active_question_id {
                    new_segments.push_back(segment.clone());
                } else {
                    samples_to_discard += segment.samples_remaining;
                }
            } else {
                // Segments without question_id are discarded
                samples_to_discard += segment.samples_remaining;
            }
        }

        if samples_to_discard > 0 {
            log::info!(
                "Smart reset: discarding {} samples from stale questions, keeping {} segments for question_id={}",
                samples_to_discard,
                new_segments.len(),
                active_question_id
            );

            // Advance read position past discarded samples
            self.read_pos = (self.read_pos + samples_to_discard) % self.buffer_size;
            self.available_samples = self.available_samples.saturating_sub(samples_to_discard);
            self.segments = new_segments;

            // Update current participant from remaining segments
            self.current_playing_participant =
                self.segments.front().and_then(|s| s.participant_id.clone());
        }
    }

    fn available(&self) -> usize {
        self.available_samples
    }
}

/// Shared state between audio thread and main thread
struct SharedAudioState {
    buffer_fill: f64,
    buffer_seconds: f64,
    is_playing: bool,
    current_participant: Option<String>,
    output_waveform: Vec<f32>, // Samples currently being played (for visualization)
}

/// Audio player handle
#[derive(Clone)]
pub struct AudioPlayer {
    command_tx: Sender<AudioCommand>,
    state: Arc<Mutex<SharedAudioState>>,
    sample_rate: u32,
    /// Instant mute flag - checked by audio callback for immediate silence
    /// Used for human speech interrupt to bypass command channel latency
    force_mute: Arc<AtomicBool>,
}

impl AudioPlayer {
    /// Create a new audio player with specified sample rate
    pub fn new(sample_rate: u32) -> Result<Self, String> {
        let (command_tx, command_rx) = unbounded::<AudioCommand>();

        let state = Arc::new(Mutex::new(SharedAudioState {
            buffer_fill: 0.0,
            buffer_seconds: 0.0,
            is_playing: false,
            current_participant: None,
            output_waveform: vec![0.0; 512],
        }));

        // Force mute flag for instant silencing (human speech interrupt)
        let force_mute = Arc::new(AtomicBool::new(false));

        let state_clone = Arc::clone(&state);
        let force_mute_clone = Arc::clone(&force_mute);

        std::thread::spawn(move || {
            if let Err(e) = run_audio_thread(sample_rate, command_rx, state_clone, force_mute_clone)
            {
                log::error!("Audio thread error: {}", e);
            }
        });

        Ok(Self {
            command_tx,
            state,
            sample_rate,
            force_mute,
        })
    }

    /// Add audio samples to the buffer
    pub fn write_audio(&self, samples: &[f32], participant_id: Option<String>) {
        let _ = self
            .command_tx
            .send(AudioCommand::Write(samples.to_vec(), participant_id, None));
    }

    /// Add audio samples to the buffer with question_id for smart reset support
    pub fn write_audio_with_question(
        &self,
        samples: &[f32],
        participant_id: Option<String>,
        question_id: Option<String>,
    ) {
        let _ = self.command_tx.send(AudioCommand::Write(
            samples.to_vec(),
            participant_id,
            question_id,
        ));
    }

    /// Get buffer fill percentage
    pub fn buffer_fill_percentage(&self) -> f64 {
        self.state.lock().buffer_fill
    }

    /// Get available seconds in buffer
    pub fn buffer_seconds(&self) -> f64 {
        self.state.lock().buffer_seconds
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        self.state.lock().is_playing
    }

    /// Get current participant being played
    pub fn current_participant(&self) -> Option<String> {
        self.state.lock().current_participant.clone()
    }

    /// Pause playback
    pub fn pause(&self) {
        let _ = self.command_tx.send(AudioCommand::Pause);
    }

    /// Resume playback
    pub fn resume(&self) {
        let _ = self.command_tx.send(AudioCommand::Resume);
    }

    /// Reset the buffer
    pub fn reset(&self) {
        // Immediately mute audio output before clearing buffer
        self.force_mute.store(true, Ordering::Release);
        let _ = self.command_tx.send(AudioCommand::Reset);
    }

    /// Immediately mute audio output (for human speech interrupt)
    /// This is checked by the audio callback directly, bypassing command channel
    pub fn force_mute(&self) {
        self.force_mute.store(true, Ordering::Release);
        log::info!("🔇 Audio force muted (instant)");
    }

    /// Get the force_mute flag Arc for sharing with other components
    pub fn force_mute_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.force_mute)
    }

    /// Smart reset - keep only audio for the specified question_id
    /// Use this after receiving a new question to discard stale audio
    pub fn smart_reset(&self, question_id: &str) {
        let _ = self
            .command_tx
            .send(AudioCommand::SmartReset(question_id.to_string()));
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get waveform data for visualization (from current audio output)
    /// Returns 512 samples representing the audio currently being played
    pub fn get_waveform_data(&self) -> Vec<f32> {
        self.state.lock().output_waveform.clone()
    }

    /// Get current participant index (0=student1, 1=student2, 2=tutor)
    /// Matches conference-dashboard's interface for consistent behavior
    pub fn current_participant_idx(&self) -> Option<usize> {
        self.state
            .lock()
            .current_participant
            .as_ref()
            .and_then(|p| match p.as_str() {
                "student1" => Some(0),
                "student2" => Some(1),
                "tutor" => Some(2),
                _ => None,
            })
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AudioCommand::Stop);
    }
}

/// Run the audio thread with cpal stream
fn run_audio_thread(
    sample_rate: u32,
    command_rx: Receiver<AudioCommand>,
    state: Arc<Mutex<SharedAudioState>>,
    force_mute: Arc<AtomicBool>,
) -> Result<(), String> {
    let buffer_seconds = 30.0; // 30 second audio buffer
    let buffer = Arc::new(Mutex::new(CircularAudioBuffer::new(
        buffer_seconds,
        sample_rate,
    )));
    let is_playing = Arc::new(AtomicBool::new(false));

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "No audio output device found".to_string())?;

    log::info!(
        "Audio player started - device: {}",
        device.name().unwrap_or_default()
    );

    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let buffer_clone = Arc::clone(&buffer);
    let is_playing_clone = Arc::clone(&is_playing);
    let state_for_callback = Arc::clone(&state);
    let force_mute_clone = Arc::clone(&force_mute);

    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Check force_mute first - this provides instant silencing for human interrupt
                if force_mute_clone.load(Ordering::Acquire) {
                    // Output silence immediately
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                    return;
                }

                if is_playing_clone.load(Ordering::Relaxed) {
                    let mut buf = buffer_clone.lock();
                    buf.read(data);
                    let current_participant = buf.current_participant();
                    drop(buf);

                    if let Some(mut s) = state_for_callback.try_lock() {
                        s.current_participant = current_participant;

                        // Store output samples for waveform visualization
                        // Match conference-dashboard's approach
                        let samples: Vec<f32> = data.iter().copied().collect();
                        if samples.len() >= 512 {
                            s.output_waveform = samples[..512].to_vec();
                        } else if !samples.is_empty() {
                            // Stretch samples to fill 512 by repeating/interpolating
                            s.output_waveform.clear();
                            s.output_waveform.reserve(512);
                            let ratio = samples.len() as f32 / 512.0;
                            for i in 0..512 {
                                let src_idx = ((i as f32 * ratio) as usize).min(samples.len() - 1);
                                s.output_waveform.push(samples[src_idx]);
                            }
                        } else {
                            s.output_waveform = vec![0.0; 512];
                        }
                    }
                } else {
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                }
            },
            move |err| {
                log::error!("Audio stream error: {}", err);
            },
            None,
        )
        .map_err(|e| format!("Failed to build audio stream: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("Failed to start audio stream: {}", e))?;

    loop {
        match command_rx.try_recv() {
            Ok(AudioCommand::Write(samples, participant_id, question_id)) => {
                let mut buf = buffer.lock();
                buf.write_with_participant(&samples, participant_id, question_id);

                // Start playing if we have enough audio
                if buf.available() > sample_rate as usize / 10 {
                    is_playing.store(true, Ordering::Relaxed);
                }
            }
            Ok(AudioCommand::Reset) => {
                is_playing.store(false, Ordering::Relaxed);
                buffer.lock().reset();
                // Clear force_mute after buffer is reset - playback can resume when new audio arrives
                force_mute.store(false, Ordering::Release);
                log::info!("Audio buffer reset (force_mute cleared)");
            }
            Ok(AudioCommand::SmartReset(question_id)) => {
                buffer.lock().smart_reset(&question_id);
                log::info!("Audio buffer smart reset for question_id={}", question_id);
            }
            Ok(AudioCommand::Pause) => {
                is_playing.store(false, Ordering::Relaxed);
            }
            Ok(AudioCommand::Resume) => {
                is_playing.store(true, Ordering::Relaxed);
            }
            Ok(AudioCommand::Stop) => {
                log::info!("Audio thread stopping");
                break;
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {}
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                log::info!("Audio command channel disconnected");
                break;
            }
        }

        // Update shared state
        {
            let buf = buffer.lock();
            let mut s = state.lock();
            s.buffer_fill = buf.fill_percentage();
            s.buffer_seconds = buf.available_seconds(sample_rate);
            s.is_playing = is_playing.load(Ordering::Relaxed);
            s.current_participant = buf.current_participant();
        }

        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    Ok(())
}

/// Create a new audio player
pub fn create_audio_player(sample_rate: u32) -> Result<Arc<AudioPlayer>, String> {
    AudioPlayer::new(sample_rate).map(Arc::new)
}
