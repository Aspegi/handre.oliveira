//! Swarm demo: 3 drones, consensus + split-brain + safety case.
//! Run: cargo run -p rnfc-swarm-certis --example swarm_flight

use rnfc_swarm_certis::{DistributedCertisRecorder, SwarmSafetyCase};
use std::fs;

fn main() {
    println!("=== Swarm: 3 Drones with Distributed Consensus ===\n");

    // === PHASE 1: Healthy swarm (converged) ===
    println!("--- Phase 1: 3 drones, all agree ---\n");

    let mut d1 = DistributedCertisRecorder::new(1, "swarm_alpha");
    let mut d2 = DistributedCertisRecorder::new(2, "swarm_alpha");
    let mut d3 = DistributedCertisRecorder::new(3, "swarm_alpha");

    // Each drone learns peer states (0 = no events, valid baseline)
    d1.sync_with_peer(2, 0);
    d1.sync_with_peer(3, 0);
    d2.sync_with_peer(1, 0);
    d2.sync_with_peer(3, 0);
    d3.sync_with_peer(1, 0);
    d3.sync_with_peer(2, 0);

    let c1 = d1.global_consensus();
    let c2 = d2.global_consensus();
    let c3 = d3.global_consensus();

    println!("D1 consensus: 0x{:016x}", c1);
    println!("D2 consensus: 0x{:016x}", c2);
    println!("D3 consensus: 0x{:016x}", c3);

    if c1 == c2 && c2 == c3 {
        println!("[CONVERGED] All 3 drones agree on consensus hash\n");
    } else {
        println!("[SPLIT-BRAIN] Drones disagree (unexpected)\n");
    }

    println!("Local chain valid: d1={} d2={} d3={}",
        d1.verify_local_chain(), d2.verify_local_chain(), d3.verify_local_chain());

    // === PHASE 2: Split-brain detection ===
    println!("\n--- Phase 2: Simulated split-brain ---\n");

    let mut sa = DistributedCertisRecorder::new(10, "swarm_beta");
    let mut sb = DistributedCertisRecorder::new(20, "swarm_beta");

    // Drone 10 thinks 20 has hash 0xAAAA; Drone 20 thinks 10 has 0xBBBB
    sa.sync_with_peer(20, 0xAAAA);
    sb.sync_with_peer(10, 0xBBBB);

    let ca = sa.global_consensus();
    let cb = sb.global_consensus();

    println!("Drone 10 consensus: 0x{:016x}", ca);
    println!("Drone 20 consensus: 0x{:016x}", cb);

    if ca == cb {
        println!("[CONVERGED] (unexpected)\n");
    } else {
        println!("[SPLIT-BRAIN DETECTED] Consensus hashes differ\n");
    }

    // === PHASE 3: Swarm safety case ===
    println!("--- Phase 3: Safety case (from healthy swarm) ---\n");

    let case = SwarmSafetyCase::generate(&[d1, d2, d3]);
    let md = case.to_markdown();
    fs::write("swarm_safety_case.md", &md).unwrap();

    // Show key sections only
    for line in md.lines() {
        if line.starts_with("**") || line.starts_with("# ") || line.starts_with("## ") {
            println!("{}", line);
        }
    }

    println!("\n[OK] swarm_safety_case.md written ({} bytes)", md.len());
    println!("\n=== Summary ===");
    println!("  Phase 1: 3 drones converged on consensus hash");
    println!("  Phase 2: Split-brain detected when views differ");
    println!("  Phase 3: Safety case (GSN + FMEA + FTA) generated");
}
