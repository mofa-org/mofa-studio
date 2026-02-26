//! Background system monitor for CPU, memory, GPU, network, and disk usage
//!
//! This module provides a thread-safe system monitor that polls system metrics
//! in a background thread, keeping the UI thread free.
//!
//! ## Metrics Available
//!
//! - **CPU**: Global CPU usage percentage
//! - **Memory**: RAM usage percentage
//! - **GPU**: GPU utilization (macOS via IOKit, Linux/Windows via NVML planned)
//! - **VRAM**: Video memory usage percentage
//! - **Network**: Bytes sent/received per second
//! - **Disk**: Bytes read/written per second
//!
//! ## GPU monitoring:
//! - macOS: Uses IOKit via `ioreg` command to query GPU statistics
//! - Linux/Windows with NVIDIA: Uses nvml-wrapper (commented out, enable if needed)

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{System, Networks, Disks};

/// Shared system stats, updated by background thread
struct SystemStats {
    /// CPU usage scaled to 0-10000 (representing 0.00% to 100.00%)
    cpu_usage: AtomicU32,
    /// Memory usage scaled to 0-10000 (representing 0.00% to 100.00%)
    memory_usage: AtomicU32,
    /// GPU utilization scaled to 0-10000 (representing 0.00% to 100.00%)
    gpu_usage: AtomicU32,
    /// VRAM usage scaled to 0-10000 (representing 0.00% to 100.00%)
    vram_usage: AtomicU32,
    /// Whether GPU monitoring is available
    gpu_available: AtomicBool,
    /// Network bytes transmitted per second
    network_tx_bytes: AtomicU64,
    /// Network bytes received per second
    network_rx_bytes: AtomicU64,
    /// Disk bytes read per second
    disk_read_bytes: AtomicU64,
    /// Disk bytes written per second
    disk_write_bytes: AtomicU64,
}

impl SystemStats {
    fn new() -> Self {
        Self {
            cpu_usage: AtomicU32::new(0),
            memory_usage: AtomicU32::new(0),
            gpu_usage: AtomicU32::new(0),
            vram_usage: AtomicU32::new(0),
            gpu_available: AtomicBool::new(false),
            network_tx_bytes: AtomicU64::new(0),
            network_rx_bytes: AtomicU64::new(0),
            disk_read_bytes: AtomicU64::new(0),
            disk_write_bytes: AtomicU64::new(0),
        }
    }
}

/// Global system monitor instance
static SYSTEM_MONITOR: OnceLock<Arc<SystemStats>> = OnceLock::new();

// ============================================================================ 
// Network and Disk I/O snapshot helpers
// ============================================================================

/// Snapshot of network statistics for rate calculation
struct NetworkSnapshot {
    tx_bytes: u64,
    rx_bytes: u64,
}

impl NetworkSnapshot {
    fn capture(networks: &Networks) -> Self {
        let mut tx_bytes = 0u64;
        let mut rx_bytes = 0u64;
        
        for (_interface_name, network) in networks.iter() {
            tx_bytes += network.transmitted();
            rx_bytes += network.received();
        }
        
        Self { tx_bytes, rx_bytes }
    }
}

/// Snapshot of disk statistics for rate calculation
struct DiskSnapshot {
    read_bytes: u64,
    write_bytes: u64,
}

impl DiskSnapshot {
    fn capture(disks: &Disks) -> Self {
        let mut read_bytes = 0u64;
        let mut write_bytes = 0u64;
        
        for disk in disks.iter() {
            // sysinfo 0.32: DiskUsage has read_bytes and written_bytes fields
            let usage = disk.usage();
            read_bytes += usage.read_bytes;
            write_bytes += usage.written_bytes;
        }
        
        Self { read_bytes, write_bytes }
    }
}

// ============================================================================
// macOS GPU monitoring using IOKit via ioreg command
// ============================================================================

#[cfg(target_os = "macos")]
mod macos_gpu {
    use std::process::Command;

    /// GPU statistics from macOS IOKit
    #[derive(Debug, Default)]
    pub struct MacOSGpuStats {
        pub gpu_utilization: Option<f64>, // 0.0 - 1.0
        pub vram_used_mb: Option<u64>,
        pub vram_total_mb: Option<u64>,
    }

