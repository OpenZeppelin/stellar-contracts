#!/usr/bin/env bash
# Deploy a single compliance module, apply shared pre-bind setup, then bind + wire.
# Usage: ./deploy-module.sh <module-name> [hook1 hook2 ...]
# Example: ./deploy-module.sh country-allow CanTransfer CanCreate
#
# This script handles the correct ordering:
#   1. Deploy the module with bootstrap admin
#   2. Apply shared pre-bind setup (IRS on identity-aware modules only)
#   3. Set compliance address (hands off to compliance)
#   4. Register on hooks (optional)
#
# This helper intentionally does not apply module-specific business config
# (allowlists, limits, lockup periods, etc.). Use `deploy.sh` for the full
# stack configuration flow.
#
# Prerequisites: deploy.sh must have been run (needs addresses file for infra).
set -euo pipefail

if [ $# -lt 1 ]; then
  echo "Usage: $0 <module-name> [hook1 hook2 ...]"
  echo ""
  echo "Available modules: country-allow, country-restrict, initial-lockup-period,"
  echo "  max-balance, supply-limit, time-transfers-limits, transfer-restrict"
  echo ""
  echo "Available hooks: CanTransfer, CanCreate, Transferred, Created, Destroyed"
  exit 1
fi

MODULE="$1"; shift
HOOKS=("$@")

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
WASM_DIR="$ROOT_DIR/examples/rwa-deploy/wasm"
ADDR_FILE="$ROOT_DIR/examples/rwa-deploy/testnet-addresses.json"

SOURCE="${STELLAR_SOURCE:-alice}"
NETWORK="${STELLAR_NETWORK:-testnet}"

. "$SCRIPT_DIR/common.sh"

WASM_NAME="rwa_${MODULE//-/_}.wasm"
WASM_PATH="$WASM_DIR/$WASM_NAME"

if [ ! -f "$WASM_PATH" ]; then
  echo "ERROR: $WASM_NAME not found. Run build.sh or build-module.sh first." >&2
  exit 1
fi

if [ ! -f "$ADDR_FILE" ]; then
  echo "ERROR: testnet-addresses.json not found. Run deploy.sh first." >&2
  exit 1
fi

ADMIN=$(read_addr "['admin']")
TOKEN=$(read_addr "['contracts']['token']")
IRS=$(read_addr "['contracts']['irs']")
COMPLIANCE=$(read_addr "['contracts']['compliance']")

require_contract_id "token" "$TOKEN"
require_contract_id "irs" "$IRS"
require_contract_id "compliance" "$COMPLIANCE"

# ── Step 1: Deploy ──
echo "=== Deploying $MODULE ==="
MODULE_ADDR=$(stellar contract deploy \
  --wasm "$WASM_PATH" \
  --source "$SOURCE" --network "$NETWORK" \
  -- --admin "$ADMIN")
require_contract_id "$MODULE" "$MODULE_ADDR"
echo "  Address: $MODULE_ADDR"

# ── Step 2: Configure (before compliance bind) ──
IRS_MODULES=("country-allow" "country-restrict" "max-balance" "time-transfers-limits")
for irs_mod in "${IRS_MODULES[@]}"; do
  if [ "$MODULE" = "$irs_mod" ]; then
    echo "  Setting IRS..."
    invoke "$MODULE_ADDR" set_identity_registry_storage --token "$TOKEN" --irs "$IRS"
    break
  fi
done

# ── Step 3: Set compliance address (hands off to compliance) ──
echo "  Binding to compliance..."
invoke "$MODULE_ADDR" set_compliance_address --compliance "$COMPLIANCE"

# ── Step 4: Register on hooks ──
for HOOK in "${HOOKS[@]}"; do
  ensure_hook_registration "$HOOK" "$MODULE_ADDR" "$MODULE"
done

echo ""
echo "=== $MODULE deployed and bound ==="
echo "Address: $MODULE_ADDR"
