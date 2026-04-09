#!/usr/bin/env bash
# deploy-creator.sh — Build locally, upload artifacts to S3, then deploy to Creator via AWS SSM.
#
# Usage: ./deploy-creator.sh <stack-name> <path-to-test-video> [region]

set -euo pipefail
export AWS_PAGER=""

if ! command -v aws >/dev/null 2>&1; then
  if command -v aws.exe >/dev/null 2>&1; then
    AWS_IS_EXE=1
    aws() { aws.exe "$@"; }
  else
    echo "ERROR: aws CLI not found in PATH (tried aws and aws.exe)."
    exit 1
  fi
fi

AWS_IS_EXE="${AWS_IS_EXE:-0}"

to_aws_local_path() {
  local p="$1"
  if [ "$AWS_IS_EXE" = "1" ] && command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$p"
  else
    echo "$p"
  fi
}

if ! command -v cargo >/dev/null 2>&1; then
  if command -v cargo.exe >/dev/null 2>&1; then
    cargo() { cargo.exe "$@"; }
  else
    echo "ERROR: cargo not found in PATH (tried cargo and cargo.exe)."
    exit 1
  fi
fi

STACK_NAME="${1:?Usage: $0 <stack-name> <path-to-test-video> [region]}"
TEST_VIDEO="${2:?Usage: $0 <stack-name> <path-to-test-video> [region]}"
REGION="${3:-us-east-1}"
POLL_TIMEOUT_SECONDS="${POLL_TIMEOUT_SECONDS:-3600}"
POLL_INTERVAL_SECONDS="${POLL_INTERVAL_SECONDS:-15}"

cf_output() {
  local key="$1"
  aws cloudformation describe-stacks --stack-name "$STACK_NAME" --region "$REGION" --output json | \
    python -c "import json,sys; d=json.load(sys.stdin); o=d['Stacks'][0].get('Outputs',[]); print(next((x['OutputValue'] for x in o if x.get('OutputKey')=='$key'), ''))"
}

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROTO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SRC_ARCHIVE="$PROTO_ROOT/source.tar.gz"
VIDEO_NAME="$(basename "$TEST_VIDEO")"

echo "============================================"
echo "  GBN Phase 1 — Deploy Creator (SSM)"
echo "  Stack:  $STACK_NAME"
echo "  Region: $REGION"
echo "  Video:  $TEST_VIDEO"
echo "============================================"

if [ ! -f "$TEST_VIDEO" ]; then
  echo "ERROR: Test video not found: $TEST_VIDEO"
  exit 1
fi

echo "[1/5] Resolving stack outputs..."
ARTIFACT_BUCKET="$(cf_output ArtifactBucketName)"
CREATOR_INSTANCE_ID="$(cf_output CreatorInstanceId)"

if [ -z "$ARTIFACT_BUCKET" ] || [ -z "$CREATOR_INSTANCE_ID" ]; then
  echo "ERROR: Missing required CloudFormation outputs (ArtifactBucketName/CreatorInstanceId)."
  exit 1
fi

echo "[2/5] Packaging source archive..."
cd "$PROTO_ROOT"
tar --exclude='./target' --exclude='./.git' -czf "$SRC_ARCHIVE" .

echo "[3/5] Uploading artifacts to s3://$ARTIFACT_BUCKET/phase1-artifacts/..."
cat "$SRC_ARCHIVE" | aws s3 cp - "s3://$ARTIFACT_BUCKET/phase1-artifacts/source.tar.gz" --region "$REGION"
cat "$TEST_VIDEO" | aws s3 cp - "s3://$ARTIFACT_BUCKET/phase1-artifacts/$VIDEO_NAME" --region "$REGION"

