# Witness Assembly, Prover, and Chain Adapter

## Witness Assembly

An implementation MUST provide witness assembly for each circuit it supports, covering the six circuits of [Circuits](../protocol/proof-system.md#circuits): the five core circuits `Register`, `Withdraw`, `Transfer`, `SpenderTransfer`, `SetSpender`, and `Clawback`, which the auditor role proves in a deployment that enables the compliance extension ([Auditor Client](auditor-client.md), [Circuit](../compliance.md#circuit)).

**Public-input order is a wire contract.** The verifier sees an ordered vector of field elements with no knowledge of what they denote ([Public Input Sources](../protocol/operations/README.md#public-input-sources)), so a permutation of two same-typed inputs produces a well-formed vector that verifies a different statement. Each builder MUST assemble public inputs in exactly the order the contract assembles them, and MUST cite the contract function it mirrors at the site of the ordering. The per-operation public-input tables in the [operation specifications](../protocol/operations/README.md) are authoritative for *membership*; the contract's assembly is authoritative for *order*.

**The trust-boundary rule constrains the client too.** [Public Input Sources](../protocol/operations/README.md#public-input-sources) requires the contract to load state-derived public inputs itself and never accept them from the caller. The client-side corollary: a builder MUST NOT include a contract-loaded input in the payload it submits. Doing so does not break soundness, since the contract ignores it, but it produces a payload whose fields disagree with the proof's public inputs.

**Prover-supplied values MUST be freshly derived per attempt**, never reused from a previous attempt at the same logical operation ([Salt freshness](wallet.md#salt-freshness)).

Builders SHOULD return, alongside the witness and payload, the projected post-operation opening and — for transfer-family circuits — the values the recipient will recover, both of which [Holder Wallet](wallet.md) needs.

---

## Prover

### Toolchain pinning

Proofs the deployed verifier accepts MUST be generated with the toolchain and non-default flags pinned in `circuits/vks/README.md`. Two flags govern acceptance:

- **A Keccak Fiat-Shamir transcript is mandatory.** The on-chain verifier reconstructs the transcript with Keccak, while proving backends commonly default to Poseidon2, and a default-transcript proof verifies locally then fails on-chain. Implementations MUST set the Keccak transcript for proof generation, local verification, and verification-key derivation alike.
- **Zero-knowledge mode MUST NOT be enabled** while the verifier implements only the non-zk flavour.

Circuit artifacts and verification keys MUST record the toolchain version that produced them. Version drift between a client's vendored artifacts and the deployment's pinned toolchain produces verification keys that differ from the deployed ones while every local test passes.

### Verification-key identity

Before submitting a proof, an implementation MUST verify that the verification key its circuit artifact implies matches the one the deployment holds for that circuit type. A mismatch means the client and the chain disagree about what is being proven, and the proof cannot succeed.

Where a deployment permits verification-key rotation ([Governance and Upgradeability](../protocol/system-model.md#governance-and-upgradeability)), an implementation MUST re-check rather than cache across sessions.

### Backend pluggability and the trust boundary

Proving MUST sit behind an interface that admits multiple backends — in-process WASM, a native binary, a remote service — because the viable choice differs per platform. Browser bundlers, for instance, break worker-spawning proving libraries by rewriting the worker URL into a hashed chunk, so an implementation targeting browsers MUST allow the backend to be supplied by the host application rather than resolved statically.

**Witness material MUST NOT cross the trust boundary by default** ([Terminology](README.md#terminology)). Remote proving discloses the spending key, the balance, and the transfer amount to the prover. An implementation MAY offer it, but MUST require explicit opt-in, MUST NOT select it as a fallback when a local backend fails, and MUST state precisely which values leave the device.

---

## Chain Adapter

### Reads

The adapter MUST expose the account record and the spender delegation record ([Read Methods](../protocol/interface.md#read-methods)), and MUST treat the contract crate's types as authoritative for their shape. It MUST distinguish the three delegation states that [Read Methods](../protocol/interface.md#read-methods) separates — absent, active, and expired-but-not-yet-revoked — and MUST NOT collapse the latter two.

Auditor keys MUST be read from the auditor contract by the account's bound `auditor_id`, never supplied by the caller.

### Payload encoding

Proof-carrying entry points take an XDR-encoded payload. Its byte representation is fixed by Soroban's canonical XDR rules, so independent implementations compiling against the same `#[contracttype]` definitions produce byte-identical payloads ([Interface](../protocol/interface.md)).

Two specifics:

- Points are flat 64-byte values, not nested two-field structures ([Canonicality](crypto-core.md#canonicality)).
- Struct fields serialise as a map with symbol keys in canonical sorted order.

### Authorization and submission

Every state-changing operation requires both the appropriate `require_auth()` principal ([Authorization Model](../protocol/interface.md#authorization-model)) and, where applicable, a valid proof. The adapter MUST submit the principal the interface specifies — notably the *spender*, not the owner, for a delegated transfer.

Operations SHOULD be simulated before submission, which catches a stale commitment, a frozen account, or an expired delegation without a fee.

### Typed errors

The adapter MUST surface contract failures as typed, distinguishable outcomes. At minimum it MUST separate:

| Class | Meaning | Caller's next step |
|:--|:--|:--|
| Proof verification failed | The proof did not verify against the assembled public inputs | Do not retry blindly; check [Verification-key identity](#verification-key-identity) and witness assembly |
| State mismatch | The referenced commitment is no longer current | Re-sync ([Event application](wallet.md#event-application)) and rebuild the proof |
| Not registered | The account or counterparty has no confidential account | Register, or reject the recipient |
| Compliance rejection | Frozen, policy-denied, or unauthorized by the underlying asset ([Contract-Level Freeze](../compliance.md#contract-level-freeze)) | Surface to the user as an administrative state, not an error |
| Delegation state | Absent, duplicate, or expired | Distinguish per [Reads](#reads) |

Separating the first two matters most: they present as the same opaque failure on the wire but have opposite remedies.

---

Previous: [Conformance Vectors](conformance.md) · Up: [Index](../README.md#companions) · Next: [Holder Wallet](wallet.md)
