# GBN-REQ-006 — Broadcast Overlay Network: Requirements

**Document ID:** GBN-REQ-006  
**Version:** 0.1 (Draft)  
**Status:** In Review  
**Last Updated:** 2026-04-07  
**Parent:** [GBN-REQ-000](GBN-REQ-000-Top-Level-Requirements.md)  
**Architecture:** [GBN-ARCH-006](../architecture/GBN-ARCH-006-Broadcast-Network.md)

---

## 1. Overview

The **Broadcast Overlay Network (BON)** is the transport foundation of the entire GBN system. Every other component — the MCN, MPub, GDS, VCP, and VPA — uses the BON to communicate. The BON's job is therefore singular but critical: **make GBN traffic indistinguishable from ordinary internet traffic** while providing secure, authenticated, and routed connectivity between nodes regardless of geographic or network censorship.

The BON solves three fundamental problems:
1. **Censorship Resistance**: ISPs and national firewalls cannot block GBN traffic because it is disguised as ordinary HTTPS/WebRTC traffic
2. **Anonymity**: No node can determine both the source and destination of a communication
3. **Mobility**: Mobile nodes whose IP addresses change frequently are tracked by their Ed25519 node ID, not their IP

### 1.1 Scope

| In Scope | Out of Scope |
|---|---|
| Encrypted inter-node communication | Application-layer content decisions |
| DPI bypass via pluggable transports | Content storage |
| Multi-hop onion routing | Video decoding or playback |
| NAT traversal (ICE/STUN/TURN) | Publisher or Creator identity management |
| Node IP renegotiation and address updates | DHT content indexing (DHT is a higher-layer protocol running over BON) |
| Cover traffic generation | |

---

## 2. Stakeholders & Actors

| Actor | Role in BON |
|---|---|
| **Any GBN Node** | Uses BON as its transport layer |
| **Relay Node** | Volunteers to forward BON packets for others |
| **ISP / National Firewall** | Primary adversary; attempts to detect and block BON traffic |
| **Mobile App (VPA)** | BON client with CGNAT and dynamic IP challenges |
| **Bootstrap Node** | Initial known peers providing first network entry point |

---

## 3. Functional Requirements

### 3.1 Transport Encryption

| ID | Requirement | Priority |
|---|---|---|
| BON-FR-001 | All inter-node communication SHALL be encrypted with a session key established via Noise_XX handshake | **Must** |
| BON-FR-002 | The Noise handshake SHALL establish forward-secret session keys, so past traffic cannot be decrypted if long-term keys are compromised | **Must** |
| BON-FR-003 | Session keys SHALL be rotated after every 1 hour or 1GB of traffic, whichever comes first | **Should** |
| BON-FR-004 | The handshake SHALL authenticate both parties by their Ed25519 node keys | **Must** |
| BON-FR-005 | The BON SHALL encrypt the packet payload AND routing headers (onion layer) so that interceptors cannot determine packet routing even from the outer headers | **Must** |

### 3.2 Pluggable Transport Layer

| ID | Requirement | Priority |
|---|---|---|
| BON-FR-010 | The BON SHALL implement a pluggable transport architecture compatible with Tor's Pluggable Transport specification | **Must** |
| BON-FR-011 | The BON SHALL ship with the following built-in transports: **WebTunnel** (disguise as HTTPS websocket) and **obfs4** (randomized byte stream) | **Must** |
| BON-FR-012 | The BON SHALL support **Snowflake** (WebRTC-based) as an optional transport for environments where HTTPS is blocked | **Should** |
| BON-FR-013 | New pluggable transport adapters SHALL be loadable without requiring a BON core update (dynamic plugin architecture) | **Should** |
| BON-FR-014 | The BON SHALL automatically select the most appropriate transport based on detected network conditions | **Should** |
| BON-FR-015 | WebTunnel transport SHALL disguise BON traffic as HTTPS WebSocket connections to a legitimate-looking HTTPS server | **Must** |
| BON-FR-016 | The BON SHALL resist active probing — a non-GBN client connecting to a BON transport endpoint SHALL receive a plausible non-GBN response | **Must** |

### 3.3 Onion Routing

