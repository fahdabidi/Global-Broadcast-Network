# GBN Phase 1 Prototype — AWS Test Runbook

This document is the definitive, step-by-step guide to deploying and validating the Phase 1 Zero-Trust Routing Prototype on real AWS infrastructure. Follow each step in order. **Do not skip the Teardown step** — Spot instances bill continuously until the stack is destroyed.

---

## Prerequisites

Before starting, ensure you have the following installed and configured on your **local machine**:

| Tool | Required For | Install |
|---|---|---|
| `aws` CLI | Stack management + IP querying | `pip install awscli` or [official installer](https://aws.amazon.com/cli/) |
| `rustup` + Rust stable | Cross-compiling the relay binary | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| `cross` or Linux target | x86_64 Linux cross-compile | `rustup target add x86_64-unknown-linux-gnu` |
| `ssh` + `scp` | Deploying to EC2 | Pre-installed on macOS/Linux; use Git Bash on Windows |
| `ffmpeg` | Video metadata stripping (sanitizer) | `brew install ffmpeg` / `apt install ffmpeg` |

You also need:
- An **AWS account** with sufficient EC2 quota for 7 Spot instances (`t3.micro / t3.small`).
- An **EC2 Key Pair** created in your target region (default: `us-east-1`).
- A **test video file** (`.mp4` recommended). Minimum 50MB for meaningful testing; 500MB for full performance benchmarks.

---

## Step 1 — Configure AWS CLI

```bash
aws configure
# Enter:
#   AWS Access Key ID:     <your key>
#   AWS Secret Access Key: <your secret>
#   Default region name:   us-east-1
#   Default output format: json
```

Verify:
```bash
aws sts get-caller-identity
```

---

## Step 2 — Set Your Parameters

Edit `infra/cloudformation/parameters.json`:

```json
{
  "Parameters": [
    { "ParameterKey": "KeyPairName",   "ParameterValue": "YOUR-KEY-PAIR-NAME" },
    { "ParameterKey": "DeployerIP",    "ParameterValue": "YOUR.PUBLIC.IP.HERE/32" },
    { "ParameterKey": "CreatorInstanceType",   "ParameterValue": "t3.small" },
    { "ParameterKey": "RelayInstanceType",     "ParameterValue": "t3.micro" },
    { "ParameterKey": "PublisherInstanceType", "ParameterValue": "t3.small" }
  ]
}
```

Find your public IP:
```bash
curl ifconfig.me
```

---

## Step 3 — Launch the CloudFormation Stack

```bash
cd prototype/gbn-proto

aws cloudformation create-stack \
    --stack-name gbn-proto-phase1 \
    --template-body file://infra/cloudformation/phase1-stack.yaml \
    --parameters    file://infra/cloudformation/parameters.json \
    --region us-east-1

# Wait for creation (~3-5 minutes)
echo "Waiting for stack to be ready..."
aws cloudformation wait stack-create-complete \
    --stack-name gbn-proto-phase1 \
    --region us-east-1

echo "✅ Stack created."
```

---

## Step 4 — Collect Instance IPs

```bash
aws cloudformation describe-stacks \
    --stack-name gbn-proto-phase1 \
    --query 'Stacks[0].Outputs' \
    --output table
```

Note the following IPs from the Outputs table and set them as environment variables for the rest of this runbook:

```bash
export CREATOR_IP=<CreatorPublicIP output>
export PUBLISHER_IP=<PublisherPublicIP output>
export RELAY1_IP=<Relay1PublicIP output>
export RELAY2_IP=<Relay2PublicIP output>
export RELAY3_IP=<Relay3PublicIP output>
export RELAY4_IP=<Relay4PublicIP output>
export SSH_KEY=~/.ssh/YOUR-KEY-PAIR-NAME.pem
export TEST_VIDEO=/path/to/your/test-video.mp4
```

Verify SSH access to the Creator:
```bash
ssh -i $SSH_KEY ec2-user@$CREATOR_IP "echo SSH OK"
```

> **Note:** The bootstrap user-data script runs automatically on boot. Wait ~2 minutes after the stack reports `CREATE_COMPLETE` before SSH works.

---

## Step 5 — Build and Deploy the Creator

This step **cross-compiles** the entire workspace for Linux and uploads the binary + test video to the Creator EC2 instance.

```bash
cd prototype/gbn-proto

./infra/scripts/deploy-creator.sh \
    $CREATOR_IP \
    $TEST_VIDEO \
    $SSH_KEY
```

Expected output:
```
[1/4] Building release binaries (target: x86_64-unknown-linux-gnu)...
[2/4] Creating remote directory...
[3/4] Uploading binaries...
[4/4] Uploading test video...
✅ Creator deployment complete.
```

> **Note:** The first build downloads Rust dependencies and may take 5–10 minutes.

---

## Step 6 — Deploy and Start Onion Relay Nodes

This step deploys the binary to all 4 relay instances, generates their Ed25519 identity keypairs, and starts them in `onion-relay` mode joined to the Kademlia DHT.

**Relay 1 acts as the DHT seed node** — all other relays bootstrap off it.

```bash
./infra/scripts/deploy-relays.sh \
    $RELAY1_IP \
    $RELAY2_IP \
    $RELAY3_IP \
    $RELAY4_IP \
    $RELAY1_IP \    # DHT seed = Relay 1
    $SSH_KEY
```

Expected output per relay:
```
[Relay 1] Deploying to 1.2.3.4 (port 9000)...
Generating relay identity keypair...
✅ Keypair generated
✅ Started on port 9000 (DHT on 9100).
```

After all 4 are deployed, collect the relay public keys:
```bash
for IP in $RELAY1_IP $RELAY2_IP $RELAY3_IP $RELAY4_IP; do
    echo "$IP → $(ssh -i $SSH_KEY ec2-user@$IP 'cat ~/gbn-proto/identity/identity.pub')"
done
```

> These public keys are the cryptographic identities that the Creator uses to validate each `RelayExtend` Noise_XX handshake. The `run-tests.sh` script collects and distributes them automatically.

---

## Step 7 — Deploy the Publisher Receiver

```bash
./infra/scripts/deploy-publisher.sh \
    $PUBLISHER_IP \
    $SSH_KEY
```

---

## Step 8 — Run the Full Test Suite

This executes the entire test pipeline:
- Sanitize, chunk, encrypt video on the Creator
- Route encrypted chunks through 3-hop Telescopic Onion circuits
- Reassemble at the Publisher
- **Trigger S1.9**: Kill Relay 1 mid-transmission, verify route recovery
- SHA-256 integrity verification

```bash
./infra/scripts/run-tests.sh \
    $CREATOR_IP \
    $PUBLISHER_IP \
    $RELAY1_IP \
    $RELAY2_IP \
    $RELAY3_IP \
    $RELAY4_IP \
    $SSH_KEY
```

Full output is saved to `/tmp/gbn-phase1-results.log` on your local machine.

### Expected Final Output

```
============================================
  Phase 1 Test Suite Results
============================================

✅ Normal pipeline: PASS
✅ S1.9 Node Recovery: PASS
```

### What S1.9 Does (Mid-Transmission Kill)

After ~15 seconds of active transmission, `run-tests.sh` kills the **Guard relay (Relay 1)** via SSH `pkill`. The Creator's Circuit Manager heartbeat watchdog detects the dead link within 10 seconds, drains the in-flight chunk queue, and re-routes all un-ACKed chunks through a **fresh circuit using a disjoint Guard node**. The test passes if the Publisher successfully reconstructs a byte-identical copy of the original video.

---

## Step 9 — Verify Results Manually (Optional)

SSH into the Publisher and inspect logs:

```bash
ssh -i $SSH_KEY ec2-user@$PUBLISHER_IP

# Check normal pipeline reassembly
ls -lh ~/gbn-proto/reassembled/

# Check circuit-recovery reassembly (S1.9)
ls -lh ~/gbn-proto/reassembled-s19/

# View publisher log
cat /tmp/publisher.log
```

SSH into any relay to inspect DHT and connection logs:

```bash
ssh -i $SSH_KEY ec2-user@$RELAY1_IP
cat /tmp/relay-9000.log   # Relay 1 on port 9000
```

---

## Step 10 — Teardown (⚠️ Required — Stops Billing)

**Run this immediately after testing is complete.**

```bash
./infra/cloudformation/teardown.sh
```

Or manually:
```bash
aws cloudformation delete-stack --stack-name gbn-proto-phase1 --region us-east-1
aws cloudformation wait stack-delete-complete --stack-name gbn-proto-phase1 --region us-east-1
echo "✅ Stack deleted. Billing stopped."
```

Verify no orphaned resources:
```bash
aws ec2 describe-instances \
    --filters "Name=tag:aws:cloudformation:stack-name,Values=gbn-proto-phase1" \
    --query 'Reservations[].Instances[].InstanceId' \
    --output text
# Should return empty
```

---

## Cost Reference

| Resource | Count | Type | Est. $/hr |
|---|---|---|---|
| Creator | 1 | t3.small Spot | ~$0.006 |
| Relay nodes | 4 | t3.micro Spot | ~$0.003 each |
| Publisher | 1 | t3.small Spot | ~$0.006 |
| **Total** | **7 instances** | | **~$0.024/hr** |

A complete test run (build + deploy + run + teardown) takes roughly **45–60 minutes** and costs under **$0.05**.

---

## Troubleshooting

| Symptom | Likely Cause | Fix |
|---|---|---|
| `SSH connection refused` | EC2 bootstrap not done yet | Wait 2-3 min after stack CREATE_COMPLETE |
| `DeployerIP mismatch` | Your IP changed since stack creation | Update parameters.json and redeploy Security Group rule |
| `Spot request not fulfilled` | Capacity constrained | Change `RelayInstanceType` to `t3.nano` or try `us-west-2` |
| `cargo build` fails (OpenSSL) | Missing linker for cross-compile | `apt install musl-tools gcc-x86-64-linux-gnu` |
| `Identity key not found` | bootstrap-relay.sh didn't run | SSH in, run `bash ~/gbn-proto/bootstrap-relay.sh` manually |
| `Noise_XX handshake failed` | Wrong identity key collected | Re-collect pubkeys with `cat ~/gbn-proto/identity/identity.pub` |
| `S1.9: REASSEMBLY INCOMPLETE` | Heartbeat timeout too fast | Increase `HEARTBEAT_TIMEOUT` in `circuit_manager.rs` |
| `Relay DHT not finding peers` | Relay started before seed was ready | Restart non-seed relays after seed is confirmed up |
