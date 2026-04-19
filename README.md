# Global Broadcast Network (GBN) - Prototype Workspace

**A decentralized, censorship-resistant video creation, publishing, and distribution platform - designed so truth can travel faster than it can be suppressed.**

> *"The internet treats censorship as damage and routes around it."*
> - John Gilmore

---

## Project Status

This repository is an **active prototype** (`gbn-proto`) for validating core architecture and security assumptions.

- Core Rust workspace and crate boundaries are in place
- Integration test scaffolding exists for metadata stripping, multipath reassembly, tamper detection, and end-to-end pipeline tests
- CLI orchestration commands are partially implemented (see `crates/proto-cli/src/main.rs`)
- Not production-ready; APIs and protocols are expected to evolve during prototyping

If you are looking for full system design docs (requirements, architecture, security), see [`../../docs/`](../../docs/).

---

## Quick Start

### Prerequisites

- Rust 1.77+
- FFmpeg 6.0+
- (Optional for infra simulation) AWS CLI + Docker

### 1) Build the workspace

```bash
cargo build --workspace
```

### 2) Run tests

```bash
cargo test --workspace
```

### 3) Add local test videos (for media pipeline tests)

Place `.mp4` files in [`test-vectors/`](./test-vectors/) (this directory is gitignored).

See [`test-vectors/README.md`](./test-vectors/README.md) for expected files and guidance.

### 4) (Optional) AWS phase infrastructure

For EC2-based prototype runs and teardown, see [`infra/README-infra.md`](./infra/README-infra.md).

---

## Vision & Mission

In many countries, a journalist who records police violence, corruption, or protests faces an impossible choice: **publish and be identified, or stay silent and stay safe**.

Existing options leave major gaps:
- **Mainstream platforms** can remove content centrally and log subpoenaable metadata.
- **Tor + generic file sharing** protects uploader routing but does not provide an integrated publisher trust + distribution pipeline.
- **VPNs** shift trust to the VPN operator.

The **Global Broadcast Network** aims to provide a complete, end-to-end pipeline - from capture to playback - such that no single point of failure can trivially identify creators or suppress distribution.

### Design Principles

| Principle | What It Means In Practice |
|---|---|
| **Privacy by Default** | End-to-end encryption and local metadata sanitization before transmission |
| **Resilience over Efficiency** | Erasure-coded distribution across geographically diverse nodes |
| **Legal Responsibility at the Edges** | Editorial/legal responsibility is with Publishers and Content Providers |
| **Adaptive to Adversaries** | Pluggable transport strategy evolves against censorship techniques |
| **Sovereign Updates** | Supply-chain hardening via reproducible builds and multi-party governance (see [GBN-SEC-007](../../docs/security/GBN-SEC-007-Software-Supply-Chain.md)) |

---

## How It Works

**The Root of Trust:** The user journey strictly begins prior to recording the video. The Creator must first establish cryptographic trust by scanning the Publisher's Public Key via a QR code (or by downloading a pre-seeded Sovereign Publisher App). Additionally to seed the network the Publisher must provide (or the Creator must aquire) a few exit relays, located outside the geofence, that can connect to the Publisher to bypass Publisher geofencing. This ensures the MCN encrypts data specifically for that Publisher and structurally prevents adversary traffic interception.

### Journey of a Video

