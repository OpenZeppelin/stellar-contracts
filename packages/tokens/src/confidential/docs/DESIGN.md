# Confidential Token

## Abstract

We present a confidential token for Soroban that adds private balances and transfers to any SEP-41 token. Balances are stored as unchunked Pedersen commitments as single elliptic curve points, and updated homomorphically by the contract without decryption. Zero-knowledge proofs (Noir/UltraHonk) accompany each spending operation to prove correctness without revealing amounts. Transfer recipients and auditors recover amounts and blinding factors via per-transfer ephemeral ECDH key agreement over Grumpkin. A dual-balance model (spendable/receiving) prevents griefing: incoming transfers accumulate in a receiving commitment that third parties cannot use to invalidate in-flight spend proofs. A dual-auditor model provides per-account audit visibility: each transfer produces ciphertexts under two auditor keys, giving the recipient's auditor the transfer amount and the sender's auditor the transfer amount plus the sender's post-transfer balance (or post-transfer allowance for spender transfers), enabling real-time auditing. Account owners can delegate spending to time-limited spenders via escrowed allowances with derived delegation viewing keys. The system uses 6 Noir circuits, and works seamlessly with Soroban's BN254 host functions (leveraging the recently added CAP-80), and requires approximately 288 bytes of on-chain storage per account.

---

## Project Documents

This project is composed of the following documents:

- Confidential Token (this document, §1–§7; continued in [DESIGN_cont.md](./DESIGN_cont.md), §8–§13)
- Confidential Token: [Compliance Extensions](./COMPLIANCE.md)
- Confidential Token: [Selective Disclosure](./SELECTIVE_DISCLOSURE.md)
- Confidential Token: [User Flows Overview](./OVERVIEW.md)
- Indexing and Off-Chain State Recovery (to be added)
- SDK (to be added)

---

## 1. Introduction

### 1.1 Background

Confidential transfers on blockchain require balances and amounts to be hidden from public observers while remaining verifiable by the contract. The standard approach uses additively homomorphic encryption - the contract operates on ciphertexts (adding deposits, subtracting transfers) without learning the underlying values, and zero-knowledge proofs guarantee that operations are valid (sufficient funds, consistent encryption, non-negative balances).

This document defines a Soroban contracts suite that provides confidential balances and transfers features on top of SEP-41 tokens. It is not an extension to the fungible token standard; it defines a separate contract that holds tokens on behalf of users and manages encrypted state independently. The wrapping approach is chosen over native token integration for three reasons: it works with existing assets, it can evolve independently of the token standard, and it keeps confidentiality complexity separate from the underlying token.

### 1.2 Design Goals

**Amount and balance confidentiality.** An observer can see that account $$A$$ transferred to account $$B$$, and how much each party deposited or withdrew, but not how much moved between them or what their balances are. The system provides confidentiality, not anonymity - sender and recipient addresses remain visible on-chain.

**Griefing resistance.** A third party must not be able to prevent an account owner from spending by spamming transfers. The balance model must isolate incoming funds from the state that spend proofs reference.

**No mandatory maintenance operations.** Receiving funds should not require the owner to perform a costly ZK proof before those funds become accessible. The merge operation that makes received funds spendable must be lightweight and non-frontrunnable.

**Selective auditing.** Each account selects an auditor at registration. Each transfer produces ciphertexts under two auditor keys: the recipient's auditor receives the transfer amount, while the sender's auditor receives the transfer amount and sender's post-transfer balance. This dual-auditor model enables real-time visibility for both parties' auditors without granting access to uninvolved accounts' historical balances.

**Delegated spending.** Account owners can authorize spenders (separate addresses) to spend from escrowed allowances, enabling use cases like automated market makers and custodial services without sharing the owner's secret key.

### 1.3 Approach

The design is built on three interlocking mechanisms:

1. **Pedersen commitments.** Each balance is a single elliptic curve point $$C = v \cdot G + r \cdot H$$. There is no chunking, no discrete logarithm to solve for decryption, and no overflow from repeated homomorphic additions. The owner maintains the commitment opening $$(v, r)$$ as local wallet state, updated incrementally from on-chain events.

2. **ECDH-derived blinding.** When a sender transfers to a recipient, the blinding factor of the transfer commitment is derived from an ephemeral Diffie-Hellman key exchange with the recipient's public viewing key. The circuit enforces correct derivation, ensuring the recipient can always compute the blinding. The same ephemeral scalar is reused for an ECDH exchange with auditors' public key.

3. **Proof-less merge.** Incoming funds accumulate in a receiving balance that is separate from the spendable balance. To make received funds spendable, the owner authorizes a merge - no ZK proof is required. Since merge requires owner authorization and incoming transfers touch only the receiving balance, neither the spend path nor the merge path can be front-run by a third party.

Six Noir/UltraHonk circuits cover registration, withdrawal, confidential transfer, spender transfer, spender delegation, and spender revocation. The proof system leverages the Grumpkin–BN254 curve cycle: Grumpkin point arithmetic is native inside Noir circuits (no field emulation), while Soroban natively supports BN254 operations for UltraHonk proof verification.

---

## 2. Preliminaries

### 2.1 Notation

| Symbol | Definition |
|:---|:---|
| $$\mathbb{G}$$ | Grumpkin elliptic curve group (prime order) |
| $$\mathbb{F}\_r$$ | BN254 scalar field $$= \mathbb{G}$$'s base field |
| $$\mathbb{F}\_q$$ | BN254 base field $$= \mathbb{G}$$'s scalar field |
| $$G, H \in \mathbb{G}$$ | Independent generators with no known discrete log relation |
| $$\mathcal{O}$$ | Identity element (point at infinity), encoded as $$(0, 0)$$ on-chain |
| $$P.x$$ | The $$x$$-coordinate of point $$P$$, an element of $$\mathbb{F}\_r$$ |
| $$\text{Poseidon}(\cdot)$$ | Poseidon2 hash function over $$\mathbb{F}\_r$$ (Section 2.5) |
| $$\delta\_{\ast}$$ | Domain separation constants (subscript identifies the domain) |
| $$[n]$$ | The set $$\\{0, 1, \ldots, n-1\\}$$ |

### 2.2 Grumpkin–BN254 Cycle

Grumpkin is defined by $$y^2 = x^3 - 17$$ over $$\mathbb{F}\_r$$. It forms a 2-cycle with BN254:

$$\text{base}(\mathbb{G}) = \mathbb{F}\_r^{\text{BN254}}, \qquad \text{scalar}(\mathbb{G}) = \mathbb{F}\_q^{\text{BN254}}$$

A Grumpkin point is a pair $$(x, y) \in \mathbb{F}\_r^2$$. Noir's native `Field` type is $$\mathbb{F}\_r$$, so Grumpkin point arithmetic inside UltraHonk circuits incurs no non-native field emulation. On-chain, the Soroban host provides BN254 $$\mathbb{F}\_r$$ arithmetic (`bn254_fr_{add, sub, mul, inv}` via CAP-80), which suffices for Grumpkin affine point operations.

**Scalar sampling.** Grumpkin scalars live in $$\mathbb{F}\_q$$, which is slightly larger than $$\mathbb{F}\_r$$. All secret scalars in this design ($$sk$$, $$r\_e$$, $$\sigma$$, $$\sigma\_a$$) are sampled by the **rejection sampling** procedure, which produces a uniform draw from $$\mathbb{F}\_r$$:

1. Draw 32 bytes (256 bits) from a CSPRNG.
2. Mask the top 2 bits to zero, yielding a 254-bit candidate $$x \in [0, 2^{254})$$.
3. If $$x \geq r$$, reject and return to step 1.
4. If the call site requires $$x \neq 0$$ and $$x = 0$$, reject and return to step 1.
5. Output $$x$$ in its canonical form -- as a Noir `Field` for in-circuit use, or as 32 big-endian bytes (`BytesN<32>`) for storage and event emission.

**Host deserialiser caveat.** The Soroban host's `bn254_fr_from_u256val` (the underlying primitive of `Bn254Fr::from_bytes`) accepts any 32-byte representative and *silently reduces* values $$x \geq r$$ modulo $$r$$ rather than rejecting them. Two distinct byte strings ($$x$$ and $$x + r$$) therefore deserialise to the same $$\mathbb{F}\_r$$ element, which means a verifier alone cannot distinguish canonical from non-canonical inputs. To keep stored state and emitted events byte-unique per logical value, the contract layer enforces canonicality on every prover-supplied $$\mathbb{F}\_r$$ representative *before* the bytes reach the verifier.

### 2.3 Pedersen Commitments

A Pedersen commitment to a value $$v$$ with a blinding factor $$r$$, viewed as scalars in Grumpkin's scalar field $$\mathbb{F}\_q$$, is:

$$\text{Com}(v, r) = v \cdot G + r \cdot H$$

In this design both $$v$$ and $$r$$ are drawn from $$\mathbb{F}\_r \subset \mathbb{F}\_q$$ (§2.2): $$v$$ is a non-negative integer below $$2^{127}$$ (§2.6) and $$r$$ is a Poseidon2 output or an $$\mathbb{F}\_r$$-sampled CSPRNG draw; the group law operates in $$\mathbb{F}\_q$$.

**Binding.** Finding $$(v', r') \neq (v, r)$$ such that $$\text{Com}(v, r) = \text{Com}(v', r')$$ requires computing $$\log\_G H$$, which is infeasible under the discrete logarithm assumption.

**Hiding.** For any $$v$$, the commitment $$\text{Com}(v, r)$$ with uniformly random $$r \in \mathbb{F}\_q$$ is uniformly distributed over $$\mathbb{G}$$, revealing nothing about $$v$$. Sampling $$r$$ from $$\mathbb{F}\_r \subset \mathbb{F}\_q$$ instead of full $$\mathbb{F}\_q$$ (§2.2) makes the commitment distribution **statistically close** to uniform over $$\mathbb{G}$$, with total-variation distance bounded by $$(|\mathbb{F}\_q| - |\mathbb{F}\_r|)/|\mathbb{F}\_q| \approx 2^{-127}$$.

**Homomorphism.** $$\text{Com}(v\_1, r\_1) + \text{Com}(v\_2, r\_2) = \text{Com}(v\_1 + v\_2, r\_1 + r\_2)$$. Scalar addition in the commitment relation is over $$\mathbb{F}\_q^{\text{BN254}}$$ -- the scalar field of $$\mathbb{G}$$, equivalently the order of the Grumpkin group. Since every committed value is bounded by $$2^{127}$$ (§2.6) and the number of additions across the lifetime of any one commitment is far below $$2^{127}$$, the value component never wraps in $$\mathbb{F}\_q$$ and the homomorphic relation holds in $$\mathbb{Z}$$ for values. The blinding component is added in $$\mathbb{F}\_q$$ and may reduce mod $$q$$ on accumulation; the only place this has operational consequences is the wallet's post-merge spend witness, where the canonical $$\mathbb{F}\_q$$ representative of $$r\_s + r\_r$$ can land in $$[r, q)$$ with probability bounded at $$(q-r)/q \approx 2^{-127}$$ per merge (see §10.4 *Post-merge witness availability*).

