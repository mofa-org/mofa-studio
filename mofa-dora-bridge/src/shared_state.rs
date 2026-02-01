//! # Shared State for Dora↔UI Communication
//!
//! This module provides thread-safe shared state containers with dirty tracking
//! for efficient communication between Dora dataflow workers and the UI thread.
//!
//! ## Why Shared State Instead of Channels?
//!
//! Traditional channel-based communication had several issues:
//! - Multiple channels with different capacities (4+ channels)
//! - Multiple polling loops at different intervals (10ms, 50ms, 100ms)
//! - Message consolidation duplicated in multiple places
//! - ~500+ lines of boilerplate
//!
//! The shared state approach simplifies this to:
//! - Single shared state container
//! - Single UI timer reads all dirty data
//! - Built-in streaming consolidation for chat
//! - ~150 lines of code
//!
//! ## Components
//!
//! - [`DirtyVec`] - Thread-safe vector with dirty tracking and max size
//! - [`DirtyValue`] - Thread-safe single value with dirty tracking
//! - [`ChatState`] - Chat messages with automatic streaming consolidation
//! - [`AudioState`] - Ring buffer for audio chunks (producer-consumer pattern)
//! - [`SharedDoraState`] - Unified container for all Dora↔UI state
//!
//! ## Usage Pattern
//!
//! ```rust,ignore
//! use mofa_dora_bridge::SharedDoraState;
//!
//! // Create shared state (returns Arc for sharing between threads)
//! let state = SharedDoraState::new();
//!
//! // PRODUCER: Dora bridge pushes data
//! state.chat.push(ChatMessage { ... });
//! state.logs.push(LogEntry { ... });
//!
//! // CONSUMER: UI reads only when dirty
//! if let Some(messages) = state.chat.read_if_dirty() {
//!     // Update chat widget - only called when data changed
//! }
//! ```
//!
//! ## Thread Safety
//!
//! All types use `parking_lot::RwLock` for data and `AtomicBool` for dirty flags.
//! This provides:
//! - Lock-free dirty checks (just atomic load)
//! - Concurrent reads (RwLock read lock)
//! - Exclusive writes (RwLock write lock)

use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::data::{AudioData, ChatMessage, LogEntry};

/// Thread-safe vector with dirty tracking and maximum size enforcement.
///
/// Designed for producer-consumer scenarios where:
/// - Producers push items from worker threads
/// - Consumers read items from UI thread, but only when data changed
///
/// # Example
///
/// ```rust,ignore
/// use mofa_dora_bridge::DirtyVec;
///
/// let logs: DirtyVec<String> = DirtyVec::new(100);
///
/// // Producer pushes
/// logs.push("Log entry 1".into());
/// logs.push("Log entry 2".into());
///
/// // Consumer reads only if dirty
/// if let Some(entries) = logs.read_if_dirty() {
///     println!("Got {} new entries", entries.len());
/// }
///
/// // Second read returns None (not dirty anymore)
/// assert!(logs.read_if_dirty().is_none());
/// ```
///
/// # Max Size Enforcement
///
/// When the collection exceeds `max_size`, oldest items are removed:
///
/// ```rust,ignore
/// let vec: DirtyVec<i32> = DirtyVec::new(3);
/// vec.push(1);
/// vec.push(2);
/// vec.push(3);
/// vec.push(4); // Removes 1
/// assert_eq!(vec.read_all(), vec![2, 3, 4]);
/// ```
pub struct DirtyVec<T> {
    data: RwLock<Vec<T>>,
    dirty: AtomicBool,
    max_size: usize,
}

