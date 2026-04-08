#!/usr/bin/env bash
# deploy-creator.sh — Build locally, then deploy the Creator binary and test video to the Creator EC2 instance.
#
# Usage: ./deploy-creator.sh <creator-public-ip> <path-to-test-video> [ssh-key-path]
#
# Prerequisites:
#   - Rust toolchain installed locally (cross-compile target: x86_64-unknown-linux-gnu)
#   - The CloudFormation stack is running and Creator instance is bootstrapped
#   - A user-provided test video file

set -euo pipefail

CREATOR_IP="${1:?Usage: $0 <creator-public-ip> <path-to-test-video> [ssh-key-path]}"
TEST_VIDEO="${2:?Usage: $0 <creator-public-ip> <path-to-test-video> [ssh-key-path]}"
SSH_KEY="${3:-~/.ssh/gbn-proto-key.pem}"
SSH_USER="ec2-user"
REMOTE_DIR="/home/$SSH_USER/gbn-proto"

echo "============================================"
echo "  GBN Phase 1 — Deploy Creator"
echo "  Target: $SSH_USER@$CREATOR_IP"
echo "  Video:  $TEST_VIDEO"
echo "============================================"

# Step 1: Build release binaries for Linux target
echo "[1/4] Building release binaries (target: x86_64-unknown-linux-gnu)..."
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROTO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROTO_ROOT"
cargo build --release --target x86_64-unknown-linux-gnu

# Step 2: Create remote directory
echo "[2/4] Creating remote directory..."
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "$SSH_USER@$CREATOR_IP" \
    "mkdir -p $REMOTE_DIR/test-vectors"

# Step 3: Upload binaries
echo "[3/4] Uploading binaries..."
scp -i "$SSH_KEY" -o StrictHostKeyChecking=no \
    "target/x86_64-unknown-linux-gnu/release/gbn-proto" \
    "$SSH_USER@$CREATOR_IP:$REMOTE_DIR/"

# Step 4: Upload test video
echo "[4/4] Uploading test video..."
scp -i "$SSH_KEY" -o StrictHostKeyChecking=no \
    "$TEST_VIDEO" \
    "$SSH_USER@$CREATOR_IP:$REMOTE_DIR/test-vectors/"

echo ""
echo "✅ Creator deployment complete."
echo "   Binary: $REMOTE_DIR/gbn-proto"
echo "   Video:  $REMOTE_DIR/test-vectors/$(basename "$TEST_VIDEO")"
