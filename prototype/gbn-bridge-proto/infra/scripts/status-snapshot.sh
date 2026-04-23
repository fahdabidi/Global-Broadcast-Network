#!/usr/bin/env bash
set -euo pipefail

STACK_NAME="${GBN_BRIDGE_STACK_NAME:-gbn-bridge-phase2-dev}"
REGION="${GBN_BRIDGE_AWS_REGION:-${AWS_REGION:-us-east-1}}"

usage() {
  cat <<USAGE
Usage: $0 [--stack-name NAME] [--region REGION]

Print a compact AWS status snapshot for the V2 Conduit prototype stack.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --stack-name) STACK_NAME="$2"; shift 2 ;;
    --region) REGION="$2"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

command -v aws >/dev/null 2>&1 || {
  echo "required command not found: aws" >&2
  exit 127
}

stack_output() {
  local key="$1"
  aws cloudformation describe-stacks \
    --region "$REGION" \
    --stack-name "$STACK_NAME" \
    --query "Stacks[0].Outputs[?OutputKey=='${key}'].OutputValue | [0]" \
    --output text
}

echo "Stack: $STACK_NAME"
aws cloudformation describe-stacks \
  --region "$REGION" \
  --stack-name "$STACK_NAME" \
  --query "Stacks[0].{Status:StackStatus,Updated:LastUpdatedTime,Created:CreationTime}" \
  --output table

CLUSTER="$(stack_output ClusterName)"
PUBLISHER_SERVICE="$(stack_output PublisherServiceName)"
BRIDGE_SERVICE="$(stack_output BridgeServiceName)"

echo "Cluster: $CLUSTER"
echo "Publisher service: $PUBLISHER_SERVICE"
echo "ExitBridge service: $BRIDGE_SERVICE"

aws ecs describe-services \
  --region "$REGION" \
  --cluster "$CLUSTER" \
  --services "$PUBLISHER_SERVICE" "$BRIDGE_SERVICE" \
  --query "services[].{Service:serviceName,Status:status,Desired:desiredCount,Running:runningCount,Pending:pendingCount,TaskDefinition:taskDefinition}" \
  --output table
