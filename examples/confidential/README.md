# Confidential Token Examples

This guide deploys a complete compliant confidential token suite on Stellar testnet: the verifier and auditor registries, an allowlist policy, the compliant token, and the factory that deploys the last two. The library modules live in [`packages/confidential`](../../packages/confidential); start with its [README](../../packages/confidential/README.md) and the non-normative [overview](../../packages/confidential/docs/overview.md) for the design, and [`docs/compliance.md`](../../packages/confidential/docs/compliance.md) for the compliance extension.

## Architecture Overview

The suite consists of 5 contracts plus the underlying asset:

| Directory           | Contract                       | Description                                                                                                  |
| ------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| `verifier/`         | `ConfidentialVerifierContract` | Registry of UltraHonk verification keys, one per circuit; verifies the proof attached to a token operation   |
| `auditor/`          | `ConfidentialAuditorContract`  | Registry of Grumpkin auditor public keys, indexed by `auditor_id`                                            |
| `allowlist-policy/` | `AllowlistPolicyContract`      | `Policy` contract that authorizes only the accounts a `manager` has listed                                   |
| `compliant-token/`  | `CompliantTokenContract`       | Confidential token wired with `ComplianceHooks`: freeze, policy and SAC gates, config rotation, and clawback |
| `factory/`          | `ConfidentialFactoryContract`  | Permissionless factory that deploys the allowlist and the token at addresses bound to their constructor args |

```text
factory ──deploys──▶ allowlist-policy
   │                        ▲
   │                        │ is_authorized
   └──────deploys──▶ compliant-token ──verify_proof──────────▶ verifier
                            │  │
                            │  └─────get_key─────────────────▶ auditor
                            └──transfer / authorized─────────▶ underlying SAC
```

