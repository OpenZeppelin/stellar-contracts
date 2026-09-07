# Crypto Core

Every requirement in this section is reproduced from `circuits/lib/src/lib.nr`, which is the source of truth wherever this document and the protocol documents disagree.

## Fields and the `q` / `p` notation hazard

Two moduli are in play, and confusing them silently corrupts state (§4.6).

| Modulus | Value | Role | Called elsewhere |
|:--|:--|:--|:--|
| $$r$$ | `0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001` | BN254 scalar field. Noir's `Field`; the host's `Bn254Fr`; Grumpkin **coordinate** field | `FR_MODULUS` |
| $$q$$ | `0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47` | BN254 base field. Grumpkin **scalar** (multiplier) field, i.e. the group order | `FP_MODULUS`, and conventionally $$p$$ in the pairing literature |

$$r < q$$, so every $$\mathbb{F}_r$$ element is already a valid Grumpkin scalar with no reduction, which is why a Noir `Field` can be passed to `multi_scalar_mul` unambiguously.

## Canonicality

A value is a **canonical** $$\mathbb{F}_r$$ representative iff it is a 32-byte big-endian encoding of an integer in $$[0, r)$$.

- Every $$\mathbb{F}_r$$ value the SDK emits — into a payload, an event assertion, a proof input, or persisted state — MUST be canonical.
- The SDK MUST reject a non-canonical value at its own boundary rather than relying on the contract's check (DESIGN.md §2.2). A client that produces non-canonical bytes has already lost byte-uniqueness in the local state that recovery reads from.
- Points are encoded as `BytesN<64>` = $$\text{be}(x) \\| \text{be}(y)$$, a **flat** 64-byte value. The identity $$\mathcal{O}$$ is all 64 bytes zero, and decodes back to the identity.

## Poseidon2 sponge

The sponge construction, its width and rate, the IV placement, the padding rule, and the two- and three-lane forms $$\text{SpongeSqueeze}_2$$ and $$\text{SpongeSqueeze}_3$$ are specified normatively in DESIGN.md §2.5. What follows is what that construction additionally requires of a client.

**Three self-checks are available before any proof is generated.** The absorbed length in both squeeze forms is always 3, so the IV is fixed at $$3 \cdot 2^{64}$$; `lane[0]` is identical to $$\text{poseidon\\\_with\\\_domain}(\delta, [s, \sigma])$$ on the same inputs; and $$\text{SpongeSqueeze}_3(\delta, s, \sigma)[i] = \text{SpongeSqueeze}_2(\delta, s, \sigma)[i]$$ for $$i \in \\{0, 1\\}$$, since the absorb fits one rate-3 block and both forms read the same permutation. An implementation that reproduces all three has the block layout and the IV lane right. The third is pinned by `circuits/lib/testdata/sponge_squeeze_3.json`.

**The domain-tagged funnel.** Every Poseidon2 invocation in the protocol routes through one entry point that places the domain tag as the **first absorbed element**:

$$\text{poseidon\\\_with\\\_domain}(\delta, [x_1, \ldots, x_n]) = \text{sponge}([\delta, x_1, \ldots, x_n])$$

Squeeze-slot assignment is canonical and MUST be followed: `lane[0]` is always an amount mask, `lane[1]` is always a balance, allowance, or per-transfer-randomness mask, and `lane[2]` is always the sender-auditor blinding-escrow slot — the new spendable blinding on `Withdraw`, `Transfer`, and `SetSpender`, the new allowance blinding $$r_a'$$ on `SpenderTransfer` (DESIGN.md §2.5). Only the sender-auditor channel ($$\delta_{\text{aud\\\_s}}$$) is squeezed three-wide; the recipient channel ($$\delta_{\text{aud\\\_r}}$$) stays at two lanes. `Withdraw`, whose amount is public, takes `lane[1]` and `lane[2]` and leaves `lane[0]` unused (DESIGN.md W_a3–W_a5), so a checkpoint pad can never coincide with an amount pad.

## Generators and commitments

$$G$$ and $$H$$ are Barretenberg's `derive_generators("DEFAULT_DOMAIN_SEPARATOR")` outputs at indices 0 and 1, fixed in DESIGN_cont.md §10.4.

$$\text{commit}(v, r) = v \cdot G + r \cdot H$$

Scalar multiplication MUST map a zero scalar to the identity rather than erroring, because a registered account's opening commitments are the identity and a zero blinding factor is legitimate (deposits commit with $$r = 0$$, DESIGN.md §7.3).

## ECDH

$$\text{ECDH}(a, B) = \text{poseidon\\\_with\\\_domain}(\delta_{\text{ecdh}}, [S.x, S.y]) \quad \text{where} \quad S = a \cdot B$$

**Both coordinates MUST be absorbed.** An x-only extraction is negation-invariant: $$P$$ and $$-P$$ share an x-coordinate, and $$-\text{PVK} = (-vk) \cdot H$$ is itself a valid canonical registration, so an x-only map would collapse each $$(vk, -vk)$$ pair onto one shared secret (DESIGN.md §2.4). The absorb fills exactly one rate-3 block.

The derivation MUST fail rather than proceed if $$S$$ is the identity: with $$\sigma$$ public, an identity shared secret makes every derived ciphertext trivially decryptable, which is why the circuits carry explicit nonzero-scalar constraints (DESIGN_cont.md §10.8).

## Blinding accumulation

Commitment blinding factors compose under homomorphic point addition, so their arithmetic is that of the **Grumpkin scalar field** $$\mathbb{F}_q$$ (§4.1):

