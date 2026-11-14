// Copyright (c) 2026 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)
#![deny(unsafe_code)]

use crate::math::{atan2f, sqrtf};

pub struct Guidance {
    pub waypoints: [[f32; 3]; 3],
    pub current_index: usize,
    pub k_pitch: f32,
    pub max_pitch: f32,
    pub arrival_radius: f32,
    pub arrival_radius_final: f32,
}

impl Guidance {
    pub fn new() -> Self {
        Self {
            waypoints: [[100.0, 50.0, 0.0], [150.0, -30.0, 0.0], [0.0, 0.0, 0.0]],
            current_index: 0,
            k_pitch: 0.01,
            max_pitch: 15.0_f32.to_radians(),
            arrival_radius: 2.0,
            arrival_radius_final: 0.5,
        }
    }

    pub fn update(&mut self, position: [f32; 3]) -> ([f32; 3], bool) {
        let wp = self.waypoints[self.current_index];
        let dn = wp[0] - position[0];
        let de = wp[1] - position[1];
        let dist = sqrtf(dn * dn + de * de);

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
            if pitch > self.max_pitch {
                pitch = self.max_pitch;
            }
            if pitch < 0.0 {
                pitch = 0.0;
            }
            return ([0.0, pitch, yaw], false);
        }

        let yaw = atan2f(de, dn);
        let mut pitch = self.k_pitch * dist;
        if pitch > self.max_pitch {
            pitch = self.max_pitch;
        }
        if pitch < 0.0 {
            pitch = 0.0;
        }

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

#[cfg(kani)]
mod verification {
    use super::*;

    /// Prova que o Guidance nunca entra em panic com entradas finitas.
    #[kani::proof]
    #[kani::unwind(8)]
    fn guidance_no_panic() {
        let mut g = Guidance::new();

        let pos: [f32; 3] = [kani::any(), kani::any(), kani::any()];
        kani::assume(pos[0].is_finite() && pos[0].abs() < 1.0e4);
        kani::assume(pos[1].is_finite() && pos[1].abs() < 1.0e4);
        kani::assume(pos[2].is_finite() && pos[2].abs() < 1.0e4);

        let _ = g.update(pos);
    }

    /// Prova que os setpoints estao dentro dos limites fisicos.
    #[kani::proof]
    #[kani::unwind(8)]
    fn guidance_setpoints_bounded() {
        let mut g = Guidance::new();
        let max_p = g.max_pitch;

        let pos: [f32; 3] = [kani::any(), kani::any(), kani::any()];
        kani::assume(pos[0].is_finite() && pos[0].abs() < 1.0e4);
        kani::assume(pos[1].is_finite() && pos[1].abs() < 1.0e4);
        kani::assume(pos[2].is_finite() && pos[2].abs() < 1.0e4);

        let (sp, _arrived) = g.update(pos);

        kani::assert(sp[0] == 0.0, "roll is 0");
        kani::assert(sp[1] >= 0.0, "pitch non-negative");
        kani::assert(sp[1] <= max_p, "pitch <= max_pitch");

        let pi = core::f32::consts::PI;
        kani::assert(sp[2] >= -pi, "yaw >= -pi");
        kani::assert(sp[2] <= pi, "yaw <= pi");

        kani::assert(sp[0].is_finite(), "roll finite");
        kani::assert(sp[1].is_finite(), "pitch finite");
        kani::assert(sp[2].is_finite(), "yaw finite");
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn new_default_state() {
        let g = Guidance::new();
        assert_eq!(g.waypoints.len(), 3);
        assert_eq!(g.current_index, 0);
        assert_eq!(g.arrival_radius, 2.0);
        assert_eq!(g.arrival_radius_final, 0.5);
    }

    #[test]
    fn not_arrived_when_far() {
        let mut g = Guidance::new();
        let (_, arrived) = g.update([0.0, 0.0, 0.0]);
        assert!(!arrived);
        assert_eq!(g.current_index, 0);
    }

    #[test]
    fn advances_at_intermediate_waypoint() {
        let mut g = Guidance::new();
        let (_, arrived) = g.update([100.0, 50.0, 0.0]);
        assert!(!arrived);
        assert_eq!(g.current_index, 1);
    }

    #[test]
    fn arrived_only_at_final_waypoint() {
        let mut g = Guidance::new();
        g.update([100.0, 50.0, 0.0]);
        g.update([150.0, -30.0, 0.0]);
        assert_eq!(g.current_index, 2);
        let (_, arrived) = g.update([0.0, 0.0, 0.0]);
        assert!(arrived);
    }

    #[test]
    fn yaw_points_to_first_waypoint() {
        let mut g = Guidance::new();
        let (sp, _) = g.update([0.0, 0.0, 0.0]);
        let expected = 50.0_f32.atan2(100.0);
        assert!(
            approx(sp[2], expected, 1e-3),
            "yaw={} expected={}",
            sp[2],
            expected
        );
    }

    #[test]
    fn pitch_bounded_by_max_pitch() {
        let mut g = Guidance::new();
        let (sp, _) = g.update([-100000.0, -100000.0, 0.0]);
        assert!(
            sp[1] <= g.max_pitch + 1e-6,
            "pitch={} max={}",
            sp[1],
            g.max_pitch
        );
        assert!(sp[1] > 0.0, "pitch devia ser positivo, got {}", sp[1]);
    }

    #[test]
    fn no_nan_adversarial() {
        for n in [-1e6_f32, -1e3, 0.0, 1e3, 1e6] {
            for e in [-1e6_f32, -1e3, 0.0, 1e3, 1e6] {
                let mut g = Guidance::new();
                let (sp, _) = g.update([n, e, 0.0]);
                assert!(!sp[0].is_nan(), "roll NaN at ({},{})", n, e);
                assert!(!sp[1].is_nan(), "pitch NaN at ({},{})", n, e);
                assert!(!sp[2].is_nan(), "yaw NaN at ({},{})", n, e);
            }
        }
    }

    #[test]
    fn current_index_monotonic() {
        let mut g = Guidance::new();
        assert_eq!(g.current_index, 0);
        g.update([100.0, 50.0, 0.0]);
        assert_eq!(g.current_index, 1);
        g.update([150.0, -30.0, 0.0]);
        assert_eq!(g.current_index, 2);
        g.update([0.0, 0.0, 0.0]);
        assert_eq!(g.current_index, 2);
    }
}