```
  CREATOR                      RELAY NETWORK                       PUBLISHER
  (hostile jurisdiction)       (3-hop onion routing)               (trusted entity)

 +---------------------+                                       +---------------------+
 | 1. Record video     |       +========================+       | 5. Receive chunks   |
 | 2. Strip metadata   |       |  Path 1                |       |    (out-of-order)   |
 |    (GPS, device ID, |------>|  Guard > Middle > Exit |------>| 6. Decrypt each     |
 |     timestamps)     |       +========================+       | 7. Verify BLAKE3    |
 | 3. Chunk (1MB each) |       +========================+       | 8. Reassemble video |
 | 4. Encrypt chunks   |------>|  Path 2 (diff circuit) |------>| 9. Editorial review |
 |    (AES-256-GCM)    |       +=======================+        |10. Sign (Ed25519)   |
 |                     |       +========================+       |                     |
 |                     |------>|  Path 3 (diff circuit) |------>|                     |
 +---------------------+       +========================+       +----------+----------+
                                                                          |
                          GLOBAL DISTRIBUTED STORAGE                      |
                        +---------------------------------------------<---+
                        |
                        v
 +------------------------------------------------------------------------------+
 |  Reed-Solomon erasure coding: split into 20 shards (14 data + 6 parity).    |
 |  Distribute across volunteer nodes worldwide. ANY 14 of 20 shards can       |
 |  reconstruct the original. Content survives seizure of 6 nodes. Each shard  |
 | can have many replicas                                                      |
 +-------------------------------------+----------------------------------------+
                                        |
                          VIEWER        |
                        +----------------+
                        |
                        v
              +-------------------+
              | Discover content  |
              | via peer gossip   |
              |        |          |
              | Fetch 14 of 20    |
              | shards via BON    |
              |        |          |
              | Reconstruct and   |
              | play video        |
              +-------------------+
```


## Publisher Flow Packet Path implemented in Current Prototype

**Path/Return_Path**: Creator → Guard → Middle → Exit → Publisher

The path is created by the creator from its DHT which has been populated by the gossip network

**Onion build (Creator, innermost first):**

```
layer_pub  = seal(publisher_pub,  { next_hop: None,chunk_payload, chunk_id, chunk_hash, return_path, send_timestamp, total_chunks, chunk_index })
layer_exit = seal(exit_pub,       { next_hop: publisher_addr, inner: layer_pub  })
layer_mid  = seal(middle_pub,     { next_hop: exit_addr,      inner: layer_exit })
layer_grd  = seal(guard_pub,      { next_hop: middle_addr,    inner: layer_mid  })
```

Creator sends `layer_grd` over TCP to Guard.

**Each relay (Guard / Middle / Exit):**
1. Read length-prefixed bytes from TCP
2. `open(own_priv, bytes)` → `{ next_hop, inner }`
3. Connect to `next_hop`, write `inner` as length-prefixed bytes
4. (No response needed for data forwarding)

**Publisher:**
1. `open(own_priv, bytes)` → `{ next_hop: None,chunk_payload, chunk_id, chunk_hash, return_path, send_timestamp, total_chunks, chunk_index }`
2. Verify hash, store chunk
3. Build reverse-direction ACK (ChunkID, Receive Timestamp, Hash, ChunkIndex) onion using `return_path` → send back to Creator

**ACK return path**: Publisher → Exit → Middle → Guard → Creator
Creator must listen on an ACK port; return_path contains Creator's ack address.

---

## Size of Onion Packet
SNOW encryptiom library has a 64KiB size limit. So we break payloads into 8Kib Chunks and transmit individually. Table below shows how payload size increases after onion layering. 

```text
+-----------------+------------------------+--------------------------------+--------------------------+-----------------------+------------------------------------+
| Step            | Incoming payload bytes | Encoded incoming payload bytes | New header/wrapper bytes | Total plaintext bytes | Size after encryption to next step |
+-----------------+------------------------+--------------------------------+--------------------------+-----------------------+------------------------------------+
| ChunkPayload    | 16,384                 | 21,848                         | 840                      | 22,688                | n/a                                |
| Publisher layer | 22,688                 | 30,252                         | 259                      | 30,511                | 30,559                             |
| Exit layer      | 30,559                 | 40,748                         | 270                      | 41,018                | 41,066                             |
| Middle layer    | 41,066                 | 54,756                         | 272                      | 55,028                | 55,076                             |
| Guard layer     | 55,076                 | 73,436                         | 271                      | 73,707                | seal failed                        |
+-----------------+------------------------+--------------------------------+--------------------------+-----------------------+------------------------------------+
```
#TODO Currently the packet size decreases as the packet traverses the Onion Network relay to relay, this can provide a good guess on who the Creator was. We need to make packets uniform in size across Onion layers to hide Creator identify. 

