# GBN-REQ-004 — Video Content Providers: Requirements

**Document ID:** GBN-REQ-004  
**Version:** 0.1 (Draft)  
**Status:** In Review  
**Last Updated:** 2026-04-07  
**Parent:** [GBN-REQ-000](GBN-REQ-000-Top-Level-Requirements.md)  
**Architecture:** [GBN-ARCH-004](../architecture/GBN-ARCH-004-Video-Content-Providers.md)

---

## 1. Overview

**Video Content Providers (VCPs)** are businesses or individuals who build consumer-facing video streaming services on top of the GBN infrastructure. They access the Globally Distributed Storage (GDS) to discover, license, and stream Publisher-signed content to their subscribers.

VCPs are the primary legal and editorial accountability layer facing end consumers. Unlike network relays or storage nodes (which are agnostic conduits), VCPs actively curate what content appears in their service and are explicitly responsible for copyright enforcement, age verification, and adherence to local laws.

### 1.1 Scope

| In Scope | Out of Scope |
|---|---|
| GDS catalog discovery and search | Content creation or publishing |
| Channel/playlist management | Storage node operations |
| Content streaming delivery to viewers | Creator anonymization |
| Copyright compliance tools (DMCA) | Publisher operations |
| Content filtering and curation algorithms | Payment processing |

---

## 2. Stakeholders & Actors

| Actor | Role in VCP |
|---|---|
| **Content Provider Operator** | Runs the VCP service; designs content channels |
| **Publisher** | Source of signed content discovered and streamed by VCP |
| **GDS Network** | Source of content shards retrieved by VCP |
| **Viewer / App User** | End consumer of VCP-curated content streams |
| **Copyright Holder** | Sends DMCA takedown notices to the VCP |

---

## 3. Functional Requirements

### 3.1 GDS Catalog Discovery

| ID | Requirement | Priority |
|---|---|---|
| VCP-FR-001 | The VCP SHALL query the GDS DHT to discover content from specific Publishers | **Must** |
| VCP-FR-002 | The VCP SHALL search GDS content by: title, tags, Publisher, date range, duration | **Must** |
| VCP-FR-003 | The VCP SHALL verify Publisher signatures on all discovered content manifests | **Must** |
| VCP-FR-004 | The VCP SHALL maintain a local cache of content manifests to reduce DHT load | **Should** |
| VCP-FR-005 | The VCP SHOULD subscribe to Publisher feeds for automatic new-content notification | **Should** |
| VCP-FR-006 | The VCP SHALL be able to exclude specific content CIDs from its index | **Must** |

### 3.2 Channel & Playlist Management

| ID | Requirement | Priority |
|---|---|---|
| VCP-FR-010 | The VCP SHALL create named channels aggregating content from one or more Publishers | **Must** |
| VCP-FR-011 | Channels SHALL support manual curation (operator-selected content) | **Must** |
| VCP-FR-012 | Channels SHALL support algorithmic curation via rules (Publisher + tags + date + duration) | **Should** |
| VCP-FR-013 | The VCP SHALL publish a machine-readable channel manifest listing available content | **Must** |
| VCP-FR-014 | Channels SHOULD support content warnings and age-gating metadata flags | **Should** |

### 3.3 Content Streaming

| ID | Requirement | Priority |
|---|---|---|
| VCP-FR-020 | The VCP SHALL retrieve content shards from GDS and reassemble via erasure decoding | **Must** |
| VCP-FR-021 | The VCP SHALL decrypt content using the content key provided by the Publisher | **Must** |
| VCP-FR-022 | The VCP SHALL serve content to viewers via HLS or DASH streaming protocol | **Must** |
| VCP-FR-023 | The VCP SHOULD support adaptive bitrate streaming (ABR) with multiple quality levels | **Should** |
| VCP-FR-024 | The VCP SHOULD deliver content streams via the BON for viewers in censored regions | **Should** |
| VCP-FR-025 | The VCP SHALL expose a standard REST API for integration with playback apps | **Must** |

### 3.4 Copyright & Legal Compliance

| ID | Requirement | Priority |
|---|---|---|
| VCP-FR-030 | The VCP SHALL implement DMCA notice-and-takedown per 17 U.S.C. §512 | **Must** |
| VCP-FR-031 | On a valid DMCA takedown, the VCP SHALL remove the content from channels within 24 hours | **Must** |
| VCP-FR-032 | The VCP SHALL implement a DMCA counter-notice mechanism | **Must** |
| VCP-FR-033 | The VCP SHALL maintain a DMCA log for a minimum of 3 years | **Must** |
| VCP-FR-034 | The VCP SHALL implement age verification for content tagged with adult content warnings | **Must** |
| VCP-FR-035 | The VCP SHOULD implement content fingerprinting for copyright recognition before publication | **Should** |

### 3.5 GBN Integration API

| ID | Requirement | Priority |
|---|---|---|
| VCP-FR-040 | GBN SHALL provide a reference VCP SDK: catalog query, shard retrieval, erasure decoding, streaming | **Must** |
| VCP-FR-041 | The SDK SHALL have bindings in: Rust, Python, and JavaScript/TypeScript | **Should** |
| VCP-FR-042 | VCPs SHALL operate independently with no GBN project permission or registration | **Must** |

---

## 4. Non-Functional Requirements

| ID | Requirement | Priority |
|---|---|---|
| VCP-NFR-001 | A VCP SHALL begin streaming a video to a viewer within 5 seconds of request | **Should** |
| VCP-NFR-002 | VCP infrastructure SHALL scale to 100,000 simultaneous viewers | **Should** |
| VCP-NFR-003 | The VCP SDK SHALL be open-source under MIT or Apache 2.0 | **Must** |

---

## 5. Data Requirements

### 5.1 Channel Manifest

```
ChannelManifest {
    channel_id:    UUID
    vcp_name:      string
    channel_name:  string
    content_items: [
        {
            content_cid:   ContentID
            title:         string
            publisher:     Ed25519PublicKey
            published_ts:  Unix timestamp
            duration_secs: u32
            tags:          [string]
        }
    ]
    last_updated: Unix timestamp
}
```

---

## 6. Interface Requirements

| Interface | Type | Description |
|---|---|---|
| **VCP ↔ GDS DHT** | DHT query | Browse content manifests |
| **VCP ↔ Storage Nodes** | BON HTTP | Retrieve shards for streaming |
| **VCP ↔ Publisher** | Out-of-band | Obtain content decryption keys |
| **VCP → Viewer** | HLS/DASH over HTTPS | Standard video streaming |
| **VCP → VPA** | Channel Manifest API | App pulls channel manifests |

---

## 7. Threat Model

| Threat | Severity | Mitigation |
|---|---|---|
| **VCP serves DMCA-protected content** | High | Fingerprint matching; DMCA compliance workflow |
| **Regulatory authority demands removal** | High | VCP has editorial control; can remove from channels |
| **Viewer deanonymization** | Medium | Viewer IPs not stored long-term; BON streams |
| **Bad actor creates VCP for illegal content** | High | VCP is legally liable; not shielded by GBN |

---

## 8. Open Questions

| ID | Question | Impact |
|---|---|---|
| OQ-VCP-001 | Should GBN maintain a public VCP registry or are VCPs self-announced? | Medium |
| OQ-VCP-002 | Should VCPs pay storage nodes for guaranteed availability SLAs? | High |
| OQ-VCP-003 | How does a VCP handle content whose Publisher revokes it mid-stream? | Medium |
