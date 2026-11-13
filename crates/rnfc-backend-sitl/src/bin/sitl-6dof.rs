//! SITL-6DOF with MAVLink telemetry + FDIR magnetometer health.
//! Run: cargo run -p rnfc-backend-sitl --bin sitl-6dof

use rnfc_core::attitude_controller::AttitudeController;
use rnfc_fdir::{Health, SensorHealth, SpoofingDetector};
use rnfc_mavlink::MavlinkSender;
use rnfc_physics_6dof::QuadrotorSim;

fn main() {
    println!("=== SITL-6DOF: MAVLink + FDIR ===\n");

    let mut sim = QuadrotorSim::new();
    let mut pid = AttitudeController::new();
    let mut mav = MavlinkSender::new("127.0.0.1:14550", 1, 1).expect("MAVLink sender");
    let mut mag_health = SensorHealth::new(0.0);
    let mut spoofing = SpoofingDetector::new();

    println!("[MAVLINK] bound to 127.0.0.1:14550 (sys=1, comp=1)");
    println!("[FDIR]    SensorHealth + SpoofingDetector active");
    println!("[FAULT]   magnetometer STUCK at t=10..12s (simulated)\n");

    let hover: f32 = 2.5274;
    let dt: f32 = 0.01;
    let target_x: f32 = 30.0;
    let target_alt: f32 = 5.0;

    let mut t = 0.0_f32;
    let mut last_mav_t = -1.0_f32;
    let mut last_hb_t = -1.0_f32;
    let mut mav_packets = 0u32;
    let mut fdir_alerts = 0u32;
    let mut last_health = Health::Healthy;

    while t < 30.0 {
        let pos = sim.position();
        let vel = sim.velocity();
        let euler = sim.euler();

        // Cascaded control (same as validated version)
        let ex = target_x - pos[0];
        let desired_vx = (ex * 0.5).clamp(-3.0, 3.0);
        let evx = desired_vx - vel[0];
        let des_pitch = (evx * 0.05).clamp(-0.12, 0.12);
        let setpoint = [0.0_f32, des_pitch, 0.0];
        let measured = [euler[0], euler[1], euler[2]];
        let cmd = pid.update(setpoint, measured, dt);
        let cmd = [cmd[0].clamp(-2.0, 2.0), cmd[1].clamp(-2.0, 2.0), cmd[2].clamp(-2.0, 2.0)];

        let tilt_comp = 1.0 / (euler[0].cos() * euler[1].cos()).max(0.5);
        let alt_error = target_alt - pos[2];
        let alt_d = -vel[2];
        let alt_correction = (alt_error * 1.5 + alt_d * 0.8).clamp(-1.5, 1.5);
        let base = hover * tilt_comp + alt_correction;

        let k: f32 = 0.03;
        let min_thrust: f32 = 0.5;
        let thrusts = [
            (base - k*cmd[0] - k*cmd[1] + k*cmd[2]).max(min_thrust) as f64,
            (base + k*cmd[0] - k*cmd[1] - k*cmd[2]).max(min_thrust) as f64,
            (base + k*cmd[0] + k*cmd[1] + k*cmd[2]).max(min_thrust) as f64,
            (base - k*cmd[0] + k*cmd[1] - k*cmd[2]).max(min_thrust) as f64,
        ];
        sim.step(thrusts, dt as f64);

        // === MAVLink 10 Hz ATTITUDE ===
        if t - last_mav_t >= 0.1 {
            last_mav_t = t;
            let _ = mav.send_attitude(
                euler[0], euler[1], euler[2],
                0.0, 0.0, 0.0,
            );
            mav_packets += 1;
        }

        // === MAVLink 1 Hz HEARTBEAT ===
        if t - last_hb_t >= 1.0 {
            last_hb_t = t;
            let _ = mav.send_heartbeat();
        }

        // === FDIR: magnetometer health ===
        // Simulate a stuck sensor between t=10 and t=12
        let mag_reading = if t >= 10.0 && t <= 12.0 {
            euler[2] + 0.5 // significant jump (stuck/spoofed)
        } else {
            euler[2] + (t.sin() * 0.01) // healthy noise
        };

        let health = mag_health.update(mag_reading);
        let _ = spoofing.update(mag_reading, 0.0);

        if health != Health::Healthy && health != last_health {
            println!("[FDIR] t={:.2}s magnetometer health={:?}", t, health);
            last_health = health;
            fdir_alerts += 1;
        } else if health == Health::Healthy && last_health != Health::Healthy {
            println!("[FDIR] t={:.2}s magnetometer recovered", t);
            last_health = health;
        }

        t += dt;

        if ((t * 100.0) as i32) % 500 == 0 {
            println!("[t={:5.2}s] pos=[{:6.2},{:6.2},{:6.2}] health={:?}",
                t, pos[0], pos[1], pos[2], health);
        }
    }

    println!("\n=== Mission Complete ===");
    let pos = sim.position();
    println!("Final pos:  [{:.2}, {:.2}, {:.2}]", pos[0], pos[1], pos[2]);
    println!("X error:    {:.2} m", (pos[0] - target_x).abs());
    println!("Alt error:  {:.2} m", (pos[2] - target_alt).abs());
    println!("MAVLink packets sent: {}", mav_packets);
    println!("FDIR alerts:          {}", fdir_alerts);
}
