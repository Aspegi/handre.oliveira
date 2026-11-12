//! Decision demo: 3 scenarios showing multi-objective route selection.
//! Run: cargo run -p rnfc-decision --example decision_demo

use rnfc_certchain::{CertChain, SafetyState};
use rnfc_decision::{DecisionContext, DecisionEngine};
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Decision Demo: Multi-Objective Route Selection ===\n");
    let engine = DecisionEngine::new();
    let start = [0.0f32, 0.0];
    let target = [100.0f32, 0.0];
    let mut chain = CertChain::new(1, "decision_demo");
    let mut report = String::from("# Decision Rationale Report\n\n");

    let scenarios = vec![
        ("Calm conditions", DecisionContext {
            wind: [0.0, 0.0],
            obstacles: [([1000.0, 1000.0], 1.0); 3],
            battery_pct: 100.0,
        }),
        ("Obstacle on direct path", DecisionContext {
            wind: [0.0, 0.0],
            obstacles: [([50.0, 0.0], 8.0); 3],
            battery_pct: 100.0,
        }),
        ("Strong wind + low battery", DecisionContext {
            wind: [0.0, 12.0],
            obstacles: [([1000.0, 1000.0], 1.0); 3],
            battery_pct: 25.0,
        }),
    ];

    for (i, (label, ctx)) in scenarios.iter().enumerate() {
        println!("--- Scenario {}: {} ---", i + 1, label);
        let d = engine.decide(start, target, ctx);
        println!("  Chosen: {}", d.chosen);
        println!("  Rationale: {}", d.rationale);
        println!("  Scores:");
        for s in &d.scores {
            println!("    {:<14} cost={:7.2}  dist={:5.1}m  wind={:5.2}  obs={:5.2}  batt={:5.2}",
                s.name, s.cost, s.distance_m, s.wind_cost, s.obstacle_cost, s.battery_cost);
        }
        let proof = engine.prove(&chain, &d);
        chain.append(i as f32, 1, proof, SafetyState::Nominal).map_err(|e| format!("{:?}", e))?;
        println!("  Proof: 0x{:016x} (recorded in chain)\n", proof);

        report.push_str(&format!("## Scenario {}: {}\n\n", i + 1, label));
        report.push_str(&format!("- **Chosen route**: `{}`\n", d.chosen));
        report.push_str(&format!("- **Rationale**: {}\n", d.rationale));
        report.push_str(&format!("- **Proof hash**: `0x{:016x}`\n\n", proof));
        report.push_str("| Route | Cost | Distance | Wind | Obstacle | Battery |\n");
        report.push_str("|---|---|---|---|---|---|\n");
        for s in &d.scores {
            report.push_str(&format!("| {} | {:.2} | {:.1}m | {:.2} | {:.2} | {:.2} |\n",
                s.name, s.cost, s.distance_m, s.wind_cost, s.obstacle_cost, s.battery_cost));
        }
        report.push_str("\n");
    }

    println!("Chain: {} blocks, root=0x{:016x}", chain.block_count(), chain.root_hash());
    println!("Chain verified: {}", chain.verify());

    let mut f = File::create("decision_rationale_report.md")?;
    f.write_all(report.as_bytes())?;
    println!("\n[OK] decision_rationale_report.md written ({} bytes)", report.len());
    Ok(())
}
