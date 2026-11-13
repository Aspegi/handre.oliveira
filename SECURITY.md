# Security

This document consolidates all security measures in RNFC, mapped to the
threats they mitigate and the tests that prove they work.

## Threat → Mitigation → Evidence

| Threat | Mitigation | Tests | Demo |
|---|---|---|---|
| MAVLink command injection | `rnfc-mavsign` (SHA-256 + anti-replay) | 11 | `sign_demo` (4/4 rejected) |
| MAVLink replay | `rnfc-mavsign` timestamp window (±10s) | included above | `sign_demo` |
| Chain rewrite (local attacker) | `rnfc-anchor` (external anchor) | 12 | `anchor_demo` (3/3 rejected) |
| Chain truncation | `rnfc-anchor` (block_count check) | included above | `anchor_demo` |
| Sensor spoofing | `rnfc-fdir` SpoofingDetector | 17 | SITL chaos tests |
| GNSS jamming | `rnfc-magnav` (GPS-denied navigation) | 8 | `gps_denied_navigation_succeeds` |
| Altitude envelope violation | `rnfc-alert` (3 channels) | 10 | `alert_demo` |
| Firmware replacement | **NOT IMPLEMENTED** — see Secure Boot below | — | — |
| Passive radio eavesdropping | **Deployment-level** — see WireGuard below | — | `deploy/wireguard/` |

## Layers

### 1. Memory safety (`no_std` Rust)

The four MCU crates (`rnfc-abstractions`, `rnfc-fdir`, `rnfc-lnn`, `rnfc-hal`)
compile with `#![no_std]` for `thumbv7em-none-eabihf` (Cortex-M4F). Rust
eliminates buffer overflows, use-after-free, and null derefs at compile time.

**Not covered:** `unsafe` blocks (none audited yet), stack overflow (needs MPU).

### 2. MAVLink authentication (`rnfc-mavsign`)

Implements MAVLink 2 signing: SHA-256 over `packet || key || timestamp || link_id`,
13-byte signature, anti-replay window of ±10 s.

**Proven by:** `sign_demo` — tampered packet, wrong key, replay, and future timestamp
are all rejected (`BadSignature` / `ReplayTooOld` / `FutureTimestamp`).

### 3. MAVLink confidentiality (WireGuard)

See `deploy/wireguard/`. WireGuard encapsulates all IP traffic; MAVLink is
encrypted with ChaCha20-Poly1305. No Rust changes.

**Not covered:** endpoint compromise, insider threats.

### 4. Chain integrity (`rnfc-certchain` + `rnfc-anchor`)

The CertChain is a SHA-256 hash chain. A naive edit to one block breaks
verification. A full rewrite (consistent history) is detected by the external
anchor (`rnfc-anchor`) — the recomputed anchor hash will not match the stored one.

**Proven by:** `anchor_demo` — rewritten and truncated chains are rejected.

### 5. Runtime fault detection (`rnfc-fdir`)

Five layers: SensorHealth, 2-of-3 Voter, SpoofingDetector, ZombieDetector, Failsafe.
Validated by 17 tests + 4 chaos tests + 80k fuzz inputs.

### 6. Secure Boot (NOT IMPLEMENTED)

Without a hardware root of trust, an attacker with physical access can replace
the firmware. The recommended solution for STM32F4 is **X-CUBE-SBSFU** from
STMicroelectronics — a secure bootloader that verifies the firmware signature
before execution.

**Status:** planned. Requires an STM32F4 board and 1–2 weeks of integration.

### 7. DO-326A process (DOCUMENTED, NOT CERTIFIED)

See `DO-326A_COMPLIANCE.md`. The technical evidence exists (CERTIS, CertChain,
CSR); the formal process with EASA/FAA has not been initiated.

## Out of scope

- Physical security (tamper-evident enclosures, anti-tamper mesh)
- Supply chain (signed dependencies, SBOM)
- Operator training and procedures

## Reporting a vulnerability

Open a private security advisory on GitHub. Do not file a public issue.
