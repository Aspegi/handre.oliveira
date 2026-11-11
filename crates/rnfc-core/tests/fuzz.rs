//! Fuzz tests: 10.000 inputs aleatorios por funcao critica.

use rnfc_abstractions::{GnssFix, ImuSample};
use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::fusion_mag::FusionMag;
use rnfc_core::guidance::Guidance;
use rnfc_core::NavCore;

const PI: f32 = core::f32::consts::PI;
const ITERS: usize = 10_000;

struct Rng { state: u32 }
impl Rng {
    fn new(seed: u32) -> Self { Self { state: seed.wrapping_add(1) } }
    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
    fn f32_01(&mut self) -> f32 { (self.next() >> 8) as f32 / 16777216.0 }
    fn range(&mut self, lo: f32, hi: f32) -> f32 { lo + self.f32_01() * (hi - lo) }
}

#[test]
fn fuzz_fusion_mag_10k_inputs() {
    let mut rng = Rng::new(1);
    let mut f = FusionMag::new(0.0);
    for i in 0..ITERS {
        let mag_n = rng.range(-200.0, 200.0);
        let mag_e = rng.range(-200.0, 200.0);
        let gyro = rng.range(-50.0, 50.0);
        let dt = rng.range(0.001, 0.1);
        f.update(mag_n, mag_e, gyro, dt);
        assert!(f.yaw.is_finite(), "iter {} yaw NaN", i);
        assert!(f.yaw >= -PI && f.yaw <= PI, "iter {} yaw fora: {}", i, f.yaw);
    }
}

#[test]
fn fuzz_guidance_10k_inputs() {
    let mut rng = Rng::new(2);
    for i in 0..ITERS {
        let mut g = Guidance::new();
        let pos = [rng.range(-10_000.0, 10_000.0), rng.range(-10_000.0, 10_000.0), 0.0];
        let (sp, _) = g.update(pos);
        assert!(sp[0].is_finite(), "iter {} roll NaN", i);
        assert!(sp[1].is_finite(), "iter {} pitch NaN", i);
        assert!(sp[2].is_finite(), "iter {} yaw NaN", i);
        assert!(sp[1] <= g.max_pitch + 1e-6, "iter {} pitch>max", i);
    }
}

#[test]
fn fuzz_attitude_controller_10k_inputs() {
    let mut rng = Rng::new(3);
    let mut c = AttitudeController::new();
    for i in 0..ITERS {
        let sp = [rng.range(-PI, PI), rng.range(-PI, PI), rng.range(-PI, PI)];
        let m = [rng.range(-PI, PI), rng.range(-PI, PI), rng.range(-PI, PI)];
        let dt = rng.range(0.001, 0.1);
        let o = c.update(sp, m, dt);
        for k in 0..3 { assert!(o[k].is_finite(), "iter {} axis {} NaN", i, k); }
    }
}

#[test]
fn fuzz_navcore_10k_steps() {
    let mut rng = Rng::new(4);
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    for i in 0..ITERS {
        let imu = ImuSample {
            accel: [rng.range(-50.0, 50.0), rng.range(-50.0, 50.0), rng.range(-50.0, 50.0)],
            gyro: [rng.range(-10.0, 10.0), rng.range(-10.0, 10.0), rng.range(-10.0, 10.0)],
            dt: 0.01,
        };
        let gnss = if rng.f32_01() > 0.3 {
            Some(GnssFix {
                position: [rng.range(-1000.0, 1000.0), rng.range(-1000.0, 1000.0), 0.0],
                velocity: [rng.range(-10.0, 10.0), rng.range(-10.0, 10.0), 0.0],
                hdop: rng.range(0.5, 5.0),
            })
        } else { None };
        nav.step(&imu, gnss.as_ref());
        let p = nav.position();
        assert!(p[0].is_finite() && p[1].is_finite(), "iter {} pos NaN", i);
    }
}
