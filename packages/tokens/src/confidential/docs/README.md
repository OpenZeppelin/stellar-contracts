# Confidential Token Documentation

Specification set for the confidential token module. Each file owns one topic. The protocol specification is the `protocol/` directory read in the order given below; the remaining files are companion specifications that cite it.

## Normativity and precedence

- `protocol/` is normative for the on-chain protocol. `compliance.md` is normative for the optional compliance extension, `selective-disclosure/` for the off-chain disclosure layer, `indexer.md` for the durable event archive, and `sdk/` for client implementations. `overview.md` is non-normative.
- Where a document and `circuits/lib/src/lib.nr` disagree about a primitive, the Noir library is authoritative. The contract's `#[contracttype]` definitions are authoritative for the shape of on-chain types and payloads.
- Every fact has one owning section. Other files cite it rather than restating it.

## Citing a section

Cite by relative path and heading anchor, for example `protocol/wallet-state.md#recovery`: as a Markdown link from the docs, as a plain `docs/...` path from Rust and Noir comments. Anchors are GitHub heading slugs: lower-case, punctuation removed, spaces replaced by hyphens. Constraint identifiers such as `T_a9`, `S14`, or `CB3` are stable and may be cited by name.

`check_links.py` resolves every such reference in the module and fails on a missing file or heading, a duplicate heading within a file, or a file over the LaTeX rendering budget. CI runs it on every change to the module.

## Map

### Protocol

| File | Owns |
|:---|:---|
| [protocol/README.md](protocol/README.md) | Abstract, design goals, approach |
| [protocol/primitives.md](protocol/primitives.md) | Notation, Grumpkin and BN254, Pedersen commitments, ECDH, Poseidon2 sponge and lane assignment, range proofs, address-to-field encoding |
| [protocol/system-model.md](protocol/system-model.md) | Components, threat model, trust assumptions, underlying-token requirements, governance |
| [protocol/keys-and-commitments.md](protocol/keys-and-commitments.md) | Key hierarchy, balance commitments, ECDH-derived blinding, anti-poisoning constraint, encrypted balance scalar |
| [protocol/wallet-state.md](protocol/wallet-state.md) | Wallet accumulators, update rules, checkpoints, consistency check, recovery procedure and its properties, event-durability requirement |
| [protocol/account-state.md](protocol/account-state.md) | `ConfidentialAccount` and `SpenderDelegation` storage entries, transfer nonce |
| [protocol/operations/README.md](protocol/operations/README.md) | Public-input sources and the trust-boundary rule, index of operations |
| [protocol/operations/](protocol/operations/) | One file per operation: constraints, public inputs, private witnesses, post-verification state |
| [protocol/auditing.md](protocol/auditing.md) | Auditor ciphertexts, opening capabilities, key rotation, spender and allowance auditing |
| [protocol/security.md](protocol/security.md) | Griefing resistance, merge safety, balance conservation, privacy properties, revert safety, replay protection |
| [protocol/proof-system.md](protocol/proof-system.md) | Circuit set and costs, Noir primitives, verification flow, structured reference string, CAP-80, on-chain point arithmetic |
| [protocol/interface.md](protocol/interface.md) | Entry points, payload encoding, authorization model, event schema, read methods |
| [protocol/domain-separators.md](protocol/domain-separators.md) | The seventeen Poseidon2 domain tags |

### Companions

| File | Owns |
|:---|:---|
| [compliance.md](compliance.md) | Freeze, SAC passthrough, policy contract, `Hooks` customisation, clawback and forced revocation |
| [selective-disclosure/README.md](selective-disclosure/README.md) | Motivation, goals, preliminaries, threat model of the disclosure layer |
| [selective-disclosure/protocol.md](selective-disclosure/protocol.md) | Disclosure ciphertext, proof bundle, verifier protocol, on-chain read surface, end-to-end flow |
| [selective-disclosure/circuits/](selective-disclosure/circuits/) | D-recipient, D-sender, D-auditor, D-balance, aggregate disclosures |
| [selective-disclosure/security.md](selective-disclosure/security.md) | Soundness, recipient binding, scope boundary, implementation notes |
| [indexer.md](indexer.md) | Archive data model, ingestion contract, retention, API surface, trust model |
| [sdk/README.md](sdk/README.md) | Why the SDK is load-bearing, terminology, layering, roles |
| [sdk/crypto-core.md](sdk/crypto-core.md) | Client-side obligations for fields, canonicality, sponge, generators, ECDH, blinding accumulation, sampling, address compression |
| [sdk/key-derivation.md](sdk/key-derivation.md) | Root to spending key derivation, signer and raw roots, account discovery |
| [sdk/conformance.md](sdk/conformance.md) | Primitive fixtures, circuit-execution parity, required vectors |
| [sdk/proving.md](sdk/proving.md) | Witness assembly, prover, chain adapter |
| [sdk/wallet.md](sdk/wallet.md) | Holder wallet state, event application, salt freshness, ephemeral scalars, consistency, recovery, spender-side wallet |
| [sdk/auditor-client.md](sdk/auditor-client.md) | Auditor decryption, standing openings, clawback witness |
| [sdk/clients.md](sdk/clients.md) | Disclosure construction and verification, indexer client, hybrid read path |
| [sdk/requirements.md](sdk/requirements.md) | Security requirements, non-functional requirements, conformance and versioning |
| [overview.md](overview.md) | Non-normative user-flows overview |

## Reading order

- **Protocol, end to end:** `protocol/README.md`, `primitives.md`, `system-model.md`, `keys-and-commitments.md`, `wallet-state.md`, `account-state.md`, `operations/`, `auditing.md`, `security.md`, `proof-system.md`, `interface.md`, `domain-separators.md`. Concatenating the files in this order reproduces the specification as a single document.
- **Contract implementer or reviewer:** `protocol/interface.md`, `protocol/operations/`, `protocol/proof-system.md`, `compliance.md`.
- **Wallet or SDK implementer:** `sdk/README.md` through `sdk/requirements.md` in file order, then `indexer.md` and `selective-disclosure/`.
- **Auditor operator:** `protocol/auditing.md`, `sdk/auditor-client.md`, the clawback sections of `compliance.md`.
- **Indexer operator:** `indexer.md`, `protocol/wallet-state.md`.