### Gossip Network Design

Every node in the GBN relay network participates in a **PlumTree epidemic broadcast** protocol (implemented over libp2p request/response) to maintain a shared, eventually-consistent directory of all reachable nodes.

**What is gossiped:**

| Message Type | Content |
|---|---|
| `NodeAnnounce` | A single node's address, public key, role (relay / seed / creator / publisher), and capabilities |
| `DirectorySync` | A batch of `RelayNode` entries — used for initial catch-up when a new node joins |

**How PlumTree works:**

PlumTree separates peers into two sets per node:

- **Eager peers** — receive full message payloads immediately (push)
- **Lazy peers** — receive only `IHave` message-ID announcements; they pull (`IWant`) only if the payload hasn't arrived via an eager path first

This keeps redundant traffic low under normal conditions while guaranteeing delivery: if the eager path fails, a lazy peer's `IHave` triggers repair. Peers are promoted/demoted between eager and lazy sets dynamically (`Graft` / `Prune` messages) based on delivery performance.

**Deduplication:** Each message carries a 32-byte `MessageId` (content hash). Every node tracks a sliding window of seen IDs; duplicate deliveries are dropped immediately.

**Rate limiting:** Each node enforces a token-bucket bandwidth budget on outbound gossip to prevent a single announcement storm from saturating the network under high churn (validated at N=100 nodes on ECS Fargate — Phase 1 result).

**Propagation diagram:**

```
  A relay node announces itself: NodeAnnounce { addr, pub_key, role }

                        +--------------+
                        |  Originator  |
                        +------+-------+
            +------------------+------------------+
     eager push           eager push           IHave only
     (full payload)       (full payload)     (message-ID)
            |                  |                   :
            v                  v                   :
     +------------+    +--------------+    +--------------+
     |  Seed Node |    |   Guard A    |    |   Guard B    |
     +-----+------+    +------+-------+    +------+-------+
           |                  |                   | IWant (not yet seen)
    eager  |  lazy        eager|                  v
           v  : : :>           v           +-----------------+
     +---------+  IHave  +----------+      | full payload    |
     | Creator |         |  Middle  |      | pulled on-demand|
     +---------+         +----+-----+      +-----------------+
                              | eager
                              v
                        +----------+
                        |   Exit   |
                        +----------+

  --- full payload pushed immediately to eager peers
  : : IHave (message-ID only); receiver sends IWant to pull if not yet seen

  All nodes converge on an identical DHT view.
```
**How the Creator uses the gossip DHT:**

When a Creator wants to send, it queries its local in-memory DHT (populated by gossip) to find candidate nodes by role:
- **Guard** — any `HostileRelay` or `SeedRelay`
- **Middle** — same pool, Guard excluded
- **Exit** — `FreeRelay` only - outside the Geofence, Identified by the Publisher as reachable by it
- **Publisher** — the well-known Publisher address learned from initial seeding

The Creator selects one node per hop and builds the onion circuit entirely from local DHT state — no network round-trips are needed for path selection.

---

### What each participant can observe

```text
Creator      -> Sees: local video + target Publisher key
               Sees full relay topology and Pub Keys

Guard relay  -> Sees: previous hop + next hop
               Cannot see: payload plaintext or final destination context

Middle relay -> Sees: adjacent hops only
               Cannot see: creator identity, publisher identity, or content plaintext

Exit relay   -> Sees: prior hop and destination endpoint
               Cannot see: origin creator identity or content plaintext

Publisher    -> Sees: decrypted submitted content
               Can see: full relay topology and Pub Keys back to creator for Ack message

Storage node -> Sees: encrypted shards by content-addressed ID
               Cannot see: plaintext media

Viewer       -> Sees: playable stream/content
               Cannot see: creator identity or full relay path
```

### Prototype components in this workspace

