// Core metric types for the Observatory dashboard.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// Current Unix time in ms
pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// Node lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentState {
    Offline,
    Idle,
    Active,
    Error,
}

impl Default for AgentState {
    fn default() -> Self {
        Self::Offline
    }
}

// Single node status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub id: String,
    pub name: String,
    pub status: AgentState,
    pub last_seen_ms: u64,
}

// Per stage timestamps for one ASR, LLM, TTS pipeline run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTrace {
    pub asr_start_ms: u64,
    pub asr_end_ms: u64,
    pub llm_start_ms: u64,
    pub llm_end_ms: u64,
    pub tts_start_ms: u64,
    pub tts_end_ms: u64,
    // LLM tokens/sec (0.0 if unknown)
    pub tokens_per_sec: f64,
}

// System metrics snapshot normalized to 0 to 1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    // CPU
    pub cpu_pct: f64,
    // Memory
    pub memory_pct: f64,
    // GPU
    pub gpu_pct: f64,
    pub pipeline_latency_p50_ms: f64,
    // Uptime
    pub uptime_secs: u64,
    pub timestamp_ms: u64,
}

// WebSocket broadcast event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MetricEvent {
    AgentStatusChanged(AgentStatus),
    PipelineCompleted(PipelineTrace),
    SystemSnapshot(SystemSnapshot),
}
