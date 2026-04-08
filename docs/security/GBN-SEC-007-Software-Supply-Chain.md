# GBN-SEC-007 — Product Security Document: Software Supply Chain & Update Integrity

**Document ID:** GBN-SEC-007  
**Component:** Cross-Cutting (All Components)  
**Status:** V1.0  

---

## 1. Executive Summary

Every GBN component — the BON relay daemon, the MCN client, the VPA mobile app, the MPub server, the GDS storage daemon — is software that must be built, distributed, and occasionally updated. If a state actor can compromise the software supply chain at any point — the source code repository, the CI/CD build pipeline, or the update distribution mechanism — they can push a **"Death Update"** that silently:

- Disables all encryption, routing all traffic in plaintext
- Logs and exfiltrates Creator IPs, Publisher keys, and viewer watch histories
- Issues mass revocation commands deleting all GDS shards simultaneously
- Kills the BON overlay by refusing to relay traffic
- Backdoors the key vault to exfiltrate Publisher private keys

This attack is uniquely catastrophic because it exploits the *trust users place in their own software*. The update propagates voluntarily through the entire network. Unlike a Sybil flood or DPI fingerprint campaign, a compromised update doesn't fight the network — it *becomes* the network.

**This is the single most dangerous attack vector against the GBN, and it must be treated as a first-class architectural concern.**

---

## 2. Attack Taxonomy

### 2.1 Attack Vectors

| Vector | Description | Real-World Precedent |
|---|---|---|
| **Repository Seizure** | State actor acquires legal control of the GitHub/GitLab repository (e.g., via court order or platform compliance in a hostile jurisdiction) | GitHub blocking repositories under OFAC sanctions |
| **Developer Coercion** | State actor threatens, arrests, or legally compels a core developer to insert a backdoor | Lavabit (encrypted email provider) forced to hand over SSL keys |
| **CI/CD Pipeline Compromise** | Adversary gains access to the build server and injects malicious code during compilation, without modifying the public source | SolarWinds (build system compromised; signed malicious updates shipped to 18,000 organizations) |
| **Dependency Poisoning** | Adversary compromises an upstream crate/npm dependency used by the GBN, injecting malicious code that is transitively included | event-stream npm attack (cryptocurrency theft via compromised dependency) |
| **Signing Key Theft** | Adversary steals the update signing key, allowing them to produce updates that pass signature verification | ASUS Live Update compromise (attackers stole signing certificates) |
| **App Store Manipulation** | State actor compels Google/Apple to push a modified version of the app through their store | Not yet observed at scale, but technically trivial for store operators |

### 2.2 Blast Radius

A successful death update could achieve any combination of:

| Effect | Impact |
|---|---|
| **Network Kill** | New binary refuses to relay BON traffic. Overlay collapses in hours. |
| **Mass Deanonymization** | New binary silently phones home with Creator IPs, Publisher locations, viewer watch histories. |
| **Content Wipe** | New binary issues signed mass-revocation of GDS shards. Years of published content destroyed. |
| **Key Exfiltration** | New binary extracts Publisher Ed25519 private keys from key vaults and sends them to adversary. |
| **Silent Degradation** | New binary subtly weakens encryption (e.g., uses a predictable nonce), enabling future passive decryption. |

---

## 3. Defense Architecture: The Sovereign Update Model

The GBN SHALL NOT rely on any single entity, repository, developer, or signing key for software updates. Instead, it implements a **Sovereign Update Model** built on five pillars:

### 3.1 Pillar 1 — Multi-Signature Update Governance (M-of-N Signing)

No single developer, organization, or key can produce a valid update.

- Every release binary MUST be signed by **at least M of N independent maintainer keys** (recommended: 3-of-5).
- The N maintainers MUST be geographically distributed across at least 3 different legal jurisdictions to prevent a single government from coercing a quorum.
- Each maintainer key is an Ed25519 key generated offline on air-gapped hardware. The private key never touches a networked machine.
- The VPA, BON daemon, and all other GBN clients MUST reject any update that does not carry the required M-of-N valid signatures.
- If a maintainer is compromised, the remaining N-1 maintainers can issue a **key revocation** (itself requiring M-1 signatures from the remaining keys) and add a new maintainer.

