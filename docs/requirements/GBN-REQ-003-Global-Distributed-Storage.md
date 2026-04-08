# GBN-REQ-003 — Globally Distributed Storage: Requirements

**Document ID:** GBN-REQ-003  
**Version:** 0.1 (Draft)  
**Status:** In Review  
**Last Updated:** 2026-04-07  
**Parent:** [GBN-REQ-000](GBN-REQ-000-Top-Level-Requirements.md)  
**Architecture:** [GBN-ARCH-003](../architecture/GBN-ARCH-003-Global-Distributed-Storage.md)

---

## 1. Overview

The **Globally Distributed Storage (GDS)** network is the persistent backbone of the GBN — a decentralized, redundant store of encrypted video content that no single jurisdiction, authority, or technical failure can eliminate. Once a Publisher commits content to the GDS, it persists as long as the minimum storage quorum exists.

The GDS is inspired by BitTorrent and IPFS but adds:
- **Publisher-signed provenance** for every piece of content
- **Reed-Solomon erasure coding** for efficient redundancy (vs. full replication)
- **Encrypted-at-rest storage** — storage nodes never see plaintext content
- **Incentive-aware storage contracts** to motivate long-term participation

### 1.1 Scope

| In Scope | Out of Scope |
|---|---|
| Storing encrypted content shards | Decrypting or accessing content (storage nodes are blind) |
| Reed-Solomon encoding/decoding | Video playback (see GBN-REQ-005) |
| Content manifest storage in DHT | Content curation or filtering |
| Shard integrity verification | Publisher operations (see GBN-REQ-002) |
| Storage node incentive tracking | Billing or payment processing |
| Shard health monitoring & re-replication | Content provider indexing (see GBN-REQ-004) |

---

## 2. Stakeholders & Actors

| Actor | Role in GDS |
|---|---|
| **Storage Node Operator** | Provides disk space and bandwidth; stores encrypted shards |
| **Publisher** | Writes content to the GDS; issues signed manifests |
| **Content Provider** | Reads content from the GDS; retrieves shards for streaming |
| **Video Playback App** | Retrieves shards for local playback; may also seed shards |
| **GDS DHT Network** | Decentralized index of content manifests and shard locations |

---

## 3. Functional Requirements

### 3.1 Shard Storage

| ID | Requirement | Priority |
|---|---|---|
| GDS-FR-001 | Storage nodes SHALL accept and store encrypted content shards from Publishers | **Must** |
| GDS-FR-002 | Each stored shard SHALL be identified by its BLAKE3 content hash (content-addressed storage) | **Must** |
| GDS-FR-003 | Storage nodes SHALL verify the integrity of incoming shards by computing and confirming the BLAKE3 hash upon receipt | **Must** |
| GDS-FR-004 | Storage nodes SHALL never attempt to decrypt stored shards | **Must** |
| GDS-FR-005 | Storage nodes SHALL be able to serve stored shards to any requester who provides the correct shard CID | **Must** |
| GDS-FR-006 | Storage nodes SHALL support configurable storage quotas (how much disk space they pledge to the network) | **Must** |
| GDS-FR-007 | When a storage node's quota is full, it SHALL accept no new shards until space is freed by expiry or explicit deletion | **Must** |
| GDS-FR-008 | Storage nodes SHOULD prioritize retaining shards that are least-replicated across the network (scarcity-aware retention) | **Should** |

### 3.2 Erasure Coding

| ID | Requirement | Priority |
|---|---|---|
| GDS-FR-010 | The GDS SHALL support Reed-Solomon erasure coding with configurable k and n parameters (default: k=14, n=20) | **Must** |
| GDS-FR-011 | Any k of n encoded shards SHALL be sufficient to reconstruct the original content | **Must** |
| GDS-FR-012 | Erasure coding SHALL be performed by the Publisher before distributing shards to storage nodes | **Must** |
| GDS-FR-013 | The GDS SHALL support reconstruction of content from any k available shards, even if the remaining (n-k) are unavailable | **Must** |
| GDS-FR-014 | When the number of available shards for a content item falls below k+3 (three above the reconstruction threshold), the GDS SHALL initiate automatic re-replication | **Should** |
| GDS-FR-015 | Re-replication SHALL be authorized by a signed re-replication request from the original Publisher, or by designated repair nodes acting under Publisher delegation | **Should** |

