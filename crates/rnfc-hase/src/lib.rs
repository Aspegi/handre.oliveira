#![allow(clippy::needless_range_loop)]
#![allow(clippy::new_without_default)]
//! HASE — Human-Authority Safety Envelope.
//!
//! Camada entre o piloto humano e o drone. Preserva autoridade do piloto,
//! mas traduz os limites do FDIR em resistencia haptica proporcional ao risco.
//!
//! Pipeline:
//!   pilot_cmd -> predict_2s -> compute_risk -> blend -> safe_cmd + haptic
//!
//! Integrado com CERTIS (regista overrides para safety case).

use libm::sqrtf;
use rnfc_certis::{CertisRecorder, SafetyEventType};

/// Comando do piloto.
#[derive(Clone, Copy, Debug, Default)]
pub struct PilotCommand {
    pub roll: f32,     // rad
    pub pitch: f32,    // rad
    pub yaw_rate: f32, // rad/s
    pub throttle: f32, // 0.0-1.0
}

/// Estado actual do drone (vem do NavCore).
#[derive(Clone, Copy, Debug, Default)]
pub struct DroneState {
    pub position: [f32; 2], // N, E
    pub velocity: [f32; 2], // m/s
    pub altitude: f32,      // m
    pub roll: f32,
    pub pitch: f32,
}

/// Envelope de seguranca (limites operacionais).
pub struct SafetyEnvelope {
    pub max_roll_rad: f32,
    pub max_pitch_rad: f32,
    pub min_altitude_m: f32,
    pub max_altitude_m: f32,
    pub geofence_center: [f32; 2],
    pub geofence_radius_m: f32,
}

impl SafetyEnvelope {
    pub fn new() -> Self {
        Self {
            max_roll_rad: 0.5236,  // 30 graus
            max_pitch_rad: 0.5236, // 30 graus
            min_altitude_m: 2.0,
            max_altitude_m: 100.0,
            geofence_center: [0.0, 0.0],
            geofence_radius_m: 500.0,
        }
    }
}

/// Resultado do filtro HASE.
#[derive(Clone, Debug)]
pub struct HaseResult {
    pub safe_command: PilotCommand,
    pub haptic: HapticFeedback,
    pub violation: Option<EnvelopeViolation>,
}

/// Haptic feedback (resistencia ao piloto).
#[derive(Clone, Copy, Debug, Default)]
pub struct HapticFeedback {
    pub roll_resistance: f32, // 0.0 = livre, 1.0 = resistência máxima
    pub pitch_resistance: f32,
    pub throttle_resistance: f32,
    pub vibration_pattern: VibrationPattern,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VibrationPattern {
    #[default]
    None,
    Soft,   // resistência moderada
    Strong, // violação iminente
    Pulse,  // falha detectada (motor loss)
}

/// Tipo de violação detectada.
#[derive(Clone, Debug)]
pub enum EnvelopeViolation {
    RollLimit,
    PitchLimit,
    AltitudeLow,
    AltitudeHigh,
    GeofenceBreach,
    MotorLoss,
}

/// Filtro HASE.
pub struct HaseFilter {
    pub envelope: SafetyEnvelope,
    pub prediction_horizon_s: f32,
    pub attack_active: bool,     // FDIR detetou ataque
    pub motor_loss_active: bool, // LNN-FTC a controlar
}

impl HaseFilter {
    pub fn new() -> Self {
        Self {
            envelope: SafetyEnvelope::new(),
            prediction_horizon_s: 2.0,
            attack_active: false,
            motor_loss_active: false,
        }
    }

    /// Actualiza o filtro com o estado actual do FDIR.
    pub fn update_fdir_state(&mut self, attack: bool, motor_loss: bool) {
        self.attack_active = attack;
        self.motor_loss_active = motor_loss;
    }

