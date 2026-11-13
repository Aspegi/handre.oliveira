# WireGuard — MAVLink Confidentiality

WireGuard encapsulates all IP traffic between the drone companion and the
ground station. MAVLink (UDP 14550), video, and telemetry are encrypted and
authenticated transparently — no Rust code changes required.

## Threat model

| Threat | Without WireGuard | With WireGuard |
|---|---|---|
| Passive radio eavesdropping | Telemetry visible in plaintext | Encrypted (ChaCha20-Poly1305) |
| MAVLink command injection | Blocked by `rnfc-mavsign` (signature) | Blocked by both signature + tunnel |
| Replay | Blocked by `rnfc-mavsign` timestamp | Blocked twice (signature + WG nonce) |

## Topology

```
Drone companion (10.0.0.2) <--- WireGuard tunnel ---> Ground station (10.0.0.1)
```

## One-time setup

1. On the ground station, run `setup-gcs.sh <DRONE_PUBLIC_KEY>`.
   It prints the GCS public key — note it.
2. On the drone companion, run `setup-drone.sh <GCS_PUBLIC_IP> <GCS_PUBLIC_KEY>`.
   It prints the drone public key — this must match what the GCS expects.
3. Point `rnfc-mavlink` at `10.0.0.1` (the GCS tunnel IP) instead of the public address.

## Verification

```bash
sudo wg show                     # on either side — should list a peer with a recent handshake
sudo tcpdump -i any udp port 14550  # on a third machine — should see only encrypted WG traffic
```

## What this does NOT solve

- **Endpoint compromise**: if the GCS or drone is rooted, WireGuard does not help.
- **Insider threats**: a malicious operator with the private key can still send valid commands.
- **Root of trust**: Secure Boot (see `SECURITY.md`) is still required to prevent firmware replacement.

## Files

- `drone-wg0.conf.template` — reference config for the drone
- `gcs-wg0.conf.template` — reference config for the GCS
- `setup-drone.sh` — one-shot setup on the companion
- `setup-gcs.sh` — one-shot setup on the GCS
