//! 6-DOF flight demo: hover + forward motion with true dynamics.
//! Run: cargo run -p rnfc-physics-6dof --example sixdof_flight

use rnfc_physics_6dof::QuadrotorSim;

fn main() {
    println!("=== 6-DOF Flight Demo ===\n");

    let mut sim = QuadrotorSim::new();
    // Calibrated hover thrust (empirical, rotor-rs default params)
    let hover = 2.5274_f64;  // calibrated for rotor-rs default params
    let dt = 0.01_f64;

    let p0 = sim.position();
    println!("[t=0.00s] pos=[{:.2}, {:.2}, {:.2}]", p0[0], p0[1], p0[2]);

    // Phase 1: hover 1 s
    for _ in 0..100 { sim.step([hover; 4], dt); }
    let p = sim.position();
    let v = sim.velocity();
    println!("[t=1.00s] hover   pos=[{:.3}, {:.3}, {:.3}] vel=[{:.3}, {:.3}, {:.3}]",
        p[0], p[1], p[2], v[0], v[1], v[2]);

    // Phase 2: pitch forward (rear rotors more thrust) 0.5 s
    for _ in 0..50 {
        sim.step([hover * 0.7, hover * 0.7, hover * 1.3, hover * 1.3], dt);
    }
    let p = sim.position();
    let v = sim.velocity();
    let e = sim.euler();
    println!("[t=1.50s] forward pos=[{:.3}, {:.3}, {:.3}] vel=[{:.3}, {:.3}, {:.3}]",
        p[0], p[1], p[2], v[0], v[1], v[2]);
    println!("         euler roll={:.4} pitch={:.4} yaw={:.4} rad", e[0], e[1], e[2]);

    // Phase 3: back to hover 1 s
    for _ in 0..100 { sim.step([hover; 4], dt); }
    let p = sim.position();
    let v = sim.velocity();
    println!("[t=2.50s] settled pos=[{:.3}, {:.3}, {:.3}] vel=[{:.3}, {:.3}, {:.3}]",
        p[0], p[1], p[2], v[0], v[1], v[2]);

    println!();
    println!("[OK] 6-DOF physics running: quaternion attitude, coupled axes.");
}
