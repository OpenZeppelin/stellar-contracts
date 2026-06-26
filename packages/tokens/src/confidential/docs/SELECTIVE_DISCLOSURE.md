# Confidential Token: Selective Disclosure

## Abstract

This document specifies an off-chain selective-disclosure layer for the Confidential Token (see [DESIGN.md](DESIGN.md)). The core protocol already provides forward-only auditor visibility (DESIGN.md §8): a registered auditor decrypts every transfer the account participates in. That model is sufficient for trusted-third-party regulatory access but insufficient for the routine compliance case where the account holder must prove a *single* fact (a specific transfer amount, an aggregate over a window, a counterparty relationship) to a *specific* counterparty (a bank's compliance desk, a tax authority, a KYC provider) without granting blanket visibility.

This layer addresses that gap with a family of Noir circuits that produce per-event, recipient-bound disclosure proofs. The on-chain contract is untouched. Disclosure proofs are generated client-side by either the account holder (using their viewing key) or the auditor (using their auditor key), delivered out-of-band, and verified off-chain by the disclosure recipient against the on-chain event log.

---

## 1. Introduction

### 1.1 Overview

The Confidential Token hides the *amounts* that move between accounts: balances and transfer values live on-chain as encrypted commitments rather than as readable numbers (DESIGN.md §1). That is the right default for privacy, but it collides with a routine reality of regulated finance — sometimes the holder of an account *must* prove a specific fact about their own activity to a specific outside party. A bank's compliance desk asks a customer to show that a particular incoming payment was for the amount claimed. A tax authority asks for the total received over a quarter. A KYC provider asks for evidence that an account's balance sits below a threshold. Each of these is a request to reveal *one* fact, to *one* counterparty, and nothing else.

The protocol already includes an auditor mechanism (DESIGN.md §8), but it is all-or-nothing: an auditor holds a key that decrypts *every* transfer an account takes part in. Handing that key — or its decrypted output — to a bank or a tax office to answer a single question would expose the account's entire history. Selective disclosure is the missing middle ground: a way for the holder to prove exactly one statement, to exactly one recipient, in a form the recipient can check against the public ledger but cannot reuse, resell, or replay against anyone else.

The mechanism lives entirely *outside* the on-chain contract: both proving and verifying happen off-chain, and the protocol pays nothing (the narrow cases where verification can be moved on-chain are in §5.4). It behaves like a notarized, single-use statement, produced in four steps:

1. **The recipient asks.** The counterparty (bank, auditor, tax office) gives the holder a one-time reference number tied to this specific request.
2. **The holder proves.** Using the secret keys already in their wallet, the holder generates a *zero-knowledge proof* — a mathematical certificate that the claimed fact is true and is genuinely tied to a real event recorded on-chain, without revealing any of the secrets behind it. The disclosed value itself is sealed so that only the requesting recipient can open it.
3. **The holder delivers.** The proof and the sealed value go to the recipient directly, off-chain. They are never published.
4. **The recipient verifies.** The recipient checks the proof against the public on-chain record. If it passes, the recipient learns the one disclosed fact — and can trust it as much as they trust the blockchain itself — and learns nothing else about the account.

Because each proof is locked to the recipient's identity *and* to their one-time reference number, a proof handed to one bank is useless to anyone else, and useless even to the same bank for a different request. A leaked or archived proof reveals nothing.

The layer supports a small family of these statements, each a different shape of question:

- *"This on-chain payment paid me this amount."* — the recipient of a transfer proves what they received (§6).
- *"This on-chain payment was sent by me for this amount."* — the sender of a transfer proves what they sent (§7).
- *"My current balance is this — or is at most this."* — the holder proves a fact about the balance they hold right now (§9).
- *"The total across this set of payments is this."* — the holder proves an aggregate over several transfers at once (§10).
- The same proofs produced by the *auditor* instead of the holder, for cases where the disclosure must come from the regulator-facing side rather than the account owner (§8).

The remainder of this document specifies these statements precisely as zero-knowledge circuits, defines the exact checks a recipient must perform, and analyzes what each one does and does not reveal.

### 1.2 The Selective-Disclosure Gap

The core protocol's audit surface (DESIGN.md §8) gives each auditor a key that decrypts every transfer ciphertext for accounts under their scope. This is appropriate for the auditor-as-disclosure-agent role, where the auditor responds to authorized regulatory requests by decrypting specific events.

Three properties make this insufficient as the only disclosure surface:

1. **Granularity.** The auditor key cannot decrypt one transfer without being able to decrypt all of them. There is no cryptographic enforcement that the auditor disclose only what was asked for.
2. **Counterparty disclosures.** Common compliance flows (KYC source-of-funds proofs, bank inbound-payment attestations, tax declarations) require the holder to disclose to a counterparty that is not the auditor. Routing every such request through the auditor concentrates trust and adds operational latency.
3. **Recipient binding.** A plaintext disclosure can be re-shared, replayed, or archived. There is no cryptographic anchor that ties a disclosed value to the specific recipient that requested it.

### 1.3 Design Goals

**Per-event scope.** A disclosure proof corresponds to one named on-chain event (or a finite enumerated set), not to an account's history.

**Recipient binding.** Each proof is bound to a specific disclosure recipient's public key plus a fresh nonce so that proofs are non-replayable and not transferable to other parties.

**Verifiable correctness.** The disclosure recipient verifies the proof against on-chain state (event log, account record, auditor key registry) without trusting the prover.

**Zero protocol cost.** No changes to the contract's storage model, entry points, or per-transfer ciphertext layout. Disclosure proofs add wallet-side cost only.

**Two prover roles.** Either the holder (using their viewing key $vk$) or the auditor (using their auditor secret $aud_{sk}$) can produce a disclosure proof. The roles share a circuit family with swappable witness blocks.

### 1.4 Non-Goals

**Completeness proofs.** This layer proves positive statements ("this event paid me $X$"). It does not prove negatives ("I have no other transfers from $Y$"). Completeness, where required, continues to route through the auditor (DESIGN.md §8) or through a future Merkle-accumulator extension that is out of scope here.

**On-chain disclosure logging.** Disclosures are off-chain artifacts exchanged between holder and recipient. The contract does not log disclosure events; doing so would leak the metadata the rest of the protocol works to hide.

**Disclosure-recipient registry.** Recipients identify themselves by Grumpkin public keys exchanged out-of-band. The contract does not register, gate, or approve specific recipients.

---

## 2. Preliminaries

This document reuses the notation, key hierarchy, and commitment scheme from DESIGN.md §2 and §4 without restatement. The following are referenced repeatedly:

- $sk_A$, $vk_A$, $\text{PVK}_A$: an account's spending key, viewing key, and public viewing key (DESIGN.md §4).
- $\text{addr\\\_f}$: the contract's compressed address Field $\text{address\\\_to\\\_field}(\text{contract})$, bound into $vk$ derivation (DESIGN.md §2.7, §4.2). Stored once at construction in the contract's instance storage (DESIGN.md §3.5).
- $K_{\text{aud,s}}$, $K_{\text{aud,r}}$, $aud_{sk}$: the sender-side and recipient-side auditor Grumpkin public keys, and an auditor's secret key (DESIGN.md §8.1, §8.3). Each account selects an `auditor_id` at registration; the same `auditor_id` may resolve to either role depending on the transfer's direction.
- $(R_e, \sigma, \tilde{v}, \tilde{b}, \tilde{v}_{\text{aud,r}}, \tilde{r}_{\text{aud,r}}, \tilde{v}_{\text{aud,s}}, \tilde{b}_{\text{aud,s}})$: per-transfer event fields (DESIGN.md §7.6, §11.2). For `SpenderTransfer` events the recipient/auditor ECDH nonce is $\sigma_a$ in place of $\sigma$, and the sender-auditor channel emits $\tilde{a}_{\text{aud,s}}$ in place of $\tilde{b}_{\text{aud,s}}$ (DESIGN.md §7.8, §11.2). Throughout this document, the symbol $\sigma_E$ refers to the **event ECDH nonce**, equal to $\sigma$ for `Transfer` events and to $\sigma_a$ for `SpenderTransfer` events; one circuit handles both families, parameterized by which nonce the disclosing event emitted.
- $H$: the Grumpkin Pedersen generator used uniformly for key derivation and ECDH (DESIGN.md §2.3, §2.4).

### 2.1 Disclosure Recipient

A disclosure recipient publishes a long-lived Grumpkin keypair $(r_R, P_R)$ with $P_R = r_R \cdot H$, by the same mechanism they publish any other public key (web PKI, certificate, identity document). Publication is out-of-band; no on-chain registration.

For each disclosure request, the recipient supplies a fresh nonce $\nu \in \mathbb{F}_r$ over an authenticated channel. The pair $(P_R, \nu)$ binds the resulting proof. A holder cannot reuse a proof bound to $(P_R, \nu)$ against a different recipient or against the same recipient's future requests.

### 2.2 Domain Separators

Three new domain separators are added to the list in DESIGN.md §13:

| Symbol | Use |
|:---|:---|
| $\delta_{\text{disc}}$ | Disclosure ciphertext to recipient |
| $\delta_{\text{disc\_bind}}$ | Nonce binding for aggregate disclosures |
| $\delta_{\text{eph}}$ | Deterministic ephemeral-scalar ($r_e$) derivation for outgoing transfers (§7, §15.2) |

---

## 3. Threat Model

The disclosure layer inherits the protocol's threat model (DESIGN.md §3.2) and adds:

**Holder is the prover for D-recipient and D-sender variants.** The holder is trusted only to produce *correct* proofs about events they choose to disclose. The holder is *not* trusted to be complete: they may withhold events. Recipients that require completeness must obtain it from the auditor (DESIGN.md §8) or from out-of-band evidence.

**Auditor is the prover for D-auditor variants.** The auditor is trusted to disclose accurately when asked. The auditor's existing trust scope (DESIGN.md §3.3) is not enlarged.

**Disclosure recipient is honest-but-curious.** The recipient correctly verifies proofs and decrypts ciphertexts addressed to their key. The recipient may attempt to replay or rebroadcast proofs; nonce binding prevents reuse against other parties.

---

## 4. Disclosure Ciphertext to Recipient

All variants below share a common output stage that encrypts the disclosed value $v_{\text{tx}}$ under the recipient's key $P_R$.

The prover samples an ephemeral scalar $r_{\text{disc}} \in \mathbb{F}_r$ and computes:

$$R_{\text{disc}} = r_{\text{disc}} \cdot H$$
$$S_{\text{disc}} = r_{\text{disc}} \cdot P_R, \qquad s_{\text{disc}} = S_{\text{disc}}.x$$
$$\tilde{v}_{\text{disc}} = v_{\text{tx}} + \text{Poseidon}(\delta_{\text{disc}}, s_{\text{disc}}, \nu)$$

The recipient decrypts:

$$S_{\text{disc}} = r_R \cdot R_{\text{disc}}, \qquad v_{\text{tx}} = \tilde{v}_{\text{disc}} - \text{Poseidon}(\delta_{\text{disc}}, S_{\text{disc}}.x, \nu)$$

The pair $(R_{\text{disc}}, \tilde{v}_{\text{disc}})$ is part of the proof's public inputs. The disclosed amount is therefore confidential to any party other than the recipient even if the proof itself is archived in the clear.

This block is constraints **U1–U3**:

| # | Constraint |
|:--|:---|
| U1 | $R_{\text{disc}} = r_{\text{disc}} \cdot H$ |
| U2 | $S_{\text{disc}} = r_{\text{disc}} \cdot P_R$ |
| U3 | $\tilde{v}_{\text{disc}} = v_{\text{tx}} + \text{Poseidon}(\delta_{\text{disc}}, S_{\text{disc}}.x, \nu)$ |

Subsequent variants reference this block by name.

---

## 5. Proof Bundle and Verifier Protocol

The circuit-specific sections that follow (§6 - §10) define what each variant proves. This section defines the *transport* and the *verifier's mandatory checks*, which are common to all variants and are the only checks that bind a proof to specific on-chain state.

The disclosure layer runs entirely between two off-chain parties — the prover (holder, sender, or auditor) and the disclosure recipient — with the blockchain participating only as a read-only source of truth. Three properties define this operating model.

**Shared circuit artifacts.** Both parties derive their tools from the *same* compiled Noir circuit. The prover side runs the proving key and the witness generator, embedded in the wallet (§15.2); the recipient side runs the verification key for the matching `circuit_id`, embedded in the verifier library (§15.3). A verification key is the cryptographic fingerprint of one specific circuit, so the `circuit_id` carried in the bundle (§5.2) is what lets the recipient load the right key and reject a proof produced by any other circuit. Prover and verifier must therefore agree, out of band, on *which* compiled circuit each `circuit_id` denotes. A recipient who loads a verification key for a maliciously altered circuit can be convinced of false statements, so the verification-key set is a trusted input.

**Authenticated channel.** The request (§12 step 1) and the bundle delivery (§12 step 4) travel over a channel the two parties already trust for authenticity — TLS to a compliance API, a signed email, a dedicated KYC portal. The channel carries the recipient's $(P_R, \nu)$ outbound and the bundle inbound. What it does **not** have to provide is confidentiality of the disclosed value: the value is sealed to $P_R$ inside $\tilde{v}_{\text{disc}}$ (§4), so an eavesdropper learns nothing even in the value-revealing variants, and channel confidentiality is defense-in-depth rather than a requirement.

**Independent on-chain reads.** Neither party trusts the other for any value the chain can supply. The recipient resolves every event field and account record directly from the ledger (§5.3) through its own RPC endpoint or indexer (§15.3); the prover likewise reads the chain to assemble its witnesses and public inputs. The bundle is the *only* prover-to-verifier data transfer, and it carries only the proof and the references needed to locate the on-chain anchors — never the anchors themselves (§5.2).

The net effect is that a disclosure leaves no on-chain trace: no transaction, no event, no state change is associated with it (§1.4, §12). The chain does not know that a disclosure happened, to whom, or about what.

### 5.1 Event Reference

An **event reference** $\text{ref}_E$ uniquely identifies one on-chain transfer-family event:

$$\text{ref}_E = (\text{tx\_hash}, \text{op\_index}, \text{log\_index})$$

where `tx_hash` is the Soroban transaction hash that emitted the event, `op_index` selects the contract invocation within that transaction, and `log_index` selects the event among those emitted by that invocation. This triple is the canonical Stellar event identifier and is what indexers key on (cf. DESIGN.md §5.2 *Event durability requirement*). Implementations MAY substitute an equivalent canonical identifier exposed by the indexer in use, provided it deterministically resolves to a single on-chain event.

### 5.2 Proof Bundle

The prover (holder, sender, or auditor) delivers the following bundle to the disclosure recipient over any authenticated channel:

$$\text{Bundle} = (\text{circuit\_id}, \text{ref}_E, \pi, R_{\text{disc}}, \tilde{v}_{\text{disc}})$$

| Field | Purpose |
|:---|:---|
| `circuit_id` | Identifies the variant — D-recipient, D-sender, D-auditor, or one of their aggregate / balance / randomness sub-forms. Pins the verification key the recipient loads. |
| $\text{ref}_E$ | Event reference (§5.1). Tells the verifier which on-chain event the proof claims to describe. |
| $\pi$ | UltraHonk proof. |
| $R_{\text{disc}}, \tilde{v}_{\text{disc}}$ | Disclosure ciphertext (§4). Also appear in the proof's public-input vector. |

The bundle does **not** include the event's payload, the disclosing account's address, or any of the circuit's other public inputs. Those are reconstructed by the verifier from $\text{ref}_E$ and from on-chain state, never accepted from the prover's bundle. This is the analogue of DESIGN.md §7.1's trust-boundary rule: the prover supplies the proof and the event reference; everything else comes from authenticated on-chain state.

### 5.3 Verifier Protocol

Given a bundle for $(P_R, \nu)$ that this verifier previously issued, the recipient MUST perform every step below in order. Each step's failure is a hard reject; the recipient MUST NOT learn $v_{\text{tx}}$ from a bundle that fails any step.

1. **Resolve the event.** Look up $\text{ref}_E$ via the indexer or via direct RPC of the transaction. The lookup MUST return exactly one event whose contract address equals the deployed confidential-token contract. Extract the event's payload fields verbatim:
   - For `Transfer`: `from`, `to`, $R_e$, $\sigma$, $\tilde{v}$, $\tilde{b}$, $\tilde{v}_{\text{aud,r}}$, $\tilde{r}_{\text{aud,r}}$, $\tilde{v}_{\text{aud,s}}$, $\tilde{b}_{\text{aud,s}}$ (DESIGN.md §11.2).
   - For `SpenderTransfer`: `spender`, `from`, `to`, $R_e$, $\sigma_a$, $\tilde{v}$, $\tilde{v}_{\text{aud,r}}$, $\tilde{r}_{\text{aud,r}}$, $\tilde{v}_{\text{aud,s}}$, $\tilde{a}_{\text{aud,s}}$ (DESIGN.md §11.2).

   Any other event type, or a `circuit_id` whose constraints reference a field the event does not carry, is rejected here.

2. **Resolve the disclosing account(s).** Determine which on-chain account records the proof's $\text{PVK}_A$ (and, for D-sender, $\text{PVK}_B$) MUST be drawn from. This is dictated by the variant and the event payload — NOT by anything in the bundle:
   - D-recipient: $\text{PVK}_A$ is read from the account at $E.\text{to}$.
   - D-sender on `Transfer`: $\text{PVK}_A$ from $E.\text{from}$, $\text{PVK}_B$ from $E.\text{to}$.
   - D-sender on `SpenderTransfer`: $\text{PVK}_A$ from $E.\text{spender}$, $\text{PVK}_B$ from $E.\text{to}$.
   - D-auditor: no on-chain account record is consulted for $\text{PVK}_A$; instead the auditor key $K_{\text{aud}}$ is resolved per step 3.

3. **Resolve auxiliary on-chain state.** Read $\text{addr\\\_f}$ from the contract's instance storage (DESIGN.md §3.5). For D-auditor, look up the auditor key for the disclosing account's `auditor_id` at the version active at the event's ledger (DESIGN.md §8.3 *Auditor's off-chain obligation*); pick $K_{\text{aud,r}}$ vs. $K_{\text{aud,s}}$ according to which channel ciphertext the proof claims to disclose. The verifier MUST reject if the version cannot be resolved (auditor contract has no key active at that ledger).

