#!/usr/bin/env bash
# deploy-publisher.sh — Deploy the receiver binary to the Publisher EC2 instance.
#
# Usage: ./deploy-publisher.sh <publisher-public-ip> [ssh-key-path]

set -euo pipefail

PUBLISHER_IP="${1:?Usage: $0 <publisher-public-ip> [ssh-key-path]}"
SSH_KEY="${2:-~/.ssh/gbn-proto-key.pem}"
SSH_USER="ec2-user"
REMOTE_DIR="/home/$SSH_USER/gbn-proto"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROTO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BINARY="$PROTO_ROOT/target/x86_64-unknown-linux-gnu/release/gbn-proto"

echo "============================================"
echo "  GBN Phase 1 — Deploy Publisher"
echo "  Target: $SSH_USER@$PUBLISHER_IP"
echo "============================================"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY"
    echo "Run deploy-creator.sh first (it builds the binary)."
    exit 1
fi

# Create remote directory and staging area
echo "[1/2] Creating remote directories..."
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "$SSH_USER@$PUBLISHER_IP" \
    "mkdir -p $REMOTE_DIR/staging $REMOTE_DIR/reassembled"

# Upload binary
echo "[2/2] Uploading binary..."
scp -i "$SSH_KEY" -o StrictHostKeyChecking=no \
    "$BINARY" \
    "$SSH_USER@$PUBLISHER_IP:$REMOTE_DIR/"

echo ""
echo "✅ Publisher deployment complete."
echo "   Binary:      $REMOTE_DIR/gbn-proto"
echo "   Staging dir: $REMOTE_DIR/staging/"
echo "   Output dir:  $REMOTE_DIR/reassembled/"
