//! Motor Loss Baseline — Prova que o PID falha catastroficamente.
//!
//! Este teste documenta o GAP que a LNN-FTC vai fechar.
//!
//! Modelo: quadrotor X-config, 4 motores, fisica 2D (roll + altitude).
//! Controlador: PID 3-eixos do rnfc-core (apenas eixo roll activo).
//!
//! Teste 1: Motores saudaveis -> hover estavel (PID funciona)
//! Teste 2: Motor 1 falha em t=5s -> drone capota e cai (PID falha)

use rnfc_core::attitude_controller::AttitudeController;

const DT: f32 = 0.01;
const HOVER_THRUST_PER_MOTOR: f32 = 9.81 / 4.0; // 2.45 N
const MAX_THRUST_PER_MOTOR: f32 = 5.0; // ~2x hover
const ARM_LENGTH: f32 = 0.2; // m
const INERTIA_XX: f32 = 0.05; // kg·m²
const MASS: f32 = 1.0; // kg
const DAMPING: f32 = 0.8; // matches SITL
const CRASH_ROLL_RAD: f32 = 1.05; // 60 deg
const CRASH_ALT_M: f32 = 1.0;

#[derive(Clone, Copy, Default)]
struct State {
    roll: f32,
    roll_rate: f32,
    altitude: f32,
    vertical_vel: f32,
}

/// Mixer X-config (PX4), roll apenas (pitch=yaw=0).
/// roll_cmd positivo -> aumento motores 1&4 (direita), reducao 2&3 (esquerda).
fn motor_mixing(thrust: f32, roll_cmd: f32) -> [f32; 4] {
    let m1 = (thrust + roll_cmd).clamp(0.0, MAX_THRUST_PER_MOTOR);
    let m2 = (thrust - roll_cmd).clamp(0.0, MAX_THRUST_PER_MOTOR);
    let m3 = (thrust - roll_cmd).clamp(0.0, MAX_THRUST_PER_MOTOR);
    let m4 = (thrust + roll_cmd).clamp(0.0, MAX_THRUST_PER_MOTOR);
    [m1, m2, m3, m4]
}

fn simulate(fail_motor1_at: Option<f32>, verbose: bool) -> (State, bool, f32) {
    let mut state = State {
        altitude: 10.0,
        ..Default::default()
    };
    let mut ctrl = AttitudeController::new();

    for i in 0..1000 {
        let t = i as f32 * DT;

        // Attitude setpoint: nivelado (roll=0)
        let setpoint = [0.0, 0.0, 0.0];
        let measured = [state.roll, 0.0, 0.0];
        let cmd = ctrl.update(setpoint, measured, DT);

        // Escalar cmd[0] (rad/s²) para thrust diferencial
        // roll_accel = 16 * roll_diff (d=0.2, I=0.05)
        // Para roll_accel = cmd[0]: roll_diff = cmd[0] / 16 = cmd[0] * 0.0625
        let roll_thrust_diff = cmd[0] * 0.0625;

        let [mut m1, m2, m3, m4] = motor_mixing(HOVER_THRUST_PER_MOTOR, roll_thrust_diff);

        // Falha do motor 1
        if let Some(t_fail) = fail_motor1_at {
            if t >= t_fail {
                m1 = 0.0;
            }
        }

        // Fisica
        let total_thrust = m1 + m2 + m3 + m4;
        let roll_torque = ARM_LENGTH * (m1 + m4 - m2 - m3);

        // Dinamica roll com damping (match SITL)
        let roll_accel = (roll_torque / INERTIA_XX) - DAMPING * state.roll_rate;
        state.roll_rate += roll_accel * DT;
        state.roll += state.roll_rate * DT;

        // Dinamica vertical (projeccao do thrust com tilt)
        let vertical_thrust = total_thrust * state.roll.cos();
        let a_z = (vertical_thrust - MASS * 9.81) / MASS;
        state.vertical_vel += a_z * DT;
        state.altitude += state.vertical_vel * DT;

        if state.altitude < 0.0 {
            state.altitude = 0.0;
            state.vertical_vel = 0.0;
        }

        if verbose && fail_motor1_at.is_some() && t < 8.0 && i % 25 == 0 {
            println!(
                "t={:.2}s | roll={:+.1}° | rate={:+.0}°/s | alt={:.2}m | motors=[{:.1},{:.1},{:.1},{:.1}]",
                t,
                state.roll.to_degrees(),
                state.roll_rate.to_degrees(),
                state.altitude,
                m1, m2, m3, m4
            );
        }

        if state.roll.abs() > CRASH_ROLL_RAD || state.altitude < CRASH_ALT_M {
            return (state, true, t);
        }
    }

    (state, false, -1.0)
}

#[test]
fn pid_healthy_hover_stable() {
    let (state, crashed, _) = simulate(None, false);
    assert!(
        !crashed,
        "PID deveria manter hover sem falha: roll={:.2}° alt={:.2}m",
        state.roll.to_degrees(),
        state.altitude
    );
    assert!(
        state.roll.abs() < 0.1,
        "roll deveria ficar perto de 0: {:.2}°",
        state.roll.to_degrees()
    );
    assert!(
        state.altitude > 9.0,
        "altitude deveria ficar perto de 10m: {:.2}m",
        state.altitude
    );
    println!(
        "[HEALTHY] PID estabiliza a roll={:.2}° alt={:.2}m",
        state.roll.to_degrees(),
        state.altitude
    );
}

#[test]
fn pid_crashes_with_motor_loss_baseline() {
    println!("\n=== BASELINE: PID com falha do motor 1 em t=5s ===");
    let (state, crashed, crash_t) = simulate(Some(5.0), true);
    println!(
        "=== FIM: crashed={} em t={:.2}s roll={:.1}° alt={:.2}m ===\n",
        crashed,
        crash_t,
        state.roll.to_degrees(),
        state.altitude
    );
    assert!(
        crashed,
        "ESPERADO: PID DEVE falhar com motor loss (este e o GAP que LNN-FTC vai fechar). \
         Apos 10s: roll={:.1}° alt={:.2}m",
        state.roll.to_degrees(),
        state.altitude
    );
}
