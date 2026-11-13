# Security Requirements — Traceability Matrix

This matrix traces each security requirement from its originating threat
through the mitigation, the test that verifies it, and the evidence.

## Matrix

| Req ID | Requirement | Threat | Mitigation | Test | Evidence | Status |
|---|---|---|---|---|---|---|
| SEC-01 | All MAVLink commands must be authenticated | T-01 | `rnfc-mavsign` | `verify_accepts_valid` | `sign_demo` | ✅ |
| SEC-02 | Replayed commands >10s must be rejected | T-02 | `rnfc-mavsign` window | `verify_rejects_replay` | `sign_demo` | ✅ |
| SEC-03 | Navigate under GNSS denial | T-03 | `rnfc-magnav` | `gps_denied_navigation` | `chaos.rs` | ✅ (sim) |
| SEC-04 | Detect sensor spoofing within 1s | T-04, T-05 | `SpoofingDetector` | `chaos.rs` | `sitl-6dof` | ✅ |
| SEC-05 | Safety log must be tamper-evident | T-07 | `rnfc-certchain` | `reject_tampered` | `chain_tests` | ✅ |
| SEC-06 | Safety log must resist full rewrite | T-07 | `rnfc-anchor` | `verify_rejects_rewritten` | `anchor_demo` | ✅ |
| SEC-07 | Safety log must resist truncation | T-08 | `rnfc-anchor` | `verify_rejects_shortened` | `anchor_demo` | ✅ |
| SEC-08 | Altitude violations → ≥2 alert channels | T-06 | `rnfc-alert` | `exceeds_max_emits_critical` | `alert_demo` | ✅ |
| SEC-09 | Pilot commands must respect envelope | T-06 | `rnfc-hase` | HASE tests | SITL | ✅ |
| SEC-10 | Firmware must be verified before execution | T-09 | Secure Boot | (none) | (none) | ❌ |
| SEC-11 | C2 traffic must be confidential | T-10 | WireGuard | (none) | `deploy/wireguard/` | ⚠️ deploy |
| SEC-12 | Insider actions must be auditable | T-11 | HASE + CertChain | (partial) | (none) | ⚠️ partial |
| SEC-13 | Dependencies must be signed (SBOM) | T-12 | (none) | (none) | (none) | ❌ |

## Coverage

| Category | Count | Status |
|---|---|---|
| Total requirements | 13 | — |
| Implemented and tested | 9 | ✅ |
| Partial (deployment or partial) | 2 | ⚠️ |
| Not implemented | 2 | ❌ |

**Coverage: 9/13 fully, 11/13 with partial (69% → 85%).**

## Requirements Not Yet Addressed

### SEC-10: Secure Boot

- **Threat:** T-09 (firmware replacement)
- **Required:** STM32F4 + X-CUBE-SBSFU integration
- **Effort:** 1-2 weeks
- **Priority:** Critical (highest gap)

### SEC-13: SBOM

- **Threat:** T-12 (supply chain)
- **Required:** `cargo-audit`, `cargo-sbom`, signed dependency list
- **Effort:** 2-3 days
- **Priority:** Medium

## Partial Requirements

### SEC-11: WireGuard

- **Threat:** T-10 (eavesdropping)
- **Status:** scripts exist in `deploy/wireguard/`, but not part of the certified system
- **Required for certification:** mandatory deployment procedure

### SEC-12: Insider audit

- **Threat:** T-11 (insider)
- **Status:** HASE logs to CertChain, but no independent audit trail
- **Required:** separate audit log with independent signing key

## Verification Commands

```bash
# Full test suite (271 tests)
cargo test --workspace --no-fail-fast

# Attack rejection demos
cargo run -p rnfc-mavsign --example sign_demo       # 4/4 attacks rejected
cargo run -p rnfc-anchor --example anchor_demo       # 3/3 attacks rejected
cargo run -p rnfc-alert --example alert_demo         # 3 channels

# Chaos tests
cargo test -p rnfc-backend-sitl --test chaos

# Fuzz
cargo test -p rnfc-core --test fuzz
cargo test -p rnfc-fdir --test fuzz
```

---

*Last updated: v5.3.4*
