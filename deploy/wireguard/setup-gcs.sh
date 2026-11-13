#!/usr/bin/env bash
# setup-gcs.sh — configure WireGuard on the ground station
set -euo pipefail

WG_DIR="/etc/wireguard"
GCS_IP="10.0.0.1"
DRONE_PUBLIC_KEY="${1:-}"

if [[ -z "$DRONE_PUBLIC_KEY" ]]; then
  echo "Usage: $0 <DRONE_PUBLIC_KEY>" >&2
  exit 1
fi

sudo apt update && sudo apt install -y wireguard
sudo mkdir -p "$WG_DIR" && sudo chmod 700 "$WG_DIR"

if [[ ! -f "$WG_DIR/gcs_private.key" ]]; then
  umask 077
  wg genkey | sudo tee "$WG_DIR/gcs_private.key" | wg pubkey | sudo tee "$WG_DIR/gcs_public.key"
  echo "GCS public key (give this to the drone):"
  sudo cat "$WG_DIR/gcs_public.key"
fi

GCS_PRIVATE=$(sudo cat "$WG_DIR/gcs_private.key")

sudo tee "$WG_DIR/wg0.conf" > /dev/null <<EOF
[Interface]
Address = ${GCS_IP}/24
PrivateKey = ${GCS_PRIVATE}
ListenPort = 51820

[Peer]
PublicKey = ${DRONE_PUBLIC_KEY}
AllowedIPs = 10.0.0.2/32
PersistentKeepalive = 25
EOF

sudo chmod 600 "$WG_DIR/wg0.conf"
sudo systemctl enable --now wg-quick@wg0
echo "[OK] WireGuard up. GCS IP: ${GCS_IP}"
echo "MAVLink GCS should listen on 10.0.0.1 (UDP 14550)."
