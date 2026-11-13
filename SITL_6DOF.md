# SITL 6-DOF — What Works, What Does Not

This document records the state of the 6-DOF SITL binary (`sitl-6dof`)
after the multi-waypoint experiment. It is the honest record of what was
validated and what requires further research.

## What Works (v5.3.0)

| Capability | Status | Evidence |
|---|---|---|
| 6-DOF physics (`rotor-rs`) | ✅ | Quaternion dynamics, coupled axes |
| Hover calibration | ✅ | 2.5274 N/rotor (bisection, 20 iters) |
| Cascaded controller (position→velocity→attitude) | ✅ | X error 0.04m, alt 0.05m |
| Geometric altitude compensation | ✅ | `tilt_comp` keeps z stable while tilting |
| MAVLink telemetry | ✅ | 297 packets in 30s (ATTITUDE 10Hz + HEARTBEAT 1Hz) |
| FDIR magnetometer health | ✅ | Outlier → Stuck → recovered (3 alerts) |
| Single waypoint flight | ✅ | `[30.04, 0, 5.05]` vs `[30, 0, 5]` |

## What Does NOT Work (v5.4.0 experiment, reverted)

| Capability | Status | Root Cause |
|---|---|---|
| Multi-waypoint A→B→C | ❌ | Linear PID cannot arrest coupled inertia at waypoint transitions |
| A* path planning in 6-DOF | ❌ | Not tested — multi-waypoint failed first |
| LNN-FTC in 6-DOF | ❌ | Not tested — depends on multi-waypoint |

### The Multi-Waypoint Failure (Honest Record)

The experiment used 3 waypoints (A[30,0], B[30,20], C[0,20]) with the
same cascaded controller that works for a single waypoint. Results:

- Waypoint 0 (A) reached at t=15.9s — OK
- Waypoint 1 (B) reached at t=19.9s — OK, but with large Y velocity
- Waypoint 2 (C) never reached — drone tumbled and diverged

Final position after 120s: `[-1636, 3555, -52545]` (total divergence).

The cause is **not a bug in the physics or the API**. The `AttitudeController`
in `rnfc-core` is a **linear PID**, tuned for 1D physics (the `main.rs` SITL)
and for a single fixed waypoint. At waypoint transitions, the drone has
significant velocity in the previous direction. A linear PID reacts to
position error, not to velocity in the coupled frame, and cannot arrest
the inertia without overshooting into a tumble.

### What Would Fix It

Multi-waypoint 6-DOF requires a **geometric controller** (SE(3) / geometric
tracking on SO(3)), which:

1. Tracks a velocity vector in the inertial frame
2. Computes the desired attitude from the desired force direction
3. Uses the rotation matrix (not Euler angles) for attitude error
4. Compensates for the coupling between roll, pitch, and yaw rates

This is a well-known result in the literature (Lee, Leok, McClamroch 2010;
Mellinger & Kumar 2011), but implementing it correctly is **research-grade**
work — weeks, not a session. The current `AttitudeController` is intentionally
simple: it is adequate for the 1D SITL and for a single waypoint in 6-DOF.

## Decision

The multi-waypoint experiment was **reverted**. The SITL 6-DOF binary is
back to the v5.3.0 state (single waypoint, 4cm error). This is the state
that is validated and that the README claims.

Future work on multi-waypoint 6-DOF is tracked as an open research question,
not as a bug. The honest claim is: **"single waypoint in 6-DOF, 4cm error"**,
not **"multi-waypoint mission in 6-DOF"**.

## How to Reproduce

```bash
cargo run -p rnfc-backend-sitl --bin sitl-6dof
```

Expected output:

```
Final pos:  [30.04, 0.00, 5.05]
X error:    0.04 m
Alt error:  0.05 m
MAVLink packets sent: 297
FDIR alerts:          3
```
---

## Update: Se3Control attempt (v5.4.0, reverted)

A 5th attempt used `rotor-rs::se3::Se3Control` (Lee 2010 geometric controller),
which is the correct theoretical solution. The integration failed:

```
Final pos: [544.12, 0.00, -68884.30]
Waypoints reached: 0/3
```

The drone climbed to 68 km altitude. The cause is a **scale mismatch** between
the `Se3Control` output and what `QuadrotorSim::step()` expects. The `rotor-rs`
tests pass, so the controller works in isolation — the integration is wrong.

This confirms that multi-waypoint 6-DOF is **research-grade**, not a session
of coding. The correct next step is to read the `rotor-rs` source to understand
the exact unit conventions of `Se3Control::update()`, not to try a 6th variant.

The multi-waypoint experiment is **closed**. Single waypoint is validated.

---

## Final Decision: Multi-Waypoint 6-DOF Closed (9 attempts)

After 9 attempts with 4 controller architectures, multi-waypoint 6-DOF
in the linear PID is declared **out of scope**. The record:

| Attempt | Architecture | Result |
|---|---|---|
| 1 | Waypoints simples | Divergiu wp1 |
| 2 | + Rate limiting | Divergiu wp1 |
| 3 | + Speed profile + PID altitude | Divergiu wp1 |
| 4 | A* grid | Divergiu wp1 |
| 5 | Se3Control (Lee 2010) | 68 km altitude |
| 6 | Se3Control + Z-fix | 489 km altitude |
| 7 | Se3Control + hover_cal | 2/3, divergiu wp2 |
| 8 | Se3Control + settle | 2/3, divergiu wp2 |
| 9 | PID + hold-in-place | 1/3, 280 m/s |

**Root cause:** the linear PID (and the `Se3Control` without trajectory
tracking) cannot arrest the coupled inertia during 90°+ waypoint
transitions in 6-DOF. The drone reaches wp0, starts turning, and the
coupled roll/pitch/yaw dynamics cause the controller to saturate.

**What works:**
- Single waypoint 6-DOF: 4cm error (v5.3.0)
- Multi-waypoint 1D: mission A→B→C (main.rs)

**What would fix it (research, 2-3 days):**
- Minimum-snap trajectory generation (Mellinger 2011)
- OR geometric controller with trajectory tracking (not waypoint jumps)

The multi-waypoint 6-DOF experiment is **closed**. The single waypoint
is the validated state of the 6-DOF SITL.

