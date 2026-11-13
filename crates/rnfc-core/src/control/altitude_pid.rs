//! PID altitude controller with integral and anti-windup.
//!
//! Unlike the simple P+D in the original SITL, this includes an
//! integral term (eliminates steady-state error) and anti-windup
//! (clamps the integral when the output saturates).

#[derive(Clone, Copy, Debug)]
pub struct AltitudePid {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    integral: f32,
    max_integral: f32,
    pub max_output: f32,
}

impl AltitudePid {
    pub fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            kp, ki, kd,
            integral: 0.0,
            max_integral: 5.0,
            max_output: 10.0,
        }
    }

    pub fn update(&mut self, target: f32, current: f32, velocity_z: f32, dt: f32) -> f32 {
        let error = target - current;
        self.integral = (self.integral + error * dt).clamp(-self.max_integral, self.max_integral);
        let derivative = -velocity_z;
        let out = self.kp * error + self.ki * self.integral + self.kd * derivative;
        out.clamp(-self.max_output, self.max_output)
    }

    pub fn reset(&mut self) {
        self.integral = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proportional_response() {
        let mut pid = AltitudePid::new(1.0, 0.0, 0.0);
        let out = pid.update(10.0, 5.0, 0.0, 0.01);
        assert!((out - 5.0).abs() < 0.01);
    }

    #[test]
    fn integral_accumulates() {
        let mut pid = AltitudePid::new(0.0, 1.0, 0.0);
        for _ in 0..100 { pid.update(10.0, 5.0, 0.0, 0.01); }
        let out = pid.update(10.0, 5.0, 0.0, 0.01);
        assert!(out > 4.0 && out < 6.0, "out={}", out);
    }

    #[test]
    fn anti_windup_clamps() {
        let mut pid = AltitudePid::new(0.0, 1.0, 0.0);
        for _ in 0..10000 { pid.update(100.0, 0.0, 0.0, 0.01); }
        let out = pid.update(100.0, 0.0, 0.0, 0.01);
        assert!(out <= 5.0, "out={}", out);
    }

    #[test]
    fn zero_error_zero_output() {
        let mut pid = AltitudePid::new(1.0, 1.0, 1.0);
        let out = pid.update(5.0, 5.0, 0.0, 0.01);
        assert!(out.abs() < 0.01);
    }

    #[test]
    fn velocity_damping() {
        let mut pid = AltitudePid::new(0.0, 0.0, 1.0);
        let out = pid.update(5.0, 5.0, 3.0, 0.01);
        assert!((out + 3.0).abs() < 0.01, "out={}", out);
    }
}
