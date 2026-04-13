// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Spike 3 binary.

use organism_application::spike_capacity::run_capacity_planning_verbose;

fn main() {
    if let Err(err) = run_capacity_planning_verbose() {
        eprintln!("Spike 3 failed: {err}");
        std::process::exit(1);
    }
}
