# Multisig Smart Account Example

This guide demonstrates how to deploy a multisig smart account on Stellar testnet with 3 eligible signers: 2 Ed25519 signers and 1 passkey signer. Any 2 of the 3 signers can authorize any invocation (2-of-3 multisig scheme).

For more information about smart accounts and their components, check:
- [OpenZeppelin Stellar Contracts Documentation](https://docs.openzeppelin.com/stellar-contracts/accounts/smart-account)
- [Smart Accounts Package README](../../packages/accounts/README.md)

## 1. Setup

### Clone the Repository

```bash
git clone https://github.com/OpenZeppelin/stellar-contracts.git
cd stellar-contracts
```

### Build WASM Binaries

Build all the WASM binaries (we'll be using only a few of them):

```bash
cargo build --target wasm32v1-none --release
```

### Configure Network

Set Stellar CLI to use testnet:

```bash
stellar network use testnet
```

### Generate and Fund Keypair

Generate and fund a keypair to pay for the deployments:

```bash
stellar keys generate feepayer
stellar keys fund feepayer
# Use as the default source account
stellar keys use feepayer
```

## 2. Verifier Contracts

We will have 3 signers: 2 Ed25519 signers and 1 passkey signer. For each signature type, we need to deploy a verifier contract.

**What are verifier contracts?** Verifier contracts are reusable, immutable contracts that serve as cryptographic oracles for signature validation. They verify signatures on behalf of smart accounts without holding any state. Multiple smart accounts can share the same verifier contract, reducing deployment costs.

### Deploy WebAuthn Verifier

Deploy a verifier contract for the passkey signer:
```
stellar contract deploy --alias webauthn_verifier \
    --wasm ./target/wasm32v1-none/release/multisig_webauthn_verifier_example.wasm
```

For this example, we assume the passkey verifier was deployed to:
```
CC74I4FO7N2VY547Q7HA4QFIBZTYVG76DJH32ICBKI2FUPAS6J6MJE66
```

### Deploy Ed25519 Verifier

Deploy an Ed25519 verifier contract:

```bash
stellar contract deploy --alias ed25519_verifier \
    --wasm ./target/wasm32v1-none/release/multisig_ed25519_verifier_example.wasm
```

For this example, we assume the Ed25519 verifier was deployed to:
```
CDLDYJWEZSM6IAI4HHPEZTTV65WX4OVN3RZD3U6LQKYAVIZTEK7XYAYT
```

> **Note:** These verifier contracts can be reused across multiple apps and smart accounts. They are immutable and don't hold any state, serving as verifying oracles.

## 3. Threshold Policy

Policies customize signers' behavior. If we were to deploy a 3-of-3 multisig, we wouldn't need any policy at all. However, since we want to authorize any 2 of the 3 eligible signers, we need a simple threshold policy.

Deploy the threshold policy contract:

```bash
stellar contract deploy --alias threshold_policy \
    --wasm ./target/wasm32v1-none/release/multisig_threshold_policy_example.wasm
```

For this example, we assume the policy was deployed to:
```
CA7IJLIHDBTE5S5EIMTIWRKKTSJP6KPH2VOU255CB2RNTWXQGYJRKKC3
```

> **Note:** Policy contracts, unlike verifiers, can be stateful (as in this case). Policies can also be reused across apps and smart accounts.

## 4. Signer Public Keys

### Ed25519 Keys

The Ed25519 keys used in this example are:

| Key | Secret Key | Public Key |
|-----|------------|------------|
| Key 1 | `0000000000000000000000000000000000000000000000000000000000000000` | `3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29` |
| Key 2 | `0000000000000000000000000000000000000000000000000000000000000001` | `4cb5abf6ad79fbf5abbccafcc269d85cd2651ed4b885b5869f241aedf0a5ba29` |

> **⚠️ Warning:** These keys are publicly viewable and not random. **Do not use these keys for any purpose beyond this example.** Generate your own keys and update them in the commands below when executing.

### Passkey Public Key

The generation of passkey public keys is more complex when using them in the WebAuthn protocol flow. Please, bear in mind that passkeys are bound to a particular domain so they can't be re-used across domains. For demonstration purposes, you can use a helper tool like [brozorec/smart-account-sign](https://github.com/brozorec/smart-account-sign) to assist with key generation.

> **⚠️ Disclaimer:** The tool mentioned above is provided for demonstration purposes only. We do not vouch for its security or recommend it for production use. Always conduct your own security audit before using third-party tools in production environments.

For this example, assume we've generated the following passkey public key and credential ID:
```
04beb2f6bb9f9b9406c46292957836aa6bddde48131ea0fabb33c0aa39f3c9a0641d2be6caf248c91de35f5113382a046e8fc946598d028d820b056d209019f47c

2b817f01f993e42a5093d2694d88e9d849e3cd0b5ec7da7c5ce270882b92b134
```

Note that those values are concatenated in the example below.

## 5. Deploy the Multisig Smart Account

With all the elements from the previous steps, we are now ready to deploy the 2-of-3 multisig smart account:

```bash
stellar contract deploy \
    --alias multisig-smart-account \
    --wasm target/wasm32v1-none/release/multisig_account_example.wasm \
    -- \
    --signers '[
        {
            "External": [
                "CDLDYJWEZSM6IAI4HHPEZTTV65WX4OVN3RZD3U6LQKYAVIZTEK7XYAYT",
                "3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29"
            ]
        },
        {
            "External": [
                "CDLDYJWEZSM6IAI4HHPEZTTV65WX4OVN3RZD3U6LQKYAVIZTEK7XYAYT",
                "4cb5abf6ad79fbf5abbccafcc269d85cd2651ed4b885b5869f241aedf0a5ba29"
            ]
        },
        {
            "External": [
                "CC74I4FO7N2VY547Q7HA4QFIBZTYVG76DJH32ICBKI2FUPAS6J6MJE66",
                "04beb2f6bb9f9b9406c46292957836aa6bddde48131ea0fabb33c0aa39f3c9a0641d2be6caf248c91de35f5113382a046e8fc946598d028d820b056d209019f47c2b817f01f993e42a5093d2694d88e9d849e3cd0b5ec7da7c5ce270882b92b134"
            ]
        }
    ]' \
    --policies '{"CA7IJLIHDBTE5S5EIMTIWRKKTSJP6KPH2VOU255CB2RNTWXQGYJRKKC3": {"map": [{"key": {"symbol": "threshold"}, "val": {"u32": 2}}]}}'
```

### Understanding the Deployment Parameters

**Signers:**
- Three `External` signers, each containing:
  - Verifier contract address (Ed25519 or WebAuthn)
  - Public key for signature verification

**Policies:**
- Threshold policy contract address: `CA7IJLIHDBTE5S5EIMTIWRKKTSJP6KPH2VOU255CB2RNTWXQGYJRKKC3`
- Configuration: `threshold: 2` (requires 2 out of 3 signers to authorize transactions)

This setup demonstrates the flexibility of smart accounts, combining different signature types while sharing verification logic through reusable verifier contracts.

## Next Steps

1. Review the [Smart Accounts documentation](https://docs.openzeppelin.com/stellar-contracts/accounts/smart-account) to learn more about:
- the other type of signers: [`Delegated`](https://docs.openzeppelin.com/stellar-contracts/accounts/signers-and-verifiers#delegated)
- [adding or removing signers](https://docs.openzeppelin.com/stellar-contracts/accounts/signers-and-verifiers#signer-management)
- [caveats](https://docs.openzeppelin.com/stellar-contracts/accounts/policies#caveats) of policy management and configurations 
2. Explore how to invoke functions using this multisig smart account. You can use the [brozorec/smart-account-sign](https://github.com/brozorec/smart-account-sign) tool for demonstration purposes.
   > **⚠️ Disclaimer:** This tool is provided for demonstration purposes only. We do not vouch for its security or recommend it for production use. Always conduct your own security audit before using third-party tools in production environments.
