# Conformance Vectors

## Primitive fixtures

`circuits/lib/testdata/*.json` pins one primitive's output per file for a fixed input set, and is the language-agnostic contract for every off-chain consumer. An implementation MUST reproduce every output in every file byte-for-byte.

Its test suite MUST **read** those files rather than transcribe their values into source, so that a change to the Noir library's `print_fixtures` output becomes a test failure rather than a silent divergence.

## Circuit-execution parity

Fixtures pin primitives, not witnesses. A client can reproduce every fixture and still assemble a witness with a transposed public input, a wrong field name, or a stale ordering, none of which any fixture covers.

An implementation MUST therefore additionally, for every circuit it supports:

- Build the witness from its own crypto core and have the **real compiled circuit** solve it, asserting success.
- Include **tamper cases** — a witness with one value perturbed — and assert the circuit *rejects* them.

## Vectors this specification requires

Three derivations this document specifies or relies on are computed outside the core circuits, so the Noir library's `print_fixtures` does not emit them. The first already has a fixture; the other two require one:

- **$$\text{address\\\_to\\\_field}$$** (§4.9) is pinned by `circuits/lib/testdata/address_to_field.json`, which §6.1's obligation already covers. The circuits receive $$\text{addr\\\_f}$$ as an opaque public input, so this derivation is implemented twice — by the contract on-chain and by every client — making it the sole primitive with two independent implementations and no Noir version, and §4.9's bootstrap check detects a divergence only against an already-deployed contract. Being outside `print_fixtures`, it is also outside the in-Noir `fixtures_match_testdata` guard; the Rust test `address_to_field_matches_testdata_vectors` guards it instead (`circuits/lib/testdata/README.md`).
- **$$\delta_{\text{eph}}$$ derivation** (DESIGN.md §5.3, restated in §10.5). No circuit constrains $$r_e$$, so a fixture is the only mechanism keeping a user's clients in agreement; where two disagree, transfers sent from one are not disclosable from the other.
- **The §5.1 $$sk$$ chain**, from a fixed root, $$\text{addr\\\_f}$$, and $$\text{acct\\\_f}$$ through to $$sk$$, $$vk$$, $$Y$$, and $$\text{PVK}$$, without which recovery from backup material is untestable across implementations. The vector MUST start from a fixed ed25519 secret and run through §5.2's message, its SEP-0053 preimage, and the resulting signature, since the signature is where two clients most plausibly diverge — the only step in the chain whose format is set outside this document. A second vector from a fixed raw root covers §5.3.
