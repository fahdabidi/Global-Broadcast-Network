#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REGION="${GBN_BRIDGE_AWS_REGION:-${AWS_REGION:-us-east-1}}"
TAG="${GBN_BRIDGE_IMAGE_TAG:-latest}"
PUBLISHER_REPO="${GBN_BRIDGE_PUBLISHER_REPO:-gbn-bridge-proto-publisher}"
BRIDGE_REPO="${GBN_BRIDGE_EXIT_BRIDGE_REPO:-gbn-bridge-proto-exit-bridge}"

usage() {
  cat <<USAGE
Usage: $0 [--region REGION] [--tag TAG]

Build and push V2-only Conduit images to ECR.

Environment overrides:
  GBN_BRIDGE_AWS_REGION
  GBN_BRIDGE_IMAGE_TAG
  GBN_BRIDGE_PUBLISHER_REPO
  GBN_BRIDGE_EXIT_BRIDGE_REPO
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --region)
      REGION="$2"
      shift 2
      ;;
    --tag)
      TAG="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "required command not found: $1" >&2
    exit 127
  }
}

ensure_repo() {
  local repo="$1"
  if ! aws ecr describe-repositories --region "$REGION" --repository-names "$repo" >/dev/null 2>&1; then
    aws ecr create-repository \
      --region "$REGION" \
      --repository-name "$repo" \
      --image-scanning-configuration scanOnPush=true >/dev/null
  fi
}

require_cmd aws
require_cmd docker

ACCOUNT_ID="$(aws sts get-caller-identity --query Account --output text)"
REGISTRY="${ACCOUNT_ID}.dkr.ecr.${REGION}.amazonaws.com"
PUBLISHER_URI="${REGISTRY}/${PUBLISHER_REPO}:${TAG}"
BRIDGE_URI="${REGISTRY}/${BRIDGE_REPO}:${TAG}"

ensure_repo "$PUBLISHER_REPO"
ensure_repo "$BRIDGE_REPO"

aws ecr get-login-password --region "$REGION" \
  | docker login --username AWS --password-stdin "$REGISTRY"

docker build -f "$ROOT_DIR/Dockerfile.bridge-publisher" -t "$PUBLISHER_URI" "$ROOT_DIR"
docker build -f "$ROOT_DIR/Dockerfile.bridge" -t "$BRIDGE_URI" "$ROOT_DIR"

docker push "$PUBLISHER_URI"
docker push "$BRIDGE_URI"

cat <<IMAGES
PublisherImageUri=$PUBLISHER_URI
BridgeImageUri=$BRIDGE_URI
IMAGES
