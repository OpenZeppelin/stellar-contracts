# Crypto Core

Every requirement in this section is reproduced from `circuits/lib/src/lib.nr`, which is the source of truth wherever this document and the protocol documents disagree.

## Fields and the `q` / `p` notation hazard

Two moduli are in play, and confusing them silently corrupts state ([Blinding accumulation](#blinding-accumulation)).

| Modulus | Value | Role | Called elsewhere |
|:--|:--|:--|:--|
| $$r$$ | `0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001` | BN254 scalar field. Noir's `Field`; the host's `Bn254Fr`; Grumpkin **coordinate** field | `FR_MODULUS` |
| $$q$$ | `0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47` | BN254 base field. Grumpkin **scalar** (multiplier) field, i.e. the group order | `FP_MODULUS`, and conventionally $$p$$ in the pairing literature |

$$r < q$$, so every $$\mathbb{F}\_r$$ element is already a valid Grumpkin scalar with no reduction, which is why a Noir `Field` can be passed to `multi_scalar_mul` unambiguously.

## Canonicality

A value is a **canonical** $$\mathbb{F}\_r$$ representative iff it is a 32-byte big-endian encoding of an integer in $$[0, r)$$.

- Every $$\mathbb{F}\_r$$ value the SDK emits — into a payload, an event assertion, a proof input, or persisted state — MUST be canonical.
- The SDK MUST reject a non-canonical value at its own boundary rather than relying on the contract's check ([Grumpkin-BN254 Cycle](../protocol/primitives.md#grumpkin-bn254-cycle)). A client that produces non-canonical bytes has already lost byte-uniqueness in the local state that recovery reads from.
- Points are encoded as `BytesN<64>` = $$\text{be}(x) \\| \text{be}(y)$$, a **flat** 64-byte value. The identity $$\mathcal{O}$$ is all 64 bytes zero, and decodes back to the identity.

## Poseidon2 sponge

The sponge construction, its width and rate, the IV placement, the padding rule, and the two- and three-lane forms $$\text{SpongeSqueeze}\_2$$ and $$\text{SpongeSqueeze}\_3$$ are specified normatively in [Poseidon2 Hash](../protocol/primitives.md#poseidon2-hash). What follows is what that construction additionally requires of a client.

**Three self-checks are available before any proof is generated.** The absorbed length in both squeeze forms is always 3, so the IV is fixed at $$3 \cdot 2^{64}$$; `lane[0]` is identical to $$\text{poseidon\\\_with\\\_domain}(\delta, [s, \sigma])$$ on the same inputs; and $$\text{SpongeSqueeze}\_3(\delta, s, \sigma)[i] = \text{SpongeSqueeze}\_2(\delta, s, \sigma)[i]$$ for $$i \in \\{0, 1\\}$$, since the absorb fits one rate-3 block and both forms read the same permutation. An implementation that reproduces all three has the block layout and the IV lane right. The third is pinned by `circuits/lib/testdata/sponge_squeeze_3.json`.

**The domain-tagged funnel.** Every Poseidon2 invocation in the protocol routes through one entry point that places the domain tag as the **first absorbed element**:

$$\text{poseidon\\\_with\\\_domain}(\delta, [x\_1, \ldots, x\_n]) = \text{sponge}([\delta, x\_1, \ldots, x\_n])$$

Squeeze-slot assignment is canonical and MUST be followed as [Lane assignment](../protocol/primitives.md#lane-assignment) fixes it, and each channel MUST be read at exactly the width its tag is assigned ([Mode exclusivity](../protocol/primitives.md#mode-exclusivity)).

## Generators and commitments

$$G$$ and $$H$$ are Barretenberg's `derive_generators("DEFAULT_DOMAIN_SEPARATOR")` outputs at indices 0 and 1, fixed in [Noir Primitives](../protocol/proof-system.md#noir-primitives).

$$\text{commit}(v, r) = v \cdot G + r \cdot H$$

Scalar multiplication MUST map a zero scalar to the identity rather than erroring, because a registered account's opening commitments are the identity and a zero blinding factor is legitimate (deposits commit with $$r = 0$$, [Deposit](../protocol/operations/deposit.md)).

## ECDH

$$\text{ECDH}(a, B) = \text{poseidon\\\_with\\\_domain}(\delta\_{\text{ecdh}}, [S.x, S.y]) \quad \text{where} \quad S = a \cdot B$$

**Both coordinates MUST be absorbed.** An x-only extraction is negation-invariant: $$P$$ and $$-P$$ share an x-coordinate, and $$-\text{PVK} = (-vk) \cdot H$$ is itself a valid canonical registration, so an x-only map would collapse each $$(vk, -vk)$$ pair onto one shared secret ([Elliptic Curve Diffie-Hellman](../protocol/primitives.md#elliptic-curve-diffie-hellman)). The absorb fills exactly one rate-3 block.

The derivation MUST fail rather than proceed if $$S$$ is the identity: with $$\sigma$$ public, an identity shared secret makes every derived ciphertext trivially decryptable, which is why the circuits carry explicit nonzero-scalar constraints ([On-Chain Point Arithmetic](../protocol/proof-system.md#on-chain-point-arithmetic)).

## Blinding accumulation

Commitment blinding factors compose under homomorphic point addition, so their arithmetic is that of the **Grumpkin scalar field** $$\mathbb{F}\_q$$ ([Fields and the `q` / `p` notation hazard](#fields-and-the-q--p-notation-hazard)):

$$\text{Com}(v\_1, r\_1) + \text{Com}(v\_2, r\_2) = \text{Com}(v\_1 + v\_2, \\, (r\_1 + r\_2) \bmod q)$$

Reducing modulo $$r$$ instead yields an opening that is off and no longer matches the on-chain point, and for two full-size blindings the integer sum crosses $$q$$ roughly half the time. Implementations MUST provide distinct, clearly named reduction operations for the two moduli.

**One reduction point.** An implementation MUST fold blindings with unbounded integer addition and MUST NOT reduce by either modulus as folds compose. This governs merge ([Merge](../protocol/operations/merge.md)), the `RevokeSpender` and `Clawback` folds, and receiving-balance credit ([Update rules](../protocol/wallet-state.md#update-rules)) alike.

An implementation MUST reduce modulo $$q$$ to the canonical representative only where a blinding leaves an accumulator for the curve: a proof witness, or the recommit of the [Consistency checking](wallet.md#consistency-checking) consistency check. [The unspendable-blinding case](wallet.md#the-unspendable-blinding-case) tests for encodability at that same point.

Committed **values** accumulate as exact integers and are never reduced; [Pedersen Commitments](../protocol/primitives.md#pedersen-commitments) establishes that they never wrap.

## Scalar sampling

Sampled scalars — $$\sigma$$, $$\sigma\_a$$, the replacement $$\sigma\_a'$$ of a spender transfer ([Transfer nonce](../protocol/account-state.md#transfer-nonce)), and the disclosure layer's $$r\_{\text{disc}}$$ ([Disclosure Ciphertext to Recipient](../selective-disclosure/protocol.md#disclosure-ciphertext-to-recipient)) — MUST be produced by the rejection procedure of [Grumpkin-BN254 Cycle](../protocol/primitives.md#grumpkin-bn254-cycle):

1. Draw 32 bytes from a CSPRNG.
2. Clear the top **2** bits, yielding a 254-bit candidate.
3. Reject and redraw if the candidate is $$\geq r$$, or if it is zero and the call site requires nonzero.

The requirement is unpredictability and uniform distribution, not confidentiality. The protocol emits $$\sigma$$, $$\sigma\_a$$, and $$\sigma\_a'$$ in events and stores $$\sigma\_a$$ and $$\sigma\_a'$$ on-chain as `allowance_salt` ([Event application](wallet.md#event-application), [`allowance_salt`](../protocol/account-state.md#allowance_salt)), and a wallet MAY persist any of them for [Recovery](wallet.md#recovery).

## Domain separators

The seventeen tag values, the layer that absorbs each, and the sponge mode each is confined to are fixed in [Domain Separation Constants](../protocol/domain-separators.md). An implementation MUST hardcode those values and MUST NOT read any tag at a second arity ([Mode exclusivity](../protocol/primitives.md#mode-exclusivity)).

## Address compression

$$\text{address\\\_to\\\_field}(a) = \text{poseidon\\\_with\\\_domain}(\delta\_{\text{addr}}, [\text{lo}(a), \text{hi}(a)])$$

where $$\text{enc}(a)$$ is the 56-character ASCII strkey (SEP-23), and $$\text{lo}$$ and $$\text{hi}$$ interpret its lower and upper 28 bytes respectively in **little-endian** order ([Address-to-Field Encoding](../protocol/primitives.md#address-to-field-encoding)). Implementations MUST obtain the strkey from their language's stellar-strkey library.

**Bootstrap check.** On first contact with a deployment, an implementation MAY compute $$\text{addr\\\_f}$$ for the contract's own address and assert equality against the value the contract stores in instance storage ([Governance and Upgradeability](../protocol/system-model.md#governance-and-upgradeability)).

---

Previous: [SDK](README.md) · Up: [Index](../README.md#companions) · Next: [Key Derivation](key-derivation.md)