| Component | Purpose (prototype scope) | Primary use in this prototype |
|---|---|---|
| `gbn-protocol` | Shared wire types and serialization contracts for chunks, manifests, onion routing, DHT, and crypto payloads | Common dependency used across every service role |
| `mcn-sanitizer` | Media sanitization pipeline and FFmpeg-based metadata stripping | Creator-side preprocessing before chunking/upload |
| `mcn-chunker` | Chunking, hashing, and manifest-oriented segmentation helpers | Creator-side chunk generation and integrity bookkeeping |
| `mcn-crypto` | Publisher key generation, upload-session encryption, and Noise-based onion seal/open helpers | Creator and publisher cryptographic flow |
| `mcn-router-sim` | Gossip/DHT, relay control plane, telescopic onion routing, ACK relay path, and distributed trace metadata | Relay, creator, seed relay, and transport orchestration |
| `mpub-receiver` | Publisher-side onion terminal receive path, chunk acceptance, transport ACK generation, and session completion tracking | Publisher role runtime |
| `proto-cli` | The `gbn-proto` binary entrypoint that wires all crates together into runnable commands and service modes | Single executable used by the prototype containers and local CLI |

### Phase Prototype Image Mapping

Both prototype Dockerfiles currently compile the same binary:

```bash
cargo build --release --bin gbn-proto --features distributed-trace
```

That means both images currently link the full workspace transitively through `proto-cli`.

| Component | `gbn-relay` image | `gbn-publisher` image | Notes |
|---|---|---|---|
| `gbn-protocol` | Yes | Yes | Shared dependency of the single `gbn-proto` binary |
| `mcn-sanitizer` | Yes | Yes | Linked through `proto-cli`, even if not exercised by every runtime role |
| `mcn-chunker` | Yes | Yes | Linked through `proto-cli` |
| `mcn-crypto` | Yes | Yes | Linked through `proto-cli` |
| `mcn-router-sim` | Yes | Yes | Linked through `proto-cli` |
| `mpub-receiver` | Yes | Yes | Linked through `proto-cli` |
| `proto-cli` | Yes | Yes | Defines the `gbn-proto` binary built into both images |

### Current Phase Stack Runtime Usage

| Runtime role in `phase1-scale-stack.yaml` | Image currently used | Notes |
|---|---|---|
| `SeedRelayInstance` | `gbn-relay` | Static EC2 bootstrap relay; kept as the single seed relay for network bring-up |
| `HostileRelayService` | `gbn-relay` | ECS/Fargate relay tasks in hostile subnet |
| `FreeRelayService` | `gbn-relay` | ECS/Fargate relay tasks in free subnet |
| `CreatorService` | `gbn-relay` | ECS/Fargate creator role also runs the same `gbn-proto` binary image |
| `PublisherInstance` | `gbn-relay` | Current stack still launches publisher mode from the relay image |
| `gbn-publisher` ECR image | Built and published, but not wired into the current phase stack | `Dockerfile.publisher` exists, but `phase1-scale-stack.yaml` does not currently launch the publisher instance from `ContainerImagePublisher` |

---

## Repository Layout

```text
gbn-proto/
|-- Cargo.toml
|-- README.md
|-- crates/
|   |-- gbn-protocol/
|   |-- mcn-sanitizer/
|   |-- mcn-chunker/
|   |-- mcn-crypto/
|   |-- mcn-router-sim/
|   |-- mpub-receiver/
|   `-- proto-cli/
|-- infra/
|   |-- README-infra.md
|   |-- cloudformation/
|   `-- scripts/
|-- test-vectors/
|   `-- README.md
`-- tests/
    `-- integration/
        |-- test_metadata_stripping.rs
        |-- test_multipath_reassembly.rs
        |-- test_tamper_detection.rs
        `-- test_full_pipeline.rs
```

---

## Technical Stack (Prototype)

| Layer | Technology | Why |
|---|---|---|
| Core implementation | Rust | Memory safety + performance for protocol/security-critical paths |
| Crypto primitives | `x25519-dalek`, `aes-gcm`, `ed25519-dalek`, `blake3`, `hkdf` | Modern, auditable Rust crypto ecosystem |
| Async runtime | Tokio | Mature async I/O runtime |
| Erasure coding target (planned) | `reed-solomon-erasure` | k-of-n reconstruction model |
| Metadata stripping | FFmpeg (CLI integration) | Broad container support |
| Mobile target (planned) | Kotlin + Rust FFI | Native Android UX with shared Rust core |

