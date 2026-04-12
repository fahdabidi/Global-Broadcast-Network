#!/usr/bin/env bash
# run-scaled-execution.sh — Execute Step 10 scale runs (N=100, N=500, N=1000) with rollback discipline.
#
# Usage:
#   ./run-scaled-execution.sh <stack-prefix> [region]
#
# Behavior:
# - Runs scales sequentially: 100 -> 500 -> 1000
# - For N=100, sets FREE_CHURN_RATE=0.0 (per plan constraint)
# - Uses deploy-scale-test.sh / run-chaos-upload.sh / teardown-scale-test.sh
# - If any scale fails, stops immediately (strict rerun discipline)

set -euo pipefail
export AWS_PAGER=""

STACK_PREFIX="${1:?Usage: $0 <stack-prefix> [region]}"
REGION="${2:-us-east-1}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUN_RESULTS_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)/results/scale-runs"
mkdir -p "$RUN_RESULTS_DIR"

SCALES=(100 500 1000)

timestamp() { date +%Y%m%d-%H%M%S; }

run_one_scale() {
  local scale="$1"
  local stack_name="${STACK_PREFIX}-n${scale}"
  local log_file="$RUN_RESULTS_DIR/${stack_name}-$(timestamp).log"

  echo "============================================" | tee -a "$log_file"
  echo "  Step 10 Execution — Scale N=${scale}" | tee -a "$log_file"
  echo "  Stack: ${stack_name}" | tee -a "$log_file"
  echo "  Region: ${REGION}" | tee -a "$log_file"
  echo "============================================" | tee -a "$log_file"

  echo "[A] Deploy + stabilize + scale-up" | tee -a "$log_file"
  "$SCRIPT_DIR/deploy-scale-test.sh" "$stack_name" "$scale" "$REGION" 2>&1 | tee -a "$log_file"

  if [ "$scale" -eq 100 ]; then
    echo "[B] N=100 special case: setting FREE_CHURN_RATE=0.0" | tee -a "$log_file"
    local chaos_lambda
    chaos_lambda="$(aws cloudformation describe-stack-resources \
      --stack-name "$stack_name" \
      --logical-resource-id ChaosControllerLambda \
      --region "$REGION" \
      --query 'StackResources[0].PhysicalResourceId' \
      --output text)"
    aws lambda update-function-configuration \
      --function-name "$chaos_lambda" \
      --region "$REGION" \
      --environment "Variables={FREE_CHURN_RATE=0.0,HOSTILE_CHURN_RATE=0.4,DELAY_SECONDS=30}" \
      >/dev/null
  fi

  echo "[C] Enable chaos + trigger upload" | tee -a "$log_file"
  "$SCRIPT_DIR/run-chaos-upload.sh" "$stack_name" "$REGION" "gbn-proto --help" 2>&1 | tee -a "$log_file"

  echo "[D] Teardown + metrics export" | tee -a "$log_file"
  "$SCRIPT_DIR/teardown-scale-test.sh" "$stack_name" "$REGION" 2>&1 | tee -a "$log_file"

  echo "✅ Scale N=${scale} run completed successfully." | tee -a "$log_file"
}

for scale in "${SCALES[@]}"; do
  if ! run_one_scale "$scale"; then
    echo "❌ Scale N=${scale} failed. Stop and fix root cause before proceeding to larger scale."
    exit 1
  fi
done

echo ""
echo "✅ Step 10 sequence complete: N=100 -> N=500 -> N=1000"
echo "Logs: $RUN_RESULTS_DIR"
