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

The spender named by the delegation flows (`set_spender`, `confidential_transfer_from`) is not an account for the purposes of the freeze check: the freeze targets fund ownership, and the spender holds no funds — the value being moved stays the owner's, and freezing the owner halts the delegation. This mirrors the allowance models of the library's fungible and rwa tokens. The spender is instead gated by the policy contract (§3).

### 2.1 Core Interface Additions

Three functions are added to the core contract interface:

```rust
impl Token {
    fn freeze(e: Env, account: Address, operator: Address);   // admin auth
    fn unfreeze(e: Env, account: Address, operator: Address); // admin auth
    fn is_frozen(e: Env, account: Address) -> bool;
}
```

`freeze` and `unfreeze` are gated by the implementor's access-control module (§1.1) and revert when `compliance.is_none()`. `is_frozen` is a public read; it returns `false` when compliance is not configured.

### 2.2 SAC Authorization Passthrough

When `sac_passthrough = true` and the underlying SEP-41 is a Stellar Asset Contract, every state-modifying operation additionally calls `sac.authorized(account)` for each named account and reverts on `false`. This composes the contract's freeze with the issuer's freeze without requiring the admin to mirror state:

$$\text{permitted}(a) = \neg \text{frozen}(a) \\;\land\\; \text{policy\\\_ok}(a) \\;\land\\; (\neg \text{sac\\\_passthrough} \\;\lor\\; \text{sac.authorized}(a))$$

Off by default. Issuer-led deployments using a SAC underlying opt in at construction. The cost is one extra cross-contract invocation per named account per operation. This is the *transitive compliance* pattern: the issuer's own freeze/deauthorize, driven through the SAC's standardized admin interface (`set_authorized`, CAP-0046-06), takes effect at the confidential layer with no state mirrored by the token admin.

Like the contract-level freeze (§2), the SAC check names only fund-holding parties: the spender of a delegated flow is exempt.

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

**Spender gating.** Unlike the freeze (§2) and SAC (§2.2) checks, the policy gate also names the spender of a delegated flow: `set_spender` checks the spender at grant time — a delegation to a policy-denied spender fails when it is established, not only when it is exercised — and `confidential_transfer_from` checks the spender at spend time, alongside `from` and `to`. `revoke_spender` deliberately does not gate the spender: revocation is the owner's escape hatch, and blocking it once the spender turns non-compliant would entrench the bad delegation (the owner is still gated on revocation, as on every other operation).

The policy address is rotatable via `set_compliance_config` (§6) under admin auth (§1.1). Setting it to `None` disables the gate. The policy is part of the deployment's trust surface.

**Why the policy is optional.** Making it required would assume every deployment needs address-level gating, which is not the case. A confidential token deployed over a Stellar Asset Contract can rely on the base asset's own restriction configuration (the issuer's `set_authorized`/freeze, surfaced through `sac_passthrough`, §2.2) instead of a separate policy gate. Non-production deployments — testnet demos where a lightweight dapp suffices — likewise need none.

---

## 4. Customizing the Hooks Trait

The compliance surface in §§2–3 is delivered as `ComplianceHooks`, a turnkey implementation of the contract's `Hooks` trait. Deployments that need behaviour beyond the default gating — for example, the deposit-side policies sketched below — replace `ComplianceHooks` with a bespoke `Hooks` impl. The custom impl typically delegates to the same primitives the default uses (`storage::gate_account`, `storage::check_policy`, `storage::check_sac`) and only overrides the callbacks that require non-default semantics.

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

When `from` is not registered with the contract, this example skips the freeze and policy gates on the sender; checks on `to` (the registered recipient) are unaffected. The pattern fits deployments that accept inbound payments from external counterparties that never register (e.g., an exchange wallet depositing into a payroll pool) while keeping recipient-side guarantees intact.

Skipping the *policy* gate on an unregistered sender is a deliberate trade-off, not a recommendation. The policy contract screens an address and its history (SDN, KYT) and does not require that address to be a registered wrapper user, so a deployment that must screen every inbound counterparty can instead call `storage::check_policy(e, from, &config)` for the unregistered `from` and skip only the registration-dependent freeze check. The default `ComplianceHooks` gates both parties unconditionally.

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

