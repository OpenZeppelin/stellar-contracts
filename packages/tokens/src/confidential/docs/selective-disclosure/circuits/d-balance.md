# Circuit D-balance: Holder Discloses Current Balance

The account holder proves a property of their **current** confidential balance to a third party. Unlike the transfer-event variants ([D-recipient](d-recipient.md), [D-sender](d-sender.md), [D-auditor](d-auditor.md)), D-balance attests to present state, not a past event: the proof opens the on-chain Pedersen commitment $$C_{\text{spend}}$$ that records the holder's latest spend-side balance ([Balance Commitments](../../protocol/keys-and-commitments.md#balance-commitments), [Wallet State and Recovery](../../protocol/wallet-state.md)) using the holder's retained opening $$(v_s, r_s)$$. Typical uses are reporting-threshold attestations — "balance is at most $$V_{\text{threshold}}$$" for non-reportability, "balance is at least $$V_{\text{threshold}}$$" for solvency.

The holder maintains $$(v_s, r_s)$$ as normal wallet state — every successful transfer settles a fresh opening ([Wallet State and Recovery](../../protocol/wallet-state.md)) and the wallet retains the latest pair. D-balance therefore needs the current spendable opening, which the wallet either holds locally or reconstructs from the latest checkpoint per [Recovery](../../protocol/wallet-state.md#recovery); ordinary spend-proof construction is under the same condition.

## Public inputs

| Symbol | Source |
|:---|:---|
| $$\text{addr\\\_f}$$ | compressed contract-address Field, loaded from instance storage |
| $$\text{PVK}_A$$ | disclosing account's stored `viewing_public_key` |
| $$C_{\text{spend}}$$ | on-chain Pedersen commitment to the holder's current spend-side balance, read from the account's `confidential_balance` record ([Account Data Model](../../protocol/account-state.md#account-data-model), [Read Methods](../../protocol/interface.md#read-methods)) |
| $$V_{\text{threshold}}$$ | (predicate variant only) threshold value |
| $$P_R, \nu$$ | disclosure recipient pubkey and nonce |
| $$R_{\text{disc}}, \tilde{v}_{\text{disc}}$$ | (value-revealing variant only) disclosure ciphertext |

## Private witnesses

$$sk_A$$, $$vk_A$$, $$v_s$$, $$r_s$$, and $$r_{\text{disc}}$$ when the value-revealing variant is in use.

## Constraints

| # | Constraint |
|:--|:---|
| D1 | $$vk_A = \text{Poseidon}(\delta_{\text{vk}}, sk_A, \text{addr\\\_f})$$ |
| D2 | $$\text{PVK}_A = vk_A \cdot H$$ |
| DB3 | $$C_{\text{spend}}$$ opens to $$(v_s, r_s)$$ under the Grumpkin Pedersen scheme of [Pedersen Commitments](../../protocol/primitives.md#pedersen-commitments) |
| DB4 | (predicate variant) $$v_s \geq V_{\text{threshold}}$$ **or** $$v_s \leq V_{\text{threshold}}$$, fixed per `circuit_id` |
| D5 | $$v_s \in [0, 2^{127})$$ |
| U1–U3 | (value-revealing variant) disclosure ciphertext of $$v_s$$ to recipient ([Disclosure Ciphertext to Recipient](../protocol.md#disclosure-ciphertext-to-recipient)) |

Two `circuit_id` shapes are exposed: a **predicate-only** form (`disclose_balance_ge` / `disclose_balance_le`) that includes DB4 and omits U1–U3, where the proof's mere validity asserts the predicate; and a **value-revealing** form (`disclose_balance_value`) that includes U1–U3 and omits DB4, where the recipient decrypts $$\tilde{v}_{\text{disc}}$$ to learn $$v_s$$ exactly.

D1, D2 bind the proof to the disclosing account. DB3 forces the witnessed $$v_s$$ to be the value the on-chain commitment opens to — by Pedersen binding ([Pedersen Commitments](../../protocol/primitives.md#pedersen-commitments)), no alternative opening exists with non-negligible probability. D5 prevents the predicate from being satisfied by a wrapped-negative $$v_s$$ that doesn't represent any real balance.

**Distinguishing from D-auditor balance variant ([Circuit D-auditor: Auditor Discloses a Transfer](d-auditor.md)).** [Circuit D-auditor: Auditor Discloses a Transfer](d-auditor.md)'s balance variant decrypts the sender's *post-transfer* balance, and with its `lane[2]` sibling the full opening, from a specific transfer event — event-anchored, historical, requires auditor cooperation. D-balance is holder-side, reflects *current* on-chain state, and supports predicate-only disclosure that [Circuit D-auditor: Auditor Discloses a Transfer](d-auditor.md)'s variant does not. Capability does not distinguish them: the auditor tracks the current opening $$(v_s, r_s)$$ of every scoped account's $$C_{\text{spend}}$$ forward through checkpoints, merges, and revokes ([Per-Transfer Auditor Ciphertexts](../../protocol/auditing.md#per-transfer-auditor-ciphertexts), [Auditor Visibility Properties](../../protocol/auditing.md#auditor-visibility-properties)), so it could satisfy DB3 against the live commitment as readily as the holder. What distinguishes D-balance is who is bound: D1 and D2 tie the proof to the holder's $$sk_A$$, making it the holder's own attestation, whereas an auditor-produced current-balance proof would bind through auditor key ownership (A1) instead. No such variant is specified here; a recipient that needs a backstop with auditor attestation uses D-auditor over a recent event, and a recipient that needs predicate-only disclosure, or a statement the holder rather than the auditor stands behind, uses D-balance.

## Verifier flow

D-balance has no on-chain event to reference, so the bundle is:

$$\text{Bundle}_{\text{balance}} = (\text{circuit\\\_id}, \text{account}, \pi, R_{\text{disc}}, \tilde{v}_{\text{disc}}?)$$

where `account` is the disclosing address (agreed during the request, not blindly accepted from the prover) and $$\tilde{v}_{\text{disc}}$$ is omitted in the predicate-only variant. The recipient performs [Verifier Protocol](../protocol.md#verifier-protocol) with the following substitutions:

1. **Resolve account state.** Read `confidential_balance(account)`, extracting $$\text{PVK}_A$$ and $$C_{\text{spend}}$$.
2. **Resolve auxiliary state.** Read $$\text{addr\\\_f}$$ from instance storage.
3. **Construct the public-input vector.** Combine the resolved on-chain state, the recipient's $$(P_R, \nu)$$, the agreed $$V_{\text{threshold}}$$ (predicate variants), and the bundle's $$(R_{\text{disc}}, \tilde{v}_{\text{disc}})$$. As in [Proof Bundle](../protocol.md#proof-bundle) the verifier MUST NOT accept $$\text{PVK}_A$$, $$C_{\text{spend}}$$, or $$V_{\text{threshold}}$$ from the bundle.
4. **Verify proof and decrypt** as in [Verifier Protocol](../protocol.md#verifier-protocol) steps 5–6 (decryption applies only to the value-revealing variant).

The recipient and prover MUST agree on $$V_{\text{threshold}}$$ during the request — otherwise the holder could pick a threshold the recipient never authorized and produce a proof against it. The freshness of the disclosure is the ledger at which the recipient read $$C_{\text{spend}}$$: if $$C_{\text{spend}}$$ changed between proving and verification — by the holder's own operation, or without the holder's participation through a compliance clawback or forced revocation ([Contract Flow](../../compliance.md#contract-flow), [Forced Revocation](../../compliance.md#forced-revocation)) — verification fails naturally; the prover then re-runs against the new commitment.
