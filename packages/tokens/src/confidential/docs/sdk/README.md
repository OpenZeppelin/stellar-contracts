# Confidential Token: SDK

Companion specification to the protocol's [Key Hierarchy](../protocol/keys-and-commitments.md#key-hierarchy), [Wallet State and Recovery](../protocol/wallet-state.md), and [Interface](../protocol/interface.md), to [Indexing and Off-Chain State Recovery](../indexer.md), and to the [Disclosure Protocol](../selective-disclosure/protocol.md) and its [Implementation Notes](../selective-disclosure/security.md#implementation-notes). It specifies the client layer those documents assume: the crypto core that mirrors the Noir circuits off-chain, the key derivation the protocol leaves open, the witness and payload construction, the wallet state machine, and the auditor, disclosure, and indexer clients.

The key words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are to be interpreted as in RFC 2119. The normative audience is threefold:

- **SDK implementers** MUST satisfy [Crypto Core](crypto-core.md) through [Disclosure and Indexer Clients](clients.md).
- **Wallet, auditor, and application integrators** MUST NOT bypass the [Security Requirements](requirements.md#security-requirements); the security properties of the protocol do not survive it.
- **Port authors** (mobile, hardware wallet, a second language) MUST pass the [Conformance Vectors](conformance.md).

**Scope.** This document specifies obligations, not an API. It does not prescribe function signatures, module names, package layout, or class design, and it does not restate protocol formulas that [the protocol specification](../protocol/README.md) already fixes — each requirement cites its source section instead.

---

## Why the SDK Is Load-Bearing

Four properties of the protocol place correctness and confidentiality in the client rather than in the contract.

**The opening exists only off-chain.** A balance is a Pedersen commitment $$C = v \cdot G + r \cdot H$$. The chain stores the point; the opening $$(v, r)$$ that authorizes the next spend lives exclusively in client state ([Wallet State and Recovery](../protocol/wallet-state.md)). A client that loses, misderives, or misaccumulates the opening makes the funds unspendable, and the contract cannot help because it never knew the value.

**Every amount a user sees is client-decrypted.** The contract performs homomorphic point arithmetic and never learns a value. Balances, transfer amounts, allowances, and audit figures are all produced by client-side decryption of event ciphertexts, so a decryption defect yields a plausible wrong number rather than a visible failure.

**The client is an enforcement point for canonicality.** Neither the Soroban host nor the verifier distinguishes a canonical $$\mathbb{F}_r$$ representative from a non-canonical one ([Host deserialiser caveat](../protocol/primitives.md#host-deserialiser-caveat)); the contract enforces canonicality at its boundary, but the client is where the bytes are produced.

**The client is where all secrets live and all randomness is sampled.** The spending key, the viewing key, every blinding factor, and every per-operation salt originate client-side, so the protocol's confidentiality reduces to the client's key handling and CSPRNG quality. [Poseidon2 Hash](../protocol/primitives.md#poseidon2-hash) makes salt uniqueness a confidentiality requirement: a reused salt repeats the deterministic ephemeral scalar and every sponge mask keyed by it.

---

## Terminology and Layering

### Terminology

- **Root** — the secret an implementation feeds to [Derivation](key-derivation.md#derivation)'s derivation: a SEP-0053 signature by a signer on the account, or a raw 32-byte value for an address with no ed25519 signer of its own ([Key Derivation](key-derivation.md)).
- **Opening** — the pair $$(v, r)$$ such that $$C = v \cdot G + r \cdot H$$ for an on-chain commitment $$C$$.
- **Checkpoint** — an owner-initiated proof-carrying event publishing $$(\tilde{b}, \sigma)$$ for the owner's spendable balance ([Recovery](../protocol/wallet-state.md#recovery), which enumerates the qualifying events).
- **Witness material** — any value that appears as a private witness in any circuit: $$sk$$, $$vk$$, $$dvk_i$$, $$v$$, $$r$$, $$r_e$$, $$v_{\text{transfer}}$$, and every intermediate derived from them.
- **Trust boundary** — the process and storage under the account holder's exclusive control. Witness material inside it is secret; witness material that crosses it is disclosed.
- **In-flight operation** — a submitted operation whose event has not yet been observed. Its projected post-operation opening is known locally but not yet confirmed against chain state.
- **Facade** — a role-scoped interface over the crypto core ([Roles and Capability Separation](#roles-and-capability-separation)).

### Layering

An implementation MUST separate the following concerns. The boundaries are normative because [Conformance and Versioning](requirements.md#conformance-and-versioning) defines conformance over the lower layers only; the upper layers are deployment-shaped and deliberately unconstrained in structure.

| Layer | Section | Responsibility | Purity |
|:--|:--|:--|:--|
| Crypto core | [crypto-core.md](crypto-core.md) | Field and curve arithmetic, Poseidon2, derivations, encodings | Deterministic; no I/O; holds no state |
| Key derivation | [key-derivation.md](key-derivation.md) | Root → $$sk$$ → the [Key Hierarchy](../protocol/keys-and-commitments.md#key-hierarchy) hierarchy | Deterministic; no I/O |
| Conformance vectors | [conformance.md](conformance.md) | Fixture and circuit-execution parity | Test-only |
| Witness assembly | [proving.md](proving.md#witness-assembly) | Per-circuit private witnesses and public inputs | Deterministic given randomness |
| Prover | [proving.md](proving.md#prover) | Circuit artifacts, verification keys, proof generation | I/O; pluggable backend |
| Chain adapter | [proving.md](proving.md#chain-adapter) | Reads, payload encoding, submission, typed errors | I/O |
| Role facades | [wallet.md](wallet.md), [auditor-client.md](auditor-client.md), [clients.md](clients.md) | Holder wallet, auditor, disclosure, indexer clients | Stateful |

---

## Roles and Capability Separation

Five roles consume the protocol. Each holds distinct key material and MUST be *structurally* incapable of exceeding its capability.

| Role | Holds | Can |
|:--|:--|:--|
| Holder | Root, $$sk$$, $$vk$$ | Spend, withdraw, merge, delegate, disclose, read own balances |
| Spender | Own $$sk_{\text{op}}$$, escrowed $$dvk_i$$ | Spend from the allowance, read allowance state, disclose own spender transfers |
| Auditor | Auditor secret $$k$$ | Decrypt both channels for accounts bound to its `auditor_id` ([Per-Transfer Auditor Ciphertexts](../protocol/auditing.md#per-transfer-auditor-ciphertexts)) |
| Disclosure recipient | $$(r_R, P_R)$$ | Verify a disclosure proof and recover the disclosed amount |
| Observer | Nothing | Read commitments, ciphertexts, ephemerals, addresses, public amounts |