### 4.3 Restrict Auditor Selection

At `register`, the account owner chooses which `auditor_id` the account binds to. The core validates only that the id exists in the auditor registry ([DESIGN.md](DESIGN.md) §7.2), and the default `ComplianceHooks::on_register` deliberately does not restrict the choice. On a shared auditor registry, a deployment with designated auditors gates the selection against its own approved set:

```rust
impl Hooks for ApprovedAuditorHooks {
    fn on_register(e: &Env, account: &Address, auditor_id: u32, payload: Val) {
        let approved: Vec<u32> = e
            .storage()
            .instance()
            .get(&DataKey::ApprovedAuditors)
            .unwrap_or_else(|| Vec::new(e));
        if !approved.contains(auditor_id) {
            panic_with_error!(e, DeploymentError::AuditorNotApproved);
        }
        ComplianceHooks::on_register(e, account, auditor_id, payload);
    }

    // …other callbacks delegate to ComplianceHooks defaults…
}
```

`ApprovedAuditors` is a deployment-maintained instance-storage allowlist (populated at construction or through an admin entry point) and `AuditorNotApproved` a deployment-defined error. Because `auditor_id` is immutable once the account is registered ([DESIGN.md](DESIGN.md) §6.1), this gate is the single point where a deployment controls which auditors may ever observe an account's flows.

---

## 5. Clawback

This section specifies seizing value from a single frozen confidential account: reducing its committed claim by a public amount, bounded by what the account holds, and settling the corresponding underlying over a transparent path. The flow is coordinated rather than unilateral, and it presupposes a freeze (§2), which keeps the target's commitments from changing between proof construction and submission (§5.6).

**Terminology.** This flow is called *clawback* because it mirrors the clawback semantics of Stellar Classic / SAC assets, but it is a distinct mechanism. It is delivered by the opt-in `ConfidentialClawback` trait, whose two entry points are `clawback` and `force_revoke_spender` (§6); a deployment that omits that impl block ships freeze and policy gating with no seizure capability.

### 5.1 The Pooled-Custody Problem

Once an account deposits into the contract, the underlying SEP-41 ledger lists the token contract as the holder of those funds, not the depositor. An issuer's SAC-level `clawback(token_address, amount)` drains the pool as a whole and debits every holder. Seizing from one account therefore has to happen at the confidential layer: reduce that account's committed claim, then reconcile the pool.

The contract does not know the targeted account's balance. `C_spend` and `C_receive` are Pedersen commitments whose openings are private, so the seize amount must be validated against the committed values without exposing them on-chain and without trusting the admin's word for them.

### 5.2 Roles and Separation

- **Token admin** — the access-control authority on the confidential-token contract (§1.1). Authorizes the freeze and the two seizure entry points; decides *whether* to seize.
- **Witness holder** — whoever holds the Pedersen openings of the target's `C_spend` and `C_receive`. Produces the proof and thereby decides *how much* and *where to*, both being bound into it (§5.3).
- **Issuer (SAC admin)** — when the base asset is a Stellar Asset Contract, the holder of its standardized admin interface (CAP-0046-06). Extracts the pool surplus a `None` settlement leaves behind (§5.4) and can freeze independently of the token admin via SAC passthrough (§2.2).

In practice the witness comes from the **auditor**; the admin holds no blinding and cannot produce it. The **owner** can derive both openings from `vk` (`DESIGN.md` §5.2), but is the party being seized from and is not expected to cooperate. The auditor holds both openings by the standing capability of `DESIGN_cont.md` §8.1, advanced across `Clawback` per §5.7. One key serves both channels for an account (`DESIGN.md` §6.1), so a deployment that intends to use clawback need only ensure that key's custodian can assemble both halves.

Neither party can act alone: the admin cannot produce the proof, and the witness holder cannot pass the admin gate. Deployments typically place the seizure authority under a dedicated role, separate from the freeze role (§1.1).

### 5.3 Circuit

The clawback circuit proves that a public seize amount is bounded by the target's committed total without revealing either balance. It is the only circuit with no key-ownership constraint and no ephemeral scalar: both openings are pinned by Pedersen binding.

**Circuit constraints:**