### 3.3 Content Addressing & DHT

| ID | Requirement | Priority |
|---|---|---|
| GDS-FR-020 | Content manifests SHALL be stored in a Kademlia DHT, keyed by content CID | **Must** |
| GDS-FR-021 | Each DHT entry for a content manifest SHALL include the Publisher's signature, enabling authenticity verification without a central authority | **Must** |
| GDS-FR-022 | The DHT SHALL support lookup of all content published by a given Publisher (keyed by Publisher public key) | **Must** |
| GDS-FR-023 | Shard locations (which storage nodes hold specific shards) SHALL be searchable via the DHT | **Must** |
| GDS-FR-024 | DHT entries SHALL be refreshed periodically — stale entries SHALL expire after a configurable TTL (default: 24 hours) with re-announcement | **Must** |
| GDS-FR-025 | The DHT SHALL be Sybil-resistant — nodes providing consistently incorrect data SHALL be blacklisted by the requester | **Must** |

### 3.4 Content Integrity

| ID | Requirement | Priority |
|---|---|---|
| GDS-FR-030 | Every shard served by a storage node SHALL include its BLAKE3 hash for client-side verification | **Must** |
| GDS-FR-031 | Any consumer (Content Provider, App) that receives a shard SHALL verify its hash before using it | **Must** |
| GDS-FR-032 | Repeated hash verification failures from a specific storage node SHALL result in that node's reputation being downgraded | **Should** |
| GDS-FR-033 | The content manifest SHALL include a Merkle tree of all shard hashes, enabling efficient verification of partial content retrieval | **Should** |

### 3.5 Storage Node Lifecycle

| ID | Requirement | Priority |
|---|---|---|
| GDS-FR-040 | New storage nodes SHALL register with the GDS DHT by publishing their Ed25519 node ID, storage quota, and geographic region | **Must** |
| GDS-FR-041 | Storage nodes SHALL publish periodic "heartbeat" announcements to the DHT confirming they are online and their shard inventory | **Must** |
| GDS-FR-042 | If a storage node fails to heartbeat for a configurable period (default: 1 hour), it SHALL be considered unavailable and its shards marked for re-replication | **Should** |
| GDS-FR-043 | Storage nodes SHALL be able to gracefully retire — announcing their retirement and assisting in migrating their shards to willing recipients | **Should** |

### 3.6 Incentive & Reputation Model

| ID | Requirement | Priority |
|---|---|---|
| GDS-FR-050 | The GDS SHALL maintain a reputation score for each storage node based on uptime, shard integrity, and response latency | **Must** |
| GDS-FR-051 | High-reputation storage nodes SHALL be preferred for new shard placement | **Must** |
| GDS-FR-052 | Storage nodes SHOULD operate on a reciprocal model: nodes that store more shards for others are allocated preferred placement for their own Publisher's content | **Should** |
| GDS-FR-053 | The GDS SHOULD support an optional token-based incentive layer in a future version; the Phase 1 design SHALL NOT preclude this | **Could** |

### 3.7 Content Revocation

| ID | Requirement | Priority |
|---|---|---|
| GDS-FR-060 | Publishers SHALL be able to issue a signed revocation notice for their content | **Must** |
| GDS-FR-061 | Storage nodes SHALL honor revocation notices signed by the content's original Publisher by deleting the referenced shards | **Must** |
| GDS-FR-062 | Revocation notices SHALL be propagated across the DHT | **Must** |
| GDS-FR-063 | CSAM-flagged content SHALL be revocable by a designated safety authority even if the Publisher refuses | **Must** |
| GDS-FR-064 | A global revocation list (signed by multiple trusted parties) SHALL be maintained for CSAM-flagged content hashes | **Must** |

---

## 4. Non-Functional Requirements

