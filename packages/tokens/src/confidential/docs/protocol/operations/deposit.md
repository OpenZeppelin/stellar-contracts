# Deposit

Transparent tokens flow from the depositor to the contract via `token.transfer(from, self, amount)`. The amount $$a$$ is public and typed as `i128`. The contract checks $$a \ge 0$$ at the entrypoint and reverts on violation ([Underlying Token Assumptions](../system-model.md#underlying-token-assumptions)). The contract then computes the deposit commitment with zero blinding:

$$C_{\text{dep}} = a \cdot G + 0 \cdot H = a \cdot G$$

and adds it to the recipient's receiving balance:

$$C_{\text{receive}} \leftarrow C_{\text{receive}} + C_{\text{dep}}$$

No proof required. The recipient `to` **must** be registered: the receiving-balance update writes into `to`'s `ConfidentialAccount` storage entry. The depositor `from` does **not** need a registered confidential account; only the SEP-41 `token.transfer(from, self, a)` authorization is required. The recipient's off-chain state updates: $$v_{\text{receive}} \mathrel{+}= a$$, $$r_{\text{receive}} \mathrel{+}= 0$$.
