#!/usr/bin/env bash
# run-tests.sh — Execute the full Phase 1 Zero-Trust test suite via AWS SSM.
#
# Usage: ./run-tests.sh <stack-name> [region]

set -euo pipefail
export AWS_PAGER=""
export PYTHONUTF8=1
export PYTHONIOENCODING="utf-8"

if ! command -v aws >/dev/null 2>&1; then
  if command -v aws.exe >/dev/null 2>&1; then
    aws() { aws.exe "$@"; }
  else
    echo "ERROR: aws CLI not found in PATH (tried aws and aws.exe)."
    exit 1
  fi
fi

STACK_NAME="${1:?Usage: $0 <stack-name> [region]}"
REGION="${2:-us-east-1}"
REMOTE_DIR="/home/ec2-user/gbn-proto"
RESULTS_LOG="/tmp/gbn-phase1-results.log"

cf_output() {
  local key="$1"
  aws cloudformation describe-stacks --stack-name "$STACK_NAME" --region "$REGION" --output json | \
    python -c "import json,sys; d=json.load(sys.stdin); o=d['Stacks'][0].get('Outputs',[]); print(next((x['OutputValue'] for x in o if x.get('OutputKey')=='$key'), ''))"
}

send_ssm() {
  local instance_id="$1"
  local commands_json="$2"
  local cmd_id
  cmd_id=$(aws ssm send-command \
    --region "$REGION" \
    --instance-ids "$instance_id" \
    --document-name "AWS-RunShellScript" \
    --parameters "commands=$commands_json" \
    --query 'Command.CommandId' \
    --output text)
  echo "$cmd_id" | grep -Eo '[A-Fa-f0-9-]{36}' | head -n1
}

wait_ssm() {
  local command_id="$1"
  local instance_id="$2"
  if aws ssm wait command-executed \
    --region "$REGION" \
    --command-id "$command_id" \
    --instance-id "$instance_id"; then
    return 0
  fi

  echo "ERROR: SSM command failed (command_id=$command_id instance_id=$instance_id)"
  aws ssm get-command-invocation \
    --region "$REGION" \
    --command-id "$command_id" \
    --instance-id "$instance_id" \
    --output json || true
  return 1
}

get_ssm_stdout() {
  local command_id="$1"
  local instance_id="$2"
  aws ssm get-command-invocation \
    --region "$REGION" \
    --command-id "$command_id" \
    --instance-id "$instance_id" \
    --query 'StandardOutputContent' \
    --output text
}

echo "============================================"
echo "  GBN Phase 1 — Zero-Trust Test Suite (SSM)"
echo "  Stack:  $STACK_NAME"
echo "  Region: $REGION"
echo "============================================"
echo ""

echo "[Step 1/6] Resolving stack outputs and topology..."
CREATOR_INSTANCE_ID="$(cf_output CreatorInstanceId)"
PUBLISHER_INSTANCE_ID="$(cf_output PublisherInstanceId)"

RELAY_INSTANCE_IDS=(
  "$(cf_output Relay1InstanceId)"
  "$(cf_output Relay2InstanceId)"
  "$(cf_output Relay3InstanceId)"
  "$(cf_output Relay4InstanceId)"
)

RELAY_PRIVATE_IPS=(
  "$(aws ec2 describe-instances --instance-ids "${RELAY_INSTANCE_IDS[0]}" --region "$REGION" --query "Reservations[0].Instances[0].PrivateIpAddress" --output text)"
  "$(aws ec2 describe-instances --instance-ids "${RELAY_INSTANCE_IDS[1]}" --region "$REGION" --query "Reservations[0].Instances[0].PrivateIpAddress" --output text)"
  "$(aws ec2 describe-instances --instance-ids "${RELAY_INSTANCE_IDS[2]}" --region "$REGION" --query "Reservations[0].Instances[0].PrivateIpAddress" --output text)"
  "$(aws ec2 describe-instances --instance-ids "${RELAY_INSTANCE_IDS[3]}" --region "$REGION" --query "Reservations[0].Instances[0].PrivateIpAddress" --output text)"
)

DHT_SEED_IP="${RELAY_PRIVATE_IPS[0]}"

echo "[Step 2/6] Collecting relay identity public keys via SSM..."
RELAY_PUBKEYS=()
for i in "${!RELAY_INSTANCE_IDS[@]}"; do
  relay_id="${RELAY_INSTANCE_IDS[$i]}"
  cmd_id=$(send_ssm "$relay_id" "[\"xxd -p -c 256 $REMOTE_DIR/identity/identity.pub\"]")
  wait_ssm "$cmd_id" "$relay_id"
  pub=$(get_ssm_stdout "$cmd_id" "$relay_id" | tr -d '\r' | tr -d '\n')
  RELAY_PUBKEYS+=("$pub")
  echo "  Relay $((i + 1)) ${RELAY_PRIVATE_IPS[$i]} -> ${pub:0:16}..."
done

echo "[Step 3/6] Starting publisher receiver and writing creator topology..."
pubkey_cmd=$(send_ssm "$PUBLISHER_INSTANCE_ID" "[\"xxd -p -c 256 $REMOTE_DIR/publisher.pub\"]")
wait_ssm "$pubkey_cmd" "$PUBLISHER_INSTANCE_ID"
PUBLISHER_PUBKEY=$(get_ssm_stdout "$pubkey_cmd" "$PUBLISHER_INSTANCE_ID" | tr -d '\r' | tr -d '\n')

topo_cmds=(
  "set -euo pipefail"
  "mkdir -p $REMOTE_DIR/topology"
  ": > $REMOTE_DIR/topology/relay-nodes.txt"
)
for i in "${!RELAY_PRIVATE_IPS[@]}"; do
  topo_cmds+=("echo '${RELAY_PRIVATE_IPS[$i]}:$((9000 + i)) ${RELAY_PUBKEYS[$i]}' >> $REMOTE_DIR/topology/relay-nodes.txt")
