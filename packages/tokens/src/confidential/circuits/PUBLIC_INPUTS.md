# Public-input ordering (frozen for v0.8)

Each Noir circuit takes its public inputs as a flat sequence of `Field`
elements. Grumpkin points are split into `(x, y)` pairs in the order shown
below. **This doc is the integration contract** between the circuits, the
verifier (#701), the wrapper, and the SDK -- renaming or reordering after the
fact cascades through every consumer, so any change here is a v0.8-breaking
change and must be reviewed accordingly.

Per design doc Section 7.1:
- Grumpkin point coordinates are each a 32-byte representative in `[0, r)`.
- Identity is encoded as `(0, 0)`; any other coordinate pair is treated as a
  candidate affine point and validated in-circuit.
- Public-input *keys* (`R_e`, `Y`, `PVK`, `K_aud`) must satisfy `P != O`;
  *commitments* may equal `O`.

Sub-issues append their tables here as each per-circuit PR lands. See #703
for the tracking epic.

## Register (`circuit_register`) -- design doc Section 7.2

| Index | Symbol  | Source                   | Encoding         |
|:-----:|:--------|:-------------------------|:-----------------|
|   0   | `Y.x`   | spending public key      | Grumpkin x-coord |
|   1   | `Y.y`   | spending public key      | Grumpkin y-coord |
|   2   | `PVK.x` | public viewing key       | Grumpkin x-coord |
|   3   | `PVK.y` | public viewing key       | Grumpkin y-coord |
|   4   | `wrap`  | wrapper contract address | Field (Fr)       |

Private witness: `sk`.

Constraints enforced: R1 (`Y = sk * H`), R2 (`vk = Poseidon2(d_vk, sk, wrap)`),
R3 (`PVK = vk * H`), plus `assert_on_curve_non_identity` on both `Y` and `PVK`.
