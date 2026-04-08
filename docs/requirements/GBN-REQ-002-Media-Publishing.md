# GBN-REQ-002 — Media Publishing: Requirements

**Document ID:** GBN-REQ-002  
**Version:** 0.1 (Draft)  
**Status:** In Review  
**Last Updated:** 2026-04-07  
**Parent:** [GBN-REQ-000](GBN-REQ-000-Top-Level-Requirements.md)  
**Architecture:** [GBN-ARCH-002](../architecture/GBN-ARCH-002-Media-Publishing.md)

---

## 1. Overview

The **Media Publishing (MPub)** component is operated by **Publishers** — independent media outlets, journalist organizations, or individual journalists who serve as curated gatekeepers of Creator-submitted content. The Publisher is the first party in the system with a stable, publicly known identity (their public key).

Publishers perform three primary functions:
1. **Receive** encrypted video chunks from Creators via the MCN and decrypt/reassemble them
2. **Curate** content — reviewing, annotating, approving, or rejecting videos before publication
3. **Publish** approved content to the Globally Distributed Storage (GDS) network, making it available for Content Providers and the Broadcast Network

The Publisher is the **trust anchor** for content in the GBN. All downstream consumers rely on Publisher signatures to authenticate content provenance.

### 1.1 Scope

| In Scope | Out of Scope |
|---|---|
| Receiving and decrypting MCN chunk deliveries | Creator identity management (Creators are anonymous) |
| Chunk integrity verification and reassembly | Video transcoding and format conversion |
| Publisher key management | Content monetization |
| Editorial review tooling | Legal compliance enforcement (delegated to Content Providers) |
| Signing and publishing content to GDS | Viewer-facing streaming |
| Creating BitTorrent-compatible content manifests | Storage node operation (see GBN-REQ-003) |

---

## 2. Stakeholders & Actors

| Actor | Role in MPub |
|---|---|
| **Publisher** | Operates the MPub stack; decrypts, reviews, and publishes content |
| **Creator** | Upstream provider of encrypted video chunks (anonymous) |
| **GDS Network** | Downstream recipient of published chunks and manifests |
| **Content Provider** | Downstream subscriber to Publisher's content feed |
| **Relay Node** | Delivers chunks to Publisher via BON |

---

## 3. Functional Requirements

### 3.1 Chunk Reception

| ID | Requirement | Priority |
|---|---|---|
| MPub-FR-001 | The Publisher node SHALL expose a BON-addressed endpoint for receiving encrypted chunks from the MCN | **Must** |
| MPub-FR-002 | The Publisher SHALL accept chunks out of order; reassembly SHALL be driven by the chunk manifest | **Must** |
| MPub-FR-003 | The Publisher SHALL buffer received chunks securely until the manifest arrives and reassembly is complete | **Must** |
| MPub-FR-004 | The chunk buffer SHALL be stored encrypted at rest, using the Publisher's local storage key | **Must** |
| MPub-FR-005 | The Publisher SHALL verify the BLAKE3 hash of each received chunk against the manifest before accepting it | **Must** |
| MPub-FR-006 | The Publisher SHALL reject and discard any chunk whose integrity check fails, and request re-transmission via the MCN back-channel | **Must** |
| MPub-FR-007 | The Publisher SHALL support simultaneous active upload sessions from multiple Creators | **Must** |
| MPub-FR-008 | Each upload session SHALL be isolated — one Creator's data SHALL never be mixed with another's | **Must** |
| MPub-FR-009 | The Publisher architecture SHALL support deployment of geographically distributed Edge Receiver Nodes to independently receive chunks and sync them to a central reassembly master, mitigating traffic flow correlation at the Publisher ingress | **Should** |

### 3.2 Decryption & Reassembly

| ID | Requirement | Priority |
|---|---|---|
| MPub-FR-010 | The Publisher SHALL decrypt the chunk manifest using their Ed25519/X25519 private key | **Must** |
| MPub-FR-011 | The Publisher SHALL derive the per-chunk symmetric keys from the session key embedded in the manifest | **Must** |
| MPub-FR-012 | The Publisher SHALL decrypt and reassemble chunks in the sequence defined by the manifest | **Must** |
| MPub-FR-013 | Reassembled video SHALL be stored in an encrypted staging area pending editorial review | **Must** |
| MPub-FR-014 | The Publisher's private key SHALL never be exposed to any process outside the key management module | **Must** |
| MPub-FR-015 | The Publisher SHOULD support hardware security module (HSM) or TPM-backed key storage | **Should** |

