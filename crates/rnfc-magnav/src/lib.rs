#![allow(clippy::needless_range_loop)]
#![allow(clippy::new_without_default)]
//! MagNav â€” Quantum Magnetometry for GPS-Denied Navigation.
//!
//! Real MagNav systems use pre-surveyed magnetic anomaly maps to provide
//! absolute position without GPS. This crate simulates the full pipeline:
//!
//! 1. Synthetic magnetic anomaly map (3 Gaussian anomalies + baseline Earth field)
//! 2. Quantum magnetometer sensor (noisy scalar reading)
//! 3. Grid-search estimator (recovers position from reading + prior)
//!
//! The estimate is wrapped as a `GnssFix` so the existing `NavCore` (Kalman)
//! can fuse it exactly as it would a GNSS signal. This means the drone can
//! navigate with ZERO GNSS â€” only IMU prediction + MagNav correction.
//!
//! Threat model: GPS jamming/spoofing is completely defeated because the drone
//! never uses GNSS. Quantum magnetometers measure the local field directly
//! (immune to RF jamming) and the pre-surveyed map provides absolute position.

use libm::expf;
use rnfc_abstractions::GnssFix;

// ============ MAGNETIC ANOMALY MAP ============
/// 2D pre-surveyed magnetic anomaly map (uT field strength per cell).
#[derive(Clone)]
pub struct MagneticAnomalyMap {
    pub origin: [f32; 2], // (north, east) of cell (0, 0)
    pub resolution: f32,  // metres per cell
    pub width: usize,
    pub height: usize,
    pub cells: Vec<f32>, // row-major: cells[i * height + j]
}

impl MagneticAnomalyMap {
    /// Creates a synthetic map with 3 Gaussian anomalies over a 50 uT baseline.
    pub fn synthetic(width: usize, height: usize, resolution: f32) -> Self {
        let mut cells = Vec::with_capacity(width * height);
        for i in 0..width {
            for j in 0..height {
                let x = i as f32 * resolution;
                let y = j as f32 * resolution;
                let mut b = 50.0_f32;
                // Anomaly 1: +20 uT Gaussian peak at (100, 80)
                let dx1 = x - 100.0;
                let dy1 = y - 80.0;
                b += 20.0 * expf(-(dx1 * dx1 + dy1 * dy1) / 800.0);
                // Anomaly 2: -15 uT Gaussian dip at (200, 120)
                let dx2 = x - 200.0;
                let dy2 = y - 120.0;
                b -= 15.0 * expf(-(dx2 * dx2 + dy2 * dy2) / 1200.0);
                // Anomaly 3: +10 uT Gaussian peak at (50, 200)
                let dx3 = x - 50.0;
                let dy3 = y - 200.0;
                b += 10.0 * expf(-(dx3 * dx3 + dy3 * dy3) / 1000.0);
                cells.push(b);
            }
        }
        Self {
            origin: [0.0, 0.0],
            resolution,
            width,
            height,
            cells,
        }
    }

    /// Looks up the field value at a position (metres). Returns None if out of bounds.
    pub fn lookup(&self, pos: [f32; 2]) -> Option<f32> {
        let i = ((pos[0] - self.origin[0]) / self.resolution) as i32;
        let j = ((pos[1] - self.origin[1]) / self.resolution) as i32;
        if i >= 0 && j >= 0 && (i as usize) < self.width && (j as usize) < self.height {
            Some(self.cells[i as usize * self.height + j as usize])
        } else {
            None
        }
    }
}

// ============ QUANTUM MAGNETOMETER ============
/// Simulates a quantum magnetometer (SERF, NV-centre, etc.) with realistic noise.
pub struct QuantumMagSensor {
    pub map: MagneticAnomalyMap,
    pub noise_ut: f32, // 1-sigma noise in microtesla
}

impl QuantumMagSensor {
    pub fn new(map: MagneticAnomalyMap, noise_ut: f32) -> Self {
        Self { map, noise_ut }
    }

    /// Measures the field at true_pos with additive noise (clamped to Â±noise_ut).
    pub fn measure(&self, true_pos: [f32; 2], noise_ut: f32) -> f32 {
        let truth = self.map.lookup(true_pos).unwrap_or(50.0);
        truth + noise_ut.clamp(-self.noise_ut, self.noise_ut)
    }
}

// ============ MAGNAV ESTIMATOR ============
/// Grid-search estimator: recovers position from a scalar field reading + prior.
pub struct MagNavEstimator {
    pub search_radius_m: f32,
    pub search_step_m: f32,
}

impl MagNavEstimator {
    pub fn new() -> Self {
        Self {
            search_radius_m: 15.0,
            search_step_m: 1.0,
        }
    }

    /// Finds the position within Â±search_radius of prior whose map value
    /// is closest to the measurement. Returns prior on NaN input.
    pub fn estimate(
        &self,
        sensor: &QuantumMagSensor,
        measured_b: f32,
        prior: [f32; 2],
    ) -> [f32; 2] {
        if !measured_b.is_finite() {
            return prior;
        }
        let mut best_pos = prior;
        let mut best_err = f32::INFINITY;
        let n = (self.search_radius_m / self.search_step_m) as i32;
        for di in -n..=n {
            for dj in -n..=n {
                let candidate = [
                    prior[0] + di as f32 * self.search_step_m,
                    prior[1] + dj as f32 * self.search_step_m,
                ];
                if let Some(b_map) = sensor.map.lookup(candidate) {
                    let err = (b_map - measured_b).abs();
                    if err < best_err {
                        best_err = err;
                        best_pos = candidate;
                    }
                }
            }
        }
        best_pos
    }

