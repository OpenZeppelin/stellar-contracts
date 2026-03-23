#!/usr/bin/env bash

retryable_invoke_error() {
  local output=$1
  case "$output" in
    *"transaction submission timeout"* | \
    *"could not load platform certs"* | \
    *"connection reset"* | \
    *"temporarily unavailable"* | \
    *"deadline has elapsed"* | \
    *"timed out"*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

is_contract_id() {
  case "$1" in
    C[A-Z0-9]*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

require_contract_id() {
  local label=$1
  local contract_id=$2

  if ! is_contract_id "$contract_id"; then
    echo "ERROR: Missing or invalid $label contract id: '$contract_id'" >&2
    return 1
  fi
}

invoke() {
  local contract_id=$1
  require_contract_id "invoke target" "$contract_id" || return 1

  stellar contract invoke --id "$contract_id" \
    --source "$SOURCE" --network "$NETWORK" \
    -- "${@:2}"
}

invoke_readonly() {
  local contract_id=$1
  require_contract_id "readonly invoke target" "$contract_id" || return 1

  stellar contract invoke --id "$contract_id" \
    --source-account "$ADMIN" --network "$NETWORK" \
    -- "${@:2}"
}

invoke_with_retry() {
  local attempts=${STELLAR_INVOKE_RETRIES:-4}
  local delay=${STELLAR_INVOKE_RETRY_DELAY_SECONDS:-3}
  local attempt output status

  for attempt in $(seq 1 "$attempts"); do
    if output=$(invoke "$@" 2>&1); then
      printf '%s\n' "$output"
      return 0
    fi
    status=$?

    if ! retryable_invoke_error "$output"; then
      printf '%s\n' "$output" >&2
      return "$status"
    fi

    if [ "$attempt" -eq "$attempts" ]; then
      printf '%s\n' "$output" >&2
      return "$status"
    fi

    echo "Retrying deploy invoke after transient Stellar CLI failure..." >&2
    sleep $((delay * attempt))
  done
}

read_addr() {
  python3 -c "import json; d=json.load(open('$ADDR_FILE')); print(d$1)"
}

hook_modules() {
  invoke_readonly "$COMPLIANCE" get_modules_for_hook --hook "\"$1\""
}

is_module_registered_for_hook() {
  local hook=$1
  local module=$2
  local output

  if ! output=$(hook_modules "$hook" 2>&1); then
    return 1
  fi

  MODULE_TO_FIND="$module" python3 -c '
import json, os, sys

module = os.environ["MODULE_TO_FIND"]
lines = [line.strip() for line in sys.stdin.read().splitlines() if line.strip()]
payload = lines[-1] if lines else "[]"
modules = json.loads(payload)
sys.exit(0 if module in modules else 1)
' <<<"$output"
}

ensure_hook_registration() {
  local hook=$1
  local module_addr=$2
  local name=$3
  local attempts=${STELLAR_INVOKE_RETRIES:-4}
  local delay=${STELLAR_INVOKE_RETRY_DELAY_SECONDS:-3}
  local attempt output status

  echo "  $name -> $hook"

  for attempt in $(seq 1 "$attempts"); do
    if is_module_registered_for_hook "$hook" "$module_addr"; then
      echo "    already registered"
      return 0
    fi

    if output=$(invoke "$COMPLIANCE" add_module_to --hook "\"$hook\"" --module "$module_addr" --operator "$ADMIN" 2>&1); then
      printf '%s\n' "$output"
      return 0
    fi
    status=$?

    if is_module_registered_for_hook "$hook" "$module_addr"; then
      echo "    registered after retryable failure"
      return 0
    fi

    case "$output" in
      *"Error(Contract, #360)"* | *"ModuleAlreadyRegistered"*)
        echo "    already registered"
        return 0
        ;;
    esac

    if ! retryable_invoke_error "$output"; then
      printf '%s\n' "$output" >&2
      return "$status"
    fi

    if [ "$attempt" -eq "$attempts" ]; then
      printf '%s\n' "$output" >&2
      return "$status"
    fi

    echo "    retrying after transient Stellar CLI failure..." >&2
    sleep $((delay * attempt))
  done
}

verify_hook_wiring_with_retry() {
  local module_addr=$1
  local name=$2
  local attempts=${STELLAR_INVOKE_RETRIES:-4}
  local delay=${STELLAR_INVOKE_RETRY_DELAY_SECONDS:-3}
  local attempt output status

  echo "  Verifying $name..."

  for attempt in $(seq 1 "$attempts"); do
    if output=$(invoke "$module_addr" verify_hook_wiring 2>&1); then
      printf '%s\n' "$output"
      return 0
    fi
    status=$?

    if ! retryable_invoke_error "$output"; then
      printf '%s\n' "$output" >&2
      return "$status"
    fi

    if [ "$attempt" -eq "$attempts" ]; then
      printf '%s\n' "$output" >&2
      return "$status"
    fi

    echo "    retrying verification after transient Stellar CLI failure..." >&2
    sleep $((delay * attempt))
  done
}

identity_matches() {
  local contract_addr=$1
  local account=$2
  local expected_identity=$3
  local output

  if ! output=$(invoke_readonly "$contract_addr" stored_identity --account "$account" 2>&1); then
    return 1
  fi

  EXPECTED_IDENTITY="$expected_identity" python3 -c '
import os, sys

expected = os.environ["EXPECTED_IDENTITY"]
lines = [line.strip() for line in sys.stdin.read().splitlines() if line.strip()]
payload = lines[-1].strip("\"") if lines else ""
sys.exit(0 if payload == expected else 1)
' <<<"$output"
}

country_profiles_to_scval_json() {
  python3 - "$1" <<'PY'
import json
import sys

profiles = json.loads(sys.argv[1])

def sc_symbol(value):
    return {"symbol": value}

def sc_string(value):
    return {"string": value}

def sc_u32(value):
    return {"u32": value}

def metadata_to_scval(metadata):
    if metadata is None:
        return "void"

    return {
        "map": [
            {"key": sc_string(key), "val": sc_string(value)}
            for key, value in sorted(metadata.items())
        ]
    }

def enum_to_scval(enum_value):
    [(outer_name, outer_payload)] = enum_value.items()
    [(inner_name, inner_payload)] = outer_payload.items()
    return {"vec": [sc_symbol(outer_name), {"vec": [sc_symbol(inner_name), sc_u32(inner_payload)]}]}

def country_data_to_scval(profile):
    return {
        "map": [
            {"key": sc_symbol("country"), "val": enum_to_scval(profile["country"])},
            {"key": sc_symbol("metadata"), "val": metadata_to_scval(profile.get("metadata"))},
        ]
    }

print(json.dumps([country_data_to_scval(profile) for profile in profiles], separators=(",", ":")))
PY
}

ensure_identity_registered() {
  local contract_addr=$1
  local account=$2
  local identity=$3
  local profiles_json=$4
  local profiles_scval_json
  local attempts=${STELLAR_INVOKE_RETRIES:-4}
  local delay=${STELLAR_INVOKE_RETRY_DELAY_SECONDS:-3}
  local attempt output status

  profiles_scval_json=$(country_profiles_to_scval_json "$profiles_json")

  if identity_matches "$contract_addr" "$account" "$identity"; then
    echo "  Identity already registered for $account."
    return 0
  fi

  for attempt in $(seq 1 "$attempts"); do
    if output=$(invoke "$contract_addr" add_identity \
      --account "$account" \
      --identity "$identity" \
      --initial_profiles "$profiles_scval_json" \
      --operator "$ADMIN" 2>&1); then
      printf '%s\n' "$output"
      return 0
    fi
    status=$?

    if identity_matches "$contract_addr" "$account" "$identity"; then
      echo "  Identity registration confirmed after retryable failure for $account."
      return 0
    fi

    if ! retryable_invoke_error "$output"; then
      printf '%s\n' "$output" >&2
      return "$status"
    fi

    if [ "$attempt" -eq "$attempts" ]; then
      printf '%s\n' "$output" >&2
      return "$status"
    fi

    echo "  Retrying identity registration after transient Stellar CLI failure..." >&2
    sleep $((delay * attempt))
  done
}
