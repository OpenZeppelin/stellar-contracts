# Confidential Token: SDK

Companion specification to [DESIGN.md](./DESIGN.md) §4 (Key Hierarchy) and §5.2 (Off-Chain Opening Maintenance), [DESIGN_cont.md](./DESIGN_cont.md) §9.5 (State Recovery) and §11 (Interface), [INDEXER.md](./INDEXER.md), and [SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §5 and §15. It specifies the client layer those documents assume: the crypto core that mirrors the Noir circuits off-chain, the key derivation the protocol leaves open, the witness and payload construction, the wallet state machine, and the auditor, disclosure, and indexer clients.

The key words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are to be interpreted as in RFC 2119. The normative audience is threefold:

- **SDK implementers** MUST satisfy §4–§12.
- **Wallet, auditor, and application integrators** MUST NOT bypass §13; the security properties of the protocol do not survive it.
- **Port authors** (mobile, hardware wallet, a second language) MUST pass §6.

**Scope.** This document specifies obligations, not an API. It does not prescribe function signatures, module names, package layout, or class design, and it does not restate protocol formulas that [DESIGN.md](./DESIGN.md) already fixes — each requirement cites its source section instead.

---

## Why the SDK Is Load-Bearing

Four properties of the protocol place correctness and confidentiality in the client rather than in the contract.

**The opening exists only off-chain.** A balance is a Pedersen commitment $$C = v \cdot G + r \cdot H$$. The chain stores the point; the opening $$(v, r)$$ that authorizes the next spend lives exclusively in client state (DESIGN.md §5.2). A client that loses, misderives, or misaccumulates the opening makes the funds unspendable, and the contract cannot help because it never knew the value.

**Every amount a user sees is client-decrypted.** The contract performs homomorphic point arithmetic and never learns a value. Balances, transfer amounts, allowances, and audit figures are all produced by client-side decryption of event ciphertexts, so a decryption defect yields a plausible wrong number rather than a visible failure.

**The client is an enforcement point for canonicality.** Neither the Soroban host nor the verifier distinguishes a canonical $$\mathbb{F}_r$$ representative from a non-canonical one (DESIGN.md §2.2, *Host deserialiser caveat*); the contract enforces canonicality at its boundary, but the client is where the bytes are produced.

**The client is where all secrets live and all randomness is sampled.** The spending key, the viewing key, every blinding factor, and every per-operation salt originate client-side, so the protocol's confidentiality reduces to the client's key handling and CSPRNG quality. DESIGN.md §2.5 makes salt uniqueness a confidentiality requirement: a reused salt repeats the deterministic ephemeral scalar and every sponge mask keyed by it.

---

## Terminology and Layering

### Terminology

- **Root** — the secret an implementation feeds to §5.1's derivation: a SEP-0053 signature by a signer on the account, or a raw 32-byte value for an address with no ed25519 signer of its own (§5).
- **Opening** — the pair $$(v, r)$$ such that $$C = v \cdot G + r \cdot H$$ for an on-chain commitment $$C$$.
- **Checkpoint** — an owner-initiated proof-carrying event publishing $$(\tilde{b}, \sigma)$$ for the owner's spendable balance (DESIGN.md §5.2 *Recovery*, which enumerates the qualifying events).
- **Witness material** — any value that appears as a private witness in any circuit: $$sk$$, $$vk$$, $$dvk_i$$, $$v$$, $$r$$, $$r_e$$, $$v_{\text{transfer}}$$, and every intermediate derived from them.
- **Trust boundary** — the process and storage under the account holder's exclusive control. Witness material inside it is secret; witness material that crosses it is disclosed.
- **In-flight operation** — a submitted operation whose event has not yet been observed. Its projected post-operation opening is known locally but not yet confirmed against chain state.
- **Facade** — a role-scoped interface over the crypto core (§3).

### Layering

An implementation MUST separate the following concerns. The boundaries are normative because §15 defines conformance over the lower layers only; the upper layers are deployment-shaped and deliberately unconstrained in structure.

| Layer | Section | Responsibility | Purity |
|:--|:--|:--|:--|
| Crypto core | §4 | Field and curve arithmetic, Poseidon2, derivations, encodings | Deterministic; no I/O; holds no state |
| Key derivation | §5 | Root → $$sk$$ → the DESIGN.md §4 hierarchy | Deterministic; no I/O |
| Conformance vectors | §6 | Fixture and circuit-execution parity | Test-only |
| Witness assembly | §7 | Per-circuit private witnesses and public inputs | Deterministic given randomness |
| Prover | §8 | Circuit artifacts, verification keys, proof generation | I/O; pluggable backend |
| Chain adapter | §9 | Reads, payload encoding, submission, typed errors | I/O |
| Role facades | §10–§12 | Holder wallet, auditor, disclosure, indexer clients | Stateful |

---

## Roles and Capability Separation

Five roles consume the protocol. Each holds distinct key material and MUST be *structurally* incapable of exceeding its capability.

| Role | Holds | Can |
|:--|:--|:--|
| Holder | Root, $$sk$$, $$vk$$ | Spend, withdraw, merge, delegate, disclose, read own balances |
| Spender | Own $$sk_{\text{op}}$$, escrowed $$dvk_i$$ | Spend from the allowance, read allowance state, disclose own spender transfers |
| Auditor | Auditor secret $$k$$ | Decrypt both channels for accounts bound to its `auditor_id` (DESIGN_cont.md §8.1) |
| Disclosure recipient | $$(r_R, P_R)$$ | Verify a disclosure proof and recover the disclosed amount |
| Observer | Nothing | Read commitments, ciphertexts, ephemerals, addresses, public amounts |
