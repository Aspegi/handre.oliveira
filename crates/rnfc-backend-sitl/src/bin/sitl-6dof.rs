//! SITL with 6-DOF physics — cascaded position/velocity/attitude control.
//! Run: cargo run -p rnfc-backend-sitl --bin sitl-6dof

use rnfc_core::attitude_controller::AttitudeController;
use rnfc_physics_6dof::QuadrotorSim;

fn main() {
    println!("=== SITL-6DOF: Cascaded Waypoint Flight ===\n");

    let mut sim = QuadrotorSim::new();
    let mut pid = AttitudeController::new();
    let hover: f32 = 2.5274;
    let dt: f32 = 0.01;
    let target_x: f32 = 30.0;
    let target_alt: f32 = 5.0;

    let mut t = 0.0_f32;
    let mut max_x = 0.0f32;

    while t < 30.0 {
        let pos = sim.position();
        let vel = sim.velocity();
        let euler = sim.euler();

        // === Cascaded position controller ===
        // 1. Position error -> desired velocity (P, clamped)
        let ex = target_x - pos[0];
        let desired_vx = (ex * 0.5).clamp(-3.0, 3.0);

        // 2. Velocity error -> desired pitch (P, clamped)
        let evx = desired_vx - vel[0];
        let des_pitch = (evx * 0.05).clamp(-0.12, 0.12);

        // === Attitude PID ===
        let setpoint = [0.0_f32, des_pitch, 0.0];
        let measured = [euler[0], euler[1], euler[2]];
        let cmd = pid.update(setpoint, measured, dt);
        let cmd = [cmd[0].clamp(-2.0, 2.0), cmd[1].clamp(-2.0, 2.0), cmd[2].clamp(-2.0, 2.0)];

        // === Altitude: geometric comp + PD ===
        let tilt_comp = 1.0 / (euler[0].cos() * euler[1].cos()).max(0.5);
        let alt_error = target_alt - pos[2];
        let alt_d = -vel[2];
        let alt_correction = (alt_error * 1.5 + alt_d * 0.8).clamp(-1.5, 1.5);
        let base = hover * tilt_comp + alt_correction;

        // === Mixer: torques -> 4 rotor thrusts ===
        let k: f32 = 0.03;
        let min_thrust: f32 = 0.5;
        let thrusts = [
            (base - k*cmd[0] - k*cmd[1] + k*cmd[2]).max(min_thrust) as f64,
            (base + k*cmd[0] - k*cmd[1] - k*cmd[2]).max(min_thrust) as f64,
            (base + k*cmd[0] + k*cmd[1] + k*cmd[2]).max(min_thrust) as f64,
            (base - k*cmd[0] + k*cmd[1] - k*cmd[2]).max(min_thrust) as f64,
        ];

        sim.step(thrusts, dt as f64);
        t += dt;
        if pos[0] > max_x { max_x = pos[0]; }

        if ((t * 100.0) as i32) % 100 == 0 {
            println!("[t={:5.2}s] pos=[{:6.2},{:6.2},{:6.2}] vel=[{:5.2},{:5.2},{:5.2}] pitch={:5.1} alt_err={:5.2}",
                t, pos[0], pos[1], pos[2], vel[0], vel[1], vel[2],
                euler[1].to_degrees(), target_alt - pos[2]);
        }
    }

    println!("\n=== Mission Complete ===");
    let pos = sim.position();
    println!("Final pos:  [{:.2}, {:.2}, {:.2}]", pos[0], pos[1], pos[2]);
    println!("Target:     [{:.1}, 0.0, {:.1}]", target_x, target_alt);
    println!("X error:    {:.2} m", (pos[0] - target_x).abs());
    println!("Alt error:  {:.2} m", (pos[2] - target_alt).abs());
    println!("Max X:      {:.2} m", max_x);
}
