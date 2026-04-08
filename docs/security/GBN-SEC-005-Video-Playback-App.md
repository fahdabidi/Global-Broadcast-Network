# GBN-SEC-005 — Product Security Document: Video Playback App

**Document ID:** GBN-SEC-005  
**Component:** Video Playback App (VPA)  
**Status:** V1.0  

---

## 1. Executive Summary

The Video Playback App (VPA) acts as the decentralized viewer client and the primary edge node of the GBN. Because the VPA runs on millions of personal smartphones (primarily Android), it exists entirely in hostile territory. 

The security objective of the VPA is to protect the viewer's consumption habits, protect the device from malicious network payloads, and enable the app to silently and securely contribute bandwidth back to the network without jeopardizing the user's safety or data caps.

## 2. Security Model & Trust Boundaries

### 2.1 Trust Assumptions
* **Trusted:** The physical device hardware (Android Keystore) and the Publisher signatures attached to video manifests.
* **Untrusted:** Every other peer device in the swarm, the Google Play Store (for app availability), local Wi-Fi networks, and cellular ISPs.

### 2.2 Security Architecture
The VPA enforces a strict cryptographic boundary using a Rust core accessed via JNI by the Kotlin UI. 
1. **Gossip Isolation & Out-Of-Band PKI:** The VPA connects to a small pool of peers using the HyParView gossip protocol. It shares network state and content manifests, but it cryptographically verifies every manifest locally. Because a Publisher's clearweb website might be geo-fenced, the VPA never relies on fetching keys from the internet. Publisher public keys are imported strictly out-of-band (e.g., via QR codes, Secure Messaging, or "Share-to-Install" APK packages) and stored locally. Forgeries are utterly rejected.
2. **Key Enclave:** The app generates a permanent Ed25519 node identity key that remains locked inside the hardware Android Keystore.
3. **Blind Caching:** The VPA acts as an embedded HTTP server to seed video chunks to nearby peers. Crucially, these are encrypted shards. A user seeding video data cannot be prosecuted for hosting illegal material because they possess neither the key to decrypt the shard nor the context of what the shard belongs to.

---

## 3. Attack Resistance (Mitigated Threats)

### 3.1 Resistance to Disablement & Censorship
Authoritarian regimes frequently coerce Google and Apple into removing dissenting apps from their regional stores. To resist app store disablement, the VPA includes a "Share-to-Install" feature. A user can generate a compressed QR code or a Bluetooth payload containing the APK and bootstrap nodes. The VPA spreads virally device-to-device entirely outside of corporate app store ecosystems.

### 3.2 Resistance to Anonymity Circumvention
The VPA protects viewer anonymity by fetching shards through the BON overlay, not directly from peers over the clearweb. When the VPA functions as a relay or cache server, the connections are authenticated and encrypted. A hostile peer cannot see what videos the user is actively watching, because the user's active watch queue is decoupled from their background storage cache.

---

## 4. Formal Threat Model (STRIDE)

| Threat Type | Vector | Mitigation Strategy |
|---|---|---|
| **Spoofing** | Attacker floods the gossip network with fake video alerts | The VPA refuses to display or cache any content manifest that lacks a valid Ed25519 signature from a Publisher explicitly subscribed to by the user. |
| **Tampering** | A malicious peer serves a corrupted video shard | Before passing the shard to the local ExoPlayer, the Rust core hashes the data with BLAKE3. If it doesn't match the shard CID requested, it is dropped. |
| **Repudiation** | Node changes IP to evade bandwidth limits or bans | The VPA implements signed `IPRenegotiation` messages. Sequence numbers prevent replay attacks, ensuring nodes cannot spoof older addresses. |
| **Information Disclosure** | Adversary connects to VPA's embedded server to map viewer's IP | Shard requests occur over the BON, so the local VPA embedded server sees the BON Node Address, hiding the user's physical IP from the requester. |
| **Denial of Service** | Battery exhaustion attack via endless gossip queries | The VPA aggressively throttles background network activity. Background relay and serving are disabled when the battery drops below 30% or when off Wi-Fi (unless overridden). |
| **Elevation of Privilege** | RCE via malformed video files | Video decoding is delegated to hardware-accelerated MediaCodec paths on Android through ExoPlayer. Sandboxing minimizes the impact of potential codec parsing bugs. |

---

## 5. Unmitigated Threats & Fatal Vulnerabilities

Because the VPA operates on consumer computing devices, it is uniquely exposed to physical and operating system-level attacks.

### 5.1 Device Seizure & Forensics (Endpoint Compromise)
* **Description:** Law enforcement physically confiscates a viewer's phone, bypasses the lock screen (e.g., via Cellebrite or physical coercion), and extracts the SQLite database.
* **Why it succeeds:** While shards are encrypted, the VPA's local SQLite database holds the user's watch history, subscriptions, and cached manifests in plaintext or lightly encrypted formats accessible while the phone is unlocked. 
* **Status:** Partially mitigated by Android's Full Disk Encryption, but utterly unmitigated if the device is seized while unlocked. Requires the implementation of a "Panic Button" or "Duress PIN" in future phases to instantly wipe the DB.

### 5.2 OS-Level Network Eavesdropping
* **Description:** A state-sponsored Android firmware update or malware installs a custom root certificate on the device, terminating SSL locally to inspect traffic.
* **Why it succeeds:** If the OS is compromised, the attacker can extract data before encryption or intercept the display buffer entirely.
* **Status:** Unmitigated. The GBN assumes a secure device OS.

### 5.3 Sybil Battery Draining (Eclipse Attack)
* **Description:** A massively well-funded adversary surrounds a specific VPA node entirely with hostile peers in the HyParView protocol. They force the device to constantly process cryptographic signatures or drop all real network traffic.
* **Why it succeeds:** While HyParView is highly resilient to Eclipse attacks theoretically, a mobile device has limited CPU and battery. An attacker willing to burn vast resources can force the phone to drain its battery verifying signatures, effectively disabling the VPA until the phone dies.
* **Status:** Unmitigated. Rate-limiting slows the drain, but the device is still rendered useless for media playback during the attack.
