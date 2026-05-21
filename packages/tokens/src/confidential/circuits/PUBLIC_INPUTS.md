# Public-input ordering (frozen for v0.8)

Each Noir circuit takes its public inputs as a flat sequence of `Field`
elements. Grumpkin points are split into `(x, y)` pairs in the order shown
below. **This doc is the integration contract** between the circuits, the
verifier (#701), the wrapper, and the SDK -- renaming or reordering after the
fact cascades through every consumer, so any change here is a v0.8-breaking
change and must be reviewed accordingly.

Point-validation doctrine (design doc Section 10.8):
- Grumpkin point coordinates are each a 32-byte representative in `[0, r)`.
- Identity is encoded as `(0, 0)`.
- Public-input keys bound to in-circuit `scalar_mul` outputs (`Y`, `PVK`,
  `R_e`, ECDH shared secrets, commitments) are on-curve by construction --
  no explicit on-curve check is needed in the circuit. Non-identity is
  enforced by `sk != 0` / `vk != 0` / `r_e != 0` constraints on the
  underlying scalars.
- Auditor keys (`K_aud`) are the only proof-less entry point; the auditor
  contract validates them at insertion.

Section 7.1 ("Public Input Sources") defines the trust-boundary rule: each
public input must be loaded by the wrapper from trusted state (account
storage, current contract address, auditor lookup) or come from invocation
arguments bound under `require_auth`. Caller-supplied `data` payloads MUST
NOT carry inputs that should originate from trusted state.

Sub-issues append their tables here as each per-circuit PR lands. See #703
for the tracking epic.

## Register (`circuit_register`) -- design doc Section 7.2

| Index | Symbol  | Source                                        | Encoding         |
|:-----:|:--------|:----------------------------------------------|:-----------------|
|   0   | `Y.x`   | prover-supplied, written to `account.spending_key` | Grumpkin x-coord |
|   1   | `Y.y`   | prover-supplied, written to `account.spending_key` | Grumpkin y-coord |
|   2   | `PVK.x` | prover-supplied, written to `account.viewing_public_key` | Grumpkin x-coord |
|   3   | `PVK.y` | prover-supplied, written to `account.viewing_public_key` | Grumpkin y-coord |
|   4   | `wrap`  | `env.current_contract_address()`              | Field (Fr)       |

Private witness: `sk`.

Constraints enforced:
- R1: `Y = sk * H`
- R2: `vk = Poseidon2(d_vk, sk, wrap)`
- R3: `PVK = vk * H`
- R4: `sk != 0` (rules out `Y = O`)
- R5: `vk != 0` (rules out `PVK = O`, which would collapse every
  incoming-transfer ECDH)