| ID | Requirement | Priority |
|---|---|---|
| BON-FR-020 | The BON SHALL route packets through a minimum of 3 relay hops by default | **Must** |
| BON-FR-021 | Each relay in an onion circuit SHALL know only: its upstream peer and downstream peer | **Must** |
| BON-FR-022 | Each relay hop's routing information SHALL be encrypted with that relay's public key (layered encryption) | **Must** |
| BON-FR-023 | Onion circuits SHALL be pre-constructed and maintained to avoid per-packet circuit-building latency | **Should** |
| BON-FR-024 | Circuit construction SHALL randomly select relay nodes from the available pool, weighted by uptime and reputation | **Must** |
| BON-FR-025 | The BON SHALL implement circuit isolation: different application streams (e.g., MCN upload vs. VPA gossip) SHALL use different circuits | **Must** |
| BON-FR-026 | The BON SHOULD implement "guard nodes" — a stable small set of first-hop relays chosen for long-term use to reduce first-hop deanonymization risk | **Should** |
| BON-FR-027 | Guard Node selection SHALL be jurisdiction-aware. Clients operating behind a known national firewall MUST select Guard Nodes physically located outside that geo-fenced jurisdiction to ensure the Guard can freely route traffic to the Middle node | **Must** |

### 3.4 NAT Traversal

| ID | Requirement | Priority |
|---|---|---|
| BON-FR-030 | The BON SHALL implement ICE (Interactive Connectivity Establishment) for establishing direct P2P connections between nodes | **Must** |
| BON-FR-031 | The BON SHALL use STUN to discover nodes' public IP addresses | **Must** |
| BON-FR-032 | The BON SHALL use TURN relay servers as a fallback when direct P2P connections fail (e.g., behind CGNAT) | **Must** |
| BON-FR-033 | TURN server addresses SHALL be configurable; the GBN project SHALL operate a baseline set of TURN servers in multiple regions | **Must** |
| BON-FR-034 | The BON SHALL implement UDP hole punching for peers behind standard NATs | **Should** |
| BON-FR-035 | The BON SHALL support IPv6 in addition to IPv4 | **Should** |

### 3.5 Node Identity & IP Renegotiation

| ID | Requirement | Priority |
|---|---|---|
| BON-FR-040 | Every BON node SHALL be identified by an Ed25519 public key (node ID), independent of its current IP address | **Must** |
| BON-FR-041 | When a node's IP address changes, it SHALL broadcast a signed IP renegotiation announcement to its active peer set | **Must** |
| BON-FR-042 | IP renegotiation announcements SHALL be propagated via gossip to at least 2 hops beyond the announcing node | **Must** |
| BON-FR-043 | Receiving nodes SHALL validate the Ed25519 signature on a renegotiation announcement before updating their peer address table | **Must** |
| BON-FR-044 | Nodes SHALL timestamp renegotiation announcements; stale announcements (>5 minutes old) SHALL be rejected | **Must** |
| BON-FR-045 | The BON SHALL maintain a local routing table mapping node IDs to their most recently confirmed BON addresses | **Must** |

### 3.6 Cover Traffic

| ID | Requirement | Priority |
|---|---|---|
| BON-FR-050 | The BON SHOULD generate configurable cover traffic (dummy encrypted packets) during idle periods to prevent traffic-silence correlation | **Should** |
| BON-FR-051 | Cover traffic packets SHALL be indistinguishable from real packets to an outside observer | **Should** |
| BON-FR-052 | Cover traffic volume SHALL be configurable by the user (off / low / medium / match-real-pattern) | **Could** |
| BON-FR-053 | Cover traffic SHALL be generated at the transport level, not the application level — applications need not be aware of it | **Should** |

### 3.7 Relay Node Operations

| ID | Requirement | Priority |
|---|---|---|
| BON-FR-060 | Any BON-capable device MAY register as a relay node by announcing itself to the DHT | **Must** |
| BON-FR-061 | Relay node registration SHALL include: node ID, supported transports, geographic region, and uptime score | **Must** |
| BON-FR-062 | Relay nodes SHALL forward encrypted packets to the next hop without decrypting the payload | **Must** |
| BON-FR-063 | Relay nodes SHALL rate-limit forwarded traffic to prevent abuse | **Must** |
| BON-FR-064 | Relay nodes SHALL implement a simple reputation reporting mechanism — abnormally behaving relays are blacklisted by circuit builders | **Should** |