| # | Constraint |
|:--|:---|
| CB1 | `C_spend = Com(v_s, r_s)` (prover knows the spendable opening) |
| CB2 | `C_receive = Com(v_r, r_r)` (prover knows the receiving opening) |
| CB3 | `v_s, v_r, alpha, v_s + v_r - alpha ∈ [0, 2^127)` (range validity, `DESIGN.md` §2.6) |

Range on `v_s` and `v_r` alone does not bound their sum against `alpha`, and an over-seize would drive the committed value negative mod `r` — a commitment the owner can still open but never again satisfy under W4 / T4.

**Public inputs (8 fields):**

| Input | Notes |
|:---|:---|
| `C_spend`, `C_receive` | Loaded from the target's `spendable_commitment` and `receiving_commitment`, in this order |
| `alpha` | Public seize amount from invocation inputs |
| `addr_f` | Loaded from instance storage (`DESIGN.md` §2.7) |
| `acct_f` | Binds the proof to the target account |
| `dest_f` | `address_to_field(destination)` under `Some`, the zero field under `None` |

No public input is prover-supplied (`DESIGN.md` §7.1). `addr_f`, `acct_f`, and `dest_f` are referenced by no gate; their membership in the public-input set is the binding, on the `register` / `acct_f` precedent (`DESIGN.md` §7.2). For `dest_f` that binding is what stops a compromised clawback signer from settling a witness built for one destination to an address of its own choosing; its zero sentinel for `None` is unambiguous because `address_to_field` is a Poseidon2 output.

**Private witnesses:** `v_s`, `r_s`, `v_r`, `r_r` — the openings of the two commitments.

### 5.4 Contract Flow

`clawback(account, amount, destination, data, operator)` runs, after the deployment's access-control check on `operator`:

1. **Preconditions.** `is_frozen(account)`, else `AccountNotFrozen`; `amount > 0`, else `InvalidClawbackAmount`; `destination` is not `Some` naming this contract, else `InvalidClawbackDestination` — such a seize is a `None` in effect while reporting a settlement, which breaks per-branch pool reconciliation for any indexer.
2. **Public inputs.** Assembled in the §5.3 order through `append_point` / `append_field` / `append_amount`, so every coordinate and scalar passes the canonicality guards. `data` decodes to `ClawbackData { proof }`.
3. **Verification** against `CircuitType::Clawback`.
4. **State update.** `C_spend <- C_spend + C_receive - alpha·G` and `C_receive <- O`: the `Merge` rule (`DESIGN.md` §7.4) followed by a public debit, with no fresh randomness. The new opening is `(v_s + v_r - alpha, r_s + r_r)`, which the owner and the auditor recompute from the event's `amount` alone, so the seized account stays spendable. Re-randomizing under a prover-chosen blinding instead would leave the owner unable to open its own commitment.
5. **Settlement**, by `destination`:
   - `None` — no underlying moves.
   - `Some(d)` — exactly `amount` is transferred to `d` in the same invocation.
6. **Event.** `Clawback { account, amount, destination }` (§6.1).

**Extraction order.** The seize and the issuer's extraction are separate invocations by different parties, and Soroban admits one per transaction, so the pool sits mismatched for at least a ledger between them. Nothing in the contract enforces which comes first; under a `None` settlement the extraction must follow the seize:

- **Seize, then extract.** Between the two the pool holds `amount` more than the claims against it. No holder is affected, and the extraction returns it to exact collateralization.
- **Extract, then seize.** Between the two the pool holds `amount` less than the claims against it, so it cannot honour them all: the shortfall lands on whichever holders withdraw last, none of whom is the target. It becomes permanent if the seize then turns out to be unbuildable — no witness holder produces the proof (§5.2).

The confidential debit strictly precedes the SEP-41 transfer, matching `withdraw`. Clawback is the one operation that reduces the sum of claims without a `Withdraw` (`DESIGN_cont.md` §9.3), and it invokes no `Hooks` callback: the freeze gate would reject exactly the account it acts on (`DESIGN_cont.md` §11).

### 5.5 Forced Revocation