**Generators.** $$G$$ and $$H$$ are inherited from Barretenberg's standard Grumpkin Pedersen instantiation (the same generators that the toolchain's `pedersen_commitment` and `pedersen_hash` primitives use). Their provenance is part of the toolchain's audited surface, so the contract inherits both the generators and the soundness assumption that $$\log\_G H$$ is unknown. The Noir circuits import them as `embedded_curve_ops::generator()`.

### 2.4 Elliptic Curve Diffie-Hellman

Given a long-term keypair $$(a, A = a \cdot H)$$ and an ephemeral keypair $$(r\_e, R\_e = r\_e \cdot H)$$, the ECDH shared secret is:

$$S = r\_e \cdot A = a \cdot R\_e = a \cdot r\_e \cdot H \in \mathbb{G}$$

Both parties compute $$S$$ independently. We extract the scalar $$s = S.x \in \mathbb{F}\_r$$ for use as a Poseidon input.

### 2.5 Poseidon2 Hash

The system uses **Poseidon2**, the algebraic hash function native to Noir's standard library and implemented as a custom gate in Barretenberg. For the complete parameter specification of the Noir/Barretenberg instantiation, see:

- [Poseidon2 paper](https://eprint.iacr.org/2023/323) (Grassi, Khovratovich, Schofnegger, AFRICACRYPT 2023) - parameter derivation and security analysis
- [Barretenberg `poseidon2_params.hpp`](https://github.com/AztecProtocol/aztec-packages/blob/next/barretenberg/cpp/src/barretenberg/crypto/poseidon2/poseidon2_params.hpp) - concrete round constants and matrix entries
- [HorizenLabs reference implementation](https://github.com/HorizenLabs/poseidon2) - parameter generation script (`poseidon2_rust_params.sage`)
- [Noir stdlib `hash/mod.nr`](https://github.com/noir-lang/noir/blob/master/noir_stdlib/src/hash/mod.nr) - sponge construction wrapping the `Poseidon2Permutation` ACIR opcode

**Usage in this system:**

- Key derivation: $$vk = \text{Poseidon2}(\delta\_{\text{vk}}, sk, \text{addr\\\_f})$$
- Randomness derivation: $$r = \text{Poseidon2}(\delta\_{\text{spend\\\_r}}, vk, \sigma)$$
- Symmetric encryption: $$\tilde{v} = v + \text{Poseidon2}(\delta\_{\text{tx\\\_amount}}, s, \sigma)$$
- Domain separation: each invocation includes a leading constant $$\delta$$ to prevent cross-context collisions

**Sponge mode for auditor channels.** The per-transfer auditor ciphertexts (Section 8) use Poseidon2 in sponge mode. A single absorb of $$(\delta\_{\text{channel}}, S.x, \sigma)$$ is followed by $$n$$ sequential squeezes producing $$(m\_1, \ldots, m\_n) \in \mathbb{F}\_r^n$$, denoted $$\text{SpongeSqueeze}\_n(\delta\_{\text{channel}}, S.x, \sigma)$$. Two channel tags are used: $$\delta\_{\text{aud\\\_s}}$$ for the sender-auditor channel keyed by $$S\_{a,s}.x = (r\_e \cdot K\_{\text{aud,s}}).x$$, and $$\delta\_{\text{aud\\\_r}}$$ for the recipient-auditor channel keyed by $$S\_{a,r}.x = (r\_e \cdot K\_{\text{aud,r}}).x$$.

Squeeze order is canonical. The first squeezed mask is always an amount mask and the second is always a balance, allowance, or randomness mask, fixed per operation by the formulas in Sections 7 and 8. Single-ciphertext channels (the Withdraw balance checkpoint, W\_a3) take the *second* squeeze and leave the amount slot unused, so a checkpoint pad can never coincide with an amount pad.

A $$(r\_e, \sigma)$$ pair MUST be unique per proof. The sponge masks are deterministic in $$(S.x, \sigma)$$, so reusing the pair across two operations reuses every pad slot they share, and a slot whose plaintext is known in one operation (e.g. a transfer amount known to its recipient) decrypts the other operation's ciphertext in that slot. The canonical slot assignment above limits the blast radius of such reuse to same-slot pairs, but does not eliminate it; provers and wallets MUST sample a fresh $$(r\_e, \sigma)$$ for every proof (Section 9.6 already guarantees $$\sigma$$ freshness on retry).

All references to "Poseidon" in this document denote this Poseidon2 instantiation.

### 2.6 Integer Embedding and Range Proofs

**The problem.** Noir circuits operate over $$\mathbb{F}\_r$$, where every element is a non-negative integer modulo $$r \approx 2^{254}$$. The statement $$v \geq 0$$ is vacuously true for all $$v \in \mathbb{F}\_r$$, and $$v\_A \geq v\_{\text{tx}}$$ is undefined without specifying how integers are embedded in the field. Without explicit range constraints, a prover can claim a balance of 1 and transfer 1,000,000: the "new balance" $$1 - 1{,}000{,}000 \equiv r - 999{,}999 \pmod{r}$$ is a valid field element, and the commitment equation holds. The attacker has minted 999,999 tokens.

**Integer embedding.** We define a canonical embedding $$\iota: [0, 2^{127}) \to \mathbb{F}\_r$$ mapping non-negative integers to their natural field representatives. A field element $$x \in \mathbb{F}\_r$$ represents a valid balance or transfer amount if and only if $$x < 2^{127}$$.

**Range proof mechanism.** A range proof for $$x \in [0, 2^{127})$$ is implemented by decomposing $$x$$ into 127 bits inside the circuit and checking the recomposition:

$$x = \sum\_{i=0}^{126} b\_i \cdot 2^i, \qquad b\_i \in \\{0, 1\\} \\;\forall\\, i$$

Each $$b\_i$$ is constrained to be Boolean ($$b\_i \cdot (b\_i - 1) = 0$$) and the recomposition is checked against $$x$$. Noir's standard library exposes this directly:

```noir
// Range check: [0, 2^127)
value.assert_max_bit_size::<127>();
```

**Sufficiency argument.** If the prover supplies $$v\_A$$ and $$v\_{\text{tx}}$$ such that the circuit verifies:

1. $$v\_A \in [0, 2^{127})$$ (the opening of $$C\_{\text{spend}}$$)
2. $$v\_{\text{tx}} \in [0, 2^{127})$$ (the transfer amount)
3. $$v\_A - v\_{\text{tx}} \in [0, 2^{127})$$ (the new balance)

then $$v\_A - v\_{\text{tx}}$$ is a non-negative integer less than $$2^{127}$$, which is only possible if the integer subtraction did not underflow. This is because $$v\_A < 2^{127}$$ and $$v\_{\text{tx}} < 2^{127}$$, so if $$v\_A < v\_{\text{tx}}$$ as integers, then $$v\_A - v\_{\text{tx}} \pmod{r}$$ would be $$r - (v\_{\text{tx}} - v\_A)$$, which is at least $$r - 2^{127} \gg 2^{127}$$, failing constraint (3).

**Value capacity.** Both balances and transfer amounts are constrained to $$[0, 2^{127})$$. These bounds are enforced in every circuit that manipulates values. The bound is exactly the SEP-41 non-negative `i128` range, so the contract's value domain matches the underlying token's domain by construction. The gap between $$2^{127}$$ and $$|\mathbb{F}\_r| \approx 2^{254}$$ ensures that modular wrap-around is detectable by the range check.

**Receiving balance (unproven accumulation).** The receiving balance $$C\_{\text{receive}}$$ is updated by contract-side point addition without any proof from the recipient. Therefore, the receiving balance's committed value $$v\_r$$ is never directly range-checked by any circuit.

This is safe because $$v\_r$$ is *indirectly* bounded:

1. Each deposit adds a public `i128` amount validated by the contract ($$\ge 0$$, hence $$< 2^{127}$$).
2. Each incoming transfer adds a commitment whose sender circuit proved $$v\_{\text{tx}} \in [0, 2^{127})$$ (constraint T4 / O4).
3. All tokens in the contract entered through deposits, so the sum of all committed values is bounded by the underlying token's total supply ($$< 2^{127}$$). No single account can receive more than the total supply.
4. For the field-arithmetic concern (could $$v\_r$$ reach $$r$$ and wrap around), that would require $$r / 2^{127} > 2^{127}$$ incoming transfers, which is computationally infeasible.

When the owner spends after a merge, the spend proof constrains the full post-merge opening: $$v\_s + v\_r \in [0, 2^{127})$$ (via constraint W4 or T4 on the spendable balance). This provides an implicit range check at the next spend boundary.

### 2.7 Address-to-Field Encoding

In Soroban, the host's `address_to_strkey` function is defined for the two `ScAddressType` variants the contract interacts with as actors -- `Account` (Stellar ed25519 account) and `Contract` (Soroban contract instance) -- and errors on every other variant the SDK's `Address` type can wrap. The protocol encodes those addresses via their **canonical Stellar strkey** (SEP-23) representation:

$$\text{enc}(a) \\;=\\; \text{Address::to\\\_string}(a)\text{.to\\\_bytes}() \\;\in\\; \\{\text{ASCII}\\}^{56}$$

This is the 56-character ASCII strkey produced by the host's `address_to_strkey` function: a 1-byte version tag (`G` = `0x47` for `Account`, `C` = `0x43` for `Contract`), a 32-byte payload (ed25519 public key or contract hash), and a 2-byte CRC16 checksum, all base32-encoded into 56 ASCII characters. The byte string is fixed-length, canonical, and reproducible in every Stellar SDK via the language's stellar-strkey library; the protocol commits to these 56 ASCII bytes.

The Poseidon-compressed Field encoding splits the 56-byte string into two 28-byte limbs (each $$\le 2^{224} \ll r \approx 2^{254}$$, hence trivially in $$\mathbb{F}\_r$$):

$$\text{address\\\_to\\\_field}(a) \\;=\\; \text{Poseidon2}\big(\delta\_{\text{addr}}, \\;\text{lo}(a), \\;\text{hi}(a)\big)$$

where $$\text{lo}(a) = \sum\_{i=0}^{27} 256^{\\,i} \cdot \text{enc}(a)[i]$$ and $$\text{hi}(a) = \sum\_{i=0}^{27} 256^{\\,i} \cdot \text{enc}(a)[28 + i]$$ interpret the lower and upper 28 bytes of the strkey in little-endian byte order.

The contract, the SDK, the wallet, and any indexer reproduce the same Field value from the same Address by running their language's stellar-strkey encoder over the same `(version, payload)` pair and applying the same limb decomposition. No implementation needs to handle `ScAddress` XDR or the inner `AccountID` / `ContractID` union nesting.

**Usage sites.**

| Site | When computed | Storage |
|:---|:---|:---|
| $$\text{addr\\\_f}$$ | Once, by the contract's `__constructor` over `env.current_contract_address()` | Stored as a single Field in the contract's **instance storage** (§3.5); read on every proof verification |
| $$\text{op}\_i$$ | Per-call, by the contract at `set_spender` and `revoke_spender` over the `spender` argument | Not stored; recomputed each call. The circuit binds it via S5 / V3 |

---

## 3. System Model

### 3.1 Components

The system comprises three contracts deployed on Soroban:

**Token contract.** Holds SEP-41 token balances, manages encrypted account state, and delegates proof verification via cross-contract calls. Performs Grumpkin point arithmetic through $$\mathbb{F}\_r$$ host operations for homomorphic balance updates.

**Verifier contract.** A modified [UltraHonk verifier](https://github.com/indextree/ultrahonk_soroban_contract) storing one verification key per circuit type. Accepts a circuit identifier, serialized public inputs, and a proof blob; returns success or failure.

**Auditor contract.** Manages auditor encryption keys independently of the contract. One auditor contract serves multiple token contracts. Stores Grumpkin public keys as full affine points $$(x, y)$$ indexed by `auditor_id`. The contract validates that stored keys are non-identity curve points; a zero or identity key would make ECDH-derived ciphertexts trivially decryptable (since $$\sigma$$ is public). The contract fetches the active auditor key at operation time and passes it as a public input to the relevant circuit.

### 3.2 Threat Model

- The contract execution environment is trusted for correctness but not for privacy: all on-chain state and invocation inputs are public.
- Proof verification is sound: a valid proof guarantees the proven statement holds. This depends on the UltraHonk knowledge soundness assumption *and* the integrity of the Structured Reference String (Section 10.6).
- The discrete logarithm problem on Grumpkin is hard.
- Poseidon2 (Section 2.5) is a pseudorandom function (PRF) and is preimage-resistant over $$\mathbb{F}\_r$$ at the parameterized round count ($$R\_F = 8$$, $$R\_P = 56$$, 128-bit security target).
- Third parties may submit arbitrary transactions, including spam transfers to any registered account.

### 3.3 Trust Assumptions

The contract, verifier, and auditor contracts are trusted code. Users trust that the verification keys embedded in the verifier correspond to the correct circuits and were derived from a honestly generated Structured Reference String (Section 10.6). The auditor is trusted to protect its decryption key and exercise access only upon legitimate regulatory request.

### 3.4 Underlying Token Assumptions

The contract holds units of an underlying SEP-41 token on behalf of its users. The confidential accounting invariant (Section 9.3) implicitly assumes:

$$\sum\_i v\_{\text{committed},i} \\;\le\\; \text{token.balance}(\text{contract})$$

i.e., the total committed value across all confidential accounts never exceeds the public token balance held by the contract. The deployer's choice of underlying token determines whether that invariant is actually preserved over time. The contract itself does not, and cannot, defend against every misbehavior of the wrapped asset.

**Required properties of the underlying token.**

- *Non-rebasing.* The token's balance attributed to the contract address changes only as a result of explicit operations that the contract itself originated. Tokens whose balances change as a function of supply, oracle data, or external triggers break the accounting invariant and are unsupported.
- *No fee-on-transfer.* `token.transfer(from, to, amount)` MUST move exactly `amount` units. A fee deducted in transit would leave the contract's confidential accounting larger than its public backing.
- *Deterministic revert.* A failed `token.transfer` MUST cause the enclosing contract invocation (`deposit` or `withdraw`) to revert atomically, so confidential state is never updated against a token transfer that did not happen.
- *Underlying clawback / freeze / deauthorization.* These are surfaces of the Stellar Asset Contract (`StellarAssetInterface`), not the generic SEP-41 (`TokenInterface`). If the underlying token is a SAC whose issuer can clawback, freeze, or deauthorize the contract's holdings, confidential accounting at the contract layer may temporarily or permanently exceed the contract's accessible backing. This is an operational risk borne by the deployer's choice of underlying token. The token layer offers its own freeze and per-account clawback flows that operate inside the confidential surface; see [COMPLIANCE.md](./COMPLIANCE.md) §2 (contract-level freeze) and §5 (admin + auditor clawback). [COMPLIANCE.md](./COMPLIANCE.md) §2.2 additionally specifies SAC authorization passthrough, which composes the contract's freeze with the issuer's freeze without requiring the admin to mirror state.

**Non-negativity check.** The contract's public interface uses `i128` end-to-end, matching SEP-41. Every entrypoint that accepts a public amount (`deposit`, `withdraw`) MUST reject `amount < 0` and revert. The in-circuit range constraint (Section 2.6) bounds the same value at $$2^{127}$$ from above; together they pin the contract's value domain to $$[0, 2^{127}) = [0, \text{i128::MAX}]$$, matching SEP-41 exactly. No conversion at the SEP-41 boundary is needed.

### 3.5 Governance and Upgradeability

The constructor binds the contract to fixed `admin`, `token`, `verifier`, and `auditor` addresses. It additionally computes and stores $$\text{addr\\\_f} = \text{address\\\_to\\\_field}(\text{env.current\\\_contract\\\_address}())$$ (§2.7) in **instance storage** as a single canonical $$\mathbb{F}\_r$$ Field; this is the value every owner-initiated proof references via constraints R2 / W2 / T2 / S2 / V2. The compressed `addr_f` Field is computed once at construction (not recomputed per call) to ensure all proofs across the contract's lifetime bind to the same Field representative of the contract's address. Beyond that, this specification does not prescribe a governance policy for upgrading these components or for rotating per-circuit verification keys. Concrete deployments differ widely in spender structure, regulatory posture, and emergency-response requirements, so these decisions are deliberately left to implementers.

Questions an implementer must answer:

- May `admin` replace the `verifier` contract, or the `auditor` contract after deployment?
- Are per-circuit verification keys immutable for the lifetime of the deployment, or may they be updated?
- If any of the above is upgradeable, what authorization (single key, multisig), timelock, and event-emission rules apply?
- How do users independently reproduce a deployed VK from circuit source, toolchain, and SRS (Section 10.6)?

**Recommendation.** The strongest soundness posture is full immutability: `token`, `verifier`, `auditor`, and per-circuit verification keys all fixed at deployment, with any circuit or verifier change requiring a fresh deployment and an explicit user-side migration. Where operational realities make full immutability impractical (for example, a discovered soundness bug in a circuit or verifier that needs a fast fix), implementers may expose admin-guarded upgrade entrypoints for the `verifier` address or per-circuit VKs. In that case the upgrade path should be gated.

---

## 4. Key Hierarchy

All keys derive from a single spending secret $$sk \in \mathbb{F}\_r$$.

### 4.1 Spending Key

$$Y = sk \cdot H$$

The spending public key is stored on-chain at registration. Knowledge of $$sk$$ is required to authorize transfers, withdrawals, spender delegations, and merges.

### 4.2 Viewing Key

$$vk = \text{Poseidon}(\delta\_{\text{vk}}, sk, \text{addr\\\_f})$$

A scalar in $$\mathbb{F}\_r$$, unique per $$(sk, \text{addr\\\_f})$$ pair. Enables balance decryption without spending authority. Cannot recover $$sk$$ (Poseidon preimage resistance). Because $$\text{addr\\\_f}$$ is bound into the derivation, proofs that constrain $$vk$$ (R2, W2, T2, S2, V2) are inherently bound to the contract contract, eliminating the need for explicit per-circuit context binding.

### 4.3 Public Viewing Key

$$\text{PVK} = vk \cdot H$$

A Grumpkin point stored on-chain at registration. Serves as the recipient's ECDH public key for incoming transfers. The registration proof constrains $$\text{PVK} = vk \cdot H$$ where $$vk = \text{Poseidon}(\delta\_{\text{vk}}, sk, \text{addr\\\_f})$$ and $$Y = sk \cdot H$$, preventing a user from registering an unrelated $$\text{PVK}$$.

### 4.4 Delegation Viewing Key

For spender $$i$$ with address $$\text{op}\_i$$, the owner derives:

$$dvk\_i = \text{Poseidon}(\delta\_{\text{dvk}}, vk, \text{op}\_i)$$

Properties:
- $$dvk\_i$$ reveals only this spender's allowance state in this contract's context ($$vk$$ is contract-specific, Section 4.2).
- $$dvk\_i$$ cannot recover $$vk$$ (preimage resistance).
- Different $$(vk, \text{op}\_i)$$ tuples yield independent keys.

---

## 5. Commitment Scheme

The following symbols are used throughout this section:

| Symbol | Definition |
|:---|:---|
| $$C\_{\text{spend}}$$ | On-chain spendable balance commitment (Pedersen point) |
| $$C\_{\text{receive}}$$ | On-chain receiving balance commitment (Pedersen point) |
| $$C\_{\text{tx}}$$ | Transfer commitment added to recipient's $$C\_{\text{receive}}$$ |
| $$\text{Com}(v, r)$$ | Pedersen commitment $$v \cdot G + r \cdot H$$ |
| $$v\_s, r\_s$$ | Value and blinding factor of $$C\_{\text{spend}}$$ (off-chain wallet state) |
| $$v\_r, r\_r$$ | Value and blinding factor of $$C\_{\text{receive}}$$ (off-chain wallet state) |
| $$v\_{\text{tx}}$$ | Transfer amount (private) |
| $$r\_{\text{tx}}$$ | ECDH-derived blinding factor for $$C\_{\text{tx}}$$ |
| $$W\_{\text{spend}}, W\_{\text{receive}}$$ | Wallet-side accumulators: $$(v, r)$$ pairs tracking commitment openings |
| $$r\_e$$ | Ephemeral scalar sampled per transfer |
| $$R\_e$$ | Ephemeral public key $$r\_e \cdot H$$ (published in event data) |
| $$S$$ | ECDH shared secret point $$r\_e \cdot \text{PVK}\_B$$ |
| $$s$$ | Scalar extracted from shared secret: $$S.x \in \mathbb{F}\_r$$ |
| $$\tilde{v}$$ | Encrypted transfer amount: $$v\_{\text{tx}} + \text{Poseidon}(\delta\_{\text{tx\\\_amount}}, s, \sigma)$$ |
| $$\tilde{b}$$ | Encrypted balance scalar: $$v\_{\text{new}} + \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk, \sigma)$$ |
| $$\sigma$$ | Prover-chosen random salt, sampled per operation via the rejection sampling procedure of §2.2; canonical $$\mathbb{F}\_r$$ representative encoded as `BytesN<32>` |

### 5.1 Balance Commitments

Each balance is a single Pedersen commitment $$C = \text{Com}(v, r) \in \mathbb{G}$$, represented on-chain as an uncompressed affine point $$(x, y) \in \mathbb{F}\_r^2$$ (64 bytes). The identity $$\mathcal{O}$$ is encoded as $$(0, 0)$$ and handled as a special case in point arithmetic.

The committed value $$v$$ can represent the full range of practical balances (up to $$2^{127} - 1$$, bounded by the SEP-41 `i128` interface) without discrete logarithm concerns, because the owner maintains the commitment opening off-chain (Section 5.2) and the auditor reads an encrypted scalar (Section 5.5).

### 5.2 Off-Chain Opening Maintenance

A Pedersen commitment $$C = \text{Com}(v, r)$$ hides its opening $$(v, r)$$. The owner must know this opening to construct spend proofs so they must maintain $$(v, r)$$ as local wallet state, updated incrementally as balance-modifying events occur.

**Definition** (Wallet state). The owner's wallet maintains two running accumulators:

$$W\_{\text{spend}} = (v\_s, r\_s) \quad \text{such that} \quad C\_{\text{spend}} = v\_s \cdot G + r\_s \cdot H$$
$$W\_{\text{receive}} = (v\_r, r\_r) \quad \text{such that} \quad C\_{\text{receive}} = v\_r \cdot G + r\_r \cdot H$$

**Initialization.** At registration, $$C\_{\text{spend}} = C\_{\text{receive}} = \mathcal{O}$$. The wallet sets $$W\_{\text{spend}} = W\_{\text{receive}} = (0, 0)$$.

**Update rules.** Each balance-modifying event updates exactly one accumulator:

| Event | Accumulator update |
|:---|:---|
| Deposit of public amount $$a$$ to this account | $$W\_{\text{receive}} \mathrel{+}= (a, 0)$$ |
| Incoming transfer with event $$(R\_e, \tilde{v}, \sigma)$$ | Compute $$S = vk \cdot R\_e$$, $$s = S.x$$; derive $$v\_{\text{tx}} = \tilde{v} - \text{Poseidon}(\delta\_{\text{tx\\\_amount}}, s, \sigma)$$ and $$r\_{\text{tx}} = \text{Poseidon}(\delta\_{\text{tx\\\_blind}}, s, \sigma)$$. Then $$W\_{\text{receive}} \mathrel{+}= (v\_{\text{tx}}, r\_{\text{tx}})$$ |
| Outgoing transfer/withdrawal of amount $$a$$ | Proof outputs new commitment with deterministic randomness. $$W\_{\text{spend}} \leftarrow (v\_s - a, \\; \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk, \sigma))$$ |
| Merge | $$W\_{\text{spend}} \leftarrow (v\_s + v\_r, \\; r\_s + r\_r)$$; $$W\_{\text{receive}} \leftarrow (0, 0)$$ |
| Set spender (escrow amount $$a$$) | Proof outputs new commitment. $$W\_{\text{spend}} \leftarrow (v\_s - a, \\; \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk, \sigma))$$ |
| Revoke spender (reclaim amount $$a$$) | Proof outputs new commitment. $$W\_{\text{spend}} \leftarrow (v\_s + a, \\; \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk, \sigma))$$ |

The Merge row uses exact integer addition; $$W\_{\text{spend}}.r$$ is not reduced modulo $$r$$ or $$q$$ as merges accumulate. At proof-construction time the wallet reduces $$W\_{\text{spend}}.r$$ modulo $$q$$ and encodes the canonical $$\mathbb{F}\_q$$ representative as a single $$\mathbb{F}\_r$$ `Field`. This encoding succeeds when the representative lies in $$[0, r)$$, with probability $$\geq 1 - 2^{-127}$$ per merge; the complementary case is acknowledged in §10.4 *Post-merge witness availability*.

After every owner-initiated operation that produces a proof, $$r\_s$$ resets to a deterministic value. This is the **normalization** property: the spendable balance's blinding factor is always recoverable from $$(vk, \sigma)$$ at spend boundaries. Together with $$\tilde{b}$$, both emitted in the spend-boundary event, each spend boundary forms a **checkpoint** from which the spendable opening $$(v\_s, r\_s)$$ is recoverable via a single event lookup, with no exhaustive history replay needed for $$W\_{\text{spend}}$$. Recovering $$W\_{\text{receive}}$$, and folding in any post-checkpoint merges, still requires replaying events emitted after the checkpoint (see Recovery below).

**Consistency check.** At any time, the wallet can verify its state: $$C\_{\text{spend}} \stackrel{?}{=} v\_s \cdot G + r\_s \cdot H$$ and $$C\_{\text{receive}} \stackrel{?}{=} v\_r \cdot G + r\_r \cdot H$$, where $$C\_{\text{spend}}$$ and $$C\_{\text{receive}}$$ are read from on-chain state.

**Recovery.** If the wallet loses local state, it recovers from the **last checkpoint**: the most recent owner-initiated proof operation (`withdraw`, `confidential_transfer`, `set_spender`, or `revoke_spender`), which emitted both $$\tilde{b}$$ and $$\sigma$$ in its event. By construction, only deposits, incoming transfers, and merges can occur after this event; any later owner-initiated proof operation would itself become the new checkpoint. Steps 1-4 recover the spendable balance using $$\tilde{b}$$, $$\sigma$$ (both from the event), and $$vk$$. Event replay (steps 5-6) folds in the bounded post-checkpoint activity:

1. Fetch $$(\tilde{b}, \sigma)$$ from the most recent **checkpoint event** for this account, where a checkpoint event is exactly one of `Withdraw`, `Transfer` (where the account is the `from`), `SetSpender`, or `RevokeSpender` -- the four event types that carry a proof-bound $$(\tilde{b}, \sigma)$$ for the account's spendable balance. `Deposit`, `Transfer` (where the account is the `to`), `SpenderTransfer` (recipient side), and `Merge` are explicitly **not** checkpoints: they either carry no $$(\tilde{b}, \sigma)$$ at all or carry one that is bound to a different account's spendable balance. **No-checkpoint case:** if the account has no checkpoint event since `Register`, initialize $$W\_{\text{spend}} \leftarrow (0, 0)$$ and skip to step 5 with the replay window starting at the `Register` event.
2. Recover the spendable balance value: $$v\_s = \tilde{b} - \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk, \sigma)$$.
3. Recover the spendable balance blinding: $$r\_s = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk, \sigma)$$.
4. Set $$W\_{\text{spend}} \leftarrow (v\_s, r\_s)$$ and $$W\_{\text{receive}} \leftarrow (0, 0)$$.
5. Replay all events since the checkpoint in ledger order. For each event:
   - **Incoming transfer** $$(R\_e, \tilde{v}, \sigma\_{\text{sender}})$$: compute $$S = vk \cdot R\_e$$, derive $$v\_{\text{tx}}$$ and $$r\_{\text{tx}}$$. Accumulate $$W\_{\text{receive}} \mathrel{+}= (v\_{\text{tx}}, r\_{\text{tx}})$$.
   - **Deposit** of amount $$a$$: accumulate $$W\_{\text{receive}} \mathrel{+}= (a, 0)$$.
   - **Merge**: fold $$W\_{\text{spend}} \leftarrow (W\_{\text{spend}}.v + W\_{\text{receive}}.v, \\; W\_{\text{spend}}.r + W\_{\text{receive}}.r)$$, reset $$W\_{\text{receive}} \leftarrow (0, 0)$$.