impl<T: Clone> DirtyVec<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            data: RwLock::new(Vec::new()),
            dirty: AtomicBool::new(false),
            max_size,
        }
    }

    /// Push item, mark dirty, enforce max size
    pub fn push(&self, item: T) {
        let mut data = self.data.write();
        data.push(item);
        if data.len() > self.max_size {
            data.remove(0);
        }
        self.dirty.store(true, Ordering::Release);
    }

    /// Read all data if dirty, clearing dirty flag
    pub fn read_if_dirty(&self) -> Option<Vec<T>> {
        if self.dirty.swap(false, Ordering::AcqRel) {
            Some(self.data.read().clone())
        } else {
            None
        }
    }

    /// Read all data unconditionally
    pub fn read_all(&self) -> Vec<T> {
        self.data.read().clone()
    }

    /// Clear all data
    pub fn clear(&self) {
        self.data.write().clear();
        self.dirty.store(true, Ordering::Release);
    }

    /// Check if dirty without consuming
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }
}

/// Thread-safe single value with dirty tracking.
///
/// Similar to [`DirtyVec`] but for single values instead of collections.
/// Useful for status, configuration, or any value that changes infrequently.
///
/// # Example
///
/// ```rust,ignore
/// use mofa_dora_bridge::DirtyValue;
///
/// let status: DirtyValue<String> = DirtyValue::new("disconnected".into());
///
/// // Producer updates value
/// status.set("connected".into());
///
/// // Consumer reads only if changed
/// if let Some(new_status) = status.read_if_dirty() {
///     println!("Status changed to: {}", new_status);
/// }
/// ```
pub struct DirtyValue<T> {
    data: RwLock<T>,
    dirty: AtomicBool,
}

impl<T: Clone + Default> DirtyValue<T> {
    pub fn new(initial: T) -> Self {
        Self {
            data: RwLock::new(initial),
            dirty: AtomicBool::new(false),
        }
    }

    /// Set value and mark dirty
    pub fn set(&self, value: T) {
        *self.data.write() = value;
        self.dirty.store(true, Ordering::Release);
    }

    /// Read value if dirty, clearing dirty flag
    pub fn read_if_dirty(&self) -> Option<T> {
        if self.dirty.swap(false, Ordering::AcqRel) {
            Some(self.data.read().clone())
        } else {
            None
        }
    }

    /// Read value unconditionally
    pub fn read(&self) -> T {
        self.data.read().clone()
    }
}

impl<T: Default> Default for DirtyValue<T> {
    fn default() -> Self {
        Self {
            data: RwLock::new(T::default()),
            dirty: AtomicBool::new(false),
        }
    }
}

/// Chat state with automatic streaming message consolidation.
///
/// This is the key innovation for handling LLM streaming responses.
/// When a message is marked as `is_streaming: true`, subsequent messages
/// from the same sender/session are **accumulated** (appended) rather than
/// creating new messages.
///
/// # Streaming Consolidation
///
/// ```text
/// Push: { sender: "Bot", content: "Hello", is_streaming: true, session_id: "s1" }
/// Push: { sender: "Bot", content: " world", is_streaming: true, session_id: "s1" }
/// Push: { sender: "Bot", content: "!", is_streaming: false, session_id: "s1" }
///
/// Result: ONE message with content "Hello world!" (not three separate messages)
/// ```
///
/// # Multi-Participant Isolation
///
/// Messages are only consolidated if they have the **same sender AND session_id**.
/// This prevents mixing up concurrent streams from different participants:
///
/// ```rust,ignore
/// // These will NOT be consolidated (different session_ids)
/// chat.push(ChatMessage { sender: "Tutor", session_id: Some("s1"), ... });
/// chat.push(ChatMessage { sender: "Student", session_id: Some("s2"), ... });
/// ```
///
/// # No Session ID = No Consolidation
///
/// Messages without `session_id` are never consolidated, even from the same sender.
/// This is a safety feature to prevent accidental merging.
pub struct ChatState {
    messages: RwLock<Vec<ChatMessage>>,
    dirty: AtomicBool,
    max_messages: usize,
}

