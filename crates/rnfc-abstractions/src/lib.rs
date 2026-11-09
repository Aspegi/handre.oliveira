// Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)
#![no_std]
#![deny(unsafe_code)]

#[derive(Clone, Copy, Debug)]
pub struct ImuSample {
    pub accel: [f32; 3],
    pub gyro: [f32; 3],
    pub dt: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct GnssFix {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub hdop: f32,
}

pub trait Imu {
    type Error;
    fn read(&mut self) -> Result<ImuSample, Self::Error>;
}

pub trait Gnss {
    type Error;
    fn read(&mut self) -> Result<Option<GnssFix>, Self::Error>;
}
