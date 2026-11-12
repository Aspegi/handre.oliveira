//! Alert — unified alert system for safety envelope violations.
//!
//! Consumes events (altitude, speed, battery, obstacles) and emits:
//! 1. MAVLink STATUSTEXT (msgid 253) for the ground station
//! 2. A block in the CertChain (cryptographic proof the alert was raised)
//! 3. Haptic feedback level for the pilot
//!
//! Thresholds:
//!   Info     — inside envelope but approaching limit
//!   Warning  — at limit
//!   Critical — exceeded limit
//!   Emergency— exceeded by >10% (triggers EmergencyLand)

#[allow(unused_imports)]
use rnfc_certchain::{CertChain, SafetyState};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertLevel { Info, Warning, Critical, Emergency }

#[derive(Clone, Debug)]
pub struct Alert {
    pub level: AlertLevel,
    pub code: &'static str,
    pub message: String,
    pub timestamp_s: f32,
}

impl Alert {
    /// Serialize to MAVLink STATUSTEXT payload format (simplified):
    /// severity(1) + text(up to 50 bytes).
    pub fn to_statustext(&self) -> Vec<u8> {
        let severity: u8 = match self.level {
            AlertLevel::Info => 6,      // MAV_SEVERITY_INFO
            AlertLevel::Warning => 4,   // MAV_SEVERITY_WARNING
            AlertLevel::Critical => 2,  // MAV_SEVERITY_CRITICAL
            AlertLevel::Emergency => 0, // MAV_SEVERITY_EMERGENCY
        };
        let mut out = vec![severity];
        let text = format!("[{}] {}", self.code, self.message);
        let bytes = text.as_bytes();
        let n = bytes.len().min(50);
        out.extend_from_slice(&bytes[..n]);
        out
    }

    pub fn to_haptic_strong(&self) -> bool {
        matches!(self.level, AlertLevel::Critical | AlertLevel::Emergency)
    }
}

pub struct AlertManager {
    pub min_alt_m: f32,
    pub max_alt_m: f32,
    pub emitted: u32,
}

impl AlertManager {
    pub fn new(min_alt_m: f32, max_alt_m: f32) -> Self {
        Self { min_alt_m, max_alt_m, emitted: 0 }
    }

    /// Check altitude against envelope. Returns Some(Alert) if a threshold crossed.
    pub fn check_altitude(&mut self, alt_m: f32, t_s: f32) -> Option<Alert> {
        let (level, msg) = if alt_m > self.max_alt_m * 1.1 {
            (AlertLevel::Emergency, format!("ALT {:.1}m > {:.0}m (+10%)", alt_m, self.max_alt_m))
        } else if alt_m > self.max_alt_m {
            (AlertLevel::Critical, format!("ALT {:.1}m > {:.0}m", alt_m, self.max_alt_m))
        } else if alt_m > self.max_alt_m * 0.9 {
            (AlertLevel::Warning, format!("ALT {:.1}m near max", alt_m))
        } else if alt_m < self.min_alt_m {
            (AlertLevel::Critical, format!("ALT {:.1}m < {:.0}m", alt_m, self.min_alt_m))
        } else if alt_m < self.min_alt_m * 1.5 {
            (AlertLevel::Warning, format!("ALT {:.1}m near min", alt_m))
        } else {
            return None;
        };
        self.emitted += 1;
        Some(Alert { level, code: "ALT", message: msg, timestamp_s: t_s })
    }

    pub fn record_in_chain(&self, chain: &mut CertChain, alert: &Alert) -> u64 {
        let mut buf = Vec::new();
        buf.extend_from_slice(alert.message.as_bytes());
        buf.extend_from_slice(&alert.timestamp_s.to_le_bytes());
        buf.push(alert.level as u8);
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in &buf { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        let _ = chain.append(alert.timestamp_s, 1, h, SafetyState::FailsafeTriggered);
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_envelope_no_alert() {
        let mut m = AlertManager::new(5.0, 120.0);
        assert!(m.check_altitude(50.0, 1.0).is_none());
    }

    #[test]
    fn near_max_emits_warning() {
        let mut m = AlertManager::new(5.0, 120.0);
        let a = m.check_altitude(110.0, 1.0).unwrap();
        assert_eq!(a.level, AlertLevel::Warning);
    }

    #[test]
    fn exceeds_max_emits_critical() {
        let mut m = AlertManager::new(5.0, 120.0);
        let a = m.check_altitude(125.0, 1.0).unwrap();
        assert_eq!(a.level, AlertLevel::Critical);
        assert!(a.message.contains("125.0m"));
    }

    #[test]
    fn exceeds_max_by_10pct_emits_emergency() {
        let mut m = AlertManager::new(5.0, 120.0);
        let a = m.check_altitude(135.0, 1.0).unwrap();
        assert_eq!(a.level, AlertLevel::Emergency);
    }

    #[test]
    fn below_min_emits_critical() {
        let mut m = AlertManager::new(5.0, 120.0);
        let a = m.check_altitude(3.0, 1.0).unwrap();
        assert_eq!(a.level, AlertLevel::Critical);
    }

    #[test]
    fn statustext_has_severity_and_text() {
        let a = Alert { level: AlertLevel::Critical, code: "ALT",
            message: "125m > 120m".into(), timestamp_s: 1.0 };
        let st = a.to_statustext();
        assert_eq!(st[0], 2); // CRITICAL
        let text = String::from_utf8_lossy(&st[1..]);
        assert!(text.contains("ALT"));
        assert!(text.contains("125m"));
    }

    #[test]
    fn emergency_triggers_haptic() {
        let a = Alert { level: AlertLevel::Emergency, code: "ALT",
            message: "x".into(), timestamp_s: 0.0 };
        assert!(a.to_haptic_strong());
    }

    #[test]
    fn warning_does_not_trigger_haptic() {
        let a = Alert { level: AlertLevel::Warning, code: "ALT",
            message: "x".into(), timestamp_s: 0.0 };
        assert!(!a.to_haptic_strong());
    }

    #[test]
    fn alert_recorded_in_chain() {
        let mut m = AlertManager::new(5.0, 120.0);
        let a = m.check_altitude(125.0, 1.0).unwrap();
        let mut c = CertChain::new(1, "alert_test");
        let h = m.record_in_chain(&mut c, &a);
        assert_ne!(h, 0);
        assert_eq!(c.block_count(), 1);
        assert!(c.verify());
    }

    #[test]
    fn emitted_counter() {
        let mut m = AlertManager::new(5.0, 120.0);
        let _ = m.check_altitude(125.0, 1.0);
        let _ = m.check_altitude(130.0, 2.0);
        assert_eq!(m.emitted, 2);
    }
}
