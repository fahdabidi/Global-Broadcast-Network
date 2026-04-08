#!/usr/bin/env bash
# deploy-relays.sh — Deploy the relay binary to all Relay EC2 instances.
#
# Usage: ./deploy-relays.sh <relay1-ip> <relay2-ip> <relay3-ip> <relay4-ip> [ssh-key-path]
#
# The relay binary is the same gbn-proto binary used in router-sim mode.
# Each relay will be started with: gbn-proto relay --listen <port> --forward <next-hop-ip:port>

set -euo pipefail

if [ "$#" -lt 4 ]; then
    echo "Usage: $0 <relay1-ip> <relay2-ip> <relay3-ip> <relay4-ip> [ssh-key-path]"
    exit 1
fi

RELAY1_IP="$1"
RELAY2_IP="$2"
RELAY3_IP="$3"
RELAY4_IP="$4"
SSH_KEY="${5:-~/.ssh/gbn-proto-key.pem}"
SSH_USER="ec2-user"
REMOTE_DIR="/home/$SSH_USER/gbn-proto"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROTO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BINARY="$PROTO_ROOT/target/x86_64-unknown-linux-gnu/release/gbn-proto"

echo "============================================"
echo "  GBN Phase 1 — Deploy Relays"
echo "============================================"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY"
    echo "Run deploy-creator.sh first (it builds the binary)."
    exit 1
fi

RELAY_IPS=("$RELAY1_IP" "$RELAY2_IP" "$RELAY3_IP" "$RELAY4_IP")

for i in "${!RELAY_IPS[@]}"; do
    IP="${RELAY_IPS[$i]}"
    NUM=$((i + 1))
    echo "[Relay $NUM] Deploying to $IP..."

    ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "$SSH_USER@$IP" \
        "mkdir -p $REMOTE_DIR"

    scp -i "$SSH_KEY" -o StrictHostKeyChecking=no \
        "$BINARY" \
        "$SSH_USER@$IP:$REMOTE_DIR/"

    echo "[Relay $NUM] ✅ Done."
done

echo ""
echo "✅ All 4 relay instances deployed."
echo "   Start relays with: gbn-proto relay --listen 9000 --forward <next-hop-ip>:9000"