### 3.3 Editorial Review

| ID | Requirement | Priority |
|---|---|---|
| MPub-FR-020 | The Publisher SHALL have a review dashboard displaying pending videos | **Must** |
| MPub-FR-021 | The Publisher SHALL be able to preview video content before approving it for publication | **Must** |
| MPub-FR-022 | The Publisher SHALL be able to add editorial metadata to a video: title, description, tags, content warnings, approximate date | **Must** |
| MPub-FR-023 | The Publisher SHALL be able to approve a video for publication to the GDS | **Must** |
| MPub-FR-024 | The Publisher SHALL be able to reject a video and purge it from the staging area | **Must** |
| MPub-FR-025 | The Publisher SHALL be able to apply partial redactions (request re-upload with specific time ranges removed) | **Could** |
| MPub-FR-026 | The Publisher dashboard SHOULD maintain an audit log of all editorial decisions | **Should** |

### 3.4 Signing & Content Provenance

| ID | Requirement | Priority |
|---|---|---|
| MPub-FR-030 | All published content SHALL be signed with the Publisher's Ed25519 private key | **Must** |
| MPub-FR-031 | The signature SHALL cover: the content CID, the content manifest, the Publisher's metadata, and the publication timestamp | **Must** |
| MPub-FR-032 | The signed manifest SHALL be published to the GDS DHT, making it discoverable by Content Providers and App nodes | **Must** |
| MPub-FR-033 | The Publisher SHOULD optionally embed C2PA (Coalition for Content Provenance and Authenticity) metadata into the video container | **Should** |
| MPub-FR-034 | The Publisher SHALL maintain a signed, versioned catalog of all their published content | **Must** |
| MPub-FR-035 | Content revocation (removing previously published content from the catalog) SHALL also be possible via a signed revocation record | **Should** |

### 3.5 Publication to GDS

| ID | Requirement | Priority |
|---|---|---|
| MPub-FR-040 | The Publisher SHALL re-chunk the reassembled video for GDS storage using a different (larger, optimized) chunk size than MCN chunking | **Must** |
| MPub-FR-041 | The Publisher SHALL apply Reed-Solomon erasure coding (k=14, n=20 by default, configurable) to the storage chunks | **Must** |
| MPub-FR-042 | The Publisher SHALL distribute encoded shards to ≥20 geographically diverse storage nodes | **Must** |
| MPub-FR-043 | The Publisher SHALL generate a content manifest listing: all shard CIDs, their Reed-Solomon parameters, the content key reference, and the Publisher signature | **Must** |
| MPub-FR-044 | The Publisher SHALL publish the signed manifest to the GDS DHT | **Must** |
| MPub-FR-045 | The content encryption key for storage chunks SHALL be encrypted with the Publisher's public key (Publisher holds it; grants access to Content Providers) | **Must** |
| MPub-FR-046 | The Publisher SHALL monitor shard health and initiate re-replication if storage nodes go offline | **Should** |

### 3.6 Publisher Feed & Subscriptions

| ID | Requirement | Priority |
|---|---|---|
| MPub-FR-050 | The Publisher SHALL maintain a signed, append-only "publication feed" listing all published content CIDs in order | **Must** |
| MPub-FR-051 | Content Providers and App nodes SHALL be able to subscribe to a Publisher's feed via their public key | **Must** |
| MPub-FR-052 | The publication feed SHALL be broadcast via the BON gossip network whenever new content is published | **Must** |
| MPub-FR-053 | Feed entries SHALL include: content CID, title, tags, publication timestamp, content duration, and thumbnail CID | **Must** |
| MPub-FR-054 | The Publisher SHOULD support multiple named "channels" within their feed (e.g., "Breaking News", "Features") | **Could** |

### 3.7 DMCA & Legal Compliance

| ID | Requirement | Priority |
|---|---|---|
| MPub-FR-060 | Publishers operating in the US SHALL maintain a registered DMCA agent and implement a notice-and-takedown workflow | **Must** (for US-based Publishers) |
| MPub-FR-061 | The Publisher SHALL be able to issue a signed revocation notice for published content, which propagates to GDS and instructs storage nodes to delete that content | **Must** |
| MPub-FR-062 | The Publisher SHALL maintain records of DMCA notices received and actions taken for a minimum of 3 years | **Should** |

