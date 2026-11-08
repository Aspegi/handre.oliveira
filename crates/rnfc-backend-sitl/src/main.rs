// ULTRA ZENITE - Pure Pursuit build
use rnfc_abstractions::{GnssFix, ImuSample};
use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::NavCore;
use rnfc_daa::trajectory::Trajectory;
use rnfc_daa::{plan_astar, LidarSim, Obstacle, OccupancyGrid};

fn to_deg(rad: f32) -> f32 { rad * 180.0 / core::f32::consts::PI }

fn main() {
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let mut controller = AttitudeController::new();

    let obstacles = vec![
        Obstacle { n_start: 50.0, e_start: 0.0, n_end: 50.0, e_end: 40.0 },
    ];
    let lidar = LidarSim::new(obstacles.clone(), 60.0);

    let mut grid = OccupancyGrid::new(-10.0, -50.0, 200, 150, 1.0);
    for obs in &obstacles {
        let dn = obs.n_end - obs.n_start;
        let de = obs.e_end - obs.e_start;
        let steps = ((dn * dn + de * de).sqrt() / 0.5) as usize;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            grid.mark_obstacle(obs.n_start + dn * t, obs.e_start + de * t);
        }
    }

    let plan = plan_astar(&grid, [0.0, 0.0], [100.0, 50.0]).expect("A* falhou");
    let mut trajectory = Trajectory::from_waypoints(&plan, 0.5);

    println!("=== TRAJETORIA ===");
    println!("Waypoints A*: {} | Pontos densos: {} | Comprimento: {:.1}m",
             plan.len(), trajectory.points.len(), trajectory.total_length);

    let max_speed = 8.0_f32;
    let max_pitch = 20.0_f32.to_radians();
    let a_lat_max = 3.0_f32;
    let lookahead_base = 4.0_f32;
    let lookahead_speed_gain = 0.6_f32;
    let arrival_radius = 1.0_f32;

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

    println!("\n{:>6}  {:>9}  {:>9}  {:>7}  {:>8}  {:>8}  {:>8}",
        "t(s)", "north(m)", "east(m)", "speed", "prog(m)", "sp_yaw", "replan");

    for i in 0..=40000 {
        t += dt;

        if done {
            if i % 1000 == 0 {
                let p = nav.position();
                println!("{:6.2}  {:9.3}  {:9.3}  {:7.2}  {:8.2}  {:8.2}  {:>8}",
                    t, p[0], p[1], 0.0, progress_d, 0.0, "-");
            }
            continue;
        }

        let pos = nav.position();
        let pos2 = [pos[0], pos[1]];

        // === Replan se LiDAR detecta obstaculo nao mapeado ===
        if t - last_replan_t > 2.0 {
            let hits = lidar.scan(pos[0], pos[1]);
            let mut needs_replan = false;
            for h in &hits {
                let (_, hd) = trajectory.find_closest(*h);
                if hd > progress_d && hd < progress_d + 25.0 {
                    let pp = trajectory.point_at(hd);
                    let d = ((h[0]-pp[0]).powi(2) + (h[1]-pp[1]).powi(2)).sqrt();
                    if d < 2.5 && grid.to_grid(h[0], h[1]).map_or(false, |(gi, gj)| grid.is_free(gi, gj)) {
                        grid.mark_obstacle(h[0], h[1]);
                        needs_replan = true;
                    }
                }
            }
            if needs_replan {
                let goal = trajectory.point_at(trajectory.total_length);
                if let Some(new_plan) = plan_astar(&grid, pos2, goal) {
                    trajectory = Trajectory::from_waypoints(&new_plan, 0.5);
                    progress_d = 0.0;
                    replans += 1;
                    last_replan_t = t;
                    println!("    *** REPLAN #{} em t={:.1}s ***", replans, t);
                }
            }
        }

        // === Pure Pursuit ===
        let (_, closest_d) = trajectory.find_closest(pos2);
        progress_d = closest_d;

        // dist_to_end DEFINIDO AQUI (antes do lookahead)
        let dist_to_end = (trajectory.total_length - progress_d).max(0.0);

        // Lookahead adaptativo (curto quando perto do fim)
        let lookahead = if dist_to_end < 15.0 {
            (dist_to_end * 0.5).max(1.5)
        } else {
            lookahead_base + current_speed * lookahead_speed_gain
        };

        let target_d = (closest_d + lookahead).min(trajectory.total_length);
        let target = trajectory.point_at(target_d);

        let dn = target[0] - pos[0];
        let de = target[1] - pos[1];
        // Cross-track: puxa o drone para a trajetoria se estiver a desviar
        let closest_pt = trajectory.point_at(closest_d);
        let cross_n = closest_pt[0] - pos[0];
        let cross_e = closest_pt[1] - pos[1];
        let sp_yaw = (de + 0.4 * cross_e).atan2(dn + 0.4 * cross_n);

        // === Velocidade por curvatura ===
        let curvature = trajectory.curvature_at(target_d);
        let v_curve = if curvature > 0.005 {
            (a_lat_max / curvature).sqrt().min(max_speed)
        } else {
            max_speed
        };

        // Desacelerar a partir de 20m do fim
        let dist_to_goal = ((pos[0]-100.0).powi(2) + (pos[1]-50.0).powi(2)).sqrt();
        let v_end = if dist_to_goal < 15.0 {
            (dist_to_goal / 15.0 * max_speed).max(0.3)
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

        // === Chegada baseada no GOAL real (100, 50) ===
        let dist_to_goal = ((pos[0]-100.0).powi(2) + (pos[1]-50.0).powi(2)).sqrt();
        if dist_to_goal < 1.5 && current_speed < 0.5 {
            println!("\n*** DESTINO FINAL ATINGIDO em t={:.2}s ***\n", t);
            done = true;
            continue;
        }

        let imu = ImuSample { accel: [0.0, 0.0, -9.81], gyro: [roll_rate, pitch_rate, yaw_rate], dt };
        let gnss = Some(GnssFix { position: new_pos, velocity: [vel_n, vel_e, 0.0], hdop: 1.0 });
        let _s = nav.step(&imu, gnss.as_ref());

        if i % 500 == 0 {
            let p = nav.position();
            println!("{:6.2}  {:9.3}  {:9.3}  {:7.2}  {:8.2}  {:8.2}  {:>8}",
                t, p[0], p[1], current_speed, progress_d,
                to_deg(sp_yaw), replans.to_string());
        }
    }

    let final_pos = nav.position();
    let dist = ((final_pos[0]-100.0).powi(2) + (final_pos[1]-50.0).powi(2)).sqrt();
    println!("\n=== RESULTADO FINAL ===");
    println!("posicao final: N={:.2}m E={:.2}m", final_pos[0], final_pos[1]);
    println!("comprimento trajetoria: {:.1}m", trajectory.total_length);
    println!("replans durante missao: {}", replans);
    println!("distancia ao alvo: {:.2}m", dist);
    if dist < 2.0 {
        println!("\nOK: Ultra Zenite Ã¢â‚¬â€ Pure Pursuit + curvatura + replan ativos.");
    }
}