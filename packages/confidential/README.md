# Confidential Token

A Soroban contract suite that provides SEP-41 tokens with **private balances and transfers**. Balances are stored as unchunked Pedersen commitments on the Grumpkin curve; every state-changing operation that consumes private state is accompanied by an UltraHonk zero-knowledge proof that the contract verifies cross-contract. Recipients and auditors recover plaintext via per-transfer ephemeral ECDH.

The protocol provides **confidentiality, not anonymity** — sender and recipient addresses remain visible on-chain; amounts and balances do not.

## Layout

```text
confidential/
├── src/
│   ├── lib.rs      # ConfidentialToken trait + Hooks + events
│   ├── storage.rs  # operation-level orchestration, public-input assembly
│   ├── auditor/    # Grumpkin auditor-key registry (separate contract)
│   ├── verifier/   # UltraHonk VK registry (separate contract)
│   └── compliance/ # turnkey ComplianceHooks: freeze, SAC passthrough, external policy
├── circuits/       # Noir/UltraHonk circuits + pinned VKs
└── docs/
    ├── README.md                  # documentation index: where to start, map, citation rules
    ├── protocol/                  # normative protocol specification, one topic per file
    │   └── operations/            # one file per entry point
    ├── compliance.md              # compliance-extension specification
    ├── selective-disclosure/      # off-chain selective-disclosure layer
    ├── indexer.md                 # durable event archive specification
    ├── sdk/                       # client SDK specification
    └── overview.md                # non-normative user-flows overview
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

The core module holds the per-account `ConfidentialAccount` and per-`(owner, spender)` `SpenderDelegation` entries, and exposes the entry points that drive them. Every state-changing entry point:

1. Calls `require_auth()` on the appropriate account.
2. XDR-decodes the `data: Bytes` envelope into a typed `…Payload`.
3. Runs the matching [`Hooks`] callback.
4. Delegates to a function in [`storage`] that loads trusted public
   inputs, assembles the public-input blob, calls
   [`ConfidentialVerifier::verify_proof`] cross-contract, applies the
   state mutation, and emits the event.

The `Hooks` associated type is the extension point — wire `NoHooks` for a plain deployment, or [`ComplianceHooks`](./src/compliance/) for a gated one.

### `auditor`

Stores Grumpkin public keys used to produce per-transfer auditor ciphertexts. Each key is indexed by a `u32` `auditor_id`. Writes (register / rotate) are privileged and require the implementor to wire access control.

### `verifier`

Stores one UltraHonk verification key per [`CircuitType`]. Updating a VK is **soundness-critical** and should be treated as a break-glass operation — see the module docstring for the rationale and the recommended governance posture.

### `compliance`

A turnkey [`Hooks`] implementation layering deployer-configurable controls on top of the token: per-account freezing, SAC `authorized()` passthrough, and an optional external authorization policy. Wire as `type Hooks = ComplianceHooks;`. See [`docs/compliance.md`](./docs/compliance.md) for the specification.

### `circuits`

Noir sources, build scripts, and the pinned UltraHonk VKs that the verifier registry serves. Regeneration is gated on a CI diff against the committed `circuits/vks/*.vk.json` files; see [`circuits/vks/README.md`](./circuits/vks/README.md) for the toolchain pin and re-extraction procedure.

## Documentation

Start with [`docs/overview.md`](./docs/overview.md) for the design without mathematics, then read the protocol specification from [`docs/protocol/README.md`](./docs/protocol/README.md) onward; each file links to the next. [`docs/README.md`](./docs/README.md) maps the whole set: reading paths per role, the compliance, selective-disclosure, indexer, and SDK companions, and the citation rules that CI enforces with lychee.

## Packaging

The crate's packaging follows from its UltraHonk verifier backend
(`ultrahonk-soroban-verifier`), which links Rust's `alloc` crate and has no
crates.io release.

- **Separate from `stellar-tokens`.** As a module of `stellar-tokens`, the verifier would put `alloc` in the dependency graph of every contract that uses a fungible, non-fungible, RWA, or vault token, and each of them would need a global allocator it never uses. A `confidential` cargo feature on `stellar-tokens` would remove that coupling but would leave `stellar-tokens` unpublishable, because `cargo publish` rejects git dependencies even when they are optional.
- **Contracts provide the allocator.** A wasm binary whose dependency graph includes `alloc` needs exactly one `#[global_allocator]`. Enabling the `alloc` feature of `soroban-sdk` registers the SDK's bump allocator as that allocator, and Cargo features are additive, so a library that enables it prevents every dependent contract from registering its own (`the #[global_allocator] in this crate conflicts with global allocator in: soroban_sdk`). This crate leaves the feature off, and each contract either enables it or registers its own allocator. The verifier's repository splits its library and contracts the same way.
- **`lib` only.** Without an allocator, a standalone `cdylib` build of this crate fails with `no global memory allocator found but one is required`. The crate is compiled to wasm as a dependency of the contracts under `examples/confidential/`, which provide one.
- **`publish = false`.** The verifier is a git dependency, which `cargo publish` rejects. The crate can be published once the verifier has a crates.io release.
