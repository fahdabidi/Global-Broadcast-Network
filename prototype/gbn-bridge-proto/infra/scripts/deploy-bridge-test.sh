#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TEMPLATE="$ROOT_DIR/infra/cloudformation/phase2-bridge-stack.yaml"
STACK_NAME="${GBN_BRIDGE_STACK_NAME:-gbn-bridge-phase2-dev}"
REGION="${GBN_BRIDGE_AWS_REGION:-${AWS_REGION:-us-east-1}}"
ENVIRONMENT_NAME="${GBN_BRIDGE_ENVIRONMENT:-dev}"
DESIRED_BRIDGE_COUNT="${GBN_BRIDGE_DESIRED_BRIDGE_COUNT:-3}"
PUBLISHER_HTTP_PORT="${GBN_BRIDGE_PUBLISHER_HTTP_PORT:-8080}"
UDP_PUNCH_PORT="${GBN_BRIDGE_PUNCH_PORT:-443}"
BATCH_WINDOW_MS="${GBN_BRIDGE_BATCH_WINDOW_MS:-500}"
PUBLISHER_ENDPOINT="${GBN_BRIDGE_PUBLISHER_URL:-http://publisher.veritas.local:8080}"

usage() {
  cat <<USAGE
Usage: $0 --vpc-id VPC --subnet-ids SUBNET_A,SUBNET_B --publisher-image URI --bridge-image URI [options]

Options:
  --stack-name NAME
  --region REGION
  --environment NAME
  --desired-bridge-count COUNT
  --publisher-http-port PORT
  --udp-punch-port PORT
  --batch-window-ms MS
  --publisher-endpoint URL
USAGE
}

VPC_ID=""
SUBNET_IDS=""
PUBLISHER_IMAGE_URI=""
BRIDGE_IMAGE_URI=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --stack-name) STACK_NAME="$2"; shift 2 ;;
    --region) REGION="$2"; shift 2 ;;
    --environment) ENVIRONMENT_NAME="$2"; shift 2 ;;
    --vpc-id) VPC_ID="$2"; shift 2 ;;
    --subnet-ids) SUBNET_IDS="$2"; shift 2 ;;
    --publisher-image) PUBLISHER_IMAGE_URI="$2"; shift 2 ;;
    --bridge-image) BRIDGE_IMAGE_URI="$2"; shift 2 ;;
    --desired-bridge-count) DESIRED_BRIDGE_COUNT="$2"; shift 2 ;;
    --publisher-http-port) PUBLISHER_HTTP_PORT="$2"; shift 2 ;;
    --udp-punch-port) UDP_PUNCH_PORT="$2"; shift 2 ;;
    --batch-window-ms) BATCH_WINDOW_MS="$2"; shift 2 ;;
    --publisher-endpoint) PUBLISHER_ENDPOINT="$2"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ "$STACK_NAME" != gbn-bridge-phase2-* ]]; then
  echo "stack name must start with gbn-bridge-phase2-: $STACK_NAME" >&2
  exit 2
fi

if [[ -z "$VPC_ID" || -z "$SUBNET_IDS" || -z "$PUBLISHER_IMAGE_URI" || -z "$BRIDGE_IMAGE_URI" ]]; then
  usage >&2
  exit 2
fi

command -v aws >/dev/null 2>&1 || {
  echo "required command not found: aws" >&2
  exit 127
}

aws cloudformation deploy \
  --region "$REGION" \
  --stack-name "$STACK_NAME" \
  --template-file "$TEMPLATE" \
  --capabilities CAPABILITY_NAMED_IAM \
  --parameter-overrides \
    EnvironmentName="$ENVIRONMENT_NAME" \
    VpcId="$VPC_ID" \
    PublicSubnetIds="$SUBNET_IDS" \
    PublisherImageUri="$PUBLISHER_IMAGE_URI" \
    BridgeImageUri="$BRIDGE_IMAGE_URI" \
    DesiredBridgeCount="$DESIRED_BRIDGE_COUNT" \
    PublisherHttpPort="$PUBLISHER_HTTP_PORT" \
    UdpPunchPort="$UDP_PUNCH_PORT" \
    BatchWindowMs="$BATCH_WINDOW_MS" \
    PublisherEndpoint="$PUBLISHER_ENDPOINT"

"$ROOT_DIR/infra/scripts/status-snapshot.sh" --stack-name "$STACK_NAME" --region "$REGION"
