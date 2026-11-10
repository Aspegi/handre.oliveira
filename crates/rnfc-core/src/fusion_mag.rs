#![deny(unsafe_code)]
use libm::{atan2f, sqrtf};

/// Fusão complementar gyro + magnetómetro para yaw absoluto.
pub struct FusionMag {
    pub yaw: f32,
    pub alpha: f32,       // peso do gyro (0.95 típico)
    pub declination: f32, // rad, leste positivo
}

impl FusionMag {
    pub fn new(declination: f32) -> Self {
        Self { yaw: 0.0, alpha: 0.95, declination }
    }

    /// mag: campo em body NED (uT). gyro_yaw_rate: rad/s.
    pub fn update(&mut self, mag_n: f32, mag_e: f32, gyro_yaw_rate: f32, dt: f32) {
        // 1. Heading do magnetómetro (compensar declinação)
        let mag_yaw = atan2f(mag_e, mag_n) - self.declination;

        // 2. Predição gyro
        let gyro_yaw = self.yaw + gyro_yaw_rate * dt;

        // 3. Wrap do erro para [-pi, pi]
        let mut err = mag_yaw - gyro_yaw;
        while err >  core::f32::consts::PI { err -= 2.0 * core::f32::consts::PI; }
        while err < -core::f32::consts::PI { err += 2.0 * core::f32::consts::PI; }

        // 4. Fusão complementar
        self.yaw = gyro_yaw + (1.0 - self.alpha) * err;

        // 5. Wrap final
        while self.yaw >  core::f32::consts::PI { self.yaw -= 2.0 * core::f32::consts::PI; }
        while self.yaw < -core::f32::consts::PI { self.yaw += 2.0 * core::f32::consts::PI; }
    }

    pub fn heading_deg(&self) -> f32 {
        self.yaw * 180.0 / core::f32::consts::PI
    }
}