> Note: Some architectural docs discuss future VCP service implementations in Go. Those are design-stage targets, not part of this prototype workspace.

---

## Prototyping Phases

### Phase 1 - Media Creation Network & zero-trust routing
Plan: [`../../docs/prototyping/GBN-PROTO-001-Phase1-Media-Creation.md`](../../docs/prototyping/GBN-PROTO-001-Phase1-Media-Creation.md)

### Phase 2 - Publishing & distributed storage
Plan: [`../../docs/prototyping/GBN-PROTO-002-Phase2-Publishing-Storage.md`](../../docs/prototyping/GBN-PROTO-002-Phase2-Publishing-Storage.md)

### Phase 3 - Overlay broadcast network & playback
Plan: [`../../docs/prototyping/GBN-PROTO-003-Phase3-Broadcast-Playback.md`](../../docs/prototyping/GBN-PROTO-003-Phase3-Broadcast-Playback.md)

---

## Security Model (Summary)

GBN uses a **Zero-Knowledge Transit** design goal: intermediate nodes should know only what is necessary for forwarding.

Detailed security docs:
- [GBN-SEC-001 â€” Media Creation Network](../../docs/security/GBN-SEC-001-Media-Creation-Network.md)
- [GBN-SEC-002 â€” Media Publishing](../../docs/security/GBN-SEC-002-Media-Publishing.md)
- [GBN-SEC-003 â€” Global Distributed Storage](../../docs/security/GBN-SEC-003-Global-Distributed-Storage.md)
- [GBN-SEC-004 â€” Video Content Providers](../../docs/security/GBN-SEC-004-Video-Content-Providers.md)
- [GBN-SEC-005 â€” Video Playback App](../../docs/security/GBN-SEC-005-Video-Playback-App.md)
- [GBN-SEC-006 â€” Broadcast Network](../../docs/security/GBN-SEC-006-Broadcast-Network.md)
- [GBN-SEC-007 â€” Software Supply Chain](../../docs/security/GBN-SEC-007-Software-Supply-Chain.md)

### Dynamic Circuit Rebuilding & Anonymity

Because the GBN relies on consumer devices scaling dynamically to provide routing services, node churn is inevitable. The architecture implements **Active Heartbeat Disconnects** over the inner `Noise_XX` layer, enabling near-instantaneous detection of relay failure. Upon failure, dropping circuits immediately release un-ACKed chunks into a reassignment queue, dialing fresh circuits. To resist **Temporal Circuit Correlation** (adversaries mapping sequential circuit rebuilds to origin metadata), replacement circuits explicitly select completely separate Guard hubs — rendering temporal drops disjoint and preserving anonymity.

### Important limitations

As documented in the security files, the system **does not fully mitigate**:
- endpoint compromise (malware/physical seizure)
- global passive adversary traffic correlation (partially mitigated)
- complete internet shutdown/physical disconnection events

---

## Documentation Index

All system-level docs live under [`../../docs/`](../../docs/):

- Requirements: `../../docs/requirements/GBN-REQ-*.md`
- Architecture: `../../docs/architecture/GBN-ARCH-*.md`
- Security: `../../docs/security/GBN-SEC-*.md`
- Prototyping: `../../docs/prototyping/GBN-PROTO-*.md`
- Research: `../../docs/research/GBN-RESEARCH-*.md`

---

## Contributing (Prototype)

Contributions are welcome for prototype hardening, test coverage, and correctness improvements.

Suggested contribution flow:
1. Open an issue describing the problem or enhancement
2. Propose scope aligned to the active prototype phase
3. Submit a PR with tests (`cargo test --workspace`)
4. Include doc updates when behavior/protocol assumptions change

---

## License

This prototype workspace is currently licensed under **AGPL-3.0-or-later** (see workspace `Cargo.toml`).
