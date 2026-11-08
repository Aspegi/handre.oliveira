use rnfc_abstractions::{GnssFix, ImuSample};
use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::NavCore;
use rnfc_daa::{plan_astar, LidarSim, Obstacle, OccupancyGrid};

fn to_deg(rad: f32) -> f32 {
    rad * 180.0 / core::f32::consts::PI
}

fn main() {
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let mut controller = AttitudeController::new();

    // ===== DAA: obstaculos e grid =====
    let obstacles = vec![
        Obstacle { n_start: 50.0, e_start: 0.0, n_end: 50.0, e_end: 40.0 },
    ];
    let lidar = LidarSim::new(obstacles.clone(), 80.0);

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

    // ===== Planejar caminho =====
    let start = [0.0, 0.0];
    let goal = [100.0, 50.0];
    let plan = plan_astar(&grid, start, goal).expect("A* falhou");

    println!("=== PLANO A* ===");
    println!("Waypoints planeados: {} pontos", plan.len());
    for (k, p) in plan.iter().enumerate() {
        if k < 5 || k > plan.len() - 3 {
            println!("  [{}] N={:.1} E={:.1}", k, p[0], p[1]);
        } else if k == 5 {
            println!("  ... ({} waypoints omitidos) ...", plan.len().saturating_sub(7));
        }
    }

    // ===== Parametros de guiamento =====
    let mut plan_idx = 0;
    let arrival_radius = 2.0_f32;
    let arrival_radius_final = 0.5_f32;
    let k_pitch = 0.01_f32;
    let max_pitch = 15.0_f32.to_radians();

    let mut roll: f32 = 0.0;
    let mut pitch: f32 = 0.0;
    let mut yaw: f32 = 0.0;
    let mut roll_rate: f32 = 0.0;
    let mut pitch_rate: f32 = 0.0;
    let mut yaw_rate: f32 = 0.0;

    let dt = 0.01_f32;
    let mut t = 0.0_f32;
    let mut done = false;

    println!("\n{:>6}  {:>9}  {:>9}  {:>8}  {:>8}  {:>8}",
        "t(s)", "north(m)", "east(m)", "sp_pit", "sp_yaw", "wp_idx");

    for i in 0..=40000 {
        t += dt;

        if done {
            if i % 1000 == 0 {
                let p = nav.position();
                println!("{:6.2}  {:9.3}  {:9.3}  {:8.2}  {:8.2}  {:>8}",
                    t, p[0], p[1], 0.0, 0.0, "DONE");
            }
            continue;
        }

        let pos = nav.position();

        // === Guiamento direto para o waypoint atual do plano ===
        let target = plan[plan_idx];
        let dn = target[0] - pos[0];
        let de = target[1] - pos[1];
        let dist = (dn * dn + de * de).sqrt();

        let radius = if plan_idx + 1 >= plan.len() {
            arrival_radius_final
        } else {
            arrival_radius
        };

        if dist < radius {
            if plan_idx + 1 < plan.len() {
                println!("    -> Waypoint [{}] N={:.1} E={:.1} atingido em t={:.2}s",
                         plan_idx, target[0], target[1], t);
                plan_idx += 1;
                continue;
            } else {
                println!("\n*** DESTINO FINAL ATINGIDO em t={:.2}s ***\n", t);
                done = true;
                continue;
            }
        }

        // Setpoints de atitude
        let sp_yaw = de.atan2(dn);
        let mut sp_pitch = k_pitch * dist;
        if sp_pitch > max_pitch { sp_pitch = max_pitch; }
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

        let max_speed = 5.0_f32;
        let speed = max_speed * (pitch.abs() / max_pitch).min(1.0);
        let vel_n = speed * yaw.cos();
        let vel_e = speed * yaw.sin();

        let new_pos = [pos[0] + vel_n * dt, pos[1] + vel_e * dt, 0.0];

        // LiDAR scan
        if i % 2000 == 0 && i > 0 {
            let hits = lidar.scan(pos[0], pos[1]);
            let nearest = hits.iter()
                .map(|h| ((h[0]-pos[0]).powi(2) + (h[1]-pos[1]).powi(2)).sqrt())
                .fold(f32::INFINITY, f32::min);
            println!("    LiDAR: {} hits, mais proximo a {:.1}m",
                     hits.len(),
                     if nearest.is_finite() { nearest } else { -1.0 });
        }

        let imu = ImuSample { accel: [0.0, 0.0, -9.81], gyro: [roll_rate, pitch_rate, yaw_rate], dt };
        let gnss = Some(GnssFix { position: new_pos, velocity: [vel_n, vel_e, 0.0], hdop: 1.0 });
        let _s = nav.step(&imu, gnss.as_ref());

        if i % 500 == 0 {
            let p = nav.position();
            println!("{:6.2}  {:9.3}  {:9.3}  {:8.2}  {:8.2}  {:>8}",
                t, p[0], p[1],
                to_deg(sp_pitch), to_deg(sp_yaw),
                plan_idx.to_string());
        }
    }

    let final_pos = nav.position();
    let dn = final_pos[0] - 100.0;
    let de = final_pos[1] - 50.0;
    let dist = (dn * dn + de * de).sqrt();

    println!("\n=== RESULTADO FINAL ===");
    println!("posicao final: N={:.2}m E={:.2}m", final_pos[0], final_pos[1]);
    println!("obstaculo: parede em N=50, E=[0,40]");
    println!("distancia ao alvo (100,50): {:.2}m", dist);

    if dist < 5.0 {
        println!("\nOK: Passo 8 — drone desviou da parede e chegou ao alvo.");
    } else {
        println!("\nDistancia ao alvo = {:.2}m", dist);
    }
}