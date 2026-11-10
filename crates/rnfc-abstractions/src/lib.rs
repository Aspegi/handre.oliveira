#![deny(unsafe_code)]

pub struct ImuSample {
    pub accel: [f32; 3],
    pub gyro: [f32; 3],
    pub dt: f32,
}
pub struct GnssFix {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub hdop: f32,
}
pub struct MagSample {
    pub field: [f32; 3], // NED body frame (uT)
    pub dt: f32,
}

pub trait Imu {
    fn read(&mut self) -> ImuSample;
}
pub trait Gnss {
    fn fix(&mut self) -> Option<GnssFix>;
}
pub trait Magnetometer {
    fn read(&mut self) -> MagSample;
}
