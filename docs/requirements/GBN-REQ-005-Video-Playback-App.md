# GBN-REQ-005 — Video Playback App & Broadcast: Requirements

**Document ID:** GBN-REQ-005  
**Version:** 0.1 (Draft)  
**Status:** In Review  
**Last Updated:** 2026-04-07  
**Parent:** [GBN-REQ-000](GBN-REQ-000-Top-Level-Requirements.md)  
**Architecture:** [GBN-ARCH-005](../architecture/GBN-ARCH-005-Video-Playback-App.md)

---

## 1. Overview

The **Video Playback App (VPA)** is the primary end-user facing component of the GBN. It is a mobile application installed on a viewer's phone that:

1. **Discovers and receives** new video content metadata from a decentralized peer network (no central server required)
2. **Filters and stores** content based on the user's Publisher subscriptions
3. **Streams video** by retrieving content shards from peers and GDS storage nodes
4. **Acts as a peer node** — both forwarding content metadata to neighboring apps and serving video chunks to other peers
5. **Seeds itself** — new installs can be bootstrapped via a share-to-install mechanism through ordinary messaging apps

The VPA is the final link that closes the full decentralized loop: content flows from Creator → MCN → Publisher → GDS → VPA → Viewer, with no step requiring a central server for the content itself.

### 1.1 Scope

| In Scope | Out of Scope |
|---|---|
| Mobile app for Android (Phase 1) | iOS (Phase 2) |
| Peer discovery and gossip | Publisher node operations |
| Content metadata polling and storage | Video creation or recording |
| HLS/DASH video streaming from chunks | Content moderation |
| Embedded HTTP server for peer serving | Payment or subscription billing |
| App bootstrapping and seeding mechanism | VCP channel management |

---

## 2. Stakeholders & Actors

| Actor | Role in VPA |
|---|---|
| **Viewer (App User)** | Installs app; selects Publisher subscriptions; watches videos |
| **Peer App Node** | Another VPA installation that the App exchanges peer lists and content metadata with |
| **VCP** | Provides curated channel manifests that the App can optionally subscribe to |
| **GDS Storage Nodes** | Source of content shards for streaming |
| **Publisher** | Content authority; App uses Publisher public key as the trust anchor for content |

---

## 3. Functional Requirements

### 3.1 App Bootstrapping & Initial Seeding

| ID | Requirement | Priority |
|---|---|---|
| VPA-FR-001 | The App SHALL ship with a set of hardcoded bootstrap peer addresses (5–10 geographically distributed) | **Must** |
| VPA-FR-002 | The App SHALL support a "share-to-install" seeding mechanism: an existing App user generates a share package containing the APK, their peer list, and a Publisher key | **Must** |
| VPA-FR-003 | The share package SHALL be distributable via standard messaging apps (WhatsApp, Signal, SMS) as a file or deep link | **Must** |
| VPA-FR-004 | On first launch, the App SHALL connect to bootstrap peers to receive an initial peer list | **Must** |
| VPA-FR-005 | After the initial peer list is established, the App SHALL self-update its peer list through the peer gossip protocol — no further dependency on bootstrap nodes | **Must** |

### 3.2 Peer Discovery & Gossip

| ID | Requirement | Priority |
|---|---|---|
| VPA-FR-010 | The App SHALL maintain an active peer view (5–10 directly connected peers) and a passive peer view (50–200 known peers for backup) | **Must** |
| VPA-FR-011 | The App SHALL implement a gossip protocol (HyParView or equivalent) for peer list maintenance | **Must** |
| VPA-FR-012 | The App SHALL periodically (every 30–60 seconds by default) poll active peers for content updates | **Must** |
| VPA-FR-013 | The App SHALL share its own peer list with peers that request it, extending the network | **Must** |
| VPA-FR-014 | The App SHALL handle peer churn gracefully — promote passive peers to active when active peers disconnect | **Must** |
| VPA-FR-015 | All peer-to-peer communication SHALL occur via the BON, preventing direct IP exposure | **Must** |

### 3.3 Content Metadata Management

| ID | Requirement | Priority |
|---|---|---|
| VPA-FR-020 | The App SHALL store received content manifests (metadata-only; no video chunks) locally | **Must** |
| VPA-FR-021 | The App SHALL filter received manifests by the user's Publisher subscription list, discarding non-subscribed content | **Must** |
| VPA-FR-022 | The App SHALL verify Publisher signatures on all received manifests, discarding unsigned or invalid ones | **Must** |
| VPA-FR-023 | The App SHALL forward newly received (and accepted) content manifests to its active peer set via gossip | **Must** |
| VPA-FR-024 | The App SHALL present a user-facing content library of available videos based on stored manifests | **Must** |
| VPA-FR-025 | The App SHOULD support browsing content from subscribed VCP channel manifests in addition to raw Publisher feeds | **Should** |

### 3.4 Publisher Subscriptions

| ID | Requirement | Priority |
|---|---|---|
| VPA-FR-030 | The App SHALL allow users to add Publisher subscriptions by manually entering or scanning a Publisher's public key | **Must** |
| VPA-FR-031 | The App SHALL exclusively rely on Out-Of-Band (OOB) key distribution (e.g. QR codes, Share-packages, messaging apps) rather than fetching keys from a Publisher's clearweb domain, which is vulnerable to geo-fencing | **Must** |
| VPA-FR-032 | The App SHALL allow users to remove Publisher subscriptions and remove locally cached metadata for that Publisher | **Must** |
| VPA-FR-032 | The App SHOULD display a human-readable Publisher fingerprint (word-based representation of public key) | **Should** |
| VPA-FR-033 | The App MAY optionally allow subscription to VCP channels (which wrap Publisher content with editorial curation) | **Could** |

### 3.5 Video Streaming & Playback