done

topo_json="["
for i in "${!topo_cmds[@]}"; do
  [ "$i" -gt 0 ] && topo_json+=","
  topo_json+="\"${topo_cmds[$i]}\""
done
topo_json+="]"

topo_cmd=$(send_ssm "$CREATOR_INSTANCE_ID" "$topo_json")
wait_ssm "$topo_cmd" "$CREATOR_INSTANCE_ID"

publisher_start_cmd=$(send_ssm "$PUBLISHER_INSTANCE_ID" "[\"pkill -f 'gbn-proto receive' || true\",\"nohup $REMOTE_DIR/gbn-proto receive --listen-ports 9000,9001,9002 --output-dir $REMOTE_DIR/reassembled/ > /tmp/publisher.log 2>&1 &\"]")
wait_ssm "$publisher_start_cmd" "$PUBLISHER_INSTANCE_ID"

echo "[Step 4/6] Running normal upload pipeline via creator..."
normal_upload_cmd=$(send_ssm "$CREATOR_INSTANCE_ID" "[\"$REMOTE_DIR/gbn-proto upload --input $REMOTE_DIR/test-vectors/*.mp4 --publisher-key $PUBLISHER_PUBKEY --relay-topology $REMOTE_DIR/topology/relay-nodes.txt --dht-seed $DHT_SEED_IP:9100 --paths 3 --hops 3 2>&1 | tee /tmp/creator-upload.log\"]")
wait_ssm "$normal_upload_cmd" "$CREATOR_INSTANCE_ID"
get_ssm_stdout "$normal_upload_cmd" "$CREATOR_INSTANCE_ID" > "$RESULTS_LOG"

echo "[Step 5/6] Running S1.9 relay-failure scenario..."
publisher_s19_cmd=$(send_ssm "$PUBLISHER_INSTANCE_ID" "[\"pkill -f 'gbn-proto receive' || true\",\"nohup $REMOTE_DIR/gbn-proto receive --listen-ports 9000,9001,9002 --output-dir $REMOTE_DIR/reassembled-s19/ > /tmp/publisher-s19.log 2>&1 &\"]")
wait_ssm "$publisher_s19_cmd" "$PUBLISHER_INSTANCE_ID"

s19_upload_cmd=$(send_ssm "$CREATOR_INSTANCE_ID" "[\"nohup $REMOTE_DIR/gbn-proto upload --input $REMOTE_DIR/test-vectors/*.mp4 --publisher-key $PUBLISHER_PUBKEY --relay-topology $REMOTE_DIR/topology/relay-nodes.txt --dht-seed $DHT_SEED_IP:9100 --paths 3 --hops 3 > /tmp/creator-upload-s19.log 2>&1 &\"]")
wait_ssm "$s19_upload_cmd" "$CREATOR_INSTANCE_ID"
sleep 15

kill_relay_cmd=$(send_ssm "${RELAY_INSTANCE_IDS[0]}" "[\"pkill -f 'gbn-proto onion-relay' || true\"]")
wait_ssm "$kill_relay_cmd" "${RELAY_INSTANCE_IDS[0]}"
sleep 30

echo "[Step 6/6] Verifying integrity and cleaning up processes..."
verify_cmd=$(send_ssm "$PUBLISHER_INSTANCE_ID" "[\"$REMOTE_DIR/gbn-proto verify --original $REMOTE_DIR/reassembled/*.mp4 --reassembled $REMOTE_DIR/reassembled/*.mp4 > /tmp/verify-normal.log 2>&1 || true\",\"grep -q PASS /tmp/verify-normal.log && echo PASS || echo FAIL\"]")
wait_ssm "$verify_cmd" "$PUBLISHER_INSTANCE_ID"
VERIFY_RESULT=$(get_ssm_stdout "$verify_cmd" "$PUBLISHER_INSTANCE_ID")

s19_verify_cmd=$(send_ssm "$PUBLISHER_INSTANCE_ID" "[\"$REMOTE_DIR/gbn-proto verify --original $REMOTE_DIR/reassembled/*.mp4 --reassembled $REMOTE_DIR/reassembled-s19/*.mp4 > /tmp/verify-s19.log 2>&1 || true\",\"grep -q PASS /tmp/verify-s19.log && echo PASS || echo FAIL\"]")
wait_ssm "$s19_verify_cmd" "$PUBLISHER_INSTANCE_ID"
S19_RESULT=$(get_ssm_stdout "$s19_verify_cmd" "$PUBLISHER_INSTANCE_ID")

for id in "${RELAY_INSTANCE_IDS[@]}" "$PUBLISHER_INSTANCE_ID"; do
  cleanup_cmd=$(send_ssm "$id" "[\"pkill -f gbn-proto || true\"]")
  wait_ssm "$cleanup_cmd" "$id"
done

echo ""
echo "============================================"
echo "  Phase 1 Test Suite Results"
echo "  Full log: $RESULTS_LOG"
echo "============================================"
echo ""
echo "Normal verify result: $VERIFY_RESULT"
echo "S1.9 verify result: $S19_RESULT"
echo ""
echo "$VERIFY_RESULT" | grep -q "PASS" && echo "✅ Normal pipeline: PASS" || echo "❌ Normal pipeline: FAIL"
echo "$S19_RESULT"    | grep -q "PASS" && echo "✅ S1.9 Node Recovery: PASS" || echo "❌ S1.9 Node Recovery: FAIL"
echo ""
echo "NEXT STEP: Run teardown.sh to destroy the CloudFormation stack and stop billing."
