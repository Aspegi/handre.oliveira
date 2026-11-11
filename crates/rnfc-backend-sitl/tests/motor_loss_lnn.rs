//! Motor Loss FTC via LNN Evolution
//!
//! Fase 2b: evoluir 100 LNNs x 30 geracoes para sobreviver a motor loss.
//! Compara com PID baseline (0.5s de sobrevivencia).
//!
//! Arquitectura LNN: 5 inputs -> 8 hidden -> 2 outputs = 138 params.
//! Fitness = healthy_hover + motor_loss_survival.

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
const MAX_STEPS: usize = 2000;  // 20s

const POP_SIZE: usize = 150;
const GENERATIONS: usize = 60;
const ELITE: usize = 15;
const MUTATION_RATE: f32 = 0.25;
const MUTATION_STD: f32 = 0.3;

type Net = LiquidNetwork<5, 32, 2>;

#[derive(Clone, Copy, Default)]
struct State {
    roll: f32,
    roll_rate: f32,
    altitude: f32,
    vertical_vel: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Outcome {
    Landed,     // descida controlada
    Timeout,    // sobreviveu 20s sem aterrar
    Crashed,    // roll > 60 graus
}

fn motor_mixing(thrust: f32, roll_diff: f32) -> [f32; 4] {
    [
        (thrust + roll_diff).clamp(0.0, MAX_THRUST),
        (thrust - roll_diff).clamp(0.0, MAX_THRUST),
        (thrust - roll_diff).clamp(0.0, MAX_THRUST),
        (thrust + roll_diff).clamp(0.0, MAX_THRUST),
    ]
}

fn simulate_lnn(net: &mut Net, fail_m1_at: Option<f32>) -> (State, Outcome, f32) {
    net.reset();
    let mut state = State { altitude: 10.0, ..Default::default() };

    for i in 0..MAX_STEPS {
        let t = i as f32 * DT;

        // LNN input (normalizado)
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

        let output = net.forward_step(input, DT);

        // Decode: [thrust_mult, roll_diff_mult]
        let thrust = ((output[0] + 0.5).clamp(0.0, 1.5)) * HOVER_THRUST;
        let roll_diff = output[1].clamp(-1.0, 1.0) * MAX_THRUST;

        let [mut m1, m2, m3, m4] = motor_mixing(thrust, roll_diff);
        if let Some(tf) = fail_m1_at {
            if t >= tf { m1 = 0.0; }
        }

        // Physics
        let total_thrust = m1 + m2 + m3 + m4;
        let roll_torque = ARM_LEN * (m1 + m4 - m2 - m3);
        let roll_accel = (roll_torque / INERTIA) - DAMPING * state.roll_rate;

        state.roll_rate += roll_accel * DT;
        state.roll += state.roll_rate * DT;
        while state.roll > core::f32::consts::PI { state.roll -= 2.0 * core::f32::consts::PI; }
        while state.roll < -core::f32::consts::PI { state.roll += 2.0 * core::f32::consts::PI; }

        let vert_thrust = total_thrust * state.roll.cos();
        let a_z = (vert_thrust - MASS * 9.81) / MASS;
        state.vertical_vel += a_z * DT;
        state.altitude += state.vertical_vel * DT;
        if state.altitude < 0.0 { state.altitude = 0.0; state.vertical_vel = 0.0; }

        // Classify
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

fn simulate_pid_baseline(fail_m1_at: Option<f32>) -> (State, Outcome, f32) {
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

fn fitness(net: &mut Net) -> f32 {
    // ========== FASE 1: HARD GATE ==========
    // Sem hover saudavel, a LNN e' INUTIL.
    // Fitness cai para ~zero, independentemente de motor loss.
    let (state_h, outcome_h, t_h) = simulate_lnn(net, None);
    let hover_ok = matches!(outcome_h, Outcome::Timeout)
        && (state_h.altitude - 10.0).abs() < 3.0
        && state_h.roll.abs() < 0.5;

    if !hover_ok {
        return match outcome_h {
            Outcome::Crashed => t_h * 0.1,   // max 2.0
            Outcome::Landed => -500.0,        // CHEAT: aterrou sem falha
            _ => 0.0,
        };
    }

    // ========== FASE 2: HOVER REWARD ==========
    // Hover vale ~1000 (dominante sobre qualquer bonus de recovery)
    let healthy_reward = 1000.0
        - (state_h.altitude - 10.0).abs() * 50.0
        - state_h.roll.abs() * 200.0;

    // ========== FASE 3: MOTOR LOSS BONUS ==========
    // Recovery vale ate ~500, MENOS que hover (forca coexistencia)
    let (state_f, outcome_f, t_f) = simulate_lnn(net, Some(1.0));
    let failed_bonus = match outcome_f {
        Outcome::Landed if t_f > 2.0 => {
            let roll_bonus = (1.0 - state_f.roll.abs() / CRASH_ROLL).max(0.0) * 200.0;
            let vel_bonus = (1.0 - state_f.vertical_vel.abs() / 5.0).max(0.0) * 100.0;
            300.0 + roll_bonus + vel_bonus
        }
        Outcome::Landed => t_f,
        Outcome::Timeout => 200.0,
        Outcome::Crashed => 0.0,         // sem bonus, mas mantem hover
    };

    healthy_reward + failed_bonus
}

// ============ Genetic Algorithm ============
struct Rng { state: u32 }
impl Rng {
    fn new(seed: u32) -> Self { Self { state: seed.wrapping_add(1) } }
    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
    fn f32(&mut self) -> f32 { (self.next() >> 8) as f32 / 16777216.0 }
    fn normal(&mut self) -> f32 {
        // Box-Muller simplificado
        let u1 = self.f32().max(1e-6);
        let u2 = self.f32();
        (-2.0 * u1.ln()).sqrt() * (2.0 * core::f32::consts::PI * u2).cos()
    }
}

fn random_net(rng: &mut Rng) -> Net {
    let mut net = Net::zeros();
    let v_len = Net::param_count();
    let mut v = vec![0.0_f32; v_len];
    for i in 0..v_len {
        v[i] = rng.normal() * 0.1;
    }
    // tau entre 0.1 e 1.0
    let tau_start = v_len - 8;
    for i in tau_start..v_len {
        v[i] = 0.1 + rng.f32() * 0.9;
    }
    net.from_vec(&v);
    net
}

fn mutate(v: &mut [f32], rng: &mut Rng) {
    let n = v.len();
    for i in 0..n {
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
fn lnn_evolution_survives_motor_loss() {
    let mut rng = Rng::new(42);
    let n_params = Net::param_count();
    println!("LNN architecture: 5 -> 8 -> 2 ({} parametros)", n_params);
    println!("Populacao: {}, Geracoes: {}, Total: {} simulacoes\n",
        POP_SIZE, GENERATIONS, POP_SIZE * GENERATIONS);

    // Populacao inicial
    let mut population: Vec<Net> = (0..POP_SIZE)
        .map(|_| random_net(&mut rng))
        .collect();

    let mut best_fitness_ever = 0.0_f32;
    let mut best_net_ever: Option<Net> = None;

    for gen in 0..GENERATIONS {
        // Avaliar
        let mut scored: Vec<(usize, f32)> = population.iter_mut()
            .enumerate()
            .map(|(i, net)| (i, fitness(net)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let best_fit = scored[0].1;
        let avg_fit = scored.iter().map(|(_, f)| f).sum::<f32>() / POP_SIZE as f32;

        if best_fit > best_fitness_ever {
            best_fitness_ever = best_fit;
            best_net_ever = Some(population[scored[0].0].clone());
        }

        if gen % 5 == 0 || gen == GENERATIONS - 1 {
            println!("Gen {:2}: best={:.1} avg={:.1}", gen, best_fit, avg_fit);
        }

        // Proxima geracao
        let mut next_pop: Vec<Net> = Vec::with_capacity(POP_SIZE);

        // Elites
        for i in 0..ELITE {
            next_pop.push(population[scored[i].0].clone());
        }

        // Crossover + mutacao
        while next_pop.len() < POP_SIZE {
            let p1 = &population[scored[rng.next() as usize % ELITE].0];
            let p2 = &population[scored[rng.next() as usize % ELITE].0];
            let v1 = p1.to_vec();
            let v2 = p2.to_vec();
            let mut child_v = crossover(&v1[..n_params], &v2[..n_params], &mut rng);
            mutate(&mut child_v, &mut rng);
            let mut child = Net::zeros();
            child.from_vec(&child_v);
            next_pop.push(child);
        }

        population = next_pop;
    }

    println!("\n========== RESULTADO ==========");
    println!("Best fitness ever: {:.1}", best_fitness_ever);

    // Testar o melhor LNN
    if let Some(mut best) = best_net_ever {
        let (state_h, out_h, t_h) = simulate_lnn(&mut best, None);
        println!("HEALTHY: {:?} em {:.2}s (roll={:.1}° alt={:.2}m)",
            out_h, t_h, state_h.roll.to_degrees(), state_h.altitude);

        let (state_f, out_f, t_f) = simulate_lnn(&mut best, Some(1.0));  // sincronizado
        println!("MOTOR LOSS: {:?} em {:.2}s (roll={:.1}° alt={:.2}m)",
            out_f, t_f, state_f.roll.to_degrees(), state_f.altitude);

        // PID baseline
        let (_, pid_f, pid_t) = simulate_pid_baseline(Some(5.0));
        println!("PID BASELINE (motor loss): {:?} em {:.2}s", pid_f, pid_t);

        // Asserts
        assert!(best_fitness_ever > 500.0,
            "LNN nao superou PID (fitness={:.1} < 5.0)", best_fitness_ever);
        
        // Comparar OUTCOMES (nao tempos)
        let pid_crashed = matches!(pid_f, Outcome::Crashed);
        let lnn_survived = matches!(out_f, Outcome::Landed | Outcome::Timeout);
        
        assert!(pid_crashed, "PID deveria crashar (baseline). Outcome={:?}", pid_f);
        assert!(lnn_survived,
            "LNN deveria sobreviver (Landed ou Timeout). Outcome={:?}, t={:.2}s", out_f, t_f);
        
        // Verificar que a LNN tambem faz hover saudavel (nao aterra cheat)
        assert!(matches!(out_h, Outcome::Timeout),
            "LNN nao faz hover saudavel (Outcome={:?}, t={:.2}s)", out_h, t_h);
        assert!((state_h.altitude - 10.0).abs() < 3.0,
            "LNN healthy altitude fora de range: {:.2}m", state_h.altitude);

        println!("\n[OK] LNN sobrevive {:.1}x mais que PID", t_f / pid_t.max(0.1));
    } else {
        panic!("Nenhuma LNN encontrada");
    }
}