$$\text{Com}(v_1, r_1) + \text{Com}(v_2, r_2) = \text{Com}(v_1 + v_2, \\, (r_1 + r_2) \bmod q)$$

Reducing modulo $$r$$ instead yields an opening that is off by $$q - r$$ and no longer matches the on-chain point, and for two full-size blindings the integer sum crosses $$q$$ roughly half the time. Implementations MUST provide distinct, clearly named reduction operations for the two moduli.

**One reduction point.** An implementation MUST fold blindings with unbounded integer addition and MUST NOT reduce by either modulus as folds compose. This governs merge (DESIGN.md §7.4), the `RevokeSpender` and `Clawback` folds, and receiving-balance credit (DESIGN.md §5.2 *Update rules*) alike.

An implementation MUST reduce modulo $$q$$ to the canonical representative only where a blinding leaves an accumulator for the curve: a proof witness, or the recommit of the §10.6 consistency check. §10.7 tests for encodability at that same point.

Committed **values** accumulate as exact integers and are never reduced; DESIGN.md §2.3 establishes that they never wrap.

## Scalar sampling

Secret scalars — $$\sigma$$, $$\sigma_a$$, the replacement $$\sigma_a'$$ of a spender transfer (DESIGN.md §6.2 *Transfer nonce*), and the disclosure layer's $$r_{\text{disc}}$$ (SELECTIVE_DISCLOSURE.md §4) — MUST be produced by the rejection procedure of DESIGN.md §2.2:

1. Draw 32 bytes from a CSPRNG.
2. Clear the top **2** bits, yielding a 254-bit candidate.
3. Reject and redraw if the candidate is $$\geq r$$, or if it is zero and the call site requires nonzero.

## Domain separators

| Tag | Value | Absorbed in a core circuit? |
|:--|:--:|:--|
| $$\delta_{\text{addr}}$$ | 1 | No — absorbed on-chain by the contract (DESIGN.md §2.7) |
| $$\delta_{\text{vk}}$$ | 2 | Yes |
| $$\delta_{\text{dvk}}$$ | 3 | Yes |
| $$\delta_{\text{spend\\\_r}}$$ | 4 | Yes |
| $$\delta_{\text{transfer\\\_blind}}$$ | 5 | Yes |
| $$\delta_{\text{transfer\\\_amount}}$$ | 6 | Yes |
| $$\delta_{\text{enc\\\_bal}}$$ | 7 | Yes |
| $$\delta_{\text{enc\\\_allow}}$$ | 8 | Yes |
| $$\delta_{\text{allow\\\_r}}$$ | 9 | Yes |
| $$\delta_{\text{esc\\\_dvk}}$$ | 10 | Yes |
| $$\delta_{\text{aud\\\_s}}$$ | 11 | Yes |
| $$\delta_{\text{aud\\\_r}}$$ | 12 | Yes |
| $$\delta_{\text{ecdh}}$$ | 13 | Yes |
| $$\delta_{\text{eph}}$$ | 14 | No — derived off-circuit (DESIGN.md §5.3) |
| $$\delta_{\text{disc\\\_bind}}$$ | 15 | No — off-chain disclosure only |
| $$\delta_{\text{disc}}$$ | 16 | No — off-chain disclosure only |
| $$\delta_{\text{esc\\\_allow\\\_r\\\_aud}}$$ | 17 | Yes |

DESIGN_cont.md §13 assigns all seventeen values and is their only source; the right-hand column is this document's addition. $$\delta_{\text{disc\\\_bind}}$$ and $$\delta_{\text{disc}}$$ belong to the off-chain disclosure layer (SELECTIVE_DISCLOSURE.md §2.2). Tag 1 is absorbed by the contract rather than by a circuit — the contract derives $$\text{addr\\\_f}$$ and $$\text{op}_i$$ on-chain and the circuits receive them as opaque public inputs (DESIGN.md §2.7 *Usage sites*) — so it is part of the on-chain wire contract all the same. None of 14–16 is absorbed either in a circuit or on-chain, so none is part of the on-chain wire contract, but all three are part of the cross-client contract because two wallets serving the same account must agree on them (§6.3).

All seventeen values MUST be distinct, and each MUST be used in exactly one sponge mode, per DESIGN.md §2.5 *Mode exclusivity*. Tags 11 and 12 are the multi-lane tags — 11 read three-wide wherever the sender / owner channel is instantiated, 12 always two-wide; the remaining fifteen, including 1, 14–16, and 17, are single-output tags. Tag 17 is absorbed only by the `SetSpender` circuit (DESIGN.md S14, DESIGN_cont.md §8.5); its fixture is `circuits/lib/testdata/encrypt_esc_allow_r_auditor.json`.

## Address compression

$$\text{address\\\_to\\\_field}(a) = \text{poseidon\\\_with\\\_domain}(\delta_{\text{addr}}, [\text{lo}(a), \text{hi}(a)])$$

where $$\text{enc}(a)$$ is the 56-character ASCII strkey (SEP-23), and $$\text{lo}$$ and $$\text{hi}$$ interpret its lower and upper 28 bytes respectively in **little-endian** order (DESIGN.md §2.7). Implementations MUST obtain the strkey from their language's stellar-strkey library.

**Bootstrap check.** On first contact with a deployment, an implementation MAY compute $$\text{addr\\\_f}$$ for the contract's own address and assert equality against the value the contract stores in instance storage (DESIGN.md §3.5).
