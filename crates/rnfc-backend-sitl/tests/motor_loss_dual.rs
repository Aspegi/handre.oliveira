//! Dual-LNN Fault-Tolerant Control
//!
//! Pattern industrial: duas redes especializadas com switch.
//! - LNN_healthy: hover quando motor OK
//! - LNN_recovery: recuperacao quando motor falha
//!
//! Compara com:
//! - Single-LNN (16 hidden): hover OK, recovery crash (trade-off)
//! - Single-LNN (8 hidden): recovery OK, hover crash (trade-off)
//! - PID baseline: 0% survival

use rnfc_core::attitude_controller::AttitudeController;
use rnfc_lnn::LiquidNetwork;

const DT: f32 = 0.01;
const HOVER_THRUST: f32 = 9.81 / 4.0;
const MAX_THRUST: f32 = 5.0;
const ARM_LEN: f32 = 0.2;
const INERTIA: f32 = 0.05;
const MASS: f32 = 1.0;
const DAMPING: f32 = 0.8;
const CRASH_ROLL: f32 = 1.05;
const CRASH_ALT: f32 = 1.0;
const MAX_STEPS: usize = 2000;

const POP_SIZE: usize = 150;
const GENERATIONS: usize = 60;
const ELITE: usize = 15;
const MUTATION_RATE: f32 = 0.25;
const MUTATION_STD: f32 = 0.3;

type Net = LiquidNetwork<5, 16, 2>;

/// Duas redes especializadas com switch por m1_alive.
#[derive(Clone, Copy)]
struct DualLNN {
    healthy: Net,
    recovery: Net,
}

impl DualLNN {
    fn zeros() -> Self {
        Self { healthy: Net::zeros(), recovery: Net::zeros() }
    }

    fn reset(&mut self) {
        self.healthy.reset();
        self.recovery.reset();
    }

    fn forward_step(&mut self, input: [f32; 5], m1_alive: f32, dt: f32) -> [f32; 2] {
        if m1_alive > 0.5 {
            self.healthy.forward_step(input, dt)
        } else {
            self.recovery.forward_step(input, dt)
        }
    }

    fn to_vec(&self) -> Vec<f32> {
        let h = self.healthy.to_vec();
        let r = self.recovery.to_vec();
        let n = Net::param_count();
        let mut v = vec![0.0_f32; n * 2];
        v[..n].copy_from_slice(&h[..n]);
        v[n..].copy_from_slice(&r[..n]);
        v
    }

    fn from_vec(&mut self, v: &[f32]) {
        let n = Net::param_count();
        self.healthy.from_vec(&v[..n]);
        self.recovery.from_vec(&v[n..]);
    }
}