4. **Construct the public-input vector.** Build the vector from the event payload (step 1), the on-chain account records (step 2), the auxiliary state (step 3), the recipient's own $(P_R, \nu)$, and the bundle's $(R_{\text{disc}}, \tilde{v}_{\text{disc}})$. The verifier MUST NOT use any value from the bundle other than these last two. If any public input the circuit expects is unavailable (e.g., a referenced account is not registered), the verifier rejects.

5. **Verify the proof.** Run UltraHonk verification with the verification key for `circuit_id` against the constructed public inputs and $\pi$. Reject on failure.

6. **Decrypt.** Compute $S_{\text{disc}} = r_R \cdot R_{\text{disc}}$ and $v_{\text{tx}} = \tilde{v}_{\text{disc}} - \text{Poseidon}(\delta_{\text{disc}}, S_{\text{disc}}.x, \nu)$ as in §4.

### 5.4 On-Chain Verification

Nothing about the proofs themselves prevents a contract from verifying them. They are UltraHonk proofs, and the confidential-token contract already runs UltraHonk verification on-chain for the core transfer-family circuits (DESIGN.md §10); a disclosure circuit's verification key could be registered the same way, behind an entry point that verifies a submitted proof and acts on the result. Verifying on-chain means submitting the proof as a transaction, which publishes the disclosure's existence, the recipient identity, the referenced event, and the timing into the public ledger. The defining property of the off-chain model is that a disclosure leaves no on-chain trace.

