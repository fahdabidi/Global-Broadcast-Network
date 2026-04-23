#!/usr/bin/env bash
set -euo pipefail

STACK_NAME="${GBN_BRIDGE_STACK_NAME:-gbn-bridge-phase2-dev}"
REGION="${GBN_BRIDGE_AWS_REGION:-${AWS_REGION:-us-east-1}}"

usage() {
  cat <<USAGE
Usage: $0 [--stack-name NAME] [--region REGION]

Run the Phase 10 AWS smoke gate for the V2 prototype stack.

This script verifies that the V2-only stack is present, ECS services are
running, and task definitions carry the expected GBN_BRIDGE_* deployment
wiring for publisher authority, UDP punch port, and batch-window settings.
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

CLUSTER="$(stack_output ClusterName)"
PUBLISHER_SERVICE="$(stack_output PublisherServiceName)"
BRIDGE_SERVICE="$(stack_output BridgeServiceName)"

readarray -t COUNTS < <(
  aws ecs describe-services \
    --region "$REGION" \
    --cluster "$CLUSTER" \
    --services "$PUBLISHER_SERVICE" "$BRIDGE_SERVICE" \
    --query "services[].runningCount" \
    --output text | tr '\t' '\n'
)

for running in "${COUNTS[@]}"; do
  if [[ "$running" -lt 1 ]]; then
    echo "expected each V2 service to have at least one running task" >&2
    exit 1
  fi
done

readarray -t TASK_DEFS < <(
  aws ecs describe-services \
    --region "$REGION" \
    --cluster "$CLUSTER" \
    --services "$PUBLISHER_SERVICE" "$BRIDGE_SERVICE" \
    --query "services[].taskDefinition" \
    --output text | tr '\t' '\n'
)

for task_def in "${TASK_DEFS[@]}"; do
  env_names="$(aws ecs describe-task-definition \
    --region "$REGION" \
    --task-definition "$task_def" \
    --query "taskDefinition.containerDefinitions[].environment[].name" \
    --output text)"

  for required in GBN_BRIDGE_ROLE GBN_BRIDGE_STACK_NAME GBN_BRIDGE_PUNCH_PORT GBN_BRIDGE_BATCH_WINDOW_MS; do
    if [[ "$env_names" != *"$required"* ]]; then
      echo "task definition $task_def is missing $required" >&2
      exit 1
    fi
  done
done

echo "Phase 10 smoke gate passed for $STACK_NAME in $REGION"
echo "Note: this verifies AWS deployment wiring. Full live first-contact bootstrap remains a Phase 10 manual AWS scenario until a network listener replaces the current in-process prototype entrypoints."
