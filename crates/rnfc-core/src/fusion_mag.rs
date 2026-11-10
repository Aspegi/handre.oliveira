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

impl FusionMag {
    /// Atualiza com magnetometro 3D (body frame) + tilt compensation (AN3192).
    /// Compensa roll/pitch para recuperar heading mesmo com drone inclinado.
    pub fn update_3d(&mut self, mb_n: f32, mb_e: f32, mb_d: f32,
                     roll: f32, pitch: f32, gyro_yaw_rate: f32, dt: f32) {
        let sr = libm::sinf(roll);
        let cr = libm::cosf(roll);
        let sp = libm::sinf(pitch);
        let cp = libm::cosf(pitch);

        // Projecao do vetor corpo no plano horizontal (AN3192)
        let xh = mb_n * cp + mb_d * sp;
        let yh = mb_n * sr * sp + mb_e * cr - mb_d * sr * cp;

        let mag_yaw = libm::atan2f(-yh, xh) - self.declination;

        let gyro_yaw = self.yaw + gyro_yaw_rate * dt;
        let mut err = mag_yaw - gyro_yaw;
        while err >  core::f32::consts::PI { err -= 2.0 * core::f32::consts::PI; }
        while err < -core::f32::consts::PI { err += 2.0 * core::f32::consts::PI; }

        self.yaw = gyro_yaw + (1.0 - self.alpha) * err;
        while self.yaw >  core::f32::consts::PI { self.yaw -= 2.0 * core::f32::consts::PI; }
        while self.yaw < -core::f32::consts::PI { self.yaw += 2.0 * core::f32::consts::PI; }
    }
}

#[cfg(test)]
mod tests_3d {
    use super::*;
    const PI: f32 = core::f32::consts::PI;

    /// Gera leitura de magnetometro em body frame (world field [H,0,0]).
    fn body_mag(h: f32, yaw: f32, pitch: f32, roll: f32) -> (f32, f32, f32) {
        let cy = yaw.cos(); let sy = yaw.sin();
        let cp = pitch.cos(); let sp = pitch.sin();
        let cf = roll.cos(); let sf = roll.sin();
        let mb_n = h * cp * cy;
        let mb_e = -h * sy * cf + h * sp * cy * sf;
        let mb_d = h * sy * sf + h * sp * cy * cf;
        (mb_n, mb_e, mb_d)
    }

    fn run(f: &mut FusionMag, h: f32, yaw: f32, pitch: f32, roll: f32) {
        let (mb_n, mb_e, mb_d) = body_mag(h, yaw, pitch, roll);
        for _ in 0..200 {
            f.update_3d(mb_n, mb_e, mb_d, roll, pitch, 0.0, 0.01);
        }
    }

    #[test]
    fn level_returns_correct_yaw() {
        for &yaw in &[0.0_f32, PI/4.0, -PI/4.0, PI, -PI/2.0] {
            let mut f = FusionMag::new(0.0);
            f.yaw = yaw;
            run(&mut f, 50.0, yaw, 0.0, 0.0);
            let err = (f.yaw - yaw).abs();
            assert!(err < 0.05, "level yaw={} err={}", yaw, err);
        }
    }

    #[test]
    fn pitched_returns_correct_yaw() {
        for &yaw in &[0.0_f32, PI/4.0, PI/2.0, -PI/2.0, PI] {
            for &pitch in &[0.1_f32, 0.3, 0.5] {
                let mut f = FusionMag::new(0.0);
                f.yaw = yaw;
                run(&mut f, 50.0, yaw, pitch, 0.0);
                let err = (f.yaw - yaw).abs();
                assert!(err < 0.05, "pitch yaw={} pitch={} err={}", yaw, pitch, err);
            }
        }
    }

    #[test]
    fn rolled_returns_correct_yaw() {
        for &yaw in &[0.0_f32, PI/4.0, PI/2.0, -PI/2.0, PI] {
            for &roll in &[0.1_f32, 0.3, 0.5] {
                let mut f = FusionMag::new(0.0);
                f.yaw = yaw;
                run(&mut f, 50.0, yaw, 0.0, roll);
                let err = (f.yaw - yaw).abs();
                assert!(err < 0.05, "roll yaw={} roll={} err={}", yaw, roll, err);
            }
        }
    }
}