**When it makes sense.** The situation that justifies the cost for on-chain verification is when the *result* of a disclosure must gate another on-chain action: a compliance escrow that releases funds only after a "balance ≥ $X$" proof verifies, an on-chain attestation registry, a permissioned pool that admits an account once an eligibility predicate passes.

**A separate on-chain verifier protocol (out of scope).** Serving those cases is a protocol in its own rather than an addition to the disclosure layer, but its shape is straightforward, and the trust-boundary rule (§5.2) carries over unchanged: the public inputs must come from somewhere other than the prover, who supplies only $\pi$.

For *current-state* facts the inputs are already on-chain. A D-balance predicate (§9) draws its public inputs — $\text{addr\\\_f}$, $\text{PVK}_A$, $C_{\text{spend}}$ — from the token contract's live storage, which a verifier contract reads by cross-contract call (`confidential_balance`, §11). The verifier contract assembles the vector itself and runs UltraHonk verification (DESIGN.md §10) to produce a verdict the gating logic consumes.

For *event-anchored* facts the event fields ($R_e$, $\sigma_E$, $\tilde{v}$, the auditor ciphertexts) are emitted as events rather than held in contract storage, so they reach the contract through the request itself. A natural design is request/response: the disclosure recipient — the party that will rely on the verdict — submits an on-chain request to the verifier contract carrying the event data to be proven; the contract records it under the requestor's state; the prover then posts $\pi$ in a follow-up transaction; and the contract builds the public-input vector from the requestor's stored request, plus whatever it reads from the token contract (such as $\text{PVK}_A$), and verifies. The trust boundary holds because the inputs originate with the requestor and the token contract, never with the prover — the same division of roles as the off-chain protocol, where the verifier is likewise the party that supplies the public inputs. The one design point such a protocol must settle is that the contract attests *consistency with the submitted event data* but does not by itself confirm that data is a genuine ledger event: that suffices when the requestor is the consumer of the verdict, but trustworthiness to unrelated third parties needs an additional event-inclusion binding.

