//! Prometheus metrics for Sheriff daemon
//!
//! Provides observability metrics for monitoring the Sheriff daemon in production.

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge, register_gauge_vec, register_histogram_vec, CounterVec,
    Encoder, Gauge, GaugeVec, HistogramVec, TextEncoder,
};

lazy_static! {
    /// Histogram: sync duration per rig (seconds)
    pub static ref SYNC_DURATION: HistogramVec = register_histogram_vec!(
        "allbeads_sync_duration_seconds",
        "Duration of rig sync operations",
        &["rig_id"],
        vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0]
    )
    .expect("Failed to create sync_duration metric");

    /// Counter: API errors by type
    pub static ref API_ERRORS: CounterVec = register_counter_vec!(
        "allbeads_api_errors_total",
        "Total API errors by type",
        &["error_type", "integration"]
    )
    .expect("Failed to create api_errors metric");

    /// Gauge: message queue depth
    pub static ref MESSAGE_QUEUE_DEPTH: Gauge = register_gauge!(
        "allbeads_message_queue_depth",
        "Current depth of message queue"
    )
    .expect("Failed to create message_queue_depth metric");

    /// Counter: cache operations (hit/miss)
    pub static ref CACHE_OPERATIONS: CounterVec = register_counter_vec!(
        "allbeads_cache_operations_total",
        "Cache operations by type",
        &["operation"]
    )
    .expect("Failed to create cache_operations metric");

    /// Gauge: active locks count
    pub static ref ACTIVE_LOCKS: Gauge = register_gauge!(
        "allbeads_active_locks",
        "Number of currently active locks"
    )
    .expect("Failed to create active_locks metric");

    /// Gauge: number of shadows per rig
    pub static ref SHADOWS_PER_RIG: GaugeVec = register_gauge_vec!(
        "allbeads_shadows_count",
        "Number of shadow beads per rig",
        &["rig_id"]
    )
    .expect("Failed to create shadows_count metric");

    /// Gauge: daemon health status (1 = healthy, 0 = unhealthy)
    pub static ref HEALTH_STATUS: Gauge = register_gauge!(
        "allbeads_health_status",
        "Daemon health status (1 = healthy, 0 = unhealthy)"
    )
    .expect("Failed to create health_status metric");

    /// Counter: total sync cycles completed
    pub static ref SYNC_CYCLES: CounterVec = register_counter_vec!(
        "allbeads_sync_cycles_total",
        "Total sync cycles by status",
        &["status"]
    )
    .expect("Failed to create sync_cycles metric");
}

/// Record a rig sync duration
pub fn record_sync_duration(rig_id: &str, duration_secs: f64) {
    SYNC_DURATION
        .with_label_values(&[rig_id])
        .observe(duration_secs);
}

/// Increment API error counter
pub fn record_api_error(error_type: &str, integration: &str) {
    API_ERRORS
        .with_label_values(&[error_type, integration])
        .inc();
}

/// Set message queue depth
pub fn set_queue_depth(depth: i64) {
    MESSAGE_QUEUE_DEPTH.set(depth as f64);
}

/// Record cache hit
pub fn record_cache_hit() {
    CACHE_OPERATIONS.with_label_values(&["hit"]).inc();
}

/// Record cache miss
pub fn record_cache_miss() {
    CACHE_OPERATIONS.with_label_values(&["miss"]).inc();
}

/// Set active locks count
pub fn set_active_locks(count: i64) {
    ACTIVE_LOCKS.set(count as f64);
}

/// Set shadow count for a rig
pub fn set_shadows_count(rig_id: &str, count: i64) {
    SHADOWS_PER_RIG
        .with_label_values(&[rig_id])
        .set(count as f64);
}

/// Set health status
pub fn set_health_status(healthy: bool) {
    HEALTH_STATUS.set(if healthy { 1.0 } else { 0.0 });
}

/// Record a sync cycle completion
pub fn record_sync_cycle(status: &str) {
    SYNC_CYCLES.with_label_values(&[status]).inc();
}

/// Encode all metrics as Prometheus text format
pub fn encode_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        // Just verify metrics can be accessed without panic
        record_sync_duration("test-rig", 1.5);
        record_api_error("timeout", "jira");
        set_queue_depth(10);
        record_cache_hit();
        record_cache_miss();
        set_active_locks(5);
        set_shadows_count("test-rig", 100);
        set_health_status(true);
        record_sync_cycle("success");

        let output = encode_metrics();
        assert!(output.contains("allbeads_sync_duration_seconds"));
        assert!(output.contains("allbeads_api_errors_total"));
    }
}
