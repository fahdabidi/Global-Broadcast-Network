#!/usr/bin/env bash
set -euo pipefail
export AWS_PAGER=""
export DOCKER_CONFIG="/tmp/docker-ecr"

mkdir -p "$DOCKER_CONFIG"
cat > "$DOCKER_CONFIG/config.json" <<'JSON'
{"auths":{}}
JSON

STACK_NAME="${1:-gbn-proto-phase1-scale-n100}"
REGION="${2:-us-east-1}"
SHA="$(git rev-parse --short HEAD 2>/dev/null || echo local)"

cf_output() {
  local key="$1"
  aws cloudformation describe-stacks --stack-name "$STACK_NAME" --region "$REGION" --output json | \
    python -c "import json,sys; d=json.load(sys.stdin); o=d['Stacks'][0].get('Outputs',[]); print(next((x['OutputValue'] for x in o if x.get('OutputKey')=='$key'), ''))"
}

ECR_URI_RELAY="$(cf_output ECRUriRelay)"
ECR_URI_PUBLISHER="$(cf_output ECRUriPublisher)"

if [ -z "$ECR_URI_RELAY" ] || [ -z "$ECR_URI_PUBLISHER" ]; then
  echo "ERROR: Missing CloudFormation outputs ECRUriRelay and/or ECRUriPublisher for stack '$STACK_NAME'."
  exit 1
fi

RELAY_REGISTRY="$(echo "$ECR_URI_RELAY" | cut -d'/' -f1)"
PUBLISHER_REGISTRY="$(echo "$ECR_URI_PUBLISHER" | cut -d'/' -f1)"

aws ecr get-login-password --region "$REGION" | docker login --username AWS --password-stdin "$RELAY_REGISTRY"
if [ "$PUBLISHER_REGISTRY" != "$RELAY_REGISTRY" ]; then
  aws ecr get-login-password --region "$REGION" | docker login --username AWS --password-stdin "$PUBLISHER_REGISTRY"
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROTO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

docker build --pull=false -t gbn-relay -f "$PROTO_ROOT/Dockerfile.relay" "$PROTO_ROOT"
docker tag gbn-relay "$ECR_URI_RELAY:$SHA"
docker tag gbn-relay "$ECR_URI_RELAY:latest"
docker push "$ECR_URI_RELAY:$SHA"
docker push "$ECR_URI_RELAY:latest"

docker build --pull=false -t gbn-publisher -f "$PROTO_ROOT/Dockerfile.publisher" "$PROTO_ROOT"
docker tag gbn-publisher "$ECR_URI_PUBLISHER:$SHA"
docker tag gbn-publisher "$ECR_URI_PUBLISHER:latest"
docker push "$ECR_URI_PUBLISHER:$SHA"
docker push "$ECR_URI_PUBLISHER:latest"

RELAY_REPO_NAME="${ECR_URI_RELAY##*/}"
PUBLISHER_REPO_NAME="${ECR_URI_PUBLISHER##*/}"
aws ecr list-images --repository-name "$RELAY_REPO_NAME" --region "$REGION" --query 'imageIds' --output json --no-cli-pager
aws ecr list-images --repository-name "$PUBLISHER_REPO_NAME" --region "$REGION" --query 'imageIds' --output json --no-cli-pager