Either way the request and the proof are public, which is the deliberate privacy cost. None of this is part of the present document (§14); it is outlined here only to show that on-chain verification is a separable protocol rather than a property of this layer.

---

## 6. Circuit D-recipient: Holder Discloses an Inbound Transfer

The account holder is the recipient of an on-chain confidential transfer (either a `Transfer` to them or a `SpenderTransfer` whose `to` is them) and proves to a third party that the transfer was for amount $v_{\text{tx}}$. The same circuit covers both event families because the recipient-side ECDH constraint has identical shape in either case; only the value of the event nonce $\sigma_E$ differs ($\sigma$ for `Transfer`, $\sigma_a$ for `SpenderTransfer`; see DESIGN.md §7.6 T9, §7.8 O9).

**Public inputs**

| Symbol | Source |
|:---|:---|
| $\text{addr\\\_f}$ | compressed contract-address Field, loaded from instance storage (DESIGN.md §2.7, §3.5) |
| $\text{PVK}_A$ | disclosing account's stored `viewing_public_key` (DESIGN.md §6.1); $A$ is the address listed as the event's `to` |
| $R_e, \sigma_E, \tilde{v}$ | from the on-chain event being disclosed (DESIGN.md §11.2). $\sigma_E = \sigma$ for `Transfer`, $\sigma_E = \sigma_a$ for `SpenderTransfer`. |
| $P_R$ | disclosure recipient's Grumpkin pubkey (§2.1) |
| $\nu$ | recipient-supplied nonce (§2.1) |
| $R_{\text{disc}}, \tilde{v}_{\text{disc}}$ | disclosure ciphertext to recipient (§4) |

**Private witnesses:** $sk_A$, $vk_A$, $v_{\text{tx}}$, $r_{\text{disc}}$.

**Circuit constraints (D-recipient):**

| # | Constraint |
|:--|:---|
| D1 | $vk_A = \text{Poseidon}(\delta_{\text{vk}}, sk_A, \text{addr\\\_f})$ (viewing key correctly derived, binds proof to contract; mirrors DESIGN.md R2/T2/W2/S2/V2) |
| D2 | $\text{PVK}_A = vk_A \cdot H$ (binds proof to on-chain account) |
| D3 | $S = vk_A \cdot R_e$ (recipient-side ECDH) |
| D4 | $v_{\text{tx}} = \tilde{v} - \text{Poseidon}(\delta_{\text{tx\_amount}}, S.x, \sigma_E)$ (correct decryption of event amount; matches DESIGN.md T9 for `Transfer` and O9 for `SpenderTransfer`) |
| D5 | $v_{\text{tx}} \in [0, 2^{127})$ (range, DESIGN.md §2.6) |
| U1–U3 | Disclosure ciphertext to recipient (§4) |

D1 and D2 anchor the proof to the disclosing account's on-chain record without revealing $sk_A$ or $vk_A$. D3 and D4 recompute the standard recipient-side decryption that the holder would normally perform offline to learn the incoming amount. The result $v_{\text{tx}}$ then feeds the U-block, which encrypts it to the disclosure recipient.

**Verifier flow.** Follow §5.3 with `circuit_id = D-recipient`. Step 2 resolves $\text{PVK}_A$ at $E.\text{to}$ (the only account record this variant consults). On success, the recipient now knows that the named on-chain event paid the named account exactly $v_{\text{tx}}$ tokens, and learns nothing else.

---

## 7. Circuit D-sender: Sender Discloses an Outbound Transfer

The party that **originated** an on-chain confidential transfer proves to a third party that they paid $v_{\text{tx}}$ to the on-chain recipient address recorded in the event. "Sender" here covers both:

- **`Transfer` events:** the originator is the account holder $A$ at `from`. The disclosing key material is the holder's own $(sk_A, vk_A)$ and the ephemeral scalar $r_e$ used at transfer time.
- **`SpenderTransfer` events:** the originator is the spender at `spender`, **not** the owner at `from`. The disclosing key material is the spender's own $(sk_{\text{op}}, vk_{\text{op}})$ and the ephemeral scalar $r_e$ used at transfer time.

In both cases the prover must supply the ephemeral scalar $r_e$ as a witness: the sender has no ECDH path through their own $vk$ into the event ciphertext $\tilde{v}$ (that ciphertext is keyed to the recipient's $\text{PVK}_B$), so $r_e$ is necessary to reconstruct the recipient-side decryption from the sender's side.

**Deterministic $r_e$ (no per-transfer storage).** Rather than sample $r_e$ from fresh randomness and persist it for every outgoing transfer, a wallet derives it from material it already recovers — the originator's viewing key and the event nonce:

$$r_e = \text{Poseidon2}(\delta_{\text{eph}}, vk, \sigma_E)$$

where $\delta_{\text{eph}}$ is a dedicated domain separator (the `EPHEMERAL_KEY` tag, §2.2), $vk$ is the originator's viewing key ($vk_A$ for `Transfer`, $vk_{\text{op}}$ for `SpenderTransfer`), and $\sigma_E$ is the event nonce ($\sigma$ or $\sigma_a$). This is the same construction the protocol already uses for the normalized spend randomness $r' = \text{Poseidon}(\delta_{\text{spend\_r}}, vk, \sigma)$ and the encrypted-balance mask $\text{Poseidon}(\delta_{\text{enc\_bal}}, vk, \sigma)$ (DESIGN.md §5.2, §5.5): $r_e$ joins the family of per-operation secrets recoverable from $(vk, \sigma_E)$ alone. Because $vk$ is secret, $r_e$ stays secret to everyone but the originator's wallet; because $\sigma_E$ is published in the event, the wallet recomputes $r_e$ at disclosure time having stored nothing.

Once $r_e$ is recovered the disclosed amount follows, $v_{\text{tx}} = \tilde{v} - \text{Poseidon}(\delta_{\text{tx\_amount}}, (r_e \cdot \text{PVK}_B).x, \sigma_E)$ with $\text{PVK}_B$ read from the event's `to` address, so D-sender needs **no** per-transfer wallet state — only the wallet's $vk$ and an on-chain read of the event, matching the storage-free posture of D-recipient (§6). This is a wallet-side convention applied when *constructing* outgoing transfers; the contract and the six circuits are untouched (T5/T6 hold for any $r_e$), and it is forward-looking — a transfer whose $r_e$ was sampled randomly and not retained remains undiscloseable.

**Security note.** Deriving $r_e$ from $\sigma_E$ makes $\sigma_E$ the sole freshness input for the whole transfer, including the recipient and auditor channels that otherwise draw independent freshness from a separately sampled $r_e$. This is safe under the protocol's existing requirement that $\sigma$ be unique per operation: the balance channel $\tilde{b} = v + \text{Poseidon}(\delta_{\text{enc\_bal}}, vk, \sigma)$ and the normalized $r'$ already depend on $\sigma$ alone, so a $\sigma$ collision is already disallowed and is negligible under the rejection sampling of DESIGN.md §2.2. The cost is the loss of $r_e$ as an independent second freshness source; a deployment that wants defense-in-depth against $\sigma$ misuse on the recipient and auditor channels should keep sampling $r_e$ and storing it instead.

In the symbols below, $A$ denotes the **originating** address — the holder's address for `Transfer` and the spender's address for `SpenderTransfer`. $sk_A$ is the originator's spending key, $\text{PVK}_A$ is the originator's stored public viewing key, and $\sigma_E = \sigma$ for `Transfer`, $\sigma_E = \sigma_a$ for `SpenderTransfer`.

**Public inputs**

| Symbol | Source |
|:---|:---|
| $\text{addr\\\_f}$ | compressed contract-address Field, loaded from instance storage |
| $\text{PVK}_A$ | originating account's stored `viewing_public_key` (holder for `Transfer`, spender for `SpenderTransfer`) |
| $R_e, \sigma_E, \tilde{v}$ | from the on-chain event |
| $\text{PVK}_B$ | recipient's stored `viewing_public_key` (looked up from event's `to` address) |
| $P_R, \nu$ | disclosure recipient pubkey and nonce |
| $R_{\text{disc}}, \tilde{v}_{\text{disc}}$ | disclosure ciphertext |

