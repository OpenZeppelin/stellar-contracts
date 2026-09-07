# Confidential Token: Selective Disclosure

## Abstract

This document specifies an off-chain selective-disclosure layer for the Confidential Token (see [DESIGN.md](DESIGN.md)). The core protocol already provides forward-only auditor visibility (DESIGN_cont.md §8): a registered auditor decrypts every transfer the account participates in. That model is sufficient for trusted-third-party regulatory access but insufficient for the routine compliance case where the account holder must prove a *single* fact (a specific transfer amount, an aggregate over a window, a counterparty relationship) to a *specific* counterparty (a bank's compliance desk, a tax authority, a KYC provider) without granting blanket visibility.

This layer addresses that gap with a family of Noir circuits that produce per-event, recipient-bound disclosure proofs. The on-chain contract is untouched. Disclosure proofs are generated client-side by either the account holder (using their viewing key) or the auditor (using their auditor key), delivered out-of-band, and verified off-chain by the disclosure recipient against the on-chain event log.

---

## Introduction

### Overview

The Confidential Token hides the *amounts* that move between accounts: balances and transfer values live on-chain as encrypted commitments rather than as readable numbers (DESIGN.md §1). That is the right default for privacy, but it collides with a routine reality of regulated finance — sometimes the holder of an account *must* prove a specific fact about their own activity to a specific outside party. A bank's compliance desk asks a customer to show that a particular incoming payment was for the amount claimed. A tax authority asks for the total received over a quarter. A KYC provider asks for evidence that an account's balance sits below a threshold. Each of these is a request to reveal *one* fact, to *one* counterparty, and nothing else.

The protocol already includes an auditor mechanism (DESIGN_cont.md §8), but it is all-or-nothing: an auditor holds a key that decrypts *every* transfer an account takes part in. Handing that key — or its decrypted output — to a bank or a tax office to answer a single question would expose the account's entire history. Selective disclosure is the missing middle ground: a way for the holder to prove exactly one statement, to exactly one recipient, in a form the recipient can check against the public ledger but cannot reuse, resell, or replay against anyone else.

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

### The Selective-Disclosure Gap

The core protocol's audit surface (DESIGN_cont.md §8) gives each auditor a key that decrypts every transfer ciphertext for accounts under their scope. This is appropriate for the auditor-as-disclosure-agent role, where the auditor responds to authorized regulatory requests by decrypting specific events.

Three properties make this insufficient as the only disclosure surface:

1. **Granularity.** The auditor key cannot decrypt one transfer without being able to decrypt all of them. There is no cryptographic enforcement that the auditor disclose only what was asked for.
2. **Counterparty disclosures.** Common compliance flows (KYC source-of-funds proofs, bank inbound-payment attestations, tax declarations) require the holder to disclose to a counterparty that is not the auditor. Routing every such request through the auditor concentrates trust and adds operational latency.
3. **Recipient binding.** A plaintext disclosure can be re-shared, replayed, or archived. There is no cryptographic anchor that ties a disclosed value to the specific recipient that requested it.

### Design Goals

**Per-event scope.** A disclosure proof corresponds to one named on-chain event (or a finite enumerated set), not to an account's history.

**Recipient binding.** Each proof is bound to a specific disclosure recipient's public key plus a fresh nonce so that proofs are non-replayable and not transferable to other parties.

**Verifiable correctness.** The disclosure recipient verifies the proof against on-chain state (event log, account record, auditor key registry) without trusting the prover.

**Zero protocol cost.** No changes to the contract's storage model, entry points, or per-transfer ciphertext layout. Disclosure proofs add wallet-side cost only.

**Two prover roles.** Either the holder (using their viewing key $$vk$$) or the auditor (using their auditor secret $$aud\_{sk}$$) can produce a disclosure proof. The roles share a circuit family with swappable witness blocks.

### Non-Goals

**Completeness proofs.** This layer proves positive statements ("this event paid me $$X$$"). It does not prove negatives ("I have no other transfers from $$Y$$"). Completeness, where required, continues to route through the auditor (DESIGN_cont.md §8) or through a future Merkle-accumulator extension that is out of scope here.

**On-chain disclosure logging.** Disclosures are off-chain artifacts exchanged between holder and recipient. The contract does not log disclosure events; doing so would leak the metadata the rest of the protocol works to hide.

**Disclosure-recipient registry.** Recipients identify themselves by Grumpkin public keys exchanged out-of-band. The contract does not register, gate, or approve specific recipients.

---

## Preliminaries

This document reuses the notation, key hierarchy, and commitment scheme from DESIGN.md §2 and §4 without restatement. The following are referenced repeatedly:

- $$sk\_A$$, $$vk\_A$$, $$\text{PVK}\_A$$: an account's spending key, viewing key, and public viewing key (DESIGN.md §4).
- $$\text{addr\\\_f}$$: the contract's compressed address Field $$\text{address\\\_to\\\_field}(\text{contract})$$, bound into $$vk$$ derivation (DESIGN.md §2.7, §4.2). Stored once at construction in the contract's instance storage (DESIGN.md §3.5).
- $$K\_{\text{aud,s}}$$, $$K\_{\text{aud,r}}$$, $$aud\_{sk}$$: the sender-side and recipient-side auditor Grumpkin public keys, and an auditor's secret key (DESIGN_cont.md §8.1, §8.3). Each account selects an `auditor_id` at registration; the same `auditor_id` may resolve to either role depending on the transfer's direction.
- $$(R\_e, \sigma, \tilde{v}, \tilde{b}, \tilde{v}\_{\text{aud,r}}, \tilde{r}\_{\text{aud,r}}, \tilde{v}\_{\text{aud,s}}, \tilde{b}\_{\text{aud,s}}, \tilde{r}\_{\text{aud,s}})$$: per-transfer event fields (DESIGN.md §7.6, §11.2). For `SpenderTransfer` events the recipient/auditor ECDH nonce is $$\sigma\_a'$$ in place of $$\sigma$$, and the sender-auditor channel emits $$\tilde{a}\_{\text{aud,s}}$$ in place of $$\tilde{b}\_{\text{aud,s}}$$, with `lane[2]`'s $$\tilde{r}\_{\text{aud,s}}$$ carrying the post-transfer allowance blinding $$r\_a'$$ rather than a spendable one (DESIGN.md §7.8, §11.2). Throughout this document, the symbol $$\sigma\_E$$ refers to the **event ECDH nonce**, equal to $$\sigma$$ for `Transfer` events and to $$\sigma\_a'$$ for `SpenderTransfer` events; one circuit handles both families, parameterized by which nonce the disclosing event emitted.
- $$H$$: the Grumpkin Pedersen generator used uniformly for key derivation and ECDH (DESIGN.md §2.3, §2.4).

### Disclosure Recipient

A disclosure recipient publishes a long-lived Grumpkin keypair $$(r\_R, P\_R)$$ with $$P\_R = r\_R \cdot H$$, by the same mechanism they publish any other public key (web PKI, certificate, identity document). Publication is out-of-band; no on-chain registration.

For each disclosure request, the recipient supplies a fresh nonce $$\nu \in \mathbb{F}\_r$$ over an authenticated channel. The pair $$(P\_R, \nu)$$ binds the resulting proof. A holder cannot reuse a proof bound to $$(P\_R, \nu)$$ against a different recipient or against the same recipient's future requests.

### Domain Separators

Two domain separators are specific to this layer: $$\delta\_{\text{disc\\\_bind}}$$, the disclosure-ciphertext domain for aggregate disclosures (§10), and $$\delta\_{\text{disc}}$$, which keys the disclosure ciphertext to the recipient (§4). Neither is absorbed in a core circuit. Both take their values from DESIGN_cont.md §13, which assigns them as a continuation of its own sequence.

A wallet producing a D-sender proof also uses $$\delta\_{\text{eph}}$$ (DESIGN_cont.md §13) to recover the event's ephemeral scalar before proving (§7). That use is off-circuit: no disclosure circuit absorbs the tag.

---

## Threat Model

The disclosure layer inherits the protocol's threat model (DESIGN.md §3.2) and adds:

**Holder is the prover for D-recipient and D-sender variants.** The holder is trusted only to produce *correct* proofs about events they choose to disclose. The holder is *not* trusted to be complete: they may withhold events. Recipients that require completeness must obtain it from the auditor (DESIGN_cont.md §8) or from out-of-band evidence.

**Auditor is the prover for D-auditor variants.** The auditor is trusted to disclose accurately when asked. The auditor's existing trust scope (DESIGN.md §3.3) is not enlarged. That scope includes the full Pedersen opening of every $$C\_a$$ (DESIGN_cont.md §8.5) and, at each checkpoint event that escrows `lane[2]`, of the account's $$C\_{\text{spend}}$$ as of that event (DESIGN_cont.md §8.1 *Sender-auditor opening capability*, §8.2). The latter is standing rather than event-scoped: it carries across merges and across `revoke_spender` (DESIGN_cont.md §8.1). The D-auditor variants expose to a disclosure recipient only what the chosen variant states.

**Disclosure recipient is honest-but-curious.** The recipient correctly verifies proofs and decrypts ciphertexts addressed to their key. The recipient may attempt to replay or rebroadcast proofs; nonce binding prevents reuse against other parties.
