#![allow(clippy::new_without_default)]
#![allow(clippy::needless_range_loop)]

//! FDIR — Fault Detection, Isolation & Recovery.
//!
//! Deteta sensores avariados (stuck/outlier), isola (marca como não-fiável),
//! e dispara ações de failsafe (RTL, hover, land).

/// Health state de um sensor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Warning,
    Stuck,
    Outlier,
    Failed,
}

/// Monitor de saúde por sensor. Deteta stuck (valor constante) e outlier (>N sigma).
pub struct SensorHealth {
    pub last_value: f32,
    pub stuck_count: u32,
    pub outlier_count: u32,
    pub stuck_threshold: u32,
    pub outlier_sigma: f32,
    pub running_mean: f32,
    pub running_var: f32,
    pub alpha: f32,
}

impl SensorHealth {
    pub fn new(initial: f32) -> Self {
        Self {
            last_value: initial,
            stuck_count: 0,
            outlier_count: 0,
            stuck_threshold: 100,
            outlier_sigma: 5.0,
            running_mean: initial,
            running_var: 1.0,
            alpha: 0.05,
        }
    }

    pub fn update(&mut self, value: f32) -> Health {
        if !value.is_finite() {
            return Health::Failed;
        }

        // 1. Stuck detector
        if (value - self.last_value).abs() < 1e-6 {
            self.stuck_count += 1;
        } else {
            self.stuck_count = 0;
        }
        self.last_value = value;

        // 2. Outlier detector
        let deviation = (value - self.running_mean).abs();
        let sigma = self.running_var.sqrt().max(1e-6);
        let is_outlier = deviation > self.outlier_sigma * sigma;
        if is_outlier {
            self.outlier_count += 1;
        } else {
            self.outlier_count = self.outlier_count.saturating_sub(1);
        }

        // 3. EMA update (só se não outlier/stuck)
        if !is_outlier && self.stuck_count == 0 {
            let diff = value - self.running_mean;
            self.running_mean += self.alpha * diff;
            self.running_var = (1.0 - self.alpha) * (self.running_var + self.alpha * diff * diff);
        }

        // 4. Classificar
        if self.stuck_count >= self.stuck_threshold {
            Health::Stuck
        } else if self.outlier_count >= 3 {
            Health::Outlier
        } else if self.stuck_count > self.stuck_threshold / 2 {
            Health::Warning
        } else {
            Health::Healthy
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailsafeTrigger {
    None,
    LowBattery,
    GeofenceBreach,
    SensorCritical,
    LossOfLink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailsafeAction {
    Continue,
    Warn,
    ReturnToLaunch,
    EmergencyLand,
    Hover,
}

pub struct Failsafe {
    pub battery_min_v: f32,
    pub geofence_radius_m: f32,
    pub geofence_center: [f32; 2],
}

impl Failsafe {
    pub fn new() -> Self {
        Self {
            battery_min_v: 10.5,
            geofence_radius_m: 500.0,
            geofence_center: [0.0, 0.0],
        }
    }

    pub fn evaluate(
        &self,
        pos: [f32; 2],
        battery_v: f32,
        sensor_health: Health,
    ) -> (FailsafeTrigger, FailsafeAction) {
        // P1: Geofence
        let dist = ((pos[0] - self.geofence_center[0]).powi(2)
            + (pos[1] - self.geofence_center[1]).powi(2))
        .sqrt();
        if dist > self.geofence_radius_m {
            return (FailsafeTrigger::GeofenceBreach, FailsafeAction::ReturnToLaunch);
        }

        // P2: Bateria
        if battery_v < self.battery_min_v * 0.85 {
            return (FailsafeTrigger::LowBattery, FailsafeAction::EmergencyLand);
        }
        if battery_v < self.battery_min_v {
            return (FailsafeTrigger::LowBattery, FailsafeAction::ReturnToLaunch);
        }

        // P3: Sensor
        match sensor_health {
            Health::Stuck | Health::Failed => {
                (FailsafeTrigger::SensorCritical, FailsafeAction::Hover)
            }
            Health::Outlier => (FailsafeTrigger::SensorCritical, FailsafeAction::Warn),
            _ => (FailsafeTrigger::None, FailsafeAction::Continue),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_sensor_stays_healthy() {
        let mut h = SensorHealth::new(50.0);
        for i in 0..200 {
            let v = 50.0 + (i as f32 * 0.1).sin() * 0.5;
            let health = h.update(v);
            assert_eq!(health, Health::Healthy, "iter {} v={}", i, v);
        }
    }

    #[test]
    fn stuck_sensor_detected() {
        let mut h = SensorHealth::new(50.0);
        // 150 iterações com o mesmo valor → deve declarar Stuck
        for i in 0..150 {
            let health = h.update(50.0);
            if i < 50 {
                // early: pode ser Healthy ou Warning
            } else if i >= 100 {
                assert_eq!(health, Health::Stuck, "iter {} devia ser Stuck", i);
            }
        }
    }

    #[test]
    fn outlier_sensor_detected() {
        let mut h = SensorHealth::new(0.0);
        // Warm-up com valores normais
        for i in 0..100 {
            h.update((i as f32 * 0.1).sin() * 0.1);
        }
        // 5 outliers consecutivos (100x maior que o normal)
        let mut last = Health::Healthy;
        for _ in 0..5 {
            last = h.update(100.0);
        }
        assert!(matches!(last, Health::Outlier | Health::Stuck),
                "esperado Outlier/Stuck, obtido {:?}", last);
    }

    #[test]
    fn failsafe_geofence_triggers_rtl() {
        let fs = Failsafe::new();
        let (trigger, action) = fs.evaluate([600.0, 0.0], 12.0, Health::Healthy);
        assert_eq!(trigger, FailsafeTrigger::GeofenceBreach);
        assert_eq!(action, FailsafeAction::ReturnToLaunch);
    }

    #[test]
    fn failsafe_low_battery_triggers_rtl() {
        let fs = Failsafe::new();
        let (trigger, action) = fs.evaluate([0.0, 0.0], 10.0, Health::Healthy);
        assert_eq!(trigger, FailsafeTrigger::LowBattery);
        assert_eq!(action, FailsafeAction::ReturnToLaunch);
    }

    #[test]
    fn failsafe_critical_battery_emergency_land() {
        let fs = Failsafe::new();
        let (trigger, action) = fs.evaluate([0.0, 0.0], 8.5, Health::Healthy);
        assert_eq!(trigger, FailsafeTrigger::LowBattery);
        assert_eq!(action, FailsafeAction::EmergencyLand);
    }

    #[test]
    fn failsafe_stuck_sensor_hover() {
        let fs = Failsafe::new();
        let (trigger, action) = fs.evaluate([0.0, 0.0], 12.0, Health::Stuck);
        assert_eq!(trigger, FailsafeTrigger::SensorCritical);
        assert_eq!(action, FailsafeAction::Hover);
    }

    #[test]
    fn failsafe_nominal_continues() {
        let fs = Failsafe::new();
        let (trigger, action) = fs.evaluate([100.0, 50.0], 12.0, Health::Healthy);
        assert_eq!(trigger, FailsafeTrigger::None);
        assert_eq!(action, FailsafeAction::Continue);
    }
}