**Private witnesses:** $sk_A$, $vk_A$, $r_e$, $v_{\text{tx}}$, $r_{\text{disc}}$.

**Circuit constraints (D-sender):**

| # | Constraint |
|:--|:---|
| D1 | $vk_A = \text{Poseidon}(\delta_{\text{vk}}, sk_A, \text{addr\\\_f})$ |
| D2 | $\text{PVK}_A = vk_A \cdot H$ |
| DS3 | $R_e = r_e \cdot H$ (prover knows the ephemeral scalar used at transfer time; same shape as DESIGN.md T6 for `Transfer` and O6 for `SpenderTransfer`) |
| DS4 | $S_B = r_e \cdot \text{PVK}_B$ (sender-side ECDH to recipient) |
| DS5 | $v_{\text{tx}} = \tilde{v} - \text{Poseidon}(\delta_{\text{tx\_amount}}, S_B.x, \sigma_E)$ |
| D5 | $v_{\text{tx}} \in [0, 2^{127})$ |
| U1–U3 | Disclosure ciphertext to recipient (§4) |

DS3 anchors $R_e$ to the originator by forcing them to know $r_e$. Combined with D1/D2, this proves the prover is the same party that produced the transfer's ephemeral key — the holder for `Transfer`, the spender for `SpenderTransfer`. DS4 and DS5 reconstruct the recipient-side decryption from the originator's perspective.

**Coverage asymmetry: owner cannot D-sender a `SpenderTransfer`.** The owner whose allowance was spent does not hold $r_e$ for the spender-originated event and has no ECDH path into $\tilde{v}$ (the recipient channel is keyed to $\text{PVK}_B$, not to anything the owner controls). The owner therefore cannot independently produce a D-sender disclosure for a `SpenderTransfer`. The owner's cryptographic paths for that event are:

1. **D-auditor (§8)** routed through the owner's auditor key $K_{\text{aud,s}}$, which decrypts $\tilde{v}_{\text{aud,s}}$ for every `SpenderTransfer` from the owner's account (DESIGN.md §8.4). This is the canonical owner-side path.
2. **D-sender by the cooperating spender.** If the spender is willing, they construct a D-sender proof against the spender's own $(sk_{\text{op}}, \text{PVK}_{\text{op}})$ and deliver it to the owner, who forwards it (or the owner asks the disclosure recipient to accept proofs originated by the spender). The proof's $\text{PVK}_A$ is the spender's PVK; the verifier looks it up at the event's `spender` address.

A D-sender proof for a `SpenderTransfer` proves that the spender (not the owner) paid the on-chain `to`. If the disclosure recipient additionally needs proof that the owner authorized this spender, they read the `SetSpender` event and observe the on-chain `(owner, spender)` delegation entry; nothing in D-sender attests to delegation provenance.

**Verifier flow.** Follow §5.3 with `circuit_id = D-sender`. Step 2 looks up two account records: $\text{PVK}_A$ at $E.\text{from}$ (for `Transfer`) or $E.\text{spender}$ (for `SpenderTransfer`), and $\text{PVK}_B$ at $E.\text{to}$ in both cases.

---

## 8. Circuit D-auditor: Auditor Discloses a Transfer

The auditor proves to a third party that an on-chain event corresponds to a transfer of amount $v_{\text{tx}}$ for one of the accounts under the auditor's scope. Used when the holder is uncooperative or when the disclosure recipient requires a guarantee that the auditor (not just the holder) has attested.

