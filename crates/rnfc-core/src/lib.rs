// Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)
#![allow(unexpected_cfgs)]
#![no_std]
#![deny(unsafe_code)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#![allow(clippy::new_without_default)]

pub mod control;

pub mod attitude;
pub mod attitude_controller;
pub mod attitude_fusion;
pub mod geodesy;
pub mod guidance;
pub mod kalman;
pub mod math;
pub mod pid;
pub mod rotation;

use rnfc_abstractions::{GnssFix, ImuSample};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavSource {
    GnssAided,
    Degraded,
    Inertial,
}

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
        let accel_ned =
            rotation::body_to_ned(imu.accel, self.att.roll, self.att.pitch, self.att.yaw);
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
                self.source = if self.gnss_lost_for > GNSS_TIMEOUT_S {
                    NavSource::Inertial
                } else {
                    NavSource::Degraded
                };
            }
        }

        NavState {
            position: [self.kf.x[0][0], self.kf.x[1][0], self.kf.x[2][0]],
            velocity: [self.kf.x[0][1], self.kf.x[1][1], self.kf.x[2][1]],
            attitude: [self.att.roll, self.att.pitch, self.att.yaw],
            source: self.source,
        }
    }

    pub fn source(&self) -> NavSource {
        self.source
    }
    pub fn roll(&self) -> f32 {
        self.att.roll
    }
    pub fn pitch(&self) -> f32 {
        self.att.pitch
    }
    pub fn yaw(&self) -> f32 {
        self.att.yaw
    }
    pub fn position(&self) -> [f32; 3] {
        [self.kf.x[0][0], self.kf.x[1][0], self.kf.x[2][0]]
    }
    pub fn velocity(&self) -> [f32; 3] {
        [self.kf.x[0][1], self.kf.x[1][1], self.kf.x[2][1]]
    }
}

pub mod fusion_mag;

#[cfg(test)]
mod tests {
    use super::*;
    use rnfc_abstractions::{GnssFix, ImuSample};

    fn imu_still() -> ImuSample {
        ImuSample {
            accel: [0.0, 0.0, -9.81],
            gyro: [0.0, 0.0, 0.0],
            dt: 0.01,
        }
    }

    fn gnss_at(n: f32, e: f32) -> GnssFix {
        GnssFix {
            position: [n, e, 0.0],
            velocity: [0.0, 0.0, 0.0],
            hdop: 1.0,
        }
    }

    #[test]
    fn new_starts_at_origin() {
        let nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let p = nav.position();
        assert_eq!(p[0], 0.0);
        assert_eq!(p[1], 0.0);
        assert_eq!(p[2], 0.0);
    }

    #[test]
    fn no_nan_after_many_steps() {
        let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let imu = imu_still();
        let gnss = gnss_at(10.0, 20.0);
        for _ in 0..1000 {
            nav.step(&imu, Some(&gnss));
            let p = nav.position();
            assert!(p[0].is_finite(), "north NaN/inf");
            assert!(p[1].is_finite(), "east NaN/inf");
            assert!(p[2].is_finite(), "down NaN/inf");
        }
    }

    #[test]
    fn gnss_fix_pulls_position_toward_fix() {
        let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let imu = imu_still();
        let gnss = gnss_at(50.0, 30.0);
        for _ in 0..200 {
            nav.step(&imu, Some(&gnss));
        }
        let p = nav.position();
        let dist_to_fix = ((p[0] - 50.0).powi(2) + (p[1] - 30.0).powi(2)).sqrt();
        assert!(
            dist_to_fix < 5.0,
            "dist ao fix = {:.2}m (devia < 5m)",
            dist_to_fix
        );
    }

    #[test]
    fn no_gnss_no_movement() {
        let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let imu = imu_still();
        for _ in 0..100 {
            nav.step(&imu, None);
        }
        let p = nav.position();
        let dist = (p[0].powi(2) + p[1].powi(2)).sqrt();
        assert!(dist < 1.0, "moveu {:.2}m sem GNSS", dist);
    }
}

pub mod attitude_estimator;
