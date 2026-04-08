# GBN-SEC-002 — Product Security Document: Media Publishing

**Document ID:** GBN-SEC-002  
**Component:** Media Publishing (MPub)  
**Status:** V1.0  

---

## 1. Executive Summary

The Media Publishing (MPub) component acts as the cryptographic and editorial bedrock of the Global Broadcast Network. It is operated by independent media outlets, journalists, or curators. Unlike the Creator, the Publisher is a **publicly known entity** identified by their long-term Ed25519 public key.

The primary security objective of the MPub component is twofold:
1. **Protect incoming anonymous sources**: Ensure decrypted staging data is guarded so that a physical raid on the Publisher does not expose unpublished source material.
2. **Guarantee content provenance**: Ensure that once a Publisher signs and publishes a video to the Globally Distributed Storage (GDS), no adversary can alter, fake, or spoof that content under the Publisher's name.

## 2. Security Model & Trust Boundaries

### 2.1 Trust Assumptions
* **Trusted:** The Publisher's local hardware (servers, key vaults) and their editorial judgment.
* **Untrusted:** The incoming chunks from the MCN (treated as potentially malicious files), the GDS storage nodes, and the network transit layer.

### 2.2 Security Architecture
The Publisher node acts as a secure enclave. Incoming video chunks are decrypted using session keys, but the reassembled video is immediately written to an **Encrypted Staging Store** using a local, Publisher-controlled AES key. The Publisher's core identity—the Ed25519 private key—is isolated, ideally stored in a Hardware Security Module (HSM) or a highly protected software vault (libsodium).

When publishing, the video is re-chunked, erasure-coded (Reed-Solomon), and signed. By pushing signed manifests to the GDS, the Publisher relies on cryptography, rather than centralized hosting, to prove the authenticity of their media.

---

## 3. Attack Resistance (Mitigated Threats)

### 3.1 Resistance to Disablement & Censorship
If an adversary attempts to silence a Publisher by launching a Distributed Denial of Service (DDoS) attack against their IP, they will fail because the Publisher's IP is never exposed to the public. The Publisher receives data and publishes data exclusively through the Broadcast Overlay Network (BON). Their "address" is an invisible rendezvous point within the overlay. 

Furthermore, if a state actor successfully seizes the Publisher's server hardware, the published content remains online. Because the content is distributed across geography via GDS erasure coding, the Publisher node is not a single point of failure for media availability. 

### 3.2 Resistance to Impersonation (Anonymity Circumvention)
An adversary attempting to ruin a Publisher's reputation by uploading fake videos under their name cannot do so. Every content manifest requires a valid Ed25519 signature. The DHT architecture allows any downstream Content Provider or Viewer to mathematically verify that the video came exactly from the holder of the Publisher's private key.

---

## 4. Formal Threat Model (STRIDE)

| Threat Type | Vector | Mitigation Strategy |
|---|---|---|
| **Spoofing** | State actor publishes a fake video claiming to be the Publisher | Cryptographically impossible without the private Ed25519 key. Downstream nodes drop unsigned or invalidly signed manifests. |
| **Tampering** | Adversary alters video content in the Publisher's staging area | The Staging store uses AES-GCM at rest. Modification of files directly on the disk results in a failed decryption tag on load. |
| **Repudiation** | Publisher claims they didn't upload specific illegal content | Digital signatures provide non-repudiation. If signed by the Publisher's key, they are cryptographically responsible for the action. |
| **Information Disclosure** | Server seized; police attempt to extract pending anonymous videos | The Staging Store is encrypted at rest using keys derived from a passphrase not stored on the machine. Unapproved chunks are auto-purged securely. |
| **Information Disclosure** | Adversary extracts the Publisher's Master Private Key | Private key should be stored in an HSM or TPM, rendering extraction impossible even with root access to the OS. |
| **Denial of Service** | Malicious Creator floods Publisher with endless garbage chunks | The Publisher drops incoming sessions that exceed chunk counts, fail validation, or eat excessive bandwidth. The BON layer enforces connection limits. |
| **Elevation of Privilege** | Malicious MP4 file triggers an exploit in the Publisher's video previewer | The Publisher dashboard is isolated. Video preview relies on sandboxed/hardened codecs with strictly restricted system permissions. |

---

## 5. Unmitigated Threats & Fatal Vulnerabilities

The Publisher node is a high-value target. It cannot protect against the following vectors:

