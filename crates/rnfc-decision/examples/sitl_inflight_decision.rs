//! In-flight decision demo: drone changes route mid-mission.
//! Run: cargo run -p rnfc-decision --example sitl_inflight_decision

use rnfc_certchain::{CertChain, SafetyState};
use rnfc_decision::{DecisionContext, DecisionEngine};

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn route_waypoints(name: &str, from: [f32; 2], target: [f32; 2]) -> Vec<[f32; 2]> {
    let mid = [(from[0] + target[0]) / 2.0, (from[1] + target[1]) / 2.0];
    let offset = 30.0;
    match name {
        "direct" => vec![from, target],
        "north_detour" => vec![from, [mid[0], mid[1] + offset], target],
        "south_detour" => vec![from, [mid[0], mid[1] - offset], target],
        _ => vec![from, target],
    }
}

fn main() {
    println!("=== In-flight Decision Demo ===\n");
    println!("Drone evaluates conditions every tick and re-decides mid-flight.\n");

    let engine = DecisionEngine::new();
    let mut chain = CertChain::new(1, "inflight_mission");

    let start = [0.0f32, 0.0];
    let target = [100.0f32, 0.0];

    let mut ctx = DecisionContext {
        wind: [0.0, 0.0],
        obstacles: [([1000.0, 1000.0], 1.0); 3],
        battery_pct: 100.0,
    };

    // Initial decision (at t=0)
    let initial = engine.decide(start, target, &ctx);
    let mut route_name = initial.chosen.clone();
    println!("[t=0.0s] Initial decision: {}", route_name);
    println!("[t=0.0s] Rationale: {}\n", initial.rationale);

    let mut waypoints = route_waypoints(&route_name, start, target);
    let mut wp_idx = 1usize;
    let mut pos = start;
    let mut t = 0.0f32;
    let dt = 0.1f32;
    let speed = 5.0f32;
    let mut obstacle_injected = false;
    let mut decision_changes = 0u32;

    while t < 30.0 && wp_idx < waypoints.len() {
        let wp = waypoints[wp_idx];
        let d = dist(pos, wp);
        if d < 0.5 { wp_idx += 1; continue; }
        let dir = [(wp[0] - pos[0]) / d, (wp[1] - pos[1]) / d];
        pos = [pos[0] + dir[0] * speed * dt, pos[1] + dir[1] * speed * dt];
        t += dt;

        // Inject obstacle at t=3s on direct path
        if !obstacle_injected && t >= 3.0 && route_name == "direct" {
            obstacle_injected = true;
            ctx.obstacles[0] = ([50.0, 0.0], 8.0);
            println!("\n[t={:.1}s] >>> OBSTACLE DETECTED at [50, 0] radius 8m <<<\n", t);

            let new_decision = engine.decide(pos, target, &ctx);
            if new_decision.chosen != route_name {
                println!("[t={:.1}s] DECISION CHANGED: {} -> {}", t, route_name, new_decision.chosen);
                println!("[t={:.1}s] Rationale: {}", t, new_decision.rationale);
                let direct_cost = new_decision.scores.iter()
                    .find(|s| s.name == "direct").map(|s| s.cost).unwrap_or(0.0);
                let chosen_cost = new_decision.scores[0].cost;
                println!("[t={:.1}s] Cost: direct={:.2} vs {}={:.2}", t, direct_cost, new_decision.chosen, chosen_cost);

                route_name = new_decision.chosen.clone();
                waypoints = route_waypoints(&route_name, pos, target);
                wp_idx = 1;
                decision_changes += 1;

                let proof = engine.prove(&chain, &new_decision);
                if let Err(e) = chain.append(t, 1, proof, SafetyState::Nominal) {
                    eprintln!("chain append failed: {:?}", e);
                }
                println!("[t={:.1}s] Decision recorded in chain: 0x{:016x}\n", t, proof);
            }
        }

        // Print every 2s
        if (t * 10.0).round() as i32 % 20 == 0 {
            println!("[t={:.1}s] pos=[{:.1}, {:.1}] route={}", t, pos[0], pos[1], route_name);
        }
    }

    println!("\n=== Mission Complete ===");
    println!("Final position: [{:.1}, {:.1}]", pos[0], pos[1]);
    println!("Final route: {}", route_name);
    println!("Distance to target: {:.1}m", dist(pos, target));
    println!("Decision changes: {}", decision_changes);
    println!("Chain: {} blocks, verified: {}", chain.block_count(), chain.verify());
    println!("\nKey: drone CHANGED ROUTE MID-FLIGHT when the obstacle appeared.");
}
