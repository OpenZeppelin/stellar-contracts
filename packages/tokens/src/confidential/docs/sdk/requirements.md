# Security, Non-Functional, and Conformance Requirements

## Security Requirements

**Secret handling.** The root, $$sk$$, $$vk$$, $$dvk_i$$, every derived $$r_e$$, every cached opening, and — on the auditor side — $$k$$ and the accumulated openings of §11 are secrets. Implementations MUST keep them within the trust boundary (§2.1), SHOULD zeroize buffers holding them once no longer needed, and MUST NOT transmit them to any remote service except under §8.3's explicit opt-in.

$$vk$$ MUST NOT be presented as a safely-shareable read-only credential. It exposes every historical balance checkpoint, every incoming amount, every delegation allowance, and — through the ephemeral-scalar derivation of DESIGN.md §5.3 — the opening of every transfer the account originated. Its only guarantee is that it cannot authorize spending. A party that needs outbound visibility is served with D-sender proofs, which are bound to that party and to a nonce (SELECTIVE_DISCLOSURE.md §13.2), never by handing over the key.

**Storage at rest.** Persisted openings are as sensitive as the amounts they represent, and a persisted root is equivalent to the funds. Implementations MUST document what they persist and where, and SHOULD encrypt both at rest under a key that is not itself derived from persisted material. A signer root (§5.2) MUST NOT be persisted at all, since it is reproducible on demand from the signer.

**Logging and telemetry.** Witness material, decrypted amounts, balances, and openings MUST NOT reach logs, telemetry, analytics, error reports, or crash dumps.

**Randomness.** All sampling MUST use a cryptographically secure generator (§4.7). An implementation MUST NOT fall back to a non-CSPRNG source when one is unavailable; it MUST fail.

**Supply chain.** Soundness depends on the verification keys corresponding to the intended circuits and on the structured reference string having been generated honestly (DESIGN_cont.md §10.6). Implementations MUST pin verification keys and the proving toolchain (§8.1), SHOULD document how a user reproduces a deployed key from circuit source, and MUST NOT fetch either at runtime from a mutable location.

**What the SDK cannot do.** It cannot mitigate a compromised host: malicious code in the process, the bundle, or the storage layer reaches the root. Implementations MUST NOT present software-only custody as equivalent to hardware custody. Hardware custody is additionally constrained here, because proving requires $$sk$$ as a witness, so a device that does not prove internally must expose $$sk$$ to the host.

---

## Non-Functional Requirements

**Proving latency.** Single-digit seconds on contemporary hardware is the design target (OVERVIEW.md). Implementations MUST treat it as user-visible.

**Sync and replay bounds.** The replay window runs from the account's last `Merge` or `Clawback` at or before its latest checkpoint, or from registration if there is none before that checkpoint (DESIGN.md §5.2 *Recovery*), which is unbounded in age. Implementations MUST NOT assume a bounded window, and SHOULD use the archive's checkpoint lookup where available (INDEXER.md §6, C1) so that a dormant account is not obliged to transfer its entire history.

**Storage growth.** Per-account event volume is linear in inbound transfers and unbounded by design, since incoming-transfer spam is rate-limited only by transaction fees (DESIGN_cont.md §9.5). Implementations MUST NOT size local storage on the assumption that inbound volume tracks the user's own activity.

**Offline capability.** Key derivation, witness assembly, and proof generation need no network. Balance display needs synced state. Submission and consistency checking need the chain. Implementations SHOULD make the boundary explicit rather than failing opaquely when offline.

**Version matrix.** An implementation MUST expose, for inspection and for bug reports: the protocol documentation version it targets, the contract address and its `addr_f`, the verification-key set and producing toolchain versions (§8.1), and the domain-separator scheme in use (§4.8, including whether the alternate hashed scheme of DESIGN_cont.md §13 was selected). Each is an interoperability boundary whose mismatch is undiagnosable without knowing its value.

---

## Conformance and Versioning

An implementation conforms to this specification iff it:

1. reproduces every vector of §6.1 byte-for-byte, reading the fixture files rather than transcribing them;
2. passes circuit-execution parity with tamper rejection (§6.2) for every circuit it supports;
3. satisfies §4–§12 for the roles it implements;
4. exposes the version matrix of §14.

Partial role coverage is conformant and MUST be declared: an implementation supporting only the holder role is conformant for that role, and MUST NOT claim spender, auditor, or disclosure conformance. Partial *circuit* coverage MUST likewise be declared.

**The portable surface is §4–§7.** The crypto core, key derivation, and witness assembly are what two implementations must agree on byte-for-byte. The facades of §10–§12 are deployment-shaped, and this document constrains their obligations rather than their structure.

This document is versioned with the protocol documentation set. A change to any primitive in §4, to the derivation in §5, to the ephemeral-scalar derivation (DESIGN.md §5.3, restated in §10.5), or to the domain separators in §4.8 breaks the cross-implementation contract: it MUST bump the protocol documentation version, MUST be called out in release notes, and MUST be accompanied by updated fixtures in `circuits/lib/testdata/`. A change to the numeric value of a domain separator, or to the sponge convention, invalidates every previously derived key and every previously emitted ciphertext, and is a new deployment rather than an upgrade.
