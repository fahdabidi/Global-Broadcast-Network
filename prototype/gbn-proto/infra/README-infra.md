# GBN Phase 1 — AWS Infrastructure Guide

## Prerequisites

- AWS CLI installed and configured (`aws configure`)
- An EC2 key pair created in your target region
- Your public IP address (for SSH security group)

## Quick Start

### 1. Update Parameters

Edit `cloudformation/parameters.json`:
- Set `KeyPairName` to your EC2 key pair name
- Set `DeployerIP` to your public IP in CIDR notation (e.g., `203.0.113.50/32`)
  - Find your IP: `curl ifconfig.me`

### 2. Launch the Stack

```bash
aws cloudformation create-stack \
    --stack-name gbn-proto-phase1 \
    --template-body file://cloudformation/phase1-stack.yaml \
    --parameters file://cloudformation/parameters.json \
    --region us-east-1

# Wait for completion (~3-5 minutes)
aws cloudformation wait stack-create-complete \
    --stack-name gbn-proto-phase1 \
    --region us-east-1
```

### 3. Get Instance IPs

```bash
aws cloudformation describe-stacks \
    --stack-name gbn-proto-phase1 \
    --query 'Stacks[0].Outputs' \
    --output table
```

### 4. Deploy and Test

```bash
# Deploy Creator (builds binary + uploads test video)
./scripts/deploy-creator.sh <creator-public-ip> /path/to/your/video.mp4

# Deploy Relays
./scripts/deploy-relays.sh <relay1-ip> <relay2-ip> <relay3-ip> <relay4-ip>

# Deploy Publisher
./scripts/deploy-publisher.sh <publisher-public-ip>

# Run full test suite
./scripts/run-tests.sh \
    <creator-ip> <publisher-ip> \
    <relay1-ip> <relay2-ip> <relay3-ip> <relay4-ip>
```

### 5. Teardown (IMPORTANT — stops billing)

```bash
./cloudformation/teardown.sh
```

## Cost Estimate

| Resource | Count | Type | Spot Price (est.) |
|---|---|---|---|
| Creator | 1 | t3.small | ~$0.006/hr |
| Relays | 4 | t3.micro | ~$0.003/hr each |
| Publisher | 1 | t3.small | ~$0.006/hr |
| **Total** | **7** | | **~$0.024/hr** |

Plus minor costs for VPC, data transfer (~$0.01/GB), and EBS storage.

**A full test run (1 hour) costs approximately $0.03–$0.05.**

## Troubleshooting

| Problem | Solution |
|---|---|
| Stack creation fails | Check that your key pair exists in the target region |
| SSH connection refused | Verify `DeployerIP` matches your current public IP |
| Spot request not fulfilled | Try a different instance type or region |
| Bootstrap not complete | SSH in and check `/tmp/bootstrap-done` exists |
| Binary won't run | Ensure you built with `--target x86_64-unknown-linux-gnu` |