    /// Filtra o comando do piloto.
    /// Devolve o comando seguro + haptic feedback + eventual violação.
    pub fn filter(&self, pilot_cmd: PilotCommand, state: &DroneState) -> HaseResult {
        let mut violations = Vec::new();

        // ========== 1. Roll limit ==========
        let roll_risk = (pilot_cmd.roll.abs() / self.envelope.max_roll_rad).min(1.5);
        let safe_roll = if roll_risk > 1.0 {
            violations.push(EnvelopeViolation::RollLimit);
            pilot_cmd.roll.signum() * self.envelope.max_roll_rad
        } else {
            pilot_cmd.roll
        };

        // ========== 2. Pitch limit ==========
        let pitch_risk = (pilot_cmd.pitch.abs() / self.envelope.max_pitch_rad).min(1.5);
        let safe_pitch = if pitch_risk > 1.0 {
            violations.push(EnvelopeViolation::PitchLimit);
            pilot_cmd.pitch.signum() * self.envelope.max_pitch_rad
        } else {
            pilot_cmd.pitch
        };

        // ========== 3. Altitude prediction (2s à frente) ==========
        let predicted_alt = state.altitude + pilot_cmd.throttle * self.prediction_horizon_s * 2.0;
        let safe_throttle = if predicted_alt < self.envelope.min_altitude_m {
            violations.push(EnvelopeViolation::AltitudeLow);
            pilot_cmd.throttle.max(0.3) // forçar subida
        } else if predicted_alt > self.envelope.max_altitude_m {
            violations.push(EnvelopeViolation::AltitudeHigh);
            pilot_cmd.throttle.min(-0.3) // forçar descida
        } else {
            pilot_cmd.throttle
        };

        // ========== 4. Geofence prediction (2s à frente) ==========
        let predicted_pos = [
            state.position[0] + state.velocity[0] * self.prediction_horizon_s,
            state.position[1] + state.velocity[1] * self.prediction_horizon_s,
        ];
        let dx = predicted_pos[0] - self.envelope.geofence_center[0];
        let dy = predicted_pos[1] - self.envelope.geofence_center[1];
        let predicted_dist = sqrtf(dx * dx + dy * dy);

        if predicted_dist > self.envelope.geofence_radius_m * 0.95 {
            violations.push(EnvelopeViolation::GeofenceBreach);
            // Bloquear o comando horizontal (não podemos mudar a física, mas moderamos)
        }

        // ========== 5. Motor loss (contexto FDIR) ==========
        if self.motor_loss_active {
            violations.push(EnvelopeViolation::MotorLoss);
        }

        // ========== Haptic feedback ==========
        let haptic = HapticFeedback {
            roll_resistance: (roll_risk - 1.0).max(0.0).min(1.0),
            pitch_resistance: (pitch_risk - 1.0).max(0.0).min(1.0),
            throttle_resistance: if matches!(
                violations.last(),
                Some(EnvelopeViolation::AltitudeLow | EnvelopeViolation::AltitudeHigh)
            ) {
                0.5
            } else {
                0.0
            },
            vibration_pattern: if self.motor_loss_active {
                VibrationPattern::Pulse
            } else if !violations.is_empty() {
                if roll_risk > 1.3 || pitch_risk > 1.3 {
                    VibrationPattern::Strong
                } else {
                    VibrationPattern::Soft
                }
            } else {
                VibrationPattern::None
            },
        };

        HaseResult {
            safe_command: PilotCommand {
                roll: safe_roll,
                pitch: safe_pitch,
                yaw_rate: pilot_cmd.yaw_rate,
                throttle: safe_throttle,
            },
            haptic,
            violation: violations.into_iter().next(),
        }
    }
}

/// Regista uma violação no CERTIS (para safety case).
pub fn log_violation_to_certis(
    recorder: &mut CertisRecorder,
    t: f32,
    violation: &EnvelopeViolation,
) {
    let event = match violation {
        EnvelopeViolation::RollLimit | EnvelopeViolation::PitchLimit => {
            SafetyEventType::PilotResisted {
                reason: "attitude_limit".into(),
            }
        }
        EnvelopeViolation::AltitudeLow | EnvelopeViolation::AltitudeHigh => {
            SafetyEventType::PilotResisted {
                reason: "altitude_limit".into(),
            }
        }
        EnvelopeViolation::GeofenceBreach => SafetyEventType::PilotResisted {
            reason: "geofence".into(),
        },
        EnvelopeViolation::MotorLoss => SafetyEventType::MotorLossDetected { motor: 0 },
    };
    recorder.record(t, event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pilot_within_envelope_passes() {
        let hase = HaseFilter::new();
        let cmd = PilotCommand {
            roll: 0.1,
            pitch: 0.1,
            yaw_rate: 0.0,
            throttle: 0.0,
        };
        let state = DroneState {
            altitude: 10.0,
            ..Default::default()
        };
        let result = hase.filter(cmd, &state);
        assert!(result.violation.is_none());
        assert_eq!(result.haptic.vibration_pattern, VibrationPattern::None);
        assert!((result.safe_command.roll - 0.1).abs() < 1e-6);
    }

    #[test]
    fn roll_limit_clamped() {
        let hase = HaseFilter::new();
        let cmd = PilotCommand {
            roll: 1.5,
            pitch: 0.0,
            yaw_rate: 0.0,
            throttle: 0.0,
        };
        let state = DroneState {
            altitude: 10.0,
            ..Default::default()
        };
        let result = hase.filter(cmd, &state);
        assert!(result.violation.is_some());
        assert!((result.safe_command.roll - 0.5236).abs() < 0.01);
        assert!(result.haptic.roll_resistance > 0.0);
    }

    #[test]
    fn altitude_low_forces_climb() {
        let hase = HaseFilter::new();
        let cmd = PilotCommand {
            roll: 0.0,
            pitch: 0.0,
            yaw_rate: 0.0,
            throttle: -0.5,
        };
        let state = DroneState {
            altitude: 2.5,
            ..Default::default()
        };
        let result = hase.filter(cmd, &state);
        assert!(matches!(
            result.violation,
            Some(EnvelopeViolation::AltitudeLow)
        ));
        assert!(result.safe_command.throttle >= 0.3);
        assert_eq!(result.haptic.vibration_pattern, VibrationPattern::Soft);
    }

    #[test]
    fn geofence_breach_predicted() {
        let hase = HaseFilter::new();
        let cmd = PilotCommand::default();
        let state = DroneState {
            position: [490.0, 0.0],
            velocity: [10.0, 0.0], // a voar em direcção ao limite
            altitude: 10.0,
            ..Default::default()
        };
        let result = hase.filter(cmd, &state);
        assert!(matches!(
            result.violation,
            Some(EnvelopeViolation::GeofenceBreach)
        ));
    }

    #[test]
    fn motor_loss_triggers_pulse() {
        let mut hase = HaseFilter::new();
        hase.update_fdir_state(false, true);
        let cmd = PilotCommand::default();
        let state = DroneState {
            altitude: 10.0,
            ..Default::default()
        };
        let result = hase.filter(cmd, &state);
        assert!(matches!(
            result.violation,
            Some(EnvelopeViolation::MotorLoss)
        ));
        assert_eq!(result.haptic.vibration_pattern, VibrationPattern::Pulse);
    }

    #[test]
    fn strong_resistance_on_extreme_command() {
        let hase = HaseFilter::new();
        let cmd = PilotCommand {
            roll: 2.0,
            pitch: 0.0,
            yaw_rate: 0.0,
            throttle: 0.0,
        };
        let state = DroneState {
            altitude: 10.0,
            ..Default::default()
        };
        let result = hase.filter(cmd, &state);
        assert_eq!(result.haptic.vibration_pattern, VibrationPattern::Strong);
    }

    #[test]
    fn certis_logging_pilot_override() {
        let mut recorder = CertisRecorder::new("hase_test");
        let hase = HaseFilter::new();
        let cmd = PilotCommand {
            roll: 1.5,
            pitch: 0.0,
            yaw_rate: 0.0,
            throttle: 0.0,
        };
        let state = DroneState {
            altitude: 10.0,
            ..Default::default()
        };
        let result = hase.filter(cmd, &state);
        if let Some(violation) = &result.violation {
            log_violation_to_certis(&mut recorder, 10.0, violation);
        }
        assert!(!recorder.events.is_empty());
        assert!(matches!(
            recorder.events[0].kind,
            SafetyEventType::PilotResisted { .. }
        ));
        assert!(recorder.verify_chain());
    }

    #[test]
    fn hase_preserves_pilot_authority_when_safe() {
        let hase = HaseFilter::new();
        let cmd = PilotCommand {
            roll: 0.3,
            pitch: -0.2,
            yaw_rate: 0.5,
            throttle: 0.1,
        };
        let state = DroneState {
            altitude: 15.0,
            ..Default::default()
        };
        let result = hase.filter(cmd, &state);
        // Comando seguro -> passa intacto
        assert!((result.safe_command.roll - 0.3).abs() < 1e-6);
        assert!((result.safe_command.pitch - (-0.2)).abs() < 1e-6);
        assert!((result.safe_command.yaw_rate - 0.5).abs() < 1e-6);
    }
}
