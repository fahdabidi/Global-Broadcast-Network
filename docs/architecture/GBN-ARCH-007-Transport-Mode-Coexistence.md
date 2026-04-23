# GBN-ARCH-007 - Transport Mode Coexistence

**Document ID:** GBN-ARCH-007  
**Status:** Accepted - Conduit remains experimental  
**Last Updated:** 2026-04-23  
**Related:** [GBN-ARCH-000-V2](GBN-ARCH-000-System-Architecture-V2.md), [GBN-ARCH-001-V2](GBN-ARCH-001-Media-Creation-Network-V2.md), [GBN-PROTO-005 Decision Record](../prototyping/GBN-PROTO-005-Decision-Record.md), [Veritas Lattice 0.1.0](https://github.com/fahdabidi/Veritas/releases/tag/veritas-lattice-0.1.0-baseline)

---

## 1. Decision Summary

Veritas now has two transport architectures in the repository:

- **Lattice (V1)**: the frozen onion-mode baseline
- **Conduit (V2)**: the bridge-mode prototype track

The current coexistence decision is:

**Conduit remains experimental. Lattice remains the baseline implementation and the release-facing transport mode until Conduit has live AWS/mobile validation strong enough to justify promotion.**

---

## 2. Current Mode Status

| Mode | Current Status | Release Position |
|---|---|---|
| Lattice (V1) | baseline, preserved, published | default and authoritative |
| Conduit (V2) | implemented as a prototype, locally validated, not yet live-validated | experimental only |

This means:

- Conduit may continue to evolve in the V2 workspace
- Lattice remains the historical and operational reference point
- no current document should imply that Conduit has replaced Lattice in production

---

## 3. Ownership Boundaries

### 3.1 Lattice Boundary

Lattice owns:

- the current release baseline
- the historical onion transport implementation
- V1 architecture history
- V1 deployment and validation paths

### 3.2 Conduit Boundary

Conduit owns:

- the V2 workspace under `prototype/gbn-bridge-proto/`
- V2 bridge protocol and runtime experimentation
- V2 AWS prototype assets
- V2 mobile-validation tooling
- V2-specific architecture and prototype documentation

### 3.3 No-Retroactive-Rewrite Rule

Conduit work must not rewrite Lattice history. Lattice remains the baseline implementation unless a later approved migration plan explicitly changes that state.

---

## 4. Promotion Gates

Conduit may be reconsidered for promotion only after all of the following are true:

1. live AWS deployment validation succeeds
2. live mobile-network validation succeeds
3. coordinated UDP punch success is measured under real NAT/carrier conditions
4. first-contact bootstrap latency is measured and accepted
5. failover and bridge reuse behavior are measured under churn
6. batch onboarding latency is measured in live conditions
7. extended V1 AWS regression passes after V2 infra merges
8. an updated decision record explicitly recommends promotion

Until then, Conduit must not be described as:

- the default mobile transport
- the default creator transport
- a replacement for Lattice

---

## 5. Allowed Coexistence Model

The accepted coexistence model is:

- **release/default path:** Lattice
- **experimental path:** Conduit
- **workspace split:** preserved
- **infra split:** preserved
- **docs split:** preserved

This allows continued Conduit implementation and testing without destabilizing the current Lattice baseline.

---

## 6. Current Risks Holding Back Promotion

The current blockers to promotion are:

- no live AWS bootstrap validation results
- no live mobile IP churn or network-switch measurements
- no real coordinated UDP punch success-rate data
- no live batch onboarding latency measurements
- reduced path anonymity relative to Lattice remains an accepted tradeoff, but not yet one backed by production-grade reachability data

---

## 7. Resulting Architectural Rule

Until a future approved decision supersedes this document:

**Lattice is the baseline transport architecture. Conduit is an experimental parallel transport architecture under active validation.**
