#![allow(clippy::new_without_default)]
//! Attitude estimator: accel + gyro -> roll/pitch (complementary filter).
//! Padrao ArduPilot/PX4. Sem isto, o controlador nao voa em hardware real.

use libm::{atan2f, sqrtf};

pub struct AttitudeEstimator {
    pub roll: f32,
    pub pitch: f32,
    pub alpha: f32,
}

impl AttitudeEstimator {
    pub fn new() -> Self {
        Self { roll: 0.0, pitch: 0.0, alpha: 0.98 }
    }

    /// accel (body frame, m/s²), gyro (body frame, rad/s), dt (s).
    pub fn update(&mut self, accel: [f32; 3], gyro: [f32; 3], dt: f32) {
        let ax = accel[0];
        let ay = accel[1];
        let az = accel[2];

        // Accel-derived attitude (assume gravity dominates)
        let accel_norm = sqrtf(ax * ax + ay * ay + az * az);
        if accel_norm < 1.0 || az > 0.0 {
            // Em queda livre ou orientacao invertida - confiar so no gyro
            self.roll += gyro[0] * dt;
            self.pitch += gyro[1] * dt;
            return;
        }
        let accel_roll = atan2f(-ay, -az);
        let accel_pitch = atan2f(-ax, sqrtf(ay * ay + az * az));

        // Gyro integration (prediction)
        let gyro_roll = self.roll + gyro[0] * dt;
        let gyro_pitch = self.pitch + gyro[1] * dt;

        // Complementary filter
        self.roll = self.alpha * gyro_roll + (1.0 - self.alpha) * accel_roll;
        self.pitch = self.alpha * gyro_pitch + (1.0 - self.alpha) * accel_pitch;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_estimates_zero() {
        let mut e = AttitudeEstimator::new();
        for _ in 0..200 {
            e.update([0.0, 0.0, -9.81], [0.0, 0.0, 0.0], 0.01);
        }
        assert!(e.roll.abs() < 0.01, "roll={}", e.roll);
        assert!(e.pitch.abs() < 0.01, "pitch={}", e.pitch);
    }

    #[test]
    fn pitched_forward_detects_pitch() {
        let mut e = AttitudeEstimator::new();
        // pitch +30 graus: ax = -sin(30)*g, az = -cos(30)*g
        let pitch_target = 0.5236_f32;  // 30 graus
        let ax = -pitch_target.sin() * 9.81;
        let az = -pitch_target.cos() * 9.81;
        for _ in 0..500 {
            e.update([ax, 0.0, az], [0.0, 0.0, 0.0], 0.01);
        }
        assert!((e.pitch - pitch_target).abs() < 0.05,
            "pitch={} target={}", e.pitch, pitch_target);
    }

    #[test]
    fn rolled_right_detects_roll() {
        let mut e = AttitudeEstimator::new();
        let roll_target = 0.3_f32;  // ~17 graus
        let ay = -roll_target.sin() * 9.81;
        let az = -roll_target.cos() * 9.81;
        for _ in 0..500 {
            e.update([0.0, ay, az], [0.0, 0.0, 0.0], 0.01);
        }
        assert!((e.roll - roll_target).abs() < 0.05,
            "roll={} target={}", e.roll, roll_target);
    }

    #[test]
    fn gyro_only_rotates() {
        let mut e = AttitudeEstimator::new();
        // gyro puro sem accel confiavel (free-fall) - integra direto
        for _ in 0..100 {
            e.update([0.0, 0.0, 0.0], [0.5, 0.0, 0.0], 0.01);  // 0.5 rad/s
        }
        // 100 * 0.01 * 0.5 = 0.5 rad
        assert!((e.roll - 0.5).abs() < 0.05, "roll={}", e.roll);
    }

    #[test]
    fn no_nan_extreme_inputs() {
        let mut e = AttitudeEstimator::new();
        for ax in [-100.0_f32, -9.81, 0.0, 9.81, 100.0] {
            for ay in [-100.0_f32, -9.81, 0.0, 9.81, 100.0] {
                for az in [-100.0_f32, -9.81, 0.0, 9.81, 100.0] {
                    e.update([ax, ay, az], [0.1, -0.05, 0.02], 0.01);
                    assert!(e.roll.is_finite(), "roll NaN em ({},{},{})", ax, ay, az);
                    assert!(e.pitch.is_finite(), "pitch NaN");
                }
            }
        }
    }

    #[test]
    fn converges_from_wrong_initial() {
        let mut e = AttitudeEstimator::new();
        e.roll = 0.5;  // começa errado
        // Accel diz "nivelado"
        for _ in 0..1000 {
            e.update([0.0, 0.0, -9.81], [0.0, 0.0, 0.0], 0.01);
        }
        assert!(e.roll.abs() < 0.05, "nao convergiu: roll={}", e.roll);
    }
}


