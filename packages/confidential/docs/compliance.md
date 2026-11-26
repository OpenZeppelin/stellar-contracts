# Confidential Token: Compliance Extensions

## Abstract

This document specifies optional, deployer-configurable controls layered on top of the core Confidential Token (see [the protocol specification](protocol/README.md)). It covers account freezing, SAC authorization passthrough, pluggable authorization policies, customization patterns for the `Hooks` extension surface, the pooled-custody clawback flow, and the RWA profile for regulated assets.

All controls are configured at construction time through a single `compliance: Option<ComplianceConfig>` entry. A vanilla deployment leaves the entry empty and pays no compliance overhead. Regulated deployments populate the entry once; subsequent state changes (freeze toggles, admin rotation, policy swap) flow through admin-gated entry points. The RWA profile keeps a second entry, `RwaConfig` ([RWA Profile](#rwa-profile)).

---

## Configuration

```rust
struct ComplianceConfig {
    policy: Option<Address>, // Policy Contract
    sac_passthrough: bool,   // Contract-Level Freeze
}
```

| Field | Purpose |
|:---|:---|
| `policy` | Optional external authorization contract ([Policy Contract](#policy-contract)). `None` means no policy gate. |
| `sac_passthrough` | When `true`, every state-modifying operation additionally consults the underlying SAC's `authorized()` check ([Contract-Level Freeze](#contract-level-freeze)). |

The constructor takes `compliance: Option<ComplianceConfig>`. When `None`, the contract behaves exactly as [the protocol specification](protocol/README.md) specifies: no pre-checks run, and the admin-gated entry points in [Interface Summary](#interface-summary) revert with `NotConfigured`.

### Admin Authority

This document refers to an "admin" as the authority gating freeze, unfreeze, configuration rotation, and clawback. The contract does not prescribe how that authority is structured. Implementors can compose with an access-control module from the OpenZeppelin Soroban library (e.g., `ownable` for a single-owner model or `access_control` for role-based separation between freeze, policy, and clawback authorities). Admin-gated entry points invoke the chosen module's check (`only_owner`, `only_role`, etc.) at the top of the function.

Deployments that need separation of duties (distinct freeze, policy, and clawback signers) reach for RBAC, as the [compliant-token example](../../../examples/confidential/compliant-token/src/contract.rs) does; deployments with a single jurisdictional authority use ownable. The contract sees only the result of the access check.

---

## Contract-Level Freeze

The contract maintains a `frozen(account) -> bool` entry per account. Before applying any state change, every state-modifying operation runs `check_not_frozen` against each account it names (sender, recipient). A frozen account cannot send, receive, deposit, or withdraw. The check reverts at the contract boundary.

Full freeze (rather than outbound-only) keeps semantics clean: no further accumulation is possible after the freeze takes effect.

The spender named by the delegation flows (`set_spender`, `confidential_transfer_from`) is not an account for the purposes of the freeze check: the freeze targets fund ownership, and the spender holds no funds — the value being moved stays the owner's, and freezing the owner halts the delegation. This mirrors the allowance models of the library's fungible and rwa tokens. The spender is instead gated by the policy contract ([Policy Contract](#policy-contract)).

### Core Interface Additions

Three functions are added to the core contract interface:

```rust
impl Token {
    fn freeze(e: Env, account: Address, operator: Address);   // admin auth
    fn unfreeze(e: Env, account: Address, operator: Address); // admin auth
    fn is_frozen(e: Env, account: Address) -> bool;
}
```

`freeze` and `unfreeze` are gated by the implementor's access-control module ([Admin Authority](#admin-authority)) and revert when `compliance.is_none()`. `is_frozen` is a public read; it returns `false` when compliance is not configured.

### SAC Authorization Passthrough

When `sac_passthrough = true` and the underlying SEP-41 is a Stellar Asset Contract, every state-modifying operation additionally calls `sac.authorized(account)` for each named account and reverts on `false`. This composes the contract's freeze with the issuer's freeze without requiring the admin to mirror state:

$$\text{permitted}(a) = \neg \text{frozen}(a) \\;\land\\; \text{policy\\\_ok}(a) \\;\land\\; (\neg \text{sac\\\_passthrough} \\;\lor\\; \text{sac.authorized}(a))$$

Off by default. Issuer-led deployments using a SAC underlying opt in at construction. The cost is one extra cross-contract invocation per named account per operation. This is the *transitive compliance* pattern: the issuer's own freeze/deauthorize, driven through the SAC's standardized admin interface (`set_authorized`, CAP-0046-06), takes effect at the confidential layer with no state mirrored by the token admin.

Like the contract-level freeze ([Contract-Level Freeze](#contract-level-freeze)), the SAC check names only fund-holding parties: the spender of a delegated flow is exempt.

---

## Policy Contract

When `compliance.policy = Some(addr)`, every state-modifying operation invokes `policy.is_authorized(account, token) -> bool` on the configured contract for each named account, reverting on `false`. The policy is consulted in addition to the freeze check and (where enabled) the SAC passthrough.

```rust
trait Policy {
    fn is_authorized(e: Env, account: Address, token: Address) -> bool;
}
```

This single hook covers the common deployment modes without baking them into the contract:

- **Allowlist:** the policy returns `true` only for listed addresses ([allowlist-policy example](../../../examples/confidential/allowlist-policy/src/contract.rs)).
- **Denylist:** the policy returns `true` for everything except listed addresses.
- **KYC / ASP / sanctions screening:** the policy delegates to an identity registry, attestation provider, or sanctions oracle.

Membership management, list semantics, and identity proofs live entirely inside the policy contract. The token's only agreement with the policy is the boolean return value.

Externalizing the policy also lets a single registry serve multiple tokens. An issuer running several confidential tokens (different denominations, jurisdictions, or product lines) can point every token at the same KYC or sanctions contract and maintain one source of truth, rather than mirroring lists into each token. The `token` argument to `is_authorized` lets the registry apply per-token rules when needed (e.g., a jurisdiction filter) without giving up the shared baseline.

**Spender gating.** Unlike the freeze ([Contract-Level Freeze](#contract-level-freeze)) and SAC ([SAC Authorization Passthrough](#sac-authorization-passthrough)) checks, the policy gate also names the spender of a delegated flow: `set_spender` checks the spender at grant time — a delegation to a policy-denied spender fails when it is established, not only when it is exercised — and `confidential_transfer_from` checks the spender at spend time, alongside `from` and `to`. `revoke_spender` deliberately does not gate the spender: revocation is the owner's escape hatch, and blocking it once the spender turns non-compliant would entrench the bad delegation (the owner is still gated on revocation, as on every other operation).

The policy address is rotatable via `set_compliance_config` ([Interface Summary](#interface-summary)) under admin auth ([Admin Authority](#admin-authority)). Setting it to `None` disables the gate. The policy is part of the deployment's trust surface.

**Why the policy is optional.** Making it required would assume every deployment needs address-level gating, which is not the case. A confidential token deployed over a Stellar Asset Contract can rely on the base asset's own restriction configuration (the issuer's `set_authorized`/freeze, surfaced through `sac_passthrough`, [SAC Authorization Passthrough](#sac-authorization-passthrough)) instead of a separate policy gate. Non-production deployments — testnet demos where a lightweight dapp suffices — likewise need none.

---

## Customizing the Hooks Trait

The compliance surface of [Contract-Level Freeze](#contract-level-freeze) and [Policy Contract](#policy-contract) is delivered as `ComplianceHooks`, a turnkey implementation of the contract's `Hooks` trait. Deployments that need behaviour beyond the default gating — for example, the deposit-side policies sketched below — replace `ComplianceHooks` with a bespoke `Hooks` impl. The custom impl typically delegates to the same primitives the default uses (`storage::gate_account`, `storage::check_policy`, `storage::check_sac`) and only overrides the callbacks that require non-default semantics.

`deposit` is the canonical entry point for customization because it is the only operation where `from` may legitimately be an address that has never registered with the contract (the depositor only needs to hold the underlying SEP-41). The default `ComplianceHooks::on_deposit` gates both `from` and `to` unconditionally, which means every depositor must first register and pass the policy gate. Deployments that need other semantics override `on_deposit`.

### Permit Unregistered Deposits

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

### Permit Deposits Only For Oneself

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

### Restrict Auditor Selection

At `register`, the account owner chooses which `auditor_id` the account binds to. The core validates only that the id exists in the auditor registry ([Registration](protocol/operations/register.md)), and the default `ComplianceHooks::on_register` deliberately does not restrict the choice. On a shared auditor registry, a deployment with designated auditors gates the selection against its own approved set:

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

`ApprovedAuditors` is a deployment-maintained instance-storage allowlist (populated at construction or through an admin entry point) and `AuditorNotApproved` a deployment-defined error. Because `auditor_id` is immutable once the account is registered ([Account Data Model](protocol/account-state.md#account-data-model)), this gate is the single point where a deployment controls which auditors may ever observe an account's flows.

---

## Clawback

This section specifies seizing value from a single frozen confidential account: reducing its committed claim by a public amount, bounded by what the account holds, and settling the corresponding underlying over a transparent path. The flow is coordinated rather than unilateral, and it presupposes a freeze ([Contract-Level Freeze](#contract-level-freeze)), which keeps the target's commitments from changing between proof construction and submission ([Anti-Replay and the Freeze](#anti-replay-and-the-freeze)).

**Terminology.** This flow is called *clawback* because it mirrors the clawback semantics of Stellar Classic / SAC assets, but it is a distinct mechanism. It is delivered by the opt-in `ConfidentialClawback` trait, whose two entry points are `clawback` and `force_revoke_spender` ([Interface Summary](#interface-summary); implemented in the [compliant-token example](../../../examples/confidential/compliant-token/src/contract.rs)); a deployment that omits that impl block ships freeze and policy gating with no seizure capability.

### The Pooled-Custody Problem

Once an account deposits into the contract, the underlying SEP-41 ledger lists the token contract as the holder of those funds, not the depositor. An issuer's SAC-level `clawback(token_address, amount)` drains the pool as a whole and debits every holder. Seizing from one account therefore has to happen at the confidential layer: reduce that account's committed claim, then reconcile the pool.

The contract does not know the targeted account's balance. `C_spend` and `C_receive` are Pedersen commitments whose openings are private, so the seize amount must be validated against the committed values without exposing them on-chain and without trusting the admin's word for them.

### Roles and Separation

- **Token admin** — the access-control authority on the confidential-token contract ([Admin Authority](#admin-authority)). Authorizes the freeze and the two seizure entry points; decides *whether* to seize.
- **Auditor** — the holder of the secret key behind the auditor the target's account is bound to (`auditor_id`, [Account Data Model](protocol/account-state.md#account-data-model)) and of the Pedersen openings of the target's `C_spend` and `C_receive`. Produces the proof and thereby decides *how much* and *where to*, both being bound into it ([Circuit](#circuit)).
- **Issuer (SAC admin)** — when the base asset is a Stellar Asset Contract, the holder of its standardized admin interface (CAP-0046-06). Extracts the pool surplus a `None` settlement leaves behind ([Contract Flow](#contract-flow)) and can freeze independently of the token admin via SAC passthrough ([SAC Authorization Passthrough](#sac-authorization-passthrough)).

The openings alone do not single out the auditor. An account whose history is registration, public deposits, and merges carries zero blinding on both commitments ([Registration](protocol/operations/register.md), [Deposit](protocol/operations/deposit.md), [Merge](protocol/operations/merge.md)), so anyone — the admin included — reconstructs its openings from events. The circuit therefore also binds the witness to the auditor's secret key (CB1, [Circuit](#circuit)). The auditor holds both openings by the standing capability of [Per-Transfer Auditor Ciphertexts](protocol/auditing.md#per-transfer-auditor-ciphertexts), advanced across `Clawback` per [Wallet and Auditor Consequences](#wallet-and-auditor-consequences).

Neither party can act alone: the admin cannot produce the proof, and the auditor cannot pass the admin gate. Deployments typically place the seizure authority under a dedicated role, separate from the freeze role ([Admin Authority](#admin-authority)).

### Circuit

The clawback circuit proves that a public seize amount is bounded by the target's committed total without revealing either balance, and that the prover is the target's auditor. It is the only circuit with no ephemeral scalar, and the only one whose key-ownership constraint is over an auditor key rather than an owner's: both openings are pinned by Pedersen binding, and the witness is pinned to the account's `auditor_id`.

**Circuit constraints:**

| # | Constraint |
|:--|:---|
| CB1 | `K_aud = k_aud · H` (auditor key ownership) |
| CB2 | `C_spend = Com(v_s, r_s)` (prover knows the spendable opening) |
| CB3 | `C_receive = Com(v_r, r_r)` (prover knows the receiving opening) |
| CB4 | `v_s, v_r, alpha, v_s + v_r - alpha ∈ [0, 2^127)` (range validity, [Integer Embedding and Range Proofs](protocol/primitives.md#integer-embedding-and-range-proofs)) |

CB1 is what makes the witness an approval rather than a computation: without it, the public openings of a zero-blinding account ([Roles and Separation](#roles-and-separation)) would let the clawback signer prove — and settle to itself — a seizure of everything such an account holds. Range on `v_s` and `v_r` alone does not bound their sum against `alpha`, and an over-seize would drive the committed value negative mod `r` — a commitment the owner can still open but never again satisfy under W4 / T4.

**Public inputs (11 fields):**

| Input | Notes |
|:---|:---|
| `C_spend`, `C_receive` | Loaded from the target's `spendable_commitment` and `receiving_commitment`, in this order |
| `K_aud` | Fetched from the auditor registry under the target's `auditor_id` ([Auditor Key Management and Rotation](protocol/auditing.md#auditor-key-management-and-rotation)) |
| `alpha` | Public seize amount from invocation inputs |
| `addr_f` | Loaded from instance storage ([Address-to-Field Encoding](protocol/primitives.md#address-to-field-encoding)) |
| `acct_f` | Binds the proof to the target account |
| `dest_f` | `address_to_field(destination)` under `Some`, the zero field under `None` |
| `nonce` | The account's clawback nonce, loaded from storage ([Contract Flow](#contract-flow)) |

No public input is prover-supplied ([Public Input Sources](protocol/operations/README.md#public-input-sources)). `addr_f`, `acct_f`, `dest_f`, and `nonce` are referenced by no gate; their membership in the public-input set is the binding, on the `register` / `acct_f` precedent ([Registration](protocol/operations/register.md)). For `dest_f` that binding is what stops a compromised clawback signer from settling a witness built for one destination to an address of its own choosing; its zero sentinel for `None` is unambiguous because `address_to_field` is a Poseidon2 output. For `nonce` it is what makes a proof single-use ([Anti-Replay and the Freeze](#anti-replay-and-the-freeze)).

**Private witnesses:** `k_aud`, `v_s`, `r_s`, `v_r`, `r_r` — the auditor secret and the openings of the two commitments.

### Contract Flow

`clawback(account, amount, destination, data, operator)` runs, after the deployment's access-control check on `operator`:

1. **Preconditions.** `is_frozen(account)`, else `AccountNotFrozen`; `amount > 0`, else `InvalidClawbackAmount`; `destination` is not `Some` naming this contract, else `InvalidClawbackDestination` — such a seize is a `None` in effect while reporting a settlement, which breaks per-branch pool reconciliation for any indexer.
2. **Public inputs.** Assembled in the [Circuit](#circuit) order through `append_point` / `append_field` / `append_amount`, so every coordinate and scalar passes the canonicality guards. `data` decodes to `ClawbackData { proof }`.
3. **Verification** against `CircuitType::Clawback`.
4. **State update.** `C_spend <- C_spend + C_receive - alpha·G` and `C_receive <- O`: the `Merge` rule ([Merge](protocol/operations/merge.md)) followed by a public debit, with no fresh randomness. The new opening is `(v_s + v_r - alpha, r_s + r_r)`, which the owner and the auditor recompute from the event's `amount` alone, so the seized account stays spendable. Re-randomizing under a prover-chosen blinding instead would leave the owner unable to open its own commitment.
5. **Nonce.** `ClawbackNonce(account) <- nonce + 1`, where `nonce` is the value step 2 read into the public inputs. The entry is absent until the first seizure and reads as `0`; `clawback_nonce(account)` exposes it ([Interface Summary](#interface-summary)).
6. **Settlement**, by `destination`:
   - `None` — no underlying moves.
   - `Some(d)` — exactly `amount` is transferred to `d` in the same invocation. `d` passes no compliance gate ([Contract-Level Freeze](#contract-level-freeze), [Policy Contract](#policy-contract)): the auditor chooses it when building the proof ([Circuit](#circuit)) and the operator confirms it as an argument.
7. **Event.** `Clawback { account, amount, destination }` ([Events](#events)).

**Extraction order.** The seize and the issuer's extraction are separate invocations by different parties, and Soroban admits one per transaction, so the pool sits mismatched for at least a ledger between them. Nothing in the contract enforces which comes first; under a `None` settlement the extraction must follow the seize:

- **Seize, then extract.** Between the two the pool holds `amount` more than the claims against it. No holder is affected, and the extraction returns it to exact collateralization.
- **Extract, then seize.** Between the two the pool holds `amount` less than the claims against it, so it cannot honour them all: the shortfall lands on whichever holders withdraw last, none of whom is the target. It becomes permanent if the seize then turns out to be unbuildable — the auditor produces no proof ([Roles and Separation](#roles-and-separation)).

The confidential debit strictly precedes the SEP-41 transfer, matching `withdraw`. Clawback is the one operation that reduces the sum of claims without a `Withdraw` ([Balance Conservation](protocol/security.md#balance-conservation)), and it invokes no `Hooks` callback: the freeze gate would reject exactly the account it acts on ([Interface](protocol/interface.md)).

### Forced Revocation

Escrowed allowances are invisible to `clawback`, which sees only `C_spend` and `C_receive`. `force_revoke_spender(account, spender, operator)` moves them into reach: with `account` frozen (`AccountNotFrozen` otherwise), it performs the owner's `revoke_spender` fold ([Revoke Spender](protocol/operations/revoke-spender.md)) under the admin gate in place of the owner's authorization and emits the same `RevokeSpender` event, carrying `a_tilde` and `allowance_salt`. No proof is involved, and expired delegations are revocable: expiry blocks spending but not reclamation. Like `clawback`, it invokes no `Hooks` callback.

The owner and the auditor fold the event as they would an owner-initiated one ([Wallet State and Recovery](protocol/wallet-state.md); [Spender Allowance Auditing](protocol/auditing.md#spender-allowance-auditing)). For the auditor the archive dependence of [Spender Allowance Auditing](protocol/auditing.md#spender-allowance-auditing) applies unchanged: `r_a` is not derivable from `a_tilde` and `σ_a`, so its standing opening of `C_spend` survives the fold only if it observed the delegation's last `SetSpender` / `SpenderTransfer` event.

### Anti-Replay and the Freeze

`C_spend` and `C_receive` are public inputs, so any change to either between proof construction and submission — an inbound transfer, a merge, a revoke — fails verification with `InvalidProof`. The freeze holds the commitments still. An auditor key rotation in the same window fails the same way ([In-flight proofs across rotation](protocol/auditing.md#in-flight-proofs-across-rotation)).

Commitment equality is not freshness. A seizure of `alpha` from `(C, O)` leaves `(C - alpha·G, O)`, and a zero-blinding credit of `alpha` restores `(C, O)` exactly — a public deposit followed by a merge once the account is unfrozen, or a `force_revoke_spender` fold of an allowance committed as `Com(alpha, 0)` while it is still frozen — with every other public input unchanged. The `nonce` input advances on every seizure ([Contract Flow](#contract-flow)) and never returns to an earlier value, so the retained proof cannot execute a second seizure the auditor did not approve.

`ConfidentialClawback: ConfidentialCompliance` forces a `freeze` / `unfreeze` implementation but constrains nothing about the deployment's `Hooks`. Wiring `NoHooks` next to a `ConfidentialClawback` impl yields a `freeze` that writes the flag and an `is_frozen` that returns `true` while every token operation stays ungated, so the target spends out before the seizure lands and the admin's only signal is an `InvalidProof` once the commitments have moved. A deployment that enables clawback MUST wire `ComplianceHooks`, or a custom `Hooks` impl that gates the same seven positions ([Customizing the Hooks Trait](#customizing-the-hooks-trait)).

### Wallet and Auditor Consequences

`Clawback` is a receiving-side reset and a `T_0` anchor, alongside `Merge` ([Recovery](protocol/wallet-state.md#recovery); [Terminology](indexer.md#terminology)). It is not a checkpoint: it carries no `b_tilde`, so its effect on the spendable side is absorbed into the next owner-initiated proof operation. The owner applies the `Merge` row of [Wallet State and Recovery](protocol/wallet-state.md) followed by `W_spend.v -= amount`; the auditor advances its standing opening identically ([Sender-auditor opening capability](protocol/auditing.md#sender-auditor-opening-capability)). A `RevokeSpender` emitted by `force_revoke_spender` is indistinguishable from an owner-initiated one for both consumers.

---

## RWA Profile

The RWA profile is the configuration for regulated assets whose issuer must see every balance and may have to enforce rules on them. It is delivered as `RwaHooks`, a `Hooks` implementation a deployment wires instead of `ComplianceHooks`, plus the `ConfidentialRwa` trait and the opt-in `ConfidentialForcedTransfer` trait. It adds no circuit and changes no core entry point. [rwa.md](rwa.md) is the integrator guide: deployment models, the approver service, and the mapping to ERC-7984, ERC-8424 and ERC-3643.

The profile splits compliance by what the chain can see. Rules over public addresses (eligibility, freeze) run on-chain. Rules over hidden balances and amounts are evaluated off-chain by the issuer's compliance service, which holds the designated auditor key and therefore decrypts every operation; it either monitors and remediates, or, when configured as the approver, authorizes each value-moving operation before it settles.

```rust
struct RwaConfig {
    auditor_id: u32,           // Designated Auditor
    approver: Option<Address>, // Approval Gate
}
```

`RwaConfig` is stored under its own key; `ComplianceConfig` is unchanged. `RwaHooks` differs from `ComplianceHooks` as follows:

- **Fails closed.** Every callback reverts with `RwaNotConfigured` without an `RwaConfig`, with `NotConfigured` without a `ComplianceConfig`, and with `EligibilityRequired` when the `ComplianceConfig` has neither a policy nor SAC passthrough. While an `RwaConfig` exists, `set_compliance_config` rejects a configuration with neither.
- **Spender.** The spender of a delegated flow passes the freeze, policy and SAC gates, not the policy gate alone; under SAC passthrough it must therefore be an authorized holder of the base asset, which registration already requires. `revoke_spender` still gates only the owner.
- **Deposits.** The recipient must be a registered account.

### Designated Auditor

The compliance service can monitor, approve and seize only what it can decrypt, so every account of an RWA deployment is bound to one auditor. `RwaConfig.auditor_id` is fixed by the first `set_rwa_config` call, which requires the id to resolve in the auditor registry; a later call with another id reverts with `AuditorAlreadySet`. Auditor keys under that id still rotate ([Auditor Key Management and Rotation](protocol/auditing.md#auditor-key-management-and-rotation)).

`register` reverts with `AuditorNotAllowed` for any other id. The confidential accounts named by `deposit` (`to`), `confidential_transfer` and `confidential_transfer_from` (`from`, `to`), `set_spender` (`account`) and `forced_transfer` (`to`) must be bound to the designated auditor, else `AuditorMismatch`. The spender's binding is not checked.

`merge`, `revoke_spender` and `withdraw` carry no such check. An account bound to another auditor, which can exist only where a deployment adopted the profile after accounts had registered, can therefore fold its balances and leave publicly, and can be seized against a proof from the auditor it is bound to; it cannot send, receive or delegate. Such deployments should drain those accounts before adopting the profile.

### Approval Gate

When `RwaConfig.approver` is set, `deposit`, `withdraw`, `confidential_transfer` and `confidential_transfer_from` additionally require the approver's authorization. The hook calls `approver.require_auth_for_args(args)` inside the token's own invocation, so the host binds the authorization to the network, to this contract and to the entry point being executed; `args` bind the rest:

| Entry point | `args` |
|:---|:---|
| `deposit` | `(epoch, from, to, amount)` |
| `withdraw` | `(epoch, from, to, amount, payload)` |
| `confidential_transfer` | `(epoch, from, to, payload)` |
| `confidential_transfer_from` | `(epoch, spender, from, to, payload)` |

`payload` is the operation's decoded payload ([Interface](protocol/interface.md)), so an authorization covers one statement: any valid proof of it is acceptable, and no other payload is. The approver obtains the amount of a transfer from the recipient-auditor ciphertexts in that payload ([Per-Transfer Auditor Ciphertexts](protocol/auditing.md#per-transfer-auditor-ciphertexts)).

The approval is additional. The operation's own authorizer, its proof and every other gate are still required, and an approver cannot move, create or destroy value. `set_spender`, `merge` and `revoke_spender` change no ownership and require no approval. `clawback` and `forced_transfer` are administrative overrides and require none either.

**Epoch.** `epoch` is a contract-managed `u32` that only increases: `set_rwa_config`, `bump_approval_epoch`, and `set_compliance_config` while an `RwaConfig` exists each advance it, and it reverts with `ApprovalEpochOverflow` instead of wrapping. An authorization names the epoch it was issued for, so removing or replacing the approver, swapping the policy, or an explicit bump voids every outstanding one; disabling and re-enabling the approver does not return to an earlier epoch.

**Who the approver may be.** The approver must not be a party to the operation it approves (`from`, `to`, or `spender` for a delegated transfer): the hook reverts with `ApproverIsParty`. A party approving its own operation gives no independent check, and an approver that also authorized the invocation would authorize it twice. `set_rwa_config` rejects this contract's own address with `InvalidApprover`. Three host behaviours bound the guarantee above:

- A detached address credential binds `args`, a nonce and an expiry ledger. This is the form an approver service should issue.
- A transaction whose source account is the approver satisfies the gate through that account's signature on the envelope. The entry still carries `args`, but has no expiry of its own; such envelopes need time bounds.
- A contract approver that itself invokes the token is authorized without `args` being matched and without its `__check_auth` running. Any path through which that contract calls the token must itself check the token address, the expected epoch, the entry point and the full `args`; a generic forwarding function, even one gated by the contract's own authorization, carries no epoch and is not voided by a bump.

### Forced Transfer

`forced_transfer(from, to, amount, data, operator)` moves a public `amount` from a frozen account to another registered account inside the pool. It is `clawback` with a confidential destination: the same proof statement, verified against the same key, with no underlying moved. Recovery of a lost account is a forced transfer of its full balance to a fresh account.

The proof is a clawback proof ([Circuit](#circuit)) built by the auditor `from` is bound to. Its public inputs are those of [Contract Flow](#contract-flow) step 2, in the same order and from the same sources, except the destination input:

| Entry point | `dest_f` |
|:---|:---|
| `clawback`, `Some(d)` | `address_to_field(d)` |
| `clawback`, `None` | the zero field |
| `forced_transfer` | `Poseidon2(18, address_to_field(to))` ([Domain Separation Constants](protocol/domain-separators.md)) |

The circuit references `dest_f` in no gate, so the tag is what assigns a proof to one settlement mode: a proof built for a forced transfer verifies in neither clawback branch, and a clawback proof does not verify as a forced transfer. A `to` whose `dest_f` is zero is rejected, keeping the third row disjoint from the second. Both entry points read and advance the same `ClawbackNonce(from)` ([Anti-Replay and the Freeze](#anti-replay-and-the-freeze)), so a proof executes at most once across both.

`forced_transfer` runs, after the deployment's access-control check on `operator`:

1. **Preconditions**, in this order. Both configurations valid, as for `RwaHooks`. `is_frozen(from)`, else `AccountNotFrozen`. `amount > 0`, else `InvalidForcedTransferAmount`. `to` is neither `from` nor this contract and its `dest_f` is non-zero, else `InvalidForcedTransferDestination`. `from` is registered. `to` passes the freeze, policy and SAC gates. `to` is registered and bound to the designated auditor, else `AuditorMismatch`. `from` passes no eligibility gate and may be bound to any auditor.
2. **Verification** against `CircuitType::Clawback`.
3. **State update.** `from.C_spend <- C_spend + C_receive - amount·G`, `from.C_receive <- O`, then `to.C_receive <- to.C_receive + amount·G`. The sum of all commitments is unchanged and the pool's balance is untouched.
4. **Nonce.** `ClawbackNonce(from) <- nonce + 1`, reverting with `NonceOverflow` instead of wrapping.
5. **Event.** `ForcedTransfer { from, to, amount, nonce }`, where `nonce` is the value step 4 consumed.

No `Hooks` callback is invoked. For `from`, a `ForcedTransfer` is what a `Clawback` is: a receiving-side reset and a `T_0` anchor, not a checkpoint, and the source owner and its auditor apply the rule of [Wallet and Auditor Consequences](#wallet-and-auditor-consequences); the destination owner and its auditor apply the `Deposit` rule, the credit carrying zero blinding ([Wallet State and Recovery](protocol/wallet-state.md)).

**Recovery procedure.** Freeze the lost account; `force_revoke_spender` each of its delegations, including expired ones, which the contract cannot enumerate and the operator takes from the event archive; then force-transfer the full balance. The circuit proves `amount <= v_spend + v_receive`, so a smaller amount leaves a remainder a further forced transfer can move.

**Restrictions.** A deployment that implements `ConfidentialForcedTransfer` MUST wire `RwaHooks`, for the reason given in [Anti-Replay and the Freeze](#anti-replay-and-the-freeze). An issued proof stays executable by the operator until the source's commitments, its nonce or the auditor key change. The amount is public. The proof's witness includes the auditor's secret key, and the verifier accepts only non-zero-knowledge proofs today (see the package `README.md`): neither `forced_transfer` nor `clawback` should be used on a public network until it accepts zero-knowledge ones, and nothing on-chain enforces that.

---

## Interface Summary

```rust
impl Token {
    // Core arguments as declared in protocol/interface.md, plus the compliance entry.
    fn __constructor(e: Env, admin: Address, token: Address, verifier: Address,
                     auditor: Address, compliance: Option<ComplianceConfig>);

    // Freeze (Contract-Level Freeze)
    fn freeze(e: Env, account: Address, operator: Address);
    fn unfreeze(e: Env, account: Address, operator: Address);
    fn is_frozen(e: Env, account: Address) -> bool;

    // Config rotation (admin auth per Admin Authority, reverts when compliance.is_none())
    // Replaces the entire ComplianceConfig in one call.
    fn set_compliance_config(e: Env, config: ComplianceConfig, operator: Address);

    // Reads
    fn compliance_config(e: Env) -> Option<ComplianceConfig>;

    // Clawback, opt-in via ConfidentialClawback
    fn clawback(e: Env, account: Address, amount: i128, destination: Option<Address>,
                data: Bytes /* XDR ClawbackData { proof } */, operator: Address);
    fn force_revoke_spender(e: Env, account: Address, spender: Address, operator: Address);
    fn clawback_nonce(e: Env, account: Address) -> u32;

    // RWA profile, via ConfidentialRwa (admin auth per Admin Authority)
    fn set_rwa_config(e: Env, auditor_id: u32, approver: Option<Address>, operator: Address);
    fn bump_approval_epoch(e: Env, operator: Address);
    fn rwa_config(e: Env) -> Option<RwaConfig>;
    fn approval_epoch(e: Env) -> u32;
    fn can_send(e: Env, account: Address) -> bool;
    fn can_receive(e: Env, account: Address) -> bool;

    // Forced transfer, opt-in via ConfidentialForcedTransfer
    fn forced_transfer(e: Env, from: Address, to: Address, amount: i128,
                       data: Bytes /* XDR ClawbackData { proof } */, operator: Address);
}
```

`can_send` and `can_receive` are the same predicate: both configurations are set with an eligibility gate, and `account` is registered, bound to the designated auditor, not frozen, and passes the policy and SAC gates. They describe public eligibility of a confidential account, not whether an operation will settle, and they return `false` instead of raising a contract error, including when the policy or SAC call fails recoverably.

`operator` is the address whose authorization the deployment's access-control module checks ([Admin Authority](#admin-authority)).

### Events

| Event | Fields |
|:---|:---|
| `Frozen`, `Unfrozen` | `account` |
| `ComplianceConfigChanged` | `policy`, `sac_passthrough` |
| `Clawback` | `account` (topic), `amount`, `destination` |
| `RevokeSpender` | as [Event Schema](protocol/interface.md#event-schema), when emitted by `force_revoke_spender` |
| `RwaConfigChanged` | `auditor_id`, `approver`, `epoch` |
| `ApprovalEpochBumped` | `epoch`; also emitted after `ComplianceConfigChanged` while an `RwaConfig` exists |
| `ForcedTransfer` | `from` (topic), `to` (topic), `amount`, `nonce` |

### Errors

The seizure entry points add three variants to `ComplianceError`:

| Error | Code | Raised when |
|:---|:--|:---|
| `AccountNotFrozen` | 3604 | the target of `clawback` or `force_revoke_spender` is not frozen |
| `InvalidClawbackAmount` | 3605 | `amount <= 0` |
| `InvalidClawbackDestination` | 3606 | `destination` is `Some` naming the contract's own address |

The RWA profile adds:

| Error | Code | Raised when |
|:---|:--|:---|
| `RwaNotConfigured` | 3607 | `RwaHooks` or an RWA entry point runs without an `RwaConfig` |
| `AuditorNotAllowed` | 3608 | `register` names an auditor other than the designated one |
| `AuditorMismatch` | 3609 | an operation names a confidential account bound to another auditor |
| `AuditorAlreadySet` | 3610 | `set_rwa_config` names an auditor other than the stored one |
| `InvalidApprover` | 3611 | the approver is the contract's own address |
| `ApproverIsParty` | 3612 | the approver is `from`, `to` or `spender` of the operation |
| `InvalidForcedTransferAmount` | 3613 | `amount <= 0` |
| `InvalidForcedTransferDestination` | 3614 | `to` is `from` or the contract's own address, or its `dest_f` is zero |
| `EligibilityRequired` | 3615 | the compliance configuration of an RWA deployment has neither a policy nor SAC passthrough |
| `ApprovalEpochOverflow` | 3616 | the approval epoch is `u32::MAX` |
| `NonceOverflow` | 3617 | the clawback nonce is `u32::MAX` in `forced_transfer` |

---

Up: [Index](README.md#companions)
