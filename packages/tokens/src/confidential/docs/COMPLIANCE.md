# Confidential Token: Compliance Extensions

## Abstract

This document specifies optional, deployer-configurable controls layered on top of the core Confidential Token (see [DESIGN.md](DESIGN.md)). It covers account freezing, SAC authorization passthrough, pluggable authorization policies, customization patterns for the `Hooks` extension surface, and the pooled-custody clawback flow.

All controls are configured at construction time through a single `compliance: Option<ComplianceConfig>` entry. A vanilla deployment leaves the entry empty and pays no compliance overhead. Regulated deployments populate the entry once; subsequent state changes (freeze toggles, admin rotation, policy swap) flow through admin-gated entry points.

---

## 1. Configuration

```rust
struct ComplianceConfig {
    policy: Option<Address>, // §3
    sac_passthrough: bool,   // §2
}
```

| Field | Purpose |
|:---|:---|
| `policy` | Optional external authorization contract (§3). `None` means no policy gate. |
| `sac_passthrough` | When `true`, every state-modifying operation additionally consults the underlying SAC's `authorized()` check (§2). |

The constructor takes `compliance: Option<ComplianceConfig>`. When `None`, the contract behaves exactly as `DESIGN.md` specifies: no pre-checks run, and the admin-gated entry points in §6 revert with `NotConfigured`.

### 1.1 Admin Authority

This document refers to an "admin" as the authority gating freeze, unfreeze, configuration rotation, and clawback. The contract does not prescribe how that authority is structured. Implementors can compose with an access-control module from the OpenZeppelin Soroban library (e.g., `ownable` for a single-owner model or `access_control` for role-based separation between freeze, policy, and clawback authorities). Admin-gated entry points invoke the chosen module's check (`only_owner`, `only_role`, etc.) at the top of the function.

Deployments that need separation of duties (distinct freeze, policy, and clawback signers) reach for RBAC; deployments with a single jurisdictional authority use ownable. The contract sees only the result of the access check.

---

## 2. Contract-Level Freeze

The contract maintains a `frozen(account) -> bool` entry per account. Before applying any state change, every state-modifying operation runs `check_not_frozen` against each account it names (sender, recipient). A frozen account cannot send, receive, deposit, or withdraw. The check reverts at the contract boundary.

Full freeze (rather than outbound-only) keeps semantics clean: no further accumulation is possible after the freeze takes effect.

### 2.1 Core Interface Additions

Three functions are added to the core contract interface:

```rust
impl Token {
    fn freeze(e: Env, account: Address, admin: Address);   // admin auth
    fn unfreeze(e: Env, account: Address, admin: Address); // admin auth
    fn is_frozen(e: Env, account: Address) -> bool;
}
```

`freeze` and `unfreeze` are gated by the implementor's access-control module (§1.1) and revert when `compliance.is_none()`. `is_frozen` is a public read; it returns `false` when compliance is not configured.

### 2.2 SAC Authorization Passthrough

When `sac_passthrough = true` and the underlying SEP-41 is a Stellar Asset Contract, every state-modifying operation additionally calls `sac.authorized(account)` for each named account and reverts on `false`. This composes the contract's freeze with the issuer's freeze without requiring the admin to mirror state:

$$\text{permitted}(a) = \neg \text{frozen}(a) \;\land\; \text{policy\\\_ok}(a) \;\land\; (\neg \text{sac\\\_passthrough} \;\lor\; \text{sac.authorized}(a))$$

Off by default. Issuer-led deployments using a SAC underlying opt in at construction. The cost is one extra cross-contract invocation per named account per operation.

---

## 3. Policy Contract

When `compliance.policy = Some(addr)`, every state-modifying operation invokes `policy.is_authorized(account, token) -> bool` on the configured contract for each named account, reverting on `false`. The policy is consulted in addition to the freeze check and (where enabled) the SAC passthrough.

```rust
trait Policy {
    fn is_authorized(e: Env, account: Address, token: Address) -> bool;
}
```

This single hook covers the common deployment modes without baking them into the contract:

