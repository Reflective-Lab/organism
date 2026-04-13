// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Binary entry point for Spike 4: Batch Anomaly Detection.

use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let result = spike_batch_anomaly::run_batch_anomaly_verbose().await?;

    tracing::info!(
        "Pipeline complete — {} users flagged, {} total convergence cycles",
        result.flagged_users,
        result.total_cycles,
    );

    Ok(())
}