6. Verify consistency: $$C\_{\text{spend}} \stackrel{?}{=} W\_{\text{spend}}.v \cdot G + W\_{\text{spend}}.r \cdot H$$ and $$C\_{\text{receive}} \stackrel{?}{=} W\_{\text{receive}}.v \cdot G + W\_{\text{receive}}.r \cdot H$$.

Steps 1-3 require $$(\tilde{b}, \sigma)$$ from the latest owner event and $$vk$$. No full event replay is needed. Step 5 replays only events since the last checkpoint and correctly handles any number of interleaved deposits, transfers, and merges. A wallet that spends regularly produces frequent checkpoints, bounding the replay window. In the worst case (funds received but never spent), the replay window extends back to registration.

**Event durability requirement.** Recovery depends on the wallet being able to retrieve every event since the last checkpoint, plus the checkpoint event itself, in ledger order. Stellar RPC retains event history for a 7-days window only, so a wallet that loses local state after that window cannot recover from RPC alone. The protocol therefore assumes a durable event archive that retains the full per-account history of `Withdraw`, `Transfer` (both directions), `SpenderTransfer` (recipient side), `Deposit`, `Merge`, `SetSpender`, and `RevokeSpender` events forever. The data model, ingestion contract, retention obligations, and recommended API surface for that archive are specified in the indexing document (INDEXER.md, to be added). Wallets and SDKs MUST consume an indexer that meets that contract for recovery.

