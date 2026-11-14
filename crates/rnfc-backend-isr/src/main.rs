//! ISR backend - patrol loop + camera/mic payload + MAVLink video.
//! Copyright (c) 2026 Andre Oliveira. Todos os direitos reservados.
#![deny(unsafe_code)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::useless_vec)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::approx_constant)]

use rnfc_abstractions::{GnssFix, ImuSample};
use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::NavCore;
use rnfc_daa::plan_astar;
use rnfc_daa::trajectory::Trajectory;
use rnfc_mavlink::MavlinkSender;
use rnfc_surveillance::{Payload, SimulatedCamera, SimulatedMicrophone, SurveillanceContext};

const IMG_W: u16 = 64;
const IMG_H: u16 = 48;
const CHUNK_SIZE: usize = 253;

fn to_deg(rad: f32) -> f32 {
    rad * 180.0 / core::f32::consts::PI
}

/// Gera uma "imagem" RGB sintetica (gradiente) de `w*h*3` bytes.
fn generate_rgb_frame(w: u16, h: u16, frame_id: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity((w as usize) * (h as usize) * 3);
    for y in 0..h {
        for x in 0..w {
            let r = ((x as u32 * 255) / w as u32) as u8;
            let g = ((y as u32 * 255) / h as u32) as u8;
            let b = ((frame_id % 256) as u32) as u8;
            buf.push(r);
            buf.push(g);
            buf.push(b);
        }
    }
    buf
}

fn main() {
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let mut controller = AttitudeController::new();
    let mut mav = MavlinkSender::new("127.0.0.1:14550", 1, 1).expect("MAVLink");

    let mission = vec![[100.0_f32, 50.0], [150.0, -30.0], [0.0, 0.0]];
    let mut mission_idx = 0;

    let mut camera = SimulatedCamera::new((IMG_W as u32, IMG_H as u32));
    let mut mic = SimulatedMicrophone::new();
    camera.init().expect("camera init");
    mic.init().expect("mic init");

    let grid = rnfc_daa::OccupancyGrid::new(-30.0, -50.0, 220, 150, 1.0);
    let plan = plan_astar(&grid, [0.0, 0.0], mission[0]).expect("A* falhou");
    let mut trajectory = Trajectory::from_waypoints(&plan, 0.5);

    println!("=== ISR PATROL + MAVLINK VIDEO ===");
    println!("Missao: A(100,50) -> B(150,-30) -> C(0,0) -> loop");
    println!(
        "Payload: camera simulada {}x{} RGB + microfone",
        IMG_W, IMG_H
    );
    println!("MAVLink: DATA_TRANSMISSION_HANDSHAKE + ENCAPSULATED_DATA em 127.0.0.1:14550");
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
    let mut loiter_active = false;
    let mut loiter_timer: f32 = 0.0;
    let mut frames_sent: u64 = 0;
    let mut last_frame_at = -1.0_f32;

    for i in 0..=60000 {
        t += dt;

        let pos = nav.position();
        let pos2 = [pos[0], pos[1]];
        let goal = mission[mission_idx];
        let dist_to_goal = ((pos[0] - goal[0]).powi(2) + (pos[1] - goal[1]).powi(2)).sqrt();

        if dist_to_goal < arrival_radius && !loiter_active {
            loiter_active = true;
            loiter_timer = 0.0;
            current_speed = 0.0;
            println!("    *** LOITER: WP {} ***", mission_idx);
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
            if camera.capture(&ctx).is_ok() {
                let _ = mic.capture(&ctx);
            }

            // Envia 1 frame por segundo (nao repetir no mesmo segundo)
            let sec = loiter_timer.floor() as i32;
            if sec as f32 > last_frame_at {
                last_frame_at = sec as f32;
                let rgb = generate_rgb_frame(IMG_W, IMG_H, frames_sent);
                let size = rgb.len() as u32;
                let packets = ((rgb.len() + CHUNK_SIZE - 1) / CHUNK_SIZE) as u16;
                let _ = mav.send_image_handshake(IMG_W, IMG_H, size, packets, CHUNK_SIZE as u8);
                for (idx, chunk) in rgb.chunks(CHUNK_SIZE).enumerate() {
                    let _ = mav.send_image_chunk(idx as u16, chunk);
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                frames_sent += 1;
                println!(
                    "    [MAV] frame {} enviado ({} bytes, {} chunks)",
                    frames_sent, size, packets
                );
            }

            if loiter_timer >= 3.0 {
                println!(
                    "    -> WP {} vigiado 3.0s (frames enviados: {})",
                    mission_idx, frames_sent
                );
                mission_idx += 1;
                if mission_idx >= mission.len() {
                    patrol_laps += 1;
                    mission_idx = 0;
                    println!("    *** PATRULHA: volta {} completa ***", patrol_laps);
                }
                if let Some(np) = plan_astar(&grid, pos2, mission[mission_idx]) {
                    trajectory = Trajectory::from_waypoints(&np, 0.5);
                }
                loiter_active = false;
                loiter_timer = 0.0;
                last_frame_at = -1.0;
            }
            continue;
        }

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
        let sp_pitch = (v_end / max_speed) * max_pitch;
        let setpoints = [0.0_f32, sp_pitch, sp_yaw];
        let cmd = controller.update(setpoints, [roll, pitch, yaw], dt);

        let damping = 0.8_f32;
        roll_rate += (cmd[0] - damping * roll_rate) * dt;
        pitch_rate += (cmd[1] - damping * pitch_rate) * dt;
        yaw_rate += (cmd[2] - damping * yaw_rate) * dt;
        roll += roll_rate * dt;
        pitch += pitch_rate * dt;
        yaw += yaw_rate * dt;

        let accel_max = 10.0_f32;
        let desired = max_speed * (pitch.abs() / max_pitch).min(1.0);
        let dv = (desired - current_speed).clamp(-accel_max * dt, accel_max * dt);
        current_speed += dv;
        current_speed = current_speed.min(v_end).max(0.0);

        let vel_n = current_speed * yaw.cos();
        let vel_e = current_speed * yaw.sin();
        let new_pos = [pos[0] + vel_n * dt, pos[1] + vel_e * dt, 0.0];

        let gnss = Some(GnssFix {
            position: new_pos,
            velocity: [vel_n, vel_e, 0.0],
            hdop: 1.0,
        });
        let imu = ImuSample {
            accel: [0.0, 0.0, -9.81],
            gyro: [roll_rate, pitch_rate, yaw_rate],
            dt,
        };
        let _ = nav.step(&imu, gnss.as_ref());

        if i % 1000 == 0 && i > 0 {
            println!(
                "{:>6.1}s N={:7.1} E={:7.1} sp_yaw={:6.1} wp={} frames={}",
                t,
                pos[0],
                pos[1],
                to_deg(sp_yaw),
                mission_idx,
                frames_sent
            );
        }
    }

    println!("\n=== RESULTADO FINAL ===");
    println!("Voltas de patrulha: {}", patrol_laps);
    println!("Frames MAVLink enviados: {}", frames_sent);
    println!("Chunks totais: {}", mav.stats_image_chunks);
    println!("Erros: {}", mav.stats_errors);
}
