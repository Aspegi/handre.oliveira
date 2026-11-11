# RNFC — Resilient Navigation & Flight Control

A `no_std` Rust flight-control stack for multirotor UAVs, with layered FDIR,
learning-based actuator fault recovery, and automated safety-case generation.

**172 tests · 13 verification layers · 4 crates compile for Cortex-M4F**

---

## ⚠️ Honest status

This is a **software simulation and research prototype**. It is **not**:
- flight-tested on physical hardware
- certified by EASA or FAA
- a production flight controller

What it **is**:
- a working reference implementation of layered FDIR
- a demonstration of two-phase LNN training for motor-loss recovery
- an exploration of automated safety-case generation from runtime events
- a portfolio-quality Rust project with verifiable tests

If you need a flight controller for a physical drone **today**,
use [ArduPilot](https://ardupilot.org) or [PX4](https://px4.io).
This project is research-grade, not production-grade.

---

## What it does

| Layer | Crate | Function |
|---|---|---|
| Perception | `rnfc-core` | Kalman 6-state + attitude estimator + mag fusion |
| Control | `rnfc-core` | PID 3-axis attitude controller |
| Guidance | `rnfc-core` | Waypoint guidance + lookahead |
| Planning | `rnfc-daa` | A* over a 2D occupancy grid |
| FDIR (sensors) | `rnfc-fdir` | SensorHealth + Voter + Spoofing + Zombie + Failsafe |
| FDIR (actuators) | `rnfc-lnn` | Liquid Neural Network, two-phase training, 804 params |
| Certification | `rnfc-certis` | FTA + FMEA + GSN + hash chain from runtime events |
| Human authority | `rnfc-hase` | Pilot command filter with haptic feedback semantics |
| Swarm | `rnfc-swarm-certis` | Distributed consensus over hash chains |
| Ledger | `rnfc-certchain` | Proof-of-Safety chain + regulator verification |
| GPS-denied nav | `rnfc-magnav` | Grid-search over a synthetic magnetic anomaly map |

---

## Quick start

```bash
cargo build --release
cargo test --workspace --release
cargo run -p rnfc-backend-sitl --release
```

Expected: simulated drone flies A→B→C, publishes MAVLink telemetry on UDP 14550,
publishes 64x48 image frames on UDP 14551, generates `safety_case_demo.md`.

---

## Test coverage

| Layer | Count |
|---|---|
| Unit (core / fdir / hal / lnn) | 65 |
| Integration (mission A→B→C) | 1 |
| Chaos (combined adversities) | 4 |
| Monte Carlo (1000 random + 5000 evolved) | 2 |
| Fuzz (80,000 inputs) | 8 |
| Regression (9 historical bugs) | 9 |
| Research (CERTIS + HASE + Swarm + Chain + MagNav) | 80 |
| **Total** | **172** |

---

## What is really proven

- Layered FDIR isolates 1 sensor fault; 2-of-3 voting validated.
- LNN two-phase: hover (roll error < 3°) + motor-loss recovery (landed at 17.73s, roll error < 1°).
- 4 crates compile for `thumbv7em-none-eabihf` (Cortex-M4F) with `#![no_std]`.
- Regression tests prevent 9 historical bugs from returning.
- Adversarial Monte Carlo (5000 evolved) finds worst case 2.00m from target, no crashes.

## What is NOT proven

- No flight testing on real hardware.
- Physics model is 1D/2D, not 6-DOF.
- MagNav uses a synthetic 2D anomaly map, not a quantum magnetometer.
- CertChain uses FNV-1a, not SHA-256, and has no cryptographic signatures.
- CERTIS generates artifacts but has not been audited by any regulator.
- HASE haptic feedback is a data structure, not a physical force-feedback device.

---

## Architecture

```
Drone (Cortex-M4F, no_std, real-time):
  rnfc-abstractions  ← traits (Imu, Gnss, Magnetometer)
  rnfc-fdir          ← 5-layer fault detection
  rnfc-lnn           ← Liquid Neural Network FTC
  rnfc-hal           ← FlightController trait

Companion (Linux, std, offline):
  rnfc-core          ← Kalman, PID, Guidance
  rnfc-certis        ← Safety case generator
  rnfc-hase          ← Pilot authority envelope
  rnfc-swarm-certis  ← Distributed consensus
  rnfc-certchain     ← Proof-of-Safety ledger
  rnfc-magnav        ← GPS-denied navigation

Ground station (x86_64, std):
  rnfc-daa           ← A* path planning
  rnfc-mavlink       ← MAVLink encoder
```

This matches the ArduPilot/PX4 pattern: critical real-time logic on the MCU,
non-critical processing and certification on the companion/ground.

---

## Research directions

1. Implement the HAL for a real Pixhawk (STM32 HAL, I2C IMU drivers).
2. Replace 1D physics with a 6-DOF model (Brakiri/JSBSim).
3. Use a real magnetometer dataset for MagNav.
4. Replace FNV-1a with SHA-256 and add Ed25519 signatures to CertChain.
5. Write a formal safety case for CERTIS using GSN.

---

## License

Dual-licensed under MIT OR Apache-2.0. See `LICENSE-MIT`.

## Contributing

Portfolio/research artifact. Contributions welcome via issues and PRs,
but scope is intentionally narrow: this is not a general flight controller.