### 5.3 ECDH-Derived Blinding

When a sender (spending key $$sk\_A$$) transfers to a recipient with public viewing key $$\text{PVK}\_B$$, the transfer commitment uses blinding derived from an ephemeral ECDH exchange.

**Definition 1** (Transfer blinding derivation). The sender samples $$r\_e, \sigma \in \mathbb{F}\_r$$ via the rejection sampling procedure (§2.2), then computes:

$$R\_e = r\_e \cdot H$$
$$S = r\_e \cdot \text{PVK}\_B$$
$$s = S.x \in \mathbb{F}\_r$$
$$r\_{\text{tx}} = \text{Poseidon}(\delta\_{\text{tx\\\_blind}}, s, \sigma)$$
$$\tilde{v} = v\_{\text{tx}} + \text{Poseidon}(\delta\_{\text{tx\\\_amount}}, s, \sigma)$$

where $$v\_{\text{tx}}$$ is the transfer amount. The transfer commitment is $$C\_{\text{tx}} = \text{Com}(v\_{\text{tx}}, r\_{\text{tx}})$$. The ephemeral public key $$R\_e$$, encrypted amount $$\tilde{v}$$, and $$\sigma$$ are published in the transaction event data so recipients can derive both $$v\_{\text{tx}}$$ and $$r\_{\text{tx}}$$ during replay.

Since $$vk\_B \cdot R\_e = r\_e \cdot \text{PVK}\_B = S$$ by ECDH commutativity, both sender and recipient can independently derive $$r\_{\text{tx}}$$ and decrypt $$v\_{\text{tx}} = \tilde{v} - \text{Poseidon}(\delta\_{\text{tx\\\_amount}}, s, \sigma)$$, provided they know $$\sigma$$ emitted with the event. The auditor decrypts the transfer amount via a separate ECDH channel (Section 8.1).

**Note.** Each transfer involves two auditor ECDH exchanges: one with the recipient's auditor key ($$S\_{a,r} = r\_e \cdot K\_{\text{aud,r}}$$) and one with the sender's auditor key ($$S\_{a,s} = r\_e \cdot K\_{\text{aud,s}}$$). Both reuse the ephemeral scalar $$r\_e$$, as does the $$dvk\_i$$ escrow ECDH in `set_spender` (§7.11) when one is present. Neither auditor recovers any account's viewing key.

**Why reusing $$r\_e$$ is safe.** Each ECDH channel keyed from the same $$r\_e$$ produces a distinct shared scalar because the counterparty public keys are distinct ($$\text{PVK}\_B$$, $$K\_{\text{aud,r}}$$, $$K\_{\text{aud,s}}$$, $$Y\_{\text{op}}$$ are independent Grumpkin points, none derivable from one another). Each channel further uses a distinct Poseidon domain tag ($$\delta\_{\text{tx\\\_blind}}/\delta\_{\text{tx\\\_amount}}$$ for the recipient channel, $$\delta\_{\text{aud\\\_r}}$$ and $$\delta\_{\text{aud\\\_s}}$$ for the two auditor channels, $$\delta\_{\text{esc\\\_dvk}}$$ for the spender escrow), so masks across channels are independent under the PRF assumption on Poseidon (§3.2). The channel masks are used as one-time pads against fresh per-transfer randomness ($$\sigma$$ or $$\sigma\_a$$), and each per-channel sponge re-absorbs that nonce, so a given mask is never reused even for the same counterparty across two operations. Together these three properties (distinct shared scalars, distinct domains, fresh per-operation nonce) close the standard ECDH key-reuse attack surface; the contract's enumeration of channels in §13 satisfies the domain-distinctness condition.

