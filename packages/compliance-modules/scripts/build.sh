#!/usr/bin/env bash
# Build all 11 RWA contract WASMs (7 compliance modules + 4 infrastructure).
# Uses `stellar contract build` which handles WASM feature stripping properly,
# unlike raw `cargo build` + deprecated `stellar contract optimize`.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
WASM_DIR="$ROOT_DIR/packages/compliance-modules/deploy/wasm"

mkdir -p "$WASM_DIR"

MODULES=(
  stellar-compliance-country-allow
  stellar-compliance-country-restrict
  stellar-compliance-initial-lockup-period
  stellar-compliance-max-balance
  stellar-compliance-supply-limit
  stellar-compliance-time-transfers-limits
  stellar-compliance-transfer-restrict
)

INFRA=(
  deploy-irs
  deploy-verifier
  deploy-compliance
  deploy-token
)

ALL=("${MODULES[@]}" "${INFRA[@]}")

echo "=== Building ${#ALL[@]} WASMs ==="

cd "$ROOT_DIR"
for pkg in "${ALL[@]}"; do
  echo "  Building $pkg..."
  stellar contract build --package "$pkg" --out-dir "$WASM_DIR" 2>&1 | grep -v "^$" || true
done

echo ""
echo "=== WASM sizes ==="
for pkg in "${ALL[@]}"; do
  WASM_NAME="${pkg//-/_}.wasm"
  if [ -f "$WASM_DIR/$WASM_NAME" ]; then
    SIZE=$(wc -c < "$WASM_DIR/$WASM_NAME" | tr -d ' ')
    echo "  $WASM_NAME (${SIZE} bytes)"
  else
    echo "  WARNING: $WASM_NAME not found!"
  fi
done

echo ""
echo "=== All WASMs built to deploy/wasm/ ==="
