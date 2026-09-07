# Merge

The owner folds the receiving balance into the spendable balance.

## Contract logic (no proof)

```
require account.require_auth()
C_spend ← C_spend + C_receive
C_receive ← O
```

**Proposition 1** (Merge correctness). If $$C\_{\text{spend}} = \text{Com}(v\_s, r\_s)$$ and $$C\_{\text{receive}} = \text{Com}(v\_r, r\_r)$$ before merge, then after merge $$C\_{\text{spend}} = \text{Com}(v\_s + v\_r, r\_s + r\_r)$$ and $$C\_{\text{receive}} = \mathcal{O} = \text{Com}(0, 0)$$.

*Proof.* By the homomorphic property of Pedersen commitments:
$$C\_{\text{spend}} + C\_{\text{receive}} = (v\_s \cdot G + r\_s \cdot H) + (v\_r \cdot G + r\_r \cdot H) = (v\_s + v\_r) \cdot G + (r\_s + r\_r) \cdot H = \text{Com}(v\_s + v\_r, r\_s + r\_r)$$
No value is created or destroyed. $$\square$$

**Owner state update.** The owner knows the opening of the post-merge commitment: $$v\_{\text{spend}}' = v\_s + v\_r$$, $$r\_{\text{spend}}' = r\_s + r\_r$$. The owner knows $$v\_r$$ and $$r\_r$$ from processing incoming transfer and deposit events into $$W\_{\text{receive}}$$ (Section 5.2, *Update rules*; the per-transfer derivation is Definition 1 in Section 5.3). The values $$v\_s$$ and $$r\_s$$ are known from the owner's last proof output.

**Griefing analysis.** Merge is not front-runnable, and incoming transfers cannot invalidate an in-flight spend proof (Proposition 2, Section 9.1; Proposition 3, Section 9.2).

## Encrypted balance

Merge emits no $$\tilde{b}$$ (there is no proof to enforce consistency between $$\tilde{b}$$ and the post-merge $$C\_{\text{spend}}$$). The next owner-initiated proof operation issues a fresh checkpoint. The auditor tracks incoming amounts independently from transfer events.
