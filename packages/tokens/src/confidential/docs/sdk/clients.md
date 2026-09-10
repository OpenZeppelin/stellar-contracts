# Disclosure and Indexer Clients

## Disclosure construction

An implementation supporting selective disclosure MUST follow the [Selective Disclosure](../selective-disclosure/README.md) specification for the holder, sender, and auditor variants, and MUST bind each proof to the requesting recipient's key and nonce so that a proof cannot be replayed against a different recipient or a later request ([Disclosure Recipient](../selective-disclosure/README.md#disclosure-recipient)). A D-auditor witness reads each auditor channel at the width [Poseidon2 sponge](crypto-core.md#poseidon2-sponge) fixes for its tag — three lanes on $$\delta\_{\text{aud\\\_s}}$$, two on $$\delta\_{\text{aud\\\_r}}$$ — exactly as [Auditor Client](auditor-client.md) requires of the auditor client ([D-auditor](../selective-disclosure/circuits/d-auditor.md) A3).

Not every historical transfer is disclosable by its sender: one predating [Deterministic ephemeral scalars](wallet.md#deterministic-ephemeral-scalars)'s requirement may carry an ephemeral scalar that does not reproduce, and the disclosure recipient holds no key material that could detect this. The disclosing wallet MUST therefore establish disclosability before constructing a D-sender proof: derive the candidate $$r\_e$$ from $$(vk, \sigma\_E)$$ and compare $$r\_e \cdot H$$ against the event's $$R\_e$$. A mismatch MUST surface to the caller as a distinct *not disclosable* outcome that carries neither a proof nor an amount, and MUST NOT be reported as a proof-construction error. The comparison costs one Poseidon2 call and one scalar multiplication and is authoritative, where a stored per-transfer flag is not ([Deterministic ephemeral scalars](wallet.md#deterministic-ephemeral-scalars)).

Disclosure circuits are verified entirely off-chain and MUST NOT be registered with the on-chain verifier set ([Circuits](../selective-disclosure/security.md#circuits)).

## Disclosure verification

The verifier MUST be distributable independently of any wallet, since its purpose is to let a party who trusts no holder check a claim. It consumes a chain endpoint, the recipient keypair, and the proof bundle with its event reference, and returns the disclosed amount or a **typed** indication of which check failed — proof verification, on-chain state mismatch, or decryption failure. A single boolean is not conformant, because the three outcomes have different meanings to the recipient.

Verification MUST include comparing the circuit's verification key against the pinned key for that disclosure circuit, without which the proof attests to an unknown statement.

## Indexer client

Recovery beyond the RPC retention window requires a conforming durable archive ([Indexing and Off-Chain State Recovery](../indexer.md)). A client MUST propagate the archive's completeness signal to its caller rather than swallowing it, since an incomplete range and a tampered range both end in the same refusal at [Consistency checking](wallet.md#consistency-checking) and only that signal distinguishes them.

Clients SHOULD support multiple independent archive endpoints, since withholding is the residual trust the archive retains ([Trust Model and Client-Side Verification](../indexer.md#trust-model-and-client-side-verification)).

## The hybrid read path and its two failure modes

RPC and archive compose: the RPC serves the recent tail, the archive everything older, and the client stitches them at a seam ([Why the Indexer Is Load-Bearing](../indexer.md#why-the-indexer-is-load-bearing)). Two requirements are not derivable from [Indexing and Off-Chain State Recovery](../indexer.md) and are specified here.

**The seam MUST sit strictly above the RPC's reported retention floor, by a margin.** The floor advances as ledgers are collected, including between the moment the client reads it and the moment it issues the range query, with the archive request in between taking real time. A seam placed exactly at the observed floor therefore intermittently produces a rejected query. The archive covers everything below the seam, so a margin loses no events. Implementations MUST set the two legs to disjoint ledger ranges so that correctness does not depend on cross-source id equality, and MUST still deduplicate ([Event application](wallet.md#event-application)) as a guard at the boundary.

**A configured archive's failure MUST fail the whole sync.** If an archive is configured and its request fails, an implementation MUST NOT degrade silently to RPC-only and MUST NOT persist a sync position derived from the RPC leg alone. Persisting it moves the position past the pre-window range, so every later sync takes the warm path that never consults the archive and the openings in the skipped range become unrecoverable. An archive that is *not* configured is a different case and MAY be absent, provided [Recovery](wallet.md#recovery)'s warning is surfaced.

---

Previous: [Auditor Client](auditor-client.md) · Up: [Index](../README.md#companions) · Next: [Security, Non-Functional, and Conformance Requirements](requirements.md)