impl ChatState {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: RwLock::new(Vec::new()),
            dirty: AtomicBool::new(false),
            max_messages,
        }
    }

    /// Push message with automatic streaming consolidation
    ///
    /// If message is streaming, ACCUMULATES content to existing streaming message from same sender/session.
    /// If message is complete, finalizes any existing streaming message.
    pub fn push(&self, msg: ChatMessage) {
        let mut messages = self.messages.write();

        // Find existing streaming message from same sender + session
        // IMPORTANT: Only match if BOTH have valid session_ids (not None)
        // to prevent incorrectly merging messages from different participants
        let existing_idx = messages.iter().position(|m| {
            m.sender == msg.sender
                && m.is_streaming
                && m.session_id.is_some()
                && m.session_id == msg.session_id
        });

        if let Some(idx) = existing_idx {
            // ACCUMULATE content for streaming messages (append, not replace)
            messages[idx].content.push_str(&msg.content);
            if !msg.is_streaming {
                // Finalize: mark as complete
                messages[idx].is_streaming = false;
                messages[idx].timestamp = msg.timestamp;
            }
        } else {
            // New message
            messages.push(msg);

            // Enforce max size
            if messages.len() > self.max_messages {
                messages.remove(0);
            }
        }

        self.dirty.store(true, Ordering::Release);
    }

    /// Read all messages if dirty
    pub fn read_if_dirty(&self) -> Option<Vec<ChatMessage>> {
        if self.dirty.swap(false, Ordering::AcqRel) {
            Some(self.messages.read().clone())
        } else {
            None
        }
    }

    /// Read all messages unconditionally
    pub fn read_all(&self) -> Vec<ChatMessage> {
        self.messages.read().clone()
    }

    /// Clear all messages
    pub fn clear(&self) {
        self.messages.write().clear();
        self.dirty.store(true, Ordering::Release);
    }

    /// Get message count
    pub fn len(&self) -> usize {
        self.messages.read().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.messages.read().is_empty()
    }
}

/// Ring buffer for audio chunks with producer-consumer semantics.
///
/// Unlike [`ChatState`] and [`DirtyVec`], audio data is **consumed** (drained)
/// rather than just read. This matches the audio playback pattern where each
/// chunk should be played exactly once.
///
/// # Producer-Consumer Pattern
///
/// ```rust,ignore
/// use mofa_dora_bridge::AudioState;
///
/// let audio = AudioState::new(100); // Max 100 pending chunks
///
/// // PRODUCER (Dora bridge thread)
/// audio.push(AudioData { samples: vec![0.1, 0.2], ... });
/// audio.push(AudioData { samples: vec![0.3, 0.4], ... });
///
/// // CONSUMER (Audio playback thread)
/// let chunks = audio.drain(); // Takes all chunks, empties buffer
/// for chunk in chunks {
///     play_samples(&chunk.samples);
/// }
/// ```
///
/// # Bounded Buffer
///
/// When the buffer exceeds `max_chunks`, oldest chunks are dropped.
/// This prevents memory growth if the consumer can't keep up.
///
/// # Instant Mute for Human Interrupt
///
/// For immediate audio silencing (when human starts speaking), the UI can
/// register its AudioPlayer's force_mute flag with [`AudioState::register_force_mute`].
/// When the bridge receives a reset signal, calling [`AudioState::signal_clear`]
/// will immediately set the force_mute flag, bypassing any polling latency.
pub struct AudioState {
    chunks: RwLock<VecDeque<AudioData>>,
    max_chunks: usize,
    /// Signal for immediate buffer clear (human speaking interrupt)
    /// When set to true, UI should clear its circular buffer immediately
    should_clear: std::sync::atomic::AtomicBool,
    /// Registered force_mute flag from AudioPlayer for instant silencing
    /// Set by the bridge to immediately mute audio output
    force_mute_flag: RwLock<Option<Arc<AtomicBool>>>,
}

