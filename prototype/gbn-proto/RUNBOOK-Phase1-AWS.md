# GBN Phase 1 Prototype — AWS Test Runbook (SSM-Only)

This runbook deploys and validates the Phase 1 prototype using **AWS CLI + SSM only**.
No SSH, no key pair, and no public-IP based deployment steps are required.

---

## Prerequisites

Install/configure on your local machine:

| Tool | Required For |
|---|---|
| `aws` CLI | CloudFormation, EC2 metadata, SSM, S3 operations |
| `rustup` + Rust stable | Building Linux release binary |
| Linux target toolchain | `rustup target add x86_64-unknown-linux-gnu` |
| `ffmpeg` | Creating/processing test media |

You also need:
- AWS account with EC2 quota for the Phase 1 instances.
- SSM-enabled IAM permissions (CloudFormation/EC2/SSM/S3/IAM).

---

## Step 1 — Configure AWS CLI

```bash
aws configure
aws sts get-caller-identity
```

---

## Step 2 — Prepare Parameters

`infra/cloudformation/parameters.json` should include only instance sizes (no key pair / no deployer IP):

```json
[
  {"ParameterKey":"CreatorInstanceType","ParameterValue":"t3.small"},
  {"ParameterKey":"RelayInstanceType","ParameterValue":"t3.micro"},
  {"ParameterKey":"PublisherInstanceType","ParameterValue":"t3.small"}
]
```

---

## Step 3 — Create Stack

```bash
cd prototype/gbn-proto

aws cloudformation create-stack \
  --stack-name gbn-proto-phase1 \
  --template-body file://infra/cloudformation/phase1-stack.yaml \
  --parameters file://infra/cloudformation/parameters.json \
  --capabilities CAPABILITY_IAM \
  --region us-east-1

aws cloudformation wait stack-create-complete \
  --stack-name gbn-proto-phase1 \
  --region us-east-1
```

---

## Step 4 — Create a Dummy Test Video (Local)

Create a small local test artifact (already gitignored by `test-vectors/*.mp4`):

```bash
mkdir -p test-vectors
ffmpeg -f lavfi -i testsrc=size=1280x720:rate=30 -t 10 -pix_fmt yuv420p test-vectors/dummy-phase1.mp4
```

---

## Step 5 — Deploy Creator Artifacts via SSM

```bash
./infra/scripts/deploy-creator.sh gbn-proto-phase1 test-vectors/dummy-phase1.mp4 us-east-1
```

This builds `gbn-proto` locally, uploads binary+video to the stack artifact bucket, then uses `aws ssm send-command` to place them on the Creator instance.

---

## Step 6 — Deploy/Start Relays via SSM

```bash
./infra/scripts/deploy-relays.sh gbn-proto-phase1 us-east-1
```

This pushes relay artifacts via S3 and starts all 4 relays through SSM commands using private networking.

---

## Step 7 — Deploy Publisher via SSM

```bash
./infra/scripts/deploy-publisher.sh gbn-proto-phase1 us-east-1
```

---

## Step 8 — Run Full Test Suite via SSM

```bash
./infra/scripts/run-tests.sh gbn-proto-phase1 us-east-1
```

This performs:
- Relay pubkey collection via SSM
- Publisher startup
- Normal upload + verify
- S1.9 relay-failure simulation + verify
- Process cleanup

The local consolidated log is written to:

```bash
/tmp/gbn-phase1-results.log
```

---

## Step 9 — Teardown (Required)

```bash
./infra/cloudformation/teardown.sh
```

Or:

```bash
aws cloudformation delete-stack --stack-name gbn-proto-phase1 --region us-east-1
aws cloudformation wait stack-delete-complete --stack-name gbn-proto-phase1 --region us-east-1
```

---

## Troubleshooting (SSM Mode)

| Symptom | Likely Cause | Fix |
|---|---|---|
| `send-command` fails | Instance not SSM-registered yet | Wait 1-3 min after CREATE_COMPLETE and retry |
| S3 copy denied on instance | IAM policy mismatch | Verify instance role has `s3:GetObject` on artifact bucket |
| Script exits early | command invocation failed | Inspect `aws ssm get-command-invocation` stderr/stdout |
| Relay peer discovery issues | Seed relay not fully started | Re-run relay deployment or restart non-seed relays |