echo "[4/5] Running SSM command on Creator ($CREATOR_INSTANCE_ID)..."
COMMAND_ID=$(aws ssm send-command \
  --region "$REGION" \
  --instance-ids "$CREATOR_INSTANCE_ID" \
  --document-name "AWS-RunShellScript" \
  --comment "GBN Phase1 deploy creator artifacts" \
  --parameters commands="[
    \"set -euo pipefail\",
    \"rm -rf /tmp/gbn-build\",
    \"mkdir -p /home/ec2-user/gbn-proto/test-vectors /tmp/gbn-build\",
    \"aws s3 cp s3://$ARTIFACT_BUCKET/phase1-artifacts/source.tar.gz /tmp/source.tar.gz\",
    \"tar -xzf /tmp/source.tar.gz -C /tmp/gbn-build\",
    \"cd /tmp/gbn-build\",
    \"/root/.cargo/bin/cargo build --release\",
    \"cp target/release/gbn-proto /home/ec2-user/gbn-proto/gbn-proto\",
    \"chmod +x /home/ec2-user/gbn-proto/gbn-proto\",
    \"aws s3 cp /home/ec2-user/gbn-proto/gbn-proto s3://$ARTIFACT_BUCKET/phase1-artifacts/gbn-proto\",
    \"aws s3 cp s3://$ARTIFACT_BUCKET/phase1-artifacts/$VIDEO_NAME /home/ec2-user/gbn-proto/test-vectors/$VIDEO_NAME\"
  ]" \
  --query 'Command.CommandId' \
  --output text)
COMMAND_ID="$(echo "$COMMAND_ID" | grep -Eo '[A-Fa-f0-9-]{36}' | head -n1)"
if [ -z "$COMMAND_ID" ]; then
  echo "ERROR: Failed to parse SSM CommandId"
  exit 1
fi

echo "[5/5] Waiting for SSM command completion..."
start_ts=$(date +%s)
while true; do
  STATUS_RAW="$(aws ssm get-command-invocation \
    --region "$REGION" \
    --command-id "$COMMAND_ID" \
    --instance-id "$CREATOR_INSTANCE_ID" \
    --query 'Status' \
    --output text 2>/dev/null || true)"
  STATUS="$(printf '%s\n' "$STATUS_RAW" | tr -d '\r' | head -n1 | awk '{print $1}')"

  case "$STATUS" in
    Success)
      break
      ;;
    Failed|Cancelled|TimedOut|Cancelling)
      echo "ERROR: Creator SSM command ended with status: $STATUS"
      aws ssm get-command-invocation \
        --region "$REGION" \
        --command-id "$COMMAND_ID" \
        --instance-id "$CREATOR_INSTANCE_ID" \
        --output json || true
      exit 1
      ;;
    Pending|InProgress|Delayed|"")
      now_ts=$(date +%s)
      elapsed=$((now_ts - start_ts))
      if [ "$elapsed" -ge "$POLL_TIMEOUT_SECONDS" ]; then
        echo "ERROR: Timed out waiting for Creator SSM command after ${elapsed}s"
        aws ssm get-command-invocation \
          --region "$REGION" \
          --command-id "$COMMAND_ID" \
          --instance-id "$CREATOR_INSTANCE_ID" \
          --output json || true
        exit 1
      fi
      echo "  - status=$STATUS elapsed=${elapsed}s (polling every ${POLL_INTERVAL_SECONDS}s)"
      sleep "$POLL_INTERVAL_SECONDS"
      ;;
    *)
      echo "ERROR: Unknown SSM command status: $STATUS"
      aws ssm get-command-invocation \
        --region "$REGION" \
        --command-id "$COMMAND_ID" \
        --instance-id "$CREATOR_INSTANCE_ID" \
        --output json || true
      exit 1
      ;;
  esac
done

echo ""
echo "✅ Creator deployment complete (SSM)."
echo "   Instance: $CREATOR_INSTANCE_ID"
echo "   Binary:   /home/ec2-user/gbn-proto/gbn-proto"
echo "   Video:    /home/ec2-user/gbn-proto/test-vectors/$VIDEO_NAME"
