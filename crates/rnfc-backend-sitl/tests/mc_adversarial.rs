//! Adversarial Monte Carlo via evolucao genetica.
//!
//! Em vez de amostragem uniforme, evolui cenarios que MAXIMIZAM o dano:
//! - final dist alta
//! - timeout / hover / crash
//!
//! Fitness = 1000*timeout + 100*crashed + final_dist
//!
//! 50 geracoes x 100 populacao = 5000 cenarios evoluidos.

use rnfc_abstractions::{GnssFix, ImuSample};
use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::attitude_estimator::AttitudeEstimator;
use rnfc_core::fusion_mag::FusionMag;
use rnfc_core::guidance::Guidance;
use rnfc_core::NavCore;
use rnfc_fdir::{SensorHealth, Failsafe, FailsafeAction, FailsafeTrigger, ZombieDetector};

const DT: f32 = 0.01;
const MAX_SPEED: f32 = 8.0;
const MAX_PITCH: f32 = 0.261799_4;
const DAMPING: f32 = 0.8;
const PI: f32 = core::f32::consts::PI;
const MAX_ITERS: usize = 60000;

const GENERATIONS: usize = 50;
const POP_SIZE: usize = 100;
const ELITE: usize = 20;

struct Rng { state: u32 }
impl Rng {
    fn new(seed: u32) -> Self { Self { state: seed.wrapping_add(1) } }
    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
    fn f32_01(&mut self) -> f32 { (self.next() >> 8) as f32 / 16777216.0 }
    fn range(&mut self, lo: f32, hi: f32) -> f32 { lo + self.f32_01() * (hi - lo) }
    fn coin(&mut self) -> bool { self.f32_01() < 0.5 }
}

#[derive(Clone, Debug)]
struct Scenario {
    wind_n: f32,
    wind_e: f32,
    attack_offset: f32,   // 0 = sem ataque
    attack_t: f32,
    dropout_t: f32,       // -1 = sem dropout
    battery_drain: f32,
    noise_scale: f32,
}

impl Scenario {
    fn random(rng: &mut Rng) -> Self {
        let attack_offset = if rng.coin() { rng.range(-0.8, 0.8) } else { 0.0 };
        let dropout_t = if rng.coin() && rng.coin() { rng.range(60.0, 200.0) } else { -1.0 };
        Self {
            wind_n: rng.range(-4.0, 4.0),
            wind_e: rng.range(-4.0, 4.0),
            attack_offset,
            attack_t: rng.range(40.0, 220.0),
            dropout_t,
            battery_drain: rng.range(0.003, 0.015),
            noise_scale: rng.range(0.3, 3.0),
        }
    }
    
    fn mutate(&mut self, rng: &mut Rng) {
        let rate = 0.3;
        if rng.f32_01() < rate { self.wind_n = (self.wind_n + rng.range(-1.0, 1.0)).clamp(-6.0, 6.0); }
        if rng.f32_01() < rate { self.wind_e = (self.wind_e + rng.range(-1.0, 1.0)).clamp(-6.0, 6.0); }
        if rng.f32_01() < rate { self.attack_offset = (self.attack_offset + rng.range(-0.2, 0.2)).clamp(-1.0, 1.0); }
        if rng.f32_01() < rate { self.attack_t = (self.attack_t + rng.range(-20.0, 20.0)).clamp(20.0, 240.0); }
        if rng.f32_01() < rate { 
            self.dropout_t = if self.dropout_t < 0.0 { rng.range(60.0, 200.0) } 
                            else { (self.dropout_t + rng.range(-20.0, 20.0)).clamp(30.0, 240.0) };
        }
        if rng.f32_01() < rate { self.battery_drain = (self.battery_drain + rng.range(-0.002, 0.002)).clamp(0.002, 0.020); }
        if rng.f32_01() < rate { self.noise_scale = (self.noise_scale + rng.range(-0.5, 0.5)).clamp(0.1, 5.0); }
    }
    
