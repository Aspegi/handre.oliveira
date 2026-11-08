#![deny(unsafe_code)]

use libm::{atan2f, sqrtf};

pub struct Guidance {
    pub waypoints: [[f32; 3]; 3],
    pub current_index: usize,
    pub k_pitch: f32,
    pub max_pitch: f32,
    pub arrival_radius: f32,        // waypoints intermedios: passagem rapida
    pub arrival_radius_final: f32,  // ultimo waypoint: precisao
}

impl Guidance {
    pub fn new() -> Self {
        Self {
            waypoints: [
                [100.0, 50.0, 0.0],   // A
                [150.0, -30.0, 0.0],  // B
                [0.0, 0.0, 0.0],      // C (base)
            ],
            current_index: 0,
            k_pitch: 0.01,
            max_pitch: 15.0_f32.to_radians(),
            arrival_radius: 2.0,        // A e B: passa a voar, sem travar
            arrival_radius_final: 0.5,  // C: precisao
        }
    }

    pub fn update(&mut self, position: [f32; 3]) -> ([f32; 3], bool) {
        let wp = self.waypoints[self.current_index];
        let dn = wp[0] - position[0];
        let de = wp[1] - position[1];
        let dist = sqrtf(dn * dn + de * de);

        // Raio adaptativo: 0.5m no ultimo, 2.0m nos intermedios
        let radius = if self.current_index + 1 >= self.waypoints.len() {
            self.arrival_radius_final
        } else {
            self.arrival_radius
        };

        if dist < radius {
            if self.current_index + 1 >= self.waypoints.len() {
                return ([0.0, 0.0, 0.0], true);
            }
            self.current_index += 1;
            let wp = self.waypoints[self.current_index];
            let dn = wp[0] - position[0];
            let de = wp[1] - position[1];
            let dist = sqrtf(dn * dn + de * de);
            let yaw = atan2f(de, dn);
            let mut pitch = self.k_pitch * dist;
            if pitch > self.max_pitch { pitch = self.max_pitch; }
            return ([0.0, pitch, yaw], false);
        }

        let yaw = atan2f(de, dn);
        let mut pitch = self.k_pitch * dist;
        if pitch > self.max_pitch { pitch = self.max_pitch; }

        ([0.0, pitch, yaw], false)
    }

    pub fn current_waypoint(&self) -> [f32; 3] {
        self.waypoints[self.current_index]
    }

    pub fn current_label(&self) -> char {
        match self.current_index {
            0 => 'A',
            1 => 'B',
            _ => 'C',
        }
    }
}
