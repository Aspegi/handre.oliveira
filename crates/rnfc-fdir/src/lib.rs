#![cfg_attr(not(test), no_std)]
#![allow(clippy::new_without_default)]
#![allow(clippy::needless_range_loop)]



//! FDIR â€” Fault Detection, Isolation & Recovery.
//!
//! Deteta sensores avariados (stuck/outlier), isola (marca como nÃ£o-fiÃ¡vel),
//! e dispara aÃ§Ãµes de failsafe (RTL, hover, land).

use libm::sqrtf;


/// Health state de um sensor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Warning,
    Stuck,
    Outlier,
    Failed,
}

/// Monitor de saÃºde por sensor. Deteta stuck (valor constante) e outlier (>N sigma).
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
        let sigma = sqrtf(self.running_var).max(1e-6);
        let is_outlier = deviation > self.outlier_sigma * sigma;
        if is_outlier {
            self.outlier_count += 1;
        } else {
            self.outlier_count = self.outlier_count.saturating_sub(1);
        }

        // 3. EMA update (sÃ³ se nÃ£o outlier/stuck)
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
        let dx = pos[0] - self.geofence_center[0];
        let dy = pos[1] - self.geofence_center[1];
        let dist = sqrtf(dx * dx + dy * dy);
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
        // 150 iteraÃ§Ãµes com o mesmo valor â†’ deve declarar Stuck
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
/// Detector de spoofing ativo no magnetÃ³metro.
/// Compara heading do mag vs heading do gyro: se divergem abruptamente e mantÃªm,
/// e spoofing (ataque) e nao bias (drift lento).
pub struct SpoofingDetector {
    pub err_history: [f32; 10],
    pub idx: usize,
    pub threshold_rad: f32,
    pub spike_threshold_rad: f32,
    pub spike_count: u32,
}

impl SpoofingDetector {
    pub fn new() -> Self {
        Self {
            err_history: [0.0; 10],
            idx: 0,
            threshold_rad: 0.35,        // 20 graus
            spike_threshold_rad: 0.52,  // 30 graus
            spike_count: 0,
        }
    }

    /// Devolve true se spoofing detetado (deve abortar).
    pub fn update(&mut self, mag_yaw: f32, gyro_yaw: f32) -> bool {
        let mut err = mag_yaw - gyro_yaw;
        while err > core::f32::consts::PI { err -= 2.0 * core::f32::consts::PI; }
        while err < -core::f32::consts::PI { err += 2.0 * core::f32::consts::PI; }

        self.err_history[self.idx] = err;
        self.idx = (self.idx + 1) % 10;

        // Spoofing = erro persistente > threshold_rad durante 10 amostras
        let all_high = self.err_history.iter().all(|&e| e.abs() > self.threshold_rad);
        if all_high { return true; }

        // Spoofing abrupto = spike unico > spike_threshold
        if err.abs() > self.spike_threshold_rad {
            self.spike_count += 1;
            if self.spike_count >= 50 { return true; } // 0.5s a 100Hz
        } else {
            self.spike_count = self.spike_count.saturating_sub(1);
        }
        false
    }
}

#[cfg(test)]
mod tests_spoofing {
    use super::*;

    #[test]
    fn no_spoofing_when_aligned() {
        let mut d = SpoofingDetector::new();
        for _ in 0..100 {
            assert!(!d.update(0.5, 0.5));
        }
    }

    #[test]
    fn slow_drift_not_spoofing() {
        let mut d = SpoofingDetector::new();
        // Drift lento: err cresce gradualmente
        for i in 0..300 {
            let err = (i as f32) * 0.001;
            let detected = d.update(err, 0.0);
            // Nao deve detetar (drift lento, nao e spoofing)
            assert!(!detected, "drift lento detetado como spoofing no iter {}", i);
        }
    }

    #[test]
    fn abrupt_spoofing_detected() {
        let mut d = SpoofingDetector::new();
        // Baseline alinhado
        for _ in 0..20 { d.update(0.0, 0.0); }
        // Ataque abrupto +90 graus mantido
        let mut detected = false;
        for _ in 0..100 {
            if d.update(1.57, 0.0) { detected = true; break; }
        }
        assert!(detected, "spoofing +90 nao detetado");
    }

    #[test]
    fn brief_spike_not_immediate() {
        let mut d = SpoofingDetector::new();
        // Alinhado com spike unico
        for _ in 0..20 { d.update(0.0, 0.0); }
        // Spike unico nao deve disparar imediatamente (spike_count < 50)
        assert!(!d.update(1.57, 0.0));
    }
}
// ==================== 2-of-3 SENSOR VOTING ====================
/// Aerospace-grade 2-of-3 sensor voting.
/// Com 3 sensores, se 1 falha, usa os outros 2. So falha se 2+ divergem.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SensorStatus {
    AllHealthy,
    OneDegraded,
    TwoDisagree,
    TotalFailure,
}

