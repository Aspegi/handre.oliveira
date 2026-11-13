# Threat Model — RNFC

This document contains the threat trees for the RNFC system, following
the DO-326A/ED-202A methodology. Each threat is decomposed into its
root causes and the mitigation that addresses each cause.

## Threat T-01: MAVLink Command Injection

**Goal of attacker:** send unauthorised commands to the drone.

```
T-01 Command Injection
├── T-01.1 No authentication on MAVLink
│   └── Mitigation: rnfc-mavsign (SHA-256 signature)
├── T-01.2 Weak key storage
│   └── Mitigation: key stored in companion (not MCU flash)
└── T-01.3 Key provisioning over untrusted channel
    └── Mitigation: SETUP_SIGNING via USB only (documented)
```

**Verification:** `sign_demo` — tampered packet → `BadSignature`.

## Threat T-02: MAVLink Replay

**Goal of attacker:** replay a previously captured valid command.

```
T-02 Replay
├── T-02.1 No timestamp in signature
│   └── Mitigation: rnfc-mavsign includes timestamp (6 bytes)
├── T-02.2 Timestamp window too wide
│   └── Mitigation: ±10s window (DEFAULT_WINDOW_US)
└── T-02.3 Clock manipulation
    └── Mitigation: FutureTimestamp rejection
```

**Verification:** `sign_demo` — 20s later → `ReplayTooOld`.

## Threat T-03: GNSS Jamming

**Goal of attacker:** deny GNSS, causing loss of navigation.

```
T-03 GNSS Jamming
├── T-03.1 Loss of position fix
│   └── Mitigation: rnfc-magnav (GPS-denied navigation)
├── T-03.2 Loss of velocity
│   └── Mitigation: Kalman prediction from IMU
└── T-03.3 Prolonged denial
    └── Mitigation: FDIR triggers RTL after 60s
```

**Verification:** `gps_denied_navigation_succeeds`, `chaos.rs` gnss_dropout.

## Threat T-04: GNSS Spoofing

**Goal of attacker:** feed false GNSS, causing controlled flight into terrain.

```
T-04 GNSS Spoofing
├── T-04.1 False position
│   └── Mitigation: SpoofingDetector (mag vs gyro divergence)
├── T-04.2 False velocity
│   └── Mitigation: Kalman innovation check
└── T-04.3 Coordinated spoofing
    └── Mitigation: (none — open research)
```

**Verification:** `chaos.rs` spoofing test.

## Threat T-05: Magnetometer Spoofing

**Goal of attacker:** feed false heading, causing wrong direction.

```
T-05 Magnetometer Spoofing
├── T-05.1 False yaw
│   └── Mitigation: FusionMag (gyro + mag fusion, α=0.95)
├── T-05.2 Sudden jump
│   └── Mitigation: SensorHealth (Outlier detection)
└── T-05.3 Stuck sensor
    └── Mitigation: SensorHealth (Stuck detection)
```

**Verification:** `sitl-6dof` — magnetometer stuck at t=40..42s → Outlier/Stuck.

## Threat T-06: Altitude Envelope Violation

**Goal of attacker:** force the drone above 120m (EASA limit).

```
T-06 Altitude Violation
├── T-06.1 Pilot command >120m
│   └── Mitigation: HASE (throttle_resistance)
├── T-06.2 Wind pushes above 120m
│   └── Mitigation: Failsafe (altitude correction)
└── T-06.3 No alert to operator
    └── Mitigation: rnfc-alert (3 channels)
```

**Verification:** `alert_demo` — 135m → Emergency → EmergencyLand.

## Threat T-07: Chain Rewrite

**Goal of attacker:** replace the entire safety log with a forged history.

```
T-07 Chain Rewrite
├── T-07.1 No external reference
│   └── Mitigation: rnfc-anchor (periodic external anchor)
├── T-07.2 Anchor key compromise
│   └── Mitigation: key rotation (planned)
└── T-07.3 Anchor service compromise
    └── Mitigation: (none — trust in external service)
```

**Verification:** `anchor_demo` — forged chain → `ChainRewritten`.

## Threat T-08: Chain Truncation

**Goal of attacker:** remove the last N blocks (hide a failure).

```
T-08 Chain Truncation
├── T-08.1 No block count check
│   └── Mitigation: rnfc-anchor (block_count comparison)
└── T-08.2 Anchor not checked
    └── Mitigation: verify_anchor at audit time
```

**Verification:** `anchor_demo` — truncated chain → `ChainShortened`.

## Threat T-09: Firmware Replacement

**Goal of attacker:** replace the firmware with a malicious version.

```
T-09 Firmware Replacement
├── T-09.1 No boot verification
│   └── Mitigation: Secure Boot (NOT IMPLEMENTED)
├── T-09.2 Unsigned firmware accepted
│   └── Mitigation: X-CUBE-SBSFU (planned)
└── T-09.3 Debug port open
    └── Mitigation: readout protection (planned)
```

**Status:** ❌ Not mitigated. Requires STM32F4 hardware.

## Threat T-10: Passive Radio Eavesdropping

**Goal of attacker:** read telemetry and commands without modifying.

```
T-10 Eavesdropping
├── T-10.1 Plaintext MAVLink
│   └── Mitigation: WireGuard (deployment)
├── T-10.2 Weak VPN cipher
│   └── Mitigation: WireGuard uses ChaCha20-Poly1305
└── T-10.3 Endpoint compromise
    └── Mitigation: (none — out of scope)
```

**Status:** ⚠️ Deployment-level (see `deploy/wireguard/`).

## Threat T-11: Insider (Malicious Operator)

**Goal of attacker:** an authorised operator sends commands to cause harm.

```
T-11 Insider
├── T-11.1 Operator has signing key
│   └── Mitigation: (none — trust in operator)
├── T-11.2 No audit of operator actions
│   └── Mitigation: HASE logs to CertChain
└── T-11.3 Operator bypasses HASE
    └── Mitigation: (none — HASE is advisory)
```

**Status:** ❌ Not fully addressed. Open research.

## Threat T-12: Supply Chain

**Goal of attacker:** compromise a dependency to inject malicious code.

```
T-12 Supply Chain
├── T-12.1 Unsigned dependencies
│   └── Mitigation: (none — no SBOM)
├── T-12.2 Typosquatting
│   └── Mitigation: (none)
└── T-12.3 Compromised maintainer
    └── Mitigation: (none)
```

**Status:** ❌ Not addressed. Requires SBOM + dependency signing.

---

*Last updated: v5.3.4*