impl AudioState {
    pub fn new(max_chunks: usize) -> Self {
        Self {
            chunks: RwLock::new(VecDeque::new()),
            max_chunks,
            should_clear: std::sync::atomic::AtomicBool::new(false),
            force_mute_flag: RwLock::new(None),
        }
    }

    /// Register the AudioPlayer's force_mute flag for instant silencing.
    ///
    /// When the bridge calls `signal_clear()`, it will set this flag to immediately
    /// mute audio output, bypassing any UI polling latency.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // In UI initialization after creating AudioPlayer:
    /// let audio_player = AudioPlayer::new(32000)?;
    /// shared_state.audio.register_force_mute(audio_player.force_mute_flag());
    /// ```
    pub fn register_force_mute(&self, flag: Arc<AtomicBool>) {
        *self.force_mute_flag.write() = Some(flag);
    }

    /// Push audio chunk (producer - bridge thread)
    pub fn push(&self, chunk: AudioData) {
        let mut chunks = self.chunks.write();
        chunks.push_back(chunk);
        // Bound to prevent memory growth
        while chunks.len() > self.max_chunks {
            chunks.pop_front();
        }
    }

    /// Drain all available chunks (consumer - audio thread)
    pub fn drain(&self) -> Vec<AudioData> {
        self.chunks.write().drain(..).collect()
    }

    /// Drain up to N chunks
    pub fn drain_n(&self, n: usize) -> Vec<AudioData> {
        let mut chunks = self.chunks.write();
        let drain_count = n.min(chunks.len());
        chunks.drain(..drain_count).collect()
    }

    /// Check if audio available
    pub fn has_audio(&self) -> bool {
        !self.chunks.read().is_empty()
    }

    /// Get pending chunk count
    pub fn len(&self) -> usize {
        self.chunks.read().len()
    }

    /// Clear all pending audio
    pub fn clear(&self) {
        self.chunks.write().clear();
    }

    /// Signal UI to clear its circular buffer immediately (human interrupt)
    ///
    /// This does three things for maximum responsiveness:
    /// 1. Sets force_mute flag (if registered) - instant audio silencing
    /// 2. Sets should_clear flag for UI polling
    /// 3. Clears pending audio chunks in shared state
    ///
    /// The force_mute flag provides **instant** silencing because it's checked
    /// directly by the audio callback, while should_clear is polled by the UI.
    pub fn signal_clear(&self) {
        // Set force_mute FIRST for instant silencing (bypasses UI polling)
        if let Some(ref flag) = *self.force_mute_flag.read() {
            flag.store(true, std::sync::atomic::Ordering::Release);
            tracing::info!("🔇 Force mute set (instant audio silencing)");
        }
        // Also set should_clear for backwards compatibility with UI polling
        self.should_clear.store(true, std::sync::atomic::Ordering::Release);
        self.clear();
    }

    /// Check and reset the clear signal (UI calls this)
    /// Returns true if buffer should be cleared, resets flag
    pub fn take_clear_signal(&self) -> bool {
        self.should_clear.swap(false, std::sync::atomic::Ordering::AcqRel)
    }
}

/// Dora connection status
#[derive(Debug, Clone, Default)]
pub struct DoraStatus {
    /// List of connected bridge node IDs
    pub active_bridges: Vec<String>,
    /// Last error message if any
    pub last_error: Option<String>,
}

/// Microphone input state (from AEC bridge)
///
/// Provides dirty-tracked state for mic level visualization and speech detection.
/// The AEC input bridge writes to this state from its worker thread,
/// and the UI thread reads it on timer to update visualizations.
///
/// # Example
///
/// ```rust,ignore
/// // Producer (AEC bridge thread)
/// state.mic.set_level(0.7);
/// state.mic.set_speaking(true);
///
/// // Consumer (UI thread)
/// if let Some(level) = state.mic.read_level_if_dirty() {
///     update_mic_level_leds(level);
/// }
/// ```
pub struct MicState {
    /// Microphone input level (0.0 - 1.0, RMS normalized)
    level: DirtyValue<f32>,
    /// Whether VAD detects speech
    is_speaking: DirtyValue<bool>,
    /// Whether recording is active
    is_recording: DirtyValue<bool>,
    /// Whether AEC is enabled
    aec_enabled: DirtyValue<bool>,
}

