# Primitives

## Notation

| Symbol | Definition |
|:---|:---|
| $$\mathbb{G}$$ | Grumpkin elliptic curve group (prime order) |
| $$\mathbb{F}_r$$ | BN254 scalar field $$= \mathbb{G}$$'s base field |
| $$\mathbb{F}_q$$ | BN254 base field $$= \mathbb{G}$$'s scalar field |
| $$G, H \in \mathbb{G}$$ | Independent generators with no known discrete log relation |
| $$\mathcal{O}$$ | Identity element (point at infinity), encoded as $$(0, 0)$$ on-chain |
| $$P.x$$ | The $$x$$-coordinate of point $$P$$, an element of $$\mathbb{F}_r$$ |
| $$\text{Poseidon}(\cdot)$$ | Poseidon2 hash function over $$\mathbb{F}_r$$ ([Poseidon2 Hash](#poseidon2-hash)) |
| $$\delta_{\ast}$$ | Domain separation constants (subscript identifies the domain) |
| $$[n]$$ | The set $$\\{0, 1, \ldots, n-1\\}$$ |

## Grumpkin-BN254 Cycle

Grumpkin is defined by $$y^2 = x^3 - 17$$ over $$\mathbb{F}_r$$. It forms a 2-cycle with BN254:

$$\text{base}(\mathbb{G}) = \mathbb{F}_r^{\text{BN254}}, \qquad \text{scalar}(\mathbb{G}) = \mathbb{F}_q^{\text{BN254}}$$

A Grumpkin point is a pair $$(x, y) \in \mathbb{F}_r^2$$. Noir's native `Field` type is $$\mathbb{F}_r$$, so Grumpkin point arithmetic inside UltraHonk circuits incurs no non-native field emulation. On-chain, the Soroban host provides BN254 $$\mathbb{F}_r$$ arithmetic (`bn254_fr_{add, sub, mul, inv}` via CAP-80), which suffices for Grumpkin affine point operations.

**Scalar sampling.** Grumpkin scalars live in $$\mathbb{F}_q$$, which is slightly larger than $$\mathbb{F}_r$$. Every secret scalar in the core protocol that is drawn rather than derived ($$\sigma$$, $$\sigma_a$$) is produced by the **rejection sampling** procedure, which yields a uniform draw from $$\mathbb{F}_r$$; the extension layers draw one further scalar the same way, $$r_{\text{disc}}$$ ([Disclosure Ciphertext to Recipient](../selective-disclosure/protocol.md#disclosure-ciphertext-to-recipient)).

1. Draw 32 bytes (256 bits) from a CSPRNG.
2. Mask the top 2 bits to zero, yielding a 254-bit candidate $$x \in [0, 2^{254})$$.
3. If $$x \geq r$$, reject and return to step 1.
4. If the call site requires $$x \neq 0$$ and $$x = 0$$, reject and return to step 1.
5. Output $$x$$ in its canonical form -- as a Noir `Field` for in-circuit use, or as 32 big-endian bytes (`BytesN<32>`) for storage and event emission.

### Host deserialiser caveat

The Soroban host's `bn254_fr_from_u256val` (the underlying primitive of `Bn254Fr::from_bytes`) accepts any 32-byte representative and *silently reduces* values $$x \geq r$$ modulo $$r$$ rather than rejecting them. Two distinct byte strings ($$x$$ and $$x + r$$) therefore deserialise to the same $$\mathbb{F}_r$$ element, which means a verifier alone cannot distinguish canonical from non-canonical inputs. To keep stored state and emitted events byte-unique per logical value, the contract layer enforces canonicality on every prover-supplied $$\mathbb{F}_r$$ representative *before* the bytes reach the verifier.

## Pedersen Commitments

A Pedersen commitment to a value $$v$$ with a blinding factor $$r$$, viewed as scalars in Grumpkin's scalar field $$\mathbb{F}_q$$, is:

$$\text{Com}(v, r) = v \cdot G + r \cdot H$$

In this design both $$v$$ and $$r$$ are drawn from $$\mathbb{F}_r \subset \mathbb{F}_q$$ ([Grumpkin-BN254 Cycle](#grumpkin-bn254-cycle)): $$v$$ is a non-negative integer below $$2^{127}$$ ([Integer Embedding and Range Proofs](#integer-embedding-and-range-proofs)) and $$r$$ is a Poseidon2 output or an $$\mathbb{F}_r$$-sampled CSPRNG draw; the group law operates in $$\mathbb{F}_q$$.

**Binding.** Finding $$(v', r') \neq (v, r)$$ such that $$\text{Com}(v, r) = \text{Com}(v', r')$$ requires computing $$\log_G H$$, which is infeasible under the discrete logarithm assumption.

**Hiding.** For any $$v$$, the commitment $$\text{Com}(v, r)$$ with uniformly random $$r \in \mathbb{F}_q$$ is uniformly distributed over $$\mathbb{G}$$, revealing nothing about $$v$$. Sampling $$r$$ from $$\mathbb{F}_r \subset \mathbb{F}_q$$ instead of full $$\mathbb{F}_q$$ ([Grumpkin-BN254 Cycle](#grumpkin-bn254-cycle)) makes the commitment distribution **statistically close** to uniform over $$\mathbb{G}$$, with total-variation distance bounded by $$(|\mathbb{F}_q| - |\mathbb{F}_r|)/|\mathbb{F}_q| \approx 2^{-127}$$.

**Homomorphism.** $$\text{Com}(v_1, r_1) + \text{Com}(v_2, r_2) = \text{Com}(v_1 + v_2, r_1 + r_2)$$. Scalar addition in the commitment relation is over $$\mathbb{F}_q^{\text{BN254}}$$ -- the scalar field of $$\mathbb{G}$$, equivalently the order of the Grumpkin group. Since every committed value is bounded by $$2^{127}$$ ([Integer Embedding and Range Proofs](#integer-embedding-and-range-proofs)) and the number of additions across the lifetime of any one commitment is far below $$2^{127}$$, the value component never wraps in $$\mathbb{F}_q$$ and the homomorphic relation holds in $$\mathbb{Z}$$ for values. The blinding component is added in $$\mathbb{F}_q$$ and may reduce mod $$q$$ on accumulation; the only place this has operational consequences is the wallet's post-merge spend witness (see [Post-merge witness availability](proof-system.md#post-merge-witness-availability)).

**Generators.** $$G$$ and $$H$$ are inherited from Barretenberg's standard Grumpkin Pedersen instantiation (the same generators that the toolchain's `pedersen_commitment` and `pedersen_hash` primitives use). Their provenance is part of the toolchain's audited surface, so the contract inherits both the generators and the soundness assumption that $$\log_G H$$ is unknown. Concretely they are `derive_generators("DEFAULT_DOMAIN_SEPARATOR")` at indices 0 and 1; the values are listed in [Noir Primitives](proof-system.md#noir-primitives).

## Elliptic Curve Diffie-Hellman

Given a long-term keypair $$(a, A = a \cdot H)$$ and an ephemeral keypair $$(r_e, R_e = r_e \cdot H)$$, the ECDH shared secret is:

$$S = r_e \cdot A = a \cdot R_e = a \cdot r_e \cdot H \in \mathbb{G}$$

Both parties compute $$S$$ independently. The scalar shared secret used as a Poseidon input binds both coordinates of $$S$$:

$$s = \text{Poseidon}(\delta_{\text{ecdh}}, S.x, S.y) \in \mathbb{F}_r$$

Throughout this document, $$\text{ECDH}(a, B)$$ denotes this derivation with $$S = a \cdot B$$.

**Why the y-coordinate is bound.** A point and its negation share an x-coordinate ($$-P = (P.x, -P.y)$$), so an x-only extraction $$s = S.x$$ would satisfy $$\text{ECDH}(r_e, P) = \text{ECDH}(r_e, -P)$$ for every scalar. The negated key is reachable: $$-\text{PVK} = (-vk) \cdot H$$ is itself a valid, canonical, on-curve registration (with secret $$-vk$$), so an x-only map from registered keys to shared secrets would be two-to-one, collapsing each $$(vk, -vk)$$ pair onto one shared secret. Absorbing $$S.y$$ into the Poseidon funnel binds the full shared point. The y-coordinate is consumed *inside* the derivation and never carried through any downstream formula, event, or storage encoding; the absorb $$(\delta_{\text{ecdh}}, S.x, S.y)$$ fills exactly one rate-3 block, so the binding costs a single Poseidon2 permutation.

## Poseidon2 Hash

The system uses **Poseidon2**, the algebraic hash function native to Noir's standard library and implemented as a custom gate in Barretenberg. For the complete parameter specification of the Noir/Barretenberg instantiation, see:

- [Poseidon2 paper](https://eprint.iacr.org/2023/323) (Grassi, Khovratovich, Schofnegger, AFRICACRYPT 2023) - parameter derivation and security analysis
- [Barretenberg `poseidon2_params.hpp`](https://github.com/AztecProtocol/aztec-packages/blob/next/barretenberg/cpp/src/barretenberg/crypto/poseidon2/poseidon2_params.hpp) - concrete round constants and matrix entries
- [HorizenLabs reference implementation](https://github.com/HorizenLabs/poseidon2) - parameter generation script (`poseidon2_rust_params.sage`)
- [Noir stdlib `hash/mod.nr`](https://github.com/noir-lang/noir/blob/master/noir_stdlib/src/hash/mod.nr) - sponge construction wrapping the `Poseidon2Permutation` ACIR opcode

**Sponge construction.** Every Poseidon2 value in this specification is produced by the following sponge over $$\mathbb{F}_r$$ at width $$t = 4$$, rate 3, capacity 1.

1. Let $$M$$ be the number of absorbed field elements, counting the leading domain tag. Set $$\text{iv} = M \cdot 2^{64}$$.
2. Initialise the state to $$[0, 0, 0, \text{iv}]$$, placing the IV in the capacity lane $$\text{state}[3]$$.
3. For each full block of three inputs, **add** them into $$\text{state}[0], \text{state}[1], \text{state}[2]$$ -- addition in $$\mathbb{F}_r$$, not assignment -- then apply the permutation.
4. If $$M$$ is not a multiple of three, add the remaining inputs into $$\text{state}[0]$$ upward and apply one further permutation. This trailing permutation is also applied when $$M = 0$$, so the empty input hashes to $$\text{permute}([0,0,0,0])[0]$$ rather than to zero.
5. Output $$\text{state}[0]$$.

The domain tag is always the first element absorbed, so $$\text{Poseidon2}(\delta, x_1, \ldots, x_n)$$ throughout this document denotes this sponge applied to $$(\delta, x_1, \ldots, x_n)$$.

**Usage in this system:**

- Key derivation: $$vk = \text{Poseidon2}(\delta_{\text{vk}}, sk, \text{addr\\\_f})$$
- Randomness derivation: $$r = \text{Poseidon2}(\delta_{\text{spend\\\_r}}, vk, \sigma)$$
- Symmetric encryption: $$\tilde{v} = v + \text{Poseidon2}(\delta_{\text{transfer\\\_amount}}, s, \sigma)$$
- Domain separation: each invocation includes a leading constant $$\delta$$ to prevent cross-context collisions

**Multi-lane mode for auditor channels.** The per-transfer auditor ciphertexts ([Auditing](auditing.md)) need several masks from one absorb. Since $$(\delta_{\text{channel}}, s, \sigma)$$ is exactly one rate-3 block, the masks are taken as **lanes of a single permutation output**, not as sequential squeezes:

$$\text{SpongeSqueeze}_n(\delta_{\text{channel}}, s, \sigma) = \bigl(\text{state}[0], \\, \ldots, \\, \text{state}[n-1]\bigr), \qquad \text{state} = \text{permute}\bigl([\delta_{\text{channel}}, \\, s, \\, \sigma, \\, 3 \cdot 2^{64}]\bigr)$$

where $$s$$ is the ECDH shared scalar of [Elliptic Curve Diffie-Hellman](#elliptic-curve-diffie-hellman) and $$n \in \\{2, 3\\}$$ is the number of rate lanes read; the capacity lane $$\text{state}[3]$$ is never squeezed. Two channel tags are used: $$\delta_{\text{aud\\\_s}}$$ for the sender-auditor channel keyed by $$s_{a,s} = \text{ECDH}(r_e, K_{\text{aud,s}})$$, squeezed three-wide, and $$\delta_{\text{aud\\\_r}}$$ for the recipient-auditor channel keyed by $$s_{a,r} = \text{ECDH}(r_e, K_{\text{aud,r}})$$, squeezed two-wide. No other arity is instantiated. Because the absorb is one block, every width reads a prefix of one permutation output: $$\text{SpongeSqueeze}_3(\delta, s, \sigma)[i] = \text{SpongeSqueeze}_2(\delta, s, \sigma)[i]$$ for $$i \in \\{0, 1\\}$$, and $$\text{SpongeSqueeze}_n(\delta, s, \sigma)[0] = \text{Poseidon2}(\delta, s, \sigma)$$.

### Lane assignment

Squeeze order is canonical. Lanes are named by their zero-based index into the squeeze output: `lane[i]` is $$\text{SpongeSqueeze}_n(\delta, s, \sigma)[i]$$, so a three-wide squeeze yields `lane[0]`, `lane[1]`, and `lane[2]`. `lane[0]` is always an amount mask and `lane[1]` is always a balance, allowance, or randomness mask, fixed per operation by the formulas in [Operations](operations/README.md) and [Auditing](auditing.md). `lane[2]`, present only on the sender-auditor channel, is the **blinding-escrow slot**. It carries the new spendable blinding on the three checkpoint operations (W\_a5, T\_a9, S\_a6) and the new allowance blinding on spender transfers (O\_a9). The Withdraw checkpoint (W\_a3, W\_a5) takes `lane[1]` and `lane[2]` and leaves the amount lane unused, so a checkpoint pad can never coincide with an amount pad.

### Mode exclusivity

By the prefix property, the single-output form is `lane[0]` of the multi-lane one, so distinct domain tags ([Domain Separation Constants](domain-separators.md)) are not sufficient on their own: each tag MUST additionally be used in exactly one sponge mode, or the same $$(\delta, s, \sigma)$$ would yield one mode's mask as the other's output. $$\delta_{\text{aud\\\_s}}$$ and $$\delta_{\text{aud\\\_r}}$$ are the two multi-lane tags; every other tag in [Domain Separation Constants](domain-separators.md) is used only with the single-output form above.

### Pad freshness

The sponge masks are deterministic in $$(s, \sigma)$$, so reusing the pair across two operations reuses every pad slot they share, and a slot whose plaintext is known in one operation (e.g. a transfer amount known to its recipient) decrypts the other operation's ciphertext in that slot. The canonical slot assignment above limits the blast radius of such reuse to same-slot pairs, but does not eliminate it; provers and wallets MUST use a fresh $$(r_e, \sigma)$$ for every proof. Because $$r_e$$ is derived from the originator's viewing key and the salt rather than drawn independently ([ECDH-Derived Blinding](keys-and-commitments.md#ecdh-derived-blinding)), a fresh salt is the only thing that makes the pair fresh, and [Revert Safety](security.md#revert-safety)'s retry rule is what discharges the requirement. Freshness applies to whichever salt the operation's pads absorb -- $$\sigma$$ for owner-initiated operations, the prover-chosen $$\sigma_a'$$ for spender transfers ([Transfer nonce](account-state.md#transfer-nonce)).

All references to "Poseidon" in this document denote this Poseidon2 instantiation.

## Integer Embedding and Range Proofs

**The problem.** Noir circuits operate over $$\mathbb{F}_r$$, where every element is a non-negative integer modulo $$r \approx 2^{254}$$. The statement $$v \geq 0$$ is vacuously true for all $$v \in \mathbb{F}_r$$, and $$v_A \geq v_{\text{transfer}}$$ is undefined without specifying how integers are embedded in the field. Without explicit range constraints, a prover can claim a balance of 1 and transfer 1,000,000: the "new balance" $$1 - 1{,}000{,}000 \equiv r - 999{,}999 \pmod{r}$$ is a valid field element, and the commitment equation holds. The attacker has minted 999,999 tokens.

**Integer embedding.** We define a canonical embedding $$\iota: [0, 2^{127}) \to \mathbb{F}_r$$ mapping non-negative integers to their natural field representatives. A field element $$x \in \mathbb{F}_r$$ represents a valid balance or transfer amount if and only if $$x < 2^{127}$$.

**Range proof mechanism.** A range proof for $$x \in [0, 2^{127})$$ is implemented by decomposing $$x$$ into 127 bits inside the circuit and checking the recomposition:

$$x = \sum_{i=0}^{126} b_i \cdot 2^i, \qquad b_i \in \\{0, 1\\} \\;\forall\\, i$$

Each $$b_i$$ is constrained to be Boolean ($$b_i \cdot (b_i - 1) = 0$$) and the recomposition is checked against $$x$$. Noir's standard library exposes this directly:

```noir
// Range check: [0, 2^127)
value.assert_max_bit_size::<127>();
```

**Sufficiency argument.** If the prover supplies $$v_A$$ and $$v_{\text{transfer}}$$ such that the circuit verifies:

1. $$v_A \in [0, 2^{127})$$ (the opening of $$C_{\text{spend}}$$)
2. $$v_{\text{transfer}} \in [0, 2^{127})$$ (the transfer amount)
3. $$v_A - v_{\text{transfer}} \in [0, 2^{127})$$ (the new balance)

then $$v_A - v_{\text{transfer}}$$ is a non-negative integer less than $$2^{127}$$, which is only possible if the integer subtraction did not underflow. This is because $$v_A < 2^{127}$$ and $$v_{\text{transfer}} < 2^{127}$$, so if $$v_A < v_{\text{transfer}}$$ as integers, then $$v_A - v_{\text{transfer}} \pmod{r}$$ would be $$r - (v_{\text{transfer}} - v_A)$$, which is at least $$r - 2^{127} \gg 2^{127}$$, failing constraint (3).

**Value capacity.** Both balances and transfer amounts are constrained to $$[0, 2^{127})$$. These bounds are enforced in every circuit that manipulates values. The bound is exactly the SEP-41 non-negative `i128` range, so the contract's value domain matches the underlying token's domain by construction. The gap between $$2^{127}$$ and $$|\mathbb{F}_r| \approx 2^{254}$$ ensures that modular wrap-around is detectable by the range check.

**Receiving balance (unproven accumulation).** The receiving balance $$C_{\text{receive}}$$ is updated by contract-side point addition without any proof from the recipient. Therefore, the receiving balance's committed value $$v_r$$ is never directly range-checked by any circuit.

This is safe because $$v_r$$ is *indirectly* bounded:

1. Each deposit adds a public `i128` amount validated by the contract ($$\ge 0$$, hence $$< 2^{127}$$).
2. Each incoming transfer adds a commitment whose sender circuit proved $$v_{\text{transfer}} \in [0, 2^{127})$$ (constraint T4 / O4).
3. All tokens in the contract entered through deposits, so the sum of all committed values is bounded by the underlying token's total supply ($$< 2^{127}$$). No single account can receive more than the total supply.
4. For the field-arithmetic concern (could $$v_r$$ reach $$r$$ and wrap around), that would require $$r / 2^{127} > 2^{127}$$ incoming transfers, which is computationally infeasible.

When the owner spends after a merge, the spend proof constrains the full post-merge opening: $$v_s + v_r \in [0, 2^{127})$$ (via constraint W4 or T4 on the spendable balance). This provides an implicit range check at the next spend boundary.

## Address-to-Field Encoding

In Soroban, the host's `address_to_strkey` function is defined for the two `ScAddressType` variants the contract interacts with as actors -- `Account` (Stellar ed25519 account) and `Contract` (Soroban contract instance) -- and errors on every other variant the SDK's `Address` type can wrap. The protocol encodes those addresses via their **canonical Stellar strkey** (SEP-23) representation:

$$\text{enc}(a) \\;=\\; \text{Address::to\\\_string}(a)\text{.to\\\_bytes}() \\;\in\\; \\{\text{ASCII}\\}^{56}$$

This is the 56-character ASCII strkey produced by the host's `address_to_strkey` function: a 1-byte SEP-23 version byte (`0x30` for `Account`, `0x10` for `Contract`), a 32-byte payload (ed25519 public key or contract hash), and a 2-byte CRC16 checksum, all base32-encoded into 56 ASCII characters whose leading character is correspondingly `G` (`0x47` in ASCII) or `C` (`0x43`). The byte string is fixed-length, canonical, and reproducible in every Stellar SDK via the language's stellar-strkey library; the protocol commits to these 56 ASCII bytes.

The Poseidon-compressed Field encoding splits the 56-byte string into two 28-byte limbs (each $$\le 2^{224} \ll r \approx 2^{254}$$, hence trivially in $$\mathbb{F}_r$$):

$$\text{address\\\_to\\\_field}(a) \\;=\\; \text{Poseidon2}\big(\delta_{\text{addr}}, \\;\text{lo}(a), \\;\text{hi}(a)\big)$$

where $$\text{lo}(a) = \sum_{i=0}^{27} 256^{\\,i} \cdot \text{enc}(a)[i]$$ and $$\text{hi}(a) = \sum_{i=0}^{27} 256^{\\,i} \cdot \text{enc}(a)[28 + i]$$ interpret the lower and upper 28 bytes of the strkey in little-endian byte order.

The contract, the SDK, the wallet, and any indexer reproduce the same Field value from the same Address by running their language's stellar-strkey encoder over the same `(version, payload)` pair and applying the same limb decomposition. No implementation needs to handle `ScAddress` XDR or the inner `AccountID` / `ContractID` union nesting.

### Usage sites

| Site | When computed | Storage |
|:---|:---|:---|
| $$\text{addr\\\_f}$$ | Once, by the contract's `__constructor` over `env.current_contract_address()` | Stored as a single Field in the contract's **instance storage** ([Governance and Upgradeability](system-model.md#governance-and-upgradeability)); read on every proof verification |
| $$\text{op}_i$$ | Per-call, by the contract at `set_spender` over the `spender` argument | Not stored; recomputed each call. The circuit binds it via S5 and absorbs it as the S12 / S14 escrow nonce. |
