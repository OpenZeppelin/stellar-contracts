# Auditor Client

An auditor decrypts from the public event and its own secret $$k$$ alone, with no viewing key, holder cooperation, or extra on-chain read. For each channel it computes the shared scalar against the event's ephemeral point, derives that channel's lane masks (§4.3) — three on the sender / owner channel, two on the recipient channel — and subtracts. The allowance opening comes straight out of the event: the blinding of the $$C_a$$ that operation writes is escrowed in the event itself — tag 17 on `SetSpender`, `lane[2]` on `SpenderTransfer` — and the matching value is in the sender-channel ciphertext (DESIGN_cont.md §8.5). An auditor that did not observe the event holds no opening for that state.

The two channels differ in what they yield (DESIGN_cont.md §8.1):

| Channel | `lane[0]` | `lane[1]` | `lane[2]` |
|:--|:--|:--|:--|
| Sender / owner ($$\delta_{\text{aud\\\_s}}$$) | Transfer amount, or the escrowed amount for `SetSpender` | Sender's post-operation balance, or post-operation allowance for a spender transfer | Post-operation spendable blinding on `Withdraw`, `Transfer`, and `SetSpender`; post-transfer allowance blinding $$r_a'$$ on `SpenderTransfer` |
| Recipient ($$\delta_{\text{aud\\\_r}}$$) | Transfer amount | Per-transfer Pedersen randomness $$r_{\text{transfer}}$$ | — (channel is two-lane) |

`Withdraw` and `SetSpender` carry a sender-channel balance checkpoint whose pad is `lane[1]`. Only `Withdraw` leaves `lane[0]` unused, its amount being public (DESIGN.md W_a3, §4.3); `SetSpender` reads `lane[0]` as well, for the escrowed amount (DESIGN.md S_a4).

An implementation MUST squeeze the sender / owner channel three-wide and MUST NOT widen the recipient channel. `RevokeSpender` opens no auditor channel, the fold being proofless (DESIGN.md §7.9); an implementation MUST treat that event as carrying no escrowed blinding rather than substituting a stale one.

**Cross-channel agreement.** Where an auditor holds the key for both parties, the amount decrypts independently on each channel and the circuit constrains both to the same value, so the two MUST agree. An implementation SHOULD perform this comparison and treat disagreement as evidence that $$k$$ is not the auditor key for both parties of that event.

**Scope MUST be represented, not implied.** The recipient-channel capability is forward-only, reset by merge, and yields no opening of the spendable commitment (DESIGN_cont.md §8.1). The `lane[2]` opening of the sender channel is forward-only and **standing**: it opens the spendable commitment as of the checkpoint that escrowed it and stays valid through merges provided the client folded in every inbound flow since the previous merge, one account key serving both channels (DESIGN_cont.md §8.1 *Sender-auditor opening capability*). Maintaining it is the client's job: an implementation MUST add the `lane[0]` amount and `lane[1]` $$r_{\text{transfer}}$$ of every inbound `Transfer` and `SpenderTransfer` to its stored $$(v, r)$$, and MUST treat each `Deposit` as $$(\text{amount}, 0)$$ (DESIGN.md §7.3). On a `RevokeSpender` event it MUST add the $$(v_a, r_a)$$ it recorded for that delegation, the fold being public and the addend already escrowed (DESIGN_cont.md §8.5); where it holds no such record it MUST mark the opening unavailable until the next `lane[2]` escrow rather than carry a stale one. Across a key rotation, what an implementation may carry forward and what it MUST treat as unopened are fixed by DESIGN_cont.md §8.3.

**Clawback witness.** In a deployment that enables seizure (COMPLIANCE.md §5), the clawback witness is the pair of openings the client already maintains — the standing sender-channel opening of $$C_{\text{spend}}$$ and the recipient-channel opening of $$C_{\text{receive}}$$ (COMPLIANCE.md §5.2, §5.3). An implementation MUST verify both against the on-chain commitments before proving, and on a `Clawback` event MUST advance the standing opening by the `Merge` rule and the event's public `amount` (COMPLIANCE.md §5.7).

An auditor facade MUST NOT be able to construct a spending witness. It can open the spendable balance past a merge — the inbound $$r_{\text{transfer}}$$ reaches the same key on the recipient channel — so a facade that exposes openings exposes them for the account's whole history under the active key.
