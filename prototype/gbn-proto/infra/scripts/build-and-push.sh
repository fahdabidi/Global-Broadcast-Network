#!/usr/bin/env bash
# build-and-push.sh — Build Docker images and push to Amazon ECR for GBN Phase 1 Scale Test.
#
# Usage: ./build-and-push.sh [stack-name] [region]
# From WSL Ubuntu: ./build-and-push.sh gbn-proto-phase1-scale-n100 us-east-1

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

if ! command -v docker >/dev/null 2>&1; then
  if command -v docker.exe >/dev/null 2>&1; then
    docker() { docker.exe "$@"; }
  elif command -v wsl.exe >/dev/null 2>&1 && wsl.exe -e docker version >/dev/null 2>&1; then
    # Running in Git Bash on Windows; Docker is available inside WSL2.
    # Re-execute the entire script inside WSL2 so Docker paths work natively.
    # Convert Git Bash absolute path (/c/...) to WSL path (/mnt/c/...) via sed.
    echo "[INFO] Docker not in PATH; re-running script inside WSL2..."
    WSL_SCRIPT="$(realpath "$0" | sed 's|^/\([a-zA-Z]\)/|/mnt/\1/|')"
    # MSYS_NO_PATHCONV=1 prevents Git Bash from mangling /mnt/c/... paths
    # before they reach wsl.exe (which would prepend Git Bash's install root).
    export MSYS_NO_PATHCONV=1
    exec wsl.exe -e bash "$WSL_SCRIPT" "$@"
  else
    echo "ERROR: docker not found in PATH (tried docker, docker.exe, and wsl docker)."
    exit 1
  fi
fi

STACK_NAME="${1:-gbn-proto-phase1-scale-n100}"
REGION="${2:-us-east-1}"

cf_output() {
  local key="$1"
  aws cloudformation describe-stacks --stack-name "$STACK_NAME" --region "$REGION" --output json | \
    python -c "import json,sys; d=json.load(sys.stdin); o=d['Stacks'][0].get('Outputs',[]); print(next((x['OutputValue'] for x in o if x.get('OutputKey')=='$key'), ''))"
}

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROTO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "============================================"
echo "  GBN Phase 1 — Build & Push to ECR"
echo "  Stack:  $STACK_NAME"
echo "  Region: $REGION"
echo "============================================"

echo "[1/4] Resolving stack outputs..."
ECR_URI_RELAY="$(cf_output ECRUriRelay)"
ECR_URI_PUBLISHER="$(cf_output ECRUriPublisher)"

if [ -z "$ECR_URI_RELAY" ] || [ -z "$ECR_URI_PUBLISHER" ]; then
  echo "ERROR: Missing CloudFormation outputs 'ECRUriRelay' and/or 'ECRUriPublisher'."
  exit 1
fi

echo "  Relay ECR Repository:     $ECR_URI_RELAY"
echo "  Publisher ECR Repository: $ECR_URI_PUBLISHER"

echo "[2/4] Determining git SHA..."
cd "$PROTO_ROOT"
if ! git rev-parse --short HEAD >/dev/null 2>&1; then
  echo "WARNING: Not a git repository, using 'local' as SHA."
  GIT_SHA="local"
else
  GIT_SHA="$(git rev-parse --short HEAD)"
fi
echo "  Git SHA: $GIT_SHA"

echo "[3/4] Building Docker images..."
# Build relay image (no ffmpeg)
docker build -t gbn-relay -f "$PROTO_ROOT/Dockerfile.relay" "$PROTO_ROOT"
docker tag gbn-relay "${ECR_URI_RELAY}:${GIT_SHA}"
docker tag gbn-relay "${ECR_URI_RELAY}:latest"

# Build publisher image (includes ffmpeg)
docker build -t gbn-publisher -f "$PROTO_ROOT/Dockerfile.publisher" "$PROTO_ROOT"
docker tag gbn-publisher "${ECR_URI_PUBLISHER}:${GIT_SHA}"
docker tag gbn-publisher "${ECR_URI_PUBLISHER}:latest"

echo "[4/4] Logging into ECR and pushing images..."
RELAY_REGISTRY="$(echo "$ECR_URI_RELAY" | cut -d'/' -f1)"
PUBLISHER_REGISTRY="$(echo "$ECR_URI_PUBLISHER" | cut -d'/' -f1)"
aws ecr get-login-password --region "$REGION" | docker login --username AWS --password-stdin "$RELAY_REGISTRY"
if [ "$PUBLISHER_REGISTRY" != "$RELAY_REGISTRY" ]; then
  aws ecr get-login-password --region "$REGION" | docker login --username AWS --password-stdin "$PUBLISHER_REGISTRY"
fi

echo "  Pushing relay:${GIT_SHA}"
docker push "${ECR_URI_RELAY}:${GIT_SHA}"
echo "  Pushing relay:latest"
docker push "${ECR_URI_RELAY}:latest"

echo "  Pushing publisher:${GIT_SHA}"
docker push "${ECR_URI_PUBLISHER}:${GIT_SHA}"
echo "  Pushing publisher:latest"
docker push "${ECR_URI_PUBLISHER}:latest"

echo ""
echo "✅ All images pushed successfully."
echo "   Relay ECR Repository:     $ECR_URI_RELAY"
echo "   Publisher ECR Repository: $ECR_URI_PUBLISHER"
echo "   Relay image:              ${ECR_URI_RELAY}:${GIT_SHA}"
echo "   Publisher image:          ${ECR_URI_PUBLISHER}:${GIT_SHA}"
echo ""
echo "To deploy the latest images, update your ECS Task Definitions to use:"
echo "  image: ${ECR_URI_RELAY}:latest      (or :${GIT_SHA} for pinning)"
echo "  image: ${ECR_URI_PUBLISHER}:latest"
