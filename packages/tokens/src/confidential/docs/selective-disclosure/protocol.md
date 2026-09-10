# Disclosure Protocol

## Disclosure Ciphertext to Recipient

All variants below share a common output stage that encrypts the disclosed value $$v\_{\text{transfer}}$$ under the recipient's key $$P\_R$$.

The prover samples an ephemeral scalar $$r\_{\text{disc}} \in \mathbb{F}\_r$$ by the rejection-sampling procedure of [Grumpkin-BN254 Cycle](../protocol/primitives.md#grumpkin-bn254-cycle) and computes:

$$R\_{\text{disc}} = r\_{\text{disc}} \cdot H$$
$$s\_{\text{disc}} = \text{ECDH}(r\_{\text{disc}}, P\_R) \qquad \text{(Elliptic Curve Diffie-Hellman)}$$
$$\tilde{v}\_{\text{disc}} = v\_{\text{transfer}} + \text{Poseidon}(\delta\_{\text{disc}}, s\_{\text{disc}}, \nu)$$

The recipient decrypts:

$$s\_{\text{disc}} = \text{ECDH}(r\_R, R\_{\text{disc}}), \qquad v\_{\text{transfer}} = \tilde{v}\_{\text{disc}} - \text{Poseidon}(\delta\_{\text{disc}}, s\_{\text{disc}}, \nu)$$

The pair $$(R\_{\text{disc}}, \tilde{v}\_{\text{disc}})$$ is part of the proof's public inputs. The disclosed amount is therefore confidential to any party other than the recipient even if the proof itself is archived in the clear.

This block is constraints **U1–U3**:

| # | Constraint |
|:--|:---|
| U1 | $$R\_{\text{disc}} = r\_{\text{disc}} \cdot H$$ |
| U2 | $$s\_{\text{disc}} = \text{ECDH}(r\_{\text{disc}}, P\_R)$$ |
| U3 | $$\tilde{v}\_{\text{disc}} = v\_{\text{transfer}} + \text{Poseidon}(\delta\_{\text{disc}}, s\_{\text{disc}}, \nu)$$ |

Subsequent variants reference this block by name.

---

## Proof Bundle and Verifier Protocol

The [circuit specifications](circuits/) define what each variant proves. This section defines the *transport* and the *verifier's mandatory checks*, which are common to all variants and are the only checks that bind a proof to specific on-chain state.

The disclosure layer runs entirely between two off-chain parties — the prover (holder, sender, or auditor) and the disclosure recipient — with the blockchain participating only as a read-only source of truth. Three properties define this operating model.

**Shared circuit artifacts.** Both parties derive their tools from the *same* compiled Noir circuit. The prover side runs the proving key and the witness generator, embedded in the wallet ([Wallet Responsibilities](security.md#wallet-responsibilities)); the recipient side runs the verification key for the matching `circuit_id`, embedded in the verifier library ([Verifier Library](security.md#verifier-library)). A verification key is the cryptographic fingerprint of one specific circuit, so the `circuit_id` carried in the bundle ([Proof Bundle](#proof-bundle)) is what lets the recipient load the right key and reject a proof produced by any other circuit. Prover and verifier must therefore agree, out of band, on *which* compiled circuit each `circuit_id` denotes. A recipient who loads a verification key for a maliciously altered circuit can be convinced of false statements, so the verification-key set is a trusted input.

**Authenticated channel.** The request ([End-to-End Flow](#end-to-end-flow) step 1) and the bundle delivery ([End-to-End Flow](#end-to-end-flow) step 4) travel over a channel the two parties already trust for authenticity — TLS to a compliance API, a signed email, a dedicated KYC portal. The channel carries the recipient's $$(P\_R, \nu)$$ outbound and the bundle inbound. What it does **not** have to provide is confidentiality of the disclosed value: the value is sealed to $$P\_R$$ inside $$\tilde{v}\_{\text{disc}}$$ ([Disclosure Ciphertext to Recipient](#disclosure-ciphertext-to-recipient)), so an eavesdropper learns nothing even in the value-revealing variants, and channel confidentiality is defense-in-depth rather than a requirement.

**Independent on-chain reads.** Neither party trusts the other for any value the chain can supply. The recipient resolves every event field and account record directly from the ledger ([Verifier Protocol](#verifier-protocol)) through its own RPC endpoint or indexer ([Verifier Library](security.md#verifier-library)); the prover likewise reads the chain to assemble its witnesses and public inputs. The bundle is the *only* prover-to-verifier data transfer, and it carries only the proof and the references needed to locate the on-chain anchors — never the anchors themselves ([Proof Bundle](#proof-bundle)).

The net effect is that a disclosure leaves no on-chain trace: no transaction, no event, no state change is associated with it ([Non-Goals](README.md#non-goals), [End-to-End Flow](#end-to-end-flow)). The chain does not know that a disclosure happened, to whom, or about what.

### Event Reference

An **event reference** $$\text{ref}\_E$$ uniquely identifies one on-chain transfer-family event:

$$\text{ref}\_E = (\text{tx\\\_hash}, \text{op\\\_index}, \text{event\\\_index})$$

where `tx_hash` is the Soroban transaction hash that emitted the event and `event_index` selects the event among those the transaction emitted; `op_index` is always 0, since a Soroban transaction carries a single operation. This triple deterministically resolves to exactly one on-chain event, which is all the verifier's lookup ([Verifier Protocol](#verifier-protocol)) needs — it is a resolver, not an ordering key. Its `tx_hash` and `event_index` are the same fields the durable indexer keys on; the indexer's full event-identity and ordering scheme is defined in [Terminology](../indexer.md#terminology) — `(ledger_seq, tx_hash, event_index)` for identity, `(ledger_seq, tx_application_order, event_index)` for order. Implementations MAY substitute any identifier the indexer in use exposes, provided it deterministically resolves to a single on-chain event.

### Proof Bundle

The prover (holder, sender, or auditor) delivers the following bundle to the disclosure recipient over any authenticated channel:

$$\text{Bundle} = (\text{circuit\\\_id}, \text{ref}\_E, \pi, R\_{\text{disc}}, \tilde{v}\_{\text{disc}})$$

| Field | Purpose |
|:---|:---|
| `circuit_id` | Identifies the variant — D-recipient, D-sender, D-auditor, or one of their aggregate / balance / randomness sub-forms — and, for D-auditor, the channel disclosed. Pins the verification key the recipient loads. |
| $$\text{ref}\_E$$ | Event reference ([Event Reference](#event-reference)). Tells the verifier which on-chain event the proof claims to describe. An aggregate form ([Aggregate Disclosures](circuits/aggregate.md)) carries one $$\text{ref}\_{E,i}$$ per event, in the order the circuit's per-event list expects. |
| $$\pi$$ | UltraHonk proof. |
| $$R\_{\text{disc}}, \tilde{v}\_{\text{disc}}$$ | Disclosure ciphertext ([Disclosure Ciphertext to Recipient](#disclosure-ciphertext-to-recipient)). Also appear in the proof's public-input vector. |

The bundle does **not** include the event's payload, the disclosing account's address, or any of the circuit's other public inputs. Those are reconstructed by the verifier from $$\text{ref}\_E$$ and from on-chain state, never accepted from the prover's bundle. This is the analogue of [Public Input Sources](../protocol/operations/README.md#public-input-sources)'s trust-boundary rule: the prover supplies the proof and the event reference; everything else comes from authenticated on-chain state.

### Verifier Protocol

Given a bundle for $$(P\_R, \nu)$$ that this verifier previously issued, the recipient MUST perform every step below in order. Each step's failure is a hard reject; the recipient MUST NOT learn $$v\_{\text{transfer}}$$ from a bundle that fails any step.

1. **Resolve the event.** Look up $$\text{ref}\_E$$ via the indexer or via direct RPC of the transaction. The lookup MUST return exactly one event whose contract address equals the deployed confidential-token contract. Extract the event's payload fields verbatim:
   - For `Transfer`: `from`, `to`, $$R\_e$$, $$\sigma$$, $$\tilde{v}$$, $$\tilde{b}$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$ ([Event Schema](../protocol/interface.md#event-schema)).
   - For `SpenderTransfer`: `spender`, `from`, `to`, $$R\_e$$, $$\sigma\_a'$$, $$\tilde{v}$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{a}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$ ([Event Schema](../protocol/interface.md#event-schema)).

   Any other event type, or a `circuit_id` whose constraints reference a field the event does not carry, is rejected here.

2. **Resolve the disclosing account(s).** Determine which on-chain account records the proof's $$\text{PVK}\_A$$ (and, for D-sender, $$\text{PVK}\_B$$) MUST be drawn from. This is dictated by the variant and the event payload — NOT by anything in the bundle:
   - D-recipient: $$\text{PVK}\_A$$ is read from the account at $$E.\text{to}$$.
   - D-sender on `Transfer`: $$\text{PVK}\_A$$ from $$E.\text{from}$$, $$\text{PVK}\_B$$ from $$E.\text{to}$$.
   - D-sender on `SpenderTransfer`: $$\text{PVK}\_A$$ from $$E.\text{spender}$$, $$\text{PVK}\_B$$ from $$E.\text{to}$$.
   - D-auditor: no on-chain account record is consulted for $$\text{PVK}\_A$$; instead the auditor key $$K\_{\text{aud}}$$ is resolved per step 3.

3. **Resolve auxiliary on-chain state.** Read $$\text{addr\\\_f}$$ from the contract's instance storage ([Governance and Upgradeability](../protocol/system-model.md#governance-and-upgradeability)). For D-auditor, look up the auditor key for the disclosing account's `auditor_id` at the version active at the event's ledger ([Auditor's off-chain obligation](../protocol/auditing.md#auditors-off-chain-obligation)); pick $$K\_{\text{aud,r}}$$ vs. $$K\_{\text{aud,s}}$$ according to the channel the `circuit_id` names. The verifier MUST reject if the version cannot be resolved (auditor contract has no key active at that ledger).

4. **Construct the public-input vector.** Build the vector from the event payload (step 1), the on-chain account records (step 2), the auxiliary state (step 3), the recipient's own $$(P\_R, \nu)$$, and the bundle's $$(R\_{\text{disc}}, \tilde{v}\_{\text{disc}})$$. The verifier MUST NOT use any value from the bundle other than these last two. If any public input the circuit expects is unavailable (e.g., a referenced account is not registered), the verifier rejects.

5. **Verify the proof.** Run UltraHonk verification with the verification key for `circuit_id` against the constructed public inputs and $$\pi$$. Reject on failure.

6. **Decrypt.** Compute $$s\_{\text{disc}} = \text{ECDH}(r\_R, R\_{\text{disc}})$$ and $$v\_{\text{transfer}} = \tilde{v}\_{\text{disc}} - \text{Poseidon}(\delta\_{\text{disc}}, s\_{\text{disc}}, \nu)$$ as in [Disclosure Ciphertext to Recipient](#disclosure-ciphertext-to-recipient). Aggregate forms substitute $$\delta\_{\text{disc\\\_bind}}$$ for $$\delta\_{\text{disc}}$$ ([Aggregate Disclosures](circuits/aggregate.md)).

### On-Chain Verification

Nothing about the proofs themselves prevents a contract from verifying them. They are UltraHonk proofs, and the confidential-token contract already runs UltraHonk verification on-chain for the core transfer-family circuits ([Proof System](../protocol/proof-system.md)); a disclosure circuit's verification key could be registered the same way, behind an entry point that verifies a submitted proof and acts on the result. Verifying on-chain means submitting the proof as a transaction, which publishes the disclosure's existence, the recipient identity, the referenced event, and the timing into the public ledger. The defining property of the off-chain model is that a disclosure leaves no on-chain trace.

**When it makes sense.** The situation that justifies the cost for on-chain verification is when the *result* of a disclosure must gate another on-chain action: a compliance escrow that releases funds only after a "balance ≥ $$X$$" proof verifies, an on-chain attestation registry, a permissioned pool that admits an account once an eligibility predicate passes.

**A separate on-chain verifier protocol (out of scope).** Serving those cases is a protocol in its own rather than an addition to the disclosure layer, but its shape is straightforward, and the trust-boundary rule ([Proof Bundle](#proof-bundle)) carries over unchanged: the public inputs must come from somewhere other than the prover, who supplies only $$\pi$$.

For *current-state* facts the inputs are already on-chain. A D-balance predicate ([D-balance](circuits/d-balance.md)) draws its public inputs — $$\text{addr\\\_f}$$, $$\text{PVK}\_A$$, $$C\_{\text{spend}}$$ — from the token contract's live storage, which a verifier contract reads by cross-contract call (`confidential_balance`, [On-Chain Read Surface](#on-chain-read-surface)). The verifier contract assembles the vector itself and runs UltraHonk verification ([Proof System](../protocol/proof-system.md)) to produce a verdict the gating logic consumes.

For *event-anchored* facts the event fields ($$R\_e$$, $$\sigma\_E$$, $$\tilde{v}$$, the auditor ciphertexts) are emitted as events rather than held in contract storage, so they reach the contract through the request itself. A natural design is request/response: the disclosure recipient — the party that will rely on the verdict — submits an on-chain request to the verifier contract carrying the event data to be proven; the contract records it under the requester's state; the prover then posts $$\pi$$ in a follow-up transaction; and the contract builds the public-input vector from the requester's stored request, plus whatever it reads from the token contract (such as $$\text{PVK}\_A$$), and verifies. The trust boundary holds because the inputs originate with the requester and the token contract, never with the prover — the same division of roles as the off-chain protocol, where the verifier is likewise the party that supplies the public inputs. The one design point such a protocol must settle is that the contract attests *consistency with the submitted event data* but does not by itself confirm that data is a genuine ledger event: that suffices when the requester is the consumer of the verdict, but trustworthiness to unrelated third parties needs an additional event-inclusion binding.

Either way the request and the proof are public, which is the deliberate privacy cost. None of this is part of the present document ([Out of Scope](security.md#out-of-scope)); it is outlined here only to show that on-chain verification is a separable protocol rather than a property of this layer.

---

## On-Chain Read Surface

The confidential-token contract requires no new state-modifying entry points to support this layer. The disclosure verifier needs the following public reads, all of which return data that is already stored or already emitted:

| Read | Purpose | Notes |
|:---|:---|:---|
| `confidential_balance(account) -> ConfidentialAccount` | Verifier extracts $$\text{PVK}\_A$$ (and $$\text{PVK}\_B$$ for D-sender, $$C\_{\text{spend}}$$ for D-balance) from the returned `ConfidentialAccount` tuple | Already exposed ([Read Methods](../protocol/interface.md#read-methods)); the struct carries every field this layer reads, so no narrower accessor is required |
| Auditor contract's key lookup for `auditor_id` | Verifier looks up $$K\_{\text{aud,r}}$$ or $$K\_{\text{aud,s}}$$ | Already exposed ([Auditor Key Management and Rotation](../protocol/auditing.md#auditor-key-management-and-rotation)). The auditor contract MAY maintain a sequence of versioned keys per `auditor_id` with activation ledgers; the verifier MUST select the version whose activation ledger is the largest value not exceeding the disclosed event's ledger ([Auditor's off-chain obligation](../protocol/auditing.md#auditors-off-chain-obligation)). |
| Transfer-family events | Verifier reads the per-event fields ($$R\_e$$, $$\sigma$$ or $$\sigma\_a'$$, $$\tilde{v}$$, $$\tilde{b}$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$ / $$\tilde{a}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$) | Already emitted ([Event Schema](../protocol/interface.md#event-schema)). `SpenderTransfer` uses $$\sigma\_a'$$ in place of $$\sigma$$ and $$\tilde{a}\_{\text{aud,s}}$$ in place of $$\tilde{b}\_{\text{aud,s}}$$. |
| Instance storage: $$\text{addr\\\_f}$$ | D-recipient, D-sender, and D-balance bind $$vk$$ derivation to the contract via $$\text{addr\\\_f}$$ | Computed once at construction ([Governance and Upgradeability](../protocol/system-model.md#governance-and-upgradeability)); the verifier reproduces it from the contract address using the encoding in [Address-to-Field Encoding](../protocol/primitives.md#address-to-field-encoding) |

These are the only on-chain dependencies. Disclosure proofs are otherwise self-contained off-chain artifacts.

---

## End-to-End Flow

The diagram below shows the D-recipient case. D-sender and D-auditor follow the same shape with different prover roles and a different `circuit_id` in step (4); D-balance ([D-balance](circuits/d-balance.md)) differs in that no $$\text{ref}\_E$$ is exchanged — see [D-balance](circuits/d-balance.md) for its bundle shape.

```text
   Recipient/Verifier                                Holder Wallet
         |                                                |
         |  (1) request: "disclose event E to me"         |
         |      includes (P_R, ν) and ref_E               |
         | ---------------------------------------------->|
         |                                                |
         |                                       (2) resolve ref_E to
         |                                           on-chain event E
         |                                       (3) run D-recipient
         |                                           circuit locally
         |                                                |
         |  (4) Bundle =                                  |
         |      (circuit_id, ref_E, π, R_disc, ṽ_disc)    |
         | <----------------------------------------------|
         |                                                |
   (5) Verifier Protocol step 1: resolve ref_E from chain
       (event E with from, to, R_e, σ_E, ṽ, ...)
   (6) Verifier Protocol step 2: read PVK_A from E.to
   (7) Verifier Protocol step 3: read addr_f from instance storage
   (8) Verifier Protocol step 4: build public-input vector
   (9) Verifier Protocol step 5: verify π
   (10) Verifier Protocol step 6: decrypt ṽ_disc with r_R, ν
        → learns v_transfer
```

The recipient MAY agree on ref_E in step (1) ahead of time (e.g., "disclose the transfer at tx 0xabc, log #3") or leave it to the holder, in which case the bundle in step (4) is the first time ref_E is communicated; either way the verifier resolves it independently in step (5). There is no on-chain transaction for the disclosure itself. Steps (1) and (4) flow over any authenticated channel the parties already use (TLS, signed email, dedicated compliance API).

---

Previous: [Selective Disclosure](README.md) · Up: [Index](../README.md#companions) · Next: [D-recipient](circuits/d-recipient.md)