impl MicState {
    pub fn new() -> Self {
        Self {
            level: DirtyValue::new(0.0),
            is_speaking: DirtyValue::new(false),
            is_recording: DirtyValue::new(false),
            aec_enabled: DirtyValue::new(true),
        }
    }

    // Setters (for AEC bridge thread)

    /// Set mic level (0.0 - 1.0)
    pub fn set_level(&self, level: f32) {
        self.level.set(level);
    }

    /// Set speaking state (from VAD)
    pub fn set_speaking(&self, speaking: bool) {
        self.is_speaking.set(speaking);
    }

    /// Set recording state
    pub fn set_recording(&self, recording: bool) {
        self.is_recording.set(recording);
    }

    /// Set AEC enabled state
    pub fn set_aec_enabled(&self, enabled: bool) {
        self.aec_enabled.set(enabled);
    }

    // Getters (for UI thread)

    /// Read mic level if changed
    pub fn read_level_if_dirty(&self) -> Option<f32> {
        self.level.read_if_dirty()
    }

    /// Read speaking state if changed
    pub fn read_speaking_if_dirty(&self) -> Option<bool> {
        self.is_speaking.read_if_dirty()
    }

    /// Read recording state if changed
    pub fn read_recording_if_dirty(&self) -> Option<bool> {
        self.is_recording.read_if_dirty()
    }

    /// Read AEC enabled state if changed
    pub fn read_aec_enabled_if_dirty(&self) -> Option<bool> {
        self.aec_enabled.read_if_dirty()
    }

    /// Read mic level unconditionally
    pub fn level(&self) -> f32 {
        self.level.read()
    }

    /// Read speaking state unconditionally
    pub fn is_speaking(&self) -> bool {
        self.is_speaking.read()
    }

    /// Read recording state unconditionally
    pub fn is_recording(&self) -> bool {
        self.is_recording.read()
    }

    /// Read AEC enabled state unconditionally
    pub fn is_aec_enabled(&self) -> bool {
        self.aec_enabled.read()
    }

    /// Clear all state
    pub fn clear(&self) {
        self.level.set(0.0);
        self.is_speaking.set(false);
        self.is_recording.set(false);
        self.aec_enabled.set(true);
    }
}

impl Default for MicState {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified shared state container for all Dora↔UI communication.
///
/// This is the main entry point for the shared state system. Create one instance
/// at app startup and share it (via `Arc`) between all Dora bridges and the UI.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
/// │  Dora Bridges   │────▶│  SharedDoraState │◀────│  UI Thread      │
/// │  (push data)    │     │  (Arc<...>)      │     │  (read dirty)   │
/// └─────────────────┘     └──────────────────┘     └─────────────────┘
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use mofa_dora_bridge::SharedDoraState;
/// use std::sync::Arc;
///
/// // Create shared state
/// let state = SharedDoraState::new(); // Returns Arc<SharedDoraState>
///
/// // Clone for bridge thread
/// let bridge_state = Arc::clone(&state);
/// std::thread::spawn(move || {
///     bridge_state.chat.push(ChatMessage { ... });
///     bridge_state.audio.push(AudioData { ... });
/// });
///
/// // UI thread polls on timer
/// loop {
///     if let Some(messages) = state.chat.read_if_dirty() {
///         update_chat_widget(messages);
///     }
///     let audio_chunks = state.audio.drain();
///     play_audio(audio_chunks);
///
///     std::thread::sleep(Duration::from_millis(50));
/// }
/// ```
///
/// # Default Capacities
///
/// - Chat: 500 messages
/// - Audio: 100 chunks
/// - Logs: 1000 entries
///
/// Use [`SharedDoraState::with_capacities`] for custom limits.
pub struct SharedDoraState {
    /// Chat messages (with streaming consolidation)
    pub chat: ChatState,