### 5.4 Anti-Poisoning Constraint

The transfer circuit enforces that $$C\_{\text{tx}}$$ was constructed using the ECDH-derived $$r\_{\text{tx}}$$:

$$C\_{\text{tx}} = v\_{\text{tx}} \cdot G + r\_{\text{tx}} \cdot H \quad \text{where} \quad r\_{\text{tx}} = \text{Poseidon}(\delta\_{\text{tx\\\_blind}}, s, \sigma)$$

This prevents a malicious sender from committing with arbitrary blinding, which would cause the recipient to lose track of their accumulated blinding factor and be unable to spend.

### 5.5 Encrypted Balance Scalar

Owner-initiated operations (transfers, withdrawals) produce a new spendable balance commitment with deterministic randomness (Section 7). To enable wallet recovery without full event replay, the proof also outputs an **encrypted balance scalar**:

$$\tilde{b} = v\_{\text{new}} + \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk, \sigma)$$

where $$v\_{\text{new}}$$ is the new spendable balance and $$\sigma$$ is the prover-chosen random salt. The contract emits $$\tilde{b}$$ in the operation's event (Section 11.2) rather than storing it on-chain; the contract never reads it after the proof has bound it to $$C\_{\text{spend}}$$. Anyone with $$vk$$ recovers $$v\_{\text{new}} = \tilde{b} - \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk, \sigma)$$ from the event. The primary consumer is the owner's wallet for checkpoint recovery (Section 5.2); auditors do not hold $$vk$$ and instead read balances via per-transfer ECDH ciphertexts (Section 8.1). The circuit enforces consistency between $$\tilde{b}$$ and the committed value in $$C\_{\text{spend}}$$.

---

## 6. Account State

### 6.1 Account Data Model

Each registered account stores a `ConfidentialAccount` in persistent storage, keyed by `Address`:

```rust
ConfidentialAccount {
    spending_public_key:         BytesN<64>,   // Y = sk · H
    viewing_public_key:   BytesN<64>,   // PVK = vk · H
    spendable_commitment:    BytesN<64>,   // C_spend: single Pedersen commitment
    receiving_commitment:    BytesN<64>,   // C_receive: single Pedersen commitment
    auditor_id:           u32,
}
```

**`spending_public_key`**

$$Y = sk \cdot H$$. Set once at registration. Authorizes all spending operations.

**`viewing_public_key`**

$$\text{PVK} = vk \cdot H$$. Set once at registration. Used by senders for ECDH key agreement. The registration proof enforces derivation from the same $$sk$$ as $$Y$$.

**`spendable_commitment`**

The commitment the owner can spend from. Modified only by owner-authorized operations: transfers out, withdrawals, merge, `set_spender`, `revoke_spender`. Encoded as a single Grumpkin affine point (64 bytes).


**`receiving_commitment`**

Accumulates incoming deposits and transfers via homomorphic addition. The contract adds to this without any proof from the recipient. Reset to $$\mathcal{O}$$ on merge. Encoded as a single Grumpkin affine point (64 bytes).

**`auditor_id`**

Index into the auditor contract's key store. Set once at registration. Used by the contract to fetch the correct auditor public key when building transfer public inputs. For incoming transfers, the recipient's `auditor_id` determines the key under which the transfer amount is encrypted. For outgoing transfers (and spender transfers), the sender's (or owner's) `auditor_id` determines the key under which the transfer amount and post-transfer balance (or allowance) are encrypted.

### 6.2 Spender Delegation

Spender delegations are stored in persistent storage, keyed by `(owner, spender)`:

```rust
SpenderDelegation {
    allowance_commitment: BytesN<64>,   // Single Pedersen commitment
    a_tilde:  BytesN<32>,   // Poseidon-encrypted allowance scalar
    escrowed_dvk:         BytesN<64>,   // ECDH escrow of dvk_i under spender key
    allowance_salt:       BytesN<32>,
    live_until_ledger:    u32,
}
```

**`allowance_commitment`**

The spender's remaining escrowed allowance, a single Pedersen commitment: $$C\_a = \text{Com}(v\_a, r\_a)$$ where $$r\_a = \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a)$$. One Grumpkin point (64 bytes).

**`a_tilde`**

Poseidon-encrypted allowance scalar: $$\tilde{a} = v\_a + \text{Poseidon}(\delta\_{\text{enc\\\_allow}}, dvk\_i, \sigma\_a)$$. Enables the spender (who holds $$dvk\_i$$ via `escrowed_dvk`) to read the current allowance without DLP when constructing an `SpenderTransfer` witness. The owner can also read it via $$vk \rightarrow dvk\_i$$. The auditor does not consume this field; allowance visibility for the auditor is provided by the per-event ciphertexts (Section 8.5).

**`escrowed_dvk`**

$$dvk\_i$$ encrypted under the spender's spending key via ECDH. (64 bytes)

**`allowance_salt`**

Per-delegation salt for allowance randomness derivation, encoded as `BytesN<32>` (canonical $$\mathbb{F}\_r$$ representative). $$\sigma\_a$$ is sampled by the rejection sampling procedure of §2.2 (same as $$\sigma$$) and is the sole freshness input to all allowance Poseidon derivations. Set by the owner at `set_spender` and replaced by the spender on every `confidential_transfer_from` (the spender samples a fresh `new_allowance_salt` and that becomes the stored value alongside the updated `allowance_commitment`). The salt is bound to the current commitment: when the commitment changes, the salt changes with it. It is stored on-chain so the owner can decrypt the allowance at revocation without depending on event history.

**Dual role.** In spender transfers, $$\sigma\_a$$ also serves as the nonce for the recipient ECDH encryption (O7, O9) and the auditor channel sponges (O\_a2 and O\_a6, which absorb $$\sigma\_a$$ alongside the channel shared scalar). This is safe because ECDH confidentiality derives from the shared secret $$S.x$$ (or $$S\_{a,r}.x$$, $$S\_{a,s}.x$$), not from $$\sigma\_a$$ being secret. However, this couples the allowance salt to the transfer event: the event must emit $$\sigma\_a$$ so that the recipient and auditor can decrypt. Any change to how the salt is stored or exposed must preserve this invariant.

**`live_until_ledger`**

The ledger number at which the delegation expires. The delegation is live while `ledger.sequence() <= live_until_ledger` and expired once `ledger.sequence() > live_until_ledger`. Checked on every `confidential_transfer_from`. The delegation persists in storage until explicitly revoked (if it were in temporary storage automatic cleanup would destroy escrowed funds).

The `(owner, spender)` storage entry holds at most one delegation. `set_spender` (Section 7.7) reverts if a delegation already exists for that pair, regardless of whether the existing delegation is past `live_until_ledger`. Expiry only prevents the spender from spending; the escrowed value persists on-chain until `revoke_spender` (Section 7.9) folds it back into the owner's spendable balance. Re-delegating to the same spender therefore requires the sequence: `revoke_spender` then `set_spender`. This rule is what keeps the balance-conservation invariant (Section 9.3) ranging cleanly over stored delegations: every delegation is either active, expired-pending-revoke, or absent, and the escrowed value is never silently dropped.

---

## 7. Operations

### 7.1 Public Input Sources

UltraHonk verifies the relation between a proof and its public-input vector. The verifier sees only field elements -- it has no knowledge of which account, contract, or auditor those values are supposed to describe. Binding each public input to the correct provenance is the contract's responsibility. If the contract takes a value that should come from trusted state (e.g. the sender's `spending_public_key`) and instead reads it from caller-controlled invocation inputs, a soundly proven statement can verify for the wrong account.

Each operation below lists, for every public input, where the contract loads it from -- persistent account storage, the delegation entry, the contract's own contract address, an auditor-contract lookup, an invocation argument, or a prover-supplied value that the circuit binds.

**Trust-boundary rule.** Public inputs that derive from trusted state (account storage, delegation storage, the current contract address, or auditor-contract lookups) MUST be loaded by the contract itself. The contract MUST NOT accept these values from the caller's `data` payload. Only invocation arguments (which are bound under `require_auth()` per §11.1) and prover-supplied values (which the circuit binds to its constraints) may originate from the caller. Violating this rule breaks soundness even with a perfectly sound circuit.

### 7.2 Registration

An account provides a Grumpkin spending key $$Y$$, a public viewing key $$\text{PVK}$$, and a chosen `auditor_id`, accompanied by a proof of key well-formedness.

**Circuit constraints (Register):**

| # | Constraint |
|:--|:---|
| R1 | $$Y = sk \cdot H$$ (spending key well-formed) |
| R2 | $$vk = \text{Poseidon}(\delta\_{\text{vk}}, sk, \text{addr\\\_f})$$ (viewing key correctly derived, binds proof to contract) |
| R3 | $$\text{PVK} = vk \cdot H$$ (public viewing key matches $$vk$$) |
| R4 | $$sk \neq 0$$ (rules out $$Y = \mathcal{O}$$) |
| R5 | $$vk \neq 0$$ (rules out $$\text{PVK} = \mathcal{O}$$, which would collapse every incoming-transfer ECDH) |

**Public inputs:**

| Input | Notes |
|:---|:---|
| $$Y$$, $$\text{PVK}$$ | Prover-supplied; written to `account.spending_public_key` and `account.viewing_public_key` on success |
| $$\text{addr\\\_f}$$ | Loaded from instance storage; set once at construction (§3.5) |
| $$\text{acct\\\_f}$$ | Binds the proof to the registering address that is authenticated with `require_auth()`|

$$\text{acct\\\_f}$$ is referenced by no circuit constraint; its membership in the public-input set is the binding. The verifier absorbs every public input into the proof transcript, so a proof produced for one account fails verification when the contract assembles the blob for any other address. Without this input, the register proof and its public keys — all published on-chain by a legitimate registration — could be replayed by any caller to create duplicate-key accounts under fresh addresses.

**Private witnesses:** $$sk$$.

**Post-verification state:** The contract validates that `auditor_id` exists in the auditor contract and points to a valid key, then stores `spending_public_key`, `viewing_public_key`, `auditor_id`, and initializes `spendable_commitment = receiving_commitment = ` $$\mathcal{O}$$.

**Auditor selection.** The registering account owner chooses `auditor_id` freely: the register proof does not constrain it, and the core validates only that the id exists in the auditor registry. On a shared auditor registry, deployments that must restrict which auditors an account may bind to MUST enforce that restriction in their `Hooks::on_register` implementation — the default `ComplianceHooks::on_register` deliberately does not restrict it. See [COMPLIANCE.md](./COMPLIANCE.md) §4.3 for a worked example.

### 7.3 Deposit

Transparent tokens flow from the depositor to the contract via `token.transfer(from, self, amount)`. The amount $$a$$ is public and typed as `i128`. The contract checks $$a \ge 0$$ at the entrypoint and reverts on violation (Section 3.4). The contract then computes the deposit commitment with zero blinding:

