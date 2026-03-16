// Thread-safe metrics store with dirty tracking.

use crate::{AgentStatus, MetricEvent, PipelineTrace, SystemSnapshot};
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct MetricsStore {
    agents: RwLock<HashMap<String, AgentStatus>>,
    traces: RwLock<VecDeque<PipelineTrace>>,
    snapshots: RwLock<VecDeque<SystemSnapshot>>,

    agents_dirty: AtomicBool,
    traces_dirty: AtomicBool,
    snapshots_dirty: AtomicBool,

    broadcast_tx: RwLock<Option<broadcast::Sender<String>>>,
}

// Ring buffer caps
const MAX_TRACES: usize = 200;
const MAX_SNAPSHOTS: usize = 300;

impl MetricsStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            agents: RwLock::new(HashMap::new()),
            traces: RwLock::new(VecDeque::new()),
            snapshots: RwLock::new(VecDeque::new()),
            agents_dirty: AtomicBool::new(false),
            traces_dirty: AtomicBool::new(false),
            snapshots_dirty: AtomicBool::new(false),
            broadcast_tx: RwLock::new(None),
        })
    }

    pub fn init_broadcast(&self, tx: broadcast::Sender<String>) {
        *self.broadcast_tx.write() = Some(tx);
    }

    pub fn subscribe(&self) -> Option<broadcast::Receiver<String>> {
        self.broadcast_tx.read().as_ref().map(|tx| tx.subscribe())
    }

    fn broadcast_event(&self, event: &MetricEvent) {
        if let Some(ref tx) = *self.broadcast_tx.read() {
            if let Ok(json) = serde_json::to_string(event) {
                let _ = tx.send(json);
            }
        }
    }

    pub fn record_agent(&self, status: AgentStatus) {
        self.broadcast_event(&MetricEvent::AgentStatusChanged(status.clone()));
        self.agents.write().insert(status.id.clone(), status);
        self.agents_dirty.store(true, Ordering::Release);
    }

    pub fn push_trace(&self, trace: PipelineTrace) {
        self.broadcast_event(&MetricEvent::PipelineCompleted(trace.clone()));
        let mut buf = self.traces.write();
        if buf.len() >= MAX_TRACES {
            buf.pop_front();
        }
        buf.push_back(trace);
        self.traces_dirty.store(true, Ordering::Release);
    }

    pub fn push_snapshot(&self, snap: SystemSnapshot) {
        self.broadcast_event(&MetricEvent::SystemSnapshot(snap.clone()));
        let mut buf = self.snapshots.write();
        if buf.len() >= MAX_SNAPSHOTS {
            buf.pop_front();
        }
        buf.push_back(snap);
        self.snapshots_dirty.store(true, Ordering::Release);
    }

    pub fn take_agents_if_dirty(&self) -> Option<Vec<AgentStatus>> {
        if self.agents_dirty.swap(false, Ordering::AcqRel) {
            Some(self.agents.read().values().cloned().collect())
        } else {
            None
        }
    }

    pub fn take_traces_if_dirty(&self) -> Option<Vec<PipelineTrace>> {
        if self.traces_dirty.swap(false, Ordering::AcqRel) {
            Some(self.traces.read().iter().cloned().collect())
        } else {
            None
        }
    }

    pub fn take_snapshots_if_dirty(&self) -> Option<Vec<SystemSnapshot>> {
        if self.snapshots_dirty.swap(false, Ordering::AcqRel) {
            Some(self.snapshots.read().iter().cloned().collect())
        } else {
            None
        }
    }

    pub fn get_agents(&self) -> Vec<AgentStatus> {
        self.agents.read().values().cloned().collect()
    }

    pub fn get_latest_snapshot(&self) -> Option<SystemSnapshot> {
        self.snapshots.read().back().cloned()
    }

    pub fn get_traces(&self, n: usize) -> Vec<PipelineTrace> {
        let buf = self.traces.read();
        buf.iter().rev().take(n).rev().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{unix_ms, AgentState};

    fn sample_agent(id: &str, state: AgentState) -> AgentStatus {
        AgentStatus {
            id: id.into(),
            name: id.into(),
            status: state,
            last_seen_ms: unix_ms(),
        }
    }

    fn sample_snapshot() -> SystemSnapshot {
        SystemSnapshot {
            cpu_pct: 0.45,
            memory_pct: 0.60,
            gpu_pct: 0.0,
            pipeline_latency_p50_ms: 120.0,
            uptime_secs: 30,
            timestamp_ms: unix_ms(),
        }
    }

    #[test]
    fn dirty_flag_cleared_on_read() {
        let store = MetricsStore::new();
        assert!(store.take_agents_if_dirty().is_none());

        store.record_agent(sample_agent("n1", AgentState::Active));
        let agents = store.take_agents_if_dirty().unwrap();
        assert_eq!(agents.len(), 1);

        // Not dirty anymore
        assert!(store.take_agents_if_dirty().is_none());
    }

    #[test]
    fn agent_upsert() {
        let store = MetricsStore::new();
        store.record_agent(sample_agent("n1", AgentState::Idle));
        store.record_agent(sample_agent("n1", AgentState::Active));

        let agents = store.get_agents();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].status, AgentState::Active);
    }

    #[test]
    fn trace_ring_buffer_cap() {
        let store = MetricsStore::new();
        for i in 0..(MAX_TRACES + 50) {
            store.push_trace(PipelineTrace {
                asr_start_ms: i as u64,
                asr_end_ms: i as u64 + 10,
                llm_start_ms: 0,
                llm_end_ms: 0,
                tts_start_ms: 0,
                tts_end_ms: 0,
                tokens_per_sec: 0.0,
            });
        }
        let traces = store.get_traces(999);
        assert_eq!(traces.len(), MAX_TRACES);
        // Oldest evicted
        assert_eq!(traces[0].asr_start_ms, 50);
    }

    #[test]
    fn snapshot_ring_buffer_cap() {
        let store = MetricsStore::new();
        for _ in 0..(MAX_SNAPSHOTS + 10) {
            store.push_snapshot(sample_snapshot());
        }
        let snaps = store.take_snapshots_if_dirty().unwrap();
        assert_eq!(snaps.len(), MAX_SNAPSHOTS);
    }

    #[test]
    fn get_latest_snapshot_returns_newest() {
        let store = MetricsStore::new();
        store.push_snapshot(SystemSnapshot {
            cpu_pct: 0.1,
            ..sample_snapshot()
        });
        store.push_snapshot(SystemSnapshot {
            cpu_pct: 0.9,
            ..sample_snapshot()
        });
        let latest = store.get_latest_snapshot().unwrap();
        assert!((latest.cpu_pct - 0.9).abs() < f64::EPSILON);
    }
}
