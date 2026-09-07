# Registration

An account provides a Grumpkin spending key $$Y$$, a public viewing key $$\text{PVK}$$, and a chosen `auditor_id`, accompanied by a proof of key well-formedness.

## Constraints

| # | Constraint |
|:--|:---|
| R1 | $$Y = sk \cdot H$$ (spending key well-formed) |
| R2 | $$vk = \text{Poseidon}(\delta_{\text{vk}}, sk, \text{addr\\\_f})$$ (viewing key correctly derived, binds proof to contract) |
| R3 | $$\text{PVK} = vk \cdot H$$ (public viewing key matches $$vk$$) |
| R4 | $$sk \neq 0$$ (rules out $$Y = \mathcal{O}$$) |
| R5 | $$vk \neq 0$$ (rules out $$\text{PVK} = \mathcal{O}$$, which would collapse every incoming-transfer ECDH) |

## Public inputs

| Input | Notes |
|:---|:---|
| $$Y$$, $$\text{PVK}$$ | Prover-supplied; written to `account.spending_public_key` and `account.viewing_public_key` on success |
| $$\text{addr\\\_f}$$ | Loaded from instance storage; set once at construction ([Governance and Upgradeability](../system-model.md#governance-and-upgradeability)) |
| $$\text{acct\\\_f}$$ | Binds the proof to the registering address that is authenticated with `require_auth()`|

$$\text{acct\\\_f}$$ is referenced by no circuit constraint; its membership in the public-input set is the binding. The verifier absorbs every public input into the proof transcript, so a proof produced for one account fails verification when the contract assembles the blob for any other address. Without this input, the register proof and its public keys — all published on-chain by a legitimate registration — could be replayed by any caller to create duplicate-key accounts under fresh addresses.

## Private witnesses

$$sk$$.

## Post-verification state

The contract validates that `auditor_id` exists in the auditor contract and points to a valid key, then stores `spending_public_key`, `viewing_public_key`, `auditor_id`, and initializes `spendable_commitment = receiving_commitment = ` $$\mathcal{O}$$.

**Auditor selection.** The registering account owner chooses `auditor_id` freely: the register proof does not constrain it, and the core validates only that the id exists in the auditor registry. On a shared auditor registry, deployments that must restrict which auditors an account may bind to MUST enforce that restriction in their `Hooks::on_register` implementation — the default `ComplianceHooks::on_register` deliberately does not restrict it. See [Restrict Auditor Selection](../../compliance.md#restrict-auditor-selection) for a worked example.

---

Up: [Operations](README.md) · Next: [Deposit](deposit.md)
