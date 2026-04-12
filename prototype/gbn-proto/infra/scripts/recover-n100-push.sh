#!/usr/bin/env bash
set -euo pipefail
export AWS_PAGER=""
export DOCKER_CONFIG="/tmp/docker-ecr"

mkdir -p "$DOCKER_CONFIG"
cat > "$DOCKER_CONFIG/config.json" <<'JSON'
{"auths":{}}
JSON

REGION="us-east-1"
REGISTRY="138472308340.dkr.ecr.us-east-1.amazonaws.com"
REPO="gbn-proto-phase1-scale-n100/gbn-phase1"
ECR_URI="${REGISTRY}/${REPO}"
SHA="$(git rev-parse --short HEAD 2>/dev/null || echo local)"

aws ecr get-login-password --region "$REGION" | docker login --username AWS --password-stdin "$REGISTRY"

docker build --pull=false -t gbn-relay -f Dockerfile.relay .
docker tag gbn-relay "$ECR_URI/gbn-relay:$SHA"
docker tag gbn-relay "$ECR_URI/gbn-relay:latest"
docker push "$ECR_URI/gbn-relay:$SHA"
docker push "$ECR_URI/gbn-relay:latest"

docker build --pull=false -t gbn-publisher -f Dockerfile.publisher .
docker tag gbn-publisher "$ECR_URI/gbn-publisher:$SHA"
docker tag gbn-publisher "$ECR_URI/gbn-publisher:latest"
docker push "$ECR_URI/gbn-publisher:$SHA"
docker push "$ECR_URI/gbn-publisher:latest"

aws ecr list-images --repository-name "$REPO" --region "$REGION" --query 'imageIds' --output json --no-cli-pager