### 3.2 Pillar 2 — Reproducible Builds (Build Verification)

Any person on Earth must be able to independently verify that a distributed binary was built from the published source code.

- All GBN components MUST support **deterministic, reproducible builds**. Given the same source commit, the same build toolchain version, and the same build flags, any builder MUST produce a byte-identical binary.
- The build environment is fully specified in a lockfile (Rust: `Cargo.lock`, Go: `go.sum`) and a pinned Docker image for the build container.
- Independent verifiers can download the source, run the build themselves, and compare the SHA-256 hash of their local build against the signed release. If they don't match, the release is provably compromised.
- The GBN project SHALL maintain at least 3 independent build verifiers (individuals or organizations) who publicly attest to each release.

### 3.3 Pillar 3 — Decentralized Code Hosting (Repo Resilience)

The source code must survive the seizure or deletion of any single hosting platform.

- The canonical source SHALL be hosted on **at least 3 independent platforms** simultaneously (e.g., GitHub, Codeberg, self-hosted Gitea instance, and a Radicle P2P repo).
- All commits are signed with the committer's GPG/Ed25519 key. Unsigned commits are rejected by the CI.
- The full git history (including all branches and tags) SHALL be mirrored to IPFS/GDS on every release, creating an immutable snapshot that survives platform seizure.
- If GitHub is compromised or seized, development seamlessly continues on the other mirrors with zero interruption.

### 3.4 Pillar 4 — Canary Rollout & Network-Level Update Quarantine

Updates must prove themselves safe *in the wild* before the network accepts them.

- **Phased Rollout**: Updates are distributed to a **1% canary group** first (randomly selected nodes). The canary period lasts a minimum of 72 hours.
- **Behavioral Watchdog**: During the canary period, non-updated nodes monitor the behavior of updated nodes. If updated nodes exhibit anomalous behavior (sudden traffic pattern changes, mass shard deletions, unusual outbound connections to unknown IPs), the network gossips a **quarantine signal** flagging the update as suspicious.
- **Automatic Block**: If >10% of non-canary nodes independently flag anomalous behavior from canary nodes, a network-wide **UPDATE_REJECT** signal propagates via gossip. All nodes refuse to install the flagged version.
- **Voluntary Adoption**: Even after the canary period, updates are never auto-installed. The user must explicitly approve the update. The update notification displays the M-of-N signature count, the build verifier attestations, and the number of canary nodes that have been running the update without incident.

### 3.5 Pillar 5 — Protocol Constitution (Immutable Invariants)

Certain behaviors are hardcoded into the protocol as **constitutional invariants** that NO software update is permitted to violate, regardless of how many signatures it carries.

- **Invariant 1**: All inter-node relay traffic MUST be encrypted. A binary that transmits plaintext relay traffic is rejected by peers.
- **Invariant 2**: Chunk hashes MUST be verified via BLAKE3 before acceptance. A binary that skips hash verification is rejected by peers.
- **Invariant 3**: Publisher signatures MUST be verified before content is displayed. A binary that displays unsigned content is rejected by peers.
- **Invariant 4**: The BON MUST use a minimum of 3 relay hops. A binary that routes in fewer than 3 hops is rejected by peers.
- **Invariant 5**: The update mechanism itself MUST require M-of-N signatures. A binary that accepts single-signature updates is rejected by peers. *(This invariant protects against an update that weakens the update mechanism itself.)*

These invariants are enforced at the **protocol handshake level**. When two nodes connect, they exchange version metadata. If a node's advertised behavior violates a constitutional invariant, the peer terminates the connection. A compromised binary cannot participate in the network because the *uncompromised nodes refuse to talk to it.*

---

## 4. Formal Threat Model (STRIDE)

