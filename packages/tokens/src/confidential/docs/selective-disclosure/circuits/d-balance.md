# Circuit D-balance: Holder Discloses Current Balance

The account holder proves a property of their **current** confidential balance to a third party. Unlike the transfer-event variants (§6–§8), D-balance attests to present state, not a past event: the proof opens the on-chain Pedersen commitment $$C\_{\text{spend}}$$ that records the holder's latest spend-side balance (DESIGN.md §5.1, §5.2) using the holder's retained opening $$(v\_s, r\_s)$$. Typical uses are reporting-threshold attestations — "balance is at most $$V\_{\text{threshold}}$$" for non-reportability, "balance is at least $$V\_{\text{threshold}}$$" for solvency.

The holder maintains $$(v\_s, r\_s)$$ as normal wallet state — every successful transfer settles a fresh opening (DESIGN.md §5.2) and the wallet retains the latest pair. D-balance therefore needs the current spendable opening, which the wallet either holds locally or reconstructs from the latest checkpoint per DESIGN.md §5.2 *Recovery*; ordinary spend-proof construction is under the same condition.

## Public inputs

| Symbol | Source |
|:---|:---|
| $$\text{addr\\\_f}$$ | compressed contract-address Field, loaded from instance storage |
| $$\text{PVK}\_A$$ | disclosing account's stored `viewing_public_key` |
| $$C\_{\text{spend}}$$ | on-chain Pedersen commitment to the holder's current spend-side balance, read from the account's `confidential_balance` record (DESIGN.md §6.1, §11.3) |
| $$V\_{\text{threshold}}$$ | (predicate variant only) threshold value |
| $$P\_R, \nu$$ | disclosure recipient pubkey and nonce |
| $$R\_{\text{disc}}, \tilde{v}\_{\text{disc}}$$ | (value-revealing variant only) disclosure ciphertext |

## Private witnesses

$$sk\_A$$, $$vk\_A$$, $$v\_s$$, $$r\_s$$, and $$r\_{\text{disc}}$$ when the value-revealing variant is in use.

## Constraints

| # | Constraint |
|:--|:---|
| D1 | $$vk\_A = \text{Poseidon}(\delta\_{\text{vk}}, sk\_A, \text{addr\\\_f})$$ |
| D2 | $$\text{PVK}\_A = vk\_A \cdot H$$ |
| DB3 | $$C\_{\text{spend}}$$ opens to $$(v\_s, r\_s)$$ under the Grumpkin Pedersen scheme of DESIGN.md §2.3 |
| DB4 | (predicate variant) $$v\_s \geq V\_{\text{threshold}}$$ **or** $$v\_s \leq V\_{\text{threshold}}$$, fixed per `circuit_id` |
| D5 | $$v\_s \in [0, 2^{127})$$ |
| U1–U3 | (value-revealing variant) disclosure ciphertext of $$v\_s$$ to recipient (§4) |

Two `circuit_id` shapes are exposed: a **predicate-only** form (`disclose_balance_ge` / `disclose_balance_le`) that includes DB4 and omits U1–U3, where the proof's mere validity asserts the predicate; and a **value-revealing** form (`disclose_balance_value`) that includes U1–U3 and omits DB4, where the recipient decrypts $$\tilde{v}\_{\text{disc}}$$ to learn $$v\_s$$ exactly.

D1, D2 bind the proof to the disclosing account. DB3 forces the witnessed $$v\_s$$ to be the value the on-chain commitment opens to — by Pedersen binding (DESIGN.md §2.3), no alternative opening exists with non-negligible probability. D5 prevents the predicate from being satisfied by a wrapped-negative $$v\_s$$ that doesn't represent any real balance.

**Distinguishing from D-auditor balance variant (§8).** §8's balance variant decrypts the sender's *post-transfer* balance, and with its `lane[2]` sibling the full opening, from a specific transfer event — event-anchored, historical, requires auditor cooperation. D-balance is holder-side, reflects *current* on-chain state, and supports predicate-only disclosure that §8's variant does not. Capability does not distinguish them: the auditor tracks the current opening $$(v\_s, r\_s)$$ of every scoped account's $$C\_{\text{spend}}$$ forward through checkpoints, merges, and revokes (DESIGN_cont.md §8.1, §8.2), so it could satisfy DB3 against the live commitment as readily as the holder. What distinguishes D-balance is who is bound: D1 and D2 tie the proof to the holder's $$sk\_A$$, making it the holder's own attestation, whereas an auditor-produced current-balance proof would bind through auditor key ownership (A1) instead. No such variant is specified here; a recipient that needs a backstop with auditor attestation uses D-auditor over a recent event, and a recipient that needs predicate-only disclosure, or a statement the holder rather than the auditor stands behind, uses D-balance.

## Verifier flow

D-balance has no on-chain event to reference, so the bundle is:

$$\text{Bundle}\_{\text{balance}} = (\text{circuit\\\_id}, \text{account}, \pi, R\_{\text{disc}}, \tilde{v}\_{\text{disc}}?)$$

where `account` is the disclosing address (agreed during the request, not blindly accepted from the prover) and $$\tilde{v}\_{\text{disc}}$$ is omitted in the predicate-only variant. The recipient performs §5.3 with the following substitutions:

1. **Resolve account state.** Read `confidential_balance(account)`, extracting $$\text{PVK}\_A$$ and $$C\_{\text{spend}}$$.
2. **Resolve auxiliary state.** Read $$\text{addr\\\_f}$$ from instance storage.
3. **Construct the public-input vector.** Combine the resolved on-chain state, the recipient's $$(P\_R, \nu)$$, the agreed $$V\_{\text{threshold}}$$ (predicate variants), and the bundle's $$(R\_{\text{disc}}, \tilde{v}\_{\text{disc}})$$. As in §5.2 the verifier MUST NOT accept $$\text{PVK}\_A$$, $$C\_{\text{spend}}$$, or $$V\_{\text{threshold}}$$ from the bundle.
4. **Verify proof and decrypt** as in §5.3 steps 5–6 (decryption applies only to the value-revealing variant).

The recipient and prover MUST agree on $$V\_{\text{threshold}}$$ during the request — otherwise the holder could pick a threshold the recipient never authorized and produce a proof against it. The freshness of the disclosure is the ledger at which the recipient read $$C\_{\text{spend}}$$: if $$C\_{\text{spend}}$$ changed between proving and verification — by the holder's own operation, or without the holder's participation through a compliance clawback or forced revocation (COMPLIANCE.md §5.4, §5.5) — verification fails naturally; the prover then re-runs against the new commitment.
