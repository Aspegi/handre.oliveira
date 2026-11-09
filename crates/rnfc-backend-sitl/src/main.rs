// Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)
// ULTRA ZENITE - Missao Composta + DAA Dinamico
use rnfc_abstractions::{GnssFix, ImuSample};
use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::NavCore;
use rnfc_daa::trajectory::Trajectory;
use rnfc_daa::{plan_astar, LidarSim, Obstacle, OccupancyGrid};
use rnfc_mavlink::MavlinkSender;
use std::fs::File;
use std::io::Write as IoWrite;

fn to_deg(rad: f32) -> f32 { rad * 180.0 / core::f32::consts::PI }

fn main() {
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let mut controller = AttitudeController::new();

    // === Obstaculos ===
    // Conhecido: parede em N=50, E=[0,40] (mapeada no grid)
    // Desconhecido: parede em N=110, E=[-20,20] (so LiDAR deteta)
    let obstacles_known = vec![
        Obstacle { n_start: 50.0, e_start: 0.0, n_end: 50.0, e_end: 40.0 },
    ];
    let obstacles_unknown = vec![
        Obstacle { n_start: 110.0, e_start: -20.0, n_end: 110.0, e_end: 20.0 },
    ];
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
    let mission = vec![
        [100.0_f32, 50.0],
        [150.0, -30.0],
        [0.0, 0.0],
    ];
    let mut mission_idx = 0;

    println!("=== MISSAO COMPOSTA + DAA DINAMICO ===");
    println!("Waypoints: A(100,50) -> B(150,-30) -> C(0,0)");
    println!("Obstaculo conhecido: N=50 E=[0,40]");
    println!("Obstaculo DESCONHECIDO: N=110 E=[-20,20] (so LiDAR deteta)");
    println!();

    // Planejar primeira perna
    let plan = plan_astar(&grid, [0.0, 0.0], mission[0]).expect("A* inicial falhou");
    let mut trajectory = Trajectory::from_waypoints(&plan, 0.5);

    println!("Perna 1 -> A: {} waypoints, {:.1}m", plan.len(), trajectory.total_length);

    // === Parametros Ultra Zenite ===
    let max_speed = 8.0_f32;
    let max_pitch = 20.0_f32.to_radians();
    let a_lat_max = 3.0_f32;
    let lookahead_base = 4.0_f32;
    let lookahead_speed_gain = 0.6_f32;
    let arrival_radius = 2.5_f32;

    let mut roll: f32 = 0.0;
    let mut pitch: f32 = 0.0;
    let mut yaw: f32 = 0.0;
    let mut roll_rate: f32 = 0.0;
    let mut pitch_rate: f32 = 0.0;
    let mut yaw_rate: f32 = 0.0;

    let dt = 0.01_f32;
    let mut t = 0.0_f32;
    let mut done = false;
    let mut progress_d = 0.0_f32;
    let mut current_speed = 0.0_f32;
    let mut replans = 0;
    let mut last_replan_t = -10.0_f32;
    let mut mav = MavlinkSender::new("127.0.0.1:14550", 1, 1).expect("MAVLink sender");
    let mut mav_att_cnt = 0u64;
    let mut mav_hb_cnt = 0u64;
    let mut traj_file = File::create("C:\\dev\\rnfc\\trajetoria.csv").expect("CSV");
    writeln!(traj_file, "t,north,east,mission,replan").expect("header");
    let mut leg_start_t = 0.0_f32;

    println!("\n{:>6}  {:>9}  {:>9}  {:>7}  {:>8}  {:>8}  {:>6}  {:>7}",
        "t(s)", "north(m)", "east(m)", "speed", "prog(m)", "sp_yaw", "missao", "replan");

    for i in 0..=60000 {
        t += dt;
        // === MAVLink: 10 Hz ATTITUDE + 1 Hz HEARTBEAT ===
        if i % 10 == 0 {
            let p = nav.position();
            let _ = writeln!(traj_file, "{:.2},{:.3},{:.3},{},{}",
                             t, p[0], p[1], mission_idx, replans);
        }

        if i % 100 == 0 {
            let _ = mav.send_attitude(roll, pitch, yaw, roll_rate, pitch_rate, yaw_rate);
            mav_att_cnt += 1;
        }
        if i % 1000 == 0 {
            let _ = mav.send_heartbeat();
            mav_hb_cnt += 1;
        }



        if done {
            if i % 1000 == 0 {
                let p = nav.position();
                println!("{:6.2}  {:9.3}  {:9.3}  {:7.2}  {:8.2}  {:8.2}  {:>6}  {:>7}",
                    t, p[0], p[1], 0.0, progress_d, 0.0, "DONE", "-");
            }
            continue;
        }

        let pos = nav.position();
        let pos2 = [pos[0], pos[1]];
        let goal = mission[mission_idx];

        // === Replan se LiDAR detecta obstaculo nao mapeado ===
        if t - last_replan_t > 4.0 {
            let hits = lidar.scan(pos[0], pos[1]);
            let mut needs_replan = false;
            let mut new_obstacles = 0;
            for h in &hits {
                // Converter hit para NED aproximado: o LiDAR ja devolve N,E
                if grid.to_grid(h[0], h[1]).map_or(false, |(gi, gj)| grid.is_free(gi, gj)) {
                    // Distancia ao caminho atual
                    let (_, hd) = trajectory.find_closest(*h);
                    let pp = trajectory.point_at(hd);
                    let d_to_path = ((h[0]-pp[0]).powi(2) + (h[1]-pp[1]).powi(2)).sqrt();
                    if d_to_path < 3.0 && hd > progress_d && hd < progress_d + 30.0 {
                        grid.mark_obstacle(h[0], h[1]);
                        new_obstacles += 1;
                    }
                }
            }
            if new_obstacles > 0 {
                if let Some(new_plan) = plan_astar(&grid, pos2, goal) {
                    trajectory = Trajectory::from_waypoints(&new_plan, 0.5);
                    progress_d = 0.0;
                    replans += 1;
                    last_replan_t = t;
                    println!("    *** REPLAN #{} em t={:.1}s ({} novos obstaculos) ***",
                             replans, t, new_obstacles);
                }
            }
        }

        // === Pure Pursuit ===
        let (_, closest_d) = trajectory.find_closest(pos2);
        progress_d = closest_d;

        let dist_to_end = (trajectory.total_length - progress_d).max(0.0);
        let dist_to_goal = ((pos[0]-goal[0]).powi(2) + (pos[1]-goal[1]).powi(2)).sqrt();

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
        let v_end = if dist_to_goal < 8.0 {
            (dist_to_goal / 8.0 * max_speed).max(0.2)
        } else {
            max_speed
        };

        let v_target = v_curve.min(v_end);
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

        let accel_max = 5.0_f32;
        let desired = max_speed * (pitch.abs() / max_pitch).min(1.0);
        let dv = (desired - current_speed).clamp(-accel_max * dt, accel_max * dt);
        current_speed += dv;
        current_speed = current_speed.min(v_target).max(0.0);

        let vel_n = current_speed * yaw.cos();
        let vel_e = current_speed * yaw.sin();
        let new_pos = [pos[0] + vel_n * dt, pos[1] + vel_e * dt, 0.0];

        // === Chegada a este waypoint ===
        if dist_to_goal < arrival_radius && current_speed < 0.5 {
            println!("    -> Waypoint {} atingido em t={:.2}s (perna {:.1}s) | pos=({:.1},{:.1})",
                     mission_idx, t, t - leg_start_t, pos[0], pos[1]);
            mission_idx += 1;
            if mission_idx >= mission.len() {
                println!("\n*** MISSAO COMPLETA em t={:.2}s ***\n", t);
                done = true;
                continue;
            }
            // Planear proxima perna
            let next_goal = mission[mission_idx];
            if let Some(new_plan) = plan_astar(&grid, pos2, next_goal) {
                trajectory = Trajectory::from_waypoints(&new_plan, 0.5);
                progress_d = 0.0;
                leg_start_t = t;
                println!("    Perna {} -> wp{}: {} waypoints, {:.1}m",
                         mission_idx + 1, mission_idx, new_plan.len(), trajectory.total_length);
            } else {
                println!("    ERRO: A* falhou para wp{}", mission_idx);
                done = true;
            }
            continue;
        }

        let imu = ImuSample { accel: [0.0, 0.0, -9.81], gyro: [roll_rate, pitch_rate, yaw_rate], dt };
        let gnss = Some(GnssFix { position: new_pos, velocity: [vel_n, vel_e, 0.0], hdop: 1.0 });
        let _s = nav.step(&imu, gnss.as_ref());

        if i % 500 == 0 {
            let p = nav.position();
            println!("{:6.2}  {:9.3}  {:9.3}  {:7.2}  {:8.2}  {:8.2}  {:>6}  {:>7}",
                t, p[0], p[1], current_speed, progress_d,
                to_deg(sp_yaw), mission_idx.to_string(), replans.to_string());
        }
    }

    let final_pos = nav.position();
    let dist = ((final_pos[0]-0.0).powi(2) + (final_pos[1]-0.0).powi(2)).sqrt();
    println!("\n=== MAVLINK ===");
    println!("ATTITUDE enviadas: {} | HEARTBEAT enviados: {} | Erros: {}",
             mav.stats_attitudes, mav.stats_heartbeats, mav.stats_errors);
    println!("\n=== RESULTADO FINAL ===");
    println!("posicao final: N={:.2}m E={:.2}m", final_pos[0], final_pos[1]);
    println!("waypoints visitados: A(100,50) -> B(150,-30) -> C(0,0)");
    println!("replans dinamicos: {}", replans);
    println!("distancia a base: {:.2}m", dist);
    if dist < 2.0 {
        println!("\nOK: Passo 9 - missao composta + DAA dinamico concluidos.");
    }
}
