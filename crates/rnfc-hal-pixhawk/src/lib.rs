//! Pixhawk HAL — FlightController implementation for STM32F765.
//!
//! This is a skeleton. The PWM output is real (uses embedded-hal);
//! the IMU/GNSS/MAG drivers are stubs (return default values).
//! To complete: connect a Pixhawk 4, implement the sensor drivers,
//! flash with probe-rs, verify PWM with an oscilloscope.

#![no_std]
#![deny(unsafe_code)]

use embedded_hal::pwm::SetDutyCycle;
use rnfc_abstractions::{GnssFix, ImuSample, MagSample};
use rnfc_hal::FlightController;

/// Pixhawk 4 hardware implementation.
pub struct Pixhawk4<PWM> {
    pwm: [PWM; 4],
    armed: bool,
    imu: ImuSample,
    mag: MagSample,
    gnss: Option<GnssFix>,
    t_ms: u64,
}

impl<PWM: SetDutyCycle> Pixhawk4<PWM> {
    pub fn new(pwm: [PWM; 4]) -> Self {
        Self {
            pwm,
            armed: false,
            imu: ImuSample { accel: [0.0, 0.0, -9.81], gyro: [0.0, 0.0, 0.0], dt: 0.01 },
            mag: MagSample { field: [50.0, 0.0, 0.0], dt: 0.01 },
            gnss: None,
            t_ms: 0,
        }
    }

    /// Arm ESCs (1ms pulse for 2s).
    pub fn arm(&mut self) {
        for pwm in &mut self.pwm {
            let _ = pwm.set_duty_cycle(1000);
        }
        self.armed = true;
    }
}

impl<PWM: SetDutyCycle> FlightController for Pixhawk4<PWM> {
    fn read_imu(&mut self) -> ImuSample {
        // TODO: ICM-20689 / BMI088 driver
        self.imu
    }

    fn read_gnss(&mut self) -> Option<GnssFix> {
        // TODO: u-blox M9N driver via UART
        self.gnss
    }

    fn read_mag(&mut self) -> MagSample {
        // TODO: IST8310 driver via I2C
        self.mag
    }

    fn set_motors(&mut self, throttles: [f32; 4]) {
        if !self.armed {
            return;
        }
        for (i, pwm) in self.pwm.iter_mut().enumerate() {
            let us = 1000.0 + throttles[i].clamp(0.0, 1.0) * 1000.0;
            let _ = pwm.set_duty_cycle(us as u16);
        }
    }

    fn now_ms(&mut self) -> u64 {
        self.t_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockPwm;
    impl embedded_hal::pwm::ErrorType for MockPwm { type Error = core::convert::Infallible; }
    impl SetDutyCycle for MockPwm {
        fn max_duty_cycle(&self) -> u16 { 2000 }
        fn set_duty_cycle(&mut self, _: u16) -> Result<(), Self::Error> { Ok(()) }
    }

    #[test]
    fn new_is_not_armed() {
        let fc = Pixhawk4::new(core::array::from_fn(|_| MockPwm));
        assert!(!fc.armed);
    }

    #[test]
    fn arm_sets_armed() {
        let mut fc = Pixhawk4::new(core::array::from_fn(|_| MockPwm));
        fc.arm();
        assert!(fc.armed);
    }

    #[test]
    fn set_motors_before_arm_is_ignored() {
        let mut fc = Pixhawk4::new(core::array::from_fn(|_| MockPwm));
        fc.set_motors([0.5, 0.5, 0.5, 0.5]);
        // No panic — silently ignored
    }

    #[test]
    fn set_motors_after_arm_succeeds() {
        let mut fc = Pixhawk4::new(core::array::from_fn(|_| MockPwm));
        fc.arm();
        fc.set_motors([0.5, 0.5, 0.5, 0.5]);
    }
}
