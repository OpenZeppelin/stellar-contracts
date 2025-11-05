# Example Contract Deployment Commands

Sample Stellar CLI deployment commands for all example contracts in this repository.
All examples target the testnet network and use mock parameter values.

You can create a testnet identity (e.g., `alice`) using the following command:

```bash
stellar keys generate alice --network testnet --fund
```

## Examples included:

- fungible_allowlist_example
- fungible_blocklist_example
- fungible_capped_example
- fungible_merkle_airdrop_example
- fungible_pausable_example
- fungible_vault_example
- merkle_voting_example
- multisig_account_example
- multisig_ed25519_verifier_example
- multisig_spending_limit_policy_example
- multisig_threshold_policy_example
- multisig_webauthn_verifier_example
- nft_access_control_example
- nft_consecutive_example
- nft_enumerable_example
- nft_royalties_example
- nft_sequential_minting_example
- ownable_example
- pausable_example
- sac_admin_generic_example
- sac_admin_wrapper_example
- upgrader_example
- upgradeable_v1_example
- upgradeable_v2_example

RWA examples are not yet included.

---

## FungibleAllowList

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/fungible_allowlist_example.wasm --source-account alice --network testnet -- --admin alice --manager bob --initial_supply "1000000"
```

## FungibleBlockList

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/fungible_blocklist_example.wasm --source-account alice --network testnet -- --admin alice --manager bob --initial_supply "1000000"
```

## FungibleCapped

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/fungible_capped_example.wasm --source-account alice --network testnet -- --owner alice --cap "1000000"
```

## FungibleMerkleAirdrop

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/fungible_merkle_airdrop_example.wasm --source-account alice --network testnet -- --root_hash 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef --token CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC --funding_amount 100000000 --funding_source alice
```

## FungiblePausable

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/fungible_pausable_example.wasm --source-account alice --network testnet -- --owner alice --initial_supply "1000000"
```

## FungibleVault

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/fungible_vault_example.wasm --source-account alice --network testnet -- --owner alice --asset CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC --decimals_offset 6
```

## MerkleVoting

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/merkle_voting_example.wasm --source-account alice --network testnet -- --root_hash 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

## MultisigAccount

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/multisig_account_example.wasm --source-account alice --network testnet -- --signers '[{"Delegated":"GCXWIES6GBSIFK3UXNUV2O2XGZSVOQR4VVWRC65UXVMXSYOPE6BCO27K"}]' --policies '{}'
```

## MultisigEd25519Verifier

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/multisig_ed25519_verifier_example.wasm --source-account alice --network testnet
```

## MultisigSpendingLimitPolicy

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/multisig_spending_limit_policy_example.wasm --source-account alice --network testnet
```

## MultisigThresholdPolicy

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/multisig_threshold_policy_example.wasm --source-account alice --network testnet
```

## MultisigWebauthnVerifier

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/multisig_webauthn_verifier_example.wasm --source-account alice --network testnet
```

## NFTAccessControl

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/nft_access_control_example.wasm --source-account alice --network testnet -- --admin alice
```

## NFTConsecutive

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/nft_consecutive_example.wasm --source-account alice --network testnet -- --owner alice
```

## NFTEnumerable

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/nft_enumerable_example.wasm --source-account alice --network testnet -- --owner alice
```

## NFTRoyalties

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/nft_royalties_example.wasm --source-account alice --network testnet -- --admin alice --manager bob
```

## NFTSequentialMinting

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/nft_sequential_minting_example.wasm --source-account alice --network testnet -- --owner alice
```

## Ownable

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/ownable_example.wasm --source-account alice --network testnet -- --owner alice
```

## Pausable

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/pausable_example.wasm --source-account alice --network testnet -- --owner alice
```

## SACAdminGeneric

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/sac_admin_generic_example.wasm --source-account alice --network testnet -- --sac alice --chief 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef --operator 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

## SACAdminWrapper

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/sac_admin_wrapper_example.wasm --source-account alice --network testnet -- --default_admin alice --manager bob --sac alice
```

## Upgrader

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/upgrader_example.wasm --source-account alice --network testnet -- --owner alice
```

## UpgradeableV1

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/upgradeable_v1_example.wasm --source-account alice --network testnet -- --admin alice
```

## UpgradeableV2

```bash
stellar contract deploy --wasm target/wasm32v1-none/release/upgradeable_v2_example.wasm --source-account alice --network testnet
```

---
