#!/usr/bin/env bash
# Master deploy script: deploys ALL contracts, configures every module, then locks.
#
# CRITICAL ordering: every module admin function uses `require_compliance_auth`,
# which is a no-op before `set_compliance_address` but requires the Compliance
# contract's auth after. Since the CLI can't authorize as the Compliance contract,
# ALL configuration must happen BEFORE calling `set_compliance_address`.
#
# Flow:
#   1. Deploy infrastructure (IRS, Verifier, Compliance, Token)
#   2. Bind token to compliance + IRS
#   3. Deploy all 7 compliance modules
#   4. Configure every module (IRS, rules, limits, allowlists)
#   5. Set compliance address on all modules (locks admin forever)
#
# After this: run wire.sh, then test scripts.
#
# Prerequisites: build.sh must have been run first.
# Env vars:
#   STELLAR_SOURCE  - signing key alias (default: alice)
#   STELLAR_NETWORK - network passphrase (default: testnet)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
WASM_DIR="$ROOT_DIR/examples/rwa-deploy/wasm"
ADDR_FILE="$ROOT_DIR/examples/rwa-deploy/testnet-addresses.json"

SOURCE="${STELLAR_SOURCE:-alice}"
NETWORK="${STELLAR_NETWORK:-testnet}"

if [ ! -d "$WASM_DIR" ] || [ -z "$(ls -A "$WASM_DIR" 2>/dev/null)" ]; then
  echo "ERROR: No WASMs found. Run build.sh first." >&2
  exit 1
fi

echo "=== Deploying RWA Stack ==="
echo "Source: $SOURCE | Network: $NETWORK"
echo ""

ADMIN=$(stellar keys address "$SOURCE")

deploy_contract() {
  local LABEL=$1; shift
  echo "--- Deploying $LABEL ---" >&2
  local ADDR
  ADDR=$(stellar contract deploy "$@")
  echo "  $LABEL: $ADDR" >&2
  echo "$ADDR"
}

invoke() {
  stellar contract invoke --id "$1" \
    --source "$SOURCE" --network "$NETWORK" \
    -- "${@:2}"
}

# ── Step 1: Deploy infrastructure ──

echo "=== Step 1/5: Infrastructure ==="

IRS=$(deploy_contract "IRS" \
  --wasm "$WASM_DIR/deploy_irs.wasm" \
  --source "$SOURCE" --network "$NETWORK" \
  -- --admin "$ADMIN" --manager "$ADMIN")

VERIFIER=$(deploy_contract "Verifier" \
  --wasm "$WASM_DIR/deploy_verifier.wasm" \
  --source "$SOURCE" --network "$NETWORK" \
  -- --irs "$IRS")

COMPLIANCE=$(deploy_contract "Compliance" \
  --wasm "$WASM_DIR/deploy_compliance.wasm" \
  --source "$SOURCE" --network "$NETWORK" \
  -- --admin "$ADMIN")

TOKEN=$(deploy_contract "Token" \
  --wasm "$WASM_DIR/deploy_token.wasm" \
  --source "$SOURCE" --network "$NETWORK" \
  -- --name "RWA Test Token" --symbol "RWAT" \
  --admin "$ADMIN" --compliance "$COMPLIANCE" --identity_verifier "$VERIFIER")

# ── Step 2: Bind token ──

echo ""
echo "=== Step 2/5: Binding token ==="
invoke "$COMPLIANCE" bind_token --token "$TOKEN" --operator "$ADMIN"
invoke "$IRS" bind_token --token "$TOKEN" --operator "$ADMIN"
echo "  Token bound to Compliance + IRS."

# ── Step 3: Deploy all 7 modules ──

echo ""
echo "=== Step 3/5: Deploying compliance modules ==="

deploy_module() {
  local NAME=$1
  local WASM_NAME="rwa_${NAME//-/_}.wasm"
  if [ ! -f "$WASM_DIR/$WASM_NAME" ]; then
    echo "ERROR: $WASM_NAME not found — run build.sh first" >&2
    return 1
  fi
  local ADDR
  ADDR=$(stellar contract deploy \
    --wasm "$WASM_DIR/$WASM_NAME" \
    --source "$SOURCE" --network "$NETWORK")
  echo "  $NAME: $ADDR" >&2
  echo "$ADDR"
}

COUNTRY_ALLOW=$(deploy_module country-allow)
COUNTRY_RESTRICT=$(deploy_module country-restrict)
INITIAL_LOCKUP=$(deploy_module initial-lockup-period)
MAX_BALANCE=$(deploy_module max-balance)
SUPPLY_LIMIT=$(deploy_module supply-limit)
TIME_TRANSFERS=$(deploy_module time-transfers-limits)
TRANSFER_RESTRICT=$(deploy_module transfer-restrict)

