#!/usr/bin/env bash
# setup-drone.sh — configure WireGuard on the drone companion
set -euo pipefail

WG_DIR="/etc/wireguard"
DRONE_IP="10.0.0.2"
GCS_PUBLIC_IP="${1:-}"
GCS_PUBLIC_KEY="${2:-}"

if [[ -z "$GCS_PUBLIC_IP" || -z "$GCS_PUBLIC_KEY" ]]; then
  echo "Usage: $0 <GCS_PUBLIC_IP> <GCS_PUBLIC_KEY>" >&2
  exit 1
fi

sudo apt update && sudo apt install -y wireguard
sudo mkdir -p "$WG_DIR" && sudo chmod 700 "$WG_DIR"

# Generate drone keys if missing
if [[ ! -f "$WG_DIR/drone_private.key" ]]; then
  umask 077
  wg genkey | sudo tee "$WG_DIR/drone_private.key" | wg pubkey | sudo tee "$WG_DIR/drone_public.key"
  echo "Drone public key (give this to the GCS):"
  sudo cat "$WG_DIR/drone_public.key"
fi

DRONE_PRIVATE=$(sudo cat "$WG_DIR/drone_private.key")

sudo tee "$WG_DIR/wg0.conf" > /dev/null <<EOF
[Interface]
Address = ${DRONE_IP}/24
PrivateKey = ${DRONE_PRIVATE}
ListenPort = 51820

[Peer]
PublicKey = ${GCS_PUBLIC_KEY}
Endpoint = ${GCS_PUBLIC_IP}:51820
AllowedIPs = 10.0.0.1/32
PersistentKeepalive = 25
EOF

sudo chmod 600 "$WG_DIR/wg0.conf"
sudo systemctl enable --now wg-quick@wg0
echo "[OK] WireGuard up. Drone IP: ${DRONE_IP}"
echo "Point rnfc-mavlink at 10.0.0.1 (GCS tunnel IP)."
