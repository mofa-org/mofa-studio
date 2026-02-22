//! # Data Types for MoFA-Dora Communication
//!
//! This module defines the core data types exchanged between the MoFA Studio UI
//! and the Dora dataflow runtime.
//!
//! ## Overview
//!
//! | Type | Purpose | Direction |
//! |------|---------|-----------|
//! | [`AudioData`] | TTS audio samples with metadata | Dora → UI |
//! | [`ChatMessage`] | Conversation messages | Dora → UI |
//! | [`LogEntry`] | System/debug logs | Dora → UI |
//! | [`ControlCommand`] | Dataflow control commands | UI → Dora |
//! | [`DoraData`] | Unified wrapper for all data types | Both |
//!
//! ## Key Design Decisions
//!
//! ### Audio with Metadata
//!
//! [`AudioData`] includes `participant_id` and `question_id` to support:
//! - Multi-speaker visualization (which participant is speaking)
//! - Smart reset (discard stale audio from previous questions)
//!
//! ### Streaming Support
//!
//! [`ChatMessage`] includes `is_streaming` and `session_id` to support:
//! - Progressive display of LLM responses
//! - Automatic consolidation of streaming chunks (see [`ChatState`](crate::ChatState))
//!
//! ### Log Levels
//!
//! [`LogLevel`] is ordered for filtering:
//! - Debug < Info < Warning < Error
//! - UI can filter to show only logs >= a threshold

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unified data type for all MoFA-Dora communication
#[derive(Debug, Clone)]
pub enum DoraData {
    /// Audio samples (f32, mono or stereo)
    Audio(AudioData),
    /// Text string
    Text(String),
    /// Structured JSON data
    Json(serde_json::Value),
    /// Raw binary data
    Binary(Vec<u8>),
    /// Control command
    Control(ControlCommand),
    /// Log entry
    Log(LogEntry),
    /// Chat message
    Chat(ChatMessage),
    /// Empty/signal data
    Empty,
}

impl DoraData {
    /// Create audio data from f32 samples
    pub fn audio(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        DoraData::Audio(AudioData {
            samples,
            sample_rate,
            channels,
            participant_id: None,
            question_id: None,
        })
    }

    /// Create text data
    pub fn text(s: impl Into<String>) -> Self {
        DoraData::Text(s.into())
    }

    /// Create log entry
    pub fn log(level: LogLevel, message: impl Into<String>, node_id: impl Into<String>) -> Self {
        DoraData::Log(LogEntry {
            level,
            message: message.into(),
            node_id: node_id.into(),
            timestamp: current_timestamp(),
            metadata: HashMap::new(),
        })
    }

    /// Create control command
    pub fn control(command: impl Into<String>) -> Self {
        DoraData::Control(ControlCommand {
            command: command.into(),
            params: HashMap::new(),
        })
    }
}

/// Audio data with metadata for playback and visualization.
///
/// Represents a chunk of audio samples from TTS synthesis, along with
/// metadata for multi-speaker scenarios and smart reset functionality.
///
/// # Fields
///
/// | Field | Purpose |
/// |-------|---------|
/// | `samples` | Raw f32 audio samples, normalized to [-1.0, 1.0] |
/// | `sample_rate` | Sample rate in Hz (typically 32000 for TTS) |
/// | `channels` | 1 = mono, 2 = stereo |
/// | `participant_id` | Speaker ID for multi-participant visualization |
/// | `question_id` | Question ID for smart reset filtering |
///
/// # Smart Reset
///
/// The `question_id` field enables "smart reset" - when switching questions,
/// audio chunks with old question IDs can be discarded while keeping chunks
/// for the new question. This prevents playing stale audio from the previous
/// conversation turn.
///
/// # Example
///
/// ```rust,ignore
/// let audio = AudioData {
///     samples: vec![0.1, 0.2, -0.1, 0.0],
///     sample_rate: 32000,
///     channels: 1,
///     participant_id: Some("tutor".into()),
///     question_id: Some("q42".into()),
/// };
///
/// println!("Duration: {:.2}s", audio.duration_secs());
/// ```
#[derive(Debug, Clone)]
pub struct AudioData {
    /// Audio samples in f32 format (-1.0 to 1.0)
    pub samples: Vec<f32>,
    /// Sample rate in Hz (e.g., 32000, 44100, 48000)
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u16,
    /// Optional participant ID for multi-speaker scenarios
    pub participant_id: Option<String>,
    /// Optional question ID for smart reset (discard stale audio)
    pub question_id: Option<String>,
}