### 5.1 Publisher Coercion (The "Rubber Hose" Attack)
* **Description:** Law enforcement or hostile actors physically locate the Publisher and apply legal coercion, torture, or threats to force them to sign a revocation manifest, taking down a video.
* **Why it succeeds:** Cryptography cannot solve human coercion. While the system requires the private key to revoke content, the Publisher possesses the ability to use that key. 
* **Status:** Unmitigated. If the Publisher is compelled to issue a valid signed revocation, the GDS nodes will comply and delete the content. Operating the Publisher key from a jurisdiction beyond the adversary's reach is the only defense.

### 5.2 Zero-Day Codec Exploits
* **Description:** A highly sophisticated state adversary uploads a video via the MCN containing a zero-day exploit targeting the underlying video codec (e.g., heavily obfuscated malformed H.264 data).
* **Why it succeeds:** To review the video, the Publisher must load it into memory and parse it. If the parser is exploited, the adversary gains Remote Code Execution (RCE) on the Publisher's staging server.
* **Status:** Partially mitigated by sandboxing the previewer, but ultimately unmitigated against advanced 0-day exploits. The system requires parsing untrusted data to perform its editorial function.

### 5.3 Rogue Publisher / Legal Liability
* **Description:** A Publisher intentionally or negligently publishes illegal content (e.g., CSAM, or content violating local laws), inviting extreme legal scrutiny.
* **Why it succeeds:** The GBN is a neutral protocol. The Publisher holds total editorial power. If they publish illegal content, they open themselves (and potentially downstream Content Providers) to criminal liability. 
* **Status:** Unmitigated on the technical layer. The system architecture specifically places *editorial and legal responsibility* on the Publisher, creating a liability firewall protecting the rest of the network relays.

### 5.4 Social Engineering & Out-of-Band Key Deception
* **Description:** A state actor distributes a visually identical clone of the GBN app via a honeypot Telegram group, or hacks a journalist's social media and posts a "Fake" QR Code containing a State-controlled Public Key.
* **Why it succeeds:** Cryptography relies on "Trust on First Use" (TOFU). If the human Creator is tricked into loading the adversary's Public Key into their clean app, the MCN will flawlessly encrypt the video... locked so only the adversary can open it.
* **Status:** Structurally unmitigated. Cryptography cannot fix human deception prior to the math executing. Mitigated only via external UI/UX (e.g., "Publisher Fingerprints" — generating a 4-word dictionary string from the key that the Creator verifies verbally with the journalist over a secure phone call).

---

## 6. Route Discovery & Cryptographic Trust Validation

To anonymously route media from the Creator to the Publisher, the network employs a zero-trust pathfinding mechanism preventing Sybil, Blackhole, and Identity Spoofing attacks.

### 6.1 The Root of Trust
Publishers are identified solely by their Ed25519 Public Key. The Creator obtains this key out-of-band (e.g., scanning a QR code from a trusted publisher). This Public Key serves as the absolute Root of Trust for both Publisher identity and video encryption.

### 6.2 The Distributed Relay Directory (DHT)
Instead of central directory servers, the network utilizes a Kademlia DHT. 
* **Relay Descriptors:** Volunteer nodes publish their `NodeID`, IP address, and available transports. Creators act exclusively as read-only "ghosts" and **never** publish their IPs to the DHT. 
* **Passive Syncing:** To prevent "Query Correlation" (an adversary seeing a Creator query for a route and linking it to a payload), the Creator's app downloads DHT buckets continuously in the background, selecting routes strictly from a local cached network map.
* **Pre-Flight Reachability Check:** Before a node is permitted to advertise itself to the DHT, it must physically establish encrypted connections to multiple international endpoints. If it is trapped behind a national firewall, it silently fails the check, preventing "Domestic Blackholes" from cluttering the directory.

### 6.3 Anti-Spoofing the Publisher
The Publisher publishes their listening network address to the DHT under `hash(Publisher_PublicKey)`. To prevent an adversary from overwriting this entry with their own IP address, the descriptor must include a valid cryptographic signature. Since the adversary lacks the Publisher's Private Key, they cannot spoof the announcement.

### 6.4 Telescopic Onion Routing (Anti-Sinkhole)
When building the path (`Guard -> Middle -> Exit -> Publisher`), the Creator relies on mathematical validation:
1. The Creator does a base `Noise_XX` handshake with the Guard.
2. The Creator extends to the Middle by wrapping a new `Noise_XX` handshake in the Guard's encryption layer. The Guard passes it to the Middle node.
3. The Middle node calculates a cryptographic response and passes it back. 

By successfully validating the Middle node's cryptographic reply, the Creator simultaneously proves the Middle node is authentic **and** that the Guard node honestly forwarded the packet. If the Guard attempts to blackhole the traffic and fake a success message, it fails because it lacks the Middle node's private key to execute the math. This strict cryptographic relay validation guarantees that no node can silently drop packets while pretending the circuit is alive.
