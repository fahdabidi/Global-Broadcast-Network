#!/usr/bin/env bash
# deploy-relays.sh — Deploy relay binary/bootstrap script to all Relay EC2 instances via SSM.
#
# Usage: ./deploy-relays.sh <stack-name> [region]

set -euo pipefail
export AWS_PAGER=""

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
POLL_TIMEOUT_SECONDS="${POLL_TIMEOUT_SECONDS:-900}"
POLL_INTERVAL_SECONDS="${POLL_INTERVAL_SECONDS:-10}"

cf_output() {
  local key="$1"
  aws cloudformation describe-stacks --stack-name "$STACK_NAME" --region "$REGION" --output json | \
    python -c "import json,sys; d=json.load(sys.stdin); o=d['Stacks'][0].get('Outputs',[]); print(next((x['OutputValue'] for x in o if x.get('OutputKey')=='$key'), ''))"
}

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROTO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BOOTSTRAP_SCRIPT="$SCRIPT_DIR/bootstrap-relay.sh"

echo "============================================"
echo "  GBN Phase 1 — Deploy Relays (SSM)"
echo "  Stack:  $STACK_NAME"
echo "  Region: $REGION"
echo "============================================"

echo "[1/5] Resolving stack outputs..."
ARTIFACT_BUCKET="$(cf_output ArtifactBucketName)"

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

if [ ! -f "$BOOTSTRAP_SCRIPT" ]; then
  echo "ERROR: bootstrap script not found at $BOOTSTRAP_SCRIPT"
  exit 1
fi

echo "[2/5] Uploading relay artifacts to s3://$ARTIFACT_BUCKET/phase1-artifacts/..."
cat "$BOOTSTRAP_SCRIPT" | aws s3 cp - "s3://$ARTIFACT_BUCKET/phase1-artifacts/bootstrap-relay.sh" --region "$REGION"

DHT_SEED_IP="${RELAY_PRIVATE_IPS[0]}"

echo "[3/5] Deploying artifacts and starting relays via SSM..."
for i in "${!RELAY_INSTANCE_IDS[@]}"; do
  INSTANCE_ID="${RELAY_INSTANCE_IDS[$i]}"
  RELAY_IP="${RELAY_PRIVATE_IPS[$i]}"
  NUM=$((i + 1))
  PORT=$((9000 + i))
  SEED_ARG=""
  if [ "$i" -ne 0 ]; then
    SEED_ARG="--dht-seed $DHT_SEED_IP:9100"
  fi

  echo "  [Relay $NUM] instance=$INSTANCE_ID ip=$RELAY_IP port=$PORT"
  COMMAND_ID=$(aws ssm send-command \
    --region "$REGION" \
    --instance-ids "$INSTANCE_ID" \
    --document-name "AWS-RunShellScript" \
    --comment "GBN Phase1 deploy/start relay $NUM" \
    --parameters commands="[
      \"set -euo pipefail\",
      \"mkdir -p /home/ec2-user/gbn-proto\",
      \"aws s3 cp s3://$ARTIFACT_BUCKET/phase1-artifacts/gbn-proto /home/ec2-user/gbn-proto/gbn-proto\",
      \"chmod +x /home/ec2-user/gbn-proto/gbn-proto\",
      \"aws s3 cp s3://$ARTIFACT_BUCKET/phase1-artifacts/bootstrap-relay.sh /home/ec2-user/gbn-proto/bootstrap-relay.sh\",
      \"chmod +x /home/ec2-user/gbn-proto/bootstrap-relay.sh\",
      \"bash /home/ec2-user/gbn-proto/bootstrap-relay.sh\",
      \"pkill -f 'gbn-proto onion-relay' || true\",
      \"nohup /home/ec2-user/gbn-proto/gbn-proto onion-relay --identity /home/ec2-user/gbn-proto/identity/identity.key --listen 0.0.0.0:$PORT --dht-listen 0.0.0.0:9100 $SEED_ARG > /tmp/relay-$PORT.log 2>&1 &\"
    ]" \
    --query 'Command.CommandId' \
    --output text)
  COMMAND_ID="$(echo "$COMMAND_ID" | grep -Eo '[A-Fa-f0-9-]{36}' | head -n1)"
  if [ -z "$COMMAND_ID" ]; then
    echo "ERROR: Failed to parse SSM CommandId for relay $NUM"
    exit 1
  fi
  echo "    command-id=$COMMAND_ID"

  start_ts=$(date +%s)
  while true; do
    STATUS_RAW="$(aws ssm get-command-invocation \
      --region "$REGION" \
      --command-id "$COMMAND_ID" \
      --instance-id "$INSTANCE_ID" \
      --query 'Status' \
      --output text 2>/dev/null || true)"
    STATUS="$(printf '%s\n' "$STATUS_RAW" | tr -d '\r' | head -n1 | awk '{print $1}')"

    case "$STATUS" in
      Success)
        break
        ;;
      Failed|Cancelled|TimedOut|Cancelling)
        echo "ERROR: Relay $NUM SSM command failed with status: $STATUS"
        aws ssm get-command-invocation \
          --region "$REGION" \
          --command-id "$COMMAND_ID" \
          --instance-id "$INSTANCE_ID" \
          --output json || true
        exit 1
        ;;
      Pending|InProgress|Delayed|"")
        now_ts=$(date +%s)
        elapsed=$((now_ts - start_ts))
        if [ "$elapsed" -ge "$POLL_TIMEOUT_SECONDS" ]; then
          echo "ERROR: Timed out waiting for relay $NUM after ${elapsed}s"
          aws ssm get-command-invocation \
            --region "$REGION" \
            --command-id "$COMMAND_ID" \
            --instance-id "$INSTANCE_ID" \
            --output json || true
          exit 1
        fi
        sleep "$POLL_INTERVAL_SECONDS"
        ;;
      *)
        echo "ERROR: Relay $NUM unknown SSM status: $STATUS"
        aws ssm get-command-invocation \
          --region "$REGION" \
          --command-id "$COMMAND_ID" \
          --instance-id "$INSTANCE_ID" \
          --output json || true
        exit 1
        ;;
    esac
  done
done

echo "[4/5] Persisting relay private-IP topology file locally..."
TOPOLOGY_FILE="$PROTO_ROOT/infra/scripts/.relay-topology"
{
  for i in "${!RELAY_PRIVATE_IPS[@]}"; do
    echo "${RELAY_PRIVATE_IPS[$i]}:$((9000 + i))"
  done
} > "$TOPOLOGY_FILE"

echo "[5/5] Relay deployment complete."

echo ""
echo "✅ All 4 relay instances deployed as Onion Relays (SSM)."
echo "   DHT seed private IP: $DHT_SEED_IP"
echo "   Local topology hint file: $TOPOLOGY_FILE"
