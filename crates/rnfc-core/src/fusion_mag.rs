#![deny(unsafe_code)]
use libm::atan2f;

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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_always_bounded_after_update() {
        let mut f = FusionMag::new(0.0);
        for mag_n in [-100.0_f32, -50.0, 0.0, 50.0, 100.0] {
            for mag_e in [-100.0_f32, -50.0, 0.0, 50.0, 100.0] {
                for gyro in [-10.0_f32, -1.0, 0.0, 1.0, 10.0] {
                    f.update(mag_n, mag_e, gyro, 0.01);
                    assert!(f.yaw >= -core::f32::consts::PI);
                    assert!(f.yaw <=  core::f32::consts::PI);
                    assert!(!f.yaw.is_nan());
                }
            }
        }
    }

    #[test]
    fn mag_corrects_gyro_bias() {
        let truth = 0.5_f32;
        let mut f = FusionMag::new(0.0);
        f.yaw = truth;
        for _ in 0..2000 {
            f.update(truth.cos() * 50.0, truth.sin() * 50.0, 0.002, 0.01);
        }
        let err = (f.yaw - truth).abs();
        assert!(err < 0.05);
    }

    #[test]
    fn declination_offsets_heading() {
        let mut f = FusionMag::new(0.1);
        f.update(50.0, 0.0, 0.0, 0.01);
        assert!(f.yaw < 0.0);
        assert!(f.yaw > -0.2);
    }
}