    /// Query GPU statistics from macOS using ioreg
    /// Parses IOAccelerator data for GPU utilization and VRAM info
    ///
    /// Apple Silicon format (M1/M2/M3):
    /// "PerformanceStatistics" = {"Device Utilization %"=0,"In use system memory"=1732460544,...}
    pub fn query_gpu_stats() -> MacOSGpuStats {
        let mut stats = MacOSGpuStats::default();

        // Query IOAccelerator for GPU statistics
        let output = Command::new("ioreg")
            .args(["-r", "-d", "1", "-c", "IOAccelerator"])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Find PerformanceStatistics line and parse values from dictionary
                for line in stdout.lines() {
                    let line = line.trim();

                    // Parse PerformanceStatistics dictionary
                    // Format: "PerformanceStatistics" = {"Device Utilization %"=0,"In use system memory"=123,...}
                    if line.contains("PerformanceStatistics") && line.contains("{") {
                        // Extract Device Utilization %
                        if let Some(util) = extract_dict_value(line, "Device Utilization %") {
                            stats.gpu_utilization = Some(util / 100.0);
                        }

                        // Extract memory stats for Apple Silicon (unified memory)
                        // "In use system memory" is GPU memory currently in use (bytes)
                        // "Alloc system memory" is total allocated for GPU (bytes)
                        if let Some(used_bytes) = extract_dict_value(line, "In use system memory\"")
                        {
                            stats.vram_used_mb = Some((used_bytes as u64) / (1024 * 1024));
                        }
                        if let Some(alloc_bytes) = extract_dict_value(line, "Alloc system memory") {
                            stats.vram_total_mb = Some((alloc_bytes as u64) / (1024 * 1024));
                        }

                        // Found what we need, stop searching
                        if stats.gpu_utilization.is_some() {
                            break;
                        }
                    }

                    // Fallback: discrete GPU format (Intel Macs with AMD/NVIDIA)
                    // VRAM Total (in MB)
                    if line.contains("VRAM,totalMB") {
                        if let Some(value) = extract_simple_value(line) {
                            stats.vram_total_mb = Some(value as u64);
                        }
                    }
                    // VRAM Free (calculate used from total - free)
                    if line.contains("VRAM,freeMB") {
                        if let Some(value) = extract_simple_value(line) {
                            if let Some(total) = stats.vram_total_mb {
                                stats.vram_used_mb = Some(total.saturating_sub(value as u64));
                            }
                        }
                    }
                }
            }
        }

        // Fallback for VRAM if not found
        if stats.vram_total_mb.is_none() {
            // Apple Silicon uses unified memory, estimate GPU portion
            let sys = sysinfo::System::new_all();
            let total_mb = sys.total_memory() / (1024 * 1024);
            // Apple Silicon can use up to ~75% of system memory for GPU
            stats.vram_total_mb = Some((total_mb * 3 / 4) as u64);
            stats.vram_used_mb = Some(0);
        }

        stats
    }

    /// Extract a value from dictionary format: "Key"=123 or "Key"=123,
    fn extract_dict_value(line: &str, key: &str) -> Option<f64> {
        // Find the key in the line
        let key_pos = line.find(key)?;
        let after_key = &line[key_pos + key.len()..];

        // Find the = sign after the key
        let eq_pos = after_key.find('=')?;
        let after_eq = &after_key[eq_pos + 1..];

        // Extract the number (stop at comma, space, or closing brace)
        let value_str: String = after_eq
            .trim()
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();

        value_str.parse().ok()
    }

    /// Extract a simple value from format: "key" = 123
    fn extract_simple_value(line: &str) -> Option<f64> {
        if let Some(eq_pos) = line.rfind('=') {
            let value_part = line[eq_pos + 1..].trim();
            let clean_value: String = value_part
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            clean_value.parse().ok()
        } else {
            None
        }
    }
}