$$C\_{\text{dep}} = a \cdot G + 0 \cdot H = a \cdot G$$

and adds it to the recipient's receiving balance:

$$C\_{\text{receive}} \leftarrow C\_{\text{receive}} + C\_{\text{dep}}$$

No proof required. The recipient `to` **must** be registered: the receiving-balance update writes into `to`'s `ConfidentialAccount` storage entry. The depositor `from` does **not** need a registered confidential account; only the SEP-41 `token.transfer(from, self, a)` authorization is required. The recipient's off-chain state updates: $$v\_{\text{receive}} \mathrel{+}= a$$, $$r\_{\text{receive}} \mathrel{+}= 0$$.

### 7.4 Merge

The owner folds the receiving balance into the spendable balance.

**Contract logic (no proof):**

```
require account.require_auth()
C_spend ← C_spend + C_receive
C_receive ← O
```

**Proposition 1** (Merge correctness). If $$C\_{\text{spend}} = \text{Com}(v\_s, r\_s)$$ and $$C\_{\text{receive}} = \text{Com}(v\_r, r\_r)$$ before merge, then after merge $$C\_{\text{spend}} = \text{Com}(v\_s + v\_r, r\_s + r\_r)$$ and $$C\_{\text{receive}} = \mathcal{O} = \text{Com}(0, 0)$$.

*Proof.* By the homomorphic property of Pedersen commitments:
$$C\_{\text{spend}} + C\_{\text{receive}} = (v\_s \cdot G + r\_s \cdot H) + (v\_r \cdot G + r\_r \cdot H) = (v\_s + v\_r) \cdot G + (r\_s + r\_r) \cdot H = \text{Com}(v\_s + v\_r, r\_s + r\_r)$$
No value is created or destroyed. $$\square$$

**Owner state update.** The owner knows the opening of the post-merge commitment: $$v\_{\text{spend}}' = v\_s + v\_r$$, $$r\_{\text{spend}}' = r\_s + r\_r$$. The owner knows $$v\_r$$ and $$r\_r$$ from processing incoming transfer and deposit events into $$W\_{\text{receive}}$$ (Section 5.2, *Update rules*; the per-transfer derivation is Definition 1 in Section 5.3). The values $$v\_s$$ and $$r\_s$$ are known from the owner's last proof output.

**Griefing analysis.** Merge requires `account.require_auth()`. No third party can invoke it. Incoming transfers that arrive between proof construction and submission modify only $$C\_{\text{receive}}$$, which is not referenced by spend proofs. Therefore merge is not front-runnable and incoming transfers cannot invalidate spend proofs (Proposition 2, Section 9.1).

**Encrypted balance.** Merge emits no $$\tilde{b}$$ (there is no proof to enforce consistency between $$\tilde{b}$$ and the post-merge $$C\_{\text{spend}}$$). The next owner-initiated proof operation issues a fresh checkpoint. The auditor tracks incoming amounts independently from transfer events.

### 7.5 Withdrawal

The owner withdraws a public amount $$a$$ (typed `i128`) from their spendable balance. The W4 range constraint bounds $$a$$ at $$2^{127}$$ in-circuit; the contract additionally checks $$a \ge 0$$ at the entrypoint (Section 3.4).

**Circuit constraints (Withdraw):**

| # | Constraint |
|:--|:---|
| W1 | $$Y = sk \cdot H$$ (owner key ownership) |
| W2 | $$vk = \text{Poseidon}(\delta\_{\text{vk}}, sk, \text{addr\\\_f})$$ (binds proof to contract) |
| W3 | The prover knows the opening $$(v, r)$$ of $$C\_{\text{spend}}$$: $$C\_{\text{spend}} = v \cdot G + r \cdot H$$ |
| W4 | $$v \in [0, 2^{127})$$, $$a \in [0, 2^{127})$$, $$v - a \in [0, 2^{127})$$ (range validity, Section 2.6) |
| W5 | $$r' = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk, \sigma)$$ (deterministic randomness for new balance) |
| W6 | $$C\_{\text{spend}}' = (v - a) \cdot G + r' \cdot H$$ (new spendable commitment) |
| W7 | $$\tilde{b} = (v - a) + \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk, \sigma)$$ (encrypted balance scalar) |
| W8 | $$r\_e \neq 0$$ (rules out $$R\_e = \mathcal{O}$$ and $$S\_{a,s} = \mathcal{O}$$, which would reduce $$m\_b$$ to a constant function of $$\sigma$$) |
| W\_a1 | $$R\_e = r\_e \cdot H$$ (ephemeral key for auditor ECDH) |
| W\_a2 | $$S\_{a,s} = r\_e \cdot K\_{\text{aud,s}}$$ (sender-auditor ECDH shared secret) |
| W\_a3 | $$(\cdot, m\_b) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_s}}, S\_{a,s}.x, \sigma)$$ (sender-auditor channel sponge; $$m\_b$$ is the second squeeze — the balance slot, matching T\_a6/S\_a3/V\_a3. The first-squeeze amount slot is unused: the withdrawal amount is public, and skipping the slot keeps the checkpoint pad distinct from every amount pad even under $$(r\_e, \sigma)$$ reuse, Section 2.5) |
| W\_a4 | $$\tilde{b}\_{\text{aud,s}} = (v - a) + m\_b$$ (sender-auditor encrypted balance checkpoint) |

**Public inputs (15 fields):**

| Input | Notes |
|:---|:---|
| $$C\_{\text{spend}}$$ | Loaded from `from.spendable_commitment` |
| $$Y$$ | Loaded from `from.spending_public_key` |
| $$\text{addr\\\_f}$$ | Loaded from instance storage; set once at construction (§3.5) |
| $$K\_{\text{aud,s}}$$ | Fetched from the auditor contract using `from.auditor_id` |
| $$a$$ | Public withdrawal amount from invocation inputs |
| $$C\_{\text{spend}}'$$, $$\sigma$$, $$\tilde{b}$$, $$R\_e$$, $$\tilde{b}\_{\text{aud,s}}$$ | Prover-supplied; $$C\_{\text{spend}}'$$ written to `from.spendable_commitment`, the rest emitted in event |

$$\text{to}$$ is bound under `from.require_auth()` and does not appear in the proof.

**Private witnesses:** $$sk$$, $$vk$$, $$v$$, $$r$$, $$r\_e$$.

**Post-verification:** The contract verifies the proof, sets `from`.`spendable_commitment` $$= C\_{\text{spend}}'$$, and calls `token.transfer(self, to, a)`. Emits event with $$(R\_e, \sigma, \tilde{b}, \tilde{b}\_{\text{aud,s}})$$.

### 7.6 Confidential Transfer

The sender (account $$A$$, spending key $$sk\_A$$) transfers a hidden amount $$v\_{\text{tx}}$$ to recipient $$B$$ (public viewing key $$\text{PVK}\_B$$).

**Sender computation:**

1. Sample ephemeral scalar $$r\_e \in \mathbb{F}\_r$$ via the rejection sampling procedure (§2.2); sample $$\sigma \in \mathbb{F}\_r$$ via the same procedure
2. Compute $$R\_e = r\_e \cdot H$$
3. Compute $$S = r\_e \cdot \text{PVK}\_B$$, extract $$s = S.x$$
4. Derive transfer blinding: $$r\_{\text{tx}} = \text{Poseidon}(\delta\_{\text{tx\\\_blind}}, s, \sigma)$$
5. Derive encrypted amount: $$\tilde{v} = v\_{\text{tx}} + \text{Poseidon}(\delta\_{\text{tx\\\_amount}}, s, \sigma)$$
6. Compute transfer commitment: $$C\_{\text{tx}} = v\_{\text{tx}} \cdot G + r\_{\text{tx}} \cdot H$$
7. Compute new spendable commitment with deterministic randomness:
   - $$r\_A' = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk\_A, \sigma)$$
   - $$C\_{\text{spend}}' = (v\_A - v\_{\text{tx}}) \cdot G + r\_A' \cdot H$$
8. Compute encrypted balance scalar: $$\tilde{b} = (v\_A - v\_{\text{tx}}) + \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk\_A, \sigma)$$
9. Compute recipient-auditor ECDH shared secret: $$S\_{a,r} = r\_e \cdot K\_{\text{aud,r}}$$, extract $$s\_{a,r} = S\_{a,r}.x$$
10. Squeeze recipient-auditor channel masks: $$(m\_{v,r}, m\_{r,r}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_r}}, s\_{a,r}, \sigma)$$
11. Compute recipient-auditor ciphertexts: $$\tilde{v}\_{\text{aud,r}} = v\_{\text{tx}} + m\_{v,r}$$ and $$\tilde{r}\_{\text{aud,r}} = r\_{\text{tx}} + m\_{r,r}$$
12. Compute sender-auditor ECDH shared secret: $$S\_{a,s} = r\_e \cdot K\_{\text{aud,s}}$$, extract $$s\_{a,s} = S\_{a,s}.x$$
13. Squeeze sender-auditor channel masks: $$(m\_{v,s}, m\_{b,s}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_s}}, s\_{a,s}, \sigma)$$
14. Compute sender-auditor ciphertexts: $$\tilde{v}\_{\text{aud,s}} = v\_{\text{tx}} + m\_{v,s}$$ and $$\tilde{b}\_{\text{aud,s}} = (v\_A - v\_{\text{tx}}) + m\_{b,s}$$

**Circuit constraints (Transfer):**

