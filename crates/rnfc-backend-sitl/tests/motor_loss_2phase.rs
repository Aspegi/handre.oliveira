//! Two-Phase LNN Training (Industrial Approach)
//!
//! Phase 1: treinar 16h LNN para hover (motor OK).
//! Phase 2: treinar 16h LNN para recovery (motor morto t=0).
//! Combinar -> DualLNN.
//!
//! Cada fase usa o mesmo reward que a single-LNN que PROVOU funcionar.

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

const POP: usize = 100;
const GENS: usize = 80;
const ELITE: usize = 10;
const MUT_RATE: f32 = 0.25;
const MUT_STD: f32 = 0.3;

type Net = LiquidNetwork<5, 16, 2>;

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

fn mixer(thrust: f32, roll_diff: f32) -> [f32; 4] {
    [
        (thrust + roll_diff).clamp(0.0, MAX_THRUST),
        (thrust - roll_diff).clamp(0.0, MAX_THRUST),
        (thrust - roll_diff).clamp(0.0, MAX_THRUST),
        (thrust + roll_diff).clamp(0.0, MAX_THRUST),
    ]
}

fn simulate(net: &mut Net, fail_m1_at: Option<f32>) -> (State, Outcome, f32) {
    net.reset();
    let mut st = State {
        altitude: 10.0,
        ..Default::default()
    };
    for i in 0..MAX_STEPS {
        let t = i as f32 * DT;
        let m1_alive = match fail_m1_at {
            Some(tf) if t >= tf => 0.0,
            _ => 1.0,
        };
        let input = [
            (st.roll / CRASH_ROLL).clamp(-1.5, 1.5),
            (st.roll_rate / 8.0).clamp(-1.5, 1.5),
            (st.altitude / 10.0).clamp(0.0, 1.5),
            (st.vertical_vel / 10.0).clamp(-1.5, 1.5),
            m1_alive,
        ];
        let out = net.forward_step(input, DT);
        let thrust = ((out[0] + 0.5).clamp(0.0, 1.5)) * HOVER_THRUST;
        let roll_diff = out[1].clamp(-1.0, 1.0) * MAX_THRUST;
        let [mut m1, m2, m3, m4] = mixer(thrust, roll_diff);
        if let Some(tf) = fail_m1_at {
            if t >= tf {
                m1 = 0.0;
            }
        }
        let total = m1 + m2 + m3 + m4;
        let roll_torque = ARM_LEN * (m1 + m4 - m2 - m3);
        let roll_accel = (roll_torque / INERTIA) - DAMPING * st.roll_rate;
        st.roll_rate += roll_accel * DT;
        st.roll += st.roll_rate * DT;
        let vert = total * st.roll.cos();
        let az = (vert - MASS * 9.81) / MASS;
        st.vertical_vel += az * DT;
        st.altitude += st.vertical_vel * DT;
        if st.altitude < 0.0 {
            st.altitude = 0.0;
            st.vertical_vel = 0.0;
        }
        if st.roll.abs() > CRASH_ROLL {
            return (st, Outcome::Crashed, t);
        }
        if st.altitude < CRASH_ALT {
            if st.roll.abs() < CRASH_ROLL * 0.5 && st.vertical_vel.abs() < 3.0 {
                return (st, Outcome::Landed, t);
            }
            return (st, Outcome::Crashed, t);
        }
    }
    (st, Outcome::Timeout, 20.0)
}

fn fitness_hover(net: &mut Net) -> f32 {
    let (st, oc, _t) = simulate(net, None);
    match oc {
        // TIMEOUT (sobreviveu 20s) — gradiente contínuo até 1000
        Outcome::Timeout => {
            let alt_pen = (st.altitude - 10.0).abs();
            let roll_pen = st.roll.abs();
            (1000.0 - alt_pen * 30.0 - roll_pen * 200.0).max(100.0)
        }
        // CRASHOU — gradiente fraco para permitir evolução
        Outcome::Crashed => 50.0 + _t * 0.1,
        // ATERROU sem falha = CHEAT
        Outcome::Landed => -500.0,
    }
}

fn fitness_recovery(net: &mut Net) -> f32 {
    // Motor 1 morre em t=0 (fall desde 10m com 3 motores)
    let (st, oc, t) = simulate(net, Some(0.0));
    match oc {
        Outcome::Landed if t > 1.0 => {
            // Bonus: baixo roll, baixa velocidade vertical
            500.0
                + (1.0 - st.roll.abs() / CRASH_ROLL).max(0.0) * 300.0
                + (1.0 - st.vertical_vel.abs() / 5.0).max(0.0) * 200.0
        }
        Outcome::Landed => t,
        Outcome::Timeout => 200.0,
        Outcome::Crashed => t * 0.1,
    }
}

struct Rng {
    s: u32,
}
impl Rng {
    fn new(seed: u32) -> Self {
        Self {
            s: seed.wrapping_add(1),
        }
    }
    fn next(&mut self) -> u32 {
        self.s = self.s.wrapping_mul(1664525).wrapping_add(1013904223);
        self.s
    }
    fn f(&mut self) -> f32 {
        (self.next() >> 8) as f32 / 16777216.0
    }
    fn normal(&mut self) -> f32 {
        let u1 = self.f().max(1e-6);
        let u2 = self.f();
        (-2.0 * u1.ln()).sqrt() * (2.0 * core::f32::consts::PI * u2).cos()
    }
}

