#!/usr/bin/env bash
set -euo pipefail

STACK_NAME="${GBN_BRIDGE_STACK_NAME:-gbn-bridge-phase2-dev}"
REGION="${GBN_BRIDGE_AWS_REGION:-${AWS_REGION:-us-east-1}}"
WAIT="false"

usage() {
  cat <<USAGE
Usage: $0 [--stack-name NAME] [--region REGION] [--wait]

Delete the V2-only Conduit Phase 10 prototype stack.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --stack-name) STACK_NAME="$2"; shift 2 ;;
    --region) REGION="$2"; shift 2 ;;
    --wait) WAIT="true"; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ "$STACK_NAME" != gbn-bridge-phase2-* ]]; then
  echo "refusing to delete non-V2 stack name: $STACK_NAME" >&2
  exit 2
fi

command -v aws >/dev/null 2>&1 || {
  echo "required command not found: aws" >&2
  exit 127
}

aws cloudformation delete-stack --region "$REGION" --stack-name "$STACK_NAME"

if [[ "$WAIT" == "true" ]]; then
  aws cloudformation wait stack-delete-complete --region "$REGION" --stack-name "$STACK_NAME"
fi

echo "Delete requested for $STACK_NAME in $REGION"
