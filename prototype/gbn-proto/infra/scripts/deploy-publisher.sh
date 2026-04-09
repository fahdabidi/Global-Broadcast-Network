#!/usr/bin/env bash
# deploy-publisher.sh — Deploy the receiver binary to the Publisher EC2 instance via SSM.
#
# Usage: ./deploy-publisher.sh <stack-name> [region]

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

cf_output() {
  local key="$1"
  aws cloudformation describe-stacks --stack-name "$STACK_NAME" --region "$REGION" --output json | \
    python -c "import json,sys; d=json.load(sys.stdin); o=d['Stacks'][0].get('Outputs',[]); print(next((x['OutputValue'] for x in o if x.get('OutputKey')=='$key'), ''))"
}

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROTO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "============================================"
echo "  GBN Phase 1 — Deploy Publisher (SSM)"
echo "  Stack:  $STACK_NAME"
echo "  Region: $REGION"
echo "============================================"

echo "[1/4] Resolving stack outputs..."
ARTIFACT_BUCKET="$(cf_output ArtifactBucketName)"
PUBLISHER_INSTANCE_ID="$(cf_output PublisherInstanceId)"

echo "[2/4] Using binary from s3://$ARTIFACT_BUCKET/phase1-artifacts/gbn-proto"

echo "[3/4] Deploying artifacts to Publisher ($PUBLISHER_INSTANCE_ID) via SSM..."
COMMAND_ID=$(aws ssm send-command \
  --region "$REGION" \
  --instance-ids "$PUBLISHER_INSTANCE_ID" \
  --document-name "AWS-RunShellScript" \
  --comment "GBN Phase1 deploy publisher artifacts" \
  --parameters commands="[
    \"set -euo pipefail\",
      \"mkdir -p /home/ec2-user/gbn-proto/staging /home/ec2-user/gbn-proto/reassembled /home/ec2-user/gbn-proto/reassembled-s19\",
      \"aws s3 cp s3://$ARTIFACT_BUCKET/phase1-artifacts/gbn-proto /home/ec2-user/gbn-proto/gbn-proto\",
      \"chmod +x /home/ec2-user/gbn-proto/gbn-proto\",
      \"cd /home/ec2-user/gbn-proto\",
      \"if [ ! -f publisher.key ] || [ ! -f publisher.pub ]; then ./gbn-proto keygen; fi\"
  ]" \
  --query 'Command.CommandId' \
  --output text)
COMMAND_ID="$(echo "$COMMAND_ID" | grep -Eo '[A-Fa-f0-9-]{36}' | head -n1)"
if [ -z "$COMMAND_ID" ]; then
  echo "ERROR: Failed to parse SSM CommandId"
  exit 1
fi

echo "[4/4] Waiting for SSM command completion..."
aws ssm wait command-executed \
  --region "$REGION" \
  --command-id "$COMMAND_ID" \
  --instance-id "$PUBLISHER_INSTANCE_ID"

echo ""
echo "✅ Publisher deployment complete (SSM)."
echo "   Instance:    $PUBLISHER_INSTANCE_ID"
echo "   Binary:      $REMOTE_DIR/gbn-proto"
echo "   Staging dir: $REMOTE_DIR/staging/"
echo "   Output dir:  $REMOTE_DIR/reassembled/"
