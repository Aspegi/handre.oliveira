# DO-326A / ED-202A — Airworthiness Security Compliance

**Status:** Internal draft. Not a certified PSecAC.

This document maps the RNFC implementation to the DO-326A (Airworthiness
Security Process Specification) and ED-202A (European equivalent) process.
It is the **Plan for Security Aspects of Certification (PSecAC) draft**.

## 1. Purpose and Scope

### 1.1 System

RNFC (Resilient Navigation & Flight Control) is a `no_std` Rust flight
control stack for multirotor UAVs. It includes:

- Flight control (Kalman, PID, guidance)
- FDIR (5 layers of fault detection)
- LNN-FTC (motor loss recovery)
- CertChain (safety event ledger)
- MAVLink telemetry (signed)
- 6-DOF physics simulation (rotor-rs)

### 1.2 Certification Scope

**IN scope:**
- The flight control software (all 17 crates)
- The MAVLink C2 link
- The CertChain safety ledger
- The safety case generation (CERTIS, SCC, CSR)

**OUT of scope:**
- The physical airframe and propulsion
- The GNSS receiver hardware
- The ground station hardware
- Operational procedures and pilot training

### 1.3 Applicable Documents

| Document | Status |
|---|---|
| DO-326A (2014) | Applicable |
| ED-202A (2014) | Applicable (EU) |
| DO-356A (2018) | Applicable (methods) |
| DO-178C (2011) | Not applicable (no DAL claim yet) |
| RTCA DO-160G | Not applicable (hardware) |

## 2. System Description

### 2.1 Architecture

```
Drone (Cortex-M4F, no_std):
  rnfc-abstractions  ← traits
  rnfc-fdir          ← 5-layer FDIR
  rnfc-lnn           ← LNN-FTC
  rnfc-hal           ← FlightController trait

Companion (Linux, std):
  rnfc-core          ← Kalman, PID, AltitudePid
  rnfc-certis        ← Safety case generator
  rnfc-hase          ← Pilot authority envelope
  rnfc-swarm-certis  ← Distributed consensus
  rnfc-certchain     ← Hash chain ledger
  rnfc-magnav        ← GPS-denied navigation
  rnfc-omni          ← 360° perception
  rnfc-counterfactual← Post-mission what-if
  rnfc-scc           ← Safety case compiler
  rnfc-decision      ← Multi-objective route selection
  rnfc-watch         ← Motion detection + zoom
  rnfc-mavsign       ← MAVLink signing
  rnfc-anchor        ← External anchoring
  rnfc-alert         ← Altitude envelope alerts

Ground station (x86_64, std):
  rnfc-daa           ← A* path planning
  rnfc-mavlink       ← MAVLink encoder
```

### 2.2 Data Flows

| Flow | Source | Destination | Protocol |
|---|---|---|---|
| C2 commands | Ground station | Drone | MAVLink/UDP |
| Telemetry | Drone | Ground station | MAVLink/UDP |
| Sensor data | Sensors | Flight control | I2C/SPI |
| Safety events | Flight control | CertChain | In-process |

## 3. Security Scope Definition

### 3.1 Assets

| Asset | Impact if compromised | Classification |
|---|---|---|
| Flight control commands | Catastrophic (loss of control) | Critical |
| Position/navigation data | Catastrophic (CFIT) | Critical |
| Safety event log (CertChain) | Major (loss of auditability) | High |
| Telemetry / imagery | Major (privacy, mission) | Medium |
| Pilot commands (HASE) | Hazardous (authority bypass) | High |

### 3.2 External Interfaces

| Interface | Protocol | Trust boundary |
|---|---|---|
| C2 link | MAVLink/UDP | Untrusted (radio) |
| GNSS | NMEA/UBX | Untrusted (jammable, spoofable) |
| Magnetometer | I2C | Untrusted (spoofable) |
| IMU | SPI | Untrusted (fault-injectable) |
| Ground station | Ethernet | Untrusted (network) |

### 3.3 Trust Boundaries

The system has three trust levels:

1. **Trusted:** the MCU firmware (verified by Secure Boot — not yet implemented)
2. **Semi-trusted:** the companion computer (Linux)
3. **Untrusted:** everything reachable over RF or network

## 4. Threat Assessment

See `security/THREAT_MODEL.md` for the full threat trees.

### 4.1 Summary

| ID | Threat | Severity | Mitigation | Status |
|---|---|---|---|---|
| T-01 | MAVLink command injection | Catastrophic | `rnfc-mavsign` | ✅ Implemented |
| T-02 | MAVLink replay | Catastrophic | `rnfc-mavsign` ±10s window | ✅ Implemented |
| T-03 | GNSS jamming | Hazardous | `rnfc-magnav` + FDIR | ✅ Simulated |
| T-04 | GNSS spoofing | Catastrophic | SpoofingDetector | ✅ Implemented |
| T-05 | Magnetometer spoofing | Hazardous | SpoofingDetector + FusionMag | ✅ Implemented |
| T-06 | Altitude envelope violation | Hazardous | `rnfc-alert` (3 channels) | ✅ Implemented |
| T-07 | Chain rewrite (local) | Major | `rnfc-anchor` | ✅ Implemented |
| T-08 | Chain truncation | Major | `rnfc-anchor` | ✅ Implemented |
| T-09 | Firmware replacement | Catastrophic | Secure Boot | ❌ Not implemented |
| T-10 | Passive radio eavesdropping | Major | WireGuard | ⚠️ Deployment-only |
| T-11 | Insider (malicious operator) | Catastrophic | (none) | ❌ Not addressed |
| T-12 | Supply chain (dependency) | Hazardous | (none) | ❌ Not addressed |

