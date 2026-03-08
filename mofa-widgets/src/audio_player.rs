//! Audio Player Module - Circular buffer audio playback using cpal
//!
//! Features:
//! - Thread-safe circular buffer for audio samples
//! - Configurable sample rate (default 32kHz for PrimeSpeech, 24kHz for Kokoro)
//! - Buffer status reporting for backpressure control
//! - Uses channels for thread-safe communication

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// Segment tracking for knowing which participant owns audio in the buffer
#[derive(Clone, Debug)]
struct AudioSegment {
    participant_idx: Option<usize>,
    samples_remaining: usize,
}

/// Commands sent to the audio thread
enum AudioCommand {
    Write(Vec<f32>, Option<u32>, Option<usize>), // samples, question_id, participant_idx
    Reset,
    Pause,
    Resume,
    Stop,
}

/// Circular audio buffer for thread-safe audio streaming with segment tracking
struct CircularAudioBuffer {
    buffer: Vec<f32>,
    write_pos: usize,
    read_pos: usize,
    available_samples: usize,
    buffer_size: usize,
    /// Track which participant owns each segment of audio
    segments: VecDeque<AudioSegment>,
    /// Current participant being played
    current_playing_participant: Option<usize>,
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

    fn write_with_participant(&mut self, samples: &[f32], participant_idx: Option<usize>) -> usize {
        let mut written = 0;
        for &sample in samples {
            if self.available_samples < self.buffer_size {
                self.buffer[self.write_pos] = sample;
                self.write_pos = (self.write_pos + 1) % self.buffer_size;
                self.available_samples += 1;
                written += 1;
            } else {
                // Buffer full - overwrite oldest data and update segment tracking
                self.buffer[self.write_pos] = sample;
                self.write_pos = (self.write_pos + 1) % self.buffer_size;
                self.read_pos = (self.read_pos + 1) % self.buffer_size;
                // Remove one sample from oldest segment
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

        // Add segment tracking for this write
        if written > 0 {
            // Try to merge with last segment if same participant
            if let Some(last) = self.segments.back_mut() {
                if last.participant_idx == participant_idx {
                    last.samples_remaining += written;
                } else {
                    self.segments.push_back(AudioSegment {
                        participant_idx,
                        samples_remaining: written,
                    });
                }
            } else {
                self.segments.push_back(AudioSegment {
                    participant_idx,
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

                // Update segment tracking - consume from front segment
                if let Some(front) = self.segments.front_mut() {
                    // Update current playing participant
                    self.current_playing_participant = front.participant_idx;

                    if front.samples_remaining > 0 {
                        front.samples_remaining -= 1;
                    }
                    if front.samples_remaining == 0 {
                        self.segments.pop_front();
                    }
                }
            } else {
                *sample = 0.0; // Underrun - output silence
            }
        }
        read_count
    }

    /// Get the participant whose audio is currently being played
    fn current_participant(&self) -> Option<usize> {
        self.current_playing_participant
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

    fn available(&self) -> usize {
        self.available_samples
    }

    fn get_waveform(&self, num_samples: usize) -> Vec<f32> {
        let mut samples = Vec::with_capacity(num_samples);
        if self.available_samples == 0 {
            return vec![0.0; num_samples];
        }

        let start = if self.available_samples >= num_samples {
            (self.read_pos + self.buffer_size - num_samples) % self.buffer_size
        } else {
            self.read_pos
        };

        for i in 0..num_samples.min(self.available_samples) {
            let idx = (start + i) % self.buffer_size;
            samples.push(self.buffer[idx]);
        }

        samples.resize(num_samples, 0.0);
        samples
    }
}

/// Shared state between audio thread and main thread
struct SharedAudioState {
    buffer_fill: f64,
    buffer_seconds: f64,
    is_playing: bool,
    waveform: Vec<f32>,
    output_waveform: Vec<f32>, // Samples currently being played
    current_question_id: u32,
    current_participant_idx: Option<usize>, // 0=student1, 1=student2, 2=tutor
}

/// Audio player handle - can be cloned and shared across threads
#[derive(Clone)]
pub struct AudioPlayer {
    command_tx: Sender<AudioCommand>,
    state: Arc<Mutex<SharedAudioState>>,
    sample_rate: u32,
}

impl AudioPlayer {
    /// Create a new audio player with specified sample rate
    pub fn new(sample_rate: u32) -> Result<Self, String> {
        let (command_tx, command_rx) = unbounded::<AudioCommand>();

        let state = Arc::new(Mutex::new(SharedAudioState {
            buffer_fill: 0.0,
            buffer_seconds: 0.0,
            is_playing: false,
            waveform: vec![0.0; 512],
            output_waveform: vec![0.0; 512],
            current_question_id: 0,
            current_participant_idx: None,
        }));

        let state_clone = Arc::clone(&state);

        // Spawn audio thread
        std::thread::spawn(move || {
            if let Err(e) = run_audio_thread(sample_rate, command_rx, state_clone) {
                log::error!("Audio thread error: {}", e);
            }
        });

        Ok(Self {
            command_tx,
            state,
            sample_rate,
        })
    }

    /// Add audio samples to the buffer
    pub fn write_audio(
        &self,
        samples: &[f32],
        question_id: Option<u32>,
        participant_idx: Option<usize>,
    ) {
        let _ = self.command_tx.send(AudioCommand::Write(
            samples.to_vec(),
            question_id,
            participant_idx,
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

    /// Pause playback
    pub fn pause(&self) {
        let _ = self.command_tx.send(AudioCommand::Pause);
    }

    /// Resume playback
    pub fn resume(&self) {
        let _ = self.command_tx.send(AudioCommand::Resume);
    }

    /// Reset the buffer (for new question)
    pub fn reset(&self) {
        let _ = self.command_tx.send(AudioCommand::Reset);
    }

    /// Get current question_id
    pub fn current_question_id(&self) -> u32 {
        self.state.lock().current_question_id
    }

    /// Get current participant index (0=student1, 1=student2, 2=tutor)
    pub fn current_participant_idx(&self) -> Option<usize> {
        self.state.lock().current_participant_idx
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get waveform data for visualization (from current audio output)
    pub fn get_waveform_data(&self, _num_samples: usize) -> Vec<f32> {
        self.state.lock().output_waveform.clone()
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
) -> Result<(), String> {
    let buffer_seconds = 60.0;
    let buffer = Arc::new(Mutex::new(CircularAudioBuffer::new(
        buffer_seconds,
        sample_rate,
    )));
    let is_playing = Arc::new(AtomicBool::new(false));
    let current_question_id = Arc::new(AtomicU32::new(0));

    // Initialize cpal audio output
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "No audio output device found".to_string())?;

    log::info!(
        "Audio thread started - device: {}",
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

    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if is_playing_clone.load(Ordering::Relaxed) {
                    let mut buf = buffer_clone.lock();
                    buf.read(data);
                    let current_participant = buf.current_participant();
                    drop(buf); // Release buffer lock before acquiring state lock

                    // Update state with current playing participant and waveform
                    if let Some(mut s) = state_for_callback.try_lock() {
                        // Update current participant immediately from audio callback
                        s.current_participant_idx = current_participant;

                        // Store the most recent output samples, stretching if needed
                        let samples: Vec<f32> = data.to_vec();
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

    // Process commands
    loop {
        // Non-blocking check for commands
        match command_rx.try_recv() {
            Ok(AudioCommand::Write(samples, question_id, participant_idx)) => {
                if let Some(qid) = question_id {
                    current_question_id.store(qid, Ordering::Relaxed);
                }

                let mut buf = buffer.lock();

                // Write audio with participant tracking - the buffer will track
                // which participant owns each segment of audio
                buf.write_with_participant(&samples, participant_idx);

                // Start playing if we have enough audio
                if buf.available() > sample_rate as usize / 10 {
                    is_playing.store(true, Ordering::Relaxed);
                }
            }
            Ok(AudioCommand::Reset) => {
                is_playing.store(false, Ordering::Relaxed);
                buffer.lock().reset();
                log::info!("Audio buffer reset");
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
            Err(crossbeam_channel::TryRecvError::Empty) => {
                // No command, update state
            }
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
            s.waveform = buf.get_waveform(512);
            s.current_question_id = current_question_id.load(Ordering::Relaxed);
            // Get current participant from buffer segment tracking - reflects what's actually playing
            s.current_participant_idx = buf.current_participant();
        }

        // Small sleep to avoid busy loop (5ms for responsive state updates)
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    Ok(())
}

/// Audio player reference type for sharing across threads
pub type AudioPlayerRef = Arc<AudioPlayer>;

/// Create a new audio player reference
pub fn create_audio_player(sample_rate: u32) -> Result<AudioPlayerRef, String> {
    AudioPlayer::new(sample_rate).map(Arc::new)
}
