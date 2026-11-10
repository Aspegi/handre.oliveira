// Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)
#![deny(unsafe_code)]

use crate::pid::PidController;

pub struct AttitudeController {
    pid_roll: PidController,
    pid_pitch: PidController,
    pid_yaw: PidController,
}

impl AttitudeController {
    pub fn new() -> Self {
        Self {
            pid_roll: PidController::new(4.0, 0.5, 0.8, 1.0, 0.5),
            pid_pitch: PidController::new(4.0, 0.5, 0.8, 1.0, 0.5),
            // Yaw: output_limit=2.5 -> taxa max = 2.5/0.8 = 3.1 rad/s = 178Â°/s
            pid_yaw: PidController::new(5.0, 0.3, 1.5, 2.5, 0.5),
        }
    }

    pub fn update(&mut self, setpoint: [f32; 3], measured: [f32; 3], dt: f32) -> [f32; 3] {
        [
            self.pid_roll.update(setpoint[0], measured[0], dt),
            self.pid_pitch.update(setpoint[1], measured[1], dt),
            self.pid_yaw.update(setpoint[2], measured[2], dt),
        ]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const PI: f32 = core::f32::consts::PI;

    #[test]
    fn zero_error_zero_output() {
        let mut c = AttitudeController::new();
        let sp = [0.1, -0.05, 0.3];
        let out = c.update(sp, sp, 0.01);
        assert!(out[0].abs() < 1e-3, "roll={}", out[0]);
        assert!(out[1].abs() < 1e-3, "pitch={}", out[1]);
        assert!(out[2].abs() < 1e-3, "yaw={}", out[2]);
    }

    #[test]
    fn positive_roll_error_positive_output() {
        let mut c = AttitudeController::new();
        let out = c.update([0.5, 0.0, 0.0], [0.0, 0.0, 0.0], 0.01);
        assert!(out[0] > 0.0, "roll cmd devia ser positivo, got {}", out[0]);
    }

    #[test]
    fn positive_pitch_error_positive_output() {
        let mut c = AttitudeController::new();
        let out = c.update([0.0, 0.5, 0.0], [0.0, 0.0, 0.0], 0.01);
        assert!(out[1] > 0.0, "pitch cmd devia ser positivo, got {}", out[1]);
    }

    #[test]
    fn no_nan_extreme_inputs() {
        let extremes = [-PI, -1.0, 0.0, 1.0, PI];
        for &sp_r in &extremes {
            for &sp_p in &extremes {
                for &sp_y in &extremes {
                    for &m_r in &extremes {
                        for &m_p in &extremes {
                            for &m_y in &extremes {
                                let mut c = AttitudeController::new();
                                let o = c.update([sp_r, sp_p, sp_y], [m_r, m_p, m_y], 0.01);
                                assert!(!o[0].is_nan() && o[0].is_finite(), "roll");
                                assert!(!o[1].is_nan() && o[1].is_finite(), "pitch");
                                assert!(!o[2].is_nan() && o[2].is_finite(), "yaw");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn axes_independent() {
        let mut c = AttitudeController::new();
        let o = c.update([0.5, 0.0, 0.0], [0.0, 0.0, 0.0], 0.01);
        assert!(o[1].abs() < 1e-3, "pitch nao devia reagir: {}", o[1]);
        assert!(o[2].abs() < 1e-3, "yaw nao devia reagir: {}", o[2]);
    }
}