/// Start the background system monitor thread if not already running.
/// This should be called once at app startup.
pub fn start_system_monitor() {
    SYSTEM_MONITOR.get_or_init(|| {
        let stats = Arc::new(SystemStats::new());
        let stats_clone = Arc::clone(&stats);

        thread::Builder::new()
            .name("system-monitor".to_string())
            .spawn(move || {
                let mut sys = System::new_all();

                // ============================================================
                // macOS GPU monitoring initialization
                // ============================================================
                #[cfg(target_os = "macos")]
                {
                    // Test if we can query GPU stats
                    let test_stats = macos_gpu::query_gpu_stats();
                    if test_stats.gpu_utilization.is_some() || test_stats.vram_total_mb.is_some() {
                        stats_clone.gpu_available.store(true, Ordering::Relaxed);
                        log::info!("GPU monitoring enabled (macOS IOKit)");
                    } else {
                        // Even if we can't get utilization, mark as available for VRAM display
                        stats_clone.gpu_available.store(true, Ordering::Relaxed);
                        log::info!("GPU monitoring enabled (macOS - limited stats available)");
                    }
                }

                #[cfg(not(target_os = "macos"))]
                {
                    log::info!("GPU monitoring not available (NVIDIA support commented out, enable in system_monitor.rs)");
                }

                // Initialize network and disk monitoring
                let mut networks = Networks::new_with_refreshed_list();
                let mut disks = Disks::new_with_refreshed_list();
                let mut last_network_stats = NetworkSnapshot::capture(&networks);
                let mut last_disk_stats = DiskSnapshot::capture(&disks);
                let mut last_time = Instant::now();

                loop {
                    // Sleep first to measure rate over interval
                    thread::sleep(Duration::from_secs(1));

                    let now = Instant::now();
                    let elapsed_secs = now.duration_since(last_time).as_secs_f64();
                    last_time = now;

                    // Refresh CPU and memory
                    sys.refresh_cpu_usage();
                    sys.refresh_memory();

                    // Get CPU usage (0.0 - 100.0)
                    let cpu = sys.global_cpu_usage();
                    let cpu_scaled = (cpu * 100.0) as u32; // Scale to 0-10000
                    stats_clone.cpu_usage.store(cpu_scaled, Ordering::Relaxed);

                    // Get memory usage
                    let total_memory = sys.total_memory();
                    let used_memory = sys.used_memory();
                    let memory_pct = if total_memory > 0 {
                        (used_memory as f64 / total_memory as f64 * 10000.0) as u32
                    } else {
                        0
                    };
                    stats_clone.memory_usage.store(memory_pct, Ordering::Relaxed);

                    // ========================================================
                    // macOS GPU monitoring
                    // ========================================================
                    #[cfg(target_os = "macos")]
                    {
                        let gpu_stats = macos_gpu::query_gpu_stats();

                        // GPU utilization
                        if let Some(util) = gpu_stats.gpu_utilization {
                            let gpu_pct = (util * 10000.0) as u32;
                            stats_clone.gpu_usage.store(gpu_pct, Ordering::Relaxed);
                        }

                        // VRAM usage
                        if let (Some(used), Some(total)) = (gpu_stats.vram_used_mb, gpu_stats.vram_total_mb) {
                            if total > 0 {
                                let vram_pct = ((used as f64 / total as f64) * 10000.0) as u32;
                                stats_clone.vram_usage.store(vram_pct, Ordering::Relaxed);
                            }
                        }
                    }

                    // ========================================================
                    // Network I/O monitoring
                    // ========================================================
                    networks.refresh();
                    let current_network = NetworkSnapshot::capture(&networks);
                    if elapsed_secs > 0.0 {
                        let tx_rate = ((current_network.tx_bytes.saturating_sub(last_network_stats.tx_bytes)) as f64 / elapsed_secs) as u64;
                        let rx_rate = ((current_network.rx_bytes.saturating_sub(last_network_stats.rx_bytes)) as f64 / elapsed_secs) as u64;
                        stats_clone.network_tx_bytes.store(tx_rate, Ordering::Relaxed);
                        stats_clone.network_rx_bytes.store(rx_rate, Ordering::Relaxed);
                    }
                    last_network_stats = current_network;

                    // ========================================================
                    // Disk I/O monitoring
                    // ========================================================
                    disks.refresh();
                    let current_disk = DiskSnapshot::capture(&disks);
                    if elapsed_secs > 0.0 {
                        let read_rate = ((current_disk.read_bytes.saturating_sub(last_disk_stats.read_bytes)) as f64 / elapsed_secs) as u64;
                        let write_rate = ((current_disk.write_bytes.saturating_sub(last_disk_stats.write_bytes)) as f64 / elapsed_secs) as u64;
                        stats_clone.disk_read_bytes.store(read_rate, Ordering::Relaxed);
                        stats_clone.disk_write_bytes.store(write_rate, Ordering::Relaxed);
                    }
                    last_disk_stats = current_disk;
                }
            })
            .expect("Failed to spawn system monitor thread");

        stats
    });
}

/// Get current CPU usage as a value between 0.0 and 1.0
pub fn get_cpu_usage() -> f64 {
    SYSTEM_MONITOR
        .get()
        .map(|stats| stats.cpu_usage.load(Ordering::Relaxed) as f64 / 10000.0)
        .unwrap_or(0.0)
}

/// Get current memory usage as a value between 0.0 and 1.0
pub fn get_memory_usage() -> f64 {
    SYSTEM_MONITOR
        .get()
        .map(|stats| stats.memory_usage.load(Ordering::Relaxed) as f64 / 10000.0)
        .unwrap_or(0.0)
}