    /// Audio chunks (ring buffer, consumed by audio player)
    pub audio: AudioState,

    /// Log entries
    pub logs: DirtyVec<LogEntry>,

    /// Connection/dataflow status
    pub status: DirtyValue<DoraStatus>,

    /// Microphone input state (from AEC bridge)
    pub mic: MicState,

    /// ASR transcription result (language, text)
    pub asr_transcription: DirtyValue<Option<(String, String)>>,
}

impl SharedDoraState {
    /// Create new shared state with default capacities
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            chat: ChatState::new(500),   // 500 max chat messages
            audio: AudioState::new(100), // 100 max pending audio chunks
            logs: DirtyVec::new(1000),   // 1000 max log entries
            status: DirtyValue::default(),
            mic: MicState::new(),
            asr_transcription: DirtyValue::default(),
        })
    }

    /// Create with custom capacities
    pub fn with_capacities(max_chat: usize, max_audio_chunks: usize, max_logs: usize) -> Arc<Self> {
        Arc::new(Self {
            chat: ChatState::new(max_chat),
            audio: AudioState::new(max_audio_chunks),
            logs: DirtyVec::new(max_logs),
            status: DirtyValue::default(),
            mic: MicState::new(),
            asr_transcription: DirtyValue::default(),
        })
    }

    /// Clear all state (on dataflow stop/reset)
    pub fn clear_all(&self) {
        self.chat.clear();
        self.audio.clear();
        self.logs.clear();
        self.status.set(DoraStatus::default());
        self.mic.clear();
        self.asr_transcription.set(None);
    }

    /// Add active bridge
    pub fn add_bridge(&self, bridge_id: String) {
        let mut status = self.status.read();
        if !status.active_bridges.contains(&bridge_id) {
            status.active_bridges.push(bridge_id);
            self.status.set(status);
        }
    }

    /// Remove active bridge
    pub fn remove_bridge(&self, bridge_id: &str) {
        let mut status = self.status.read();
        status.active_bridges.retain(|b| b != bridge_id);
        self.status.set(status);
    }

    /// Set error status
    pub fn set_error(&self, error: Option<String>) {
        let mut status = self.status.read();
        status.last_error = error;
        self.status.set(status);
    }
}

