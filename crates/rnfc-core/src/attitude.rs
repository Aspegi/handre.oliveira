#![deny(unsafe_code)]

pub struct AttitudeEstimator {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl AttitudeEstimator {
    pub fn new() -> Self {
        Self {
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
        }
    }

    pub fn integrate(&mut self, gyro: [f32; 3], dt: f32) {
        self.roll += gyro[0] * dt;
        self.pitch += gyro[1] * dt;
        self.yaw += gyro[2] * dt;

        let two_pi = core::f32::consts::TAU;
        let pi = core::f32::consts::PI;
        while self.yaw > pi {
            self.yaw -= two_pi;
        }
        while self.yaw < -pi {
            self.yaw += two_pi;
        }
    }
}
