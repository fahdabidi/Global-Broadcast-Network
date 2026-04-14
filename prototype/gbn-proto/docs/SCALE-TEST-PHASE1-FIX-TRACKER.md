# GBN-PROTO-004 Phase 1 — Scale Test Fix Tracker

> **Status:** In Progress  
> **Created:** 2026-04-12  
> **Goal:** Fix all infrastructure bugs blocking scale test execution and successfully run N=100, N=500, N=1000.

---

## Bug Fix Checklist

### CRITICAL — Test cannot run at all

- [x] **Fix A** — `deploy-scale-test.sh`: WSL/Windows path conversion for `--template-file`  
  _File:_ `infra/scripts/deploy-scale-test.sh` L30–33  
  _Bug:_ `wslpath` conversion only fires when `is_aws_exe_fallback=true`; doesn't fire in Git Bash → raw `/mnt/c/…` path sent to `aws.exe` → deployment fails immediately  
  _Fix:_ Replace conditional block with unconditional `convert_path()` helper using `wslpath -w` when available

- [x] **Fix B** — New `entrypoint.sh`: inject `GBN_INSTANCE_IPV4` from ECS task metadata  
  _File:_ `entrypoint.sh` (new file)  
  _Bug:_ No mechanism to read the task's private IP from ECS metadata at container start → `register_with_cloudmap()` silently skips registration  
  _Fix:_ Shell wrapper that queries `$ECS_CONTAINER_METADATA_URI_V4`, exports IP, then `exec`s binary

- [x] **Fix C** — `Dockerfile.relay` + `Dockerfile.publisher`: wire in entrypoint wrapper  
  _Files:_ `Dockerfile.relay` L11, `Dockerfile.publisher` L11  
  _Bug:_ Direct `ENTRYPOINT ["gbn-proto", "serve"]` — no wrapper, can't inject dynamic env vars  
  _Fix:_ Add `curl`, `python3`; `COPY entrypoint.sh`; switch to `ENTRYPOINT ["entrypoint.sh"]` + `CMD ["gbn-proto", "serve"]`

- [x] **Fix D1** — CFN security groups: ports 9000–9100 → 4001  
  _File:_ `infra/cloudformation/phase1-scale-stack.yaml` L153–155, L169–172  
  _Bug:_ SGs allow ports 9000–9100; swarm P2P port is 4001 → all inter-node traffic blocked  
  _Fix:_ Change `FromPort`/`ToPort` to 4001 in `RelaySecurityGroup` and `PublisherSecurityGroup`

- [x] **Fix D2** — CFN task definitions: port mappings 9000 → 4001  
  _File:_ `phase1-scale-stack.yaml` L494, L526, L564  
  _Bug:_ `ContainerPort: 9000` in all three task defs; swarm binds on 4001  
  _Fix:_ Change all `ContainerPort` values to 4001

- [x] **Fix D3** — CFN IAM: add `RegisterInstance`/`DeregisterInstance` to `EcsTaskRole`  
  _File:_ `phase1-scale-stack.yaml` L282–306  
  _Bug:_ IAM policy only grants `DiscoverInstances`; `RegisterInstance`/`DeregisterInstance` calls return AccessDenied  
  _Fix:_ Add both actions to the `GbnScaleTaskPolicy` statement

- [x] **Fix D4** — CFN: remove `ServiceRegistries` from relay ECS services  
  _File:_ `phase1-scale-stack.yaml` L598–599, L615–616  
  _Bug:_ ECS auto-registers each task (instance_id = task ARN); Rust also registers (instance_id = HOSTNAME) → two Cloud Map entries per task → duplicate peer discovery  
  _Fix:_ Delete `ServiceRegistries` blocks from `HostileRelayService` and `FreeRelayService`; let Rust handle registration (adds `GBN_PEER_ID` to attributes)

- [x] **Fix D5** — CFN task defs: add `GBN_CLOUDMAP_SERVICE_ID`, `GBN_SCALE`, `GBN_P2P_PORT`  
  _File:_ `phase1-scale-stack.yaml` — all four task definitions  
  _Bug:_ Missing env vars → `register_with_cloudmap()` silently returns; metrics lack Scale dimension; port is hardcoded in code vs. not configured  
  _Fix:_ Add to Creator/HostileRelay/FreeRelay task defs:
  - `GBN_CLOUDMAP_SERVICE_ID: !Ref RelayDiscoveryService`
  - `GBN_SCALE: !Ref ScaleTarget`
  - `GBN_P2P_PORT: '4001'`  
  Add all of the above + `GBN_CLOUDMAP_NAMESPACE: gbn.local` + `GBN_CLOUDMAP_SERVICE: relay` to Publisher task def

- [x] **Fix D6** — CFN Lambda handler: `chaos-controller.handler` → `index.handler`  
  _File:_ `phase1-scale-stack.yaml` L341  
  _Bug:_ Inline ZipFile Lambda must use `index.handler`; current name causes "Handler not found" on every invocation  
  _Fix:_ Change `Handler` value to `index.handler`

### HIGH — Test runs but produces wrong/missing results

