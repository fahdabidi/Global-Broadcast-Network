#!/usr/bin/env bash
# run-tests.sh — Execute the full Phase 1 test suite across the AWS infrastructure.
#
# Usage: ./run-tests.sh <creator-ip> <publisher-ip> <relay1-ip> <relay2-ip> <relay3-ip> <relay4-ip> [ssh-key-path]
#
# This script:
#   1. Starts relay processes on all relay instances
#   2. Starts the publisher receiver on the publisher instance
#   3. SSHs into the Creator instance and runs the full pipeline
#   4. Verifies SHA-256 match on the Publisher instance
#   5. Collects timing metrics and test results
#   6. Stops all relay and publisher processes

set -euo pipefail

if [ "$#" -lt 6 ]; then
    echo "Usage: $0 <creator-ip> <publisher-ip> <relay1-ip> <relay2-ip> <relay3-ip> <relay4-ip> [ssh-key-path]"
    exit 1
fi

CREATOR_IP="$1"
PUBLISHER_IP="$2"
RELAY1_IP="$3"
RELAY2_IP="$4"
RELAY3_IP="$5"
RELAY4_IP="$6"
SSH_KEY="${7:-~/.ssh/gbn-proto-key.pem}"
SSH_USER="ec2-user"
REMOTE_DIR="/home/$SSH_USER/gbn-proto"

SSH_OPTS="-i $SSH_KEY -o StrictHostKeyChecking=no"

echo "============================================"
echo "  GBN Phase 1 — Full Test Suite"
echo "  Creator:   $CREATOR_IP"
echo "  Publisher:  $PUBLISHER_IP"
echo "  Relays:     $RELAY1_IP, $RELAY2_IP, $RELAY3_IP, $RELAY4_IP"
echo "============================================"
echo ""

# ─────────────────────────── Step 1: Start Relays ───────────────────────────
echo "[Step 1/5] Starting relay processes..."

# Path 1: Creator → Relay1 → Relay2 → Publisher
ssh $SSH_OPTS "$SSH_USER@$RELAY1_IP" \
    "nohup $REMOTE_DIR/gbn-proto relay --listen 0.0.0.0:9000 --forward $RELAY2_IP:9000 > /tmp/relay.log 2>&1 &"
echo "  Relay 1 ($RELAY1_IP:9000) → Relay 2 ($RELAY2_IP:9000)"

ssh $SSH_OPTS "$SSH_USER@$RELAY2_IP" \
    "nohup $REMOTE_DIR/gbn-proto relay --listen 0.0.0.0:9000 --forward $PUBLISHER_IP:9000 > /tmp/relay.log 2>&1 &"
echo "  Relay 2 ($RELAY2_IP:9000) → Publisher ($PUBLISHER_IP:9000)"

# Path 2: Creator → Relay3 → Publisher
ssh $SSH_OPTS "$SSH_USER@$RELAY3_IP" \
    "nohup $REMOTE_DIR/gbn-proto relay --listen 0.0.0.0:9001 --forward $PUBLISHER_IP:9001 > /tmp/relay.log 2>&1 &"
echo "  Relay 3 ($RELAY3_IP:9001) → Publisher ($PUBLISHER_IP:9001)"

# Path 3: Creator → Relay4 → Publisher
ssh $SSH_OPTS "$SSH_USER@$RELAY4_IP" \
    "nohup $REMOTE_DIR/gbn-proto relay --listen 0.0.0.0:9002 --forward $PUBLISHER_IP:9002 > /tmp/relay.log 2>&1 &"
echo "  Relay 4 ($RELAY4_IP:9002) → Publisher ($PUBLISHER_IP:9002)"

sleep 2

# ─────────────────────────── Step 2: Start Publisher ───────────────────────────
echo ""
echo "[Step 2/5] Starting publisher receiver..."

ssh $SSH_OPTS "$SSH_USER@$PUBLISHER_IP" \
    "nohup $REMOTE_DIR/gbn-proto receive \
        --listen-ports 9000,9001,9002 \
        --output-dir $REMOTE_DIR/reassembled/ \
        > /tmp/publisher.log 2>&1 &"
echo "  Publisher listening on ports 9000, 9001, 9002"

sleep 2

# ─────────────────────────── Step 3: Run Pipeline ───────────────────────────
echo ""
echo "[Step 3/5] Executing MCN pipeline on Creator..."

ssh $SSH_OPTS "$SSH_USER@$CREATOR_IP" \
    "$REMOTE_DIR/gbn-proto upload \
        --input $REMOTE_DIR/test-vectors/*.mp4 \
        --paths $RELAY1_IP:9000,$RELAY3_IP:9001,$RELAY4_IP:9002 \
        --publisher-ip $PUBLISHER_IP \
        2>&1" | tee /tmp/gbn-phase1-results.log

# ─────────────────────────── Step 4: Verify ───────────────────────────
echo ""
echo "[Step 4/5] Verifying reassembly on Publisher..."

RESULT=$(ssh $SSH_OPTS "$SSH_USER@$PUBLISHER_IP" \
    "$REMOTE_DIR/gbn-proto verify \
        --reassembled $REMOTE_DIR/reassembled/*.mp4 \
        2>&1")

echo "$RESULT"

# ─────────────────────────── Step 5: Cleanup ───────────────────────────
echo ""
echo "[Step 5/5] Stopping all remote processes..."

for IP in "$RELAY1_IP" "$RELAY2_IP" "$RELAY3_IP" "$RELAY4_IP" "$PUBLISHER_IP"; do
    ssh $SSH_OPTS "$SSH_USER@$IP" "pkill -f gbn-proto || true" 2>/dev/null
done

echo ""
echo "============================================"
echo "  Phase 1 Test Suite Complete"
echo "  Results saved to: /tmp/gbn-phase1-results.log"
echo "============================================"
echo ""
echo "NEXT STEP: If tests passed, run teardown.sh to destroy the stack and stop billing."