**Which auditor.** Every transfer carries ciphertexts under *two* auditor keys (DESIGN.md §8.1): the recipient-side key $K_{\text{aud,r}}$ (channel $\delta_{\text{aud\_r}}$, two squeezes yielding masks for $v_{\text{tx}}$ and $r_{\text{tx}}$) and the sender-side key $K_{\text{aud,s}}$ (channel $\delta_{\text{aud\_s}}$, two squeezes yielding masks for $v_{\text{tx}}$ and the sender's post-transfer balance). Whichever auditor is disclosing reuses the same shared-secret derivation they perform to read events natively; the circuit additionally encrypts the result to the disclosure recipient.

The constraints below parameterize the channel as $\delta_{\text{aud}} \in \{\delta_{\text{aud\_r}}, \delta_{\text{aud\_s}}\}$ and the corresponding event ciphertext as $\tilde{v}_{\text{aud}} \in \{\tilde{v}_{\text{aud,r}}, \tilde{v}_{\text{aud,s}}\}$. In each case the amount mask is the *first* squeeze of the channel's two-squeeze sponge; the second squeeze ($m_{r,r}$ or $m_{b,s}$) is computed and discarded for an amount disclosure, or used in place of the first for the balance/randomness variants noted below.

**Public inputs**

| Symbol | Source |
|:---|:---|
| $K_{\text{aud}}$ | auditor's on-chain Grumpkin pubkey for the chosen channel ($K_{\text{aud,r}}$ or $K_{\text{aud,s}}$) (DESIGN.md §8.3) |
| $R_e, \sigma_E, \tilde{v}_{\text{aud}}$ | from the on-chain event ($\tilde{v}_{\text{aud,r}}$ for the recipient-side channel, $\tilde{v}_{\text{aud,s}}$ for the sender-side channel). $\sigma_E = \sigma$ for `Transfer`, $\sigma_E = \sigma_a$ for `SpenderTransfer` (DESIGN.md §7.8). |
| $P_R, \nu$ | disclosure recipient pubkey and nonce |
| $R_{\text{disc}}, \tilde{v}_{\text{disc}}$ | disclosure ciphertext |

**Private witnesses:** $aud_{sk}$, $v_{\text{tx}}$, $r_{\text{disc}}$.

**Circuit constraints (D-auditor):**

| # | Constraint |
|:--|:---|
| A1 | $K_{\text{aud}} = aud_{sk} \cdot H$ (auditor key ownership) |
| A2 | $S_{\text{aud}} = aud_{sk} \cdot R_e$ (auditor-side ECDH; mirrors the prover-side $r_e \cdot K_{\text{aud}}$ from DESIGN.md T_a1 / T_a5) |
| A3 | $(m_v, m_2) = \text{SpongeSqueeze}_2(\delta_{\text{aud}}, S_{\text{aud}}.x, \sigma_E)$ (auditor channel sponge; same construction as DESIGN.md §2.5, §8.1) |
| A4 | $v_{\text{tx}} = \tilde{v}_{\text{aud}} - m_v$ (correct decryption of the channel's amount slot, the first squeeze) |
| D5 | $v_{\text{tx}} \in [0, 2^{127})$ |
| U1–U3 | Disclosure ciphertext to recipient (§4) |

D-auditor does not bind to an account record; the auditor key already binds the proof. The disclosure recipient confirms which account the event concerns by reading the event's sender and recipient addresses directly.

**Verifier flow.** Follow §5.3 with `circuit_id = D-auditor` (or the chosen balance / randomness variant). Step 2 is skipped — no $\text{PVK}_A$ lookup is needed. Step 3 resolves $K_{\text{aud}}$ at the event's ledger: $K_{\text{aud,r}}$ from the `auditor_id` on the event's `to` account when disclosing the recipient-side channel, or $K_{\text{aud,s}}$ from the `auditor_id` on the `from` account when disclosing the sender-side channel. `from` is the funds' owner in both `Transfer` and `SpenderTransfer`, since the sender-auditor channel always tracks the owner (DESIGN.md §7.8).

**Balance / randomness variants.** The second squeeze of each channel carries a distinct datum: $m_{b,s}$ (sender's post-transfer balance checkpoint, channel $\delta_{\text{aud\_s}}$, recovered from $\tilde{b}_{\text{aud,s}}$) or $m_{r,r}$ (per-transfer Pedersen randomness, channel $\delta_{\text{aud\_r}}$, recovered from $\tilde{r}_{\text{aud,r}}$). A circuit that discloses either of these substitutes the corresponding event ciphertext for $\tilde{v}_{\text{aud}}$ in A4 and reads $m_2$ rather than $m_v$ from the sponge output. Range constraint D5 applies unchanged to a balance disclosure; for a randomness disclosure D5 is dropped since $r_{\text{tx}} \in \mathbb{F}_r$ is not range-bounded. These variants are not separately tabulated.

---

## 9. Circuit D-balance: Holder Discloses Current Balance

The account holder proves a property of their **current** confidential balance to a third party. Unlike the transfer-event variants (§6–§8), D-balance attests to present state, not a past event: the proof opens the on-chain Pedersen commitment $C_{\text{spend}}$ that records the holder's latest spend-side balance (DESIGN.md §5.1, §5.2) using the holder's retained opening $(v_s, r_s)$. Typical uses are reporting-threshold attestations — "balance is at most $V_{\text{threshold}}$" for non-reportability, "balance is at least $V_{\text{threshold}}$" for solvency.

The holder maintains $(v_s, r_s)$ as normal wallet state — every successful transfer settles a fresh opening (DESIGN.md §5.2) and the wallet retains the latest pair. Loss of the opening disables D-balance until the next inbound transfer reseeds the wallet's spend view; this is the same liveness property that governs ordinary transfer construction.

**Public inputs**

| Symbol | Source |
|:---|:---|
| $\text{addr\\\_f}$ | compressed contract-address Field, loaded from instance storage |
| $\text{PVK}_A$ | disclosing account's stored `viewing_public_key` |
| $C_{\text{spend}}$ | on-chain Pedersen commitment to the holder's current spend-side balance, read from the account's `confidential_balance` record (DESIGN.md §6.1, §11.3) |
| $V_{\text{threshold}}$ | (predicate variant only) threshold value |
| $P_R, \nu$ | disclosure recipient pubkey and nonce |
| $R_{\text{disc}}, \tilde{v}_{\text{disc}}$ | (value-revealing variant only) disclosure ciphertext |

**Private witnesses:** $sk_A$, $vk_A$, $v_s$, $r_s$, and $r_{\text{disc}}$ when the value-revealing variant is in use.

**Circuit constraints (D-balance):**

| # | Constraint |
|:--|:---|
| D1 | $vk_A = \text{Poseidon}(\delta_{\text{vk}}, sk_A, \text{addr\\\_f})$ |
| D2 | $\text{PVK}_A = vk_A \cdot H$ |
| DB3 | $C_{\text{spend}}$ opens to $(v_s, r_s)$ under the Grumpkin Pedersen scheme of DESIGN.md §2.3 |
| DB4 | (predicate variant) $v_s \geq V_{\text{threshold}}$ **or** $v_s \leq V_{\text{threshold}}$, fixed per `circuit_id` |
| D5 | $v_s \in [0, 2^{127})$ |
| U1–U3 | (value-revealing variant) disclosure ciphertext of $v_s$ to recipient (§4) |

Two `circuit_id` shapes are exposed: a **predicate-only** form (`disclose_balance_ge` / `disclose_balance_le`) that includes DB4 and omits U1–U3, where the proof's mere validity asserts the predicate; and a **value-revealing** form (`disclose_balance_value`) that includes U1–U3 and omits DB4, where the recipient decrypts $\tilde{v}_{\text{disc}}$ to learn $v_s$ exactly.

D1, D2 bind the proof to the disclosing account. DB3 forces the witnessed $v_s$ to be the value the on-chain commitment opens to — by Pedersen binding (DESIGN.md §2.3), no alternative opening exists with non-negligible probability. D5 prevents the predicate from being satisfied by a wrapped-negative $v_s$ that doesn't represent any real balance.

**Distinguishing from D-auditor balance variant (§8).** §8's balance variant decrypts the sender's *post-transfer* balance from $\tilde{b}_{\text{aud,s}}$ of a specific transfer event — event-anchored, historical, requires auditor cooperation. D-balance is holder-side, reflects *current* on-chain state, and supports predicate-only disclosure that §8's variant does not. The two are complementary: a recipient that needs a backstop with auditor attestation uses D-auditor; a recipient that needs predicate-only disclosure or that wants to avoid involving the auditor uses D-balance.

**Verifier flow.** D-balance has no on-chain event to reference, so the bundle is:

$$\text{Bundle}_{\text{balance}} = (\text{circuit\_id}, \text{account}, \pi, R_{\text{disc}}, \tilde{v}_{\text{disc}}?)$$

where `account` is the disclosing address (agreed during the request, not blindly accepted from the prover) and $\tilde{v}_{\text{disc}}$ is omitted in the predicate-only variant. The recipient performs §5.3 with the following substitutions:

1. **Resolve account state.** Read `confidential_balance(account)`, extracting $\text{PVK}_A$ and $C_{\text{spend}}$.
2. **Resolve auxiliary state.** Read $\text{addr\\\_f}$ from instance storage.
3. **Construct the public-input vector.** Combine the resolved on-chain state, the recipient's $(P_R, \nu)$, the agreed $V_{\text{threshold}}$ (predicate variants), and the bundle's $(R_{\text{disc}}, \tilde{v}_{\text{disc}})$. As in §5.2 the verifier MUST NOT accept $\text{PVK}_A$, $C_{\text{spend}}$, or $V_{\text{threshold}}$ from the bundle.
4. **Verify proof and decrypt** as in §5.3 steps 5–6 (decryption applies only to the value-revealing variant).

The recipient and prover MUST agree on $V_{\text{threshold}}$ during the request — otherwise the holder could pick a threshold the recipient never authorized and produce a proof against it. The freshness of the disclosure is the ledger at which the recipient read $C_{\text{spend}}$: if the holder transferred between proving and verification, the on-chain $C_{\text{spend}}$ has changed and verification fails naturally; the prover then re-runs against the new commitment.

---

## 10. Aggregate Disclosures

For statements of the form "this account received at least $X$ from counterparty $Y$ during window $W$", the D-recipient circuit (or D-auditor) is vectorized over $n$ events.

**Public inputs**

| Symbol | Source |
|:---|:---|
| Common: $\text{addr\\\_f}$, $\text{PVK}_A$, $P_R$, $\nu$, $R_{\text{disc}}, \tilde{V}_{\text{disc}}$ | as in §6 |
| List: $(R_{e,i}, \sigma_{E,i}, \tilde{v}_i)$ for $i \in [1, n]$ | from $n$ on-chain transfer-family events; $\sigma_{E,i} = \sigma$ if event $i$ is a `Transfer`, $\sigma_a$ if `SpenderTransfer`. Each event MUST be identified by a $\text{ref}_{E,i}$ in the proof bundle and resolved per §5.3. |
| Optional: $V_{\text{threshold}}$ | aggregate threshold |

**Private witnesses:** $sk_A$, $vk_A$, $\{v_{\text{tx},i}\}_{i=1}^n$, $r_{\text{disc}}$.

**Circuit constraints (D-aggregate, n events):**

| # | Constraint |
|:--|:---|
| D1, D2 | As in §6 |
| For each $i$: D3$_i$ | $S_i = vk_A \cdot R_{e,i}$ |
| For each $i$: D4$_i$ | $v_{\text{tx},i} = \tilde{v}_i - \text{Poseidon}(\delta_{\text{tx\_amount}}, S_i.x, \sigma_{E,i})$ |
| For each $i$: D5$_i$ | $v_{\text{tx},i} \in [0, 2^{127})$ |
| AGG | $V_{\text{total}} = \sum_{i=1}^n v_{\text{tx},i}$ |
| THR (optional) | $V_{\text{total}} \geq V_{\text{threshold}}$ |
| U1–U3 | Encrypt $V_{\text{total}}$ (not the individual $v_{\text{tx},i}$) to the recipient, with $\delta_{\text{disc\_bind}}$ replacing $\delta_{\text{disc}}$ to separate domain |

The recipient filters the $n$ events off-chain by the criteria they care about (sender address, block timestamp) before constructing the verifier's public inputs. They learn the aggregate $V_{\text{total}}$ but not the individual amounts. If THR is included and the recipient does not need the aggregate value itself, U1–U3 can be omitted; the proof's mere validity asserts the threshold.

Aggregate disclosures over outbound transfers use the D-sender constraint block per event; aggregate auditor disclosures use the D-auditor block per event.

---

## 11. On-Chain Read Surface

The confidential-token contract requires no new state-modifying entry points to support this layer. The disclosure verifier needs the following public reads, all of which return data that is already stored or already emitted:

| Read | Purpose | Notes |
|:---|:---|:---|
| `confidential_balance(account) -> Bytes` | Verifier extracts $\text{PVK}_A$ (and $\text{PVK}_B$ for D-sender, $C_{\text{spend}}$ for D-balance) from the returned `ConfidentialAccount` tuple | Already exposed (DESIGN.md §11.3); an additional trivial `viewing_public_key(account)` accessor would save the surrounding XDR-decode but is not required |
| Auditor contract's key lookup for `auditor_id` | Verifier looks up $K_{\text{aud,r}}$ or $K_{\text{aud,s}}$ | Already exposed (DESIGN.md §8.3). The auditor contract MAY maintain a sequence of versioned keys per `auditor_id` with activation ledgers; the verifier MUST select the version whose activation ledger is the largest value not exceeding the disclosed event's ledger (DESIGN.md §8.3, *Auditor's off-chain obligation*). |
| Transfer-family events | Verifier reads the per-event fields ($R_e$, $\sigma$ or $\sigma_a$, $\tilde{v}$, $\tilde{b}$, $\tilde{v}_{\text{aud,r}}$, $\tilde{r}_{\text{aud,r}}$, $\tilde{v}_{\text{aud,s}}$, $\tilde{b}_{\text{aud,s}}$ / $\tilde{a}_{\text{aud,s}}$) | Already emitted (DESIGN.md §11.2). `SpenderTransfer` uses $\sigma_a$ in place of $\sigma$ and $\tilde{a}_{\text{aud,s}}$ in place of $\tilde{b}_{\text{aud,s}}$. |
| Instance storage: $\text{addr\\\_f}$ | D-recipient, D-sender, and D-balance bind $vk$ derivation to the contract via $\text{addr\\\_f}$ | Computed once at construction (DESIGN.md §3.5); the verifier reproduces it from the contract address using the encoding in DESIGN.md §2.7 |

These are the only on-chain dependencies. Disclosure proofs are otherwise self-contained off-chain artifacts.

---

## 12. End-to-End Flow

The diagram below shows the D-recipient case. D-sender and D-auditor follow the same shape with different prover roles and a different `circuit_id` in step (4); D-balance (§9) differs in that no $\text{ref}_E$ is exchanged — see §9 for its bundle shape.

```
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
   (5) §5.3 step 1: resolve ref_E from chain
       (event E with from, to, R_e, σ_E, ṽ, ...)
   (6) §5.3 step 2: read PVK_A from E.to
   (7) §5.3 step 3: read addr_f from instance storage
   (8) §5.3 step 4: build public-input vector
   (9) §5.3 step 5: verify π
   (10) §5.3 step 6: decrypt ṽ_disc with r_R, ν
        → learns v_tx
```

The recipient MAY agree on ref_E in step (1) ahead of time (e.g., "disclose the transfer at tx 0xabc, log #3") or leave it to the holder, in which case the bundle in step (4) is the first time ref_E is communicated; either way the verifier resolves it independently in step (5). There is no on-chain transaction for the disclosure itself. Steps (1) and (4) flow over any authenticated channel the parties already use (TLS, signed email, dedicated compliance API).

---

## 13. Security Analysis

### 13.1 Soundness of Disclosed Amount

For the D-recipient circuit, soundness reduces to two facts:

1. D1, D2 force the prover to know an $sk_A$ whose derived $vk_A$ matches the on-chain $\text{PVK}_A$. By DESIGN.md §4.2 this party is the account owner.
2. D3, D4 force $v_{\text{tx}}$ to equal the decryption of $\tilde{v}$ under that owner's $vk_A$. By DESIGN.md §5.3 this is the same value the on-chain transfer commitment $C_{\text{tx}}$ commits to, since the transfer circuit enforced consistent ECDH derivation at transfer time.

Therefore, a soundness break would require either a key-derivation collision (Poseidon2 preimage break, DESIGN.md §2.5, §3.2) or a discrete-log break on Grumpkin. Both are out of scope.

D-sender soundness is symmetric: DS3 forces the prover to know $r_e$ with $R_e = r_e \cdot H$, which by the transfer circuit's constraint T6 (DESIGN.md §7.6) was the same $r_e$ used to derive the auditor and recipient ciphertexts. DS4, DS5 reconstruct the decryption from the sender side. Soundness is independent of how $r_e$ was produced: whether sampled or derived as $\text{Poseidon2}(\delta_{\text{eph}}, vk, \sigma_E)$ (§7), DS3 binds the proof through the event's $R_e$, so the deterministic derivation affects only wallet recovery, not the soundness of the disclosed amount.

D-auditor soundness is direct: A1 forces auditor-key ownership; A2–A4 reconstruct the standard auditor sponge decryption (DESIGN.md §8.1).

D-balance soundness reduces to Pedersen binding (DESIGN.md §2.3): given the on-chain $C_{\text{spend}}$, the prover's witnesses $(v_s, r_s)$ satisfying DB3 uniquely determine $v_s$ up to negligible probability. D1, D2 anchor the proof to the disclosing account as in D-recipient. The "current state" framing is established by the verifier's read protocol, not the circuit: a proof against a stale $C_{\text{spend}}$ simply fails to verify against the current public-input vector, so the recipient only ever accepts proofs about the on-chain state at the moment of verification.

**Event binding.** None of the soundness arguments above pin the proof to a *specific* on-chain event by themselves — they only force consistency with whatever $(\text{PVK}_A, R_e, \sigma_E, \tilde{v})$ tuple the public-input vector commits to. The binding to the on-chain event is established off-chain by the §5.3 verifier protocol: the verifier MUST resolve $\text{ref}_E$ from the bundle to a specific event, MUST take all event-derived public inputs verbatim from that event, and MUST take all account-derived inputs from the on-chain account record at the address the *event* names. Skipping any of these steps voids the binding. Because $R_e$ is sampled fresh per transfer (DESIGN.md §5.3, §9.6), no two distinct on-chain events share an $R_e$ except with negligible probability, so a proof that verifies against the vector built from event $E$ cannot also verify against the vector built from any $E' \neq E$. This is the soundness role the trust-boundary rule (§5.2) plays for the disclosure layer; it is the analogue of DESIGN.md §7.1's on-chain trust-boundary rule.

### 13.2 Recipient Binding

The disclosed value $v_{\text{tx}}$ is delivered only through $\tilde{v}_{\text{disc}} = v_{\text{tx}} + \text{Poseidon}(\delta_{\text{disc}}, s_{\text{disc}}, \nu)$, where $s_{\text{disc}} = r_{\text{disc}} \cdot P_R$ is recoverable only by the holder of $r_R$.

A party other than the intended recipient who obtains $(\pi, R_{\text{disc}}, \tilde{v}_{\text{disc}})$ can verify $\pi$ but cannot decrypt $\tilde{v}_{\text{disc}}$. They learn that *some* value was disclosed but not the value itself.

Nonce $\nu$ is bound into the Poseidon argument. A holder cannot reuse $(\pi, R_{\text{disc}}, \tilde{v}_{\text{disc}})$ against a recipient request that issued a different nonce, because the verifier's public inputs would not match.

### 13.3 What This Does Not Prevent

**Holder cherry-picking.** A holder may disclose three inbound transfers from counterparty $Y$ while withholding a fourth. The verifier cannot detect this from the proof alone. Mitigation: completeness routes through the auditor (D-auditor variants), who sees every event under their scope. Recipients that require completeness must request from the auditor, not the holder.

**Disclosure recipient leakage.** Once decrypted, $v_{\text{tx}}$ is plaintext in the recipient's possession. The recipient may store, share, or leak it. This is a non-cryptographic concern handled by the recipient's own data-protection obligations, not by the protocol.

**Recipient compelling disclosure.** A recipient cannot force a holder to produce a proof. Compelled disclosure is a legal mechanism, not a cryptographic one; this layer enables disclosure when the holder is willing, and the auditor variants serve as the cryptographic backstop when the holder is not.

**Side-channel inference from event metadata.** Sender and recipient addresses are cleartext in transfer events (DESIGN.md §1.2). A disclosure recipient who reads the event log can already determine *who* transacted with *whom* without any disclosure proof. The disclosure layer protects only the amount.

---

## 14. Out of Scope

The following extensions are deliberately not part of this document. They are mentioned to make the scope boundary explicit:

**Delegated viewers (passive ongoing disclosure).** A deployment where the same counterparty needs every transfer disclosed (e.g., a custody bank with continuous AML monitoring) would prefer extra per-transfer ciphertexts to per-transfer disclosure proofs. This requires core-protocol changes: an on-chain registry of viewer keys per account and modifications to the transfer-family circuits to emit additional ciphertexts. Out of scope here; revisit if real deployments demonstrate the need.

**Merkle-accumulated event history with non-membership proofs.** Would enable cryptographic completeness ("the disclosed set is exhaustive") without trusting the auditor. Requires substantial on-chain storage changes and a new accumulator-maintenance circuit.

**Public disclosure proofs (no recipient binding).** Replacing the U-block with a public-input $v_{\text{tx}}$ produces a portable proof anyone can verify. Useful for fire-and-forget compliance archives but loses recipient binding. Not included as a primary variant; can be added by trivially dropping U1–U3 and exposing $v_{\text{tx}}$ as a public input. This is also the proof shape an on-chain verifier would consume (§5.4).

---

## 15. Implementation Notes

### 15.1 Circuits

Four new Noir circuits are added to the proof system:

| Circuit | Purpose |
|:---|:---|
| `disclose_recipient` | D-recipient (§6) and its aggregate form (§10) |
| `disclose_sender` | D-sender (§7) and its aggregate form |
| `disclose_auditor` | D-auditor (§8) and its aggregate form |
| `disclose_balance` | D-balance (§9), exposed as predicate-only (`disclose_balance_ge` / `disclose_balance_le`) and value-revealing (`disclose_balance_value`) variants |

The aggregate forms can be implemented as a single parameterized circuit per role with a compile-time event-count bound, or as a family of circuits at $n \in \{1, 4, 16, 64\}$ to balance proving time against generality.

These circuits do *not* register with the on-chain verifier set (DESIGN.md §10). They are verified entirely off-chain.

### 15.2 Wallet Responsibilities

A wallet that supports holder-side disclosures must:

1. Derive the transfer ephemeral scalar deterministically as $r_e = \text{Poseidon2}(\delta_{\text{eph}}, vk, \sigma_E)$ when constructing each outgoing transfer (§7). D-sender then requires **no** per-transfer storage — both $r_e$ and $v_{\text{tx}}$ are recomputed at disclosure time from the wallet's $vk$ and the on-chain event. A wallet that instead samples $r_e$ from fresh randomness must retain $(r_e, v_{\text{tx}})$ per outbound transfer (tens of bytes each) to keep those transfers disclosable.
2. Retain the latest opening $(v_s, r_s)$ of $C_{\text{spend}}$ to support D-balance. This is part of the wallet's normal spend state.
3. Index event references (transaction hash, log index) per account event to enable selecting events by user-facing criteria (date, counterparty).
4. Expose a UI flow that takes a disclosure request $(P_R, \nu)$ and a target event (or set), produces the disclosure proof, and delivers the result over the requested channel.

### 15.3 Verifier Library

A standalone verifier library (independent of the wallet) consumes:

- Network endpoint or RPC for on-chain reads.
- Disclosure recipient's $(r_R, P_R)$ keypair.
- Proof bundle $(\pi, R_{\text{disc}}, \tilde{v}_{\text{disc}})$ and event reference.

It returns the decrypted $v_{\text{tx}}$ on successful verification, or a typed error indicating which check failed (proof verification, on-chain state mismatch, decryption failure).

This library is the integration point for bank compliance desks, tax software, and KYC providers. It is not part of the confidential-token contract surface.
