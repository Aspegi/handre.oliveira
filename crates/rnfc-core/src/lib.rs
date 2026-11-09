// Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)
#![no_std]
#![deny(unsafe_code)]

pub mod attitude;
pub mod math;
pub mod attitude_controller;
pub mod attitude_fusion;
pub mod guidance;
pub mod kalman;
pub mod pid;
pub mod rotation;

use rnfc_abstractions::{GnssFix, ImuSample};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavSource { GnssAided, Degraded, Inertial }

const GNSS_TIMEOUT_S: f32 = 1.0;
const HDOP_MAX: f32 = 5.0;

pub struct NavState {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub attitude: [f32; 3],
    pub source: NavSource,
}

pub struct NavCore {
    kf: kalman::Kalman3D,
    att: attitude_fusion::ComplementaryFilter,
    source: NavSource,
    gnss_lost_for: f32,
}

impl NavCore {
    pub fn new(init_pos: [f32; 3], init_vel: [f32; 3]) -> Self {
        Self {
            kf: kalman::Kalman3D::new(init_pos, init_vel),
            att: attitude_fusion::ComplementaryFilter::new(),
            source: NavSource::Inertial,
            gnss_lost_for: f32::INFINITY,
        }
    }

    pub fn step(&mut self, imu: &ImuSample, gnss: Option<&GnssFix>) -> NavState {
        self.att.update(imu.gyro, imu.accel, imu.dt);
        let accel_ned = rotation::body_to_ned(imu.accel, self.att.roll, self.att.pitch, self.att.yaw);
        self.kf.predict(accel_ned, imu.dt);

        let usable_gnss = gnss.filter(|f| f.hdop < HDOP_MAX);
        match usable_gnss {
            Some(fix) => {
                self.kf.update_position(fix.position);
                self.source = NavSource::GnssAided;
                self.gnss_lost_for = 0.0;
            }
            None => {
                self.gnss_lost_for += imu.dt;
                self.source = if self.gnss_lost_for > GNSS_TIMEOUT_S { NavSource::Inertial } else { NavSource::Degraded };
            }
        }

        NavState {
            position: [self.kf.x[0][0], self.kf.x[1][0], self.kf.x[2][0]],
            velocity: [self.kf.x[0][1], self.kf.x[1][1], self.kf.x[2][1]],
            attitude: [self.att.roll, self.att.pitch, self.att.yaw],
            source: self.source,
        }
    }

    pub fn source(&self) -> NavSource { self.source }
    pub fn roll(&self) -> f32 { self.att.roll }
    pub fn pitch(&self) -> f32 { self.att.pitch }
    pub fn yaw(&self) -> f32 { self.att.yaw }
    pub fn position(&self) -> [f32; 3] {
        [self.kf.x[0][0], self.kf.x[1][0], self.kf.x[2][0]]
    }
    pub fn velocity(&self) -> [f32; 3] {
        [self.kf.x[0][1], self.kf.x[1][1], self.kf.x[2][1]]
    }
}