    /// End-to-end: measure at true_pos, estimate from prior, wrap as GnssFix.
    /// hdop is set to 5.0 (MagNav is less precise than GNSS, but immune to jamming).
    pub fn estimate_as_fix(
        &self,
        sensor: &QuantumMagSensor,
        true_pos: [f32; 2],
        prior: [f32; 2],
        noise_ut: f32,
    ) -> GnssFix {
        let measured = sensor.measure(true_pos, noise_ut);
        let est = self.estimate(sensor, measured, prior);
        GnssFix {
            position: [est[0], est[1], 0.0],
            velocity: [0.0, 0.0, 0.0],
            hdop: 5.0,
        }
    }
}

// ============ TESTES ============
#[cfg(test)]
mod tests {
    use super::*;
    use rnfc_abstractions::ImuSample;
    use rnfc_core::NavCore;

    fn make_sensor() -> QuantumMagSensor {
        let map = MagneticAnomalyMap::synthetic(300, 300, 1.0);
        QuantumMagSensor::new(map, 0.5)
    }

    #[test]
    fn map_lookup_returns_anomaly_at_peak() {
        let map = MagneticAnomalyMap::synthetic(300, 300, 1.0);
        let b = map.lookup([100.0, 80.0]).unwrap();
        assert!(b > 50.0 && b < 100.0, "b={}", b);
    }

    #[test]
    fn map_lookup_out_of_bounds_returns_none() {
        let map = MagneticAnomalyMap::synthetic(300, 300, 1.0);
        assert!(map.lookup([-10.0, 50.0]).is_none());
        assert!(map.lookup([400.0, 50.0]).is_none());
    }

    #[test]
    fn sensor_measure_bounded_by_noise() {
        let sensor = make_sensor();
        let true_pos = [150.0, 100.0];
        let truth = sensor.map.lookup(true_pos).unwrap();
        for seed in 0..100 {
            let noise = ((seed as f32) / 100.0 - 0.5) * 2.0 * sensor.noise_ut;
            let m = sensor.measure(true_pos, noise);
            assert!(
                (m - truth).abs() <= sensor.noise_ut + 1e-6,
                "noise too large: measured={}, truth={}",
                m,
                truth
            );
        }
    }

    #[test]
    fn estimator_converges_on_anomaly_peak() {
        let sensor = make_sensor();
        let est = MagNavEstimator::new();
        let true_pos = [100.0, 80.0];
        let prior = [110.0, 80.0];
        let result = est.estimate(&sensor, sensor.measure(true_pos, 0.0), prior);
        let err = ((result[0] - true_pos[0]).powi(2) + (result[1] - true_pos[1]).powi(2)).sqrt();
        assert!(err < 5.0, "estimate error too large: {}m", err);
    }

    #[test]
    fn estimator_averages_noise() {
        let sensor = make_sensor();
        let est = MagNavEstimator::new();
        let true_pos = [100.0, 80.0];
        let prior = [108.0, 78.0];
        let mut sum = [0.0_f32; 2];
        let n = 10;
        for i in 0..n {
            let noise = ((i as f32) / n as f32 - 0.5) * 2.0 * sensor.noise_ut;
            let r = est.estimate(&sensor, sensor.measure(true_pos, noise), prior);
            sum[0] += r[0];
            sum[1] += r[1];
        }
        let avg = [sum[0] / n as f32, sum[1] / n as f32];
        let err = ((avg[0] - true_pos[0]).powi(2) + (avg[1] - true_pos[1]).powi(2)).sqrt();
        assert!(err < 5.0, "avg error too large: {}m", err);
    }

    #[test]
    fn estimator_returns_prior_on_nan() {
        let sensor = make_sensor();
        let est = MagNavEstimator::new();
        let prior = [50.0, 50.0];
        let result = est.estimate(&sensor, f32::NAN, prior);
        assert_eq!(result, prior);
    }

    #[test]
    fn estimate_as_fix_has_hdop_5() {
        let sensor = make_sensor();
        let est = MagNavEstimator::new();
        let fix = est.estimate_as_fix(&sensor, [100.0, 80.0], [105.0, 80.0], 0.0);
        assert_eq!(fix.hdop, 5.0);
        assert!(fix.position[0].is_finite());
        assert!(fix.position[1].is_finite());
    }

    #[test]
    fn gps_denied_navigation_succeeds() {
        // Drone flies 5m east (5m/s for 1s) WITHOUT any GNSS â€” only MagNav.
        let sensor = make_sensor();
        let est = MagNavEstimator::new();

        let mut nav = NavCore::new([100.0, 80.0, 0.0], [0.0, 0.0, 0.0]);
        let mut true_pos = [100.0_f32, 80.0_f32];

        for i in 0..100 {
            true_pos[1] += 5.0 * 0.01; // fly east at 5 m/s

            let prior = nav.position();
            let noise = ((i % 10) as f32 / 10.0 - 0.5) * 2.0 * sensor.noise_ut;
            let fix = est.estimate_as_fix(&sensor, true_pos, [prior[0], prior[1]], noise);

            let imu = ImuSample {
                accel: [0.0, 0.0, -9.81],
                gyro: [0.0, 0.0, 0.0],
                dt: 0.01,
            };
            nav.step(&imu, Some(&fix));
        }

        let p = nav.position();
        let err = ((p[0] - true_pos[0]).powi(2) + (p[1] - true_pos[1]).powi(2)).sqrt();
        assert!(
            err < 10.0,
            "GPS-denied navigation error: {:.2}m (true={:?}, est=[{:.2},{:.2}])",
            err,
            true_pos,
            p[0],
            p[1]
        );
    }
}
