# Domain Separation Constants

Each $$\delta$$ is a small positive integer in $$\mathbb{F}\_r$$, fixed for the protocol version and used as a Poseidon2 leading-input domain tag. Numeric values are assigned sequentially from 1; the protocol version is implicit in the deployment, and any change to a circuit's constraint that uses these tags requires a new deployment with a fresh verification key (§3.5, §10.6).

| $$\delta$$ | Value | Context |
|:---|:---:|:---|
| $$\delta\_{\text{addr}}$$ | 1 | Soroban Address compression into a single $$\mathbb{F}\_r$$ Field (§2.7) |
| $$\delta\_{\text{vk}}$$ | 2 | Viewing key derivation from spending key and contract address (§4.2) |
| $$\delta\_{\text{dvk}}$$ | 3 | Delegation viewing key derivation (§4.4) |
| $$\delta\_{\text{spend\\\_r}}$$ | 4 | Deterministic randomness for spendable balance commitments (§5.2 *Update rules*) |
| $$\delta\_{\text{transfer\\\_blind}}$$ | 5 | ECDH-derived transfer blinding factor (§5.3 Definition 1) |
| $$\delta\_{\text{transfer\\\_amount}}$$ | 6 | ECDH-derived transfer amount encryption (§5.3 Definition 1) |
| $$\delta\_{\text{enc\\\_bal}}$$ | 7 | Encrypted balance scalar masking (§5.5) |
| $$\delta\_{\text{enc\\\_allow}}$$ | 8 | Encrypted allowance scalar masking (§6.2 *a\_tilde*) |
| $$\delta\_{\text{allow\\\_r}}$$ | 9 | Deterministic randomness for spender allowance commitments (§6.2 *allowance\_commitment*) |
| $$\delta\_{\text{esc\\\_dvk}}$$ | 10 | Delegation key escrow (spender ECDH) (§7.11) |
| $$\delta\_{\text{aud\\\_s}}$$ | 11 | Sender / owner-auditor channel sponge (§2.5, §8.1) |
| $$\delta\_{\text{aud\\\_r}}$$ | 12 | Recipient-auditor channel sponge (§2.5, §8.1) |
| $$\delta\_{\text{ecdh}}$$ | 13 | ECDH shared-secret scalar extraction (§2.4) |
| $$\delta\_{\text{eph}}$$ | 14 | Deterministic ephemeral-scalar derivation (§5.3) |
| $$\delta\_{\text{disc\\\_bind}}$$ | 15 | Disclosure ciphertext to the disclosure recipient, aggregate variant ([SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §10) |
| $$\delta\_{\text{disc}}$$ | 16 | Disclosure ciphertext to the disclosure recipient ([SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §4) |
| $$\delta\_{\text{esc\\\_allow\\\_r\\\_aud}}$$ | 17 | Allowance-blinding escrow to the owner's auditor (S14, §8.5) |

This table assigns all seventeen values; no other document assigns them. Tags 14–16 are never absorbed inside a core circuit — 14 is derived off-circuit (DESIGN.md §5.3 makes its derivation normative for every operation whose originator holds a viewing key), and 15–16 belong to the off-chain selective-disclosure layer ([SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §2.2) — so they are not part of the on-chain wire contract; `circuits/lib/src/lib.nr` accordingly implements 1–13 and 17. All seventeen values MUST still be distinct and each MUST be confined to a single sponge mode, so a deployment treats them as one namespace. Tag 17's construction is specified in §8.5.

**Provenance.** Sequential small integers are the simplest assignment that satisfies the requirement of *distinctness* across all Poseidon2 invocations in this protocol -- §3.2 models Poseidon2 as a pseudorandom function, so evaluations whose leading input differs are computationally independent. Distinctness alone is not sufficient: each tag must also be confined to a single sponge mode, since the multi-lane forms of §2.5 share `lane[0]` with the single-output form on the same inputs; $$\delta\_{\text{aud\\\_s}}$$ is the one tag ever squeezed three-wide, and the widths it is read at agree on their shared lanes (§2.5 *Mode exclusivity*). The values themselves carry no semantic meaning; the binding is purely positional and the table is the only authoritative source. Implementations MUST hardcode these exact numeric values.

**Cross-protocol collision.** Future protocols that share Grumpkin / BN254 / Poseidon2 with this protocol -- e.g. an unrelated payments protocol that uses small-integer Poseidon2 domains -- could in principle pick the same numeric values for unrelated purposes. The protocol assumes that the surrounding inputs to Poseidon2 (key material, structural witnesses) sufficiently disambiguate even in such a case; no Poseidon2 invocation in this protocol is keyed solely on a $$\delta$$ value. If stronger isolation is desired, implementers may instead use the alternate scheme $$\delta\_X = \text{Poseidon2}(0, \text{ASCII}(\text{"openzeppelin/confidential-token/v1:X"}))$$, but this is a deployment-time choice that must be applied uniformly and disclosed in the deployment's circuit-binding documentation.
