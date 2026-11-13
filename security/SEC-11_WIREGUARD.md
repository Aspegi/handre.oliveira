# SEC-11: MAVLink Confidentiality (WireGuard) — Formal Specification

**Status:** Mandatory deployment procedure. Not optional.

## Requirement

All MAVLink C2 traffic between the drone companion and the ground station
MUST be encapsulated in a WireGuard tunnel. Plaintext MAVLink is forbidden
in production.

## Why

Without WireGuard, an attacker with a radio receiver can read all telemetry
and commands. The `rnfc-mavsign` signature authenticates messages but does
not encrypt them. WireGuard provides confidentiality (ChaCha20-Poly1305).

## Deployment Procedure (Mandatory)

1. **Ground station**: run `deploy/wireguard/setup-gcs.sh <DRONE_PUBLIC_KEY>`
2. **Drone companion**: run `deploy/wireguard/setup-drone.sh <GCS_IP> <GCS_PUBLIC_KEY>`
3. **MAVLink config**: point `rnfc-mavlink` at `10.0.0.1` (GCS tunnel IP)
4. **Verify**: `sudo wg show` must show a peer with a recent handshake

## Verification (in CI)

The CI pipeline checks that `deploy/wireguard/` exists and contains:
- `setup-drone.sh`
- `setup-gcs.sh`
- `README.md`

A deployment without these is rejected by the release checklist.

## What This Closes

| Threat | Before | After |
|---|---|---|
| T-10 (passive eavesdropping) | ⚠️ Deployment-only | ✅ Mandatory procedure |

## What Remains

- Endpoint compromise (if GCS is rooted, WireGuard does not help)
- Insider threats (operator with keys)

*Last updated: v6.4.0*
