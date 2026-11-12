//! Omni — 360° perception with graceful degradation.
//!
//! 72 bins of 5° each around the drone. N cameras distributed
//! around the ring; each camera covers a configurable FOV.
//! When a camera fails, its bins become blind — FDIR triggers Hover/RTL.

#![allow(clippy::needless_range_loop)]

pub const NUM_BINS: usize = 72;
pub const BIN_WIDTH_DEG: f32 = 360.0 / NUM_BINS as f32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OmniHealth {
    Healthy,
    Degraded,
    Blind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OmniAction {
    Continue,
    Hover,
    Rtl,
    Land,
}

#[derive(Clone, Copy, Debug)]
pub struct Obstacle {
    pub angle_deg: f32,
    pub distance_m: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SimulatedCamera {
    pub center_angle_deg: f32,
    pub fov_deg: f32,
    pub healthy: bool,
}

impl SimulatedCamera {
    pub fn new(center_angle_deg: f32, fov_deg: f32) -> Self {
        Self {
            center_angle_deg,
            fov_deg,
            healthy: true,
        }
    }
    pub fn sees(&self, angle_deg: f32) -> bool {
        if !self.healthy {
            return false;
        }
        let half = self.fov_deg / 2.0;
        let mut diff = (angle_deg - self.center_angle_deg).abs() % 360.0;
        if diff > 180.0 {
            diff = 360.0 - diff;
        }
        diff <= half
    }
}

pub struct OmniSensorSuite {
    pub cameras: Vec<SimulatedCamera>,
}

impl OmniSensorSuite {
    pub fn new(n: usize, fov_deg: f32) -> Self {
        let mut cameras = Vec::with_capacity(n);
        let spacing = 360.0 / n as f32;
        for i in 0..n {
            cameras.push(SimulatedCamera::new(i as f32 * spacing, fov_deg));
        }
        Self { cameras }
    }
    pub fn fail_camera(&mut self, idx: usize) {
        if let Some(c) = self.cameras.get_mut(idx) {
            c.healthy = false;
        }
    }
    pub fn sense(&self, obstacles: &[Obstacle]) -> OmniRing {
        let mut bins = [f32::INFINITY; NUM_BINS];
        let mut coverage_mask = [false; NUM_BINS];
        for bin in 0..NUM_BINS {
            let bin_center = (bin as f32 + 0.5) * BIN_WIDTH_DEG;
            let any_sees = self.cameras.iter().any(|c| c.sees(bin_center));
            if !any_sees {
                continue;
            }
            coverage_mask[bin] = true;
            for obs in obstacles {
                let obs_angle = ((obs.angle_deg % 360.0) + 360.0) % 360.0;
                let obs_bin = (obs_angle / BIN_WIDTH_DEG) as usize % NUM_BINS;
                if obs_bin == bin && obs.distance_m < bins[bin] {
                    bins[bin] = obs.distance_m;
                }
            }
        }
        let covered = coverage_mask.iter().filter(|&&b| b).count();
        let coverage_pct = covered as f32 / NUM_BINS as f32 * 100.0;
        let health = if coverage_pct >= 95.0 {
            OmniHealth::Healthy
        } else if coverage_pct >= 50.0 {
            OmniHealth::Degraded
        } else {
            OmniHealth::Blind
        };
        OmniRing {
            bins,
            coverage_mask,
            health,
            coverage_pct,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OmniRing {
    pub bins: [f32; NUM_BINS],
    pub coverage_mask: [bool; NUM_BINS],
    pub health: OmniHealth,
    pub coverage_pct: f32,
}

impl OmniRing {
    pub fn blind_sectors(&self) -> Vec<(f32, f32)> {
        let mut sectors = Vec::new();
        let mut start: Option<usize> = None;
        for i in 0..NUM_BINS {
            let covered = self.coverage_mask[i];
            match (start, covered) {
                (None, false) => start = Some(i),
                (Some(s), true) => {
                    sectors.push((s as f32 * BIN_WIDTH_DEG, i as f32 * BIN_WIDTH_DEG));
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(s) = start {
            sectors.push((s as f32 * BIN_WIDTH_DEG, 360.0));
        }
        sectors
    }
    pub fn safety_action(&self) -> OmniAction {
        match self.health {
            OmniHealth::Healthy => OmniAction::Continue,
            OmniHealth::Degraded => OmniAction::Hover,
            OmniHealth::Blind => OmniAction::Rtl,
        }
    }
    pub fn nearest_obstacle(&self) -> Option<f32> {
        let mut min = f32::INFINITY;
        for (i, &b) in self.bins.iter().enumerate() {
            if self.coverage_mask[i] && b < min {
                min = b;
            }
        }
        if min.is_finite() {
            Some(min)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_coverage_three_cameras() {
        let suite = OmniSensorSuite::new(3, 120.0);
        let ring = suite.sense(&[]);
        assert_eq!(ring.health, OmniHealth::Healthy);
        assert!(ring.coverage_pct >= 95.0, "coverage={}", ring.coverage_pct);
        assert!(ring.blind_sectors().is_empty());
    }

    #[test]
    fn one_camera_fails_creates_blind_sectors() {
        let mut suite = OmniSensorSuite::new(3, 120.0);
        suite.fail_camera(0);
        let ring = suite.sense(&[]);
        assert!(ring.coverage_pct < 80.0, "coverage={}", ring.coverage_pct);
        assert_eq!(ring.health, OmniHealth::Degraded);
        assert_eq!(ring.blind_sectors().len(), 2);
    }

    #[test]
    fn two_cameras_fail_triggers_blind_and_rtl() {
        let mut suite = OmniSensorSuite::new(3, 120.0);
        suite.fail_camera(0);
        suite.fail_camera(1);
        let ring = suite.sense(&[]);
        assert_eq!(ring.health, OmniHealth::Blind);
        assert_eq!(ring.safety_action(), OmniAction::Rtl);
    }

    #[test]
    fn degraded_triggers_hover() {
        let mut suite = OmniSensorSuite::new(3, 120.0);
        suite.fail_camera(0);
        assert_eq!(suite.sense(&[]).safety_action(), OmniAction::Hover);
    }

    #[test]
    fn healthy_triggers_continue() {
        let suite = OmniSensorSuite::new(3, 120.0);
        assert_eq!(suite.sense(&[]).safety_action(), OmniAction::Continue);
    }

    #[test]
    fn obstacle_recorded_in_correct_bin() {
        let suite = OmniSensorSuite::new(3, 120.0);
        let ring = suite.sense(&[Obstacle {
            angle_deg: 90.0,
            distance_m: 5.0,
        }]);
        assert_eq!(ring.bins[18], 5.0);
    }

    #[test]
    fn nearest_obstacle_wins_in_bin() {
        let suite = OmniSensorSuite::new(3, 120.0);
        let ring = suite.sense(&[
            Obstacle {
                angle_deg: 90.0,
                distance_m: 10.0,
            },
            Obstacle {
                angle_deg: 90.0,
                distance_m: 3.0,
            },
        ]);
        assert_eq!(ring.bins[18], 3.0);
    }

    #[test]
    fn nearest_obstacle_method() {
        let suite = OmniSensorSuite::new(3, 120.0);
        let ring = suite.sense(&[Obstacle {
            angle_deg: 0.0,
            distance_m: 2.5,
        }]);
        assert_eq!(ring.nearest_obstacle(), Some(2.5));
    }

    #[test]
    fn no_obstacles_returns_none() {
        let suite = OmniSensorSuite::new(3, 120.0);
        assert_eq!(suite.sense(&[]).nearest_obstacle(), None);
    }

    #[test]
    fn four_cameras_overlapping_fov_still_covered() {
        let suite = OmniSensorSuite::new(4, 120.0);
        assert_eq!(suite.sense(&[]).health, OmniHealth::Healthy);
    }

    #[test]
    fn deterministic_output() {
        let suite = OmniSensorSuite::new(3, 120.0);
        let obs = Obstacle {
            angle_deg: 45.0,
            distance_m: 4.0,
        };
        let r1 = suite.sense(&[obs]);
        let r2 = suite.sense(&[obs]);
        assert_eq!(r1.coverage_pct, r2.coverage_pct);
        assert_eq!(r1.health, r2.health);
    }
}