| ID | Requirement | Priority |
|---|---|---|
| VPA-FR-040 | When a user selects a video, the App SHALL begin retrieving content shards from available GDS nodes and peers | **Must** |
| VPA-FR-041 | Shard retrieval SHALL be parallelized across multiple sources simultaneously | **Must** |
| VPA-FR-042 | The App SHALL perform erasure decoding to reconstruct video segments from k-of-n shards | **Must** |
| VPA-FR-043 | The App SHALL verify BLAKE3 hash of each shard before decoding | **Must** |
| VPA-FR-044 | The App SHALL begin video playback within 5 seconds of the user selecting a video (progressive streaming) | **Should** |
| VPA-FR-045 | The App SHALL cache downloaded and decoded video segments to local storage for repeat plays | **Should** |
| VPA-FR-046 | The App SHOULD support offline playback of previously cached videos | **Should** |
| VPA-FR-047 | The App SHALL support standard video formats: MP4 (H.264/H.265) and WebM (VP9) | **Must** |

### 3.6 App as Peer Server

| ID | Requirement | Priority |
|---|---|---|
| VPA-FR-050 | The App SHALL run an embedded lightweight HTTP server that can serve cached video shards to other App nodes | **Must** |
| VPA-FR-051 | The embedded server SHALL bind to the BON overlay address (not the device's public IP directly) | **Must** |
| VPA-FR-052 | The embedded server SHALL serve only shards that the App has verified and cached | **Must** |
| VPA-FR-053 | The App SHALL limit shard serving bandwidth to a user-configurable cap (default: 1Mbps upload) | **Should** |
| VPA-FR-054 | On Android, the embedded server SHOULD continue operating as a background service when the App is not in the foreground | **Should** |
| VPA-FR-055 | The App SHALL act as a relay node for the BON overlay, forwarding packets for other nodes (with configurable bandwidth limit) | **Should** |

### 3.7 IP Renegotiation

| ID | Requirement | Priority |
|---|---|---|
| VPA-FR-060 | On each app startup, the App SHALL broadcast a signed IP renegotiation message to its active peer set | **Must** |
| VPA-FR-061 | The IP renegotiation message SHALL contain: the node's Ed25519 public key, the new BON address, and a timestamp | **Must** |
| VPA-FR-062 | Peers SHALL update their stored address for this node upon receiving a valid, signed renegotiation message | **Must** |
| VPA-FR-063 | The App SHALL sign all renegotiation messages with its persistent node Ed25519 key | **Must** |
| VPA-FR-064 | The App SHALL also perform IP renegotiation when it detects a network interface change (e.g., WiFi to cellular) | **Should** |

---

## 4. Non-Functional Requirements

| ID | Requirement | Priority |
|---|---|---|
| VPA-NFR-001 | The App SHALL target Android 8.0 (API level 26) and above | **Must** |
| VPA-NFR-002 | Background gossip and relay activities SHALL consume no more than 5% average CPU and 50MB RAM | **Should** |
| VPA-NFR-003 | The App SHALL consume no more than 100MB of cellular data per hour when operating as a background relay | **Should** |
| VPA-NFR-004 | Content metadata (manifests) storage SHALL not exceed 500MB for a library of 10,000 videos | **Should** |
| VPA-NFR-005 | The App APK SHALL be distributable outside the Google Play Store (sideload-compatible) | **Must** |

---

## 5. Data Requirements

### 5.1 Local App Data

| Data | Format | Encrypted? |
|---|---|---|
| Peer list (active + passive view) | SQLite | No (peer IDs are public) |
| Content manifests | SQLite + BLOB | No (manifests are public, Publisher-signed) |
| Publisher subscriptions | SQLite | No (public keys) |
| Cached video shards | File system | Yes (stored as received encrypted shards) |
| Node keypair (persistent) | Secure storage | Yes (Android Keystore) |

### 5.2 Peer Communication Envelope

```
GossipMessage {
    type:          enum { PeerListUpdate, NewContentManifest, IPRenegotiation }
    sender_id:     Ed25519PublicKey
    timestamp:     Unix timestamp
    payload:       bytes  // encrypted with recipient's session key
    signature:     Ed25519Signature
}
```

---

## 6. Interface Requirements

| Interface | Type | Description |
|---|---|---|
| **VPA ↔ Peers (VPA)** | BON gossip | Peer list exchange; content manifest propagation |
| **VPA ↔ GDS Storage Nodes** | BON HTTP | Shard retrieval |
| **VPA ↔ VCP** | HTTPS | Channel manifest subscription |
| **VPA Embedded Server ↔ Peers** | BON HTTP | Shard serving to peer nodes |
| **VPA UI** | Android native | User-facing playback, subscription management |

---

## 7. Threat Model

| Threat | Mitigation |
|---|---|
| **Peer network infiltrated by adversary nodes** | Publisher signature verification; signatures cannot be forged |
| **Fake content manifests injected** | Invalid Publisher signatures rejected |
| **App node IP exposed to GDS storage nodes** | All requests routed via BON; storage nodes see BON address not device IP |
| **App removed from Play Store** | APK sideload; F-Droid; share-to-install seeding mechanism |
| **Viewing patterns tracked** | Requests randomized across multiple GDS nodes; BON prevents correlation |

---

## 8. Open Questions

| ID | Question | Impact |
|---|---|---|
| OQ-VPA-001 | Should the App support a "priority download" mode to pre-fetch content overnight on WiFi? | Low |
| OQ-VPA-002 | What is the right default behavior when device storage is full? (stop caching vs. evict oldest) | Medium |
| OQ-VPA-003 | Should iOS Phase 2 support relay/server functionality, or viewer-only given background restrictions? | High |
| OQ-VPA-004 | Should the App support parental controls / content age-rating filters? | Medium |
