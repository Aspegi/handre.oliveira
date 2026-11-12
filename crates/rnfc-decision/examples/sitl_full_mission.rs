//! Full mission: decision + watch in a single flight.
//! Run: cargo run -p rnfc-decision --example sitl_full_mission

use rnfc_certchain::{CertChain, SafetyState};
use rnfc_decision::{DecisionContext, DecisionEngine};
use rnfc_watch::WatchManager;
use std::fs;
use std::io::Write;

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn route_wps(name: &str, from: [f32; 2], target: [f32; 2]) -> Vec<[f32; 2]> {
    let mid = [(from[0] + target[0]) / 2.0, (from[1] + target[1]) / 2.0];
    let off = 30.0;
    match name {
        "direct" => vec![from, target],
        "north_detour" => vec![from, [mid[0], mid[1] + off], target],
        "south_detour" => vec![from, [mid[0], mid[1] - off], target],
        _ => vec![from, target],
    }
}

fn static_frame() -> Vec<u8> {
    vec![128u8; 64 * 48 * 3]
}

fn motion_frame() -> Vec<u8> {
    let mut f = static_frame();
    for i in 5000..8000 {
        f[i] = 255;
    }
    f
}

fn main() {
    println!("=== RNFC Full Mission: Decision + Watch ===\n");

    let engine = DecisionEngine::new();
    let mut watch = WatchManager::new();
    let mut chain = CertChain::new(1, "full_mission");

    let start = [0.0f32, 0.0];
    let target = [100.0f32, 0.0];

    let mut ctx = DecisionContext {
        wind: [0.0, 0.0],
        obstacles: [([1000.0, 1000.0], 1.0); 3],
        battery_pct: 100.0,
    };

    let init = engine.decide(start, target, &ctx);
    let mut route = init.chosen.clone();
    let mut wps = route_wps(&route, start, target);
    let mut wi = 1usize;
    let mut pos = start;
    let mut t = 0.0f32;
    let dt = 0.1f32;
    let speed = 5.0f32;

    let mut motion_done = false;
    let mut obstacle_done = false;
    let mut zoom_restored = false;
    let mut watches = 0u32;
    let mut decisions = 0u32;

    println!(
        "[t=0.0s] Route={} FOV={:.0}",
        route,
        watch.zoom.current_fov_deg()
    );
    println!(
        "[t=0.0s] Target={:?} dist={:.0}m\n",
        target,
        dist(pos, target)
    );

    while t < 30.0 && wi < wps.len() {
        let wp = wps[wi];
        let d = dist(pos, wp);
        if d < 0.5 {
            wi += 1;
            continue;
        }
        let dir = [(wp[0] - pos[0]) / d, (wp[1] - pos[1]) / d];
        pos = [pos[0] + dir[0] * speed * dt, pos[1] + dir[1] * speed * dt];
        t += dt;
        watch.tick(dt);

        if !motion_done && t >= 2.0 {
            motion_done = true;
            let _ = watch.on_frame(&static_frame(), t - dt, pos, 15.0, 0.0, 0.0, &mut chain);
            if let Some(r) = watch.on_frame(&motion_frame(), t, pos, 15.0, 135.0, 18.0, &mut chain)
            {
                watches += 1;
                println!("[t={:.1}s] WATCH: motion sector=135 dist=18m", t);
                println!(
                    "[t={:.1}s]   zoom FOV 120 -> {:.0}",
                    t,
                    watch.zoom.current_fov_deg()
                );
                println!(
                    "[t={:.1}s]   photo={} class={}",
                    t, r.photo_file, r.classification
                );
                println!("[t={:.1}s]   proof=0x{:016x}", t, r.chain_proof);
            }
        }

        if watches > 0 && !zoom_restored && !watch.zoom.active {
            zoom_restored = true;
            println!(
                "[t={:.1}s] WATCH: zoom restored FOV={:.0}\n",
                t,
                watch.zoom.current_fov_deg()
            );
        }

        if !obstacle_done && t >= 5.0 && route == "direct" {
            obstacle_done = true;
            ctx.obstacles[0] = ([50.0, 0.0], 8.0);
            println!("[t={:.1}s] OBSTACLE at [50,0] r=8m", t);
            let nd = engine.decide(pos, target, &ctx);
            if nd.chosen != route {
                let dc = nd
                    .scores
                    .iter()
                    .find(|s| s.name == "direct")
                    .map(|s| s.cost)
                    .unwrap_or(0.0);
                let cc = nd.scores[0].cost;
                println!("[t={:.1}s] DECISION: {} -> {}", t, route, nd.chosen);
                println!(
                    "[t={:.1}s]   cost direct={:.2} vs {}={:.2}",
                    t, dc, nd.chosen, cc
                );
                route = nd.chosen.clone();
                wps = route_wps(&route, pos, target);
                wi = 1;
                decisions += 1;
                let p = engine.prove(&chain, &nd);
                let _ = chain.append(t, 1, p, SafetyState::Nominal);
                println!("[t={:.1}s]   proof=0x{:016x}\n", t, p);
            }
        }
    }

    println!("=== Mission Complete ===");
    println!("Final pos=[{:.1},{:.1}] route={}", pos[0], pos[1], route);
    println!("Target error={:.1}m", dist(pos, target));
    println!("Watches={} Decisions={}", watches, decisions);
    println!(
        "Chain: {} blocks, verified={}",
        chain.block_count(),
        chain.verify()
    );

    let mut f = fs::File::create("full_mission_report.md").unwrap();
    writeln!(f, "# Full Mission Report\n").unwrap();
    writeln!(f, "- Mission: full_mission").unwrap();
    writeln!(f, "- Final pos: [{:.1}, {:.1}]", pos[0], pos[1]).unwrap();
    writeln!(f, "- Target error: {:.1}m", dist(pos, target)).unwrap();
    writeln!(f, "- Watches: {}", watches).unwrap();
    writeln!(f, "- Decisions: {}\n", decisions).unwrap();
    writeln!(f, "## Timeline\n").unwrap();
    writeln!(f, "| t | Event | Detail |").unwrap();
    writeln!(f, "|---|---|---|").unwrap();
    writeln!(f, "| 0.0s | Start | route=direct FOV=120 |").unwrap();
    if watches > 0 {
        writeln!(
            f,
            "| 2.0s | WATCH | motion sector=135 zoom=30 photo=watch_001.ppm |"
        )
        .unwrap();
    }
    if decisions > 0 {
        writeln!(f, "| 5.0s | DECISION | direct -> north_detour |").unwrap();
    }
    writeln!(f, "| end | Arrival | error={:.1}m |", dist(pos, target)).unwrap();
    writeln!(f, "\n## Chain\n").unwrap();
    writeln!(
        f,
        "{} blocks, root=0x{:016x}, verified={}",
        chain.block_count(),
        chain.root_hash(),
        chain.verify()
    )
    .unwrap();
    println!("\n[OK] full_mission_report.md written");
}
