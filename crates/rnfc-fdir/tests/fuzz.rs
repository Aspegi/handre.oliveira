//! Fuzz tests para rnfc-fdir.

use rnfc_fdir::{Failsafe, Health, SensorHealth, SensorVoter, SpoofingDetector};

const ITERS: usize = 10_000;
const PI: f32 = core::f32::consts::PI;

struct Rng {
    state: u32,
}
impl Rng {
    fn new(seed: u32) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }
    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
    fn f32_01(&mut self) -> f32 {
        (self.next() >> 8) as f32 / 16777216.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.f32_01() * (hi - lo)
    }
    fn health(&mut self) -> Health {
        match (self.next() % 5) as u8 {
            0 => Health::Healthy,
            1 => Health::Warning,
            2 => Health::Stuck,
            3 => Health::Outlier,
            _ => Health::Failed,
        }
    }
}

#[test]
fn fuzz_sensor_health_10k() {
    let mut rng = Rng::new(10);
    let mut h = SensorHealth::new(0.0);
    for i in 0..ITERS {
        let v = rng.range(-1000.0, 1000.0);
        let _ = h.update(v);
        assert!(h.running_mean.is_finite(), "iter {} mean NaN", i);
        assert!(
            h.running_var.is_finite() && h.running_var >= 0.0,
            "iter {} var invalida",
            i
        );
    }
}

#[test]
fn fuzz_failsafe_10k() {
    let mut rng = Rng::new(11);
    let fs = Failsafe::new();
    for _ in 0..ITERS {
        let pos = [
            rng.range(-10_000.0, 10_000.0),
            rng.range(-10_000.0, 10_000.0),
        ];
        let batt = rng.range(0.0, 20.0);
        let health = rng.health();
        let _ = fs.evaluate(pos, batt, health);
    }
}

#[test]
fn fuzz_sensor_voter_10k() {
    let mut rng = Rng::new(12);
    let v = SensorVoter::new();
    for i in 0..ITERS {
        let s1 = rng.range(-1000.0, 1000.0);
        let s2 = rng.range(-1000.0, 1000.0);
        let s3 = rng.range(-1000.0, 1000.0);
        let r = v.vote(s1, s2, s3);
        assert!(r.value.is_finite(), "iter {} value NaN", i);
    }
}

#[test]
fn fuzz_spoofing_detector_10k() {
    let mut rng = Rng::new(13);
    let mut sd = SpoofingDetector::new();
    for i in 0..ITERS {
        let mag = rng.range(-PI, PI);
        let gyro = rng.range(-PI, PI);
        let _ = sd.update(mag, gyro);
        assert!(
            sd.err_history.iter().all(|e| e.is_finite()),
            "iter {} err NaN",
            i
        );
    }
}