| # | Constraint |
|:--|:---|
| T1 | $$Y\_A = sk\_A \cdot H$$ (sender key ownership) |
| T2 | $$vk\_A = \text{Poseidon}(\delta\_{\text{vk}}, sk\_A, \text{addr\\\_f})$$ (binds proof to contract) |
| T3 | Prover knows opening $$(v\_A, r\_A)$$ of $$C\_{\text{spend}}^A$$ |
| T4 | $$v\_A \in [0, 2^{127})$$, $$v\_{\text{tx}} \in [0, 2^{127})$$, $$v\_A - v\_{\text{tx}} \in [0, 2^{127})$$ (range validity, Section 2.6) |
| T5 | $$S = r\_e \cdot \text{PVK}\_B$$ (ECDH correctly computed) |
| T6 | $$R\_e = r\_e \cdot H$$ (ephemeral key well-formed) |
| T7 | $$r\_{\text{tx}} = \text{Poseidon}(\delta\_{\text{tx\\\_blind}}, S.x, \sigma)$$ (blinding correctly derived) |
| T8 | $$C\_{\text{tx}} = v\_{\text{tx}} \cdot G + r\_{\text{tx}} \cdot H$$ (transfer commitment well-formed) |
| T9 | $$\tilde{v} = v\_{\text{tx}} + \text{Poseidon}(\delta\_{\text{tx\\\_amount}}, S.x, \sigma)$$ (encrypted amount correct) |
| T10 | $$r\_A' = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk\_A, \sigma)$$ (deterministic randomness) |
| T11 | $$C\_{\text{spend}}' = (v\_A - v\_{\text{tx}}) \cdot G + r\_A' \cdot H$$ (new sender balance) |
| T12 | $$\tilde{b} = (v\_A - v\_{\text{tx}}) + \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk\_A, \sigma)$$ (encrypted balance scalar) |
| T13 | $$r\_e \neq 0$$ (rules out $$R\_e = \mathcal{O}$$ and $$S, S\_{a,r}, S\_{a,s} = \mathcal{O}$$; otherwise every ECDH mask in this transfer collapses to a constant function of $$\sigma$$) |
| T\_a1 | $$S\_{a,r} = r\_e \cdot K\_{\text{aud,r}}$$ (recipient-auditor ECDH shared secret, reuses ephemeral scalar) |
| T\_a2 | $$(m\_{v,r}, m\_{r,r}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_r}}, S\_{a,r}.x, \sigma)$$ (recipient-auditor channel masks) |
| T\_a3 | $$\tilde{v}\_{\text{aud,r}} = v\_{\text{tx}} + m\_{v,r}$$ (recipient-auditor encrypted transfer amount) |
| T\_a4 | $$\tilde{r}\_{\text{aud,r}} = r\_{\text{tx}} + m\_{r,r}$$ (recipient-auditor encrypted transfer randomness, enables Pedersen-opening reconstruction of $$C\_{\text{receive}}$$, see Section 8.1) |
| T\_a5 | $$S\_{a,s} = r\_e \cdot K\_{\text{aud,s}}$$ (sender-auditor ECDH shared secret, reuses ephemeral scalar) |
| T\_a6 | $$(m\_{v,s}, m\_{b,s}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_s}}, S\_{a,s}.x, \sigma)$$ (sender-auditor channel masks) |
| T\_a7 | $$\tilde{v}\_{\text{aud,s}} = v\_{\text{tx}} + m\_{v,s}$$ (sender-auditor encrypted transfer amount) |
| T\_a8 | $$\tilde{b}\_{\text{aud,s}} = (v\_A - v\_{\text{tx}}) + m\_{b,s}$$ (sender-auditor encrypted balance checkpoint) |

**Public inputs (24 fields, counting each Grumpkin point as two $$\mathbb{F}\_r$$ coordinates):**

| Input | Notes |
|:---|:---|
| $$C\_{\text{spend}}^A$$ | Loaded from sender's `spendable_commitment` |
| $$Y\_A$$ | Loaded from sender's `spending_public_key` |
| $$\text{PVK}\_B$$ | Loaded from recipient's `viewing_public_key`. Recipient must be registered. |
| $$\text{addr\\\_f}$$ | Loaded from instance storage; set once at construction (§3.5) |
| $$K\_{\text{aud,r}}$$ | Fetched from the auditor contract using recipient's `auditor_id` |
| $$K\_{\text{aud,s}}$$ | Fetched from the auditor contract using sender's `auditor_id` |
| $$C\_{\text{spend}}'$$, $$C\_{\text{tx}}$$, $$R\_e$$, $$\tilde{v}$$, $$\tilde{b}$$, $$\sigma$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$ | Prover-supplied; $$C\_{\text{spend}}'$$ written to sender's `spendable_commitment`, $$C\_{\text{tx}}$$ added to recipient's `receiving_commitment`, the rest emitted in event |

**Private witnesses:** $$sk\_A$$, $$vk\_A$$, $$v\_A$$, $$r\_A$$, $$v\_{\text{tx}}$$, $$r\_e$$.

**Post-verification:** The contract verifies the proof, then:
- Sets $$A$$`.spendable_commitment` $$= C\_{\text{spend}}'$$
- Adds to recipient: $$B$$`.receiving_commitment` $$\mathrel{+}= C\_{\text{tx}}$$
- Emits event with $$(R\_e, \tilde{v}, \sigma, \tilde{b}, \tilde{v}\_{\text{aud,r}}, \tilde{r}\_{\text{aud,r}}, \tilde{v}\_{\text{aud,s}}, \tilde{b}\_{\text{aud,s}})$$

**Recipient processing.** Upon observing the event, the recipient computes $$S = vk \cdot R\_e$$, derives amount and blinding. The decryption flow is independent of whether the sender was the owner or an spender.

### 7.7 Set Spender

The owner locks funds from their spendable balance into a per-spender escrow. The spender must be a registered account in the contract, so that $$Y\_{\text{op}}$$ (needed for $$dvk\_i$$ escrow) can be looked up from the spender's stored `spending_public_key`.

**Circuit constraints (SetSpender):**

| # | Constraint |
|:--|:---|
| S1 | $$Y = sk \cdot H$$ (owner key ownership) |
| S2 | $$vk = \text{Poseidon}(\delta\_{\text{vk}}, sk, \text{addr\\\_f})$$ (binds proof to contract) |
| S3 | Prover knows opening $$(v, r)$$ of $$C\_{\text{spend}}$$ |
| S4 | $$v \in [0, 2^{127})$$, $$v\_a \in [0, 2^{127})$$, $$v - v\_a \in [0, 2^{127})$$ (range validity, Section 2.6) |
| S5 | $$dvk\_i = \text{Poseidon}(\delta\_{\text{dvk}}, vk, \text{op}\_i)$$ (delegation key derivation; contract-bound via $$vk$$) |
| S6 | $$r\_a = \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a)$$ (allowance blinding) |
| S7 | $$C\_a = v\_a \cdot G + r\_a \cdot H$$ (allowance commitment) |
| S8 | $$\tilde{a} = v\_a + \text{Poseidon}(\delta\_{\text{enc\\\_allow}}, dvk\_i, \sigma\_a)$$ (encrypted allowance) |
| S9 | $$r' = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk, \sigma)$$ (new balance randomness) |
| S10 | $$C\_{\text{spend}}' = (v - v\_a) \cdot G + r' \cdot H$$ (new spendable balance) |
| S11 | $$\tilde{b} = (v - v\_a) + \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk, \sigma)$$ (encrypted balance) |
| S12 | Escrowed $$dvk\_i$$ correctly encrypts under $$Y\_{\text{op}}$$ via ECDH |
| S13 | $$r\_e \neq 0$$ (rules out $$R\_e = \mathcal{O}$$ and $$S\_{a,s} = \mathcal{O}$$; the same $$r\_e$$ is reused for the $$dvk\_i$$ escrow ECDH in Section 7.11, so this also rules out a trivial escrow shared secret) |
| S\_a1 | $$R\_e = r\_e \cdot H$$ (ephemeral key for auditor ECDH) |
| S\_a2 | $$S\_{a,s} = r\_e \cdot K\_{\text{aud,s}}$$ (owner-auditor ECDH shared secret) |
| S\_a3 | $$(m\_v, m\_b) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_s}}, S\_{a,s}.x, \sigma)$$ (owner-auditor channel masks) |
| S\_a4 | $$\tilde{v}\_{\text{aud,s}} = v\_a + m\_v$$ (owner-auditor encrypted escrow amount) |
| S\_a5 | $$\tilde{b}\_{\text{aud,s}} = (v - v\_a) + m\_b$$ (owner-auditor encrypted balance checkpoint) |

**Public inputs (24 fields):**

| Input | Notes |
|:---|:---|
| $$C\_{\text{spend}}$$ | Loaded from owner's `spendable_commitment` |
| $$Y$$ | Loaded from owner's `spending_public_key` |
| $$Y\_{\text{op}}$$ | Loaded from spender account's `spending_public_key`. Spender must be registered. |
| $$\text{op}\_i$$ | $$\text{address\\\_to\\\_field}$$(`spender` argument), computed per-call by the contract (§2.7) |
| $$\text{addr\\\_f}$$ | Loaded from instance storage; set once at construction (§3.5) |
| $$K\_{\text{aud,s}}$$ | Fetched from the auditor contract using owner's `auditor_id` |
| $$C\_{\text{spend}}'$$, $$C\_a$$, escrowed\_dvk, $$\tilde{b}$$, $$\tilde{a}$$, $$\sigma$$, $$\sigma\_a$$, $$R\_e$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$ | Prover-supplied; $$C\_{\text{spend}}'$$ written to owner's `spendable_commitment`, the delegation fields written to storage, the rest emitted in event |

**Private witnesses:** $$sk$$, $$vk$$, $$v$$, $$r$$, $$v\_a$$, $$r\_e$$.

**Post-verification:** The contract verifies the proof, sets `spendable_commitment` $$= C\_{\text{spend}}'$$ and stores the `SpenderDelegation`. Emits event with $$(R\_e, \sigma, \tilde{b}, \tilde{v}\_{\text{aud,s}}, \tilde{b}\_{\text{aud,s}})$$.

### 7.8 Spender Transfer

The spender transfers from the owner's escrowed allowance to a recipient.

**Circuit constraints (SpenderTransfer):**

| # | Constraint |
|:--|:---|
| O1 | $$Y\_{\text{op}} = sk\_{\text{op}} \cdot H$$ (spender key ownership) |
| O2 | Prover knows $$dvk\_i$$ and the opening $$(v\_a, r\_a)$$ of $$C\_a$$ |
| O3 | $$r\_a = \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a)$$ (allowance randomness matches stored state) |
| O4 | $$v\_a \in [0, 2^{127})$$, $$v\_{\text{tx}} \in [0, 2^{127})$$, $$v\_a - v\_{\text{tx}} \in [0, 2^{127})$$ (range validity, Section 2.6) |
| O5 | $$S = r\_e \cdot \text{PVK}\_{\text{recipient}}$$ (ECDH for recipient) |
| O6 | $$R\_e = r\_e \cdot H$$ |
| O7 | $$r\_{\text{tx}} = \text{Poseidon}(\delta\_{\text{tx\\\_blind}}, S.x, \sigma\_a)$$ (transfer blinding) |
| O8 | $$C\_{\text{tx}} = v\_{\text{tx}} \cdot G + r\_{\text{tx}} \cdot H$$ |
| O9 | $$\tilde{v} = v\_{\text{tx}} + \text{Poseidon}(\delta\_{\text{tx\\\_amount}}, S.x, \sigma\_a)$$ (encrypted amount) |
| O10 | $$r\_a' = \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a')$$ (new allowance randomness) |
| O11 | $$C\_a' = (v\_a - v\_{\text{tx}}) \cdot G + r\_a' \cdot H$$ (new allowance) |
| O12 | $$\tilde{a}' = (v\_a - v\_{\text{tx}}) + \text{Poseidon}(\delta\_{\text{enc\\\_allow}}, dvk\_i, \sigma\_a')$$ (encrypted allowance) |
| O13 | $$r\_e \neq 0$$ (rules out $$R\_e = \mathcal{O}$$ and $$S, S\_{a,r}, S\_{a,s} = \mathcal{O}$$; otherwise every ECDH mask in this transfer collapses to a constant function of $$\sigma\_a$$) |
| O\_a1 | $$S\_{a,r} = r\_e \cdot K\_{\text{aud,r}}$$ (recipient-auditor ECDH shared secret, reuses ephemeral scalar) |
| O\_a2 | $$(m\_{v,r}, m\_{r,r}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_r}}, S\_{a,r}.x, \sigma\_a)$$ (recipient-auditor channel masks) |
| O\_a3 | $$\tilde{v}\_{\text{aud,r}} = v\_{\text{tx}} + m\_{v,r}$$ (recipient-auditor encrypted transfer amount) |
| O\_a4 | $$\tilde{r}\_{\text{aud,r}} = r\_{\text{tx}} + m\_{r,r}$$ (recipient-auditor encrypted transfer randomness, enables Pedersen-opening reconstruction of $$C\_{\text{receive}}$$, see Section 8.1) |
| O\_a5 | $$S\_{a,s} = r\_e \cdot K\_{\text{aud,s}}$$ (owner-auditor ECDH shared secret, reuses ephemeral scalar) |
| O\_a6 | $$(m\_{v,s}, m\_{a,s}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_s}}, S\_{a,s}.x, \sigma\_a)$$ (owner-auditor channel masks) |
| O\_a7 | $$\tilde{v}\_{\text{aud,s}} = v\_{\text{tx}} + m\_{v,s}$$ (owner-auditor encrypted transfer amount) |
| O\_a8 | $$\tilde{a}\_{\text{aud,s}} = (v\_a - v\_{\text{tx}}) + m\_{a,s}$$ (owner-auditor encrypted post-transfer allowance) |

