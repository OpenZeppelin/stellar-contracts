# RWA (Real World Assets) Contracts

This directory contains a comprehensive implementation of Real World Asset (RWA) tokens following the T-REX standard, separated into individual deployable contracts to avoid WASM compilation conflicts.

## Architecture

The RWA system consists of 8 separate contracts:

### Core Contracts

1. **rwa-token** - Main RWA token contract with freezing, recovery, and compliance integration
2. **compliance** - Modular compliance framework that manages compliance modules and validates transfers
3. **identity-registry-storage** - Stores identity profiles and country data for compliance
4. **claim-topics-and-issuers** - Manages claim topics and trusted issuers for identity verification
5. **identity-claims** - Manages on-chain identity claims
6. **claim-issuer** - Validates cryptographic claims using Ed25519 signatures

### Compliance Modules

7. **transfer-limit-module** - Example compliance module enforcing transfer and minting limits
8. **country-restriction-module** - Example compliance module for country-based restrictions

## Building the Contracts

Each contract must be built separately to generate the WASM files needed for cross-contract calls.

### Build All Contracts

```bash
# From the rwa directory
cd examples/rwa

# Build core contracts
cd claim-issuer && cargo build --target wasm32-unknown-unknown --release && cd ..
cd claim-topics-and-issuers && cargo build --target wasm32-unknown-unknown --release && cd ..
cd compliance && cargo build --target wasm32-unknown-unknown --release && cd ..
cd identity-claims && cargo build --target wasm32-unknown-unknown --release && cd ..
cd identity-registry-storage && cargo build --target wasm32-unknown-unknown --release && cd ..
cd rwa-token && cargo build --target wasm32-unknown-unknown --release && cd ..

# Build compliance modules
cd compliance-modules/transfer-limit-module && cargo build --target wasm32-unknown-unknown --release && cd ../..
cd compliance-modules/country-restriction-module && cargo build --target wasm32-unknown-unknown --release && cd ../..
```

### Build Script

You can also use the provided build script:

```bash
./build.sh
```

The script handles both core contracts and compliance modules in their respective directories.

## Testing

### Individual Contract Tests

Each contract has its own test suite:

```bash
cd claim-issuer && cargo test && cd ..
cd claim-topics-and-issuers && cargo test && cd ..
cd compliance && cargo test && cd ..
cd identity-claims && cargo test && cd ..
cd identity-registry-storage && cargo test && cd ..
cd rwa-token && cargo test && cd ..
cd compliance-modules/transfer-limit-module && cargo test && cd ../..
cd compliance-modules/country-restriction-module && cargo test && cd ../..
```

### Integration Tests

The `integration-test` directory contains comprehensive tests that demonstrate how all contracts work together:

```bash
# First build all contracts (see above)
# Then run integration tests
cd integration-test && cargo test
```

## Contract Interactions

The contracts interact through cross-contract calls using the compiled WASM files:

1. **RWA Token** calls **Compliance** to validate transfers
2. **Compliance** calls **Compliance Modules** to enforce rules
3. **RWA Token** calls **Identity Registry Storage** to verify user identities
4. **RWA Token** calls **Claim Topics and Issuers** to check required claims
5. **Claim Topics and Issuers** references **Claim Issuer** for claim validation

## Usage Example

```rust
// Deploy all contracts
let claim_issuer_id = e.register(claim_issuer::WASM, (&admin,));
let compliance_id = e.register(compliance::WASM, (&admin,));
let rwa_token_id = e.register(rwa_token::WASM, (&admin, &compliance_officer, &recovery_agent, &name, &symbol, &decimals));

// Configure RWA token
let rwa_client = rwa_token::Client::new(&e, &rwa_token_id);
rwa_client.set_compliance(&compliance_id, &admin);

// Mint tokens (will check compliance)
rwa_client.mint(&user, &amount, &admin);
```

## Key Features

- **T-REX Compliance**: Full implementation of the T-REX standard for regulated securities
- **Identity Verification**: Integration with identity registry and claim validation
- **Modular Compliance**: Pluggable compliance modules for different regulatory requirements
- **Freezing Mechanisms**: Address-level and partial token freezing for regulatory compliance
- **Recovery System**: Wallet recovery for lost private keys
- **Role-based Access Control**: Granular permissions for different administrative functions

## Security Considerations

- All administrative functions require proper role-based authorization
- Cross-contract calls use typed clients for type safety
- Identity verification is required before token operations
- Compliance modules can block transfers based on regulatory rules
- Freezing mechanisms provide regulatory compliance tools

## Development Notes

- Each contract is a separate crate to avoid WASM symbol conflicts
- Cross-contract communication uses `contractimport!` macro
- Integration tests demonstrate the full system working together
- Individual contract tests provide focused unit testing
