# RWA (Real World Asset) Token Examples

This guide demonstrates how to deploy a complete RWA token system on Stellar testnet. More information about RWAs can be found in the dedicated [library module](../../packages/tokens/src/rwa) and in the official OpenZeppelin [docs](https://docs.openzeppelin.com/stellar-contracts/tokens/rwa/rwa). 

## Architecture Overview

The RWA system consists of 7 contracts that work together:

| Directory                   | Contract                        | Description                                                         |
| --------------------------- | ------------------------------- | ------------------------------------------------------------------- |
| `claim-topics-and-issuers/` | `ClaimTopicsAndIssuersContract` | Registry of claim topics (e.g., KYC, AML) and their trusted issuers |
| `claim-issuer/`             | `ClaimIssuerContract`           | Validates Ed25519-signed identity claims                            |
| `identity/`                 | `IdentityContract`              | Per-user contract storing signed identity claims                    |
| `identity-registry/`        | `IdentityRegistryContract`      | Maps wallet addresses to identity contracts with country data       |
| `identity-verifier/`        | `IdentityVerifierContract`      | Orchestrates identity verification across the entire identity stack |
| `compliance/`               | `ComplianceContract`            | Hook-based compliance framework with pluggable modules              |
| `token/`                    | `RWATokenContract`              | The RWA security token with freezing, recovery, and compliance      |

## Step-by-Step Deployment Guide

### 1. Setup

#### Clone and Build

```bash
git clone https://github.com/OpenZeppelin/stellar-contracts.git
cd stellar-contracts
stellar contract build
```

#### Configure Network

```bash
stellar network use testnet
```

#### Generate and Fund Keypairs

```bash
stellar keys generate feepayer
stellar keys fund feepayer
stellar keys use feepayer

stellar keys generate admin
stellar keys fund admin

stellar keys generate alice
stellar keys fund alice

stellar keys generate bob
stellar keys fund bob
```

> **Note:** In this guide, the same address is used as both `admin` and `manager` for simplicity. In production, these should be separate addresses with distinct privileges. Furthermore, the same `admin` is used across all contracts, while in production distinct admins and managers might be set for the different contracts.

### 2. Deploy Claim Topics and Issuers Contract

This contract manages the registry of which claim topics (e.g., KYC=1, AML=2) are required and which issuers are trusted to sign them. Deploy it first since other contracts reference it.

```bash
stellar contract deploy --alias claim-topics-and-issuers \
    --wasm target/wasm32v1-none/release/rwa_claim_topics_and_issuers_example.wasm \
    -- \
    --admin admin \
    --manager admin
```

Store the contract address:

```bash
export CTI_ADDRESS=$(stellar contract alias show claim-topics-and-issuers 2>&1 | tail -n 1)
```

#### Configure Claim Topics

Register the claim topics that investors must satisfy. Topic IDs are arbitrary `u32` values; a common convention is `1` for KYC and `2` for AML:

```bash
stellar contract invoke --id claim-topics-and-issuers --source admin \
    -- add_claim_topic --claim_topic 1 --operator admin

stellar contract invoke --id claim-topics-and-issuers --source admin \
    -- add_claim_topic --claim_topic 2 --operator admin
```

### 3. Deploy Claim Issuer Contract

The claim issuer validates identity claims using Ed25519 signatures. Each claim issuer manages a set of authorized signing keys. You may deploy multiple claim issuers for different identity providers.

```bash
stellar contract deploy --alias claim-issuer \
    --wasm target/wasm32v1-none/release/rwa_claim_issuer_example.wasm \
    -- \
    --owner admin
```

```bash
export CLAIM_ISSUER_ADDRESS=$(stellar contract alias show claim-issuer 2>&1 | tail -n 1)
```

#### Register Claim Issuer as Trusted

Register the claim issuer as trusted in the Claim Topics and Issuers registry, specifying which claim topics it is authorized to sign:

```bash
stellar contract invoke --id claim-topics-and-issuers --source admin \
    -- add_trusted_issuer \
    --trusted_issuer $CLAIM_ISSUER_ADDRESS \
    --claim_topics '[1, 2]' \
    --operator admin
```

#### Register Signing Keys

Authorize an Ed25519 public key to sign claims for specific topics. The `registry` parameter points to the Claim Topics and Issuers contract.

The Ed25519 key used in this example is:

| Secret Key                                                         | Public Key                                                         |
| ------------------------------------------------------------------ | ------------------------------------------------------------------ |
| `0000000000000000000000000000000000000000000000000000000000000000` | `3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29` |

> **⚠️ Warning:** This key is publicly viewable and not random. **Do not use it for any purpose beyond this example.** Generate your own keys and update them in the commands below when executing.

```bash
stellar contract invoke --source admin --id claim-issuer \
    -- allow_key \
    --public_key 3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29 \
    --registry $CTI_ADDRESS \
    --claim_topic 1
```

Repeat for each topic the key should be authorized for.

### 4. Deploy Identity Contracts

Each investor needs an identity contract that stores their signed claims. The identity owner (typically the investor or a custodian) controls claim management.

```bash
stellar contract deploy --alias identity-alice \
    --wasm target/wasm32v1-none/release/rwa_identity_example.wasm \
    -- \
    --owner alice
```

```bash
export ALICE_IDENTITY=$(stellar contract alias show identity-alice 2>&1 | tail -n 1)
```

#### Add Claims to an Identity

The `--signature` and `--data` values embed the deployed contract addresses and must
be computed after deployment. Build and run the `rwa-sign-claim` tool from this repo for each of the claim topics:

```bash
cargo run --manifest-path examples/rwa/sign-claim/Cargo.toml -- \
    --secret-key 0000000000000000000000000000000000000000000000000000000000000000 \
    --claim-issuer $CLAIM_ISSUER_ADDRESS \
    --identity $ALICE_IDENTITY \
    --claim-topic 1
```

> **⚠️ Warning:** The secret key above is publicly known. Replace it with your own key and update `allow_key` accordingly.

Copy the printed `--data` and `--signature` values into the invocation:

```bash
stellar contract invoke --id identity-alice --source alice \
    -- add_claim \
    --topic 1 \
    --scheme 101 \
    --issuer $CLAIM_ISSUER_ADDRESS \
    --signature <paste --signature value> \
    --data <paste --data value> \
    --uri "https://example.com/claim/alice-kyc"
```

Repeat for claim topic "2":

```bash
cargo run --manifest-path examples/rwa/sign-claim/Cargo.toml -- \
    --secret-key 0000000000000000000000000000000000000000000000000000000000000000 \
    --claim-issuer $CLAIM_ISSUER_ADDRESS \
    --identity $ALICE_IDENTITY \
    --claim-topic 2
```

Copy the printed `--data` and `--signature` values into the invocation:

```bash
stellar contract invoke --id identity-alice --source alice \
    -- add_claim \
    --topic 2 \
    --scheme 101 \
    --issuer $CLAIM_ISSUER_ADDRESS \
    --signature <paste --signature value> \
    --data <paste --data value> \
    --uri "https://example.com/claim/alice-aml"
```


### 5. Deploy Identity Registry Storage Contract

This contract maps wallet addresses to their identity contracts and stores country/jurisdiction data.

```bash
stellar contract deploy --alias identity-registry \
    --wasm target/wasm32v1-none/release/rwa_identity_registry_example.wasm \
    -- \
    --admin admin \
    --manager admin
```

```bash
export REGISTRY_ADDRESS=$(stellar contract alias show identity-registry 2>&1 | tail -n 1)
```

#### 5.1. Register Identities

Link each investor's wallet address to their identity contract:

```bash
stellar contract invoke --id identity-registry --source admin \
    -- add_identity \
    --account alice \
    --identity $ALICE_IDENTITY \
    --initial_profiles '[{"country": {"Individual": {"Residence": 840}}, "metadata": null}]' \
    --operator admin
```

`CountryData` uses a nested enum structure. The outer `CountryRelation` variant is either `Individual` or `Organization`, and the inner variant is the relationship type (e.g., `Residence`, `Citizenship`, `Incorporation`) wrapping an ISO 3166-1 numeric country code (e.g., 840 = United States).

Examples of valid `CountryData` JSON values:

```json
{"country": {"Individual": {"Residence": 840}},    "metadata": null}
{"country": {"Individual": {"Citizenship": 276}},  "metadata": null}
{"country": {"Organization": {"Incorporation": 840}}, "metadata": null}
{"country": {"Organization": {"OperatingJurisdiction": 276}}, "metadata": null}
```

### 6. Deploy Identity Verifier Contract

The identity verifier orchestrates the verification process by connecting the identity registry and claim topics registry. Both dependencies are wired at deploy time:

```bash
stellar contract deploy --alias identity-verifier \
    --wasm target/wasm32v1-none/release/rwa_identity_verifier_example.wasm \
    -- \
    --admin admin \
    --manager admin \
    --identity_registry_storage $REGISTRY_ADDRESS \
    --claim_topics_and_issuers $CTI_ADDRESS
```

```bash
export VERIFIER_ADDRESS=$(stellar contract alias show identity-verifier 2>&1 | tail -n 1)
```

### 7. Deploy Compliance Contract

The compliance contract enforces business rules through pluggable modules registered on specific hooks.

```bash
stellar contract deploy --alias compliance \
    --wasm target/wasm32v1-none/release/rwa_compliance_example.wasm \
    -- \
    --admin admin \
    --manager admin
```

```bash
export COMPLIANCE_ADDRESS=$(stellar contract alias show compliance 2>&1 | tail -n 1)
```

#### Register Compliance Modules (Optional)

If you have custom compliance module contracts (e.g., transfer limits, country restrictions), register them on the appropriate hooks:

```bash
stellar contract invoke --id compliance --source admin \
    -- add_module_to \
    --hook CanTransfer \
    --module <MODULE_ADDRESS> \
    --operator admin
```

Available hooks: `CanTransfer`, `CanCreate`, `Transferred`, `Created`, `Destroyed`.

> **Note:** Without any registered modules, the compliance contract allows all operations by default.

### 8. Deploy RWA Token Contract

Finally, deploy the token itself:

```bash
stellar contract deploy --alias rwa-token \
    --wasm target/wasm32v1-none/release/rwa_token_example.wasm \
    -- \
    --name "Acme Real Estate Token" \
    --symbol "ACRE" \
    --admin admin \
    --manager admin \
    --compliance $COMPLIANCE_ADDRESS \
    --identity_verifier $VERIFIER_ADDRESS
```

```bash
export TOKEN_ADDRESS=$(stellar contract alias show rwa-token 2>&1 | tail -n 1)
```

#### Bind the Token to Periphery Contracts

The Identity Registry and Compliance contracts need to know which tokens they serve:

```bash
stellar contract invoke --id identity-registry --source admin \
    -- bind_token \
    --token $TOKEN_ADDRESS \
    --operator admin

stellar contract invoke --id compliance --source admin \
    -- bind_token \
    --token $TOKEN_ADDRESS \
    --operator admin
```

### 9. Mint Tokens

With everything configured, mint tokens to verified investors:

```bash
stellar contract invoke --id rwa-token --source admin \
    -- mint \
    --to alice \
    --amount 1000000000 \
    --operator admin
```

> **Note:** The example token uses 7 decimals, so the above mints 100 tokens (100 \* 10^7).

## Deployment Checklist

Use this checklist to verify your deployment is complete:

- [ ] Claim Topics and Issuers deployed, topics registered
- [ ] Claim Issuer(s) deployed, registered as trusted issuers, signing keys authorized
- [ ] Identity contracts deployed for each investor, claims added
- [ ] Identity Registry Storage deployed, identities registered
- [ ] Identity Verifier deployed, linked to Claim Topics and Issuers + Identity Registry Storage
- [ ] Compliance deployed, modules registered (if any)
- [ ] RWA Token deployed, linked to Compliance + Identity Verifier
- [ ] Token bound to Identity Registry and Compliance contracts
- [ ] Initial token supply minted to verified addresses

## Key Operations

### Transfer Tokens

Regular transfers require both sender and recipient to be verified. 

**IMPORTANT**: It's required for `bob` to undergo the steps 4 and 5.1.

```bash
stellar contract invoke --id rwa-token --source alice \
    -- transfer \
    --from alice \
    --to bob \
    --amount 500000000
```

### Freeze an Address

Frozen addresses cannot send or receive tokens:

```bash
stellar contract invoke --id rwa-token --source admin \
    -- set_address_frozen \
    --user_address alice \
    --freeze true \
    --operator admin
```

### Freeze Partial Tokens

Lock a specific amount of tokens while allowing the rest to be transferred:

```bash
stellar contract invoke --id rwa-token --source admin \
    -- freeze_partial_tokens \
    --user_address alice \
    --amount 200000000 \
    --operator admin
```

### Recover Tokens to a New Account

If an investor loses access to their wallet, an operator can recover their balance to a new account. The identity stack must first set up the recovery mapping via `recover_identity` on the Identity Registry:

```bash
stellar keys generate alice_2
stellar keys fund alice_2

# Step 1: Set up recovery on the identity registry
stellar contract invoke --id identity-registry --source admin \
    -- recover_identity \
    --old_account alice \
    --new_account alice_2 \
    --operator admin

# Step 2: Recover the token balance
stellar contract invoke --id rwa-token --source admin \
    -- recover_balance \
    --old_account alice \
    --new_account alice_2 \
    --operator admin
```

### Pause the Token

Emergency pause prevents all transfers (admin operations like forced transfers still work):

```bash
stellar contract invoke --id rwa-token --source admin \
    -- pause --caller admin
```
