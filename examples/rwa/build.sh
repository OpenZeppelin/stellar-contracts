#!/bin/bash
set -e

echo "Building RWA contracts..."

# Core contracts in root directory
CORE_CONTRACTS=(
    "claim-issuer"
    "claim-topics-and-issuers"
    "compliance"
    "identity-claims"
    "identity-registry-storage"
    "rwa-token"
)

# Compliance modules in subdirectory
COMPLIANCE_MODULES=(
    "transfer-limit-module"
    "country-restriction-module"
)

# Build core contracts
for contract in "${CORE_CONTRACTS[@]}"; do
    echo "Building $contract..."
    (cd "$contract" && cargo build --target wasm32-unknown-unknown --release)
done

# Build compliance modules
for module in "${COMPLIANCE_MODULES[@]}"; do
    echo "Building compliance-modules/$module..."
    (cd "compliance-modules/$module" && cargo build --target wasm32-unknown-unknown --release)
done

echo ""
echo "All RWA contracts built successfully!"
echo ""
echo "WASM files are located in:"
echo "./target/wasm32-unknown-unknown/release"
