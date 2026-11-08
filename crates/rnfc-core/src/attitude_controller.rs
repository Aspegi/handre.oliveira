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
            pid_roll:  PidController::new(4.0, 0.5, 0.8, 1.0, 0.5),
            pid_pitch: PidController::new(4.0, 0.5, 0.8, 1.0, 0.5),
            pid_yaw:   PidController::new(3.5, 0.3, 0.8, 1.0, 0.5),
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
