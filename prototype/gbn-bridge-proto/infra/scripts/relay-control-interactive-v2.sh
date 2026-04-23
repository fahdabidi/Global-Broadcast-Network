#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STACK_NAME="${GBN_BRIDGE_STACK_NAME:-gbn-bridge-phase2-dev}"
REGION="${GBN_BRIDGE_AWS_REGION:-${AWS_REGION:-us-east-1}}"

menu() {
  cat <<MENU
Veritas Conduit V2 Prototype Control

Stack:  $STACK_NAME
Region: $REGION

1) Status snapshot
2) Bootstrap smoke gate
3) Teardown stack
4) Quit
MENU
}

while true; do
  menu
  read -r -p "Choose an action: " choice
  case "$choice" in
    1)
      "$SCRIPT_DIR/status-snapshot.sh" --stack-name "$STACK_NAME" --region "$REGION"
      ;;
    2)
      "$SCRIPT_DIR/bootstrap-smoke.sh" --stack-name "$STACK_NAME" --region "$REGION"
      ;;
    3)
      read -r -p "Type the stack name to confirm deletion: " confirm
      if [[ "$confirm" == "$STACK_NAME" ]]; then
        "$SCRIPT_DIR/teardown-bridge-test.sh" --stack-name "$STACK_NAME" --region "$REGION" --wait
      else
        echo "confirmation mismatch; not deleting"
      fi
      ;;
    4|q|quit)
      exit 0
      ;;
    *)
      echo "unknown option: $choice" >&2
      ;;
  esac
done
