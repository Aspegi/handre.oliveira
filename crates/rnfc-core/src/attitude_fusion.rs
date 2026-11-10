// Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)
#![deny(unsafe_code)]

use libm::{atan2f, sqrtf};

pub struct ComplementaryFilter {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    alpha: f32,
}

impl ComplementaryFilter {
    pub fn new() -> Self {
        Self {
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            alpha: 0.98,
        }
    }

    pub fn update(&mut self, gyro: [f32; 3], accel: [f32; 3], dt: f32) {
        let roll_gyro = self.roll + gyro[0] * dt;
        let pitch_gyro = self.pitch + gyro[1] * dt;
        let yaw_gyro = self.yaw + gyro[2] * dt;

        let roll_accel = atan2f(accel[1], -accel[2]);
        let norm = sqrtf(accel[1] * accel[1] + accel[2] * accel[2]);
        let pitch_accel = atan2f(-accel[0], norm);

        self.roll = self.alpha * roll_gyro + (1.0 - self.alpha) * roll_accel;
        self.pitch = self.alpha * pitch_gyro + (1.0 - self.alpha) * pitch_accel;
        self.yaw = yaw_gyro;

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
