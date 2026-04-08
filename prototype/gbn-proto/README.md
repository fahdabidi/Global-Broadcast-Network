# Global Broadcast Network (GBN)

A decentralized, censorship-resistant video publishing and distribution platform.

---

## Vision

A world where any individual can create video content and distribute it to any willing recipient, free from censorship by governments, ISPs, or platform monopolies — while publishers and content providers can build legitimate, responsible services on the same infrastructure.

---

## Repository Strategy

### Current: Prototype Monorepo

During the prototyping phase (Phases 1–3), all components live in this single repository as a Rust workspace. This allows rapid iteration, atomic cross-component refactoring, and a single `cargo test` to validate everything.

```
gbn-proto/
├── crates/
│   ├── gbn-protocol/        ← Shared contracts (will become its own repo)
│   ├── mcn-sanitizer/       ← Metadata stripping (will join gbn-mcn repo)
│   ├── mcn-chunker/         ← Video chunking + BLAKE3 (will join gbn-mcn repo)
│   ├── mcn-crypto/          ← Key exchange + AES-256-GCM (will join gbn-mcn repo)
│   ├── mcn-router-sim/      ← Multipath relay across EC2 instances (will join gbn-mcn repo)
│   ├── mpub-receiver/       ← Publisher reassembly (will join gbn-mpub repo)
│   └── proto-cli/           ← CLI orchestrator (prototyping only)
├── infra/
│   ├── cloudformation/      ← Stack templates for each prototype phase
│   └── scripts/             ← Deploy, bootstrap, run-tests, teardown scripts
├── test-vectors/            ← User-provided sample video files (gitignored)
└── tests/
    └── integration/         ← Cross-crate integration tests
```

### Production: Layered Multi-Repo

After prototyping validates the architecture and interfaces stabilize, the monorepo will be split into **8 independent repositories** organized by dependency layer. This enables independent collaborators to manage each component autonomously, with independent release cycles, CI/CD pipelines, and access control.

```
Layer 0 ─── gbn-protocol     Shared contracts: wire formats, traits, crypto interfaces
                │
Layer 1 ─── gbn-bon          Broadcast Overlay Network: transport, onion routing, gossip
                │
        ┌───────┼───────┐
Layer 2 │  gbn-mcn      │    Media Creation Network: anonymize, chunk, encrypt, route
        │  gbn-mpub     │    Media Publishing: receive, decrypt, sign, distribute
        │  gbn-gds      │    Globally Distributed Storage: erasure coding, DHT, shards
        └───────┼───────┘
                │
        ┌───────┼───────┐
Layer 3 │  gbn-vcp      │    Video Content Providers: streaming API, HLS, DMCA
        │  gbn-vpa      │    Video Playback App: Android client, gossip, streaming
        └───────────────┘

Layer ∞ ─── gbn-docs         Documentation: requirements, architecture, security
```

#### Why Layered Multi-Repo?

| Concern | How This Structure Addresses It |
|---|---|
| **Independent Teams** | Each repo has its own maintainers, CI/CD, and release cadence |
| **Supply Chain Resilience** | Seizing one repo does not expose others. Each repo is mirrored across ≥3 platforms (GitHub, Codeberg, self-hosted Gitee). Aligns with [GBN-SEC-007](docs/security/GBN-SEC-007-Software-Supply-Chain.md) |
| **Protocol Stability** | `gbn-protocol` changes are rare and require M-of-N maintainer approval. All components depend on it via version-pinned git tags |
| **Integration Testing** | A dedicated CI job pulls `main` from every repo and runs the full end-to-end test suite |

#### Repository Dependency Graph

```
gbn-protocol ← gbn-bon ← gbn-mcn
                       ← gbn-mpub
                       ← gbn-gds ← gbn-vcp
                                 ← gbn-vpa
```

Every component depends on `gbn-protocol` for shared types. Every component depends on `gbn-bon` for network transport. Layer 3 components additionally depend on `gbn-gds` for content retrieval.

After the split, each `Cargo.toml` will reference the shared crate via git dependency:

```toml
[dependencies]
gbn-protocol = { git = "https://github.com/gbn-project/gbn-protocol", tag = "v0.1.0" }
gbn-bon      = { git = "https://github.com/gbn-project/gbn-bon", tag = "v0.1.0" }
```

---

## Architecture Overview

The GBN consists of six core sub-systems:

| Component | Purpose |
|---|---|
| **Media Creation Network (MCN)** | Anonymous video upload: metadata stripping, chunking, encryption, multi-hop relay delivery to Publisher |
| **Media Publishing (MPub)** | Publisher-side: decryption, reassembly, editorial review, Ed25519 signing, GDS distribution |
| **Globally Distributed Storage (GDS)** | Redundant erasure-coded (Reed-Solomon) shard storage across volunteer nodes worldwide |
| **Video Content Providers (VCP)** | Curated streaming services: HLS packaging, channel management, DMCA compliance |
| **Video Playback App (VPA)** | Android client: peer gossip, shard retrieval, video playback, relay participation |
| **Broadcast Overlay Network (BON)** | Transport layer: pluggable transports (WebTunnel/obfs4), onion routing, NAT traversal |

Full documentation is in [`docs/`](docs/):
- **Requirements**: `docs/requirements/GBN-REQ-*.md`
- **Architecture**: `docs/architecture/GBN-ARCH-*.md`
- **Security**: `docs/security/GBN-SEC-*.md`
- **Prototyping Plans**: `docs/prototyping/GBN-PROTO-*.md`
- **Research**: `docs/research/GBN-RESEARCH-*.md`

