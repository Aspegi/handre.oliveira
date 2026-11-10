//! Monte Carlo suite: N runs com adversidades randomizadas.
//! Reporta taxa de sucesso + worst case.

use rnfc_abstractions::{GnssFix, ImuSample};
use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::fusion_mag::FusionMag;
use rnfc_core::guidance::Guidance;
use rnfc_core::NavCore;
use rnfc_fdir::{SensorHealth, Failsafe, FailsafeAction};

const DT: f32 = 0.01;
const MAX_SPEED: f32 = 8.0;
const MAX_PITCH: f32 = 0.261799_4;
const DAMPING: f32 = 0.8;
const PI: f32 = core::f32::consts::PI;
const MAX_ITERS: usize = 60000;

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
    attack_offset: Option<f32>,
    attack_t: f32,
    dropout_t: Option<f32>,
    battery_drain: f32,
    noise_scale: f32,
}

impl Scenario {
    fn random(rng: &mut Rng) -> Self {
        let attack_offset = if rng.coin() { Some(rng.range(-0.5, 0.5)) } else { None };
        let attack_t = rng.range(50.0, 200.0);
        let dropout_t = if rng.coin() && rng.coin() { Some(rng.range(80.0, 150.0)) } else { None };
        Self {
            wind_n: rng.range(-2.5, 2.5),
            wind_e: rng.range(-2.5, 2.5),
            attack_offset, attack_t, dropout_t,
            battery_drain: rng.range(0.005, 0.012),
            noise_scale: rng.range(0.5, 2.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Outcome { Success, RtlSuccess, Timeout }

struct SimResult { outcome: Outcome, dist: f32, duration_s: f32 }

fn run_scenario(sc: &Scenario, seed: u32) -> SimResult {
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let mut ctrl = AttitudeController::new();
    let mut guide = { let mut g = Guidance::new(); g.arrival_radius_final = 2.0; g };
    let mut fusion = FusionMag::new(0.0);
    let mut health = SensorHealth::new(0.0);
    let failsafe = Failsafe::new();
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
            return SimResult { outcome: Outcome::Timeout, dist: f32::INFINITY, duration_s: t };
        }

        let (sp, arr) = guide.update(pos);
        if arr {
            return SimResult { outcome: Outcome::Success, dist: (pos[0].powi(2) + pos[1].powi(2)).sqrt(), duration_s: t };
        }

        let measured = [roll, pitch, fusion.yaw];
        let cmd = ctrl.update(sp, measured, DT);
        rr += (cmd[0] - DAMPING * rr) * DT;
        pr += (cmd[1] - DAMPING * pr) * DT;
        yr += (cmd[2] - DAMPING * yr) * DT;
        roll += rr * DT; pitch += pr * DT; yaw += yr * DT;
        while yaw > PI { yaw -= 2.0 * PI; }
        while yaw < -PI { yaw += 2.0 * PI; }

        let speed = MAX_SPEED * (pitch.abs() / MAX_PITCH).min(1.0);
        let mut vel_n = speed * yaw.cos() + sc.wind_n;
        let mut vel_e = speed * yaw.sin() + sc.wind_e;

        let yaw_mag = if let Some(off) = sc.attack_offset {
            if t >= sc.attack_t && t < sc.attack_t + 10.0 { yaw + off } else { yaw }
        } else { yaw };

        let mag_n = 50.0 * yaw_mag.cos() + sc.noise_scale * 0.5 * rng.range(-1.0, 1.0);
        let mag_e = 50.0 * yaw_mag.sin() + sc.noise_scale * 0.5 * rng.range(-1.0, 1.0);

        let health_state = health.update(mag_n);
        battery_v -= sc.battery_drain * DT;
        let (_, fs_action) = failsafe.evaluate([pos[0], pos[1]], battery_v, health_state);

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
                    else { return SimResult { outcome: Outcome::RtlSuccess, dist: d, duration_s: t }; }
                }
                _ => {}
            }
        }

        fusion.update(mag_n, mag_e, yr, DT);

