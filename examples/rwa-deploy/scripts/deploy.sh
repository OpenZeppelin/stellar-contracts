#!/usr/bin/env bash
# Master deploy script: deploys ALL contracts, configures every module, then locks.
#
# CRITICAL ordering: module constructors store a bootstrap admin. Before
# `set_compliance_address`, privileged module actions require that admin; after
# the bind step they require the Compliance contract's auth instead. Since the
# CLI can't authorize as the Compliance contract, ALL configuration must happen
# BEFORE calling `set_compliance_address`.
#
# Flow:
#   1. Deploy infrastructure (IRS, Verifier, Compliance, Token)
#   2. Bind token to compliance + IRS
#   3. Deploy all 7 compliance modules with bootstrap admin
#   4. Configure every module (IRS, rules, limits, allowlists)
#   5. Set compliance address on all modules (transfers control to compliance)
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

. "$SCRIPT_DIR/common.sh"

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
  local attempts=${STELLAR_DEPLOY_RETRIES:-4}
  local delay=${STELLAR_DEPLOY_RETRY_DELAY_SECONDS:-3}
  local attempt output status addr

  echo "--- Deploying $LABEL ---" >&2

  for attempt in $(seq 1 "$attempts"); do
    if output=$(stellar contract deploy "$@" 2>&1); then
      printf '%s\n' "$output" >&2
      addr=$(printf '%s\n' "$output" | awk 'NF { line = $0 } END { print line }')

      if require_contract_id "$LABEL" "$addr"; then
        echo "  $LABEL: $addr" >&2
        echo "$addr"
        return 0
      fi

      output="${output}
ERROR: deploy returned an empty or invalid contract id for $LABEL"
      status=1
    else
      status=$?
    fi

    if ! retryable_invoke_error "$output"; then
      printf '%s\n' "$output" >&2
      return "$status"
    fi

    if [ "$attempt" -eq "$attempts" ]; then
      printf '%s\n' "$output" >&2
      return "$status"
    fi

    echo "Retrying $LABEL deploy after transient Stellar CLI failure..." >&2
    sleep $((delay * attempt))
  done
}

write_addresses() {
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
  -- --admin "$ADMIN" --irs "$IRS")

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
invoke_with_retry "$COMPLIANCE" bind_token --token "$TOKEN" --operator "$ADMIN"
invoke_with_retry "$IRS" bind_token --token "$TOKEN" --operator "$ADMIN"
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

  deploy_contract "$NAME" \
    --wasm "$WASM_DIR/$WASM_NAME" \
    --source "$SOURCE" --network "$NETWORK" \
    -- --admin "$ADMIN"
}

COUNTRY_ALLOW=$(deploy_module country-allow)
COUNTRY_RESTRICT=$(deploy_module country-restrict)
INITIAL_LOCKUP=$(deploy_module initial-lockup-period)
MAX_BALANCE=$(deploy_module max-balance)
SUPPLY_LIMIT=$(deploy_module supply-limit)
TIME_TRANSFERS=$(deploy_module time-transfers-limits)
TRANSFER_RESTRICT=$(deploy_module transfer-restrict)

# Persist deployed addresses early so `wire.sh` and manual recovery can resume
# if a later configuration or bind step is interrupted.
write_addresses

# ── Step 4: Configure ALL modules (before compliance bind) ──
#
# After set_compliance_address, module admin calls require Compliance contract
# auth which is impossible from the CLI. So ALL config goes here.

echo ""
echo "=== Step 4/5: Configuring modules (before compliance bind) ==="

# 4a. Set IRS on identity-aware modules
echo "  Setting IRS on identity-aware modules..."
invoke_with_retry "$COUNTRY_ALLOW" set_identity_registry_storage --token "$TOKEN" --irs "$IRS"
invoke_with_retry "$COUNTRY_RESTRICT" set_identity_registry_storage --token "$TOKEN" --irs "$IRS"
invoke_with_retry "$MAX_BALANCE" set_identity_registry_storage --token "$TOKEN" --irs "$IRS"
invoke_with_retry "$TIME_TRANSFERS" set_identity_registry_storage --token "$TOKEN" --irs "$IRS"

# 4b. CountryAllow: allow US (840), GB (826), DE (276)
echo "  CountryAllow: adding US, GB, DE..."
invoke_with_retry "$COUNTRY_ALLOW" add_allowed_country --token "$TOKEN" --country 840
invoke_with_retry "$COUNTRY_ALLOW" add_allowed_country --token "$TOKEN" --country 826
invoke_with_retry "$COUNTRY_ALLOW" add_allowed_country --token "$TOKEN" --country 276

# 4c. CountryRestrict: restrict North Korea (408), Iran (364)
echo "  CountryRestrict: blocking DPRK, IRN..."
invoke_with_retry "$COUNTRY_RESTRICT" add_country_restriction --token "$TOKEN" --country 408
invoke_with_retry "$COUNTRY_RESTRICT" add_country_restriction --token "$TOKEN" --country 364

# 4d. MaxBalance: set limit of 1,000,000 tokens
echo "  MaxBalance: setting limit 1000000..."
invoke_with_retry "$MAX_BALANCE" set_max_balance --token "$TOKEN" --max 1000000

# 4e. SupplyLimit: set total supply limit of 10,000,000
echo "  SupplyLimit: setting limit 10000000..."
invoke_with_retry "$SUPPLY_LIMIT" set_supply_limit --token "$TOKEN" --limit 10000000

# 4f. TimeTransfersLimits: set daily limit of 100,000
echo "  TimeTransfersLimits: setting daily limit 100000..."
invoke_with_retry "$TIME_TRANSFERS" set_time_transfer_limit \
  --token "$TOKEN" \
  --limit '{"limit_time":86400,"limit_value":"100000"}'

# 4g. InitialLockupPeriod: set lockup to 300 seconds (5 min for testing)
echo "  InitialLockupPeriod: setting lockup 300s..."
invoke_with_retry "$INITIAL_LOCKUP" set_lockup_period --token "$TOKEN" --lockup_seconds 300

# 4h. TransferRestrict: allow the admin address
echo "  TransferRestrict: allowing admin..."
invoke_with_retry "$TRANSFER_RESTRICT" allow_user --token "$TOKEN" --user "$ADMIN"

echo "  All modules configured."

# ── Step 5: Set compliance address on ALL modules (hands off to compliance) ──

echo ""
echo "=== Step 5/5: Locking all modules to compliance ==="
for MODULE_ADDR in "$COUNTRY_ALLOW" "$COUNTRY_RESTRICT" "$INITIAL_LOCKUP" \
  "$MAX_BALANCE" "$SUPPLY_LIMIT" "$TIME_TRANSFERS" "$TRANSFER_RESTRICT"; do
  invoke_with_retry "$MODULE_ADDR" set_compliance_address --compliance "$COMPLIANCE"
done
echo "  All 7 modules bound. Admin functions now require Compliance contract auth."

# ── Save addresses ──

write_addresses

echo ""
echo "=== Deployment Complete ==="
echo "  Infrastructure: IRS, Verifier, Compliance, Token"
echo "  Modules:        7/7 deployed and configured"
echo "  Status:         All modules bound to Compliance"
echo "  Addresses:      $ADDR_FILE"
echo ""
echo "Next steps:"
echo "  1. ./wire.sh            — Register modules on compliance hooks"
echo "  2. ./test-happy-path.sh — Mint tokens and verify compliance"