impl Default for SharedDoraState {
    fn default() -> Self {
        Self {
            chat: ChatState::new(500),
            audio: AudioState::new(100),
            logs: DirtyVec::new(1000),
            status: DirtyValue::default(),
            mic: MicState::new(),
            asr_transcription: DirtyValue::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::MessageRole;

    #[test]
    fn test_dirty_vec() {
        let vec: DirtyVec<i32> = DirtyVec::new(5);

        // Initially not dirty
        assert!(vec.read_if_dirty().is_none());

        // Push makes dirty
        vec.push(1);
        vec.push(2);

        // Read clears dirty
        let data = vec.read_if_dirty().unwrap();
        assert_eq!(data, vec![1, 2]);

        // Now not dirty
        assert!(vec.read_if_dirty().is_none());

        // Max size enforcement
        for i in 0..10 {
            vec.push(i);
        }
        let data = vec.read_all();
        assert_eq!(data.len(), 5); // Max size
        assert_eq!(data, vec![5, 6, 7, 8, 9]); // Oldest removed
    }

    #[test]
    fn test_chat_streaming_consolidation() {
        let chat = ChatState::new(100);

        // First streaming chunk
        chat.push(ChatMessage {
            content: "Hello".to_string(),
            sender: "Bot".to_string(),
            role: MessageRole::Assistant,
            timestamp: 1000,
            is_streaming: true,
            session_id: Some("s1".to_string()),
        });

        // Second streaming chunk - should ACCUMULATE, not replace
        chat.push(ChatMessage {
            content: ", world".to_string(),
            sender: "Bot".to_string(),
            role: MessageRole::Assistant,
            timestamp: 1001,
            is_streaming: true,
            session_id: Some("s1".to_string()),
        });

        // Should still be one message with accumulated content
        let messages = chat.read_all();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello, world"); // Accumulated!
        assert!(messages[0].is_streaming);

        // Finalize with final chunk
        chat.push(ChatMessage {
            content: "!".to_string(),
            sender: "Bot".to_string(),
            role: MessageRole::Assistant,
            timestamp: 1002,
            is_streaming: false,
            session_id: Some("s1".to_string()),
        });

        let messages = chat.read_all();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello, world!"); // Full accumulated content
        assert!(!messages[0].is_streaming);
    }

    #[test]
    fn test_chat_multi_participant_isolation() {
        let chat = ChatState::new(100);

        // Two participants streaming concurrently with different session_ids
        chat.push(ChatMessage {
            content: "Hello from ".to_string(),
            sender: "Tutor".to_string(),
            role: MessageRole::Assistant,
            timestamp: 1000,
            is_streaming: true,
            session_id: Some("session_tutor".to_string()),
        });

        chat.push(ChatMessage {
            content: "Hi from ".to_string(),
            sender: "Student".to_string(),
            role: MessageRole::Assistant,
            timestamp: 1001,
            is_streaming: true,
            session_id: Some("session_student".to_string()),
        });

        // Continue streaming - each should accumulate separately
        chat.push(ChatMessage {
            content: "tutor!".to_string(),
            sender: "Tutor".to_string(),
            role: MessageRole::Assistant,
            timestamp: 1002,
            is_streaming: false,
            session_id: Some("session_tutor".to_string()),
        });

        chat.push(ChatMessage {
            content: "student!".to_string(),
            sender: "Student".to_string(),
            role: MessageRole::Assistant,
            timestamp: 1003,
            is_streaming: false,
            session_id: Some("session_student".to_string()),
        });

        // Should have 2 separate messages, properly accumulated
        let messages = chat.read_all();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Hello from tutor!");
        assert_eq!(messages[0].sender, "Tutor");
        assert_eq!(messages[1].content, "Hi from student!");
        assert_eq!(messages[1].sender, "Student");
    }

    #[test]
    fn test_chat_no_session_id_creates_new_message() {
        let chat = ChatState::new(100);

        // Messages without session_id should NOT be consolidated
        chat.push(ChatMessage {
            content: "First".to_string(),
            sender: "Bot".to_string(),
            role: MessageRole::Assistant,
            timestamp: 1000,
            is_streaming: true,
            session_id: None, // No session_id
        });

        chat.push(ChatMessage {
            content: "Second".to_string(),
            sender: "Bot".to_string(),
            role: MessageRole::Assistant,
            timestamp: 1001,
            is_streaming: true,
            session_id: None, // No session_id
        });

        // Should be 2 separate messages (not consolidated)
        let messages = chat.read_all();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "First");
        assert_eq!(messages[1].content, "Second");
    }

    #[test]
    fn test_audio_drain() {
        let audio = AudioState::new(10);

        audio.push(AudioData {
            samples: vec![0.1, 0.2],
            sample_rate: 44100,
            channels: 1,
            participant_id: None,
            question_id: None,
        });
        audio.push(AudioData {
            samples: vec![0.3, 0.4],
            sample_rate: 44100,
            channels: 1,
            participant_id: None,
            question_id: None,
        });

        assert_eq!(audio.len(), 2);

        let chunks = audio.drain();
        assert_eq!(chunks.len(), 2);
        assert_eq!(audio.len(), 0);
    }
}