# ── Step 4: Configure ALL modules (before compliance lock!) ──
#
# After set_compliance_address, admin calls require Compliance contract auth
# which is impossible from the CLI. So ALL config goes here.

echo ""
echo "=== Step 4/5: Configuring modules (before compliance lock) ==="

# 4a. Set IRS on identity-aware modules
echo "  Setting IRS on identity-aware modules..."
invoke "$COUNTRY_ALLOW" set_identity_registry_storage --token "$TOKEN" --irs "$IRS"
invoke "$COUNTRY_RESTRICT" set_identity_registry_storage --token "$TOKEN" --irs "$IRS"
invoke "$MAX_BALANCE" set_identity_registry_storage --token "$TOKEN" --irs "$IRS"
invoke "$TIME_TRANSFERS" set_identity_registry_storage --token "$TOKEN" --irs "$IRS"

# 4b. CountryAllow: allow US (840), GB (826), DE (276)
echo "  CountryAllow: adding US, GB, DE..."
invoke "$COUNTRY_ALLOW" add_allowed_country --token "$TOKEN" --country 840
invoke "$COUNTRY_ALLOW" add_allowed_country --token "$TOKEN" --country 826
invoke "$COUNTRY_ALLOW" add_allowed_country --token "$TOKEN" --country 276

# 4c. CountryRestrict: restrict North Korea (408), Iran (364)
echo "  CountryRestrict: blocking DPRK, IRN..."
invoke "$COUNTRY_RESTRICT" add_country_restriction --token "$TOKEN" --country 408
invoke "$COUNTRY_RESTRICT" add_country_restriction --token "$TOKEN" --country 364

# 4d. MaxBalance: set limit of 1,000,000 tokens
echo "  MaxBalance: setting limit 1000000..."
invoke "$MAX_BALANCE" set_max_balance --token "$TOKEN" --max 1000000

# 4e. SupplyLimit: set total supply limit of 10,000,000
echo "  SupplyLimit: setting limit 10000000..."
invoke "$SUPPLY_LIMIT" set_supply_limit --token "$TOKEN" --limit 10000000

# 4f. TimeTransfersLimits: set daily limit of 100,000
echo "  TimeTransfersLimits: setting daily limit 100000..."
invoke "$TIME_TRANSFERS" set_time_transfer_limit \
  --token "$TOKEN" \
  --limit '{"limit_time":86400,"limit_value":"100000"}'

# 4g. InitialLockupPeriod: set lockup to 300 seconds (5 min for testing)
echo "  InitialLockupPeriod: setting lockup 300s..."
invoke "$INITIAL_LOCKUP" set_lockup_period --token "$TOKEN" --lockup_seconds 300

# 4h. TransferRestrict: allow the admin address
echo "  TransferRestrict: allowing admin..."
invoke "$TRANSFER_RESTRICT" allow_user --token "$TOKEN" --user "$ADMIN"

echo "  All modules configured."

# ── Step 5: Set compliance address on ALL modules (locks admin) ──

echo ""
echo "=== Step 5/5: Locking all modules to compliance ==="
for MODULE_ADDR in "$COUNTRY_ALLOW" "$COUNTRY_RESTRICT" "$INITIAL_LOCKUP" \
  "$MAX_BALANCE" "$SUPPLY_LIMIT" "$TIME_TRANSFERS" "$TRANSFER_RESTRICT"; do
  invoke "$MODULE_ADDR" set_compliance_address --compliance "$COMPLIANCE"
done
echo "  All 7 modules locked. Admin functions now require Compliance contract auth."

# ── Save addresses ──

cat > "$ADDR_FILE" <<EOF
{
  "network": "$NETWORK",
  "source": "$SOURCE",
  "admin": "$ADMIN",
  "contracts": {
    "irs": "$IRS",
    "verifier": "$VERIFIER",
    "compliance": "$COMPLIANCE",
    "token": "$TOKEN"
  },
  "modules": {
    "country_allow": "$COUNTRY_ALLOW",
    "country_restrict": "$COUNTRY_RESTRICT",
    "initial_lockup_period": "$INITIAL_LOCKUP",
    "max_balance": "$MAX_BALANCE",
    "supply_limit": "$SUPPLY_LIMIT",
    "time_transfers_limits": "$TIME_TRANSFERS",
    "transfer_restrict": "$TRANSFER_RESTRICT"
  }
}
EOF

echo ""
echo "=== Deployment Complete ==="
echo "  Infrastructure: IRS, Verifier, Compliance, Token"
echo "  Modules:        7/7 deployed and configured"
echo "  Status:         All modules locked to Compliance"
echo "  Addresses:      $ADDR_FILE"
echo ""
echo "Next steps:"
echo "  1. ./wire.sh            — Register modules on compliance hooks"
echo "  2. ./test-happy-path.sh — Mint tokens and verify compliance"
