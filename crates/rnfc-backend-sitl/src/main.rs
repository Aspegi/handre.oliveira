use rnfc_abstractions::{GnssFix, ImuSample};
use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::guidance::Guidance;
use rnfc_core::NavCore;

fn to_deg(rad: f32) -> f32 {
    rad * 180.0 / core::f32::consts::PI
}

fn main() {
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let mut controller = AttitudeController::new();
    let mut guidance = Guidance::new();

    let mut roll: f32 = 0.0;
    let mut pitch: f32 = 0.0;
    let mut yaw: f32 = 0.0;
    let mut roll_rate: f32 = 0.0;
    let mut pitch_rate: f32 = 0.0;
    let mut yaw_rate: f32 = 0.0;

    let dt = 0.01_f32;
    let mut t = 0.0_f32;
    let mut all_arrived = false;

    println!(
        "{:>6}  {:>9}  {:>9}  {:>9}  {:>9}  {:>6}  {:>7}",
        "t(s)", "north(m)", "east(m)", "sp_pitch", "sp_yaw", "wp", "chegou"
    );

    for i in 0..=30000 {
        t += dt;

        if all_arrived {
            if i % 1000 == 0 {
                let p = nav.position();
                println!(
                    "{:6.2}  {:9.3}  {:9.3}  {:9.2}  {:9.2}  {:>6}  {:>7}",
                    t, p[0], p[1], 0.0, 0.0, "DONE", "SIM"
                );
            }
            continue;
        }

        let pos = nav.position();
        let label_before = guidance.current_label();
        let (setpoints, arrived) = guidance.update(pos);
        let label_after = guidance.current_label();

        if label_before != label_after {
            println!("\n*** WAYPOINT {} ATINGIDO em t={:.2}s -> a caminho de {} ***\n",
                     label_before, t, label_after);
        }

        if arrived {
            println!("\n*** MISSAO COMPLETA em t={:.2}s ***\n", t);
            all_arrived = true;
            continue;
        }

        let measured = [roll, pitch, yaw];
        let cmd = controller.update(setpoints, measured, dt);

        let damping = 0.8_f32;
        roll_rate  += (cmd[0] - damping * roll_rate) * dt;
        pitch_rate += (cmd[1] - damping * pitch_rate) * dt;
        yaw_rate   += (cmd[2] - damping * yaw_rate) * dt;

        roll  += roll_rate  * dt;
        pitch += pitch_rate * dt;
        yaw   += yaw_rate   * dt;

        // Velocidade com abrandamento perto do waypoint
        let max_speed = 5.0_f32;
        let max_pitch = 15.0_f32.to_radians();

        let wp = guidance.current_waypoint();
        let dn = wp[0] - pos[0];
        let de = wp[1] - pos[1];
        let dist = (dn * dn + de * de).sqrt();

        let speed_factor = if dist < 10.0 { (dist / 10.0).max(0.15) } else { 1.0 };
        let base_speed = max_speed * (pitch.abs() / max_pitch).min(1.0);
        let speed = base_speed * speed_factor;

        let vel_n = speed * yaw.cos();
        let vel_e = speed * yaw.sin();

        let new_pos = [pos[0] + vel_n * dt, pos[1] + vel_e * dt, 0.0];

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

        if i % 1000 == 0 {
            let p = nav.position();
            println!(
                "{:6.2}  {:9.3}  {:9.3}  {:9.2}  {:9.2}  {:>6}  {:>7}",
                t,
                p[0], p[1],
                to_deg(setpoints[1]), to_deg(setpoints[2]),
                label_after.to_string(),
                "nao"
            );
        }
    }

    let final_pos = nav.position();
    let dist_to_base = (final_pos[0] * final_pos[0] + final_pos[1] * final_pos[1]).sqrt();

    println!("\n=== RESULTADO FINAL ===");
    println!("posicao final: N={:.3}m E={:.3}m", final_pos[0], final_pos[1]);
    println!("distancia a base: {:.3}m", dist_to_base);

    if dist_to_base < 1.0 {
        println!("\nOK: Passo 7B — missao composta concluida.");
    } else {
        println!("\nDistancia a base = {:.3}m", dist_to_base);
    }
}
