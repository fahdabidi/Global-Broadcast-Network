# GBN-SEC-003 — Product Security Document: Globally Distributed Storage

**Document ID:** GBN-SEC-003  
**Component:** Globally Distributed Storage (GDS)  
**Status:** V1.0  

---

## 1. Executive Summary

The Globally Distributed Storage (GDS) forms the resilient memory bank of the network. Comprised of volunteer storage nodes located worldwide, the GDS ensures that once a Publisher commits content, it is nearly impossible for any single government, ISP, or physical catastrophe to remove it.

The security objective of the GDS is **availability and integrity without exposure**. Storage nodes must be able to serve content reliably without ever knowing what they are serving. By separating the *storage* of data from the *keys* to decrypt it, the GDS provides strong legal and operational protection to its volunteer operators.

## 2. Security Model & Trust Boundaries

### 2.1 Trust Assumptions
* **Trusted:** The mathematics behind BLAKE3 (hashing) and Reed-Solomon (erasure coding).
* **Untrusted:** Every individual storage node (they may drop data, lie about capacity, or serve corrupt data). The Distributed Hash Table (DHT) participants.

### 2.2 Security Architecture
The GDS solves untrusted storage through two mechanisms:
1. **Erasure Coding (Resilience):** A video file is split into 20 shards, but only 14 are required to reconstruct it. If 6 nodes fail, vanish, or are arrested simultaneously, the content survives.
2. **Content Addressing (Integrity):** Every shard is requested by its BLAKE3 cryptographic hash. A malicious node cannot serve altered or censored content because the requesting client drops any shard whose hash does not perfectly match the requested ID. 
3. **Blind Storage:** Shards consist of AES-256-GCM ciphertext. A storage node administrator has no technical means to view the content they are hosting.

---

## 3. Attack Resistance (Mitigated Threats)

### 3.1 Resistance to Disablement & Censorship
If a hostile government targets the GDS by issuing physical warrants to seize servers hosting a controversial video, the attack fails on two fronts. 
First, because of Reed-Solomon encoding, the adversary must successfully locate and simultaneously seize at least 7 nodes (n-k+1) holding shards of that specific video before the network detects the node attrition and re-replicates the missing shards elsewhere. 
Second, a seized server yields no plaintext video, nor the decryption keys, protecting the operator from liability.

### 3.2 Resistance to Anonymity Circumvention
An adversary attempting to map the viewer network by running a honeypot storage node will fail. When a Viewer App requests a shard from a storage node, the request routes through the Broadcast Overlay Network (BON). The storage node sees only the exit relay of the BON circuit; they have zero visibility into the actual IP address or identity of the viewer requesting the shard.

---

## 4. Formal Threat Model (STRIDE)

| Threat Type | Vector | Mitigation Strategy |
|---|---|---|
| **Spoofing** | Adversary injects garbage shards claiming they belong to a video | Impossible. The shard ID *is* its BLAKE3 hash. Garbage data computes to a different hash and is instantly rejected. |
| **Tampering** | Hostile node alters bits in the shard to corrupt the video | Altered bits change the cryptographic hash. The client drops the shard and requests it from a different peer, flagging the malicious node's reputation. |
| **Repudiation** | Node pledges 1TB quota but deletes shards to save space | Nodes must periodically sign "Heartbeat" announcements. If a node cannot provide a requested shard, their reputation score is slashed, preventing further shard allocations. |
| **Information Disclosure** | Storage operator scans their own disk to find illicit content | By design, shards are indistinguishable, encrypted blobs. The operator cannot determine the content, title, or publisher of the shards they host. |
| **Denial of Service** | Sybil Attack: Adversary spins up 10,000 fake nodes to dominate the DHT | Node registration requires proof-of-work, making Sybil swarms expensive. Shards are distributed uniformly based on DHT distance, preventing targeted clustering. |
| **Elevation of Privilege** | Attacker poisons the DHT routing table (Eclipse Attack) | Kademlia DHT queries use disjoint paths. Content retrieval validates Publisher signatures, so an attacker controlling routing paths still cannot forge fake content manifests. |

---

## 5. Unmitigated Threats & Fatal Vulnerabilities

The decentralized nature of the GDS introduces specific operational vulnerabilities that cannot be entirely solved by cryptography.

### 5.1 Catastrophic Node Attrition (The Quorum Loss)
* **Description:** A massive, coordinated takedown event or a bug causes more than 30% of the nodes hosting a specific video's shards to go offline simultaneously, before the system has time to re-replicate.
* **Why it succeeds:** If 7 out of 20 shards are destroyed simultaneously, the mathematical threshold for Reed-Solomon reconstruction is broken. The original data is permanently lost.
* **Status:** Unmitigated. The only defense is increasing `n` (e.g., placing 40 shards) at the cost of massive network overhead.

### 5.2 Network-Wide Sybil Exhaustion
* **Description:** A massively well-funded adversary (e.g., a nation-state) spins up millions of legitimate-acting storage nodes, dwarfing the volunteer footprint, and slowly capturing the majority of all new shard distributions.
* **Why it succeeds:** If 95% of the storage capacity belongs to a hostile intelligence agency, there is a high probability that more than 6 shards of any given video will land exclusively on their servers. When commanded, the agency simply deletes the shards simultaneously.
* **Status:** Unmitigated without a centralized trust/identity layer (which violates GBN's decentralization goals). Proof-of-work limits casual attackers, but not nation-states.

### 5.3 Coordinated Regulatory Liability (CSAM Sweeps)
* **Description:** Despite the "blind storage" architecture, coordinated global law enforcement agencies pass resolutions declaring that hosting the encrypted shards, even if unreadable by the host, is illegal if the content hash matches known illicit material (e.g., global PhotoDNA enforcement).
* **Why it succeeds:** The defense of "I couldn't read the data" has not been universally tested in global anti-terrorism or CSAM laws. The chilling effect could drive the majority of volunteer storage operators offline, collapsing the GDS.
* **Status:** Unmitigated on a technical level. This is a severe legal risk to the network's sustainability.