        let gnss = if let Some(dt0) = sc.dropout_t {
            if t >= dt0 && t < dt0 + 10.0 { None }
            else { Some(GnssFix { position: [pos[0] + vel_n * DT, pos[1] + vel_e * DT, 0.0],
                velocity: [vel_n + sc.noise_scale * 0.1 * rng.range(-1.0, 1.0), vel_e + sc.noise_scale * 0.1 * rng.range(-1.0, 1.0), 0.0], hdop: 1.0 }) }
        } else {
            Some(GnssFix { position: [pos[0] + vel_n * DT, pos[1] + vel_e * DT, 0.0],
                velocity: [vel_n + sc.noise_scale * 0.1 * rng.range(-1.0, 1.0), vel_e + sc.noise_scale * 0.1 * rng.range(-1.0, 1.0), 0.0], hdop: 1.0 })
        };

        let imu = ImuSample {
            accel: [sc.noise_scale * 0.01 * rng.range(-1.0, 1.0), sc.noise_scale * 0.01 * rng.range(-1.0, 1.0), -9.81],
            gyro: [rr, pr, yr], dt: DT,
        };
        nav.step(&imu, gnss.as_ref());
    }

    let pos = nav.position();
    SimResult { outcome: Outcome::Timeout, dist: (pos[0].powi(2) + pos[1].powi(2)).sqrt(), duration_s: MAX_ITERS as f32 * DT }
}

fn run_monte_carlo(n: usize, min_pct: f32) {
    let mut rng = Rng::new(42);
    let mut results = Vec::with_capacity(n);
    for run in 0..n {
        let sc = Scenario::random(&mut rng);
        let seed = (run as u32).wrapping_mul(2654435761).wrapping_add(1);
        let res = run_scenario(&sc, seed);
        results.push((sc, res));
    }

    let total = n;
    let success = results.iter().filter(|(_, r)| r.outcome == Outcome::Success).count();
    let rtl_ok = results.iter().filter(|(_, r)| r.outcome == Outcome::RtlSuccess).count();
    let timeout = results.iter().filter(|(_, r)| r.outcome == Outcome::Timeout).count();
    let total_ok = success + rtl_ok;
    let rate = total_ok as f32 / total as f32 * 100.0;

    let dists: Vec<f32> = results.iter().filter(|(_, r)| r.outcome != Outcome::Timeout).map(|(_, r)| r.dist).collect();
    let durs: Vec<f32> = results.iter().filter(|(_, r)| r.outcome != Outcome::Timeout).map(|(_, r)| r.duration_s).collect();

    println!("\n========== MONTE CARLO REPORT ({} runs) ==========", total);
    println!("  Success (arrived):     {:4}/{}  ({:.1}%)", success, total, success as f32 / total as f32 * 100.0);
    println!("  RTL success (battery): {:4}/{}  ({:.1}%)", rtl_ok, total, rtl_ok as f32 / total as f32 * 100.0);
    println!("  Timeout:               {:4}/{}  ({:.1}%)", timeout, total, timeout as f32 / total as f32 * 100.0);
    println!("  TOTAL OK:              {:4}/{}  ({:.1}%)", total_ok, total, rate);
    if !dists.is_empty() {
        let avg_d = dists.iter().sum::<f32>() / dists.len() as f32;
        let max_d = dists.iter().cloned().fold(0.0_f32, f32::max);
        println!("  Dist avg: {:.2}m, max: {:.2}m", avg_d, max_d);
    }
    if !durs.is_empty() {
        let avg_t = durs.iter().sum::<f32>() / durs.len() as f32;
        println!("  Duration avg: {:.1}s", avg_t);
    }
    if let Some((ws, wr)) = results.iter().filter(|(_, r)| r.outcome != Outcome::Timeout)
        .max_by(|(_, a), (_, b)| a.dist.partial_cmp(&b.dist).unwrap()) {
        println!("  Worst ok: dist={:.2}m wind=({:.1},{:.1}) attack={:?}@ {:.0}s dropout={:?} noise={:.1}",
            wr.dist, ws.wind_n, ws.wind_e, ws.attack_offset, ws.attack_t, ws.dropout_t, ws.noise_scale);
    }
    println!("====================================================\n");
    assert!(rate >= min_pct, "Taxa {:.1}% < minimo {:.1}%", rate, min_pct);
}

#[test]
fn monte_carlo_100_runs() { run_monte_carlo(100, 85.0); }

#[test]
#[ignore]
fn monte_carlo_1000_runs_full() { run_monte_carlo(1000, 85.0); }