---

## 4. Non-Functional Requirements

| ID | Requirement | Priority |
|---|---|---|
| MPub-NFR-001 | The Publisher node SHALL be able to handle simultaneous chunk reception from ≥10 concurrent Creator upload sessions | **Must** |
| MPub-NFR-002 | Reassembly of a 500MB video from chunks SHALL complete within 60 seconds on a modern server | **Should** |
| MPub-NFR-003 | The Publisher staging area SHALL be encrypted at rest using AES-256 with Publisher-controlled keys | **Must** |
| MPub-NFR-004 | Publisher node SHALL support deployment on a standard VPS (2 vCPU, 8GB RAM, 1TB disk) as minimum viable hardware | **Should** |
| MPub-NFR-005 | The Publisher dashboard SHALL be accessible via a local web interface (no external SaaS dependency) | **Must** |

---

## 5. Data Requirements

### 5.1 Content Manifest Format

```
ContentManifest {
    content_id:        BLAKE3(video_plaintext)       // canonical content identifier
    publisher_key:     Ed25519PublicKey               // Publisher's identity
    publication_ts:    Unix timestamp (u64)
    title:             UTF-8 string (max 256 chars)
    description:       UTF-8 string (max 4096 chars)
    tags:              [string]
    duration_secs:     u32
    thumbnail_cid:     ContentID                     // CID of thumbnail image

    storage {
        rs_k:          u8 (default: 14)
        rs_n:          u8 (default: 20)
        chunk_size:    u32 (bytes)
        shard_cids:    [ContentID]                   // n shard CIDs
        content_key_encrypted: bytes                 // AES key, encrypted with Publisher public key
    }

    signature:         Ed25519Signature              // signs all fields above
}
```

### 5.2 Publisher Feed Entry

```
FeedEntry {
    content_id:        ContentID
    sequence_number:   u64    // monotonically increasing
    channel:           string (optional)
    manifest_cid:      ContentID   // CID of the full ContentManifest
    published_ts:      Unix timestamp
    signature:         Ed25519Signature
}
```

---

## 6. Interface Requirements

| Interface | Type | Description |
|---|---|---|
| **BON → MPub** | BON socket | Receives encrypted chunk packets from MCN via BON |
| **MPub → GDS** | DHT + direct shard transfer | Publishes manifest to DHT; sends shards to storage nodes |
| **MPub → BON** | Gossip API | Publishes feed updates via BON gossip |
| **Publisher Dashboard** | Local Web UI (HTTPS) | Review, approve, annotate, and publish content |
| **VCP ↔ MPub** | Feed subscription API | Content Providers pull/subscribe to Publisher feeds |

---

## 7. Threat Model

| Threat | Severity | Mitigation |
|---|---|---|
| **Publisher private key theft** | Critical | HSM/TPM storage; air-gapped key generation; key rotation protocol |
| **Publisher node seized by authorities** | High | Publisher's BON address (based on key, not IP) continues to work from new node; key+content migrate |
| **Malicious Creator uploads CSAM** | High | Publisher editorial review; optional PhotoDNA hash check at reception |
| **Forged Publisher signatures** | High | Ed25519 signature validation at every downstream consumer |
| **GDS storage node failure during distribution** | Medium | Reed-Solomon erasure coding provides n-k tolerance |
| **Unauthorized access to staging area** | Medium | Encryption at rest; staging area behind local-only access |

---

## 8. Open Questions

| ID | Question | Impact |
|---|---|---|
| OQ-MPub-001 | Should Publisher content keys be shared with Content Providers via an in-band GBN mechanism, or always out-of-band? | High — affects Content Provider integration |
| OQ-MPub-002 | Should the Publisher support "embargoed" publication (schedule a content release at a future timestamp)? | Low — editorial feature |
| OQ-MPub-003 | Should Publishers be able to collaborate (co-sign a content manifest) to indicate joint publication? | Low — edge case |
| OQ-MPub-004 | How should the Publisher handle a scenario where fewer than n=20 storage nodes are available at publication time? | Medium — resilience design |