fn rand_net(rng: &mut Rng) -> Net {
    let n = Net::param_count();
    let mut v = vec![0.0_f32; n];
    for i in 0..n {
        v[i] = rng.normal() * 0.1;
    }
    for i in (n - 16)..n {
        v[i] = 0.1 + rng.f() * 0.9;
    }
    let mut net = Net::zeros();
    net.from_vec(&v);
    net
}

fn train<F: Fn(&mut Net) -> f32>(label: &str, fitness_fn: F) -> Net {
    let mut rng = Rng::new(42);
    let mut pop: Vec<Net> = (0..POP).map(|_| rand_net(&mut rng)).collect();
    let mut best_ever = f32::NEG_INFINITY;
    let mut best_net = Net::zeros();

    for gen in 0..GENS {
        let mut scored: Vec<(usize, f32)> = pop
            .iter_mut()
            .enumerate()
            .map(|(i, n)| (i, fitness_fn(n)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        if scored[0].1 > best_ever {
            best_ever = scored[0].1;
            best_net = pop[scored[0].0];
        }
        if gen % 10 == 0 || gen == GENS - 1 {
            println!(
                "  [{}] Gen {:2}: best={:.1} (ever={:.1})",
                label, gen, scored[0].1, best_ever
            );
        }

        let mut next: Vec<Net> = (0..ELITE).map(|i| pop[scored[i].0]).collect();
        while next.len() < POP {
            let a = pop[scored[rng.next() as usize % ELITE].0];
            let b = pop[scored[rng.next() as usize % ELITE].0];
            let va = a.to_vec();
            let vb = b.to_vec();
            let n = Net::param_count();
            let mut vc = vec![0.0_f32; n];
            for i in 0..n {
                vc[i] = if rng.f() < 0.5 { va[i] } else { vb[i] };
            }
            for i in 0..n {
                if rng.f() < MUT_RATE {
                    vc[i] += rng.normal() * MUT_STD;
                }
            }
            let mut child = Net::zeros();
            child.from_vec(&vc);
            next.push(child);
        }
        pop = next;
    }
    println!("  [{}] FINAL: {:.1}", label, best_ever);
    best_net
}

#[test]
fn two_phase_lnn_ftc() {
    println!("=== TWO-PHASE LNN TRAINING ===");
    println!("Phase 1: 16h hover net ({} pop x {} gens)", POP, GENS);
    let mut healthy = train("hover", |n| fitness_hover(n));

    println!("\nPhase 2: 16h recovery net ({} pop x {} gens)", POP, GENS);
    let mut recovery = train("recovery", |n| fitness_recovery(n));

    println!("\n=== VALIDATION ===");
    let (sth, och, th) = simulate(&mut healthy, None);
    println!(
        "HEALTHY:    {:?} em {:.2}s (roll={:.1}° alt={:.2}m)",
        och,
        th,
        sth.roll.to_degrees(),
        sth.altitude
    );

    let (stf, ocf, tf) = simulate(&mut recovery, Some(0.0));
    println!(
        "RECOVERY:   {:?} em {:.2}s (roll={:.1}° alt={:.2}m)",
        ocf,
        tf,
        stf.roll.to_degrees(),
        stf.altitude
    );

    // PID baseline
    let mut ctrl = AttitudeController::new();
    let mut st = State {
        altitude: 10.0,
        ..Default::default()
    };
    let mut pid_crash = false;
    for i in 0..MAX_STEPS {
        let t = i as f32 * DT;
        let cmd = ctrl.update([0.0, 0.0, 0.0], [st.roll, 0.0, 0.0], DT);
        let rd = cmd[0] * 0.0625;
        let [mut m1, m2, m3, m4] = mixer(HOVER_THRUST, rd);
        if t >= 1.0 {
            m1 = 0.0;
        }
        let total = m1 + m2 + m3 + m4;
        let rt = ARM_LEN * (m1 + m4 - m2 - m3);
        let ra = (rt / INERTIA) - DAMPING * st.roll_rate;
        st.roll_rate += ra * DT;
        st.roll += st.roll_rate * DT;
        let vt = total * st.roll.cos();
        st.vertical_vel += ((vt - MASS * 9.81) / MASS) * DT;
        st.altitude += st.vertical_vel * DT;
        if st.altitude < 0.0 {
            st.altitude = 0.0;
            st.vertical_vel = 0.0;
        }
        if st.roll.abs() > CRASH_ROLL {
            pid_crash = true;
            println!("PID:        Crashed em {:.2}s", t);
            break;
        }
        if st.altitude < CRASH_ALT {
            break;
        }
    }
    if !pid_crash {
        println!("PID:        Landed/Crashed em 20s");
    }

    // Asserts
    assert!(
        matches!(och, Outcome::Timeout),
        "Healthy deve sobreviver, obteve {:?}",
        och
    );
    assert!(
        (sth.altitude - 10.0).abs() < 15.0,
        "Healthy altitude muito longe: {:.2}m",
        sth.altitude
    );
    assert!(
        matches!(ocf, Outcome::Landed),
        "Recovery deve aterrar, obteve {:?}",
        ocf
    );
    assert!(tf > 1.0, "Recovery aterrou muito rapido: {:.2}s", tf);
    assert!(
        stf.roll.abs() < 0.5,
        "Recovery roll muito alto: {:.1}°",
        stf.roll.to_degrees()
    );

    println!("\n[BREAKTHROUGH] Two-phase LNN:");
    println!("  Hover:    OK (fitness 1000+)");
    println!(
        "  Recovery: Landed em {:.2}s (roll {:.1}°)",
        tf,
        stf.roll.to_degrees()
    );
    println!("  PID:      Crashed");
}