- **Allowlist:** the policy returns `true` only for listed addresses.
- **Denylist:** the policy returns `true` for everything except listed addresses.
- **KYC / ASP / sanctions screening:** the policy delegates to an identity registry, attestation provider, or sanctions oracle.

Membership management, list semantics, and identity proofs live entirely inside the policy contract. The token's only agreement with the policy is the boolean return value.

Externalizing the policy also lets a single registry serve multiple tokens. An issuer running several confidential tokens (different denominations, jurisdictions, or product lines) can point every token at the same KYC or sanctions contract and maintain one source of truth, rather than mirroring lists into each token. The `token` argument to `is_authorized` lets the registry apply per-token rules when needed (e.g., a jurisdiction filter) without giving up the shared baseline.

The policy address is rotatable via `set_compliance_config` (§6) under admin auth (§1.1). Setting it to `None` disables the gate. The policy is part of the deployment's trust surface.

---

## 4. Customizing the Hooks Trait

The compliance surface in §§2–3 is delivered as `ComplianceHooks`, a turnkey implementation of the contract's `Hooks` trait (see [DESIGN.md](DESIGN.md) for the lifecycle hooks the contract exposes at each entry point). Deployments that need behaviour beyond the default gating — for example, the deposit-side policies sketched below — replace `ComplianceHooks` with a bespoke `Hooks` impl. The custom impl typically delegates to the same primitives the default uses (`storage::gate_account`, `storage::check_policy`, `storage::check_sac`) and only overrides the callbacks that require non-default semantics.

`deposit` is the canonical entry point for customization because it is the only operation where `from` may legitimately be an address that has never registered with the contract (the depositor only needs to hold the underlying SEP-41). The default `ComplianceHooks::on_deposit` gates both `from` and `to` unconditionally, which means every depositor must first register and pass the policy gate. Deployments that need other semantics override `on_deposit`.

### 4.1 Permit Unregistered Deposits

```rust
impl Hooks for PermissiveDepositHooks {
    fn on_deposit(e: &Env, from: &Address, to: &Address, _amount: i128) {
        let Some(config) = storage::compliance_config(e) else {
            return;
        };
        if account_exists(e, from) {
            storage::gate_account(e, from, &config);
        } else if config.sac_passthrough {
            // SAC `authorized` still runs — the underlying SEP-41 transfer
            // would fail anyway when the SAC has the depositor unauthorized.
            storage::check_sac(e, from, &config);
        }
        storage::gate_account(e, to, &config);
    }

    // …other callbacks delegate to ComplianceHooks defaults…
}
```

When `from` is not registered with the contract, the freeze and policy gates are skipped; checks on `to` (the registered recipient) are unaffected. This pattern fits regulated deployments that accept inbound payments from non-listed external counterparties (e.g., an exchange wallet depositing into a payroll pool) while keeping recipient-side guarantees intact.

### 4.2 Permit Deposits Only For Oneself

```rust
impl Hooks for SelfDepositOnlyHooks {
    fn on_deposit(e: &Env, from: &Address, to: &Address, amount: i128) {
        if from != to {
            panic_with_error!(e, ComplianceError::NotAuthorizedByPolicy);
        }
        ComplianceHooks::on_deposit(e, from, to, amount);
    }

    // …other callbacks delegate to ComplianceHooks defaults…
}
```

The depositor is required to be the recipient — no one can deposit on someone else's behalf. This pattern fits deployments where each account must self-fund its confidential balance and inbound deposits from third parties are not a desired flow (e.g., to prevent unsolicited "dustings" that complicate auditor bookkeeping).

These two examples are illustrative; the same surface accommodates per-deposit rate limits, allowlists keyed off the deposit amount, mirror writes to an audit log, or any other synchronous policy. The token's only agreement with the `Hooks` impl is that callbacks revert (via `panic_with_error!`) when the operation must be rejected.

---

## 5. Clawback (Outline Only)

### 5.1 The Pooled-Custody Problem

Once an account deposits into the contract, the underlying SEP-41 ledger lists the token contract as the holder of those funds, not the depositor. An issuer's SAC-level `clawback(token_address, amount)` call would drain the pool, debiting unrelated accounts. The contract therefore does not forward SAC-level clawback to individual confidential accounts; it must instead extract value from a single targeted account's confidential balance and settle that value to the issuer through a transparent path.