#[derive(Clone, Copy, Debug)]
pub struct VotedValue {
    pub value: f32,
    pub status: SensorStatus,
}

pub struct SensorVoter {
    pub threshold: f32,
}

impl SensorVoter {
    pub fn new() -> Self {
        Self { threshold: 5.0 }  // 5% divergencia = outlier
    }

    pub fn vote(&self, s1: f32, s2: f32, s3: f32) -> VotedValue {
        let d12 = (s1 - s2).abs();
        let d13 = (s1 - s3).abs();
        let d23 = (s2 - s3).abs();
        let mean = (s1 + s2 + s3) / 3.0;
        let thresh = self.threshold * 50.0 / 100.0; // fixo: 5% de MAG_NORM(50) = 2.5

        let close12 = d12 < thresh;
        let close13 = d13 < thresh;
        let close23 = d23 < thresh;

        match (close12, close13, close23) {
            (true, true, true) => VotedValue { value: mean, status: SensorStatus::AllHealthy },
            (true, false, false) => VotedValue { value: (s1 + s2) / 2.0, status: SensorStatus::OneDegraded },
            (false, true, false) => VotedValue { value: (s1 + s3) / 2.0, status: SensorStatus::OneDegraded },
            (false, false, true) => VotedValue { value: (s2 + s3) / 2.0, status: SensorStatus::OneDegraded },
            (true, true, false) => VotedValue { value: (s1 + s2) / 2.0, status: SensorStatus::OneDegraded },
            (true, false, true) => VotedValue { value: (s1 + s3) / 2.0, status: SensorStatus::OneDegraded },
            (false, true, true) => VotedValue { value: (s2 + s3) / 2.0, status: SensorStatus::OneDegraded },
            (false, false, false) => VotedValue { value: mean, status: SensorStatus::TotalFailure },
        }
    }
}

#[cfg(test)]
mod tests_voter {
    use super::*;

    #[test]
    fn three_agree_returns_mean() {
        let v = SensorVoter::new();
        let r = v.vote(50.0, 50.1, 49.9);
        assert_eq!(r.status, SensorStatus::AllHealthy);
        assert!((r.value - 50.0).abs() < 0.2);
    }

    #[test]
    fn one_outlier_isolated() {
        let v = SensorVoter::new();
        let r = v.vote(50.0, 50.1, 80.0);  // s3 Ã© outlier
        assert_eq!(r.status, SensorStatus::OneDegraded);
        assert!((r.value - 50.05).abs() < 0.2, "value={} devia ser ~50", r.value);
    }

    #[test]
    fn two_vs_one_uses_majority() {
        let v = SensorVoter::new();
        let r = v.vote(50.0, 70.0, 80.0);  // 1 perto de 2? Nao, todos separados
        // Todos separados > 5% â†’ TotalFailure OU TwoDisagree
        assert!(matches!(r.status, SensorStatus::TotalFailure | SensorStatus::OneDegraded));
    }

    #[test]
    fn all_disagree_failsafe() {
        let v = SensorVoter::new();
        let r = v.vote(10.0, 100.0, 500.0);
        assert_eq!(r.status, SensorStatus::TotalFailure);
    }

    #[test]
    fn slow_drift_all_healthy() {
        let v = SensorVoter::new();
        for i in 0..100 {
            let base = 50.0 + i as f32 * 0.01;
            let r = v.vote(base, base + 0.1, base - 0.1);
            assert_eq!(r.status, SensorStatus::AllHealthy, "drift devia ser AllHealthy, iter={}", i);
        }
    }
}
#[cfg(test)]
mod tests_layered_defense {
    use super::*;
    use crate::SpoofingDetector;

    /// LIMITE DOCUMENTADO: se 2 de 3 sensores forem spoofed para o MESMO valor,
    /// o SensorVoter escolhe a maioria errada. So a defesa em camadas (SpoofingDetector
    /// com gyro) deteta. Este teste prova a limitacao + a defesa em profundidade.
    #[test]
    fn layered_defense_2_of_3_compromised() {
        // 1 sensor real (50), 2 spoofed (65 cada - mesma direcao)
        let v = SensorVoter::new();
        let r = v.vote(50.0, 65.0, 65.0);

        // Voter comprometido: escolhe os 2 spoofed (maioria falsa)
        assert_eq!(r.status, SensorStatus::OneDegraded,
            "Voter isola a minoria (sensor real) - LIMITACAO do 2-of-3 sem temporal analysis");
        assert!((r.value - 65.0).abs() < 1.0,
            "Voter usa valor spoofed {} (devia ser 50, mas e limitacao conhecida)", r.value);

        // Defesa em camadas: SpoofingDetector usa gyro (nao-mag) para verificar
        let mut sd = SpoofingDetector::new();
        let gyro_yaw = 0.0;
        let spoofed_yaw = 0.52; // +30 graus
        let mut detected = false;
        for _ in 0..200 {
            if sd.update(spoofed_yaw, gyro_yaw) { detected = true; break; }
        }
        assert!(detected,
            "SpoofingDetector DEVE detetar mesmo com SensorVoter comprometido - defesa em profundidade");
    }