- [x] **Fix H2** — _(covered by Fix D5 above — `GBN_SCALE` in task defs)_

- [x] **Fix H3** — _(covered by Fix D5 above — Publisher task def gets CloudMap env vars)_

### MEDIUM — Minor script bugs

- [x] **Fix E** — `recover-n100-push.sh`: repo name extraction + build context  
  _File:_ `infra/scripts/recover-n100-push.sh` L37, L43, L49–50  
  _Bug 1:_ `${ECR_URI#*/}` strips first `/` → wrong repo name; should be `##*/`  
  _Bug 2:_ `docker build` uses `.` context; should use `$PROTO_ROOT`  
  _Fix:_ `##*/` for both repo name vars; explicit `$PROTO_ROOT` build context

### POST-RUN FIXES (after Apr 13 partial run)

- [x] **Fix F1** — `deploy-scale-test.sh` + `run-chaos-upload.sh`: Stabilisation gate timeout too short  
  _Bug:_ Default `POLL_TIMEOUT_SECONDS=600` not enough for 33 Fargate cold-starts + Cloud Map propagation  
  _Fix:_ Increased to 1200s; poll interval reduced from 30s → 10s

- [x] **Fix F2** — `deploy-scale-test.sh` + `run-chaos-upload.sh`: CloudWatch SEARCH hits `MaxMetricsExceeded`  
  _Bug:_ Unfiltered SEARCH across all NodeId series from all historical runs exceeds CloudWatch 500-series limit → bootstrap sum truncated → gate never passed even when nodes were healthy  
  _Fix:_ Added `Scale=\"$SCALE_TARGET\"` filter to SEARCH in deploy script; derived `SCALE_HINT` from stack name suffix in chaos script

- [x] **Fix F3** — Both scripts: Gate loop hung with 30s silence between prints  
  _Bug:_ Long 30s wait between status lines made deployment appear frozen  
  _Fix:_ Restructured loop: ECS `runningCount` polled every 10s (primary gate), CloudWatch polled every 30s (diagnostic only); formatted progress line printed on every iteration

### POST-RUN FIXES (after Apr 13 full run — images had no entrypoint)

- [x] **Fix G1** — Apr 13 run: Images not rebuilt before test — old images (no `entrypoint.sh`) deployed  
  _Root cause:_ ECR images were last pushed Apr 11 (before Fix B/C). Apr 13 run used old images without entrypoint wrapper → `GBN_INSTANCE_IPV4` never injected → `register_with_cloudmap()` silently returned → nodes booted in isolation → zero gossip/circuit/chunk metrics despite high bootstrap count  
  _Fix:_ Rebuild images via WSL2 Docker after CF stack is created; chain: `build-and-push.sh` → `deploy-scale-test.sh` → `run-chaos-upload.sh` → `teardown-scale-test.sh`

- [x] **Fix G2** — `phase1-scale-stack.yaml`: All 4 ECS services start at non-zero `DesiredCount` → CFN stack creation hangs when ECR repos are empty  
  _Bug:_ `HostileRelayService` (90) + `FreeRelayService` (10) + `CreatorService` (1) + `PublisherService` (1) start immediately on CF create. If ECR repos are empty, tasks fail image pull → services never stabilize → CF create never completes  
  _Fix:_ Set all 4 `DesiredCount: 0` in CFN template; deploy-scale-test.sh handles scaling after images are pushed

- [x] **Fix G3** — `deploy-scale-test.sh`: Creator and Publisher services not scaled up after CF deploy  
  _Bug:_ Script only resolved and scaled HostileRelayService + FreeRelayService; Creator/Publisher stayed at 0 (since CFN now starts at 0)  
  _Fix:_ Also resolve `CreatorService` + `PublisherService` and scale both to 1 in step [3/5]

- [x] **Fix G4** — `deploy-scale-test.sh`: `aws cloudformation deploy` fails with non-zero exit when stack has no changes  
  _Bug:_ Re-running deploy-scale-test.sh against an already-deployed stack caused `set -euo pipefail` to abort at step 1 ("no changes to deploy")  
  _Fix:_ Added `--no-fail-on-empty-changeset` to the `aws cloudformation deploy` call

- [x] **Fix G5** — `teardown-scale-test.sh`: CF stack deletion fails (`DELETE_FAILED`) when ECR repos contain images  
  _Bug:_ CFN cannot delete `AWS::ECR::Repository` resources that contain images → stack stuck in DELETE_FAILED  
  _Fix:_ Added step [4/5] to `aws ecr batch-delete-image` for both ECR repos before calling `delete-stack`

### POST-RUN FIXES (after Apr 13 N=100 run — zero gossip/circuit/chunk metrics)

