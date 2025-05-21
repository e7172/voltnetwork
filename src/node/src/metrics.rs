//! Metrics for the node daemon.

use anyhow::Result;
use lazy_static::lazy_static;
use prometheus::{
    register_counter, register_gauge, register_histogram, Counter, Gauge, Histogram, HistogramOpts,
    Opts,
};
use std::net::SocketAddr;
use warp::Filter;

lazy_static! {
    /// Counter for the number of transactions processed.
    pub static ref TRANSACTION_COUNTER: Counter = register_counter!(
        Opts::new(
            "transactions_total",
            "Total number of transactions processed"
        )
    )
    .unwrap();

    /// Counter for the number of updates received.
    pub static ref UPDATE_COUNTER: Counter = register_counter!(
        Opts::new(
            "updates_total",
            "Total number of updates received"
        )
    )
    .unwrap();

    /// Gauge for the number of connected peers.
    pub static ref PEER_COUNT: Gauge = register_gauge!(
        Opts::new(
            "peers",
            "Number of connected peers"
        )
    )
    .unwrap();

    /// Histogram for transaction processing time.
    pub static ref TRANSACTION_TIME: Histogram = register_histogram!(
        HistogramOpts::new(
            "transaction_processing_time_seconds",
            "Time to process a transaction"
        )
        .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
    )
    .unwrap();

    /// Histogram for proof verification time.
    pub static ref PROOF_VERIFICATION_TIME: Histogram = register_histogram!(
        HistogramOpts::new(
            "proof_verification_time_seconds",
            "Time to verify a proof"
        )
        .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0])
    )
    .unwrap();
}

/// Registers all metrics.
pub fn register_metrics() {
    // Metrics are registered via lazy_static
}

/// Starts the metrics server.
pub async fn start_metrics_server(addr: SocketAddr) -> Result<()> {
    let metrics_route = warp::path("metrics").map(|| {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    });

    tokio::spawn(async move {
        warp::serve(metrics_route).run(addr).await;
    });

    Ok(())
}