    /// Verifica que 2 sensores espalhados (nao concordantes) NAO enganam o voter
    #[test]
    fn two_random_outliers_not_same_value_isolated() {
        let v = SensorVoter::new();
        // mag1=50 (real), mag2=100 (outlier aleatorio), mag3=150 (outlier aleatorio)
        let r = v.vote(50.0, 100.0, 150.0);
        assert_eq!(r.status, SensorStatus::TotalFailure,
            "Sem concordancia entre spoofed, voter declara TotalFailure");
    }
}
// ==================== ZOMBIE DETECTOR ====================
/// Detecta "zombie orbit": drone estagnado longe do goal por muito tempo.
/// Fecha L9 do SAFETY.md (adversarial MC encontrou timeout a 505m).
pub struct ZombieDetector {
    pub stuck_threshold_s: f32,
    pub dist_threshold_m: f32,
    pub min_progress_m: f32,
    last_best_dist: f32,
    last_progress_t: f32,
    initialized: bool,
}

impl ZombieDetector {
    pub fn new() -> Self {
        Self {
            stuck_threshold_s: 60.0,
            dist_threshold_m: 50.0,
            min_progress_m: 0.5,
            last_best_dist: f32::INFINITY,
            last_progress_t: 0.0,
            initialized: false,
        }
    }

    /// Returns true se zombie detected (stuck >60s a >50m).
    pub fn update(&mut self, dist_to_goal: f32, t: f32) -> bool {
        if !self.initialized {
            self.last_best_dist = dist_to_goal;
            self.last_progress_t = t;
            self.initialized = true;
            return false;
        }

        if !dist_to_goal.is_finite() || !t.is_finite() {
            return false;
        }

        if dist_to_goal <= self.dist_threshold_m {
            self.last_best_dist = dist_to_goal;
            self.last_progress_t = t;
            return false;
        }

        if dist_to_goal < self.last_best_dist - self.min_progress_m {
            self.last_best_dist = dist_to_goal;
            self.last_progress_t = t;
            return false;
        }

        if dist_to_goal < self.last_best_dist {
            self.last_best_dist = dist_to_goal;
        }

        (t - self.last_progress_t) > self.stuck_threshold_s
    }
}

#[cfg(test)]
mod tests_zombie {
    use super::*;

    #[test]
    fn fresh_detector_does_not_fire_before_threshold() {
        let mut z = ZombieDetector::new();
        // 59s a 500m sem progresso — ainda nao dispara (threshold=60s)
        for t in [0.0_f32, 1.0, 30.0, 59.0] {
            assert!(!z.update(500.0, t), "nao devia disparar antes de 60s");
        }
        // 61s — AGORA sim, e zombie
        assert!(z.update(500.0, 61.0), "devia disparar apos 60s a 500m");
    }

    #[test]
    fn fresh_detector_with_progress_no_fire() {
        let mut z = ZombieDetector::new();
        // Drone comeca a 500m e PROGRIDE - nao e zombie
        z.update(500.0, 0.0);
        z.update(400.0, 10.0);
        z.update(300.0, 20.0);
        z.update(200.0, 30.0);
        z.update(100.0, 40.0);
        assert!(!z.update(60.0, 50.0), "progresso real, nao e zombie");
    }

    #[test]
    fn progress_resets_timer() {
        let mut z = ZombieDetector::new();
        z.update(500.0, 0.0);
        z.update(500.0, 30.0);
        z.update(400.0, 40.0);
        z.update(400.0, 90.0);
        assert!(!z.update(400.0, 100.0), "progresso resetou timer");
    }

    #[test]
    fn stuck_far_from_goal_triggers() {
        let mut z = ZombieDetector::new();
        z.update(500.0, 0.0);
        assert!(z.update(500.0, 61.0), "devia disparar apos 60s a 500m");
    }

    #[test]
    fn close_to_goal_never_zombie() {
        let mut z = ZombieDetector::new();
        z.update(30.0, 0.0);
        assert!(!z.update(30.0, 61.0), "perto do goal nao e zombie");
    }

    #[test]
    fn no_nan_no_panic() {
        let mut z = ZombieDetector::new();
        z.update(f32::NAN, 0.0);
        z.update(f32::INFINITY, 10.0);
        z.update(-f32::INFINITY, 20.0);
        z.update(500.0, f32::NAN);
    }

    #[test]
    fn slow_progress_no_zombie() {
        let mut z = ZombieDetector::new();
        z.update(500.0, 0.0);
        for i in 1..=100 {
            let dist = 500.0 - i as f32;
            if z.update(dist, i as f32 * 10.0) {
                panic!("progresso lento nao devia disparar");
            }
        }
    }
}
