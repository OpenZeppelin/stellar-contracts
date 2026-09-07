# Security and Implementation Notes

## Security Analysis

### Soundness of Disclosed Amount

For the D-recipient circuit, soundness reduces to two facts:

1. D1, D2 force the prover to know an $$sk_A$$ whose derived $$vk_A$$ matches the on-chain $$\text{PVK}_A$$. By [Viewing Key](../protocol/keys-and-commitments.md#viewing-key) this party is the account owner.
2. D3, D4 force $$v_{\text{transfer}}$$ to equal the decryption of $$\tilde{v}$$ under that owner's $$vk_A$$. By [ECDH-Derived Blinding](../protocol/keys-and-commitments.md#ecdh-derived-blinding) this is the same value the on-chain transfer commitment $$C_{\text{transfer}}$$ commits to, since the transfer circuit enforced consistent ECDH derivation at transfer time.

Therefore, a soundness break would require either a key-derivation collision (Poseidon2 preimage break, [Poseidon2 Hash](../protocol/primitives.md#poseidon2-hash), [Threat Model](../protocol/system-model.md#threat-model)) or a discrete-log break on Grumpkin. Both are out of scope.

D-sender soundness is symmetric: DS3 forces the prover to know $$r_e$$ with $$R_e = r_e \cdot H$$, which by the transfer circuit's constraint T6 ([Confidential Transfer](../protocol/operations/transfer.md)) was the same $$r_e$$ used to derive the auditor and recipient ciphertexts. DS4, DS5 reconstruct the decryption from the sender side. Soundness rests on DS3's binding through the event's $$R_e$$ rather than on how the prover came to hold $$r_e$$; [D-sender](circuits/d-sender.md)'s recovery step bears on wallet state, not on the soundness of the disclosed amount.

D-auditor soundness is direct: A1 forces auditor-key ownership; A2–A4 reconstruct the standard auditor sponge decryption ([Per-Transfer Auditor Ciphertexts](../protocol/auditing.md#per-transfer-auditor-ciphertexts)).

D-balance soundness reduces to Pedersen binding ([Pedersen Commitments](../protocol/primitives.md#pedersen-commitments)): given the on-chain $$C_{\text{spend}}$$, the prover's witnesses $$(v_s, r_s)$$ satisfying DB3 uniquely determine $$v_s$$ up to negligible probability. D1, D2 anchor the proof to the disclosing account as in D-recipient. The "current state" framing is established by the verifier's read protocol, not the circuit: a proof against a stale $$C_{\text{spend}}$$ simply fails to verify against the current public-input vector, so the recipient only ever accepts proofs about the on-chain state at the moment of verification.

**Event binding.** None of the soundness arguments above pin the proof to a *specific* on-chain event by themselves — they only force consistency with whatever $$(\text{PVK}_A, R_e, \sigma_E, \tilde{v})$$ tuple the public-input vector commits to. The binding to the on-chain event is established off-chain by the [Verifier Protocol](protocol.md#verifier-protocol) verifier protocol: the verifier MUST resolve $$\text{ref}_E$$ from the bundle to a specific event, MUST take all event-derived public inputs verbatim from that event, and MUST take all account-derived inputs from the on-chain account record at the address the *event* names. Skipping any of these steps voids the binding. Because $$R_e$$ is unique per operation ([Poseidon2 Hash](../protocol/primitives.md#poseidon2-hash), [ECDH-Derived Blinding](../protocol/keys-and-commitments.md#ecdh-derived-blinding)), no two distinct on-chain events share an $$R_e$$ except with negligible probability, so a proof that verifies against the vector built from event $$E$$ cannot also verify against the vector built from any $$E' \neq E$$. Event binding therefore inherits [Poseidon2 Hash](../protocol/primitives.md#poseidon2-hash)'s salt-uniqueness requirement: a repeated nonce within one account would collapse it alongside the channel masks. This is the soundness role the trust-boundary rule ([Proof Bundle](protocol.md#proof-bundle)) plays for the disclosure layer; it is the analogue of [Public Input Sources](../protocol/operations/README.md#public-input-sources)'s on-chain trust-boundary rule.

### Recipient Binding

The disclosed value $$v_{\text{transfer}}$$ is delivered only through $$\tilde{v}_{\text{disc}} = v_{\text{transfer}} + \text{Poseidon}(\delta_{\text{disc}}, s_{\text{disc}}, \nu)$$, where $$s_{\text{disc}} = \text{ECDH}(r_{\text{disc}}, P_R)$$ is recoverable only by the holder of $$r_R$$.

A party other than the intended recipient who obtains $$(\pi, R_{\text{disc}}, \tilde{v}_{\text{disc}})$$ can verify $$\pi$$ but cannot decrypt $$\tilde{v}_{\text{disc}}$$. They learn that *some* value was disclosed but not the value itself.

Nonce $$\nu$$ is bound into the Poseidon argument. A holder cannot reuse $$(\pi, R_{\text{disc}}, \tilde{v}_{\text{disc}})$$ against a recipient request that issued a different nonce, because the verifier's public inputs would not match.

### What This Does Not Prevent

**Holder cherry-picking.** A holder may disclose three inbound transfers from counterparty $$Y$$ while withholding a fourth. The verifier cannot detect this from the proof alone. Mitigation: completeness routes through the auditor (D-auditor variants), who sees every event under their scope. Recipients that require completeness must request from the auditor, not the holder.

**Disclosure recipient leakage.** Once decrypted, $$v_{\text{transfer}}$$ is plaintext in the recipient's possession. The recipient may store, share, or leak it. This is a non-cryptographic concern handled by the recipient's own data-protection obligations, not by the protocol.

**Recipient compelling disclosure.** A recipient cannot force a holder to produce a proof. Compelled disclosure is a legal mechanism, not a cryptographic one; this layer enables disclosure when the holder is willing, and the auditor variants serve as the cryptographic backstop when the holder is not.

**Side-channel inference from event metadata.** Sender and recipient addresses are cleartext in transfer events ([Design Goals](../protocol/README.md#design-goals)). A disclosure recipient who reads the event log can already determine *who* transacted with *whom* without any disclosure proof. The disclosure layer protects only the amount.

---

## Out of Scope

The following extensions are deliberately not part of this document. They are mentioned to make the scope boundary explicit:

**Delegated viewers (passive ongoing disclosure).** A deployment where the same counterparty needs every transfer disclosed (e.g., a custody bank with continuous AML monitoring) would prefer extra per-transfer ciphertexts to per-transfer disclosure proofs. This requires core-protocol changes: an on-chain registry of viewer keys per account and modifications to the transfer-family circuits to emit additional ciphertexts. Out of scope here; revisit if real deployments demonstrate the need.

**Merkle-accumulated event history with non-membership proofs.** Would enable cryptographic completeness ("the disclosed set is exhaustive") without trusting the auditor. Requires substantial on-chain storage changes and a new accumulator-maintenance circuit.

**Public disclosure proofs (no recipient binding).** Replacing the U-block with a public-input $$v_{\text{transfer}}$$ produces a portable proof anyone can verify. Useful for fire-and-forget compliance archives but loses recipient binding. Not included as a primary variant; can be added by trivially dropping U1–U3 and exposing $$v_{\text{transfer}}$$ as a public input. This is also the proof shape an on-chain verifier would consume ([On-Chain Verification](protocol.md#on-chain-verification)).

---

## Implementation Notes

### Circuits

Four new Noir circuits are added to the proof system:

| Circuit | Purpose |
|:---|:---|
| `disclose_recipient` | D-recipient ([D-recipient](circuits/d-recipient.md)) and its aggregate form ([Aggregate Disclosures](circuits/aggregate.md)) |
| `disclose_sender` | D-sender ([D-sender](circuits/d-sender.md)) and its aggregate form |
| `disclose_auditor` | D-auditor ([D-auditor](circuits/d-auditor.md)) and its aggregate form |
| `disclose_balance` | D-balance ([D-balance](circuits/d-balance.md)), exposed as predicate-only (`disclose_balance_ge` / `disclose_balance_le`) and value-revealing (`disclose_balance_value`) variants |

The aggregate forms can be implemented as a single parameterized circuit per role with a compile-time event-count bound, or as a family of circuits at $$n \in \\{1, 4, 16, 64\\}$$ to balance proving time against generality.

These circuits do *not* register with the on-chain verifier set ([Proof System](../protocol/proof-system.md)). They are verified entirely off-chain.

### Wallet Responsibilities

A wallet that supports holder-side disclosures must:

1. Recover the event's ephemeral scalar $$r_e$$ per [ECDH-Derived Blinding](../protocol/keys-and-commitments.md#ecdh-derived-blinding) when constructing a D-sender proof ([D-sender](circuits/d-sender.md)). No per-transfer storage is required: both $$r_e$$ and $$v_{\text{transfer}}$$ are recomputed at disclosure time from the wallet's $$vk$$ and the on-chain event.
2. Retain the latest opening $$(v_s, r_s)$$ of $$C_{\text{spend}}$$ to support D-balance. This is part of the wallet's normal spend state.
3. Index event references (transaction hash, log index) per account event to enable selecting events by user-facing criteria (date, counterparty).
4. Expose a UI flow that takes a disclosure request $$(P_R, \nu)$$ and a target event (or set), produces the disclosure proof, and delivers the result over the requested channel.

### Verifier Library

A standalone verifier library (independent of the wallet) consumes:

- Network endpoint or RPC for on-chain reads.
- Disclosure recipient's $$(r_R, P_R)$$ keypair.
- Proof bundle $$(\pi, R_{\text{disc}}, \tilde{v}_{\text{disc}})$$ and event reference.

It returns the decrypted $$v_{\text{transfer}}$$ on successful verification, or a typed error indicating which check failed (proof verification, on-chain state mismatch, decryption failure).

---

Previous: [Aggregate Disclosures](circuits/aggregate.md) · Up: [Documentation Index](../README.md)
