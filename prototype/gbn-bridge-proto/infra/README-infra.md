# Veritas Conduit V2 Prototype Infrastructure

This directory contains the V2-only AWS prototype deployment assets for the
Conduit bridge-mode track. These files are intentionally isolated from the
frozen V1 Lattice workspace under `prototype/gbn-proto/`.

## Naming Rules

| Surface | Convention | Example |
|---|---|---|
| Environment variables | `GBN_BRIDGE_` | `GBN_BRIDGE_PUBLISHER_URL` |
| Container images | `gbn-bridge-proto-` | `gbn-bridge-proto-exit-bridge` |
| CloudFormation stacks | `gbn-bridge-phase2-` | `gbn-bridge-phase2-dev` |
| Metrics namespace | `GBN/BridgeProto` | `GBN/BridgeProto` |

## Assets

| Path | Purpose |
|---|---|
| `../Dockerfile.bridge` | builds the V2 ExitBridge deployment binary |
| `../Dockerfile.bridge-publisher` | builds the V2 Publisher Authority deployment binary |
| `cloudformation/phase2-bridge-stack.yaml` | deploys isolated ECS/Fargate publisher and ExitBridge services |
| `cloudformation/parameters.json` | example parameter file with placeholders |
| `scripts/build-and-push.sh` | builds and pushes V2 images to ECR |
| `scripts/deploy-bridge-test.sh` | deploys the V2 CloudFormation stack |
| `scripts/status-snapshot.sh` | prints stack and ECS service status |
| `scripts/bootstrap-smoke.sh` | verifies stack wiring and ECS task-definition environment |
| `scripts/mobile-validation.sh` | runs the Phase 11 local proxy or AWS/mobile validation workflow |
| `scripts/collect-bridge-metrics.sh` | collects ECS and CloudWatch evidence for a deployed Phase 10/11 stack |
| `scripts/relay-control-interactive-v2.sh` | small interactive wrapper around status, smoke, and teardown |
| `scripts/teardown-bridge-test.sh` | deletes only `gbn-bridge-phase2-*` stacks |

## Example Flow

```bash
cd prototype/gbn-bridge-proto

infra/scripts/build-and-push.sh \
  --region us-east-1 \
  --tag phase10

infra/scripts/deploy-bridge-test.sh \
  --region us-east-1 \
  --stack-name gbn-bridge-phase2-dev \
  --environment dev \
  --vpc-id vpc-REPLACE_ME \
  --subnet-ids subnet-REPLACE_ME_A,subnet-REPLACE_ME_B \
  --publisher-image ACCOUNT_ID.dkr.ecr.us-east-1.amazonaws.com/gbn-bridge-proto-publisher:phase10 \
  --bridge-image ACCOUNT_ID.dkr.ecr.us-east-1.amazonaws.com/gbn-bridge-proto-exit-bridge:phase10

infra/scripts/bootstrap-smoke.sh \
  --region us-east-1 \
  --stack-name gbn-bridge-phase2-dev

infra/scripts/mobile-validation.sh \
  --mode aws \
  --region us-east-1 \
  --stack-name gbn-bridge-phase2-dev
```

## Current Prototype Boundary

The Phase 10 deployment assets validate V2-only stack isolation, image naming,
ECS task wiring, `GBN_BRIDGE_*` environment variables, the default UDP punch
port, and the publisher batch-window setting.

The current binaries are deployment entrypoints for the in-process Conduit
prototype. They keep ECS tasks alive and expose validated configuration, but
they do not yet provide a production network service. Treat live AWS
first-contact bootstrap as a manual prototype scenario until the deployment
entrypoints are replaced by full network listeners.

## V1 Preservation

Do not modify or call V1 deployment files from this directory. In particular,
Phase 10 must not edit:

- `prototype/gbn-proto/infra/cloudformation/**`
- `prototype/gbn-proto/infra/scripts/**`
- `prototype/gbn-proto/Dockerfile.relay`
- `prototype/gbn-proto/Dockerfile.publisher`