---

## Tech Stack

| Layer | Technology | Rationale |
|---|---|---|
| **Core Protocol** | Rust | Memory safety, performance, no runtime/GC |
| **Cryptography** | `x25519-dalek`, `aes-gcm`, `ed25519-dalek`, `blake3`, `snow` (Noise) | Audited, minimal-dependency crypto crates |
| **Async Runtime** | Tokio | Industry standard for async Rust networking |
| **Erasure Coding** | `reed-solomon-erasure` | Proven RS implementation for k-of-n reconstruction |
| **Metadata Stripping** | FFmpeg (via CLI wrapper) | Best-in-class container format support |
| **Android UI** | Kotlin + Rust FFI (JNI) | Native Android UX with Rust crypto core |
| **VCP API** | Go (Axum alternative for HTTP services) | Strong HTTP ecosystem; Go is acceptable for non-crypto paths |

---

## Prototyping Phases

The prototype is organized into 3 sequential phases, each validating a critical architectural layer. Full plans with test matrices, benchmarks, and pass/fail criteria are linked below.

### Phase 1 — Media Creation Network & Video Reconstruction
📄 **Plan:** [`GBN-PROTO-001`](../docs/prototyping/GBN-PROTO-001-Phase1-Media-Creation.md)

**Objective:** Prove that a video file can be anonymized, chunked, encrypted, sent through multiple independent simulated relay paths, received out-of-order by a Publisher, and perfectly reconstructed — with zero data loss, zero metadata leakage, and cryptographic integrity verified at every step.

| Key Validation | What It Proves |
|---|---|
| Metadata stripping completeness | Creator identity doesn't leak via EXIF/GPS/encoder strings |
| Chunk-then-encrypt with AES-256-GCM | Out-of-order decryption and per-chunk error isolation work |
| X25519 ECDH key derivation | Creator and Publisher independently derive identical session keys |
| Multipath routing simulation | No single relay path sees all chunks (MCN-FR-034) |
| Tamper detection (bit flips) | GCM auth tags reject any modification to ciphertext |
| 500MB pipeline performance | Full pipeline completes within performance targets |

### Phase 2 — Publishing & Globally Distributed Storage
📄 **Plan:** [`GBN-PROTO-002`](../docs/prototyping/GBN-PROTO-002-Phase2-Publishing-Storage.md)

**Objective:** Prove that a Publisher can re-chunk a video for storage, apply Reed-Solomon erasure coding, distribute shards across simulated storage nodes, sign a content manifest, and that any client can later reconstruct the full video from only k-of-n shards — even when (n-k) nodes are dead, corrupted, or hostile.

| Key Validation | What It Proves |
|---|---|
| Reed-Solomon (k=14, n=20) encode/decode | Any 14 of 20 shards reconstruct the original data |
| RS with encrypted data | Erasure coding works correctly on AES ciphertext |
| Ed25519 manifest signing & verification | Publisher provenance is cryptographically unforgeable |
| Shard corruption detection | BLAKE3 hash catches any tampered shard |
| Node failure tolerance | Content survives loss of up to 6 simultaneous nodes |
| Kademlia DHT manifest discovery | Content is discoverable under 20% node churn |

### Phase 3 — Broadcast Overlay Network & Video Playback
📄 **Plan:** [`GBN-PROTO-003`](../docs/prototyping/GBN-PROTO-003-Phase3-Broadcast-Playback.md)

**Objective:** Prove that two devices on separate real networks can establish a censorship-resistant encrypted connection through a multi-hop onion relay chain using pluggable transports, and that a viewer can discover, stream, and play a video from distributed storage peers — all without exposing any participant's true IP address.

| Key Validation | What It Proves |
|---|---|
| Noise_XX mutual authentication | Forward-secret sessions between nodes in < 500ms |
| 3-hop onion routing | No relay can determine both source and destination |
| WebTunnel DPI evasion | Traffic is classified as standard HTTPS by DPI tools |
| Active probe defense | Non-BON clients get a plausible HTML page, not a BON leak |
| HyParView gossip convergence | Peer graph survives 30% simultaneous node churn |
| Video streaming startup | Viewer begins playback within 5 seconds |
| Full E2E pipeline | Creator uploads → Viewer plays back, all via BON, byte-perfect |

---

## Building

### Prerequisites

- Rust 1.77+ (install via [rustup](https://rustup.rs/))
- FFmpeg 6.0+ (for metadata stripping tests)
- Docker (for Phase 3 network simulation)

### Build & Test

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run specific integration test
cargo test --test test_full_pipeline

# Run benchmarks
cargo bench --workspace
```

---

## Security Model

The GBN operates on a **Zero-Knowledge Transit** principle: relay nodes know only enough to forward packets. No node can determine both the source and destination of a transmission.

Key security documents:
- [GBN-SEC-001](docs/security/GBN-SEC-001-Media-Creation-Network.md) — Creator anonymity model
- [GBN-SEC-006](docs/security/GBN-SEC-006-Broadcast-Network.md) — Transport layer DPI resistance
- [GBN-SEC-007](docs/security/GBN-SEC-007-Software-Supply-Chain.md) — Update integrity & death-update defense

---

## License

TBD — To be determined after legal review. Candidate: AGPLv3 (strong copyleft ensuring all network service modifications remain open source).