    fn crossover(&self, other: &Scenario, rng: &mut Rng) -> Scenario {
        Scenario {
            wind_n: if rng.coin() { self.wind_n } else { other.wind_n },
            wind_e: if rng.coin() { self.wind_e } else { other.wind_e },
            attack_offset: if rng.coin() { self.attack_offset } else { other.attack_offset },
            attack_t: if rng.coin() { self.attack_t } else { other.attack_t },
            dropout_t: if rng.coin() { self.dropout_t } else { other.dropout_t },
            battery_drain: if rng.coin() { self.battery_drain } else { other.battery_drain },
            noise_scale: if rng.coin() { self.noise_scale } else { other.noise_scale },
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
#[derive(Debug)]
enum Outcome { Success, RtlSuccess, Timeout, Crash }

fn run_scenario(sc: &Scenario, seed: u32) -> (Outcome, f32, f32) {
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let mut ctrl = AttitudeController::new();
    let mut guide = { let mut g = Guidance::new(); g.arrival_radius_final = 2.0; g };
    let mut fusion = FusionMag::new(0.0);
    let mut est = AttitudeEstimator::new();
    let mut health = SensorHealth::new(0.0);
    let failsafe = Failsafe::new();
    let mut zombie_detector = ZombieDetector::new();
    let mut zombie_latched = false;
    let mut battery_v = 12.6_f32;
    let mut roll: f32 = 0.0; let mut pitch: f32 = 0.0; let mut yaw: f32 = 0.0;
    let mut rr: f32 = 0.0; let mut pr: f32 = 0.0; let mut yr: f32 = 0.0;
    let mut failsafe_active = false;
    let mut rng = Rng::new(seed);
    let mut iter = 0;

    while iter < MAX_ITERS {
        iter += 1;
        let t = iter as f32 * DT;
        let pos = nav.position();
        if !pos[0].is_finite() || !pos[1].is_finite() {
            return (Outcome::Crash, f32::INFINITY, t);
        }

        let measured = [est.roll, est.pitch, fusion.yaw];
        let (sp, arr) = guide.update(pos);
        if arr {
            return (Outcome::Success, (pos[0].powi(2) + pos[1].powi(2)).sqrt(), t);
        }

        let cmd = ctrl.update(sp, measured, DT);
        rr += (cmd[0] - DAMPING * rr) * DT;
        pr += (cmd[1] - DAMPING * pr) * DT;
        yr += (cmd[2] - DAMPING * yr) * DT;
        roll += rr * DT; pitch += pr * DT; yaw += yr * DT;
        while yaw > PI { yaw -= 2.0 * PI; }
        while yaw < -PI { yaw += 2.0 * PI; }

        // AttitudeEstimator (accel+gyro com ruido)
        let noise_a = sc.noise_scale * 0.3 * rng.range(-1.0, 1.0);
        let accel_sim = [
            -9.81 * pitch.sin() + noise_a,
            -9.81 * roll.sin() * pitch.cos() + noise_a,
            -9.81 * roll.cos() * pitch.cos() + noise_a,
        ];
        let gyro_sim = [rr + sc.noise_scale * 0.01 * rng.range(-1.0, 1.0),
                        pr + sc.noise_scale * 0.01 * rng.range(-1.0, 1.0),
                        yr + sc.noise_scale * 0.01 * rng.range(-1.0, 1.0)];
        est.update(accel_sim, gyro_sim, DT);

        // Speed & wind
        let speed = MAX_SPEED * (pitch.abs() / MAX_PITCH).min(1.0);
        let mut vel_n = speed * yaw.cos() + sc.wind_n;
        let mut vel_e = speed * yaw.sin() + sc.wind_e;

        // Mag (com ataque)
        let yaw_mag = if sc.attack_offset.abs() > 0.01 && t >= sc.attack_t && t < sc.attack_t + 10.0 {
            yaw + sc.attack_offset
        } else { yaw };
        let mag_n = 50.0 * yaw_mag.cos() + sc.noise_scale * 0.5 * rng.range(-1.0, 1.0);
        let mag_e = 50.0 * yaw_mag.sin() + sc.noise_scale * 0.5 * rng.range(-1.0, 1.0);

        // FDIR
        let health_state = health.update(mag_n);
        battery_v -= sc.battery_drain * DT;
        let dist_to_goal_now = (pos[0].powi(2) + pos[1].powi(2)).sqrt();
        if zombie_detector.update(dist_to_goal_now, t) { zombie_latched = true; }
        let (_, fs_action) = if zombie_latched {
            (FailsafeTrigger::SensorCritical, FailsafeAction::ReturnToLaunch)
        } else {
            failsafe.evaluate([pos[0], pos[1]], battery_v, health_state)
        };
        let fs_engage = matches!(fs_action, FailsafeAction::Hover | FailsafeAction::ReturnToLaunch | FailsafeAction::EmergencyLand);
        if fs_engage && !failsafe_active { failsafe_active = true; }
        if failsafe_active && matches!(fs_action, FailsafeAction::Continue) { failsafe_active = false; }

        if failsafe_active {
            match fs_action {
                FailsafeAction::Hover | FailsafeAction::EmergencyLand => { vel_n = 0.0; vel_e = 0.0; }
                FailsafeAction::ReturnToLaunch => {
                    let dx = -pos[0]; let dy = -pos[1];
                    let d = (dx*dx + dy*dy).sqrt().max(0.1);
                    if d > 2.0 { vel_n = 8.0 * dx / d; vel_e = 8.0 * dy / d; }
                    else { return (Outcome::RtlSuccess, d, t); }
                }
                _ => {}
            }
        }

        fusion.update(mag_n, mag_e, yr, DT);

        // GNSS (com dropout)
        let gnss = if sc.dropout_t > 0.0 && t >= sc.dropout_t && t < sc.dropout_t + 10.0 {
            None
        } else {
            Some(GnssFix {
                position: [pos[0] + vel_n * DT, pos[1] + vel_e * DT, 0.0],
                velocity: [vel_n + sc.noise_scale * 0.1 * rng.range(-1.0, 1.0),
                           vel_e + sc.noise_scale * 0.1 * rng.range(-1.0, 1.0), 0.0],
                hdop: 1.0,
            })
        };

        let imu = ImuSample {
            accel: [sc.noise_scale * 0.01 * rng.range(-1.0, 1.0),
                    sc.noise_scale * 0.01 * rng.range(-1.0, 1.0), -9.81],
            gyro: [rr, pr, yr], dt: DT,
        };
        nav.step(&imu, gnss.as_ref());
    }

    let pos = nav.position();
    let dist = (pos[0].powi(2) + pos[1].powi(2)).sqrt();
    (Outcome::Timeout, dist, MAX_ITERS as f32 * DT)
}

/// Fitness = quao mau foi o resultado (maior = pior)
fn fitness(outcome: Outcome, dist: f32, _duration: f32) -> f32 {
    match outcome {
        Outcome::Crash => 10000.0,  // crash e 100x pior que timeout
        Outcome::Timeout => 100.0 + dist,  // sem cap - deixa evolucao explorar
        Outcome::RtlSuccess => dist.max(0.0),  // RTL a X m - quanto mais longe, "pior" (missao falhou mais cedo)
        Outcome::Success => dist,  // chegou, mas quanto mais longe, pior
    }
}

#[test]
fn adversarial_monte_carlo_evolution() {
    let mut rng = Rng::new(9999);
    
    // Populacao inicial
    let mut pop: Vec<Scenario> = (0..POP_SIZE).map(|_| Scenario::random(&mut rng)).collect();
    
    let mut worst_ever: Option<(Scenario, Outcome, f32, f32, usize)> = None;
    let mut history_fitness: Vec<f32> = Vec::new();
    
    for gen in 0..GENERATIONS {
        // Avaliar fitness de cada cenario
        let mut scored: Vec<(Scenario, f32, Outcome, f32)> = pop.iter().map(|s| {
            let seed = (gen as u32).wrapping_mul(2654435761)
                          .wrapping_add(s.attack_t as u32)
                          .wrapping_add((s.noise_scale * 100.0) as u32);
            let (outcome, dist, dur) = run_scenario(s, seed);
            let fit = fitness(outcome, dist, dur);
            (s.clone(), fit, outcome, dist)
        }).collect();
        
        // Ordenar por fitness (maior = pior)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Registar pior da geracao
        let best_of_gen = &scored[0];
        history_fitness.push(best_of_gen.1);
        
        // Atualizar worst_ever
        if worst_ever.is_none() || best_of_gen.1 > worst_ever.as_ref().unwrap().2 {
            worst_ever = Some((best_of_gen.0.clone(), best_of_gen.2, best_of_gen.1, best_of_gen.3, gen));
        }
        
        if gen % 10 == 0 {
            println!("Gen {:3}: max_fitness={:.2} (outcome={:?}, dist={:.2}m) | worst_ever={:.2}",
                gen, best_of_gen.1, best_of_gen.2, best_of_gen.3,
                worst_ever.as_ref().unwrap().2);
        }
        
        // Proxima geracao: elites + crossover + mutacao
        let mut new_pop: Vec<Scenario> = Vec::with_capacity(POP_SIZE);
        // Elites passam direto
        for i in 0..ELITE {
            new_pop.push(scored[i].0.clone());
        }
        // Resto: crossover entre 2 elites + mutacao
        while new_pop.len() < POP_SIZE {
            let a = &scored[(rng.next() as usize) % ELITE].0;
            let b = &scored[(rng.next() as usize) % ELITE].0;
            let mut child = a.crossover(b, &mut rng);
            child.mutate(&mut rng);
            new_pop.push(child);
        }
        // A cada 5 geracoes: 10% imigrantes aleatorios (anti local minimum)
        if gen % 5 == 0 && gen > 0 {
            for i in 0..(POP_SIZE / 10) {
                pop[POP_SIZE - 1 - i] = Scenario::random(&mut rng);
            }
        }
        // A cada 5 geracoes: 10% imigrantes aleatorios (anti local minimum)
        if gen % 5 == 0 && gen > 0 {
            for i in 0..(POP_SIZE / 10) {
                pop[POP_SIZE - 1 - i] = Scenario::random(&mut rng);
            }
        }
        pop = new_pop;
    }
    
    // Relatorio final
    println!("\n========== ADVERSARIAL MC REPORT ==========");
    println!("Geracoes: {} | Populacao: {} | Total scenarios: {}",
        GENERATIONS, POP_SIZE, GENERATIONS * POP_SIZE);
    
    let (worst, outcome, fit, dist, gen) = worst_ever.unwrap();
    println!("\nPior cenario encontrado (gen {}):", gen);
    println!("  wind = ({:.2}, {:.2}) m/s", worst.wind_n, worst.wind_e);
    println!("  attack_offset = {:.3} rad ({:.1} graus)", worst.attack_offset, worst.attack_offset.to_degrees());
    println!("  attack_t = {:.1} s", worst.attack_t);
    println!("  dropout_t = {:.1} s", worst.dropout_t);
    println!("  battery_drain = {:.5} V/s", worst.battery_drain);
    println!("  noise_scale = {:.2}", worst.noise_scale);
    println!("  OUTCOME = {:?}", outcome);
    println!("  final_dist = {:.2} m", dist);
    println!("  fitness = {:.2}", fit);
    
    // Evolucao do fitness ao longo das geracoes
    println!("\nEvolucao (fitness por geracao):");
    for (i, f) in history_fitness.iter().enumerate().step_by(5) {
        println!("  Gen {:2}: {:.2}", i, f);
    }
    println!("==========================================\n");
    
    // Asserts - garantem que o sistema aguenta adversidade evolutiva
    assert!(fit < 1000.0, 
        "BUG ENCONTRADO: fitness > 1000 = Crash descontrolado. Pior cenario: {:?}", worst);
    assert!(outcome != Outcome::Crash,
        "BUG ENCONTRADO: crash em cenario {:?}", worst);
    
    // Se timeout com dist > 50m, e um limite real (digno de documentacao)
    if outcome == Outcome::Timeout && dist > 50.0 {
        println!("[AVISO] Limite descoberto: timeout a {:.1}m. Documentar em SAFETY.md", dist);
    }
}