The verifier, the auditor, and the factory are deployed once per network and shared: every token passes their addresses at construction, and per-token binding is enforced inside the circuits, not by the registries. Each further token repeats only steps [6](#6-deploy-the-allowlist-and-the-token) to [8](#8-onboard-holders), plus [step 2](#2-create-the-underlying-asset) when it wraps a new asset.

## Step-by-Step Deployment Guide

All commands run from the repository root.

### 1. Setup

#### Prerequisites

- Rust with the `wasm32v1-none` target: `rustup target add wasm32v1-none`.
- [Stellar CLI](https://developers.stellar.org/docs/tools/cli) 25.2.0 or later (earlier versions cannot build soroban-sdk 28 contracts).

#### Clone and Build

```bash
git clone https://github.com/OpenZeppelin/stellar-contracts.git
cd stellar-contracts

for example in verifier auditor allowlist-policy compliant-token factory; do
    stellar contract build --package confidential-$example-example
done
```

The wasms land in `target/wasm32v1-none/release/confidential_*_example.wasm`.

#### Configure Network

```bash
stellar network use testnet
```

#### Generate and Fund Keypairs

```bash
stellar keys generate feepayer --fund
stellar keys use feepayer

stellar keys generate admin --fund
stellar keys generate issuer --fund
stellar keys generate alice --fund
stellar keys generate bob --fund
```

`issuer` issues the underlying asset; `alice` and `bob` are token holders.

> **Note:** In this guide, `admin` holds every privileged role on every contract: access-control admin, registry `manager`, allowlist `manager`, and the token's `freezer`, `config_manager`, and `clawback` roles. In production these should be separate accounts, and the registries' `manager` role in particular belongs behind a multisig and a timelock (see [step 3](#3-deploy-the-verifier)).

### 2. Create the Underlying Asset

The confidential token wraps an existing SEP-41 token and holds it on behalf of its users. This guide issues a classic asset and deploys its Stellar Asset Contract (SAC), which lets the token consult the issuer's trustline authorization through `sac_passthrough` ([SAC Authorization Passthrough](../../packages/confidential/docs/compliance.md#sac-authorization-passthrough)):

```bash
export ASSET=CONF:$(stellar keys address issuer)

stellar contract asset deploy --asset $ASSET --alias underlying
```

> **Note:** Any SEP-41 token that satisfies the [Underlying Token Assumptions](../../packages/confidential/docs/protocol/system-model.md#underlying-token-assumptions) (non-rebasing, no fee on transfer, deterministic revert) can back the token. `sac_passthrough` requires a SAC: over any other token, every gated operation traps on the missing `authorized` function.

### 3. Deploy the Verifier

The verifier stores one UltraHonk verification key (VK) per circuit and verifies the proof attached to every proof-carrying token operation. Its constructor sets the access-control admin and grants the `manager` role, which registers keys:

```bash
stellar contract deploy --alias verifier \
    --wasm target/wasm32v1-none/release/confidential_verifier_example.wasm \
    -- \
    --admin admin \
    --manager admin
```

#### Register the Verification Keys

Register the six pinned VKs from [`packages/confidential/circuits/vks`](../../packages/confidential/circuits/vks). Each circuit is identified by its `CircuitType` value:

| Circuit            | `CircuitType` | Token entry point                      |
| ------------------ | ------------- | -------------------------------------- |
| `register`         | `0`           | `register`                             |
| `withdraw`         | `1`           | `withdraw`                             |
| `transfer`         | `2`           | `confidential_transfer`                |
| `spender_transfer` | `3`           | `confidential_transfer_from`           |
| `set_spender`      | `4`           | `set_spender`                          |
| `clawback`         | `100`         | `clawback` (compliance extension only) |

The `--verification_key-file-path` form passes the file's raw bytes, which must be the packed binary `.vk.bin`, not the review-friendly `.vk.json`:

```bash
register_vk() {
    stellar contract invoke --id verifier --source admin \
        -- register_verification_key \
        --circuit_type "$2" \
        --verification_key-file-path "packages/confidential/circuits/vks/$1.vk.bin" \
        --operator admin
}

register_vk register 0
register_vk withdraw 1
register_vk transfer 2
register_vk spender_transfer 3
register_vk set_spender 4
register_vk clawback 100
```

Each circuit accepts exactly one registration. Verify a key round-trips:

```bash
stellar contract invoke --id verifier -- get_verification_key --circuit_type 0
```

> **⚠️ Warning:** The example also exposes `update_verification_key` behind the same `manager` role, purely for illustration. A wrong key makes its circuit accept forged proofs, so a real deployment should ship VKs immutably or put the update path behind a multisig and a timelock. See the verifier [module docs](../../packages/confidential/src/verifier/mod.rs).

Proofs are accepted only if they are generated from the same circuits with the pinned toolchain and `bb` flags listed in [`circuits/vks/README.md`](../../packages/confidential/circuits/vks/README.md#proving).

### 4. Deploy the Auditor

The auditor registry stores the Grumpkin public keys that every transfer, withdrawal, and spender operation encrypts an auditor ciphertext to. Each account commits to one `auditor_id` when it registers.

```bash
stellar contract deploy --alias auditor \
    --wasm target/wasm32v1-none/release/confidential_auditor_example.wasm \
    -- \
    --admin admin \
    --manager admin
```

#### Register an Auditor Key

A key is a 64-byte Grumpkin point, `be(x) || be(y)`, that is on the curve and is not the identity. The key used in this example is the Grumpkin generator `G`:

| Secret Key | Public Key                                                                                                                         |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `1`        | `083e7911d835097629f0067531fc15cafd79a89beecb39903f69572c636f4a5a1a7f5efaad7f315c25a918f30cc8d7333fccab7ad7c90f14de81bcc528f9935d` |

> **⚠️ Warning:** The secret key behind `G` is `1`, so anyone can decrypt every auditor ciphertext produced under this `auditor_id`. **Do not use it for any purpose beyond this example.** Generate the auditor's key pair with an auditor client ([Auditor Client](../../packages/confidential/docs/sdk/auditor-client.md)) and register its public key instead.

```bash
stellar contract invoke --id auditor --source admin \
    -- register_key \
    --auditor_id 0 \
    --point 083e7911d835097629f0067531fc15cafd79a89beecb39903f69572c636f4a5a1a7f5efaad7f315c25a918f30cc8d7333fccab7ad7c90f14de81bcc528f9935d \
    --operator admin
```

`rotate_key` replaces a registered key. A rotation invalidates every in-flight proof that committed to the old key ([Auditor Key Management and Rotation](../../packages/confidential/docs/protocol/auditing.md#auditor-key-management-and-rotation)).

### 5. Deploy the Factory

The factory deploys the allowlist and the token from wasms that are already uploaded on the network, at addresses derived from their constructor arguments. Only the factory can create in its own namespace and the salt hashes the constructor arguments, so nobody can occupy a predicted address with a different configuration. Deployment through it is permissionless.

Upload the two wasms and keep their hashes:

```bash
export TOKEN_WASM_HASH=$(stellar contract upload \
    --wasm target/wasm32v1-none/release/confidential_compliant_token_example.wasm)

export ALLOWLIST_WASM_HASH=$(stellar contract upload \
    --wasm target/wasm32v1-none/release/confidential_allowlist_policy_example.wasm)
```

Deploy the factory, pinning both hashes:

```bash
stellar contract deploy --alias confidential-factory \
    --wasm target/wasm32v1-none/release/confidential_factory_example.wasm \
    -- \
    --token_wasm_hash $TOKEN_WASM_HASH \
    --allowlist_wasm_hash $ALLOWLIST_WASM_HASH
```

### 6. Deploy the Allowlist and the Token

#### Predict the Addresses

Predict the allowlist address first, since the token's compliance configuration names it. `salt` lets one configuration have several deployments; use `0` for the first:

```bash
export ALLOWLIST_ADDRESS=$(stellar contract invoke --id confidential-factory \
    -- predict_allowlist_address \
    --admin admin \
    --manager admin \
    --salt 0 | tr -d '"')

export TOKEN_ADDRESS=$(stellar contract invoke --id confidential-factory \
    -- predict_token_address \
    --admin admin \
    --underlying_asset underlying \
    --verifier verifier \
    --auditor auditor \
    --compliance "{\"policy\": \"$ALLOWLIST_ADDRESS\", \"sac_passthrough\": true}" \
    --salt 0 | tr -d '"')
```

`ComplianceConfig` is passed as JSON. Contract and identity aliases resolve in plain `Address` arguments but not inside JSON, so the policy is given by address.

#### Deploy Both Contracts

`deploy_allowlist_and_token` deploys the allowlist, then the token with `compliance.policy` pointing at it:

```bash
stellar contract invoke --id confidential-factory \
    -- deploy_allowlist_and_token \
    --policy_admin admin \
    --policy_manager admin \
    --token_admin admin \
    --underlying_asset underlying \
    --verifier verifier \
    --auditor auditor \
    --sac_passthrough true \
    --salt 0
```

The call returns `[allowlist, token]`, equal to `$ALLOWLIST_ADDRESS` and `$TOKEN_ADDRESS`. Repeating it with the same arguments fails because both addresses are taken; change `salt` to deploy another token with the same configuration. Alias the new contracts:

```bash
stellar contract alias add allowlist --id $ALLOWLIST_ADDRESS
stellar contract alias add confidential-token --id $TOKEN_ADDRESS
```

Check the token's compliance configuration:

```bash
stellar contract invoke --id confidential-token -- compliance_config
```

The factory also exposes `deploy_allowlist` and `deploy_token` separately, for a token that shares an existing policy or runs without one (`"policy": null`).

#### Alternative: Deploy Without the Factory

The two contracts can also be deployed directly. Their addresses then derive from the deployer and a random salt instead of their constructor arguments:

```bash
stellar contract deploy --alias allowlist \
    --wasm target/wasm32v1-none/release/confidential_allowlist_policy_example.wasm \
    -- \
    --admin admin \
    --manager admin

export ALLOWLIST_ADDRESS=$(stellar contract alias show allowlist)

stellar contract deploy --alias confidential-token \
    --wasm target/wasm32v1-none/release/confidential_compliant_token_example.wasm \
    -- \
    --admin admin \
    --underlying_asset underlying \
    --verifier verifier \
    --auditor auditor \
    --compliance "{\"policy\": \"$ALLOWLIST_ADDRESS\", \"sac_passthrough\": true}"
```

### 7. Grant the Token Roles

The token's constructor sets the access-control admin but grants no role. Each compliance duty has its own role ([Admin Authority](../../packages/confidential/docs/compliance.md#admin-authority)):

| Role             | Entry points                          |
| ---------------- | ------------------------------------- |
| `freezer`        | `freeze`, `unfreeze`                  |
| `config_manager` | `set_compliance_config`               |
| `clawback`       | `clawback`, `force_revoke_spender`    |

```bash
stellar contract invoke --id confidential-token --source admin \
    -- grant_role --account admin --role freezer --caller admin

stellar contract invoke --id confidential-token --source admin \
    -- grant_role --account admin --role config_manager --caller admin

stellar contract invoke --id confidential-token --source admin \
    -- grant_role --account admin --role clawback --caller admin
```

### 8. Onboard Holders

With a policy and `sac_passthrough` configured, every gated operation checks each fund-holding account it names against three gates: not frozen, authorized by the policy, and authorized by the SAC. `register` runs the policy and SAC gates, and the spender of a delegated flow is checked against the policy only. A holder therefore needs an allowlist entry and a trustline before registering.

List the holder:

```bash
stellar contract invoke --id allowlist --source admin \
    -- allow --account alice --operator admin
```

Open a trustline to the underlying asset and fund it:

```bash
stellar tx new change-trust --source alice --line $ASSET

stellar contract invoke --id underlying --source issuer \
    -- mint --to alice --amount 1000000000
```

Without a trustline, the SAC's `authorized` view traps with `Error(Contract, #13)` ("trustline entry is missing for account"), so every gated operation naming that account fails.

Check both gates:

```bash
stellar contract invoke --id allowlist -- is_authorized --account alice --token confidential-token
stellar contract invoke --id underlying -- authorized --id alice
```

Repeat for `bob`.

## Deployment Checklist

- [ ] Underlying asset issued and its SAC deployed
- [ ] Verifier deployed, all six verification keys registered
- [ ] Auditor deployed, auditor key registered under the `auditor_id` holders will use
- [ ] Token and allowlist wasms uploaded, factory deployed with their hashes
- [ ] Allowlist and token deployed through the factory, token `compliance_config` names the allowlist
- [ ] `freezer`, `config_manager`, and `clawback` roles granted on the token
- [ ] Holders listed on the allowlist, trustlines opened, underlying funded

## Using the Token

Every entry point that consumes private state takes a `data: Bytes` argument: the XDR-encoded operation payload together with an UltraHonk proof over it. A wallet builds both from the holder's keys and local balance openings, so these operations go through a client that implements the [SDK specification](../../packages/confidential/docs/sdk/README.md) rather than through the CLI:

| Entry point                  | Proof              | Description                                                                 |
| ---------------------------- | ------------------ | --------------------------------------------------------------------------- |
| `register`                   | `Register`         | Creates the confidential account and binds it to an `auditor_id`            |
| `deposit`                    | none               | Moves underlying into the token and credits a registered account            |
| `merge`                      | none               | Folds received funds into the spendable balance                             |
| `confidential_transfer`      | `Transfer`         | Moves a hidden amount between registered accounts                           |
| `withdraw`                   | `Withdraw`         | Debits a hidden balance and pays out underlying                             |
| `set_spender`                | `SetSpender`       | Delegates a confidential allowance to a spender                             |
| `confidential_transfer_from` | `SpenderTransfer`  | Spends from a delegated allowance                                           |
| `revoke_spender`             | none               | Returns a delegated allowance to the owner's balance                        |

Each flow is described in [Main User Flows](../../packages/confidential/docs/overview.md#main-user-flows) and specified in [`docs/protocol/operations`](../../packages/confidential/docs/protocol/operations).

`deposit` needs no proof but requires `to` to be registered, so a holder's first step is always `register`. The depositor signs the underlying transfer:

```bash
stellar contract invoke --id confidential-token --source alice \
    -- deposit --from alice --to alice --amount 100000000
```

Balances are readable through `confidential_balance`, which returns the account's commitments rather than plaintext amounts.

## Compliance Operations

### Freeze an Account

A frozen account cannot send, receive, deposit, merge, withdraw, or manage its spenders:

```bash
stellar contract invoke --id confidential-token --source admin \
    -- freeze --account alice --operator admin

stellar contract invoke --id confidential-token -- is_frozen --account alice

stellar contract invoke --id confidential-token --source admin \
    -- unfreeze --account alice --operator admin
```

### Remove an Account From the Allowlist

The token consults the allowlist on every gated operation, so delisting takes effect immediately for every token whose policy points at this allowlist:

```bash
stellar contract invoke --id allowlist --source admin \
    -- disallow --account bob --operator admin
```

### Deauthorize at the Issuer

With `sac_passthrough`, the issuer's trustline deauthorization reaches the confidential layer without any action on the token. SAC deauthorization requires the issuer's `AUTH_REVOCABLE` flag:

```bash
stellar tx new set-options --source issuer --set-revocable

stellar contract invoke --id underlying --source issuer \
    -- set_authorized --id alice --authorize false
```

Pass `--authorize true` to restore the account.

### Rotate the Compliance Configuration

`set_compliance_config` replaces the policy and the `sac_passthrough` flag. For example, to drop both gates and keep only the freeze:

```bash
stellar contract invoke --id confidential-token --source admin \
    -- set_compliance_config \
    --config '{"policy": null, "sac_passthrough": false}' \
    --operator admin
```

### Claw Back From a Frozen Account

A clawback reduces a frozen account's committed balance by a public amount. The `clawback` role holder decides whether to seize; the auditor bound to the account produces the `Clawback` proof that fixes the amount and destination, so neither can act alone ([Clawback](../../packages/confidential/docs/compliance.md#clawback)):

```bash
stellar contract invoke --id confidential-token --source admin \
    -- freeze --account alice --operator admin

stellar contract invoke --id confidential-token --source admin \
    -- clawback \
    --account alice \
    --amount 50000000 \
    --destination bob \
    --data <hex-encoded ClawbackData from the auditor> \
    --operator admin
```

Omit `--destination` to seize without moving underlying; the issuer then extracts the surplus through the SAC ([Contract Flow](../../packages/confidential/docs/compliance.md#contract-flow)). Allowances the account delegated sit outside `clawback`'s reach until `force_revoke_spender` folds them back into its balance:

```bash
stellar contract invoke --id confidential-token --source admin \
    -- force_revoke_spender --account alice --spender bob --operator admin
```
