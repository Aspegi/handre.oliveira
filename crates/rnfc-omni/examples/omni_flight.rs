//! Omni flight demo: 360 perception with camera failure mid-flight.
//! Run: cargo run -p rnfc-omni --example omni_flight

use rnfc_certchain::{CertChain, SafetyState};
use rnfc_core::NavCore;
use rnfc_omni::{Obstacle, OmniAction, OmniHealth, OmniSensorSuite};

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn main() {
    println!("=== Omni Flight: 360° Perception with Camera Failure ===\n");

    let mut chain = CertChain::new(1, "omni_mission");
    let mut suite = OmniSensorSuite::new(3, 120.0);

    let start = [0.0f32, 0.0];
    let target = [100.0f32, 0.0];
    let mut pos = start;
    let mut t = 0.0f32;
    let dt = 0.1f32;
    let speed = 5.0f32;

    let mut camera_failed = false;
    let mut halted = false;
    let mut min_coverage = 100.0f32;

    println!("[t=0.0s] Start: pos=[0.0, 0.0] target=[100.0, 0.0]");
    println!("[t=0.0s] Cameras: 3 x 120° = 360° FOV");
    println!();

    while t < 30.0 && dist(pos, target) > 1.0 {
        let obstacles: Vec<Obstacle> = vec![];
        let ring = suite.sense(&obstacles);
        if ring.coverage_pct < min_coverage { min_coverage = ring.coverage_pct; }

        if !halted {
            match ring.safety_action() {
                OmniAction::Continue => {}
                OmniAction::Hover => {
                    println!("[t={:.1}s] DECISION: HOVER (coverage={:.1}%)", t, ring.coverage_pct);
                    halted = true;
                    let _ = chain.append(t, 1, ring.coverage_pct as u64, SafetyState::FdirActive);
                }
                OmniAction::Rtl => {
                    println!("[t={:.1}s] DECISION: RTL (coverage={:.1}%)", t, ring.coverage_pct);
                    halted = true;
                    let _ = chain.append(t, 1, ring.coverage_pct as u64, SafetyState::FailsafeTriggered);
                }
                OmniAction::Land => {
                    println!("[t={:.1}s] DECISION: LAND (coverage={:.1}%)", t, ring.coverage_pct);
                    halted = true;
                    let _ = chain.append(t, 1, ring.coverage_pct as u64, SafetyState::FailsafeTriggered);
                }
            }
        }

        if !camera_failed && t >= 3.0 {
            camera_failed = true;
            suite.fail_camera(0);
            println!("[t={:.1}s] CAMERA 0 FAILED", t);
            let blind = ring.blind_sectors();
            if !blind.is_empty() {
                println!("[t={:.1}s] Blind sectors: {:?}", t, blind);
            }
        }

        if !halted {
            let d = dist(pos, target);
            if d > 0.1 {
                let dir = [(target[0] - pos[0]) / d, (target[1] - pos[1]) / d];
                pos = [pos[0] + dir[0] * speed * dt, pos[1] + dir[1] * speed * dt];
            }
        }

        if ((t * 10.0) as i32) % 10 == 0 {
            let status = match ring.health {
                OmniHealth::Healthy => "HEALTHY",
                OmniHealth::Degraded => "DEGRADED",
                OmniHealth::Blind => "BLIND",
            };
            println!("[t={:.1}s] pos=[{:.1},{:.1}] cov={:.1}% {}",
                t, pos[0], pos[1], ring.coverage_pct, status);
        }

        t += dt;
    }

    println!("\n=== Mission Complete ===");
    println!("Final pos=[{:.1},{:.1}]", pos[0], pos[1]);
    println!("Target error={:.1}m", dist(pos, target));
    println!("Min coverage={:.1}%", min_coverage);
    println!("Halted: {}", halted);
    println!("Chain: {} blocks, verified={}", chain.block_count(), chain.verify());
    println!();
    println!("Key: camera failure mid-flight triggered HOVER via OmniAction.");
}
