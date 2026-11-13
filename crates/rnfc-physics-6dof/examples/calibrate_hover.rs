//! Calibration: find hover thrust empirically.
use rnfc_physics_6dof::QuadrotorSim;

fn main() {
    let mut sim = QuadrotorSim::new();
    let dt = 0.01;
    let mut lo = 2.0;   // start below expected
    let mut hi = 10.0;  // start above expected

    for _ in 0..20 {
        let mid = (lo + hi) / 2.0;
        let mut s = QuadrotorSim::new();
        for _ in 0..100 { s.step([mid; 4], dt); }
        let z = s.position()[2];
        if z < 0.0 { lo = mid; } else { hi = mid; }
    }

    let hover = (lo + hi) / 2.0;
    println!("Calibrated hover thrust: {:.4} N per rotor", hover);

    // Verify
    let mut s = QuadrotorSim::new();
    for _ in 0..100 { s.step([hover; 4], dt); }
    let z = s.position()[2];
    println!("After 1s at calibrated thrust: z = {:.4} m", z);
}