---

## 4. Non-Functional Requirements

| ID | Requirement | Priority |
|---|---|---|
| BON-NFR-001 | BON per-hop latency overhead SHALL be ≤ 50ms median on a well-connected relay | **Should** |
| BON-NFR-002 | The BON protocol library SHALL be implemented in Rust for memory safety and performance | **Must** |
| BON-NFR-003 | The BON relay daemon SHALL handle ≥ 1000 simultaneous circuits on a standard VPS (2 vCPU, 2GB RAM) | **Should** |
| BON-NFR-004 | The BON library SHALL have a stable C FFI interface for integration with non-Rust components | **Must** |
| BON-NFR-005 | Circuit construction (finding path + building onion) SHALL complete within 2 seconds | **Should** |
| BON-NFR-006 | The pluggable transport mechanism SHALL be independently auditable by the security community | **Must** |

---

## 5. Data Requirements

### 5.1 BON Packet Format

```
BONPacket {
    // Outer (transport layer): encrypted with next-hop session key (Noise_XX)
    hop_payload: bytes {
        // After decryption:
        circuit_id:     u64           // identifies the onion circuit
        command:        enum { Relay, End, Created, Extended }
        relay_payload:  bytes {
            // Onion-encrypted: each hop peels one layer
            // At final destination: application data
        }
    }
}
```

### 5.2 Node Address Record

```
NodeAddressRecord {
    node_id:      Ed25519PublicKey
    addresses:    [
        {
            protocol:  enum { WebTunnel, obfs4, Snowflake, TCP }
            host:      string   // obfuscated or WebSocket URL
            port:      u16
        }
    ]
    timestamp:    Unix timestamp
    signature:    Ed25519Signature
}
```

---

## 6. Interface Requirements

| Interface | Type | Description |
|---|---|---|
| **BON ↔ Application Layer** | Local socket / API | High-level send/receive API for MCN, VPA, GDS |
| **BON ↔ BON (inter-node)** | Pluggable transport (WebTunnel / obfs4) | Encrypted packet forwarding |
| **BON ↔ DHT** | Internal | Node registration; relay node discovery |
| **BON ↔ STUN/TURN** | Standard WebRTC | NAT traversal |
| **BON ↔ Pluggable Transport Plugin** | Dynamic library / process | Transport adapter interface |

---

## 7. Threat Model

| Threat | Severity | Mitigation |
|---|---|---|
| **DPI identifies GBN traffic by pattern** | Critical | Pluggable transports mimic HTTPS/WebRTC |
| **Active probing reveals BON endpoint** | High | BON-FR-016: non-GBN clients get plausible non-GBN response |
| **Traffic correlation (timing attack)** | High | Cover traffic; timing jitter in routing |
| **Sybil attack on relay pool** | High | Reputation weighting; proof-of-work registration |
| **Malicious relay logs inter-hop addresses** | Medium | Onion encryption prevents relay from seeing source+dest simultaneously |
| **TURN server becomes single point of failure** | Medium | Multiple TURN servers in diverse jurisdictions |
| **Node ID spoofing in renegotiation** | Medium | Ed25519 signature required; timestamp prevents replay |

---

## 8. Open Questions

| ID | Question | Impact |
|---|---|---|
| OQ-BON-001 | Should the BON implement Conjure transport (uses ISP unused address space) for the most aggressively censored regions? | High — very technically complex |
| OQ-BON-002 | Should the BON support a "direct mode" (no onion routing) for trusted low-latency connections where anonymity is not required? | Medium — performance tradeoff |
| OQ-BON-003 | How should the system handle jurisdictions that block WebRTC entirely (Snowflake becomes unavailable)? | High — affects BON resilience |
| OQ-BON-004 | Should GBN operate its own TURN server infrastructure or rely on existing open TURN infrastructure? | Medium — cost and operational burden |