**Public inputs (24 fields):**

| Input | Notes |
|:---|:---|
| $$C\_a$$, $$\sigma\_a$$ | Loaded from the `(from, spender)` delegation entry |
| $$Y\_{\text{op}}$$ | Loaded from spender's `spending_public_key`; matches the auth principal |
| $$\text{PVK}\_{\text{recipient}}$$ | Loaded from recipient's `viewing_public_key` |
| $$K\_{\text{aud,r}}$$ | Fetched from the auditor contract using recipient's `auditor_id` |
| $$K\_{\text{aud,s}}$$ | Fetched from the auditor contract using **owner's** `auditor_id`, not spender's. The visibility model points balance- and allowance-checkpoint ciphertexts at the funds' owner. |
| $$C\_a'$$, $$C\_{\text{tx}}$$, $$R\_e$$, $$\tilde{v}$$, $$\tilde{a}'$$, $$\sigma\_a'$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{a}\_{\text{aud,s}}$$ | Prover-supplied; allowance fields written to delegation storage, $$C\_{\text{tx}}$$ added to recipient's `receiving_commitment`, the rest emitted in event |

**Private witnesses:** $$sk\_{\text{op}}$$, $$dvk\_i$$, $$v\_a$$, $$r\_a$$ (single-limb $$\mathbb{F}\_r$$; pinned by O3 to $$\text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a)$$), $$v\_{\text{tx}}$$, $$r\_e$$.

**Post-verification:** The contract checks `ledger.sequence() <= live_until_ledger`, updates `allowance_commitment`, `a_tilde`, stores `new_allowance_salt`, and adds $$C\_{\text{tx}}$$ to the recipient's `receiving_commitment`. Emits event with $$(R\_e, \tilde{v}, \sigma\_a, \tilde{v}\_{\text{aud,r}}, \tilde{r}\_{\text{aud,r}}, \tilde{v}\_{\text{aud,s}}, \tilde{a}\_{\text{aud,s}})$$.

**Recipient uniformity.** The recipient processes the incoming transfer identically to a direct transfer: compute $$S = vk \cdot R\_e$$, derive amount and blinding. The decryption flow is independent of whether the sender was the owner or an spender.

**Contract binding.** Unlike owner-initiated circuits, the SpenderTransfer circuit does not constrain the $$vk$$ derivation (the spender has no access to the owner's $$sk$$). Contract binding is instead inherited indirectly through the allowance commitment chain: the SetSpender circuit derives $$dvk\_i$$ from the contract-specific $$vk$$ (S2, S5), which determines $$r\_a$$ (S6) and thus $$C\_a$$ (S7). The SpenderTransfer circuit verifies $$dvk\_i$$ against $$C\_a$$ via $$\sigma\_a$$ (O3). Since $$C\_a$$ is a public input and was constructed with contract-specific randomness, a proof generated against one contract's $$C\_a$$ cannot verify against another's.

### 7.9 Revoke Spender

The owner reclaims the remaining escrowed allowance.

**Circuit constraints (RevokeSpender):**

| # | Constraint |
|:--|:---|
| V1 | $$Y = sk \cdot H$$ (owner key ownership) |
| V2 | $$vk = \text{Poseidon}(\delta\_{\text{vk}}, sk, \text{addr\\\_f})$$ (binds proof to contract) |
| V3 | $$dvk\_i = \text{Poseidon}(\delta\_{\text{dvk}}, vk, \text{op}\_i)$$ |
| V4 | Prover knows opening $$(v\_a, r\_a)$$ of $$C\_a$$, with $$r\_a = \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a)$$ (allowance randomness matches stored state, mirrors O3) |
| V5 | Prover knows opening $$(v\_s, r\_s)$$ of $$C\_{\text{spend}}$$ |
| V6 | $$r' = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk, \sigma)$$ |
| V7 | $$C\_{\text{spend}}' = (v\_s + v\_a) \cdot G + r' \cdot H$$ |
| V8 | $$\tilde{b} = (v\_s + v\_a) + \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk, \sigma)$$ |
| V9 | $$v\_s \in [0, 2^{127})$$, $$v\_a \in [0, 2^{127})$$, $$v\_s + v\_a \in [0, 2^{127})$$ (range validity, Section 2.6) |
| V10 | $$r\_e \neq 0$$ (rules out $$R\_e = \mathcal{O}$$ and $$S\_{a,s} = \mathcal{O}$$, which would reduce $$m\_v$$ and $$m\_b$$ to constant functions of $$\sigma$$) |
| V\_a1 | $$R\_e = r\_e \cdot H$$ (ephemeral key for auditor ECDH) |
| V\_a2 | $$S\_{a,s} = r\_e \cdot K\_{\text{aud,s}}$$ (owner-auditor ECDH shared secret) |
| V\_a3 | $$(m\_v, m\_b) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_s}}, S\_{a,s}.x, \sigma)$$ (owner-auditor channel masks) |
| V\_a4 | $$\tilde{v}\_{\text{aud,s}} = v\_a + m\_v$$ (owner-auditor encrypted reclaimed amount) |
| V\_a5 | $$\tilde{b}\_{\text{aud,s}} = (v\_s + v\_a) + m\_b$$ (owner-auditor encrypted balance checkpoint) |

**Public inputs (19 fields):**

| Input | Notes |
|:---|:---|
| $$C\_{\text{spend}}$$ | Loaded from owner's `spendable_commitment` |
| $$C\_a$$, $$\sigma\_a$$ | Loaded from the `(account, spender)` delegation entry |
| $$Y$$ | Loaded from owner's `spending_public_key` |
| $$\text{op}\_i$$ | $$\text{address\\\_to\\\_field}$$(`spender` argument), computed per-call by the contract (§2.7) |
| $$\text{addr\\\_f}$$ | Loaded from instance storage; set once at construction (§3.5) |
| $$K\_{\text{aud,s}}$$ | Fetched from the auditor contract using owner's `auditor_id` |
| $$C\_{\text{spend}}'$$, $$\tilde{b}$$, $$\sigma$$, $$R\_e$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$ | Prover-supplied; $$C\_{\text{spend}}'$$ written to owner's `spendable_commitment`, delegation entry deleted, the rest emitted in event |

**Private witnesses:** $$sk$$, $$vk$$, $$dvk\_i$$, $$v\_a$$, $$r\_a$$, $$v\_s$$, $$r\_s$$ (input spendable-balance blinding, encoded as a single $$\mathbb{F}\_r$$ `Field`; see §10.4 *Post-merge witness availability* for the acknowledged $$2^{-127}$$-per-merge case affecting $$r\_s$$), $$r\_e$$.

**Post-verification:** The contract verifies the proof, sets `spendable_commitment` $$= C\_{\text{spend}}'$$ and deletes the delegation. Emits event with $$(R\_e, \sigma, \tilde{b}, \tilde{v}\_{\text{aud,s}}, \tilde{b}\_{\text{aud,s}})$$.

### 7.10 Owner Operations with Active Spenders

Owner transfers, withdrawals, and merges proceed identically to the no-spender case. Spender allowances are independently escrowed - no synchronization is needed. The owner's spendable balance and spender allowances are fully isolated.

### 7.11 Delegation Key Escrow

At `set_spender`, the owner escrows $$dvk\_i$$ to the spender on-chain via ECDH, eliminating off-chain key sharing:

1. Owner picks ephemeral $$r\_e$$ (reused from the `set_spender` proof's outer ECDH; see §5.3, "Why reusing $$r\_e$$ is safe") and computes $$R = r\_e \cdot H$$.
2. Shared secret: $$s = (r\_e \cdot Y\_{\text{op}}).x$$
3. Escrowed key: $$\text{escrowed\\\_dvk} = (R.x, \\; \text{Poseidon}(\delta\_{\text{esc\\\_dvk}}, s, \text{op}\_i) + dvk\_i)$$

**Encoding.** `escrowed_dvk` is a `BytesN<64>` consisting of two 32-byte $$\mathbb{F}\_r$$ representatives: `R_x` (the $$x$$-coordinate of $$R$$) followed by `dvk_cipher` (the masked $$dvk\_i$$). $$R.y$$ is **not** stored. This is sound because ECDH on Grumpkin recovers only the $$x$$-coordinate of the shared secret: $$\pm R$$ both have $$x = R.x$$, and $$sk\_{\text{op}} \cdot R$$ and $$sk\_{\text{op}} \cdot (-R)$$ are inverse points with the same $$x$$-coordinate. The spender reconstructs the curve point by solving $$y^2 = R.x^3 - 17$$ in $$\mathbb{F}\_r$$, picks either root, and proceeds; both choices produce the same $$s = (sk\_{\text{op}} \cdot R).x$$ and therefore the same Poseidon mask.

The spender decrypts using $$sk\_{\text{op}}$$. The `set_spender` proof enforces escrow correctness via constraint S12, which expands to three sub-constraints over the prover-supplied `escrowed_dvk = (R_x, dvk_cipher)`:

- $$R\_x = (r\_e \cdot H).x$$
- $$s\_{\text{esc}} = (r\_e \cdot Y\_{\text{op}}).x$$
- $$\text{dvk\\\_cipher} = \text{Poseidon}(\delta\_{\text{esc\\\_dvk}}, s\_{\text{esc}}, \text{op}\_i) + dvk\_i$$

The $$r\_e$$ here is the same scalar S\_a1 commits to ($$R\_e = r\_e \cdot H$$), so the escrow's $$R\_x$$ and the auditor channel's $$R\_e.x$$ are forced equal.

### 7.12 Expiry and Revert Safety

Delegations use persistent storage and persist until explicitly revoked. `live_until_ledger` is checked on every spender transfer. Allowance randomness includes `allowance_salt` to prevent deterministic-randomness reuse after reverted transactions.

---

<!-- This specification is split into two files because GitHub stops rendering
     LaTeX math after ~750 expressions per page. Keep each part under that
     budget when adding content. -->

*The specification continues in [DESIGN_cont.md](./DESIGN_cont.md) with Sections 8-13:*
*Auditing, Security Analysis, Proof System, Interface, Dependencies, and Domain Separation Constants.*
