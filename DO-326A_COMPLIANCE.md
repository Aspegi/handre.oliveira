# DO-326A / ED-202A — Airworthiness Security Compliance (Draft)

This document maps the RNFC security measures to the DO-326A process.
It is **a draft for internal use**, not a certified PSecAC.

## 1. Security Scope

### Assets to protect

| Asset | Impact if compromised |
|---|---|
| Flight control commands | Catastrophic (loss of control) |
| Position/navigation data | Catastrophic (controlled flight into terrain) |
| Safety event log (CertChain) | Major (loss of auditability) |
| Telemetry / imagery | Major (privacy, mission compromise) |

### External interfaces

| Interface | Protocol | Trust boundary |
|---|---|---|
| C2 link | MAVLink over UDP | Untrusted (radio) |
| GNSS | NMEA / UBX | Untrusted (jammable, spoofable) |
| Magnetometer | I2C | Untrusted (spoofable) |
| IMU | SPI | Untrusted (fault-injectable) |

## 2. Threat Assessment (summary)

| Threat | Severity | Mitigation | Evidence |
|---|---|---|---|
| Command injection (MAVLink) | Catastrophic | `rnfc-mavsign` | `sign_demo`: 4/4 rejected |
| Replay of old commands | Catastrophic | `rnfc-mavsign` ±10 s window | `sign_demo` |
| GNSS jamming | Hazardous | `rnfc-magnav` + FDIR | `gps_denied_navigation_succeeds` |
| GNSS spoofing | Catastrophic | SpoofingDetector (mag vs gyro) | 17 FDIR tests |
| Magnetometer spoofing | Hazardous | SpoofingDetector + FusionMag | chaos tests |
| Altitude envelope violation | Hazardous | `rnfc-alert` (3 channels) | `alert_demo` |
| Chain rewrite (local attacker) | Major | `rnfc-anchor` | `anchor_demo`: 3/3 rejected |
| Firmware replacement | Catastrophic | **NOT IMPLEMENTED** | — |
| Eavesdropping | Major | WireGuard (deployment) | `deploy/wireguard/` |

## 3. Security Requirements (derived)

| ID | Requirement | Status |
|---|---|---|
| SEC-01 | All MAVLink commands must be authenticated | ✅ `rnfc-mavsign` |
| SEC-02 | Replayed commands older than 10 s must be rejected | ✅ `rnfc-mavsign` |
| SEC-03 | The system must continue to navigate under GNSS denial | ✅ `rnfc-magnav` (simulated) |
| SEC-04 | Sensor spoofing must be detected within 1 s | ✅ `SpoofingDetector` |
| SEC-05 | The safety log must be tamper-evident | ✅ `rnfc-certchain` + `rnfc-anchor` |
| SEC-06 | Altitude envelope violations must trigger alerts on ≥2 channels | ✅ `rnfc-alert` (3 channels) |
| SEC-07 | Firmware must be verified before execution | ❌ Secure Boot (planned) |
| SEC-08 | C2 traffic must be confidential | ⚠️ WireGuard (deployment, not code) |

## 4. Verification Evidence

| Evidence | Location |
|---|---|
| Unit tests (262) | `cargo test --workspace` |
| Attack rejection demos | `examples/*_demo.rs` |
| Chaos tests (4) | `rnfc-backend-sitl/tests/chaos.rs` |
| Fuzz (80k inputs) | `rnfc-core/tests/fuzz.rs`, `rnfc-fdir/tests/fuzz.rs` |
| CI | `.github/workflows/ci.yml` |

## 5. Gaps and Next Steps

1. **Secure Boot (SEC-07)** — requires STM32F4 hardware; X-CUBE-SBSFU integration.
2. **Formal PSecAC** — must be agreed with EASA before certification.
3. **Penetration test** — no external audit has been performed.
4. **`unsafe` audit** — no `unsafe` blocks have been formally reviewed.
5. **Supply chain** — no SBOM or dependency signing.

## Disclaimer

This document is **not** a certification artefact. It is an internal draft
that maps the RNFC implementation to the DO-326A process. Certification
requires engagement with EASA/FAA and a formal PSecAC.