## 5. Security Requirements

See `security/SECURITY_REQUIREMENTS.md` for the full traceability matrix.

### 5.1 Summary

| ID | Requirement | Status |
|---|---|---|
| SEC-01 | All MAVLink commands must be authenticated | ✅ `rnfc-mavsign` |
| SEC-02 | Replayed commands >10s must be rejected | ✅ `rnfc-mavsign` |
| SEC-03 | Navigate under GNSS denial | ✅ `rnfc-magnav` (sim) |
| SEC-04 | Detect sensor spoofing within 1s | ✅ `SpoofingDetector` |
| SEC-05 | Safety log must be tamper-evident | ✅ `rnfc-certchain` + `rnfc-anchor` |
| SEC-06 | Altitude violations → ≥2 alert channels | ✅ `rnfc-alert` (3) |
| SEC-07 | Firmware must be verified before execution | ❌ Secure Boot (planned) |
| SEC-08 | C2 traffic must be confidential | ⚠️ WireGuard (deploy) |
| SEC-09 | Insider threats must be detected | ❌ Not addressed |
| SEC-10 | Dependencies must be signed (SBOM) | ❌ Not addressed |

## 6. Security Architecture

### 6.1 Defence in Depth

```
Layer 1: no_std Rust         → memory safety
Layer 2: FDIR                → sensor fault detection
Layer 3: MAVLink signing     → command authentication
Layer 4: WireGuard           → confidentiality (deploy)
Layer 5: CertChain + Anchor  → tamper-evident audit
Layer 6: Secure Boot         → firmware integrity (planned)
```

Each layer is independent. A failure in one does not compromise the others.

### 6.2 What Is NOT Protected

- **Insider threats:** an operator with the MAVLink signing key can send valid commands.
- **Supply chain:** dependencies are not signed or verified.
- **Physical access:** without Secure Boot, firmware can be replaced.
- **Side channels:** timing and power analysis are not addressed.

## 7. Verification Evidence

| Evidence | Location |
|---|---|
| Unit tests (271) | `cargo test --workspace` |
| Attack rejection demos | `examples/*_demo.rs` |
| Chaos tests (4) | `rnfc-backend-sitl/tests/chaos.rs` |
| Fuzz (80k inputs) | `rnfc-core/tests/fuzz.rs`, `rnfc-fdir/tests/fuzz.rs` |
| CI (each push) | `.github/workflows/ci.yml` |
| Security demos | `sign_demo` (4/4), `anchor_demo` (3/3) |

### 7.1 Specific Threat Verification

| Threat | Test | Result |
|---|---|---|
| T-01 (injection) | `sign_demo` tampered packet | BadSignature ✅ |
| T-02 (replay) | `sign_demo` 20s later | ReplayTooOld ✅ |
| T-04 (GNSS spoof) | `chaos.rs` spoofing | FDIR detects ✅ |
| T-07 (chain rewrite) | `anchor_demo` forged chain | ChainRewritten ✅ |
| T-08 (chain truncate) | `anchor_demo` shortened | ChainShortened ✅ |

## 8. Continued Airworthiness

| Process | Status |
|---|---|
| Security monitoring in service | ❌ Not defined |
| Vulnerability disclosure | ✅ GitHub private advisory |
| Patch management | ❌ Not defined |
| Security bulletins | ❌ Not defined |

## 9. Gaps and Next Steps

| ID | Gap | Effort |
|---|---|---|
| G-01 | Secure Boot (SEC-07) | STM32F4 + X-CUBE-SBSFU (1-2 weeks) |
| G-02 | Formal PSecAC | EASA engagement (weeks) |
| G-03 | Penetration test | External audit (weeks) |
| G-04 | `unsafe` audit | Code review (days) |
| G-05 | Supply chain (SBOM) | Tooling (days) |
| G-06 | Insider threat model | Analysis (days) |
| G-07 | Side-channel analysis | Research (weeks) |

## 10. Disclaimer

**This document is NOT a certification artefact.**

It is an internal draft that maps the RNFC implementation to the DO-326A
process. Certification requires:

1. A formal PSecAC agreed with EASA/FAA
2. A complete security risk assessment
3. Verification of all mitigations by an independent auditor
4. Resolution of all gaps (G-01 to G-07)

The current document demonstrates **security awareness and traceability**,
not compliance. It is the foundation for a future certification effort.

---

*Last updated: v5.3.4-multiwaypoint-final*
*Author: Andre Oliveira*
*License: Proprietary (see LICENSE)*