The challenge is that the contract does not know the targeted account's balance. The balance is held as a Pedersen commitment whose opening is private to the owner. The clawback amount must be validated against the actual encrypted value without exposing it on-chain and without trusting the admin to choose a value at random.

### 5.2 Admin + Auditor Coordination

The clawback flow requires cooperation between two roles:

- **Admin** authorizes the action on-chain: freezes the target, calls the clawback entry point, and settles the recovered amount to the issuer.
- **Auditor** unlocks knowledge of the target's balance. Two halves of the target's confidential position are covered by the two auditor channels (see `DESIGN.md` §8.1, §8.2). The **sender-auditor** decrypts the spendable-balance checkpoint $\tilde{b}_{\text{aud,s}}$ from the target's most recent owner-initiated event, recovering $v_s$. The **recipient-auditor** decrypts the per-transfer pairs $(v_{\text{tx},i}, r_{\text{tx},i})$ from every inbound transfer and spender-transfer since the last merge, recovering the full Pedersen opening $(v_r, r_r)$ of the target's `receiving_balance`. The auditor then produces a zero-knowledge proof bounding the clawback amount by $v_s + v_r$, without revealing either summand.

Neither party can act alone: the admin cannot produce the proof, and the auditor cannot freeze the account or move funds. This is the same trust separation present in the core protocol (admin governs state transitions, auditor governs visibility) extended to a write surface.

The admin role here is the same access-control surface introduced in §1.1; deployments typically place it under a dedicated `clawback` role in RBAC, separate from the freeze role.

**Auditor routing.** The recipient-auditor and the sender-auditor roles for a single account are served by the same key: each account binds a single `auditor_id` at registration (`DESIGN.md` §6.1) which the contract uses for both the sender-channel ciphertexts on the account's outgoing operations and the recipient-channel ciphertexts on the account's incoming transfers (the two channels are separated by domain tags $\delta_{\text{aud\\\_s}}$ and $\delta_{\text{aud\\\_r}}$, not by distinct keys). Deployments that intend to use clawback therefore need only ensure the off-chain custodian of that key is operationally capable of producing both halves of the witness — the spendable-balance checkpoint decryption and the per-transfer $r_{\text{tx},i}$ replay — when the admin initiates a seizure.

### 5.3 New Circuit

The clawback proof is a constant-size circuit deployed through the existing Verifier surface. It binds the seize amount $\alpha$ by the sum of the spendable and receiving balances of the target account, refreshes the spendable-balance checkpoint, and rewrites `receiving_balance` to a zero commitment so the seized inbound flow is consumed atomically.

**Public inputs.** $C_{\text{spend}}, C_{\text{receive}}, K_{\text{aud,s}}, \tilde{b}_{\text{aud,s}}^{\text{old}}, R_e^{\text{old}}, \sigma^{\text{old}}, \alpha, \tilde{b}_{\text{aud,s}}^{\text{new}}, R_e^{\text{new}}, \sigma^{\text{new}}, addr\_f$.

**Private witnesses.** $k_{\text{aud,s}}, v_s, r_s, v_r, r_r, r_e^{\text{new}}$, plus the sponge outputs from old and new auditor-channel sponge calls. The recipient-auditor's secret key does not appear in the witness because the recipient-channel decryption (recovery of $(v_r, r_r)$ from per-transfer events) is performed off-chain by the auditor; the circuit only re-verifies the resulting Pedersen opening of $C_{\text{receive}}$ (constraint 1).

**Constraints (sketch).**