impl AudioData {
    /// Duration in seconds
    pub fn duration_secs(&self) -> f32 {
        self.samples.len() as f32 / (self.sample_rate as f32 * self.channels as f32)
    }

    /// Convert to mono if stereo
    pub fn to_mono(&self) -> Vec<f32> {
        if self.channels == 1 {
            return self.samples.clone();
        }
        self.samples
            .chunks(self.channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
            .collect()
    }
}

/// Log entry from dora nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Source node ID
    pub node_id: String,
    /// Unix timestamp in milliseconds
    pub timestamp: u64,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl LogEntry {
    /// Create a new log entry with current timestamp
    pub fn new(level: LogLevel, message: impl Into<String>, node_id: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            node_id: node_id.into(),
            timestamp: current_timestamp(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Log level for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Debug = 10,
    #[default]
    Info = 20,
    Warning = 30,
    Error = 40,
}


impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARNING" | "WARN" => LogLevel::Warning,
            "ERROR" | "ERR" => LogLevel::Error,
            _ => LogLevel::Info,
        })
    }
}

/// Chat message for conversation display.
///
/// Represents a single message in the conversation, with support for
/// streaming (progressive LLM output) and multi-participant scenarios.
///
/// # Streaming Messages
///
/// When `is_streaming` is true, the message is incomplete and may be
/// updated with additional content. The [`ChatState`](crate::ChatState)
/// automatically consolidates streaming chunks from the same sender/session.
///
/// # Session ID
///
/// The `session_id` field is critical for proper streaming consolidation:
/// - Messages with matching `sender` AND `session_id` are consolidated
/// - Messages without `session_id` are never consolidated (safety feature)
/// - Different session IDs ensure isolation between participants
///
/// # Example
///
/// ```rust,ignore
/// // User message
/// let user_msg = ChatMessage::user("What is the capital of France?");
///
/// // Streaming assistant response
/// let streaming_msg = ChatMessage {
///     content: "The capital".into(),
///     sender: "Assistant".into(),
///     role: MessageRole::Assistant,
///     timestamp: current_timestamp(),
///     is_streaming: true,
///     session_id: Some("session_123".into()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message content
    pub content: String,
    /// Sender ID (participant name or "user")
    pub sender: String,
    /// Message role (user, assistant, system)
    pub role: MessageRole,
    /// Unix timestamp in milliseconds
    pub timestamp: u64,
    /// Whether this is a streaming/partial message
    pub is_streaming: bool,
    /// Session/conversation ID for streaming consolidation
    pub session_id: Option<String>,
}

impl ChatMessage {
    /// Create user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            sender: "user".to_string(),
            role: MessageRole::User,
            timestamp: current_timestamp(),
            is_streaming: false,
            session_id: None,
        }
    }

    /// Create assistant message
    pub fn assistant(content: impl Into<String>, sender: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            sender: sender.into(),
            role: MessageRole::Assistant,
            timestamp: current_timestamp(),
            is_streaming: false,
            session_id: None,
        }
    }
}

/// Message role in conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Control command for dataflow orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlCommand {
    /// Command name (e.g., "start", "stop", "pause", "reset")
    pub command: String,
    /// Command parameters
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
}

impl ControlCommand {
    /// Create a simple command without parameters
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            params: HashMap::new(),
        }
    }

    /// Add parameter
    pub fn with_param(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Create "start" command
    pub fn start() -> Self {
        Self::new("start")
    }

    /// Create "stop" command
    pub fn stop() -> Self {
        Self::new("stop")
    }

    /// Create "reset" command
    pub fn reset() -> Self {
        Self::new("reset")
    }

    /// Create "send_prompt" command with message
    pub fn send_prompt(message: impl Into<String>) -> Self {
        Self::new("send_prompt").with_param("message", serde_json::Value::String(message.into()))
    }
}

/// Metadata from dora events
#[derive(Debug, Clone, Default)]
pub struct EventMetadata {
    /// Key-value pairs
    pub values: HashMap<String, String>,
}

impl EventMetadata {
    /// Get value by key
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    /// Get session status
    pub fn session_status(&self) -> Option<&str> {
        self.get("session_status")
    }

    /// Get question ID
    pub fn question_id(&self) -> Option<&str> {
        self.get("question_id")
    }

    /// Get participant ID
    pub fn participant_id(&self) -> Option<&str> {
        self.get("participant_id")
    }
}

/// Get current unix timestamp in milliseconds
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