#[derive(Clone, Copy, Default)]
struct State {
    roll: f32,
    roll_rate: f32,
    altitude: f32,
    vertical_vel: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Outcome {
    Landed,
    Timeout,
    Crashed,
}

fn motor_mixing(thrust: f32, roll_diff: f32) -> [f32; 4] {
    [
        (thrust + roll_diff).clamp(0.0, MAX_THRUST),
        (thrust - roll_diff).clamp(0.0, MAX_THRUST),
        (thrust - roll_diff).clamp(0.0, MAX_THRUST),
        (thrust + roll_diff).clamp(0.0, MAX_THRUST),
    ]
}

fn simulate_dual(net: &mut DualLNN, fail_m1_at: Option<f32>) -> (State, Outcome, f32) {
    net.reset();
    let mut state = State { altitude: 10.0, ..Default::default() };

    for i in 0..MAX_STEPS {
        let t = i as f32 * DT;

        let m1_alive = match fail_m1_at {
            Some(tf) if t >= tf => 0.0,
            _ => 1.0,
        };

        let input = [
            (state.roll / CRASH_ROLL).clamp(-1.5, 1.5),
            (state.roll_rate / 8.0).clamp(-1.5, 1.5),
            (state.altitude / 10.0).clamp(0.0, 1.5),
            (state.vertical_vel / 10.0).clamp(-1.5, 1.5),
            m1_alive,
        ];

        let output = net.forward_step(input, m1_alive, DT);
        let thrust = ((output[0] + 0.5).clamp(0.0, 1.5)) * HOVER_THRUST;
        let roll_diff = output[1].clamp(-1.0, 1.0) * MAX_THRUST;

        let [mut m1, m2, m3, m4] = motor_mixing(thrust, roll_diff);
        if let Some(tf) = fail_m1_at {
            if t >= tf { m1 = 0.0; }
        }

        let total_thrust = m1 + m2 + m3 + m4;
        let roll_torque = ARM_LEN * (m1 + m4 - m2 - m3);
        let roll_accel = (roll_torque / INERTIA) - DAMPING * state.roll_rate;

        state.roll_rate += roll_accel * DT;
        state.roll += state.roll_rate * DT;

        let vert_thrust = total_thrust * state.roll.cos();
        let a_z = (vert_thrust - MASS * 9.81) / MASS;
        state.vertical_vel += a_z * DT;
        state.altitude += state.vertical_vel * DT;
        if state.altitude < 0.0 { state.altitude = 0.0; state.vertical_vel = 0.0; }

        if state.roll.abs() > CRASH_ROLL {
            return (state, Outcome::Crashed, t);
        }
        if state.altitude < CRASH_ALT {
            if state.roll.abs() < CRASH_ROLL * 0.5 && state.vertical_vel.abs() < 3.0 {
                return (state, Outcome::Landed, t);
            } else {
                return (state, Outcome::Crashed, t);
            }
        }
    }
    (state, Outcome::Timeout, 20.0)
}

fn simulate_pid(fail_m1_at: Option<f32>) -> (State, Outcome, f32) {
    let mut state = State { altitude: 10.0, ..Default::default() };
    let mut ctrl = AttitudeController::new();

    for i in 0..MAX_STEPS {
        let t = i as f32 * DT;
        let cmd = ctrl.update([0.0, 0.0, 0.0], [state.roll, 0.0, 0.0], DT);
        let roll_diff = cmd[0] * 0.0625;

        let [mut m1, m2, m3, m4] = motor_mixing(HOVER_THRUST, roll_diff);
        if let Some(tf) = fail_m1_at {
            if t >= tf { m1 = 0.0; }
        }

        let total_thrust = m1 + m2 + m3 + m4;
        let roll_torque = ARM_LEN * (m1 + m4 - m2 - m3);
        let roll_accel = (roll_torque / INERTIA) - DAMPING * state.roll_rate;
        state.roll_rate += roll_accel * DT;
        state.roll += state.roll_rate * DT;

        let vert_thrust = total_thrust * state.roll.cos();
        let a_z = (vert_thrust - MASS * 9.81) / MASS;
        state.vertical_vel += a_z * DT;
        state.altitude += state.vertical_vel * DT;
        if state.altitude < 0.0 { state.altitude = 0.0; state.vertical_vel = 0.0; }

        if state.roll.abs() > CRASH_ROLL {
            return (state, Outcome::Crashed, t);
        }
        if state.altitude < CRASH_ALT {
            if state.roll.abs() < CRASH_ROLL * 0.5 {
                return (state, Outcome::Landed, t);
            } else {
                return (state, Outcome::Crashed, t);
            }
        }
    }
    (state, Outcome::Timeout, 20.0)
}

fn fitness(net: &mut DualLNN) -> f32 {
    // HEALTHY: net.healthy deve fazer hover
    let (state_h, outcome_h, t_h) = simulate_dual(net, None);
    let hover_ok = matches!(outcome_h, Outcome::Timeout)
        && (state_h.altitude - 10.0).abs() < 3.0
        && state_h.roll.abs() < 0.5;

    let healthy_score = if hover_ok {
        1000.0
            - (state_h.altitude - 10.0).abs() * 50.0
            - state_h.roll.abs() * 200.0
    } else {
        match outcome_h {
            Outcome::Crashed => t_h * 0.1,   // GRADIENTE: mais tempo = melhor
            Outcome::Landed => -500.0,        // CHEAT: aterrou sem falha
            _ => 0.0,
        }
    };

    // RECOVERY: net.recovery deve aterrar
    let (state_f, outcome_f, t_f) = simulate_dual(net, Some(1.0));
    let recovery_score = match outcome_f {
        Outcome::Landed if t_f > 2.0 => {
            let roll_bonus = (1.0 - state_f.roll.abs() / CRASH_ROLL).max(0.0) * 300.0;
            let vel_bonus = (1.0 - state_f.vertical_vel.abs() / 5.0).max(0.0) * 200.0;
            500.0 + roll_bonus + vel_bonus  // max 1000 (match healthy)
        }
        Outcome::Landed => t_f,
        Outcome::Timeout => 200.0,
        Outcome::Crashed => t_f * 0.1,  // GRADIENTE: mais tempo = melhor
    };

    // MIN em vez de SUM: forca AMBAS as redes a serem boas.
    // Se uma falha, o sistema inteiro falha.
    // Isto e a formulacao safety-critical correcta.
    healthy_score.min(recovery_score)
}

struct Rng { state: u32 }
impl Rng {
    fn new(seed: u32) -> Self { Self { state: seed.wrapping_add(1) } }
    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
    fn f32(&mut self) -> f32 { (self.next() >> 8) as f32 / 16777216.0 }
    fn normal(&mut self) -> f32 {
        let u1 = self.f32().max(1e-6);
        let u2 = self.f32();
        (-2.0 * u1.ln()).sqrt() * (2.0 * core::f32::consts::PI * u2).cos()
    }
}

fn random_dual(rng: &mut Rng) -> DualLNN {
    let mut dual = DualLNN::zeros();
    let n = Net::param_count();
    let mut v = vec![0.0_f32; n * 2];
    for i in 0..(n * 2) {
        v[i] = rng.normal() * 0.1;
    }
    // tau entre 0.1 e 1.0 para ambas as redes
    for offset in [n - 16, 2 * n - 16] {
        for i in offset..(offset + 16) {
            v[i] = 0.1 + rng.f32() * 0.9;
        }
    }
    dual.from_vec(&v);
    dual
}

fn mutate(v: &mut [f32], rng: &mut Rng) {
    for i in 0..v.len() {
        if rng.f32() < MUTATION_RATE {
            v[i] += rng.normal() * MUTATION_STD;
        }
    }
}

fn crossover(a: &[f32], b: &[f32], rng: &mut Rng) -> Vec<f32> {
    let n = a.len();
    let mut child = vec![0.0_f32; n];
    for i in 0..n {
        child[i] = if rng.f32() < 0.5 { a[i] } else { b[i] };
    }
    child
}

#[test]
#[ignore = "experimento sum/min falhado - ver SAFETY.md L1.b"]
fn dual_lnn_hover_and_recovery() {
    let mut rng = Rng::new(42);
    let n_params = Net::param_count() * 2;
    println!("DualLNN: 2 x (5 -> 16 -> 2) = {} params total", n_params);
    println!("Populacao: {}, Geracoes: {}\n", POP_SIZE, GENERATIONS);

    let mut population: Vec<DualLNN> = (0..POP_SIZE)
        .map(|_| random_dual(&mut rng))
        .collect();

    let mut best_ever = f32::NEG_INFINITY;
    let mut best_net: Option<DualLNN> = None;

    for gen in 0..GENERATIONS {
        let mut scored: Vec<(usize, f32)> = population.iter_mut()
            .enumerate()
            .map(|(i, net)| (i, fitness(net)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let best_fit = scored[0].1;
        let avg_fit = scored.iter().map(|(_, f)| f).sum::<f32>() / POP_SIZE as f32;

        if best_fit > best_ever {
            best_ever = best_fit;
            best_net = Some(population[scored[0].0]);
        }

        if gen % 10 == 0 || gen == GENERATIONS - 1 {
            println!("Gen {:2}: best={:.1} avg={:.1}", gen, best_fit, avg_fit);
        }

        let mut next: Vec<DualLNN> = Vec::with_capacity(POP_SIZE);
        for i in 0..ELITE {
            next.push(population[scored[i].0]);
        }
        while next.len() < POP_SIZE {
            let p1 = population[scored[rng.next() as usize % ELITE].0];
            let p2 = population[scored[rng.next() as usize % ELITE].0];
            let v1 = p1.to_vec();
            let v2 = p2.to_vec();
            let mut child_v = crossover(&v1, &v2, &mut rng);
            mutate(&mut child_v, &mut rng);
            let mut child = DualLNN::zeros();
            child.from_vec(&child_v);
            next.push(child);
        }
        population = next;
    }

    println!("\n========== RESULTADO DUAL-LNN ==========");
    println!("Best fitness: {:.1}", best_ever);

    let mut best = best_net.expect("sem best");
    let (state_h, outcome_h, t_h) = simulate_dual(&mut best, None);
    println!("HEALTHY:  {:?} em {:.2}s (roll={:.1}° alt={:.2}m)",
        outcome_h, t_h, state_h.roll.to_degrees(), state_h.altitude);

    let (state_f, outcome_f, t_f) = simulate_dual(&mut best, Some(1.0));
    println!("MOTOR LOSS: {:?} em {:.2}s (roll={:.1}° alt={:.2}m)",
        outcome_f, t_f, state_f.roll.to_degrees(), state_f.altitude);

    let (_, pid_outcome, pid_t) = simulate_pid(Some(1.0));
    println!("PID baseline: {:?} em {:.2}s", pid_outcome, pid_t);

    // Asserts
    assert!(matches!(outcome_h, Outcome::Timeout),
        "Healthy: esperado Timeout (hover), obtido {:?}", outcome_h);
    assert!((state_h.altitude - 10.0).abs() < 3.0,
        "Healthy: altitude fora de range: {:.2}m", state_h.altitude);
    assert!(matches!(outcome_f, Outcome::Landed),
        "Motor loss: esperado Landed, obtido {:?} em {:.2}s", outcome_f, t_f);
    assert!(matches!(pid_outcome, Outcome::Crashed),
        "PID: esperado Crashed, obtido {:?}", pid_outcome);

    println!("\n[BREAKTHROUGH] DualLNN: hover OK + motor loss survived");
    println!("             PID: crashed em {:.2}s", pid_t);
}






