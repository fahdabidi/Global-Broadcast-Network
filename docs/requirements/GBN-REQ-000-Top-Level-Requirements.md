# GBN-REQ-000 — Global Broadcast Network: Top-Level Requirements

**Document ID:** GBN-REQ-000  
**Version:** 0.1 (Draft)  
**Status:** In Review  
**Last Updated:** 2026-04-07  

---

## Table of Contents

1. [Vision & Mission](#1-vision--mission)
2. [Problem Statement](#2-problem-statement)
3. [System Overview](#3-system-overview)
4. [Stakeholders & Actors](#4-stakeholders--actors)
5. [Top-Level Functional Requirements](#5-top-level-functional-requirements)
6. [Cross-Cutting Non-Functional Requirements](#6-cross-cutting-non-functional-requirements)
7. [Component Decomposition](#7-component-decomposition)
8. [Legal & Compliance Framework](#8-legal--compliance-framework)
9. [Threat Model Overview](#9-threat-model-overview)
10. [Requirements Traceability Matrix](#10-requirements-traceability-matrix)
11. [Open Questions & Decisions](#11-open-questions--decisions)
12. [Glossary](#12-glossary)

---

## 1. Vision & Mission

### 1.1 Vision

A world where any individual can create video content and distribute it to any willing recipient, free from censorship by governments, ISPs, or platform monopolies — while content providers and publishers can build legitimate, legally responsible services on top of the same infrastructure.

### 1.2 Mission

The **Global Broadcast Network (GBN)** is a decentralized, censorship-resistant platform with the following core missions:

- **Protect creators**: Anonymize the identity and origin of video content creators from the moment of creation through delivery.
- **Empower publishers**: Give independent media outlets the ability to receive, assemble, and distribute content that cannot be suppressed.
- **Enable resilient storage**: Store video content across a globally distributed storage network that cannot be selectively purged.
- **Support content providers**: Allow services to curate, present, and stream distributed content to users in a legally compliant manner.
- **Ensure ubiquitous access**: Allow viewers to receive and consume content even when ISPs, firewalls, or geo-blocks would otherwise prevent it.
- **Resist surveillance**: Prevent ISPs and state actors from using Deep Packet Inspection (DPI) to identify which video content is being transmitted to which users.

### 1.3 Design Philosophy

| Principle | Description |
|---|---|
| **Privacy by Default** | All communications are encrypted; no plaintext transmission of content, metadata, or routing information |
| **Resilience over Efficiency** | Prefer distributed, redundant approaches over centralized-but-faster alternatives |
| **Legal Responsibility at the Edges** | The network is neutral; editorial and legal responsibility rests with Publishers and Content Providers |
| **Adaptive to Adversaries** | Pluggable, modular components allow the system to evolve as censorship techniques improve |
| **Progressive Decentralization** | Accept some minimally centralized bootstrap infrastructure; eliminate dependencies over time |

---

## 2. Problem Statement

### 2.1 Current Landscape Failures

| Problem | Impact |
|---|---|
| Centralized video platforms (YouTube, etc.) can deplatform creators unilaterally | Creator content vanishes instantly and irrecoverably |
| ISPs and governments can geo-block or throttle specific domains or IPs | Legitimate content cannot reach its intended audience |
| DPI enables ISPs to identify and block specific video content in transit | Encrypted content is still identifiable by traffic fingerprints |
| Metadata (device IDs, GPS, upload origin IPs) exposes creator identity | Creators in hostile environments face arrest or harm |
| Single-point CDN or hosting failures eliminate access to content globally | No redundancy or failover for critical journalism |
| Torrent networks expose participant IP addresses to adversaries | Viewers and seeders can be tracked and prosecuted |

### 2.2 Scope Boundary

The GBN does **not**:
- Generate or recommend content (this is the Content Provider's responsibility)
- Enforce content moderation globally (except CSAM removal — see §8)
- Provide user authentication or account management at the network level (identities are per-role)
- Replace ordinary internet streaming for non-censored environments (it is a resilience layer, not a CDN competitor)

---

## 3. System Overview

The GBN consists of six major sub-systems, each described in detail by its own requirements document:

```
┌─────────────────────────────────────────────────────────────────────┐
│                     GLOBAL BROADCAST NETWORK                        │
│                                                                     │
│  [Creator] ─► [Media Creation Network] ─► [Publisher]              │
│                        │                       │                    │
│               [Broadcast Overlay Network]  [Distributed Storage]    │
│                        │                       │                    │
│              [Video Playback App] ◄─── [Content Provider]          │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.1 Component Summary

| Component | ID | Primary Function |
|---|---|---|
| Media Creation Network | MCN | Anonymous video chunking, encryption, and multi-hop delivery to Publisher |
| Media Publishing | MPub | Publisher-side decryption, reassembly, curation, and torrent-like publication |
| Globally Distributed Storage | GDS | Redundant, erasure-coded chunk storage across volunteer nodes |
| Video Content Providers | VCP | Curated content discovery, streaming services, legal compliance |
| Video Playback App | VPA | Mobile app for receiving, storing, and playing distributed video content |
| Broadcast Overlay Network | BON | Encrypted overlay with DPI bypass, NAT traversal, and IP renegotiation |

### 3.2 Data Flow Summary

```
Creator → [anonymized, chunked, encrypted video] 
       → [multi-hop MCN relay network]
       → Publisher
       → [decrypt, reassemble, curate]
       → [create torrent metadata + storage manifest]
       → [GDS: distribute chunks with erasure coding]
       → Content Provider (indexes, curates, creates channel)
       → [BON: encrypted broadcast to app nodes]
       → Video Playback App (polls peers, stores metadata, streams chunks)
       → Viewer
```

---

## 4. Stakeholders & Actors

### 4.1 Primary Actors

| Actor | Description | Anonymity | Trust Level |
|---|---|---|---|
| **Creator** | Individual who produces video content; may be in a hostile jurisdiction | Maximum — true identity hidden from all parties | Self-trusted |
| **Publisher** | Independent media outlet or journalist org that receives and publishes Creator content | Semi-anonymous to Creator; public-facing identity | High — KYC optional; holds signing keys |
| **Storage Node** | Any device (server, desktop, mobile) that volunteers to store encrypted content chunks | Pseudo-anonymous by node ID | Medium — untrusted storage; encryption provides integrity |
| **Content Provider** | Business or individual that curates and offers a streaming service on top of GBN | Public identity — legally accountable | Medium — contractual responsibility |
| **Viewer / App User** | End user who consumes content via the Video Playback App | Pseudo-anonymous — no account required | Low — consumer role only |
| **Relay Node** | Any device running the GBN app that volunteers to route encrypted packets for others | Pseudo-anonymous by node ID | Medium — cannot read content |

### 4.2 Adversarial Actors

| Adversary | Capability | Target |
|---|---|---|
| **ISP / National Firewall** | DPI, traffic analysis, IP blocking, throttling, protocol fingerprinting | Detect and block GBN traffic in transit |
| **State Actor** | Subpoena storage nodes, infiltrate Publisher, traffic correlation attacks | Identify Creator, suppress Publisher |
| **Malicious Node** | Inject corrupted chunks, perform Sybil attacks on DHT, sniff relay traffic | Corrupt content, deanonymize peers |
| **Copyright Enforcer** | Monitor swarm IP addresses, send DMCA notices to ISPs | Identify viewers/seeders by IP |
| **Platform Monopolist** | App store removal, payment processor deplatforming | eliminate distribution of the GBN app |

### 4.3 System Roles by Capability

```
High Capability Server:  Publisher Node, Storage Infrastructure Node
Medium Capability:       Dedicated Storage Node, Always-on Relay
Low Capability Mobile:   App User, Mobile Relay/Storage Node
```

---

## 5. Top-Level Functional Requirements

Requirements are tagged with priority: **[M]** Must Have | **[S]** Should Have | **[C]** Could Have

### 5.1 Creator Privacy & Anonymization

| ID | Requirement | Priority |
|---|---|---|
| GBN-FR-001 | The system SHALL strip all device-identifying metadata from video files before transmission | M |
| GBN-FR-002 | The system SHALL anonymize the Creator's IP address such that no relay node or Publisher can determine the Creator's true network origin | M |
| GBN-FR-003 | The system SHALL support optional visual anonymization tools (face blurring, audio distortion) before upload | S |
| GBN-FR-004 | The system SHALL ensure that even if a relay node is compromised, it cannot correlate the Creator's identity with the video content | M |
| GBN-FR-005 | The system SHOULD resist traffic correlation attacks by incorporating cover traffic and timing obfuscation | S |

### 5.2 Content Delivery to Publisher

| ID | Requirement | Priority |
|---|---|---|
| GBN-FR-010 | The system SHALL chunk video files into encrypted packets that can be independently routed across different network paths | M |
| GBN-FR-011 | The system SHALL route video chunks through a multi-hop overlay network that bypasses geo-blocks and ISP firewalls | M |
| GBN-FR-012 | Chunks SHALL be encrypted end-to-end such that only the intended Publisher can decrypt the final assembled content | M |
| GBN-FR-013 | The system SHALL support routing across a volunteer relay node network, where any node can forward packets without reading their content | M |
| GBN-FR-014 | The system SHALL allow a Publisher to be designated by a cryptographic public key, not a static IP address | M |

### 5.3 Publisher Operations

| ID | Requirement | Priority |
|---|---|---|
| GBN-FR-020 | The Publisher SHALL be able to decrypt and reassemble video chunks using their private key | M |
| GBN-FR-021 | The Publisher SHALL be able to curate, approve, annotate, and publish video content to the GDS | M |
| GBN-FR-022 | The Publisher SHALL cryptographically sign all published content to establish content provenance | M |
| GBN-FR-023 | The system SHALL prevent any entity from publishing to the GDS while impersonating a Publisher | M |
| GBN-FR-024 | Publishers SHOULD be able to optionally integrate C2PA content provenance metadata | S |

### 5.4 Distributed Storage

| ID | Requirement | Priority |
|---|---|---|
| GBN-FR-030 | The GDS SHALL store video chunks across a minimum of N geographically distinct storage nodes (N configurable, default N=20) | M |
| GBN-FR-031 | The GDS SHALL use erasure coding (Reed-Solomon or equivalent) so that any k-of-n chunks can reconstruct the full content | M |
| GBN-FR-032 | Storage nodes SHALL NOT have access to plaintext video content | M |
| GBN-FR-033 | The system SHALL maintain a cryptographically verifiable manifest linking content IDs to storage chunks | M |
| GBN-FR-034 | The GDS SHALL be resilient to the loss or unavailability of up to (n-k) nodes without loss of content | M |
| GBN-FR-035 | Publishers SHALL sign storage manifests to establish ownership and enable Publisher-based content filtering | M |
| GBN-FR-036 | Storage nodes SHOULD receive incentivization (reputation, reciprocal storage, or token-based) for contributing storage | S |

### 5.5 Content Providers

| ID | Requirement | Priority |
|---|---|---|
| GBN-FR-040 | Content Providers SHALL be able to search and discover content in the GDS catalog by Publisher, content ID, or metadata tags | M |
| GBN-FR-041 | Content Providers SHALL be able to create curated "channels" that subscribe to one or more Publisher feeds | M |
| GBN-FR-042 | Content Providers SHALL be solely responsible for enforcing copyright compliance on content they present in their services | M |
| GBN-FR-043 | Content Providers SHALL implement and maintain DMCA-compliant takedown workflows for their streaming services | M |
| GBN-FR-044 | Content Providers SHALL be able to apply algorithmic filters to GDS content (by Publisher, date, tags, or custom criteria) | S |
| GBN-FR-045 | The system SHALL provide a standard API for Content Providers to integrate GDS streaming into their own platforms | M |

### 5.6 Video Playback App

| ID | Requirement | Priority |
|---|---|---|
| GBN-FR-050 | The App SHALL receive torrent-like content metadata from connected peer nodes | M |
| GBN-FR-051 | The App SHALL maintain a dynamic peer list of known receivers, updated automatically through peer gossip | M |
| GBN-FR-052 | The App SHALL filter and store content metadata based on the user's Publisher preference settings | M |
| GBN-FR-053 | The App SHALL support peer-to-peer streaming of video chunks from multiple peers simultaneously | M |
| GBN-FR-054 | The App SHALL act as a relay/server node, forwarding content to other App peers | M |
| GBN-FR-055 | The App SHALL support bootstrapping via "share-to-install" seeding (app + initial peer list spread via messaging apps) | M |
| GBN-FR-056 | The App SHOULD operate with minimal battery impact when running as a background relay | S |
| GBN-FR-057 | The App SHOULD support offline caching of video content for playback without network connectivity | S |

### 5.7 Broadcast Overlay Network

| ID | Requirement | Priority |
|---|---|---|
| GBN-FR-060 | The BON SHALL encrypt all inter-node packet payloads and headers to prevent DPI identification of content type | M |
| GBN-FR-061 | The BON SHALL implement pluggable transport adapters (obfs4, WebTunnel, Snowflake) for DPI bypass | M |
| GBN-FR-062 | The BON SHALL implement NAT traversal using ICE/STUN and TURN relay fallback for mobile nodes | M |
| GBN-FR-063 | The BON SHALL support node IP renegotiation so that a node's current IP can be updated across the network when it changes | M |
| GBN-FR-064 | The BON SHALL disguise GBN traffic as ordinary HTTPS traffic to minimize detectability | M |
| GBN-FR-065 | The BON SHOULD support cover traffic generation to resist traffic correlation attacks | S |

### 5.8 Software Update Integrity

| ID | Requirement | Priority |
|---|---|---|
| GBN-FR-070 | All GBN software releases SHALL require M-of-N cryptographic signatures from geographically distributed maintainers before a node accepts the update (minimum: 3-of-5) | M |
| GBN-FR-071 | All GBN components SHALL support deterministic, reproducible builds so that any independent party can verify binary-to-source correspondence | M |
| GBN-FR-072 | GBN source code SHALL be hosted on at least 3 independent platforms; seizure of one platform SHALL NOT disrupt development | M |
| GBN-FR-073 | Software updates SHALL be distributed via phased canary rollout (1 percent initial deployment, 72-hour observation) with network-level behavioral monitoring before general release | M |
| GBN-FR-074 | The GBN protocol SHALL enforce constitutional invariants (encryption, hop-count, signature verification) that no software update is permitted to violate; peers SHALL reject nodes that violate these invariants | M |
| GBN-FR-075 | Old software versions SHALL never be forcibly deprecated; nodes on older versions SHALL continue to interoperate via protocol version negotiation | M |
| GBN-FR-076 | Updates SHALL never be auto-installed; the user MUST explicitly approve after reviewing signature count, build attestations, and canary period results | M |

---

## 6. Cross-Cutting Non-Functional Requirements

### 6.1 Security

| ID | Requirement | Priority |
|---|---|---|
| GBN-NFR-001 | All content encryption SHALL use AES-256-GCM or equivalent authenticated encryption | M |
| GBN-NFR-002 | Key exchange SHALL use Elliptic Curve Diffie-Hellman (ECDH, Curve25519) or equivalent | M |
| GBN-NFR-003 | Digital signatures SHALL use Ed25519 or equivalent | M |
| GBN-NFR-004 | The system SHALL implement forward secrecy for inter-node communication | M |
| GBN-NFR-005 | The system SHALL be resistant to Sybil attacks on the DHT/peer discovery layer | M |
| GBN-NFR-006 | Content integrity SHALL be verified via cryptographic hash (SHA-256 or BLAKE3) at each reassembly step | M |

### 6.2 Privacy

| ID | Requirement | Priority |
|---|---|---|
| GBN-NFR-010 | No component SHALL log IP addresses of Creators, Publishers, or App users in plaintext | M |
| GBN-NFR-011 | The system SHALL not require any personally identifiable information (PII) to operate | M |
| GBN-NFR-012 | Node identifiers SHALL be pseudonymous (derived from public keys, not user identity) | M |
| GBN-NFR-013 | The system SHOULD implement k-anonymity or equivalent for content request patterns | S |

### 6.3 Performance

| ID | Requirement | Priority |
|---|---|---|
| GBN-NFR-020 | Video chunk size SHALL be configurable (default: 512KB–4MB) to balance routing efficiency and reassembly overhead | M |
| GBN-NFR-021 | A 500MB video SHALL be uploadable end-to-end to a Publisher within 30 minutes under typical network conditions | S |
| GBN-NFR-022 | The App SHALL begin playback of a stored video within 5 seconds of user selection | S |
| GBN-NFR-023 | The BON routing layer SHALL add no more than 200ms median latency per hop for control messages | S |

### 6.4 Scalability

| ID | Requirement | Priority |
|---|---|---|
| GBN-NFR-030 | The peer discovery (DHT/gossip) layer SHALL scale to 10 million+ nodes | M |
| GBN-NFR-031 | The GDS SHALL support storage of 1 petabyte+ of content across participating nodes | S |
| GBN-NFR-032 | The MCN relay network SHALL scale horizontally — adding more nodes SHALL increase capacity, not create bottlenecks | M |

### 6.5 Resilience & Availability

| ID | Requirement | Priority |
|---|---|---|
| GBN-NFR-040 | Content SHALL remain available as long as the minimum quorum of storage nodes (k of n) is operational | M |
| GBN-NFR-041 | The system SHALL tolerate the simultaneous loss of up to 50% of active relay nodes without loss of routing capability | M |
| GBN-NFR-042 | The system SHALL function in a degraded mode (store-and-forward) when connectivity is intermittent | S |

### 6.6 Interoperability

| ID | Requirement | Priority |
|---|---|---|
| GBN-NFR-050 | The GBN protocols SHALL be fully documented and open-source to enable third-party client implementations | M |
| GBN-NFR-051 | The content storage format SHALL be compatible with or extensible from BitTorrent/WebTorrent metadata | S |
| GBN-NFR-052 | The App SHALL support standard video formats (MP4/H.264, WebM/VP9) for broad device compatibility | M |

---

## 7. Component Decomposition

Each component has a dedicated requirements document:

| Component | Requirements Doc | Architecture Doc |
|---|---|---|
| Media Creation Network | [GBN-REQ-001](GBN-REQ-001-Media-Creation-Network.md) | [GBN-ARCH-001](../architecture/GBN-ARCH-001-Media-Creation-Network.md) |
| Media Publishing | [GBN-REQ-002](GBN-REQ-002-Media-Publishing.md) | [GBN-ARCH-002](../architecture/GBN-ARCH-002-Media-Publishing.md) |
| Globally Distributed Storage | [GBN-REQ-003](GBN-REQ-003-Global-Distributed-Storage.md) | [GBN-ARCH-003](../architecture/GBN-ARCH-003-Global-Distributed-Storage.md) |
| Video Content Providers | [GBN-REQ-004](GBN-REQ-004-Video-Content-Providers.md) | [GBN-ARCH-004](../architecture/GBN-ARCH-004-Video-Content-Providers.md) |
| Video Playback App | [GBN-REQ-005](GBN-REQ-005-Video-Playback-App.md) | [GBN-ARCH-005](../architecture/GBN-ARCH-005-Video-Playback-App.md) |
| Broadcast Overlay Network | [GBN-REQ-006](GBN-REQ-006-Broadcast-Network.md) | [GBN-ARCH-006](../architecture/GBN-ARCH-006-Broadcast-Network.md) |

---

## 8. Legal & Compliance Framework

> **Disclaimer:** This section is for design guidance only and does not constitute legal advice. Consult qualified legal counsel for jurisdiction-specific compliance.

### 8.1 Liability Architecture

The GBN is designed with a layered liability model:

```
Network Layer (MCN, BON, Relay Nodes)
  → Acts as a passive conduit; no editorial control
  → Analogous to common carrier / ISP under Section 230 (US)

Publisher Layer
  → Actively decides what to publish; exercises editorial judgment
  → Bears editorial responsibility for published content
  → Must implement DMCA notice-and-takedown for their publications

Content Provider Layer
  → Exercises full curatorial control over their streaming service
  → Legally responsible for all content displayed to end users
  → Must enforce copyright, age restrictions, and applicable local laws
```

### 8.2 Key Legal Frameworks

| Jurisdiction | Framework | Implication |
|---|---|---|
| United States | DMCA §512 Safe Harbor | Publishers and Content Providers must implement notice-and-takedown to maintain protection |
| United States | CDA §230 | Network relay nodes and passive storage nodes have strong protection as neutral conduits |
| European Union | Digital Services Act (DSA) | Very Large Online Platforms face additional obligations; smaller services have lighter-touch requirements |
| European Union | GDPR | No PII shall be collected without consent; pseudonymous node IDs do not constitute personal data if not linkable |
| Global | Berne Convention | Copyright subsists at creation; Content Providers must not infringe on existing copyrights |

### 8.3 Absolute Content Prohibitions

Regardless of decentralization, the following content is categorically prohibited at every layer of the GBN:

| Category | Action Required |
|---|---|
| Child Sexual Abuse Material (CSAM) | Storage nodes, Publishers, and Content Providers MUST implement NCMEC PhotoDNA hash matching and remove immediately |
| Content that directly incites imminent violence | Publishers MUST review and reject; Content Providers must not display |

### 8.4 Publisher Registration

- Publishers are identified by cryptographic key pairs, not necessarily legal identities
- For DMCA compliance, Publishers operating in the US SHOULD register a DMCA agent with the Copyright Office
- The GBN protocol itself does not enforce Publisher registration; this is a legal obligation on individual Publishers

---

## 9. Threat Model Overview

### 9.1 Threat Matrix

| Threat | Vector | Mitigation |
|---|---|---|
| **Creator Deanonymization** | Traffic correlation across relay nodes | Multi-hop routing; cover traffic; mixnet-style packet batching |
| **Creator Deanonymization** | Video metadata (GPS, device ID) | Mandatory metadata stripping pre-upload |
| **Content Suppression** | Block Publisher IP | Publisher identified by key, not IP; relayed via BON |
| **Content Removal** | Legal takedown of storage nodes | Erasure coding across N≥20 nodes in multiple jurisdictions |
| **Censorship of GBN Traffic** | DPI identifies GBN protocol | Pluggable transports; traffic obfuscation to look like HTTPS |
| **Sybil Attack** | Malicious nodes dominate DHT | DHT with proof-of-work or trust-weighted peer selection |
| **Chunk Corruption** | Malicious storage node serves bad data | Per-chunk cryptographic hash verification; multiple source redundancy |
| **IP Exposure of Viewers** | Monitoring BitTorrent swarm IPs | BON wraps all swarm traffic; no plaintext IP exposure |
| **App Store Removal** | Platform removes official app | APK sideloading; F-Droid; app distributed via GBN seeding protocol |
| **Death Update (Supply Chain)** | State actor compromises repo, CI/CD, or coerces developer to push malicious update | M-of-N multi-signature update governance; reproducible builds; canary rollout; protocol constitution invariants (see [GBN-SEC-007](../security/GBN-SEC-007-Software-Supply-Chain.md)) |

### 9.2 Trust Boundaries

```
[Creator] → Trusts: Their own device, the encryption protocol
[Relay Node] → Sees: Encrypted packets only; source/dest obfuscated
[Publisher] → Trusts: Their own private key; sees: plaintext content after decryption
[Storage Node] → Sees: Encrypted chunks only; cannot reconstruct content
[Content Provider] → Trusts: Publisher signatures; sees: plaintext metadata + streams chunks to viewers
[App User] → Trusts: Publisher signatures for content selection; peers for chunk delivery
```

---

## 10. Requirements Traceability Matrix

| Top-Level Req | Addressed By Component | Sub-Req ID(s) |
|---|---|---|
| GBN-FR-001 to 005 | Media Creation Network | MCN-FR-001 to 020 |
| GBN-FR-010 to 014 | Media Creation Network + BON | MCN-FR-021 to 040, BON-FR-001 to 020 |
| GBN-FR-020 to 024 | Media Publishing | MPub-FR-001 to 030 |
| GBN-FR-030 to 036 | Globally Distributed Storage | GDS-FR-001 to 040 |
| GBN-FR-040 to 045 | Video Content Providers | VCP-FR-001 to 030 |
| GBN-FR-050 to 057 | Video Playback App | VPA-FR-001 to 040 |
| GBN-FR-060 to 065 | Broadcast Overlay Network | BON-FR-001 to 030 |

---

## 11. Open Questions & Decisions

| ID | Question | Impact | Status |
|---|---|---|---|
| OQ-001 | Should storage nodes receive token-based incentives (blockchain-style), reputation-based, or purely reciprocal storage incentives? | High — affects GDS architecture significantly | **Open** |
| OQ-002 | Should Creator/Publisher/App User identities use a unified DID (Decentralized Identifier) framework, or separate per-role key systems? | High — affects identity layer across all components | **Open** |
| OQ-003 | Target mobile platforms: Android-only (phase 1) or Android + iOS simultaneously? | High — iOS severely restricts background services | **Decision needed** |
| OQ-004 | Minimum acceptable centralized bootstrap infrastructure (STUN/TURN, initial peer seeds)? | Medium — affects cold-start resilience | **Open** |
| OQ-005 | Should the GBN include network-level CSAM filtering (hash matching at storage nodes), or delegate entirely to Publishers and Content Providers? | High — legal and ethical | **Open** |
| OQ-006 | Should the system support live streaming or only stored-video distribution (Phase 1)? | Medium — live streaming requires very different chunking and latency requirements | **Open** |

---

## 12. Glossary

| Term | Definition |
|---|---|
| **BON** | Broadcast Overlay Network — the encrypted, DPI-resistant transport layer |
| **C2PA** | Coalition for Content Provenance and Authenticity — standard for embedding content provenance |
| **CGNAT** | Carrier-Grade NAT — large-scale NAT used by mobile operators |
| **CID** | Content Identifier — cryptographic hash-based address (IPFS-style) |
| **Creator** | Individual who produces video content and submits it anonymously via the MCN |
| **DHT** | Distributed Hash Table — decentralized key-value store for peer discovery |
| **DPI** | Deep Packet Inspection — technique to analyze packet contents in transit |
| **Erasure Coding** | A data protection method enabling reconstruction from a subset of encoded chunks |
| **GDS** | Globally Distributed Storage — the distributed storage network |
| **ICE** | Interactive Connectivity Establishment — framework for NAT traversal |
| **MCN** | Media Creation Network — the anonymous upload pipeline |
| **Mixnet** | Mix Network — a routing layer that batches and mixes packets to prevent correlation |
| **MPub** | Media Publishing component |
| **NAT** | Network Address Translation — modifies IP addresses in transit; used by routers/carriers |
| **Pluggable Transport** | A modular protocol adapter that disguises traffic as another protocol |
| **Publisher** | An independent media outlet that receives, assembles, and publishes Creator content |
| **Reed-Solomon** | An erasure coding algorithm that can reconstruct data from a subset of encoded shards |
| **Relay Node** | A GBN network participant that forwards encrypted packets without reading them |
| **STUN** | Session Traversal Utilities for NAT — discovers a device's public IP/port |
| **TURN** | Traversal Using Relays around NAT — relay server for when direct P2P fails |
| **VCP** | Video Content Provider — a service that curates and streams GBN content |
| **VPA** | Video Playback App — the end-user mobile application |
| **WebTunnel** | A pluggable transport that disguises traffic as ordinary HTTPS web browsing |
