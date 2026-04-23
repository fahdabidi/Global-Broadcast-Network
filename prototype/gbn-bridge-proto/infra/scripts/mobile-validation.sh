#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODE="local"
STACK_NAME="${GBN_BRIDGE_STACK_NAME:-gbn-bridge-phase2-dev}"
REGION="${GBN_BRIDGE_AWS_REGION:-${AWS_REGION:-us-east-1}}"
TARGET_DIR="${VERITAS_BRIDGE_TARGET_DIR:-${TMPDIR:-/tmp}/veritas-bridge-target-phase11}"
WINDOW_MINUTES="${GBN_BRIDGE_METRICS_WINDOW_MINUTES:-15}"

usage() {
  cat <<USAGE
Usage: $0 [--mode local|aws] [--stack-name NAME] [--region REGION] [--target-dir DIR] [--window-minutes MINUTES]

Run the Phase 11 validation workflow.

Modes:
  local  Run the local Conduit harness scenarios that proxy mobile-behavior checks.
  aws    Run the AWS stack smoke gate plus metrics collection for a deployed Phase 10 stack.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode) MODE="$2"; shift 2 ;;
    --stack-name) STACK_NAME="$2"; shift 2 ;;
    --region) REGION="$2"; shift 2 ;;
    --target-dir) TARGET_DIR="$2"; shift 2 ;;
    --window-minutes) WINDOW_MINUTES="$2"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if command -v cargo >/dev/null 2>&1; then
  CARGO_BIN="cargo"
elif [[ -x "/mnt/c/Users/fahd_/.cargo/bin/cargo.exe" ]]; then
  CARGO_BIN="/mnt/c/Users/fahd_/.cargo/bin/cargo.exe"
else
  CARGO_BIN=""
fi

run_local() {
  if [[ -z "$CARGO_BIN" ]]; then
    echo "cargo not found in PATH and no Windows cargo.exe fallback found" >&2
    exit 127
  fi

  echo "==> Phase 11 local validation proxy"
  echo "Target dir: $TARGET_DIR"

  cd "$ROOT_DIR"
  CARGO_INCREMENTAL=0 "$CARGO_BIN" test --manifest-path "$ROOT_DIR/Cargo.toml" --test creator_bootstrap --target-dir "$TARGET_DIR"
  CARGO_INCREMENTAL=0 "$CARGO_BIN" test --manifest-path "$ROOT_DIR/Cargo.toml" --test data_path --target-dir "$TARGET_DIR"
  CARGO_INCREMENTAL=0 "$CARGO_BIN" test --manifest-path "$ROOT_DIR/Cargo.toml" --test integration --target-dir "$TARGET_DIR"
  CARGO_INCREMENTAL=0 "$CARGO_BIN" test --manifest-path "$ROOT_DIR/Cargo.toml" --test reachability --target-dir "$TARGET_DIR"

  cat <<SUMMARY

Local proxy summary:
- app restart / cached catalog recovery: creator_bootstrap
- first-contact bootstrap + punch ACK: creator_bootstrap + integration
- bridge failover and reuse after churn: data_path + integration
- reachability filtering and stale-entry handling: reachability + integration
SUMMARY
}

run_aws() {
  echo "==> Phase 11 AWS/mobile validation"
  "$ROOT_DIR/infra/scripts/status-snapshot.sh" --stack-name "$STACK_NAME" --region "$REGION"
  "$ROOT_DIR/infra/scripts/bootstrap-smoke.sh" --stack-name "$STACK_NAME" --region "$REGION"
  "$ROOT_DIR/infra/scripts/collect-bridge-metrics.sh" \
    --stack-name "$STACK_NAME" \
    --region "$REGION" \
    --window-minutes "$WINDOW_MINUTES"
}

case "$MODE" in
  local) run_local ;;
  aws) run_aws ;;
  *) echo "unsupported mode: $MODE" >&2; usage >&2; exit 2 ;;
esac