/// Get current GPU utilization as a value between 0.0 and 1.0
/// Returns 0.0 if GPU monitoring is not available
pub fn get_gpu_usage() -> f64 {
    SYSTEM_MONITOR
        .get()
        .map(|stats| stats.gpu_usage.load(Ordering::Relaxed) as f64 / 10000.0)
        .unwrap_or(0.0)
}

/// Get current VRAM usage as a value between 0.0 and 1.0
/// Returns 0.0 if GPU monitoring is not available
pub fn get_vram_usage() -> f64 {
    SYSTEM_MONITOR
        .get()
        .map(|stats| stats.vram_usage.load(Ordering::Relaxed) as f64 / 10000.0)
        .unwrap_or(0.0)
}

/// Check if GPU monitoring is available
pub fn is_gpu_available() -> bool {
    SYSTEM_MONITOR
        .get()
        .map(|stats| stats.gpu_available.load(Ordering::Relaxed))
        .unwrap_or(false)
}

// ============================================================================
// Network I/O metrics
// ============================================================================

/// Get current network transmit rate in bytes per second
pub fn get_network_tx_rate() -> u64 {
    SYSTEM_MONITOR
        .get()
        .map(|stats| stats.network_tx_bytes.load(Ordering::Relaxed))
        .unwrap_or(0)
}

/// Get current network receive rate in bytes per second
pub fn get_network_rx_rate() -> u64 {
    SYSTEM_MONITOR
        .get()
        .map(|stats| stats.network_rx_bytes.load(Ordering::Relaxed))
        .unwrap_or(0)
}

/// Get combined network I/O rate (tx + rx) in bytes per second
pub fn get_network_total_rate() -> u64 {
    SYSTEM_MONITOR
        .get()
        .map(|stats| {
            stats.network_tx_bytes.load(Ordering::Relaxed)
                + stats.network_rx_bytes.load(Ordering::Relaxed)
        })
        .unwrap_or(0)
}

/// Format network rate as human-readable string (e.g., "1.5 MB/s")
pub fn format_network_rate(bytes_per_sec: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes_per_sec >= GB {
        format!("{:.2} GB/s", bytes_per_sec as f64 / GB as f64)
    } else if bytes_per_sec >= MB {
        format!("{:.2} MB/s", bytes_per_sec as f64 / MB as f64)
    } else if bytes_per_sec >= KB {
        format!("{:.2} KB/s", bytes_per_sec as f64 / KB as f64)
    } else {
        format!("{} B/s", bytes_per_sec)
    }
}

// ============================================================================
// Disk I/O metrics
// ============================================================================

/// Get current disk read rate in bytes per second
pub fn get_disk_read_rate() -> u64 {
    SYSTEM_MONITOR
        .get()
        .map(|stats| stats.disk_read_bytes.load(Ordering::Relaxed))
        .unwrap_or(0)
}

/// Get current disk write rate in bytes per second
pub fn get_disk_write_rate() -> u64 {
    SYSTEM_MONITOR
        .get()
        .map(|stats| stats.disk_write_bytes.load(Ordering::Relaxed))
        .unwrap_or(0)
}

/// Get combined disk I/O rate (read + write) in bytes per second
pub fn get_disk_total_rate() -> u64 {
    SYSTEM_MONITOR
        .get()
        .map(|stats| {
            stats.disk_read_bytes.load(Ordering::Relaxed)
                + stats.disk_write_bytes.load(Ordering::Relaxed)
        })
        .unwrap_or(0)
}

/// Format disk rate as human-readable string (e.g., "150.5 MB/s")
pub fn format_disk_rate(bytes_per_sec: u64) -> String {
    // Reuse network formatting logic
    format_network_rate(bytes_per_sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_network_rate() {
        assert_eq!(format_network_rate(500), "500 B/s");
        assert_eq!(format_network_rate(1024), "1.00 KB/s");
        assert_eq!(format_network_rate(1536), "1.50 KB/s");
        assert_eq!(format_network_rate(1048576), "1.00 MB/s");
        assert_eq!(format_network_rate(1572864), "1.50 MB/s");
        assert_eq!(format_network_rate(1073741824), "1.00 GB/s");
    }

    #[test]
    fn test_format_disk_rate() {
        assert_eq!(format_disk_rate(1024), "1.00 KB/s");
        assert_eq!(format_disk_rate(1048576), "1.00 MB/s");
    }
}
