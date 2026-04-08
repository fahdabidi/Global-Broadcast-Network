# GBN-SEC-004 — Product Security Document: Video Content Providers

**Document ID:** GBN-SEC-004  
**Component:** Video Content Providers (VCP)  
**Status:** V1.0  

---

## 1. Executive Summary

The Video Content Provider (VCP) is the public, curated face of the GBN. While the rest of the network optimizes for anonymity and censorship resistance, the VCP component acts as a bridge to reality. It organizes raw distributed storage fragments into playable channels and interfaces directly with the consuming public and legal authorities.

Because a VCP curates what its viewers see, **it holds legal liability for its selections**. Therefore, the primary security objective of the VCP is to securely isolate viewer privacy while maintaining rock-solid tools for legal compliance (like DMCA takedowns) and editorial content exclusion.

## 2. Security Model & Trust Boundaries

### 2.1 Trust Assumptions
* **Trusted:** The GDS manifests (when bearing a valid Publisher signature). The VCP operator's internal database.
* **Untrusted:** The viewer's client machine. Outside copyright claimants (who may file fraudulent DMCA notices). Deep Packet Inspection by the viewer's ISP.

### 2.2 Security Architecture
The VCP operates primarily on traditional web security models (HTTPS, REST APIs, WebSockets) integrated with the GBN core. It queries the GDS DHT for signed manifests, pulls shards from storage nodes (via BON or direct), decodes them, and packages them as standard HLS/DASH video streams. 

Crucially, the VCP is not an open proxy. It decrypts content on the server side using Publisher-provided keys and transmits standard, DRM-compatible streams to the viewer. This ensures the VCP maintains control over its bandwidth and content distribution policies.

---

## 3. Attack Resistance (Mitigated Threats)

### 3.1 Resistance to Disablement & Censorship
Unlike the GBN core, if a VCP uses standard HTTP streaming, it *can* be easily blocked by a national firewall via DNS blacklisting or IP blocking. To resist this, the VCP architecture includes an **optional BON delivery pipeline**. If the generic domain is blocked, the VCP can stream HLS segments back *through* the Broadcast Overlay Network, bypassing the firewall and reaching viewers in hostile regions securely.

### 3.2 Resistance to Anonymity Circumvention
The VCP protects its viewer base by explicitly stripping IP logs from its analytics server and enforcing HTTPS delivery. If streaming over BON, the VCP server cannot even see the viewer's real IP address. This prevents hostile governments from subpoenaing the VCP's logs to discover who is watching illicit or dissenting broadcasts.

---

## 4. Formal Threat Model (STRIDE)

| Threat Type | Vector | Mitigation Strategy |
|---|---|---|
| **Spoofing** | Attacker tricks the VCP into indexing a fake video | The VCP validates the Ed25519 signature on every manifest pulled from the DHT. Unsigned or invalidly signed manifests corresponding to a Publisher are dropped instantly. |
| **Tampering** | Man-in-the-middle alters the video stream directed at a viewer | Standard HTTPS TLS 1.3 encryption for clearweb; Noise_XX authenticated encryption for BON streams. |
| **Repudiation** | Copyright holder claims VCP ignored a takedown request | The VCP uses an internal, immutable DMCA log database that records the exact timestamp of notice receipt, validation, and automated channel exclusion. |
| **Information Disclosure** | Database breach exposes viewer watch habits | The VCP Architecture mandates that no viewer PII or IP addresses be retained in analytics. Only anonymized, aggregated view counts are persisted. |
| **Denial of Service** | Volumetric DDoS attack against the VCP API layer | Because the VCP API acts like a standard web service, it sits behind traditional edge protections (e.g., Cloudflare WAF or AWS Shield), absorbing mass volumetric attacks before they hit the stream packager. |
| **Elevation of Privilege** | Malicious viewer bypasses the age-gate or subscription firewall | Stream endpoints require cryptographically signed JSON Web Tokens (JWT) bound to the viewer's session. Unauthenticated requests to `.m3u8` or `.ts` files are rejected. |

---

## 5. Unmitigated Threats & Fatal Vulnerabilities

Because the VCP is a public-facing entity with a known legal presence, it is uniquely vulnerable to non-technical attacks that the rest of the decentralized network ignores.

### 5.1 Regulatory Takedown / Deplatforming
* **Description:** A government passes a law declaring the VCP illegal, seizes its domains, forces its cloud host (e.g., AWS) to delete its servers, and freezes its bank accounts.
* **Why it succeeds:** The VCP is typically a registered business or publicly known entity in order to process subscriptions or handle DMCA requests. It lives in the physical and legal world. 
* **Status:** Unmitigated. If the VCP is legally crushed, its specific channels go offline. However, the underlying content remains safe in the GDS, allowing viewers with the Video Playback App to bypass the VCP and reconstruct the raw feeds themselves.

### 5.2 Copyright Troll / Blanket DMCA Abuse
* **Description:** A malicious actor automates the filing of thousands of false DMCA takedown requests per day against the VCP's content library, hoping to censor legitimate content or bankrupt the VCP in administrative costs.
* **Why it succeeds:** Under 17 U.S.C. §512, the VCP must act expeditiously to remove content upon receiving a properly formatted notice. If the volume is too high to manually review, the VCP is forced to automate removals, resulting in aggressive censorship by exhaustion.
* **Status:** Partially unmitigated. VCPs can implement spam-detection and penalty systems for known bad actors, but the legal framework inherently favors the claimant.

### 5.3 Exit Node Content Key Exposure
* **Description:** An attacker gains access to the VCP's central Key Vault, extracting the Publisher-provided AES content keys.
* **Why it succeeds:** The VCP must maintain plaintext content keys in memory to decrypt GDS shards and package them into HLS segments on the fly. If the VCP infrastructure is breached, the keys to vast swaths of the network's content are exposed.
* **Status:** Mitigated by standard cloud security practices, but technically unmitigated against sophisticated server breaches.