- [x] **Fix H4** — Creator role never injects gossip messages → `GossipBandwidthBytes` = 0 forever  
  _Root cause:_ `gbn-proto serve` with `GBN_ROLE=creator` runs the PlumTree swarm but never calls `publish_local()`. The `GossipBandwidthBytes` metric is the delta of `bytes_sent_total()` — with no outbound messages, the delta is always 0. The `gbn-proto upload` command (the other CLI subcommand) is a local TCP simulation, not distributed routing.  
  _Files:_ `crates/mcn-router-sim/src/swarm.rs`, `crates/mcn-router-sim/src/observability.rs`  
  _Fix:_  
  - Added `role`, `creator_publish_interval`, `last_creator_publish: Option<Instant>`, `creator_seq` fields to `GossipRuntime`  
  - In `drive_swarm_once()`: when `role == "creator"` and interval elapsed and peers > 0, call `engine.publish_local(unique_msg_id, payload)` + `send_outbound(swarm, outbound)` → generates real gossip traffic  
  - Added `publish_chunks_delivered()` to `MetricsReporter`; called on every successful creator inject  
  - Default interval: 30s (overridable via `GBN_CREATOR_PUBLISH_INTERVAL_SECS`)  
  - `last_creator_publish: None` → fires immediately once first peer connects; resets only on successful inject so no timer drift if no peers yet

- [x] **Fix H5** — `run-chaos-upload.sh`: 60s observation window too short for gossip convergence  
  _Bug:_ Chaos enabled → 60s sleep → teardown. At N=100 with creator auto-publishing every 30s, 60s = at most 2 publish cycles, with no time for CloudWatch 60-second rollup window to appear in the dump.  
  _Fix:_ Added `CHAOS_OBSERVE_SECONDS` env var (default 180s) replacing hardcoded `sleep 60`; updated step label to reflect gossip propagation intent

- [x] **Fix H6** — `run-chaos-upload.sh`: `UPLOAD_COMMAND` default was `gbn-proto --help` (no-op)  
  _Bug:_ The execute-command just printed help text; with creator auto-publish the exec is now a diagnostics-only step  
  _Fix:_ Changed default to `echo 'gbn-creator-healthy'`

- [x] **Fix H7** — `build-and-push.sh`: no Docker fallback for Git Bash (Docker not in PATH)  
  _Bug:_ Script failed with "docker not found" when run from Git Bash even with Docker Desktop installed (Docker only available via WSL2 in that environment)  
  _Fix:_ Added WSL2 re-exec fallback: if `wsl.exe` is available and `wsl.exe -e docker version` succeeds, convert the Git Bash script path (`/c/...` → `/mnt/c/...`) and `exec wsl.exe -e bash "$WSL_SCRIPT" "$@"` so the entire build runs natively inside WSL2

---

## Test Execution (run after all fixes confirmed)

### Step 0 — Teardown any stuck stacks
```powershell
aws cloudformation describe-stacks --stack-name gbn-proto-phase1-scale-n500 --region us-east-1 --query "Stacks[0].StackStatus" --output text
# If not already deleted:
aws cloudformation delete-stack --stack-name gbn-proto-phase1-scale-n500 --region us-east-1
aws cloudformation wait stack-delete-complete --stack-name gbn-proto-phase1-scale-n500 --region us-east-1
```

### Step 1 — Build & push images (WSL/Git Bash)
```bash
bash prototype/gbn-proto/infra/scripts/build-and-push.sh gbn-proto-phase1-scale-n100 us-east-1
```

### Step 2 — Run N=100 test (WSL/Git Bash)
```bash
bash prototype/gbn-proto/infra/scripts/deploy-scale-test.sh gbn-proto-phase1-scale-n100 100 us-east-1
bash prototype/gbn-proto/infra/scripts/run-chaos-upload.sh  gbn-proto-phase1-scale-n100 us-east-1
bash prototype/gbn-proto/infra/scripts/teardown-scale-test.sh gbn-proto-phase1-scale-n100 us-east-1
```

### Step 3 — Verify results
- `results/scale-gbn-proto-phase1-scale-n100-*-metrics.json` — all 4 metric arrays non-empty
- CloudWatch dashboard `gbn-proto-phase1-scale-n100-protocol-metrics`

| Metric | Target | Apr 13 Run | Notes |
|--------|--------|------------|-------|
| Bootstrap Success | 100% nodes | 13/100 (partial) | `MaxMetricsExceeded` query truncation; full run had 98 ECS tasks running |
| GossipBandwidthBytes | > 0 | 0 | Creator never called `publish_local()` — fixed in H4 |
| Goodput vs. Overhead | > 60% | No data | Requires gossip traffic first |
| Blackhole Rate | < 5% | No data | — |
| Time-to-Convergence | < 15s | No data | — |
| Circuit Build Success | > 80% | 0% | No circuits attempted (no gossip) |
| Path Diversity | 100% | 0% | — |
| ChunksDelivered | ≥ 1 | 0 | Fixed in H4 (auto-publish) |

---

## File Index

| File | Fixes Applied |
|------|--------------|
| `infra/scripts/deploy-scale-test.sh` | A |
| `entrypoint.sh` | B (new) |
| `Dockerfile.relay` | C |
| `Dockerfile.publisher` | C |
| `infra/cloudformation/phase1-scale-stack.yaml` | D1 D2 D3 D4 D5 D6 |
| `infra/scripts/recover-n100-push.sh` | E |