| Threat Type | Vector | Mitigation Strategy |
|---|---|---|
| **Spoofing** | Adversary publishes a fake update signed with a stolen single key | M-of-N multi-signature requirement. Stealing one key is insufficient; attacker must compromise a geographically distributed quorum. |
| **Tampering** | Build server injects backdoor during compilation | Reproducible builds. Any independent builder can detect the discrepancy between published source and distributed binary. |
| **Repudiation** | Compromised maintainer denies inserting a backdoor | All commits are GPG-signed; all release signatures are publicly logged and timestamped. Forensic trail is immutable. |
| **Information Disclosure** | Poisoned update silently exfiltrates keys/IPs | Protocol Constitution: peers reject nodes that transmit unexpected outbound traffic patterns. Canary rollout detects anomalous behavior before mass adoption. |
| **Denial of Service** | Death update kills the BON overlay | Version pinning: nodes continue operating on the previous known-good version. Old versions are never forcibly deprecated. Network continues with mixed versions. |
| **Elevation of Privilege** | Update weakens the update mechanism itself (meta-attack) | Invariant 5: the M-of-N requirement is itself a constitutional invariant. A binary that accepts fewer signatures is rejected by peers at the protocol handshake. |

---

## 5. Unmitigated Threats & Fatal Vulnerabilities

### 5.1 Toolchain Compromise (The "Trusting Trust" Attack)
* **Description:** An adversary compromises the Rust compiler itself (`rustc`), or the LLVM backend, such that any program compiled with this toolchain silently includes a backdoor — regardless of the source code being clean. Ken Thompson's 1984 "Reflections on Trusting Trust" demonstrated this is theoretically possible.
* **Why it succeeds:** Reproducible builds only prove that the binary matches the source *given a specific compiler*. If the compiler itself is malicious, all reproducible builds will produce identical — but compromised — binaries.
* **Status:** Unmitigated. This is an unsolved problem in all of computer science. Practical mitigation: build with multiple independent compilers (Rust, GCC backend, Cranelift) and compare outputs.

### 5.2 Quorum Coercion of Maintainers
* **Description:** A state actor with global reach (e.g., Five Eyes intelligence alliance) simultaneously coerces M of the N maintainers across multiple jurisdictions using parallel legal instruments.
* **Why it succeeds:** If the adversary can physically compel 3 of 5 maintainers to sign a poisoned update, the multi-signature defense collapses. The signed update will pass all verification checks.
* **Status:** Partially mitigated. Maintainers should use **canary statements** (periodic signed attestation that they have not been coerced). If a maintainer's canary statement lapses, the remaining maintainers can trigger a key rotation. However, this defense is social, not cryptographic.

### 5.3 Gradual Capability Erosion
* **Description:** Instead of a single dramatic "death update," an adversary introduces a series of seemingly benign updates over months that each slightly degrade security — reducing default hop count from 3 to 2 "for performance," making cover traffic opt-out, weakening a cipher to "support legacy devices."
* **Why it succeeds:** Each individual change looks reasonable. Code reviewers may approve them. The canary system detects sudden behavioral changes but not gradual drift.
* **Status:** Partially mitigated by Protocol Constitution invariants (which prevent changes below a hard floor), but unmitigated for subtle degradations that stay above the constitutional minimums (e.g., reducing jitter range from 50-500ms to 50-100ms).

---

## 6. Dependency Supply Chain Hardening

Beyond the GBN's own code, the transitive dependency tree is a massive attack surface.

| Defense | Implementation |
|---|---|
| **Dependency Pinning** | `Cargo.lock` committed to repo; exact versions pinned; no floating version ranges |
| **Dependency Auditing** | `cargo-audit` and `cargo-vet` run on every CI build; blocks releases with known CVEs |
| **Minimal Dependencies** | Cryptographic primitives use only audited, minimal-dependency crates (`ring`, `snow`, `ed25519-dalek`). No "kitchen sink" frameworks. |
| **Vendored Dependencies** | Critical dependencies (crypto, networking) are vendored into the repo, reducing exposure to upstream compromise |
| **SBOM (Software Bill of Materials)** | Every release ships with a signed SBOM listing exact dependency versions, commit hashes, and audit status |

---

## 7. Version Compatibility & Backward Support

To prevent a scenario where nodes are forced to update or be orphaned:

- The BON protocol includes a **version negotiation** phase. Nodes running different versions can still communicate as long as they both support a common protocol version.
- **Old versions are never forcibly deprecated.** A node running v1.0 can interoperate with a node running v2.3 as long as both support protocol version 1.
- This ensures that even if users suspect a new update is compromised, they can remain on the old version and continue participating in the network indefinitely.
