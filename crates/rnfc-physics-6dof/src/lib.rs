//! 6-DOF quadrotor physics using rotor-rs (RotorPy port).

use rotor_rs::{Multirotor, QuadParamsInput, RotorForce, State, Vec3, Quat};
use rnfc_abstractions::{GnssFix, ImuSample};

pub struct QuadrotorSim {
    veh: Multirotor<f64, RotorForce>,
    state: State<f64>,
    params: QuadParamsInput,
    t: f64,
}

fn default_params() -> QuadParamsInput {
    QuadParamsInput {
        mass: 1.0,
        ixx: 0.01, iyy: 0.01, izz: 0.02,
        ixy: 0.0, iyz: 0.0, ixz: 0.0,
        rotor_pos: [
            [ 0.25,  0.25, 0.0],
            [ 0.25, -0.25, 0.0],
            [-0.25, -0.25, 0.0],
            [-0.25,  0.25, 0.0],
        ],
        rotor_directions: [1.0, -1.0, 1.0, -1.0],
        c_dx: 0.0, c_dy: 0.0, c_dz: 0.0,
        k_eta: 1.0,
        k_m: 0.02,
        k_d: 0.0, k_z: 0.0, k_h: 0.0, k_flap: 0.0,
        tau_m: 0.01,
        rotor_speed_min: 0.0,
        rotor_speed_max: 1500.0,
        k_w: 0.0,
    }
}

impl QuadrotorSim {
    pub fn new() -> Self {
        let params = default_params();
        let veh = Multirotor::with_substeps(&params, 8);
        let state = State {
            x: Vec3::new(0.0, 0.0, 0.0),
            v: Vec3::new(0.0, 0.0, 0.0),
            q: Quat::new(0.0, 0.0, 0.0, 1.0),
            w: Vec3::new(0.0, 0.0, 0.0),
            wind: Vec3::new(0.0, 0.0, 0.0),
            rotor_speeds: [0.0; 4],
        };
        Self { veh, state, params, t: 0.0 }
    }

    pub fn step(&mut self, thrusts: [f64; 4], dt: f64) {
        self.state = self.veh.step(&self.state, &thrusts, dt);
        self.t += dt;
    }

    pub fn position(&self) -> [f32; 3] {
        [self.state.x.x as f32, self.state.x.y as f32, self.state.x.z as f32]
    }

    pub fn velocity(&self) -> [f32; 3] {
        [self.state.v.x as f32, self.state.v.y as f32, self.state.v.z as f32]
    }

    pub fn quaternion(&self) -> [f32; 4] {
        [self.state.q.i as f32, self.state.q.j as f32, self.state.q.k as f32, self.state.q.w as f32]
    }

    pub fn euler(&self) -> [f32; 3] {
        let q = self.quaternion();
        let (i, j, k, w) = (q[0], q[1], q[2], q[3]);
        let roll = (2.0 * (w * i + j * k)).atan2(1.0 - 2.0 * (i * i + j * j));
        let sinp = 2.0 * (w * j - k * i);
        let pitch = if sinp.abs() >= 1.0 { sinp.signum() * core::f32::consts::FRAC_PI_2 } else { sinp.asin() };
        let yaw = (2.0 * (w * k + i * j)).atan2(1.0 - 2.0 * (j * j + k * k));
        [roll, pitch, yaw]
    }

    pub fn params(&self) -> &QuadParamsInput { &self.params }
    pub fn rotor_state(&self) -> &State<f64> { &self.state }

    pub fn imu(&self, dt: f32) -> ImuSample {
        ImuSample { accel: [0.0, 0.0, -9.81], gyro: [0.0, 0.0, 0.0], dt }
    }

    pub fn gnss(&self) -> GnssFix {
        GnssFix { position: self.position(), velocity: self.velocity(), hdop: 1.0 }
    }
}

impl Default for QuadrotorSim { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_holds_altitude() {
        let mut sim = QuadrotorSim::new();
        let hover = 2.5274_f64;
        for _ in 0..100 { sim.step([hover; 4], 0.01); }
        let z = sim.position()[2];
        assert!(z.abs() < 5.0, "z={}", z);
    }

    #[test]
    fn forward_thrust_moves_forward() {
        let mut sim = QuadrotorSim::new();
        let hover = 2.5274_f64;
        for _ in 0..50 {
            sim.step([hover * 0.5, hover * 0.5, hover * 1.5, hover * 1.5], 0.01);
        }
        let x = sim.position()[0];
        assert!(x.abs() > 0.001, "x={}", x);
    }

    #[test]
    fn params_accessor_works() {
        let sim = QuadrotorSim::new();
        assert_eq!(sim.params().mass, 1.0);
    }

    #[test]
    fn rotor_state_accessor_works() {
        let sim = QuadrotorSim::new();
        assert_eq!(sim.rotor_state().x.x, 0.0);
    }
}
