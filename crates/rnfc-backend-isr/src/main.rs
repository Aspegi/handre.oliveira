//! ISR backend - patrol loop + camera/mic payload.
//! Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
#![deny(unsafe_code)]

use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::NavCore;
use rnfc_daa::trajectory::Trajectory;
use rnfc_daa::plan_astar;
use rnfc_surveillance::{Payload, SimulatedCamera, SimulatedMicrophone, SurveillanceContext};

fn to_deg(rad: f32) -> f32 { rad * 180.0 / core::f32::consts::PI }

fn main() {
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let mut controller = AttitudeController::new();

    let mission = vec![
        [100.0_f32, 50.0],
        [150.0, -30.0],
        [0.0, 0.0],
    ];
    let mut mission_idx = 0;

    let mut camera = SimulatedCamera::new((1920, 1080));
    let mut mic = SimulatedMicrophone::new();
    camera.init().expect("camera init");
    mic.init().expect("mic init");

    let grid = rnfc_daa::OccupancyGrid::new(-30.0, -50.0, 220, 150, 1.0);
    let plan = plan_astar(&grid, [0.0, 0.0], mission[0]).expect("A* falhou");
    let mut trajectory = Trajectory::from_waypoints(&plan, 0.5);

    println!("=== ISR PATROL ===");
    println!("Missao: A(100,50) -> B(150,-30) -> C(0,0) -> loop");
    println!("Payload: camera 1920x1080 + microfone (simulados)");
    println!();

    let max_speed = 15.0_f32;
    let max_pitch = 20.0_f32.to_radians();
    let arrival_radius = 4.0_f32;

    let mut roll: f32 = 0.0;
    let mut pitch: f32 = 0.0;
    let mut yaw: f32 = 0.0;
    let mut roll_rate: f32 = 0.0;
    let mut pitch_rate: f32 = 0.0;
    let mut yaw_rate: f32 = 0.0;

    let dt = 0.01_f32;
    let mut t = 0.0_f32;
    let mut current_speed = 0.0_f32;
    let mut patrol_laps = 0;

    // Loiter state
    let mut loiter_active = false;
    let mut loiter_timer: f32 = 0.0;

    for i in 0..=60000 {
        t += dt;

        let pos = nav.position();
        let pos2 = [pos[0], pos[1]];
        let goal = mission[mission_idx];
        let dist_to_goal = ((pos[0]-goal[0]).powi(2) + (pos[1]-goal[1]).powi(2)).sqrt();

        // Chegou ao waypoint? Iniciar loiter
        if dist_to_goal < arrival_radius && !loiter_active {
            loiter_active = true;
            loiter_timer = 0.0;
            current_speed = 0.0;
            println!("    *** LOITER: a pairar em WP {} para vigilancia ***", mission_idx);
        }

        if loiter_active {
            loiter_timer += dt;
            current_speed = 0.0;

            let ctx = SurveillanceContext {
                position: pos,
                velocity: [0.0; 3],
                yaw,
                time_s: t,
                hovering: true,
            };
            let _ = camera.capture(&ctx);
            let _ = mic.capture(&ctx);

            if loiter_timer >= 3.0 {
                println!("    -> WP {} vigiado {:.1}s (cam={}, mic={})",
                         mission_idx, loiter_timer, camera.frames_captured, mic.samples);
                mission_idx += 1;
                if mission_idx >= mission.len() {
                    patrol_laps += 1;
                    mission_idx = 0;
                    println!("    *** PATRULHA: volta {} completa, a reiniciar ***", patrol_laps);
                }
                let next_goal = mission[mission_idx];
                if let Some(new_plan) = plan_astar(&grid, pos2, next_goal) {
                    trajectory = Trajectory::from_waypoints(&new_plan, 0.5);
                }
                loiter_active = false;
                loiter_timer = 0.0;
            }
            continue;
        }

        // Pure Pursuit
        let (_, closest_d) = trajectory.find_closest(pos2);
        let dist_to_end = (trajectory.total_length - closest_d).max(0.0);
        let lookahead = if dist_to_end < 15.0 {
            (dist_to_end * 0.5).max(1.5)
        } else {
            4.0 + current_speed * 0.6
        };
        let target_d = (closest_d + lookahead).min(trajectory.total_length);
        let target = trajectory.point_at(target_d);

        let dn = target[0] - pos[0];
        let de = target[1] - pos[1];

        let sp_yaw = if dist_to_goal < 8.0 {
            let dg_n = goal[0] - pos[0];
            let dg_e = goal[1] - pos[1];
            dg_e.atan2(dg_n)
        } else {
            de.atan2(dn)
        };

        let v_end = if dist_to_goal < 20.0 {
            (dist_to_goal / 20.0 * max_speed).max(0.3)
        } else {
            max_speed
        };
        let v_target = v_end;
        let sp_pitch = (v_target / max_speed) * max_pitch;
        let setpoints = [0.0_f32, sp_pitch, sp_yaw];

        let measured = [roll, pitch, yaw];
        let cmd = controller.update(setpoints, measured, dt);

        let damping = 0.8_f32;
        roll_rate  += (cmd[0] - damping * roll_rate) * dt;
        pitch_rate += (cmd[1] - damping * pitch_rate) * dt;
        yaw_rate   += (cmd[2] - damping * yaw_rate) * dt;

        roll  += roll_rate  * dt;
        pitch += pitch_rate * dt;
        yaw   += yaw_rate   * dt;

        let accel_max = 10.0_f32;
        let desired = max_speed * (pitch.abs() / max_pitch).min(1.0);
        let dv = (desired - current_speed).clamp(-accel_max * dt, accel_max * dt);
        current_speed += dv;
        current_speed = current_speed.min(v_target).max(0.0);

        let vel_n = current_speed * yaw.cos();
        let vel_e = current_speed * yaw.sin();
        let new_pos = [pos[0] + vel_n * dt, pos[1] + vel_e * dt, 0.0];

        let gnss = Some(rnfc_abstractions::GnssFix {
            position: new_pos,
            velocity: [vel_n, vel_e, 0.0],
            hdop: 1.0,
        });
        let imu = rnfc_abstractions::ImuSample {
            accel: [0.0, 0.0, -9.81],
            gyro: [roll_rate, pitch_rate, yaw_rate],
            dt,
        };
        let _ = nav.step(&imu, gnss.as_ref());

        if i % 1000 == 0 && i > 0 {
            let p = nav.position();
            println!("{:>6.1}s  N={:7.1}  E={:7.1}  sp_yaw={:6.1}  wp={}  cam={}",
                     t, p[0], p[1], to_deg(sp_yaw), mission_idx, camera.frames_captured);
        }
    }

    println!("\n=== RESULTADO FINAL ===");
    println!("Voltas de patrulha completas: {}", patrol_laps);
    println!("Frames capturados: {}", camera.frames_captured);
    println!("Amostras de audio: {}", mic.samples);
}