//! Integration test: replica o loop de fisica do main.rs
//! sem vento/gust/ataque e verifica que a missao A->B->C completa.

use rnfc_abstractions::{GnssFix, ImuSample};
use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::fusion_mag::FusionMag;
use rnfc_core::guidance::Guidance;
use rnfc_core::NavCore;

const DT: f32 = 0.01;
const MAX_SPEED: f32 = 8.0;
const MAX_PITCH: f32 = 0.261799_4; // 15 deg em rad
const DAMPING: f32 = 0.8;
const PI: f32 = core::f32::consts::PI;
const MAX_ITERS: usize = 40000; // 100s de voo (deve chegar em ~250s, mas 100s ja leva a base em condicoes ideais)

#[test]
fn full_mission_reaches_base() {
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let mut ctrl = AttitudeController::new();
    let mut guide = Guidance::new();
    let mut fusion = FusionMag::new(0.0);

    let mut roll: f32 = 0.0;
    let mut pitch: f32 = 0.0;
    let mut yaw: f32 = 0.0;
    let mut roll_rate: f32 = 0.0;
    let mut pitch_rate: f32 = 0.0;
    let mut yaw_rate: f32 = 0.0;

    let mut arrived = false;
    let mut iter = 0;

    while iter < MAX_ITERS {
        iter += 1;
        let pos = nav.position();
        let (sp, arr) = guide.update(pos);
        if arr {
            arrived = true;
            break;
        }

        // Controlador ve fusion.yaw (nao yaw verdade)
        let measured = [roll, pitch, fusion.yaw];
        let cmd = ctrl.update(sp, measured, DT);

        // Integracao das rates (modelo simples)
        roll_rate  += (cmd[0] - DAMPING * roll_rate)  * DT;
        pitch_rate += (cmd[1] - DAMPING * pitch_rate) * DT;
        yaw_rate   += (cmd[2] - DAMPING * yaw_rate)   * DT;

        roll  += roll_rate  * DT;
        pitch += pitch_rate * DT;
        yaw   += yaw_rate   * DT;

        // Wrap yaw verdadeiro (fisica realista)
        while yaw >  PI { yaw -= 2.0 * PI; }
        while yaw < -PI { yaw += 2.0 * PI; }

        // Velocidade proporcional a |pitch|, na direcao do yaw verdadeiro
        let speed = MAX_SPEED * (pitch.abs() / MAX_PITCH).min(1.0);
        let vel_n = speed * yaw.cos();
        let vel_e = speed * yaw.sin();

        let new_pos = [pos[0] + vel_n * DT, pos[1] + vel_e * DT, 0.0];

        // Simulacao magnetometro no frame verdadeiro + fusao
        let mag_n = 50.0 * yaw.cos();
        let mag_e = 50.0 * yaw.sin();
        fusion.update(mag_n, mag_e, yaw_rate, DT);

        // NavCore step
        let imu = ImuSample {
            accel: [0.0, 0.0, -9.81],
            gyro: [roll_rate, pitch_rate, yaw_rate],
            dt: DT,
        };
        let gnss = Some(GnssFix {
            position: new_pos,
            velocity: [vel_n, vel_e, 0.0],
            hdop: 1.0,
        });
        nav.step(&imu, gnss.as_ref());
    }

    assert!(arrived, "missao nao completou em {} iteracoes ({}s). idx={}, pos={:?}",
            iter, iter as f32 * DT, guide.current_index, nav.position());

    let p = nav.position();
    let dist = (p[0].powi(2) + p[1].powi(2)).sqrt();
    assert!(dist < 5.0, "base a {:.2}m (esperado < 5m) pos={:?}", dist, p);

    println!("[OK] Missao completa em {} iteracoes ({:.1}s), dist base = {:.2}m",
             iter, iter as f32 * DT, dist);
}