| ID | Requirement | Priority |
|---|---|---|
| GDS-NFR-001 | Shard retrieval latency SHALL be under 500ms median for nodes with good connectivity | **Should** |
| GDS-NFR-002 | Storage node daemon SHALL consume no more than 2GB RAM on a standard VPS instance | **Must** |
| GDS-NFR-003 | The GDS SHALL support a minimum of 10 million unique content items stored across the network | **Should** |
| GDS-NFR-004 | The GDS DHT SHALL support at least 1 million active nodes | **Should** |
| GDS-NFR-005 | Storage node software SHALL run on: Linux (primary), macOS, Windows, and ARM (for Raspberry Pi-class nodes) | **Should** |
| GDS-NFR-006 | A storage node SHALL be installable and operational within 10 minutes from a blank OS | **Should** |

---

## 5. Data Requirements

### 5.1 Shard Format

```
StoredShard {
    shard_cid:         BLAKE3(shard_ciphertext)     // content-addressed ID
    content_cid:       BLAKE3(original_video)       // parent content reference
    shard_index:       u16                          // index within erasure set
    rs_k:              u8                           // erasure coding parameter
    rs_n:              u8                           // erasure coding parameter
    ciphertext:        bytes                        // AES-256-GCM encrypted shard
    gcm_nonce:         bytes[12]
    gcm_tag:           bytes[16]
    publisher_sig:     Ed25519Signature             // signs: content_cid + shard_index + shard_cid
}
```

### 5.2 DHT Content Manifest Entry

Refer to the ContentManifest format defined in [GBN-REQ-002 §5.1](GBN-REQ-002-Media-Publishing.md).

### 5.3 Storage Node Heartbeat

```
NodeHeartbeat {
    node_id:           Ed25519PublicKey
    timestamp:         Unix timestamp
    quota_total_gb:    u32
    quota_used_gb:     u32
    shard_count:       u64
    geographic_region: string (ISO country code)
    signature:         Ed25519Signature
}
```

---

## 6. Interface Requirements

| Interface | Type | Description |
|---|---|---|
| **Publisher → Storage Node** | Direct TCP/UDP | Shard upload; storage node confirms receipt + hash |
| **DHT API** | Kademlia DHT | Content manifest lookup, shard location lookup, node registration |
| **Storage Node → Consumer** | BON-routed HTTP | Shard retrieval by CID |
| **GDS → GDS** | Internal gossip | Re-replication coordination, revocation propagation |
| **Safety Authority → GDS** | Signed broadcast | CSAM revocation list updates |

---

## 7. Threat Model

| Threat | Severity | Mitigation |
|---|---|---|
| **Storage node serves corrupted shards** | High | BLAKE3 hash verification at every retrieval; reputation penalty |
| **Sybil attack on DHT** | High | Proof-of-work node registration; DHT routing diversity |
| **Selective shard deletion by hostile ISP** | High | Erasure coding (k-of-n); geographic diversity requirements |
| **Storage node seized by authorities** | High | Nodes store only encrypted shards; no plaintext; k-of-n recovery |
| **Unauthorized content writes (forged Publisher)** | High | Ed25519 Publisher signature required on all manifests |
| **DHT poisoning (false shard locations)** | Medium | Multiple source lookups; hash verification at retrieval |
| **Long-term storage attrition (nodes drop out)** | Medium | Proactive re-replication triggered when quorum approaches k |

---

## 8. Open Questions

| ID | Question | Impact |
|---|---|---|
| OQ-GDS-001 | What is the right erasure coding ratio for different content tiers (e.g., journalistically critical vs. general content)? | Medium |
| OQ-GDS-002 | Should storage nodes be geographically audited (to prevent accidentally all n shards landing in one jurisdiction)? | High |
| OQ-GDS-003 | Should the global CSAM revocation list be managed by a neutral third party (e.g., Internet Watch Foundation integration)? | High — legal and operational |
| OQ-GDS-004 | Should lightweight app nodes (mobile phones) be eligible as storage nodes for small quotas, or only desktop/server nodes? | Medium |
