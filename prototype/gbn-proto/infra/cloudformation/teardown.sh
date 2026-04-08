#!/usr/bin/env bash
# teardown.sh — Destroy the Phase 1 CloudFormation stack and stop all billing.
#
# Usage: ./teardown.sh [stack-name]
#   Default stack name: gbn-proto-phase1

set -euo pipefail

STACK_NAME="${1:-gbn-proto-phase1}"
REGION="${AWS_DEFAULT_REGION:-us-east-1}"

echo "============================================"
echo "  GBN Phase 1 — Stack Teardown"
echo "  Stack:  $STACK_NAME"
echo "  Region: $REGION"
echo "============================================"
echo ""
echo "WARNING: This will DESTROY all instances and networking resources."
read -p "Are you sure? (yes/no): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    echo "Aborted."
    exit 1
fi

echo "Deleting stack $STACK_NAME..."
aws cloudformation delete-stack \
    --stack-name "$STACK_NAME" \
    --region "$REGION"

echo "Waiting for stack deletion to complete..."
aws cloudformation wait stack-delete-complete \
    --stack-name "$STACK_NAME" \
    --region "$REGION"

echo ""
echo "✅ Stack $STACK_NAME deleted. All resources destroyed. Billing stopped."
