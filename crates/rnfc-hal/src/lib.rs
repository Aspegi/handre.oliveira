#![cfg_attr(not(test), no_std)]
#![allow(clippy::new_without_default)]
//! HAL — Hardware Abstraction Layer para o RNFC.
//!
//! Define os traits que um flight controller real deve implementar
//! (Pixhawk, STM32, ou SITL). Permite que o mesmo rnfc-core + rnfc-fdir
//! corram em hardware sem alteracoes.

use rnfc_abstractions::{GnssFix, ImuSample, MagSample};

/// Contrato entre o RNFC e a plataforma de hardware.
pub trait FlightController {
    /// Leitura do IMU (accel + gyro).
    fn read_imu(&mut self) -> ImuSample;
    /// Ultimo fix GNSS (None se sem sinal).
    fn read_gnss(&mut self) -> Option<GnssFix>;
    /// Leitura do magnetometro.
    fn read_mag(&mut self) -> MagSample;
    /// Aplicar comandos aos motores (throttle por motor).
    fn set_motors(&mut self, throttles: [f32; 4]);
    /// Momento atual (ms desde boot).
    fn now_ms(&mut self) -> u64;
}

/// Actuator commands derivados do AttitudeController.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActuatorCmd {
    pub roll_torque: f32,
    pub pitch_torque: f32,
    pub yaw_torque: f32,
    pub base_throttle: f32,
}

impl ActuatorCmd {
    /// Converte torques em throttles por motor (X-config).
    pub fn to_motors(&self) -> [f32; 4] {
        let base = self.base_throttle;
        let r = self.roll_torque;
        let p = self.pitch_torque;
        let y = self.yaw_torque;
        [
            (base - r - p + y).clamp(0.0, 1.0),
            (base + r - p - y).clamp(0.0, 1.0),
            (base + r + p + y).clamp(0.0, 1.0),
            (base - r + p - y).clamp(0.0, 1.0),
        ]
    }
}

/// Implementacao simples de SITL (stand-in para testes).
pub struct SitlHal {
    pub imu: ImuSample,
    pub gnss: Option<GnssFix>,
    pub mag: MagSample,
    pub motors: [f32; 4],
    pub t_ms: u64,
}

impl SitlHal {
    pub fn new() -> Self {
        Self {
            imu: ImuSample {
                accel: [0.0, 0.0, -9.81],
                gyro: [0.0, 0.0, 0.0],
                dt: 0.01,
            },
            gnss: None,
            mag: MagSample {
                field: [50.0, 0.0, 0.0],
                dt: 0.01,
            },
            motors: [0.0; 4],
            t_ms: 0,
        }
    }
}

impl FlightController for SitlHal {
    fn read_imu(&mut self) -> ImuSample {
        self.imu
    }
    fn read_gnss(&mut self) -> Option<GnssFix> {
        self.gnss
    }
    fn read_mag(&mut self) -> MagSample {
        self.mag
    }
    fn set_motors(&mut self, throttles: [f32; 4]) {
        self.motors = throttles;
    }
    fn now_ms(&mut self) -> u64 {
        self.t_ms += 10;
        self.t_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actuator_zero_produces_equal_motors() {
        let cmd = ActuatorCmd {
            roll_torque: 0.0,
            pitch_torque: 0.0,
            yaw_torque: 0.0,
            base_throttle: 0.5,
        };
        let m = cmd.to_motors();
        for i in 0..4 {
            assert!((m[i] - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn actuator_roll_produces_asymmetric() {
        let cmd = ActuatorCmd {
            roll_torque: 0.1,
            pitch_torque: 0.0,
            yaw_torque: 0.0,
            base_throttle: 0.5,
        };
        let m = cmd.to_motors();
        // Motor 0 e 3 (esquerda) reduzem, 1 e 2 (direita) aumentam
        assert!(m[0] < 0.5, "motor 0={}", m[0]);
        assert!(m[1] > 0.5, "motor 1={}", m[1]);
    }

    #[test]
    fn throttle_clamped() {
        let cmd = ActuatorCmd {
            roll_torque: 0.0,
            pitch_torque: 0.0,
            yaw_torque: 0.0,
            base_throttle: 2.0,
        };
        let m = cmd.to_motors();
        for i in 0..4 {
            assert!((m[i] - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn sitl_hal_time_increments() {
        let mut hal = SitlHal::new();
        let t1 = hal.now_ms();
        let t2 = hal.now_ms();
        assert!(t2 > t1);
    }
}
