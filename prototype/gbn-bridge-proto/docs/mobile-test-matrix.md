# Veritas Conduit Phase 11 Mobile Validation Matrix

This matrix defines the Phase 11 scenarios, the primary evidence source for each
one, and the current execution state.

## Scope

Phase 11 does not rewrite the transport. It measures how the committed Conduit
prototype behaves under mobile-like conditions and records where live AWS/mobile
evidence is still missing.

## Scenario Matrix

| Scenario | Goal | Primary Command / Evidence | Acceptance Target | Current State |
|---|---|---|---|---|
| App restart with cached catalog | creator reconnects after restart using signed cached state | `mobile-validation.sh --mode local` plus `creator_bootstrap` tests | reconnect and refresh without trust-root drift | local harness evidence available; live mobile run pending |
| Stale bridge recovery | creator skips stale or downgraded bridges and still refreshes catalog | `mobile-validation.sh --mode local` plus `reachability` + `integration` tests | stale entry does not block refresh success | local harness evidence available; live mobile run pending |
| First-time bootstrap | new creator reaches Publisher through HostCreator path and establishes seed tunnel | `mobile-validation.sh --mode local` plus `creator_bootstrap` + `integration` tests | bootstrap completes and seed bridge becomes active | local harness evidence available; live AWS/mobile run pending |
| UDP punch ACK on default port | creator and seed bridge complete bidirectional punch ACK on `443` unless overridden | `mobile-validation.sh --mode local` plus `integration` / `bridge_runtime` tests | ACK success on signed port with no class mismatch | local harness evidence available; live AWS/mobile run pending |
| Network switch / IP churn | creator survives network identity change without losing all upload paths | live AWS/mobile run plus `collect-bridge-metrics.sh` | catalog refresh and continued fanout within one recovery cycle | pending live AWS/mobile run |
| Bridge failover latency | upload continues after one bridge failure | `mobile-validation.sh --mode local` plus `data_path` / `integration` tests | failover remains within one reassignment cycle | local harness evidence available; live AWS/mobile run pending |
| Fanout reuse after churn | creator reuses already-live bridges when full 10-bridge set is unavailable | `mobile-validation.sh --mode local` plus `integration` tests | session completes without full 10-bridge availability | local harness evidence available; live mobile run pending |
| Batched onboarding latency | first 10 join requests stay in one batch; 11th rolls cleanly into next | `integration` + AWS metrics snapshot | 10-request window stays in one batch, 11th is isolated to next rollover | local harness evidence available; live AWS timing still pending |

## Provisional Thresholds

These thresholds are the Phase 11 targets to measure against during live runs.

| Metric | Target |
|---|---|
| first-time bootstrap to seed-tunnel ACK | <= 30s |
| returning creator refresh after restart | <= 10s |
| bridge failover reassignment | <= 5s in local harness, <= 15s in AWS/mobile run |
| stale bridge recovery | <= 2 refresh attempts |
| batch rollover penalty for 11th join | <= one additional batch window plus 2s control latency |

## Tooling

- local proxy workflow: `prototype/gbn-bridge-proto/infra/scripts/mobile-validation.sh --mode local`
- AWS/mobile workflow: `prototype/gbn-bridge-proto/infra/scripts/mobile-validation.sh --mode aws`
- AWS metrics collection: `prototype/gbn-bridge-proto/infra/scripts/collect-bridge-metrics.sh`

## Current Limitation

The current Phase 10 deployment binaries are prototype entrypoints rather than
full network listeners. Live AWS/mobile validation can still verify stack
wiring, task liveness, and log/metrics collection, but full mobile-carrier
behavior should be treated as pending until the deployed binaries expose the
real network service path.
