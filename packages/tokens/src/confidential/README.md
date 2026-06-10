# Confidential Token

A Soroban contract suite that provides SEP-41 tokens with **private balances
and transfers**. Balances are stored as unchunked Pedersen commitments on
the Grumpkin curve; every state-changing operation that consumes private
state is accompanied by an UltraHonk zero-knowledge proof that the contract
verifies cross-contract. Recipients and auditors recover plaintext via
per-transfer ephemeral ECDH.

The protocol provides **confidentiality, not anonymity** — sender and
recipient addresses remain visible on-chain; amounts and balances do not.

## ⚠️ Not Production Ready

The [`verifier`](./verifier/) module's UltraHonk backend
([`rs-soroban-ultrahonk`](https://github.com/NethermindEth/rs-soroban-ultrahonk))
is still under development and has **not been audited**. Do **not** deploy
a contract built on this trait to mainnet or any environment that handles
real value.

## Layout

```text
confidential/
├── mod.rs          # ConfidentialToken trait + Hooks + events
├── storage.rs      # operation-level orchestration, public-input assembly
├── auditor/        # Grumpkin auditor-key registry (separate contract)
├── verifier/       # UltraHonk VK registry (separate contract)
├── compliance/     # turnkey ComplianceHooks: freeze, SAC passthrough, external policy
├── circuits/       # Noir/UltraHonk circuits + pinned VKs
└── docs/
    ├── DESIGN.md       # full protocol specification
    └── COMPLIANCE.md   # compliance-extension specification
```

A single deployment is **three contracts** wired together:

1. A `ConfidentialToken` contract.
2. A `ConfidentialAuditor` registry (Grumpkin public keys, indexed by
   `auditor_id`; reusable across multiple tokens).
3. A `ConfidentialVerifier` registry (one UltraHonk VK per
   [`CircuitType`]; reusable across tokens that share the protocol
   version).

## Modules

### `confidential_token`

The core module holds the per-account `ConfidentialAccount` and
per-`(owner, spender)` `SpenderDelegation` entries, and exposes the 
entry points that drive them. Every state-changing entry point:

1. Calls `require_auth()` on the appropriate account.
2. XDR-decodes the `data: Bytes` envelope into a typed `…Payload`.
3. Runs the matching [`Hooks`] callback.
4. Delegates to a function in [`storage`] that loads trusted public
   inputs, assembles the public-input blob, calls
   [`ConfidentialVerifier::verify_proof`] cross-contract, applies the
   state mutation, and emits the event.

The `Hooks` associated type is the extension point — wire `NoHooks` for a
plain deployment, or [`ComplianceHooks`](./compliance/) for a gated one.

### `auditor`

Stores Grumpkin public keys used to produce per-transfer auditor
ciphertexts. Each key is indexed by a `u32` `auditor_id`. Writes (register
/ rotate) are privileged and require the implementor to wire access
control.

### `verifier`

Stores one UltraHonk verification key per [`CircuitType`]. Updating a VK is
**soundness-critical** and should be treated as a break-glass operation —
see the module docstring for the rationale and the recommended governance
posture.

### `compliance`

A turnkey [`Hooks`] implementation layering deployer-configurable
controls on top of the token: per-account freezing, SAC `authorized()`
passthrough, and an optional external authorization policy. Wire as
`type Hooks = ComplianceHooks;`. See
[`docs/COMPLIANCE.md`](./docs/COMPLIANCE.md) for the specification.

### `circuits`

Noir sources, build scripts, and the pinned UltraHonk VKs that the
verifier registry serves. Regeneration is gated on a CI diff against the
committed `circuits/vks/*.vk.json` files; see
[`circuits/vks/README.md`](./circuits/vks/README.md) for the toolchain
pin and re-extraction procedure.

## Documentation

- [`docs/DESIGN.md`](./docs/DESIGN.md) — full protocol specification
  (cryptographic preliminaries, account model, circuits, public-input
  tables, security analysis).
- [`docs/COMPLIANCE.md`](./docs/COMPLIANCE.md) — compliance extension
  specification.