Escrowed allowances are invisible to `clawback`, which sees only `C_spend` and `C_receive`. `force_revoke_spender(account, spender, operator)` moves them into reach: with `account` frozen (`AccountNotFrozen` otherwise), it performs the owner's `revoke_spender` fold (`DESIGN.md` §7.9) under the admin gate in place of the owner's authorization and emits the same `RevokeSpender` event, carrying `a_tilde` and `allowance_salt`. No proof is involved, and expired delegations are revocable: expiry blocks spending but not reclamation. Like `clawback`, it invokes no `Hooks` callback.

The owner and the auditor fold the event as they would an owner-initiated one (`DESIGN.md` §5.2; `DESIGN_cont.md` §8.5). For the auditor the archive dependence of `DESIGN_cont.md` §8.5 applies unchanged: `r_a` is not derivable from `a_tilde` and `σ_a`, so its standing opening of `C_spend` survives the fold only if it observed the delegation's last `SetSpender` / `SpenderTransfer` event.

### 5.6 Anti-Replay and the Freeze

`C_spend` and `C_receive` are public inputs, so any change to either between proof construction and submission — an inbound transfer, a merge, a revoke — fails verification with `InvalidProof`. The freeze holds the commitments still. `ConfidentialClawback: ConfidentialCompliance` forces a `freeze` / `unfreeze` implementation but constrains nothing about the deployment's `Hooks`. Wiring `NoHooks` next to a `ConfidentialClawback` impl yields a `freeze` that writes the flag and an `is_frozen` that returns `true` while every token operation stays ungated, so the target spends out before the seizure lands and the admin's only signal is an `InvalidProof` once the commitments have moved. A deployment that enables clawback MUST wire `ComplianceHooks`, or a custom `Hooks` impl that gates the same seven positions (§4).

### 5.7 Wallet and Auditor Consequences

`Clawback` is a receiving-side reset and a `T_0` anchor, alongside `Merge` (`DESIGN.md` §5.2 *Recovery*; `INDEXER.md` §2). It is not a checkpoint: it carries no `b_tilde`, so its effect on the spendable side is absorbed into the next owner-initiated proof operation. The owner applies the `Merge` row of `DESIGN.md` §5.2 followed by `W_spend.v -= amount`; the auditor advances its standing opening identically (`DESIGN_cont.md` §8.1 *Sender-auditor opening capability*). A `RevokeSpender` emitted by `force_revoke_spender` is indistinguishable from an owner-initiated one for both consumers.

---

## 6. Interface Summary

```rust
impl Token {
    // Core arguments as declared in DESIGN_cont.md §11, plus the compliance entry.
    fn __constructor(e: Env, admin: Address, token: Address, verifier: Address,
                     auditor: Address, compliance: Option<ComplianceConfig>);

    // Freeze (§2)
    fn freeze(e: Env, account: Address, operator: Address);
    fn unfreeze(e: Env, account: Address, operator: Address);
    fn is_frozen(e: Env, account: Address) -> bool;

    // Config rotation (admin auth per §1.1, reverts when compliance.is_none())
    // Replaces the entire ComplianceConfig in one call.
    fn set_compliance_config(e: Env, config: ComplianceConfig, operator: Address);

    // Reads
    fn compliance_config(e: Env) -> Option<ComplianceConfig>;

    // Clawback (§5), opt-in via ConfidentialClawback
    fn clawback(e: Env, account: Address, amount: i128, destination: Option<Address>,
                data: Bytes /* XDR ClawbackData { proof } */, operator: Address);
    fn force_revoke_spender(e: Env, account: Address, spender: Address, operator: Address);
}
```

`operator` is the address whose authorization the deployment's access-control module checks (§1.1).

### 6.1 Events

| Event | Fields |
|:---|:---|
| `Frozen`, `Unfrozen` | `account` |
| `ComplianceConfigChanged` | `policy`, `sac_passthrough` |
| `Clawback` | `account` (topic), `amount`, `destination` |
| `RevokeSpender` | as `DESIGN_cont.md` §11.2, when emitted by `force_revoke_spender` |

### 6.2 Errors

The seizure entry points add three variants to `ComplianceError`:

| Error | Code | Raised when |
|:---|:--|:---|
| `AccountNotFrozen` | 3604 | the target of `clawback` or `force_revoke_spender` is not frozen |
| `InvalidClawbackAmount` | 3605 | `amount <= 0` |
| `InvalidClawbackDestination` | 3606 | `destination` is `Some` naming the contract's own address |