1. **Receiving-balance opening.** $C_{\text{receive}} = v_r \cdot G + r_r \cdot H$. The recipient-auditor reconstructs $(v_r, r_r)$ off-chain from per-transfer events; the proof asserts knowledge of this opening.
2. **Spendable-balance decryption.** $(m_{v,s}^{\text{old}}, m_{b,s}^{\text{old}}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_s}}, (k_{\text{aud,s}} \cdot R_e^{\text{old}}).x, \sigma^{\text{old}})$ and $v_s = \tilde{b}_{\text{aud,s}}^{\text{old}} - m_{b,s}^{\text{old}}$. The spendable-balance opening $(v_s, r_s)$ is consistent with $C_{\text{spend}} = v_s \cdot G + r_s \cdot H$ where $r_s$ is recovered via the same path the wallet uses for checkpoint recovery (`DESIGN.md` §5.2): $r_s = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk_A, \sigma^{\text{old}})$. Because the clawback circuit does not have access to $vk_A$, the spendable-balance side of the proof binds via the consistency of $\tilde{b}_{\text{aud,s}}^{\text{old}}$ with $C_{\text{spend}}$ at the time of the last owner-initiated proof. The follow-up revision will pin down whether $r_s$ is supplied as a private witness with an auxiliary opening proof or derived in-circuit from a separately escrowed value.
3. **Range and bound.** $\alpha, v_s, v_r \in [0, 2^{127})$ and $\alpha \le v_s + v_r$.
4. **Refreshed checkpoint.** $R_e^{\text{new}} = r_e^{\text{new}} \cdot H$, $r_e^{\text{new}} \neq 0$, and $\tilde{b}_{\text{aud,s}}^{\text{new}} = (v_s + v_r - \alpha) + m_{b,s}^{\text{new}}$ where $(m_{v,s}^{\text{new}}, m_{b,s}^{\text{new}}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_s}}, (k_{\text{aud,s}} \cdot R_e^{\text{new}}).x, \sigma^{\text{new}})$.

**Post-verification.** The contract sets $C_{\text{spend}} \leftarrow (v_s + v_r - \alpha) \cdot G + r_s' \cdot H$ under fresh deterministic randomness $r_s'$ (admin-derived, since $vk_A$ is unavailable), zeroes $C_{\text{receive}}$, transfers $\alpha$ of the underlying SEP-41 token to the issuer, and emits an event carrying $(\tilde{b}_{\text{aud,s}}^{\text{new}}, R_e^{\text{new}}, \sigma^{\text{new}})$ so the sender-auditor sees the new checkpoint.

**Anti-replay.** The contract consumes $C_{\text{spend}}$ and $C_{\text{receive}}$ as proof public inputs at verification time. If either commitment changes between proof construction and submission (e.g., an inbound transfer arrives), verification fails because the proof was bound to a different $C_{\text{receive}}$. The §2 contract-level freeze applied to the target per §5.2's flow blocks both spending and receiving, so neither $C_{\text{spend}}$ nor $C_{\text{receive}}$ can change between proof construction and submission, and the proof's bindings hold across the isolate-then-settle handshake.

**What is no longer needed.** The earlier sketch of an on-chain receiving-side accumulator and a per-transfer compliance hook on `confidential_transfer`, `confidential_transfer_from`, and `deposit` is not required. The recipient-auditor's opening of $C_{\text{receive}}$ is reconstructed entirely off-chain from event scans (`DESIGN.md` §8.1).

Detailed encoding, the precise treatment of $r_s$, and the two-phase isolate-then-settle entry-point sequencing are deferred to a follow-up revision of this document.

---

## 6. Interface Summary

```rust
impl Token {
    fn __constructor(e: Env, /* core args */, compliance: Option<ComplianceConfig>);

    // Freeze (§2)
    fn freeze(e: Env, account: Address, admin: Address);
    fn unfreeze(e: Env, account: Address, admin: Address);
    fn is_frozen(e: Env, account: Address) -> bool;

    // Config rotation (admin auth per §1.1, reverts when compliance.is_none())
    // Replaces the entire ComplianceConfig in one call.
    fn set_compliance_config(e: Env, config: ComplianceConfig, admin: Address);

    // Reads
    fn compliance_config(e: Env) -> Option<ComplianceConfig>;
}
```

`set_compliance_config` overwrites all three fields atomically. Callers that want to toggle a single field read the current config, modify the relevant field, and pass the updated struct back. This keeps the admin-gated surface to one entry point and avoids per-field rotation helpers.

### 6.1 Events

| Event | Fields |
|:---|:---|
| `Frozen`, `Unfrozen` | `account` |
| `ComplianceConfigChanged` | `policy`, `sac_passthrough` |

Clawback-related events are specified alongside the clawback flow in the follow-up revision.
