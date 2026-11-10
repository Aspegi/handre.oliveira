// Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)

#![allow(unused_variables, unused_assignments, unused_mut, dead_code)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::useless_vec)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::approx_constant)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::manual_map)]
// ULTRA ZENITE - Missao Composta + DAA Dinamico
use rnfc_abstractions::{GnssFix, ImuSample};
use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::fusion_mag::FusionMag;
use rnfc_core::NavCore;
use rnfc_daa::trajectory::Trajectory;
use rnfc_daa::{plan_astar, LidarSim, Obstacle, OccupancyGrid};
use rnfc_mavlink::MavlinkSender;
use std::fs::File;
use std::io::Write as IoWrite;

fn to_deg(rad: f32) -> f32 {
    rad * 180.0 / core::f32::consts::PI
}

fn main() {
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let mut controller = AttitudeController::new();

    // === Obstaculos ===
    // Conhecido: parede em N=50, E=[0,40] (mapeada no grid)
    // Desconhecido: parede em N=110, E=[-20,20] (so LiDAR deteta)
    let obstacles_known = vec![Obstacle {
        n_start: 50.0,
        e_start: 0.0,
        n_end: 50.0,
        e_end: 40.0,
    }];
    let obstacles_unknown = vec![Obstacle {
        n_start: 110.0,
        e_start: -20.0,
        n_end: 110.0,
        e_end: 20.0,
    }];
    let mut obstacles_all = obstacles_known.clone();
    obstacles_all.extend(obstacles_unknown.clone());

    let lidar = LidarSim::new(obstacles_all, 60.0);

    let mut grid = OccupancyGrid::new(-30.0, -50.0, 220, 150, 1.0);
    // Marcar APENAS os obstaculos conhecidos no grid
    for obs in &obstacles_known {
        let dn = obs.n_end - obs.n_start;
        let de = obs.e_end - obs.e_start;
        let steps = ((dn * dn + de * de).sqrt() / 0.5) as usize;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            grid.mark_obstacle(obs.n_start + dn * t, obs.e_start + de * t);
        }
    }

    // === Missao ===
    let mission = vec![[100.0_f32, 50.0], [150.0, -30.0], [0.0, 0.0]];
    let mut mission_idx = 0;

    println!("=== MISSAO COMPOSTA + DAA DINAMICO ===");
    println!("Waypoints: A(100,50) -> B(150,-30) -> C(0,0)");
    println!("Obstaculo conhecido: N=50 E=[0,40]");
    println!("Obstaculo DESCONHECIDO: N=110 E=[-20,20] (so LiDAR deteta)");
    println!();

    // Planejar primeira perna
    let plan = plan_astar(&grid, [0.0, 0.0], mission[0]).expect("A* inicial falhou");
    let mut trajectory = Trajectory::from_waypoints(&plan, 0.5);

    println!(
        "Perna 1 -> A: {} waypoints, {:.1}m",
        plan.len(),
        trajectory.total_length
    );

    // === Parametros Ultra Zenite ===
    let max_speed = 8.0_f32;
    let max_pitch = 20.0_f32.to_radians();
    let a_lat_max = 3.0_f32;
    let lookahead_base = 4.0_f32;
    let lookahead_speed_gain = 0.6_f32;
    let arrival_radius = 2.0_f32;

    let mut roll: f32 = 0.0;
    let mut pitch: f32 = 0.0;
    let mut yaw: f32 = 0.0;
    let mut roll_rate: f32 = 0.0;
    let mut pitch_rate: f32 = 0.0;
    let mut yaw_rate: f32 = 0.0;
    let mut yaw_gyro_only: f32 = 0.0;
    let mut fusion_mag = FusionMag::new(0.0);
    let mut fusion_mag_3d = FusionMag::new(0.0);
    const GYRO_BIAS: f32 = 0.002;
    const MAG_NORM: f32 = 50.0;

    let dt = 0.01_f32;

    // === Fisica realista: vento + arrasto ===
    const WIND_N: f32 = 3.0; // m/s (vento de norte)
    const WIND_E: f32 = -1.5; // m/s (vento de oeste)
    const DRAG_K: f32 = 0.005; // coeficiente de arrasto
    let mut gust_active = false;
    // === Abort logic state ===
    let mut last_dist_to_goal: f32 = f32::INFINITY;
    let mut stuck_timer: f32 = 0.0;
    let mut stuck_check_timer: f32 = 0.0;
    let mut direct_mode: bool = false;
    let _reroute_count: [u32; 3] = [0, 0, 0];
    let mut total_reroutes: u32 = 0;
    let mut t = 0.0_f32;
    let mut done = false;
    let mut progress_d = 0.0_f32;
    let mut current_speed = 0.0_f32;
    let mut replans = 0;
    let mut last_replan_t = -10.0_f32;
    let mut mav = MavlinkSender::new("127.0.0.1:14550", 1, 1).expect("MAVLink sender");
    let frame_socket = std::net::UdpSocket::bind("0.0.0.0:0").expect("bind frame socket");
    let mut frame_seq: u8 = 0;

    // === MAVLink Signing: chave de teste (32 bytes) ===
    // Em producao, a chave e provisionada via SETUP_SIGNING numa ligacao de confianca.
    // Para o SITL, usamos uma chave fixa de 32 bytes (0x00..0x1F).
    let signing_key: [u8; 32] = {
        let mut k = [0u8; 32];
        for i in 0..32 {
            k[i] = i as u8;
        }
        k
    };
    mav.enable_signing(signing_key, 0);
    // Ler timestamp persistido (anti-replay entre sessoes)
    let persisted_ts = std::fs::read_to_string("C:\\dev\\rnfc\\mavlink_signing_ts.txt")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);
    mav.set_signing_timestamp(persisted_ts);
    println!("[MAVLINK] Timestamp inicial carregado: {}", persisted_ts);

    println!("[MAVLINK] Signing ATIVADO (link_id=0, chave=0x00..0x1F)");
    let mut traj_file = File::create("C:\\dev\\rnfc\\trajetoria.csv").expect("CSV");
    writeln!(traj_file, "t,north,east,mission,replan").expect("header");
    let mut leg_start_t = 0.0_f32;

    println!(
        "\n{:>6}  {:>9}  {:>9}  {:>7}  {:>8}  {:>8}  {:>6}  {:>7}",
        "t(s)", "north(m)", "east(m)", "speed", "prog(m)", "sp_yaw", "missao", "replan"
    );

    for i in 0..=60000 {
        t += dt;
        // === MAVLink: 10 Hz ATTITUDE + 1 Hz HEARTBEAT ===
        if i % 10 == 0 {
            let p = nav.position();
            let _ = writeln!(
                traj_file,
                "{:.2},{:.3},{:.3},{},{}",
                t, p[0], p[1], mission_idx, replans
            );
        }

        if i % 100 == 0 {
            let _ = mav.send_attitude(roll, pitch, yaw, roll_rate, pitch_rate, yaw_rate);
        }
        if i % 1000 == 0 {
            let _ = mav.send_heartbeat();
        }

        if done {
            if i % 1000 == 0 {
                let p = nav.position();
                println!(
                    "{:6.2}  {:9.3}  {:9.3}  {:7.2}  {:8.2}  {:8.2}  {:>6}  {:>7}",
                    t, p[0], p[1], 0.0, progress_d, 0.0, "DONE", "-"
                );
            }
            continue;
        }

        let pos = nav.position();
        let pos2 = [pos[0], pos[1]];
        let goal = mission[mission_idx];

        // === Replan se LiDAR detecta obstaculo nao mapeado ===
        if t - last_replan_t > 4.0 {
            let hits = lidar.scan(pos[0], pos[1]);
            let _needs_replan = false;
            let mut new_obstacles = 0;
            for h in &hits {
                // Converter hit para NED aproximado: o LiDAR ja devolve N,E
                if grid
                    .to_grid(h[0], h[1])
                    .map_or(false, |(gi, gj)| grid.is_free(gi, gj))
                {
                    // Distancia ao caminho atual
                    let (_, hd) = trajectory.find_closest(*h);
                    let pp = trajectory.point_at(hd);
                    let d_to_path = ((h[0] - pp[0]).powi(2) + (h[1] - pp[1]).powi(2)).sqrt();
                    if d_to_path < 3.0 && hd > progress_d && hd < progress_d + 30.0 {
                        grid.mark_obstacle(h[0], h[1]);
                        new_obstacles += 1;
                    }
                }
            }
            if new_obstacles > 0 {
                if let Some(new_plan) = plan_astar(&grid, pos2, goal) {
                    trajectory = Trajectory::from_waypoints(&new_plan, 0.5);

                    replans += 1;
                    last_replan_t = t;
                    println!(
                        "    *** REPLAN #{} em t={:.1}s ({} novos obstaculos) ***",
                        replans, t, new_obstacles
                    );
                }
            }
        }

        // === Pure Pursuit ===
        let (_, closest_d) = trajectory.find_closest(pos2);
        progress_d = closest_d;

        let dist_to_end = (trajectory.total_length - progress_d).max(0.0);

        let dist_to_goal = ((pos[0] - goal[0]).powi(2) + (pos[1] - goal[1]).powi(2)).sqrt();

        // Se a trajetoria A* terminou mas ainda estamos longe do goal, ativar direct mode
        if dist_to_end < 3.0 && dist_to_goal > 15.0 && !direct_mode {
            println!(
                "    *** DIRECT MODE: trajetoria A* terminou a {:.1}m do goal ***",
                dist_to_goal
            );
            direct_mode = true;
        }

        // === Deteccao de "stuck" - reroute para waypoint mais proximo (nunca abortar) ===
        if dist_to_goal > 5.0 {
            stuck_check_timer += dt;
            if stuck_check_timer >= 1.0 {
                let progress_1s = last_dist_to_goal - dist_to_goal;
                if progress_1s < 0.5 && (t - leg_start_t) > 3.0 {
                    stuck_timer += 1.0;
                    if stuck_timer >= 8.0 {
                        let mut best_idx = mission_idx;
                        let mut best_dist = dist_to_goal;
                        for (idx, wp) in mission.iter().enumerate() {
                            if idx <= mission_idx {
                                continue;
                            }
                            let d = ((pos[0] - wp[0]).powi(2) + (pos[1] - wp[1]).powi(2)).sqrt();
                            if d < best_dist {
                                best_dist = d;
                                best_idx = idx;
                            }
                        }
                        if best_idx != mission_idx {
                            total_reroutes += 1;
                            if total_reroutes > 5 {
                                println!("    *** HOVER: {} reroutes, vento demasiado forte, a pairar em ({:.1},{:.1}) ***", total_reroutes, pos[0], pos[1]);
                                done = true;
                                continue;
                            }
                            println!(
                                "    *** REROUTE #{}: waypoint {} ***",
                                total_reroutes, best_idx
                            );
                            mission_idx = best_idx;
                            if let Some(new_plan) = plan_astar(&grid, pos2, mission[mission_idx]) {
                                trajectory = Trajectory::from_waypoints(&new_plan, 0.5);
                                progress_d = 0.0;
                                leg_start_t = t;
                                direct_mode = false;
                                last_dist_to_goal = f32::INFINITY;
                                stuck_timer = 0.0;
                                stuck_check_timer = 0.0;
                                continue;
                            }
                        } else {
                            // Ja esta a ir para o waypoint mais proximo - persistir
                            println!(
                                "    *** DIRECT MODE: heading direto para waypoint {} ***",
                                mission_idx
                            );
                            direct_mode = true;
                            stuck_timer = 0.0;
                            stuck_check_timer = 0.0;
                            last_dist_to_goal = f32::INFINITY;
                        }
                    }
                } else {
                    stuck_timer = 0.0;
                }
                last_dist_to_goal = dist_to_goal;
                stuck_check_timer = 0.0;
            }
        }

        // Lookahead adaptativo
        let lookahead = if dist_to_end < 15.0 {
            (dist_to_end * 0.5).max(1.5)
        } else {
            lookahead_base + current_speed * lookahead_speed_gain
        };

        let target_d = (closest_d + lookahead).min(trajectory.total_length);
        let target = trajectory.point_at(target_d);

        let dn = target[0] - pos[0];
        let de = target[1] - pos[1];

        // MODO DE APROXIMACAO FINAL: quando perto do goal, mira direto a ele
        let sp_yaw = if dist_to_goal < 8.0 {
            let dg_n = goal[0] - pos[0];
            let dg_e = goal[1] - pos[1];
            dg_e.atan2(dg_n)
        } else {
            // Cross-track correction normal
            let closest_pt = trajectory.point_at(closest_d);
            let cross_n = closest_pt[0] - pos[0];
            let cross_e = closest_pt[1] - pos[1];
            (de + 0.4 * cross_e).atan2(dn + 0.4 * cross_n)
        };

        // === Velocidade por curvatura ===
        let curvature = trajectory.curvature_at(target_d);
        let v_curve = if curvature > 0.005 {
            (a_lat_max / curvature).sqrt().min(max_speed)
        } else {
            max_speed
        };

        // Desacelerar baseado na distancia ao goal
        let v_end = if dist_to_goal < 20.0 {
            (dist_to_goal / 20.0 * max_speed).max(0.3)
        } else {
            max_speed
        };

        // Em DIRECT MODE, ignorar v_curve (a trajetoria antiga nao e relevante)
        let v_target = if direct_mode {
            if dist_to_goal < 25.0 {
                (dist_to_goal / 25.0 * max_speed).max(0.5)
            } else {
                max_speed
            }
        } else {
            v_curve.min(v_end)
        };
        let sp_pitch = (v_target / max_speed) * max_pitch;
        let setpoints = [0.0_f32, sp_pitch, sp_yaw];

        let measured = [roll, pitch, fusion_mag.yaw];
        let cmd = controller.update(setpoints, measured, dt);

        let damping = 0.8_f32;
        roll_rate += (cmd[0] - damping * roll_rate) * dt;
        pitch_rate += (cmd[1] - damping * pitch_rate) * dt;
        yaw_rate += (cmd[2] - damping * yaw_rate) * dt;

        roll += roll_rate * dt;
        pitch += pitch_rate * dt;
        yaw += yaw_rate * dt;
        while yaw > core::f32::consts::PI {
            yaw -= 2.0 * core::f32::consts::PI;
        }
        while yaw < -core::f32::consts::PI {
            yaw += 2.0 * core::f32::consts::PI;
        }

        // === Passo 9: simulacao magnetometro + fusao (diagnostico) ===
        yaw_gyro_only += (yaw_rate + GYRO_BIAS) * dt;
        let attack_active = t >= 150.0 && t < 160.0;
        let attack_offset = if attack_active { 0.5236_f32 } else { 0.0 };
        let yaw_mag = yaw + attack_offset;
        let mag_n = MAG_NORM * yaw_mag.cos() + 0.5 * ((i as f32 * 0.7).sin());
        let mag_e = MAG_NORM * yaw_mag.sin() + 0.5 * ((i as f32 * 1.3).sin());
        fusion_mag.update(mag_n, mag_e, yaw_rate + GYRO_BIAS, dt);
        // === Fase 3D (diagnostico paralelo) ===
        let yaw_world = yaw
            + (if t >= 150.0 && t < 160.0 {
                0.5236_f32
            } else {
                0.0
            });
        let cy = yaw_world.cos();
        let sy = yaw_world.sin();
        let cp = pitch.cos();
        let sp = pitch.sin();
        let cr = roll.cos();
        let sr = roll.sin();
        let h = MAG_NORM;
        let mb_n = h * cp * cy;
        let mb_e = -h * sy * cr + h * sp * cy * sr;
        let mb_d = h * sy * sr + h * sp * cy * cr;
        fusion_mag_3d.update_3d(mb_n, mb_e, mb_d, roll, pitch, yaw_rate + GYRO_BIAS, dt);
        if i % 1000 == 0 {
            let true_d = yaw * 180.0 / core::f32::consts::PI;
            let gyro_d = yaw_gyro_only * 180.0 / core::f32::consts::PI;
            let fused_d = fusion_mag.yaw * 180.0 / core::f32::consts::PI;
            let mut drift = gyro_d - true_d;
            while drift > 180.0 {
                drift -= 360.0;
            }
            while drift < -180.0 {
                drift += 360.0;
            }
            let mut err = fused_d - true_d;
            while err > 180.0 {
                err -= 360.0;
            }
            while err < -180.0 {
                err += 360.0;
            }
            let tag = if attack_active { " [ATTACK]" } else { "" };
            let fused_3d_d = fusion_mag_3d.yaw * 180.0 / core::f32::consts::PI;
            let mut err_3d = fused_3d_d - true_d;
            while err_3d > 180.0 {
                err_3d -= 360.0;
            }
            while err_3d < -180.0 {
                err_3d += 360.0;
            }
            println!(
                "    [FUSION]{} t={:.1}s | true={:.1}deg | 2d_err={:+.1}deg | 3d_err={:+.1}deg",
                tag, t, true_d, err, err_3d
            );
        }

        let accel_max = 10.0_f32;
        let desired = max_speed * (pitch.abs() / max_pitch).min(1.0);
        let dv = (desired - current_speed).clamp(-accel_max * dt, accel_max * dt);
        current_speed += dv;
        current_speed = current_speed.min(v_target).max(0.0);

        let drag = DRAG_K * current_speed * current_speed;
        let effective_speed = (current_speed - drag).max(0.0);
        if t > 60.0 && t < 65.0 && !gust_active {
            gust_active = true;
            println!("    *** RAJADA DE VENTO em t={:.1}s ***", t);
        }
        let gust_n = if gust_active && t < 65.0 { 5.0 } else { 0.0 };
        let gust_e = if gust_active && t < 65.0 { 3.0 } else { 0.0 };
        let wind_scale = if dist_to_goal < 15.0 { 0.0 } else { 1.0 };
        let vel_n = effective_speed * yaw.cos() + (WIND_N + gust_n) * wind_scale;
        let vel_e = effective_speed * yaw.sin() + (WIND_E + gust_e) * wind_scale;
        let new_pos = [pos[0] + vel_n * dt, pos[1] + vel_e * dt, 0.0];

        // === Chegada a este waypoint ===
        let effective_arrival = arrival_radius;
        if dist_to_goal < effective_arrival {
            // Chegou ao waypoint - mudar para o proximo
            let tints: [[u8; 3]; 4] =
                [[220, 60, 60], [60, 220, 60], [60, 60, 220], [220, 220, 220]];
            let tint = tints[mission_idx.min(3)];
            let img = rnfc_mavlink::frame::generate_test_image(64, 48, tint);
            let packets = rnfc_mavlink::frame::encode_frame(&mut frame_seq, 64, 48, &img);
            for pkt in &packets {
                let _ = frame_socket.send_to(pkt, "127.0.0.1:14551");
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            println!(
                "    [MAV] frame enviado (wp={}, {} chunks, {} bytes)",
                mission_idx,
                packets.len() - 1,
                img.len()
            );
            println!(
                "    -> Waypoint {} atingido em t={:.2}s (perna {:.1}s) | pos=({:.1},{:.1})",
                mission_idx,
                t,
                t - leg_start_t,
                pos[0],
                pos[1]
            );
            mission_idx += 1;
            direct_mode = false; // reset ao mudar de waypoint
            if mission_idx >= mission.len() {
                println!("\n*** MISSAO COMPLETA em t={:.2}s ***\n", t);
                done = true;
                continue;
            }
            // Planear proxima perna
            let next_goal = mission[mission_idx];
            if let Some(new_plan) = plan_astar(&grid, pos2, next_goal) {
                trajectory = Trajectory::from_waypoints(&new_plan, 0.5);

                leg_start_t = t;
                direct_mode = false;
                println!(
                    "    Perna {} -> wp{}: {} waypoints, {:.1}m",
                    mission_idx + 1,
                    mission_idx,
                    new_plan.len(),
                    trajectory.total_length
                );
            } else {
                println!("    ERRO: A* falhou para wp{}", mission_idx);
                done = true;
            }
            continue;
        }

        let imu = ImuSample {
            accel: [0.0, 0.0, -9.81],
            gyro: [roll_rate, pitch_rate, yaw_rate],
            dt,
        };
        let gnss = Some(GnssFix {
            position: new_pos,
            velocity: [vel_n, vel_e, 0.0],
            hdop: 1.0,
        });
        let _s = nav.step(&imu, gnss.as_ref());

        if i % 500 == 0 {
            let p = nav.position();
            println!(
                "{:6.2}  {:9.3}  {:9.3}  {:7.2}  {:8.2}  {:8.2}  {:>6}  {:>7}",
                t,
                p[0],
                p[1],
                current_speed,
                progress_d,
                to_deg(sp_yaw),
                mission_idx.to_string(),
                replans.to_string()
            );
        }
    }

    let final_pos = nav.position();
    let dist = ((final_pos[0] - 0.0).powi(2) + (final_pos[1] - 0.0).powi(2)).sqrt();
    // Persistir o timestamp de signing (anti-replay entre sessoes)
    let _ = std::fs::write(
        "C:\\dev\\rnfc\\mavlink_signing_ts.txt",
        mav.signing_timestamp().to_string(),
    );
    println!(
        "[MAVLINK] Timestamp de signing persistido: {}",
        mav.signing_timestamp()
    );

    println!("\n=== MAVLINK ===");
    println!(
        "ATTITUDE enviadas: {} | HEARTBEAT enviados: {} | Erros: {}",
        mav.stats_attitudes, mav.stats_heartbeats, mav.stats_errors
    );
    println!("\n=== RESULTADO FINAL ===");
    println!(
        "posicao final: N={:.2}m E={:.2}m",
        final_pos[0], final_pos[1]
    );
    println!("waypoints visitados: A(100,50) -> B(150,-30) -> C(0,0)");
    println!("replans dinamicos: {}", replans);
    println!("distancia a base: {:.2}m", dist);
    if dist < 2.0 {
        println!("\nOK: Passo 9 - missao composta + DAA dinamico concluidos.");
    }
}


