# GBN-SEC-006 — Product Security Document: Broadcast Overlay Network

**Document ID:** GBN-SEC-006  
**Component:** Broadcast Overlay Network (BON)  
**Status:** V1.0  

---

## 1. Executive Summary

The Broadcast Overlay Network (BON) is the circulatory system of the GBN. Every component depends on it. It serves two distinct yet overlapping security purposes: **Transport Obfuscation** (hiding the *nature* of the data from firewalls) and **Onion Routing** (hiding the *parties involved* from intelligence agencies).

Because the network connects untrusted mobile devices, desktop clients, and high-capacity servers globally, the BON relies heavily on NAT traversal, ephemeral cryptography, and robust error handling to maintain the illusion of a monolithic, secure tunnel spanning a volatile and highly adversarial internet.

## 2. Security Model & Trust Boundaries

### 2.1 Trust Assumptions
* **Trusted:** Mathematical hardness of X25519 ECDH and ChaCha20-Poly1305. The entropy generator of the host OS.
* **Untrusted:** Every ISP, national firewall, peering point, cloud provider, and every individual relay node in the chain.

### 2.2 Security Architecture
The BON isolates traffic using a 4-layer matryoshka design:
1. **Pluggable Transport (Layer 1):** WebTunnel or obfs4 wrapper. Makes traffic look like standard HTTPS or random noise to defeat DPI (Deep Packet Inspection).
2. **Noise Session (Layer 2):** Point-to-point authenticated encryption. If a TLS session provides transport, Noise_XX provides perfect forward secrecy specifically between Node A and Node B.
3. **Onion Routing (Layer 3):** Three hops (Guard, Middle, Exit). A relay decrypts its routing layer, sees the instruction to "forward payload to next IP," and passes it on.
4. **Application Data (Layer 4):** The actual video chunks or DHT messages.

---

## 3. Attack Resistance (Mitigated Threats)

### 3.1 Resistance to Disablement & Censorship
Authoritarian firewalls (e.g., China's GFW or Russia's Roskomnadzor) employ active probing. If they detect unfamiliar traffic, they send a probe. If the server responds in an unexpected way, the IP is blacklisted. 

The BON mitigates this through WebTunnel Active Probe Defense. The WebTunnel relay listens on port 443. If an adversary sends a standard HTTPS request, the relay serves a legitimate, boring website. The relay *only* behaves as a BON node if the client presents the proper hidden cryptographic handshake within the WebSocket URI. Active probes are completely neutralized.

### 3.2 Resistance to Anonymity Circumvention
Even if an adversary compromises a relay node, they learn almost nothing. By design, tearing open layer 2 (Noise) only gives the relay access to layer 3 (Onion). They can see that Node A sent data to Node C, but because they are Middle nodes, they have no idea if Node A is a creator or just another relay, nor if Node C is the publisher or an exit node. The anonymity set is preserved mathematically.

---

## 4. Formal Threat Model (STRIDE)

| Threat Type | Vector | Mitigation Strategy |
|---|---|---|
| **Spoofing** | Adversary attempts to hijack a relay connection | The Noise_XX handshake mutually authenticates both nodes using their static Ed25519 keys alongside ephemeral keys. |
| **Tampering** | ISP attempts to alter bytes in transit | Layer 2 is encrypted with ChaCha20-Poly1305. Any alteration immediately causes MAC verification failure and drops the connection. |
| **Repudiation** | Node drops traffic but denies doing so | The MCN's separate reverse-ACK circuit detects dropped packets, penalizing the reputation of nodes in non-performing chains. |
| **Information Disclosure** | Police extract long-term keys from a seized relay to read past traffic | Perfect Forward Secrecy (PFS). Because session keys are derived via ephemeral ECDH, discovering the long-term identity key today does not allow decryption of yesterday's captured PCAP files. |
| **Denial of Service** | Adversary runs relays and deliberately drops all traffic (Blackholing) | Circuit testing and reputation systems constantly monitor relays. Relays that fail tests or demonstrate high latency are deprioritized by client Circuit Managers. |
| **Elevation of Privilege** | Transport layer vulnerability exploited to execute code | The Pluggable Transport stack is written in memory-safe Rust, minimizing common buffer overflows or use-after-free RCEs. |

---

## 5. Unmitigated Threats & Fatal Vulnerabilities

The BON is extremely robust against local and regional adversaries, but it possesses well-documented theoretical and practical weaknesses against globally scaled attacks.

### 5.1 Global Passive Adversary (GPA) Flow Correlation
* **Description:** An adversary like the NSA monitors the entry traffic into the BON network and the exit traffic from the BON network across thousands of ISPs simultaneously.
* **Why it succeeds:** Onion routing is a low-latency network design. A packet entering a Guard node emerges from the Exit node a few hundred milliseconds later. By statistically correlating timing patterns, packet sizes, and transmission rates at both ends of the chain, a GPA can definitively link an anonymous Creator to a known Publisher without breaking any cryptography.
* **Status:** Partially Mitigated. The BON remains vulnerable to correlation at a protocol level, but the **Publisher Edge Ingestion** architecture (where Publishers deploy multiple, geographically distributed ingestion IP addresses) significantly smears the exit-flow data. An intelligence agency sees traffic entering the BON, but no single IP receives the corresponding volume of data, confounding standard timing attacks. Absolute 100% resistance would require a Mixnet, but the edge nodes make practical traffic analysis exponentially harder.

### 5.2 Guard Node Co-location
* **Description:** The Creator's ISP and the randomly selected Guard Node's hosting provider are effectively controlled by the same state entity.
* **Why it succeeds:** If the adversary controls the first hop *and* the Creator's local network, they can correlate the traffic much faster, launching targeted traffic shaping attacks to watermark the circuit. Furthermore, if a Creator inside a hostile state connects to a domestic Guard Node, that Guard Node remains trapped by the national firewall and may be unable to route traffic to the international Middle and Exit Nodes.
* **Status:** Partially mitigated. The BON enforces **Jurisdiction-Aware Guard Selection**. A client behind a national firewall uses Pluggable Transports (WebTunnel/obfs4) — which are natively compiled into the BON core library rather than relying on external proxies — strictly to tunnel out to a Guard Node physically located *outside* the adversary's geo-fenced territory. This simultaneously escapes the firewall and breaks domestic traffic correlation.

### 5.3 Complete Internet Shutdown (The "Whiteout")
* **Description:** A government physically unplugs international fiber connections, heavily throttles BGP routes, or shuts down mobile 4G/5G towers.
* **Why it succeeds:** Overlays are parasitic—they require a functional underlying host network (TCP/IP/UDP). If the physical layer is destroyed, the BON cannot establish circuits. 
* **Status:** Unmitigated. The BON currently requires a working internet connection. Future architectures may require offline mesh networking integrations to succeed in these scenarios.
