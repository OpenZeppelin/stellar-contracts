# Confidential Token

## Abstract

We present a confidential token for Soroban that adds private balances and transfers to any SEP-41 token. Balances are stored as unchunked Pedersen commitments as single elliptic curve points, and updated homomorphically by the contract without decryption. Zero-knowledge proofs (Noir/UltraHonk) accompany each spending operation to prove correctness without revealing amounts. Transfer recipients and auditors recover amounts and blinding factors via per-transfer ephemeral ECDH key agreement over Grumpkin. A dual-balance model (spendable/receiving) prevents griefing: incoming transfers accumulate in a receiving commitment that third parties cannot use to invalidate in-flight spend proofs. A dual-auditor model provides per-account audit visibility: each transfer produces ciphertexts under two auditor keys, giving the recipient's auditor the transfer amount and the sender's auditor the transfer amount plus the sender's post-transfer balance (or post-transfer allowance for operator transfers), enabling real-time auditing. Account owners can delegate spending to time-limited operators via escrowed allowances with derived delegation viewing keys. The system uses 6 Noir circuits, and works seamlessly with Soroban's BN254 host functions (leveraging the recently added CAP-80), and requires approximately 288 bytes of on-chain storage per account.

---

## Project Documents

This project is composed of the following documents:

- Confidential Token (this document)
- Confidential Token: [Compliance Extensions](./COMPLIANCE.md)
- Confidential Token: User Flows Overview (to be added)
- Indexing and Off-Chain State Recovery (to be added)
- SDK (to be added)

---

## 1. Introduction

### 1.1 Background

Confidential transfers on blockchain require balances and amounts to be hidden from public observers while remaining verifiable by the contract. The standard approach uses additively homomorphic encryption - the contract operates on ciphertexts (adding deposits, subtracting transfers) without learning the underlying values, and zero-knowledge proofs guarantee that operations are valid (sufficient funds, consistent encryption, non-negative balances).

This document defines a standalone Soroban contract that wraps any SEP-41 token to provide confidential balances and transfers. It is not an extension to the fungible token standard; it is a separate contract that holds tokens on behalf of users and manages encrypted state independently. The wrapping approach is chosen over native token integration for three reasons: it works with existing assets, it can evolve independently of the token standard, and it keeps confidentiality complexity separate from the underlying token.

### 1.2 Design Goals

**Amount and balance confidentiality.** An observer can see that account $A$ transferred to account $B$, and how much each party deposited or withdrew, but not how much moved between them or what their balances are. The system provides confidentiality, not anonymity - sender and recipient addresses remain visible on-chain.

**Griefing resistance.** A third party must not be able to prevent an account owner from spending by spamming transfers. The balance model must isolate incoming funds from the state that spend proofs reference.

**No mandatory maintenance operations.** Receiving funds should not require the owner to perform a costly ZK proof before those funds become accessible. The merge operation that makes received funds spendable must be lightweight and non-frontrunnable.

**Selective auditing.** Each account selects an auditor at registration. Each transfer produces ciphertexts under two auditor keys: the recipient's auditor receives the transfer amount, while the sender's auditor receives the transfer amount and sender's post-transfer balance. This dual-auditor model enables real-time visibility for both parties' auditors without granting access to uninvolved accounts' historical balances.

**Delegated spending.** Account owners can authorize operators (separate addresses) to spend from escrowed allowances, enabling use cases like automated market makers and custodial services without sharing the owner's secret key.

### 1.3 Approach

The design is built on three interlocking mechanisms:

1. **Pedersen commitments.** Each balance is a single elliptic curve point $C = v \cdot G + r \cdot H$. There is no chunking, no discrete logarithm to solve for decryption, and no overflow from repeated homomorphic additions. The owner maintains the commitment opening $(v, r)$ as local wallet state, updated incrementally from on-chain events.

2. **ECDH-derived blinding.** When a sender transfers to a recipient, the blinding factor of the transfer commitment is derived from an ephemeral Diffie-Hellman key exchange with the recipient's public viewing key. The circuit enforces correct derivation, ensuring the recipient can always compute the blinding. The same ephemeral scalar is reused for an ECDH exchange with auditors' public key.

3. **Proof-less merge.** Incoming funds accumulate in a receiving balance that is separate from the spendable balance. To make received funds spendable, the owner authorizes a merge - no ZK proof is required. Since merge requires owner authorization and incoming transfers touch only the receiving balance, neither the spend path nor the merge path can be front-run by a third party.

Six Noir/UltraHonk circuits cover registration, withdrawal, confidential transfer, operator transfer, operator delegation, and operator revocation. The proof system leverages the Grumpkin–BN254 curve cycle: Grumpkin point arithmetic is native inside Noir circuits (no field emulation), while Soroban natively supports BN254 operations for UltraHonk proof verification.

---

## 2. Preliminaries

### 2.1 Notation

| Symbol | Definition |
|:---|:---|
| $\mathbb{G}$ | Grumpkin elliptic curve group (prime order) |
| $\mathbb{F}_r$ | BN254 scalar field $= \mathbb{G}$'s base field |
| $\mathbb{F}_q$ | BN254 base field $= \mathbb{G}$'s scalar field |
| $G, H \in \mathbb{G}$ | Independent generators with no known discrete log relation |
| $\mathcal{O}$ | Identity element (point at infinity), encoded as $(0, 0)$ on-chain |
| $P.x$ | The $x$-coordinate of point $P$, an element of $\mathbb{F}_r$ |
| $\text{Poseidon}(\cdot)$ | Poseidon2 hash function over $\mathbb{F}_r$ (Section 2.5) |
| $\delta_{\ast}$ | Domain separation constants (subscript identifies the domain) |
| $[n]$ | The set $\{0, 1, \ldots, n-1\}$ |

### 2.2 Grumpkin–BN254 Cycle

Grumpkin is defined by $y^2 = x^3 - 17$ over $\mathbb{F}_r$. It forms a 2-cycle with BN254:

$$\text{base}(\mathbb{G}) = \mathbb{F}_r^{\text{BN254}}, \qquad \text{scalar}(\mathbb{G}) = \mathbb{F}_q^{\text{BN254}}$$

A Grumpkin point is a pair $(x, y) \in \mathbb{F}_r^2$. Noir's native `Field` type is $\mathbb{F}_r$, so Grumpkin point arithmetic inside UltraHonk circuits incurs no non-native field emulation. On-chain, the Soroban host provides BN254 $\mathbb{F}_r$ arithmetic (`bn254_fr_{add, sub, mul, inv}` via CAP-80), which suffices for Grumpkin affine point operations.

**Scalar sampling.** Grumpkin scalars live in $\mathbb{F}_q$, which is slightly larger than $\mathbb{F}_r$. All secret scalars in this design ($sk$, $r_e$, $\sigma$, $\sigma_a$) are sampled by the **rejection sampling** procedure, which produces a uniform draw from $\mathbb{F}_r$:

1. Draw 32 bytes (256 bits) from a CSPRNG.
2. Mask the top 2 bits to zero, yielding a 254-bit candidate $x \in [0, 2^{254})$.
3. If $x \geq r$, reject and return to step 1.
4. If the call site requires $x \neq 0$ and $x = 0$, reject and return to step 1.
5. Output $x$ in its canonical form -- as a Noir `Field` for in-circuit use, or as 32 big-endian bytes (`BytesN<32>`) for storage and event emission. Per §10.8 / §11 this canonical encoding is what the Soroban host's `bn254_fr_*` deserialization accepts; non-canonical values (i.e. $x \geq r$) are rejected at the host boundary.

### 2.3 Pedersen Commitments

A Pedersen commitment to a value $v$ with a blinding factor $r$, viewed as scalars in Grumpkin's scalar field $\mathbb{F}_q$, is:

$$\text{Com}(v, r) = v \cdot G + r \cdot H$$

In this design both $v$ and $r$ are drawn from $\mathbb{F}_r \subset \mathbb{F}_q$ (§2.2): $v$ is a non-negative integer below $2^{127}$ (§2.6) and $r$ is a Poseidon2 output or an $\mathbb{F}_r$-sampled CSPRNG draw; the group law operates in $\mathbb{F}_q$.

**Binding.** Finding $(v', r') \neq (v, r)$ such that $\text{Com}(v, r) = \text{Com}(v', r')$ requires computing $\log_G H$, which is infeasible under the discrete logarithm assumption.

**Hiding.** For any $v$, the commitment $\text{Com}(v, r)$ with uniformly random $r \in \mathbb{F}_q$ is uniformly distributed over $\mathbb{G}$, revealing nothing about $v$. Sampling $r$ from $\mathbb{F}_r \subset \mathbb{F}_q$ instead of full $\mathbb{F}_q$ (§2.2) makes the commitment distribution **statistically close** to uniform over $\mathbb{G}$, with total-variation distance bounded by $(|\mathbb{F}_q| - |\mathbb{F}_r|)/|\mathbb{F}_q| \approx 2^{-127}$.

**Homomorphism.** $\text{Com}(v_1, r_1) + \text{Com}(v_2, r_2) = \text{Com}(v_1 + v_2, r_1 + r_2)$. Scalar addition in the commitment relation is over $\mathbb{F}_q^{\text{BN254}}$ -- the scalar field of $\mathbb{G}$, equivalently the order of the Grumpkin group. Since every committed value is bounded by $2^{127}$ (§2.6) and the number of additions across the lifetime of any one commitment is far below $2^{127}$, the value component never wraps in $\mathbb{F}_q$ and the homomorphic relation holds in $\mathbb{Z}$ for values. The blinding component is added in $\mathbb{F}_q$ and may reduce mod $q$ on accumulation; the only place this has operational consequences is the wallet's post-merge spend witness, where the canonical $\mathbb{F}_q$ representative of $r_s + r_r$ can land in $[r, q)$ with probability bounded at $(q-r)/q \approx 2^{-127}$ per merge (see §10.4 *Post-merge witness availability*).

**Generators.** $G$ and $H$ are inherited from Barretenberg's standard Grumpkin Pedersen instantiation (the same generators that the toolchain's `pedersen_commitment` and `pedersen_hash` primitives use). Their provenance is part of the toolchain's audited surface, so the contract inherits both the generators and the soundness assumption that $\log_G H$ is unknown. The Noir circuits import them as `embedded_curve_ops::generator()`.

### 2.4 Elliptic Curve Diffie-Hellman

Given a long-term keypair $(a, A = a \cdot H)$ and an ephemeral keypair $(r_e, R_e = r_e \cdot H)$, the ECDH shared secret is:

$$S = r_e \cdot A = a \cdot R_e = a \cdot r_e \cdot H \in \mathbb{G}$$

Both parties compute $S$ independently. We extract the scalar $s = S.x \in \mathbb{F}_r$ for use as a Poseidon input.

### 2.5 Poseidon2 Hash

The system uses **Poseidon2**, the algebraic hash function native to Noir's standard library and implemented as a custom gate in Barretenberg. For the complete parameter specification of the Noir/Barretenberg instantiation, see:

- [Poseidon2 paper](https://eprint.iacr.org/2023/323) (Grassi, Khovratovich, Schofnegger, AFRICACRYPT 2023) - parameter derivation and security analysis
- [Barretenberg `poseidon2_params.hpp`](https://github.com/AztecProtocol/aztec-packages/blob/next/barretenberg/cpp/src/barretenberg/crypto/poseidon2/poseidon2_params.hpp) - concrete round constants and matrix entries
- [HorizenLabs reference implementation](https://github.com/HorizenLabs/poseidon2) - parameter generation script (`poseidon2_rust_params.sage`)
- [Noir stdlib `hash/mod.nr`](https://github.com/noir-lang/noir/blob/master/noir_stdlib/src/hash/mod.nr) - sponge construction wrapping the `Poseidon2Permutation` ACIR opcode

**Usage in this system:**

- Key derivation: $vk = \text{Poseidon2}(\delta_{\text{vk}}, sk, \text{wrap})$
- Randomness derivation: $r = \text{Poseidon2}(\delta_{\text{spend\\\_r}}, vk, \sigma)$
- Symmetric encryption: $\tilde{v} = v + \text{Poseidon2}(\delta_{\text{tx\\\_amount}}, s, \sigma)$
- Domain separation: each invocation includes a leading constant $\delta$ to prevent cross-context collisions

**Sponge mode for auditor channels.** The per-transfer auditor ciphertexts (Section 8) use Poseidon2 in sponge mode. A single absorb of $(\delta_{\text{channel}}, S.x, \sigma)$ is followed by $n$ sequential squeezes producing $(m_1, \ldots, m_n) \in \mathbb{F}_r^n$, denoted $\text{SpongeSqueeze}_n(\delta_{\text{channel}}, S.x, \sigma)$. Two channel tags are used: $\delta_{\text{aud\\\_s}}$ for the sender-auditor channel keyed by $S_{a,s}.x = (r_e \cdot K_{\text{aud,s}}).x$, and $\delta_{\text{aud\\\_r}}$ for the recipient-auditor channel keyed by $S_{a,r}.x = (r_e \cdot K_{\text{aud,r}}).x$.

Squeeze order is canonical. Where $n = 2$, the first squeezed mask is the amount mask and the second is the balance or randomness mask, fixed per operation by the formulas in Sections 7 and 8.

All references to "Poseidon" in this document denote this Poseidon2 instantiation.

### 2.6 Integer Embedding and Range Proofs

**The problem.** Noir circuits operate over $\mathbb{F}_r$, where every element is a non-negative integer modulo $r \approx 2^{254}$. The statement "$v \geq 0$" is vacuously true for all $v \in \mathbb{F}_r$, and "$v_A \geq v_{\text{tx}}$" is undefined without specifying how integers are embedded in the field. Without explicit range constraints, a prover can claim a balance of 1 and transfer 1,000,000: the "new balance" $1 - 1{,}000{,}000 \equiv r - 999{,}999 \pmod{r}$ is a valid field element, and the commitment equation holds. The attacker has minted 999,999 tokens.

**Integer embedding.** We define a canonical embedding $\iota: [0, 2^{127}) \to \mathbb{F}_r$ mapping non-negative integers to their natural field representatives. A field element $x \in \mathbb{F}_r$ represents a valid balance or transfer amount if and only if $x < 2^{127}$.

**Range proof mechanism.** A range proof for $x \in [0, 2^{127})$ is implemented by decomposing $x$ into 127 bits inside the circuit and checking the recomposition:

$$x = \sum_{i=0}^{126} b_i \cdot 2^i, \qquad b_i \in \{0, 1\} \;\forall\, i$$

Each $b_i$ is constrained to be Boolean ($b_i \cdot (b_i - 1) = 0$) and the recomposition is checked against $x$. Noir's standard library exposes this directly:

```noir
// Range check: [0, 2^127)
value.assert_max_bit_size::<127>();
```

**Sufficiency argument.** If the prover supplies $v_A$ and $v_{\text{tx}}$ such that the circuit verifies:

1. $v_A \in [0, 2^{127})$ (the opening of $C_{\text{spend}}$)
2. $v_{\text{tx}} \in [0, 2^{127})$ (the transfer amount)
3. $v_A - v_{\text{tx}} \in [0, 2^{127})$ (the new balance)

then $v_A - v_{\text{tx}}$ is a non-negative integer less than $2^{127}$, which is only possible if the integer subtraction did not underflow. This is because $v_A < 2^{127}$ and $v_{\text{tx}} < 2^{127}$, so if $v_A < v_{\text{tx}}$ as integers, then $v_A - v_{\text{tx}} \pmod{r}$ would be $r - (v_{\text{tx}} - v_A)$, which is at least $r - 2^{127} \gg 2^{127}$, failing constraint (3).

**Value capacity.** Both balances and transfer amounts are constrained to $[0, 2^{127})$. These bounds are enforced in every circuit that manipulates values. The bound is exactly the SEP-41 non-negative `i128` range, so the contract's value domain matches the underlying token's domain by construction. The gap between $2^{127}$ and $|\mathbb{F}_r| \approx 2^{254}$ ensures that modular wrap-around is detectable by the range check.

**Receiving balance (unproven accumulation).** The receiving balance $C_{\text{receive}}$ is updated by contract-side point addition without any proof from the recipient. Therefore, the receiving balance's committed value $v_r$ is never directly range-checked by any circuit.

This is safe because $v_r$ is *indirectly* bounded:

1. Each deposit adds a public `i128` amount validated by the contract ($\ge 0$, hence $< 2^{127}$).
2. Each incoming transfer adds a commitment whose sender circuit proved $v_{\text{tx}} \in [0, 2^{127})$ (constraint T4 / O4).
3. All tokens in the contract entered through deposits, so the sum of all committed values is bounded by the underlying token's total supply ($< 2^{127}$). No single account can receive more than the total supply.
4. For the field-arithmetic concern (could $v_r$ reach $r$ and wrap around), that would require $r / 2^{127} > 2^{127}$ incoming transfers, which is computationally infeasible.

When the owner spends after a merge, the spend proof constrains the full post-merge opening: $v_s + v_r \in [0, 2^{127})$ (via constraint W4 or T4 on the spendable balance). This provides an implicit range check at the next spend boundary.

### 2.7 Address-to-Field Encoding

In Soroban, the SDK's `Address` host type covers exactly the two `ScAddressType` variants the contract interacts with as actors: `Account` (Stellar ed25519 account) and `Contract` (Soroban contract instance). The protocol encodes those addresses via their **canonical Stellar strkey** (SEP-23) representation:

$$\text{enc}(a) \;=\; \text{Address::to\\\_string}(a)\text{.to\\\_bytes}() \;\in\; \{\text{ASCII}\}^{56}$$

This is the 56-character ASCII strkey produced by the host's `address_to_strkey` function: a 1-byte version tag (`G` = `0x47` for `Account`, `C` = `0x43` for `Contract`), a 32-byte payload (ed25519 public key or contract hash), and a 2-byte CRC16 checksum, all base32-encoded into 56 ASCII characters. The byte string is fixed-length, canonical, and reproducible in every Stellar SDK via the language's stellar-strkey library; the protocol commits to these 56 ASCII bytes.

The Poseidon-compressed Field encoding splits the 56-byte string into two 28-byte limbs (each $\le 2^{224} \ll r \approx 2^{254}$, hence trivially in $\mathbb{F}_r$):

$$\text{address\\\_to\\\_field}(a) \;=\; \text{Poseidon2}\big(\delta_{\text{addr}}, \;\text{lo}(a), \;\text{hi}(a)\big)$$

where $\text{lo}(a) = \sum_{i=0}^{27} 256^{\,i} \cdot \text{enc}(a)[i]$ and $\text{hi}(a) = \sum_{i=0}^{27} 256^{\,i} \cdot \text{enc}(a)[28 + i]$ interpret the lower and upper 28 bytes of the strkey in little-endian byte order.

The contract, the SDK, the wallet, and any indexer reproduce the same Field value from the same Address by running their language's stellar-strkey encoder over the same `(version, payload)` pair and applying the same limb decomposition. No implementation needs to handle `ScAddress` XDR or the inner `AccountID` / `ContractID` union nesting.

**Usage sites.**

| Site | When computed | Storage |
|:---|:---|:---|
| $\text{wrap}$ | Once, by the contract's `__constructor` over `env.current_contract_address()` | Stored as a single Field in the contract's **instance storage** (§3.5); read on every proof verification |
| $\text{op}_i$ | Per-call, by the contract at `set_operator` and `revoke_operator` over the `operator` argument | Not stored; recomputed each call. The circuit binds it via S5 / V3 |

---

## 3. System Model

### 3.1 Components

The system comprises three contracts deployed on Soroban:

**Token contract.** Holds SEP-41 token balances, manages encrypted account state, and delegates proof verification via cross-contract calls. Performs Grumpkin point arithmetic through $\mathbb{F}_r$ host operations for homomorphic balance updates.

**Verifier contract.** A modified [UltraHonk verifier](https://github.com/indextree/ultrahonk_soroban_contract) storing one verification key per circuit type. Accepts a circuit identifier, serialized public inputs, and a proof blob; returns success or failure.

**Auditor contract.** Manages auditor encryption keys independently of the contract. One auditor contract serves multiple token contracts. Stores Grumpkin public keys as full affine points $(x, y)$ indexed by `auditor_id`. The contract validates that stored keys are non-identity curve points; a zero or identity key would make ECDH-derived ciphertexts trivially decryptable (since $\sigma$ is public). The contract fetches the active auditor key at operation time and passes it as a public input to the relevant circuit.

### 3.2 Threat Model

- The contract execution environment is trusted for correctness but not for privacy: all on-chain state and invocation inputs are public.
- Proof verification is sound: a valid proof guarantees the proven statement holds. This depends on the UltraHonk knowledge soundness assumption *and* the integrity of the Structured Reference String (Section 10.6).
- The discrete logarithm problem on Grumpkin is hard.
- Poseidon2 (Section 2.5) is a pseudorandom function (PRF) and is preimage-resistant over $\mathbb{F}_r$ at the parameterized round count ($R_F = 8$, $R_P = 56$, 128-bit security target).
- Third parties may submit arbitrary transactions, including spam transfers to any registered account.

### 3.3 Trust Assumptions

The contract, verifier, and auditor contracts are trusted code. Users trust that the verification keys embedded in the verifier correspond to the correct circuits and were derived from a honestly generated Structured Reference String (Section 10.6). The auditor is trusted to protect its decryption key and exercise access only upon legitimate regulatory request.

### 3.4 Underlying Token Assumptions

The contract holds units of an underlying SEP-41 token on behalf of its users. The confidential accounting invariant (Section 9.3) implicitly assumes:

$$\sum_i v_{\text{committed},i} \;\le\; \text{token.balance}(\text{contract})$$

i.e., the total committed value across all confidential accounts never exceeds the public token balance held by the contract. The deployer's choice of underlying token determines whether that invariant is actually preserved over time. The contract itself does not, and cannot, defend against every misbehavior of the wrapped asset.

**Required properties of the underlying token.**

- *Non-rebasing.* The token's balance attributed to the contract address changes only as a result of explicit operations that the contract itself originated. Tokens whose balances change as a function of supply, oracle data, or external triggers break the accounting invariant and are unsupported.
- *No fee-on-transfer.* `token.transfer(from, to, amount)` MUST move exactly `amount` units. A fee deducted in transit would leave the contract's confidential accounting larger than its public backing.
- *Deterministic revert.* A failed `token.transfer` MUST cause the enclosing contract invocation (`deposit` or `withdraw`) to revert atomically, so confidential state is never updated against a token transfer that did not happen.
- *Underlying clawback / freeze / deauthorization.* If the underlying SEP-41 (especially a Stellar Asset Contract) supports issuer-level clawback or freeze that can reduce or block the contract's holdings, confidential accounting at the contract layer may temporarily or permanently exceed the contract's accessible backing. This is an operational risk borne by the deployer's choice of underlying token. The token layer offers its own freeze and per-account clawback flows that operate inside the confidential surface; see [COMPLIANCE.md](./COMPLIANCE.md) §2 (contract-level freeze) and §5 (admin + auditor clawback). [COMPLIANCE.md](./COMPLIANCE.md) §2.2 additionally specifies SAC authorization passthrough, which composes the contract's freeze with the issuer's freeze without requiring the admin to mirror state.

**Non-negativity check.** The contract's public interface uses `i128` end-to-end, matching SEP-41. Every entrypoint that accepts a public amount (`deposit`, `withdraw`) MUST reject `amount < 0` and revert. The in-circuit range constraint (Section 2.6) bounds the same value at $2^{127}$ from above; together they pin the contract's value domain to $[0, 2^{127}) = [0, \text{i128::MAX}]$, matching SEP-41 exactly. No conversion at the SEP-41 boundary is needed.

### 3.5 Governance and Upgradeability

The constructor binds the contract to fixed `admin`, `token`, `verifier`, and `auditor` addresses. It additionally computes and stores $\text{wrap} = \text{address\\\_to\\\_field}(\text{env.current\\\_contract\\\_address}())$ (§2.7) in **instance storage** as a single canonical $\mathbb{F}_r$ Field; this is the value every owner-initiated proof references via constraints R2 / W2 / T2 / S2 / V2. The compressed `wrap` Field is computed once at construction (not recomputed per call) to ensure all proofs across the contract's lifetime bind to the same Field representative of the contract's address. Beyond that, this specification does not prescribe a governance policy for upgrading these components or for rotating per-circuit verification keys. Concrete deployments differ widely in operator structure, regulatory posture, and emergency-response requirements, so these decisions are deliberately left to implementers.

Questions an implementer must answer:

- May `admin` replace the `verifier` contract, or the `auditor` contract after deployment?
- Are per-circuit verification keys immutable for the lifetime of the deployment, or may they be updated?
- If any of the above is upgradeable, what authorization (single key, multisig), timelock, and event-emission rules apply?
- How do users independently reproduce a deployed VK from circuit source, toolchain, and SRS (Section 10.6)?

**Recommendation.** The strongest soundness posture is full immutability: `token`, `verifier`, `auditor`, and per-circuit verification keys all fixed at deployment, with any circuit or verifier change requiring a fresh deployment and an explicit user-side migration. Where operational realities make full immutability impractical (for example, a discovered soundness bug in a circuit or verifier that needs a fast fix), implementers may expose admin-guarded upgrade entrypoints for the `verifier` address or per-circuit VKs. In that case the upgrade path should be gated.

---

## 4. Key Hierarchy

All keys derive from a single spending secret $sk \in \mathbb{F}_r$.

### 4.1 Spending Key

$$Y = sk \cdot H$$

The spending public key is stored on-chain at registration. Knowledge of $sk$ is required to authorize transfers, withdrawals, operator delegations, and merges.

### 4.2 Viewing Key

$$vk = \text{Poseidon}(\delta_{\text{vk}}, sk, \text{wrap})$$

A scalar in $\mathbb{F}_r$, unique per $(sk, \text{wrap})$ pair. Enables balance decryption without spending authority. Cannot recover $sk$ (Poseidon preimage resistance). Because $\text{wrap}$ is bound into the derivation, proofs that constrain $vk$ (R2, W2, T2, S2, V2) are inherently bound to the contract contract, eliminating the need for explicit per-circuit context binding.

### 4.3 Public Viewing Key

$$\text{PVK} = vk \cdot H$$

A Grumpkin point stored on-chain at registration. Serves as the recipient's ECDH public key for incoming transfers. The registration proof constrains $\text{PVK} = vk \cdot H$ where $vk = \text{Poseidon}(\delta_{\text{vk}}, sk, \text{wrap})$ and $Y = sk \cdot H$, preventing a user from registering an unrelated $\text{PVK}$.

### 4.4 Delegation Viewing Key

For operator $i$ with address $\text{op}_i$, the owner derives:

$$dvk_i = \text{Poseidon}(\delta_{\text{dvk}}, vk, \text{op}_i)$$

Properties:
- $dvk_i$ reveals only this operator's allowance state in this contract's context ($vk$ is contract-specific, Section 4.2).
- $dvk_i$ cannot recover $vk$ (preimage resistance).
- Different $(vk, \text{op}_i)$ tuples yield independent keys.

---

## 5. Commitment Scheme

The following symbols are used throughout this section:

| Symbol | Definition |
|:---|:---|
| $C_{\text{spend}}$ | On-chain spendable balance commitment (Pedersen point) |
| $C_{\text{receive}}$ | On-chain receiving balance commitment (Pedersen point) |
| $C_{\text{tx}}$ | Transfer commitment added to recipient's $C_{\text{receive}}$ |
| $\text{Com}(v, r)$ | Pedersen commitment $v \cdot G + r \cdot H$ |
| $v_s, r_s$ | Value and blinding factor of $C_{\text{spend}}$ (off-chain wallet state) |
| $v_r, r_r$ | Value and blinding factor of $C_{\text{receive}}$ (off-chain wallet state) |
| $v_{\text{tx}}$ | Transfer amount (private) |
| $r_{\text{tx}}$ | ECDH-derived blinding factor for $C_{\text{tx}}$ |
| $W_{\text{spend}}, W_{\text{receive}}$ | Wallet-side accumulators: $(v, r)$ pairs tracking commitment openings |
| $r_e$ | Ephemeral scalar sampled per transfer |
| $R_e$ | Ephemeral public key $r_e \cdot H$ (published in event data) |
| $S$ | ECDH shared secret point $r_e \cdot \text{PVK}_B$ |
| $s$ | Scalar extracted from shared secret: $S.x \in \mathbb{F}_r$ |
| $\tilde{v}$ | Encrypted transfer amount: $v_{\text{tx}} + \text{Poseidon}(\delta_{\text{tx\\\_amount}}, s, \sigma)$ |
| $\tilde{b}$ | Encrypted balance scalar: $v_{\text{new}} + \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$ |
| $\sigma$ | Prover-chosen random salt, sampled per operation via the rejection sampling procedure of §2.2; canonical $\mathbb{F}_r$ representative encoded as `BytesN<32>` |

### 5.1 Balance Commitments

Each balance is a single Pedersen commitment $C = \text{Com}(v, r) \in \mathbb{G}$, represented on-chain as an uncompressed affine point $(x, y) \in \mathbb{F}_r^2$ (64 bytes). The identity $\mathcal{O}$ is encoded as $(0, 0)$ and handled as a special case in point arithmetic.

The committed value $v$ can represent the full range of practical balances (up to $2^{127} - 1$, bounded by the SEP-41 `i128` interface) without discrete logarithm concerns, because the owner maintains the commitment opening off-chain (Section 5.2) and the auditor reads an encrypted scalar (Section 5.5).

### 5.2 Off-Chain Opening Maintenance {#opening-maintenance}

A Pedersen commitment $C = \text{Com}(v, r)$ hides its opening $(v, r)$. The owner must know this opening to construct spend proofs so they must maintain $(v, r)$ as local wallet state, updated incrementally as balance-modifying events occur.

**Definition** (Wallet state). The owner's wallet maintains two running accumulators:

$$W_{\text{spend}} = (v_s, r_s) \quad \text{such that} \quad C_{\text{spend}} = v_s \cdot G + r_s \cdot H$$
$$W_{\text{receive}} = (v_r, r_r) \quad \text{such that} \quad C_{\text{receive}} = v_r \cdot G + r_r \cdot H$$

**Initialization.** At registration, $C_{\text{spend}} = C_{\text{receive}} = \mathcal{O}$. The wallet sets $W_{\text{spend}} = W_{\text{receive}} = (0, 0)$.

**Update rules.** Each balance-modifying event updates exactly one accumulator:

| Event | Accumulator update |
|:---|:---|
| Deposit of public amount $a$ to this account | $W_{\text{receive}} \mathrel{+}= (a, 0)$ |
| Incoming transfer with event $(R_e, \tilde{v}, \sigma)$ | Compute $S = vk \cdot R_e$, $s = S.x$; derive $v_{\text{tx}} = \tilde{v} - \text{Poseidon}(\delta_{\text{tx\\\_amount}}, s, \sigma)$ and $r_{\text{tx}} = \text{Poseidon}(\delta_{\text{tx\\\_blind}}, s, \sigma)$. Then $W_{\text{receive}} \mathrel{+}= (v_{\text{tx}}, r_{\text{tx}})$ |
| Outgoing transfer/withdrawal of amount $a$ | Proof outputs new commitment with deterministic randomness. $W_{\text{spend}} \leftarrow (v_s - a, \; \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma))$ |
| Merge | $W_{\text{spend}} \leftarrow (v_s + v_r, \; r_s + r_r)$; $W_{\text{receive}} \leftarrow (0, 0)$ |
| Set operator (escrow amount $a$) | Proof outputs new commitment. $W_{\text{spend}} \leftarrow (v_s - a, \; \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma))$ |
| Revoke operator (reclaim amount $a$) | Proof outputs new commitment. $W_{\text{spend}} \leftarrow (v_s + a, \; \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma))$ |

The Merge row uses exact integer addition; $W_{\text{spend}}.r$ is not reduced modulo $r$ or $q$ as merges accumulate. At proof-construction time the wallet reduces $W_{\text{spend}}.r$ modulo $q$ and encodes the canonical $\mathbb{F}_q$ representative as a single $\mathbb{F}_r$ `Field`. This encoding succeeds when the representative lies in $[0, r)$, with probability $\geq 1 - 2^{-127}$ per merge; the complementary case is acknowledged in §10.4 *Post-merge witness availability*.

After every owner-initiated operation that produces a proof, $r_s$ resets to a deterministic value. This is the **normalization** property: the spendable balance's blinding factor is always recoverable from $(vk, \sigma)$ at spend boundaries. Together with $\tilde{b}$, both emitted in the spend-boundary event, each spend boundary forms a **checkpoint** from which the spendable opening $(v_s, r_s)$ is recoverable via a single event lookup, with no exhaustive history replay needed for $W_{\text{spend}}$. Recovering $W_{\text{receive}}$, and folding in any post-checkpoint merges, still requires replaying events emitted after the checkpoint (see Recovery below).

**Consistency check.** At any time, the wallet can verify its state: $C_{\text{spend}} \stackrel{?}{=} v_s \cdot G + r_s \cdot H$ and $C_{\text{receive}} \stackrel{?}{=} v_r \cdot G + r_r \cdot H$, where $C_{\text{spend}}$ and $C_{\text{receive}}$ are read from on-chain state.

**Recovery.** If the wallet loses local state, it recovers from the **last checkpoint**: the most recent owner-initiated proof operation (`withdraw`, `confidential_transfer`, `set_operator`, or `revoke_operator`), which emitted both $\tilde{b}$ and $\sigma$ in its event. By construction, only deposits, incoming transfers, and merges can occur after this event; any later owner-initiated proof operation would itself become the new checkpoint. Steps 1-4 recover the spendable balance using $\tilde{b}$, $\sigma$ (both from the event), and $vk$. Event replay (steps 5-6) folds in the bounded post-checkpoint activity:

1. Fetch $(\tilde{b}, \sigma)$ from the most recent **checkpoint event** for this account, where a checkpoint event is exactly one of `Withdraw`, `Transfer` (where the account is the `from`), `SetOperator`, or `RevokeOperator` -- the four event types that carry a proof-bound $(\tilde{b}, \sigma)$ for the account's spendable balance. `Deposit`, `Transfer` (where the account is the `to`), `OperatorTransfer` (recipient side), and `Merge` are explicitly **not** checkpoints: they either carry no $(\tilde{b}, \sigma)$ at all or carry one that is bound to a different account's spendable balance. **No-checkpoint case:** if the account has no checkpoint event since `Register`, initialize $W_{\text{spend}} \leftarrow (0, 0)$ and skip to step 5 with the replay window starting at the `Register` event.
2. Recover the spendable balance value: $v_s = \tilde{b} - \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$.
3. Recover the spendable balance blinding: $r_s = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma)$.
4. Set $W_{\text{spend}} \leftarrow (v_s, r_s)$ and $W_{\text{receive}} \leftarrow (0, 0)$.
5. Replay all events since the checkpoint in ledger order. For each event:
   - **Incoming transfer** $(R_e, \tilde{v}, \sigma_{\text{sender}})$: compute $S = vk \cdot R_e$, derive $v_{\text{tx}}$ and $r_{\text{tx}}$. Accumulate $W_{\text{receive}} \mathrel{+}= (v_{\text{tx}}, r_{\text{tx}})$.
   - **Deposit** of amount $a$: accumulate $W_{\text{receive}} \mathrel{+}= (a, 0)$.
   - **Merge**: fold $W_{\text{spend}} \leftarrow (W_{\text{spend}}.v + W_{\text{receive}}.v, \; W_{\text{spend}}.r + W_{\text{receive}}.r)$, reset $W_{\text{receive}} \leftarrow (0, 0)$.
6. Verify consistency: $C_{\text{spend}} \stackrel{?}{=} W_{\text{spend}}.v \cdot G + W_{\text{spend}}.r \cdot H$ and $C_{\text{receive}} \stackrel{?}{=} W_{\text{receive}}.v \cdot G + W_{\text{receive}}.r \cdot H$.

Steps 1-3 require $(\tilde{b}, \sigma)$ from the latest owner event and $vk$. No full event replay is needed. Step 5 replays only events since the last checkpoint and correctly handles any number of interleaved deposits, transfers, and merges. A wallet that spends regularly produces frequent checkpoints, bounding the replay window. In the worst case (funds received but never spent), the replay window extends back to registration.

**Event durability requirement.** Recovery depends on the wallet being able to retrieve every event since the last checkpoint, plus the checkpoint event itself, in ledger order. Stellar RPC retains event history for a 7-days window only, so a wallet that loses local state after that window cannot recover from RPC alone. The protocol therefore assumes a durable event archive that retains the full per-account history of `Withdraw`, `Transfer` (both directions), `OperatorTransfer` (recipient side), `Deposit`, `Merge`, `SetOperator`, and `RevokeOperator` events forever. The data model, ingestion contract, retention obligations, and recommended API surface for that archive are specified in [INDEXER.md](INDEXER.md). Wallets and SDKs MUST consume an indexer that meets that contract for recovery.

### 5.3 ECDH-Derived Blinding

When a sender (spending key $sk_A$) transfers to a recipient with public viewing key $\text{PVK}_B$, the transfer commitment uses blinding derived from an ephemeral ECDH exchange.

**Definition 1** (Transfer blinding derivation). The sender samples $r_e, \sigma \in \mathbb{F}_r$ via the rejection sampling procedure (§2.2), then computes:

$$R_e = r_e \cdot H$$
$$S = r_e \cdot \text{PVK}_B$$
$$s = S.x \in \mathbb{F}_r$$
$$r_{\text{tx}} = \text{Poseidon}(\delta_{\text{tx\\\_blind}}, s, \sigma)$$
$$\tilde{v} = v_{\text{tx}} + \text{Poseidon}(\delta_{\text{tx\\\_amount}}, s, \sigma)$$

where $v_{\text{tx}}$ is the transfer amount. The transfer commitment is $C_{\text{tx}} = \text{Com}(v_{\text{tx}}, r_{\text{tx}})$. The ephemeral public key $R_e$, encrypted amount $\tilde{v}$, and $\sigma$ are published in the transaction event data so recipients can derive both $v_{\text{tx}}$ and $r_{\text{tx}}$ during replay.

Since $vk_B \cdot R_e = r_e \cdot \text{PVK}_B = S$ by ECDH commutativity, both sender and recipient can independently derive $r_{\text{tx}}$ and decrypt $v_{\text{tx}} = \tilde{v} - \text{Poseidon}(\delta_{\text{tx\\\_amount}}, s, \sigma)$, provided they know $\sigma$ emitted with the event. The auditor decrypts the transfer amount via a separate ECDH channel (Section 8.1).

**Note.** Each transfer involves two auditor ECDH exchanges: one with the recipient's auditor key ($S_{a,r} = r_e \cdot K_{\text{aud,r}}$) and one with the sender's auditor key ($S_{a,s} = r_e \cdot K_{\text{aud,s}}$). Both reuse the ephemeral scalar $r_e$, as does the $dvk_i$ escrow ECDH in `set_operator` (§7.11) when one is present. Neither auditor recovers any account's viewing key.

**Why reusing $r_e$ is safe.** Each ECDH channel keyed from the same $r_e$ produces a distinct shared scalar because the counterparty public keys are distinct ($\text{PVK}_B$, $K_{\text{aud,r}}$, $K_{\text{aud,s}}$, $Y_{\text{op}}$ are independent Grumpkin points, none derivable from one another). Each channel further uses a distinct Poseidon domain tag ($\delta_{\text{tx\\\_blind}}/\delta_{\text{tx\\\_amount}}$ for the recipient channel, $\delta_{\text{aud\\\_r}}$ and $\delta_{\text{aud\\\_s}}$ for the two auditor channels, $\delta_{\text{esc\\\_dvk}}$ for the operator escrow), so masks across channels are independent under the PRF assumption on Poseidon (§3.2). The channel masks are used as one-time pads against fresh per-transfer randomness ($\sigma$ or $\sigma_a$), and each per-channel sponge re-absorbs that nonce, so a given mask is never reused even for the same counterparty across two operations. Together these three properties (distinct shared scalars, distinct domains, fresh per-operation nonce) close the standard ECDH key-reuse attack surface; the contract's enumeration of channels in §13 satisfies the domain-distinctness condition.

### 5.4 Anti-Poisoning Constraint

The transfer circuit enforces that $C_{\text{tx}}$ was constructed using the ECDH-derived $r_{\text{tx}}$:

$$C_{\text{tx}} = v_{\text{tx}} \cdot G + r_{\text{tx}} \cdot H \quad \text{where} \quad r_{\text{tx}} = \text{Poseidon}(\delta_{\text{tx\\\_blind}}, s, \sigma)$$

This prevents a malicious sender from committing with arbitrary blinding, which would cause the recipient to lose track of their accumulated blinding factor and be unable to spend.

### 5.5 Encrypted Balance Scalar

Owner-initiated operations (transfers, withdrawals) produce a new spendable balance commitment with deterministic randomness (Section 7). To enable wallet recovery without full event replay, the proof also outputs an **encrypted balance scalar**:

$$\tilde{b} = v_{\text{new}} + \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$$

where $v_{\text{new}}$ is the new spendable balance and $\sigma$ is the prover-chosen random salt. The contract emits $\tilde{b}$ in the operation's event (Section 11.2) rather than storing it on-chain; the contract never reads it after the proof has bound it to $C_{\text{spend}}$. Anyone with $vk$ recovers $v_{\text{new}} = \tilde{b} - \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$ from the event. The primary consumer is the owner's wallet for checkpoint recovery (Section 5.2); auditors do not hold $vk$ and instead read balances via per-transfer ECDH ciphertexts (Section 8.1). The circuit enforces consistency between $\tilde{b}$ and the committed value in $C_{\text{spend}}$.

---

## 6. Account State

### 6.1 Account Data Model

Each registered account stores a `ConfidentialAccount` in persistent storage, keyed by `Address`:

```rust
ConfidentialAccount {
    spending_key:         BytesN<64>,   // Y = sk · H
    viewing_public_key:   BytesN<64>,   // PVK = vk · H
    spendable_balance:    BytesN<64>,   // C_spend: single Pedersen commitment
    receiving_balance:    BytesN<64>,   // C_receive: single Pedersen commitment
    auditor_id:           u32,
}
```

**`spending_key`**

$Y = sk \cdot H$. Set once at registration. Authorizes all spending operations.

**`viewing_public_key`**

$\text{PVK} = vk \cdot H$. Set once at registration. Used by senders for ECDH key agreement. The registration proof enforces derivation from the same $sk$ as $Y$.

**`spendable_balance`**

The commitment the owner can spend from. Modified only by owner-authorized operations: transfers out, withdrawals, merge, `set_operator`, `revoke_operator`. Encoded as a single Grumpkin affine point (64 bytes).


**`receiving_balance`**

Accumulates incoming deposits and transfers via homomorphic addition. The contract adds to this without any proof from the recipient. Reset to $\mathcal{O}$ on merge. Encoded as a single Grumpkin affine point (64 bytes).

**`auditor_id`**

Index into the auditor contract's key store. Set once at registration. Used by the contract to fetch the correct auditor public key when building transfer public inputs. For incoming transfers, the recipient's `auditor_id` determines the key under which the transfer amount is encrypted. For outgoing transfers (and operator transfers), the sender's (or owner's) `auditor_id` determines the key under which the transfer amount and post-transfer balance (or allowance) are encrypted.

### 6.2 Operator Delegation

Operator delegations are stored in persistent storage, keyed by `(owner, operator)`:

```rust
OperatorDelegation {
    allowance_commitment: BytesN<64>,   // Single Pedersen commitment
    encrypted_allowance:  BytesN<32>,   // Poseidon-encrypted allowance scalar
    escrowed_dvk:         BytesN<64>,   // ECDH escrow of dvk_i under operator key
    allowance_salt:       BytesN<32>,
    live_until_ledger:    u32,
}
```

**`allowance_commitment`**

The operator's remaining escrowed allowance, a single Pedersen commitment: $C_a = \text{Com}(v_a, r_a)$ where $r_a = \text{Poseidon}(\delta_{\text{allow\\\_r}}, dvk_i, \sigma_a)$. One Grumpkin point (64 bytes).

**`encrypted_allowance`**

Poseidon-encrypted allowance scalar: $\tilde{a} = v_a + \text{Poseidon}(\delta_{\text{enc\\\_allow}}, dvk_i, \sigma_a)$. Enables the operator (who holds $dvk_i$ via `escrowed_dvk`) to read the current allowance without DLP when constructing an `OperatorTransfer` witness. The owner can also read it via $vk \rightarrow dvk_i$. The auditor does not consume this field; allowance visibility for the auditor is provided by the per-event ciphertexts (Section 8.5).

**`escrowed_dvk`**

$dvk_i$ encrypted under the operator's spending key via ECDH. (64 bytes)

**`allowance_salt`**

Per-delegation salt for allowance randomness derivation, encoded as `BytesN<32>` (canonical $\mathbb{F}_r$ representative). $\sigma_a$ is sampled by the rejection sampling procedure of §2.2 (same as $\sigma$) and is the sole freshness input to all allowance Poseidon derivations. Set by the owner at `set_operator` and replaced by the operator on every `confidential_transfer_from` (the operator samples a fresh `new_allowance_salt` and that becomes the stored value alongside the updated `allowance_commitment`). The salt is bound to the current commitment: when the commitment changes, the salt changes with it. It is stored on-chain so the owner can decrypt the allowance at revocation without depending on event history.

**Dual role.** In operator transfers, $\sigma_a$ also serves as the nonce for the recipient ECDH encryption (O7, O9) and the auditor channel sponges (O\_a2 and O\_a6, which absorb $\sigma_a$ alongside the channel shared scalar). This is safe because ECDH confidentiality derives from the shared secret $S.x$ (or $S_{a,r}.x$, $S_{a,s}.x$), not from $\sigma_a$ being secret. However, this couples the allowance salt to the transfer event: the event must emit $\sigma_a$ so that the recipient and auditor can decrypt. Any change to how the salt is stored or exposed must preserve this invariant.

**`live_until_ledger`**

The ledger number at which the delegation expires. The delegation is live while `ledger.sequence() <= live_until_ledger` and expired once `ledger.sequence() > live_until_ledger`. Checked on every `confidential_transfer_from`. The delegation persists in storage until explicitly revoked (if it were in temporary storage automatic cleanup would destroy escrowed funds).

**Single-slot semantics.** The `(owner, operator)` slot holds at most one delegation. `set_operator` (Section 7.7) reverts if a delegation already exists for that pair, regardless of whether the existing delegation is past `live_until_ledger`. Expiry only prevents the operator from spending; the escrowed value persists on-chain until `revoke_operator` (Section 7.9) folds it back into the owner's spendable balance. Re-delegating to the same operator therefore requires the sequence: `revoke_operator` then `set_operator`. This rule is what keeps the balance-conservation invariant (Section 9.3) ranging cleanly over stored delegations: every delegation is either active, expired-pending-revoke, or absent, and the escrowed value is never silently dropped.

---

## 7. Operations

### 7.1 Public Input Sources

UltraHonk verifies the relation between a proof and its public-input vector. The verifier sees only field elements -- it has no knowledge of which account, contract, or auditor those values are supposed to describe. Binding each public input to the correct provenance is the contract's responsibility. If the contract takes a value that should come from trusted state (e.g. the sender's `spending_key`) and instead reads it from caller-controlled invocation inputs, a soundly proven statement can verify for the wrong account.

Each operation below lists, for every public input, where the contract loads it from -- persistent account storage, the delegation entry, the contract's own contract address, an auditor-contract lookup, an invocation argument, or a prover-supplied value that the circuit binds.

**Trust-boundary rule.** Public inputs that derive from trusted state (account storage, delegation storage, the current contract address, or auditor-contract lookups) MUST be loaded by the contract itself. The contract MUST NOT accept these values from the caller's `data` payload. Only invocation arguments (which are bound under `require_auth()` per §11.1) and prover-supplied values (which the circuit binds to its constraints) may originate from the caller. Violating this rule breaks soundness even with a perfectly sound circuit.

### 7.2 Registration

An account provides a Grumpkin spending key $Y$, a public viewing key $\text{PVK}$, and a chosen `auditor_id`, accompanied by a proof of key well-formedness.

**Circuit constraints (Register):**

| # | Constraint |
|:--|:---|
| R1 | $Y = sk \cdot H$ (spending key well-formed) |
| R2 | $vk = \text{Poseidon}(\delta_{\text{vk}}, sk, \text{wrap})$ (viewing key correctly derived, binds proof to contract) |
| R3 | $\text{PVK} = vk \cdot H$ (public viewing key matches $vk$) |
| R4 | $sk \neq 0$ (rules out $Y = \mathcal{O}$) |
| R5 | $vk \neq 0$ (rules out $\text{PVK} = \mathcal{O}$, which would collapse every incoming-transfer ECDH) |

**Public inputs:**

| Input | Notes |
|:---|:---|
| $Y$, $\text{PVK}$ | Prover-supplied; written to `account.spending_key` and `account.viewing_public_key` on success |
| $\text{wrap}$ | Loaded from instance storage; set once at construction (§3.5) |

**Private witnesses:** $sk$.

**Post-verification state:** The contract validates that `auditor_id` exists in the auditor contract and points to a valid key, then stores `spending_key`, `viewing_public_key`, `auditor_id`, and initializes `spendable_balance = receiving_balance = ` $\mathcal{O}$.

### 7.3 Deposit

Transparent tokens flow from the depositor to the contract via `token.transfer(from, self, amount)`. The amount $a$ is public and typed as `i128`. The contract checks $a \ge 0$ at the entrypoint and reverts on violation (Section 3.4). The contract then computes the deposit commitment with zero blinding:

$$C_{\text{dep}} = a \cdot G + 0 \cdot H = a \cdot G$$

and adds it to the recipient's receiving balance:

$$C_{\text{receive}} \leftarrow C_{\text{receive}} + C_{\text{dep}}$$

No proof required. The recipient `to` **must** be registered: the receiving-balance update writes into `to`'s `ConfidentialAccount` slot and the contract reverts if no slot exists. The depositor `from` does **not** need a registered confidential account; only the SEP-41 `token.transfer(from, self, a)` authorization is required. The recipient's off-chain state updates: $v_{\text{receive}} \mathrel{+}= a$, $r_{\text{receive}} \mathrel{+}= 0$.

### 7.4 Merge

The owner folds the receiving balance into the spendable balance.

**Contract logic (no proof):**

```
require account.require_auth()
C_spend ← C_spend + C_receive
C_receive ← O
```

**Proposition 1** (Merge correctness). <em>If $C_{\text{spend}} = \text{Com}(v_s, r_s)$ and $C_{\text{receive}} = \text{Com}(v_r, r_r)$ before merge, then after merge $C_{\text{spend}} = \text{Com}(v_s + v_r, r_s + r_r)$ and $C_{\text{receive}} = \mathcal{O} = \text{Com}(0, 0)$.</em>

*Proof.* By the homomorphic property of Pedersen commitments:
$$C_{\text{spend}} + C_{\text{receive}} = (v_s \cdot G + r_s \cdot H) + (v_r \cdot G + r_r \cdot H) = (v_s + v_r) \cdot G + (r_s + r_r) \cdot H = \text{Com}(v_s + v_r, r_s + r_r)$$
No value is created or destroyed. $\square$

**Owner state update.** The owner knows the opening of the post-merge commitment: $v_{\text{spend}}' = v_s + v_r$, $r_{\text{spend}}' = r_s + r_r$. The owner knows $v_r$ and $r_r$ from processing incoming transfer and deposit events into $W_{\text{receive}}$ (Section 5.2, *Update rules*; the per-transfer derivation is Definition 1 in Section 5.3). The values $v_s$ and $r_s$ are known from the owner's last proof output.

**Griefing analysis.** Merge requires `account.require_auth()`. No third party can invoke it. Incoming transfers that arrive between proof construction and submission modify only $C_{\text{receive}}$, which is not referenced by spend proofs. Therefore merge is not front-runnable and incoming transfers cannot invalidate spend proofs (Proposition 2, Section 9.1).

**Encrypted balance.** Merge emits no $\tilde{b}$ (there is no proof to enforce consistency between $\tilde{b}$ and the post-merge $C_{\text{spend}}$). The next owner-initiated proof operation issues a fresh checkpoint. The auditor tracks incoming amounts independently from transfer events.

### 7.5 Withdrawal

The owner withdraws a public amount $a$ (typed `i128`) from their spendable balance. The W4 range constraint bounds $a$ at $2^{127}$ in-circuit; the contract additionally checks $a \ge 0$ at the entrypoint (Section 3.4).

**Circuit constraints (Withdraw):**

| # | Constraint |
|:--|:---|
| W1 | $Y = sk \cdot H$ (owner key ownership) |
| W2 | $vk = \text{Poseidon}(\delta_{\text{vk}}, sk, \text{wrap})$ (binds proof to contract) |
| W3 | The prover knows the opening $(v, r)$ of $C_{\text{spend}}$: $C_{\text{spend}} = v \cdot G + r \cdot H$ |
| W4 | $v \in [0, 2^{127})$, $a \in [0, 2^{127})$, $v - a \in [0, 2^{127})$ (range validity, Section 2.6) |
| W5 | $r' = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma)$ (deterministic randomness for new balance) |
| W6 | $C_{\text{spend}}' = (v - a) \cdot G + r' \cdot H$ (new spendable commitment) |
| W7 | $\tilde{b} = (v - a) + \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$ (encrypted balance scalar) |
| W8 | $r_e \neq 0$ (rules out $R_e = \mathcal{O}$ and $S_{a,s} = \mathcal{O}$, which would reduce $m_b$ to a constant function of $\sigma$) |
| W\_a1 | $R_e = r_e \cdot H$ (ephemeral key for auditor ECDH) |
| W\_a2 | $S_{a,s} = r_e \cdot K_{\text{aud,s}}$ (sender-auditor ECDH shared secret) |
| W\_a3 | $m_b = \text{SpongeSqueeze}_1(\delta_{\text{aud\\\_s}}, S_{a,s}.x, \sigma)$ (sender-auditor channel sponge, single squeeze) |
| W\_a4 | $\tilde{b}_{\text{aud,s}} = (v - a) + m_b$ (sender-auditor encrypted balance checkpoint) |

**Public inputs (15 fields):**

| Input | Notes |
|:---|:---|
| $C_{\text{spend}}$ | Loaded from `from.spendable_balance` |
| $Y$ | Loaded from `from.spending_key` |
| $\text{wrap}$ | Loaded from instance storage; set once at construction (§3.5) |
| $K_{\text{aud,s}}$ | Fetched from the auditor contract using `from.auditor_id` |
| $a$ | Public withdrawal amount from invocation inputs |
| $C_{\text{spend}}'$, $\sigma$, $\tilde{b}$, $R_e$, $\tilde{b}_{\text{aud,s}}$ | Prover-supplied; $C_{\text{spend}}'$ written to `from.spendable_balance`, the rest emitted in event |

$\text{to}$ is bound under `from.require_auth()` and does not appear in the proof.

**Private witnesses:** $sk$, $vk$, $v$, $r$, $r_e$.

**Post-verification:** The contract verifies the proof, sets `from`.`spendable_balance` $= C_{\text{spend}}'$, and calls `token.transfer(self, to, a)`. Emits event with $(R_e, \sigma, \tilde{b}, \tilde{b}_{\text{aud,s}})$.

### 7.6 Confidential Transfer

The sender (account $A$, spending key $sk_A$) transfers a hidden amount $v_{\text{tx}}$ to recipient $B$ (public viewing key $\text{PVK}_B$).

**Sender computation:**

1. Sample ephemeral scalar $r_e \in \mathbb{F}_r$ via the rejection sampling procedure (§2.2); sample $\sigma \in \mathbb{F}_r$ via the same procedure
2. Compute $R_e = r_e \cdot H$
3. Compute $S = r_e \cdot \text{PVK}_B$, extract $s = S.x$
4. Derive transfer blinding: $r_{\text{tx}} = \text{Poseidon}(\delta_{\text{tx\\\_blind}}, s, \sigma)$
5. Derive encrypted amount: $\tilde{v} = v_{\text{tx}} + \text{Poseidon}(\delta_{\text{tx\\\_amount}}, s, \sigma)$
6. Compute transfer commitment: $C_{\text{tx}} = v_{\text{tx}} \cdot G + r_{\text{tx}} \cdot H$
7. Compute new spendable commitment with deterministic randomness:
   - $r_A' = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk_A, \sigma)$
   - $C_{\text{spend}}' = (v_A - v_{\text{tx}}) \cdot G + r_A' \cdot H$
8. Compute encrypted balance scalar: $\tilde{b} = (v_A - v_{\text{tx}}) + \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk_A, \sigma)$
9. Compute recipient-auditor ECDH shared secret: $S_{a,r} = r_e \cdot K_{\text{aud,r}}$, extract $s_{a,r} = S_{a,r}.x$
10. Squeeze recipient-auditor channel masks: $(m_{v,r}, m_{r,r}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_r}}, s_{a,r}, \sigma)$
11. Compute recipient-auditor ciphertexts: $\tilde{v}_{\text{aud,r}} = v_{\text{tx}} + m_{v,r}$ and $\tilde{r}_{\text{aud,r}} = r_{\text{tx}} + m_{r,r}$
12. Compute sender-auditor ECDH shared secret: $S_{a,s} = r_e \cdot K_{\text{aud,s}}$, extract $s_{a,s} = S_{a,s}.x$
13. Squeeze sender-auditor channel masks: $(m_{v,s}, m_{b,s}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_s}}, s_{a,s}, \sigma)$
14. Compute sender-auditor ciphertexts: $\tilde{v}_{\text{aud,s}} = v_{\text{tx}} + m_{v,s}$ and $\tilde{b}_{\text{aud,s}} = (v_A - v_{\text{tx}}) + m_{b,s}$

**Circuit constraints (Transfer):**

| # | Constraint |
|:--|:---|
| T1 | $Y_A = sk_A \cdot H$ (sender key ownership) |
| T2 | $vk_A = \text{Poseidon}(\delta_{\text{vk}}, sk_A, \text{wrap})$ (binds proof to contract) |
| T3 | Prover knows opening $(v_A, r_A)$ of $C_{\text{spend}}^A$ |
| T4 | $v_A \in [0, 2^{127})$, $v_{\text{tx}} \in [0, 2^{127})$, $v_A - v_{\text{tx}} \in [0, 2^{127})$ (range validity, Section 2.6) |
| T5 | $S = r_e \cdot \text{PVK}_B$ (ECDH correctly computed) |
| T6 | $R_e = r_e \cdot H$ (ephemeral key well-formed) |
| T7 | $r_{\text{tx}} = \text{Poseidon}(\delta_{\text{tx\\\_blind}}, S.x, \sigma)$ (blinding correctly derived) |
| T8 | $C_{\text{tx}} = v_{\text{tx}} \cdot G + r_{\text{tx}} \cdot H$ (transfer commitment well-formed) |
| T9 | $\tilde{v} = v_{\text{tx}} + \text{Poseidon}(\delta_{\text{tx\\\_amount}}, S.x, \sigma)$ (encrypted amount correct) |
| T10 | $r_A' = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk_A, \sigma)$ (deterministic randomness) |
| T11 | $C_{\text{spend}}' = (v_A - v_{\text{tx}}) \cdot G + r_A' \cdot H$ (new sender balance) |
| T12 | $\tilde{b} = (v_A - v_{\text{tx}}) + \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk_A, \sigma)$ (encrypted balance scalar) |
| T13 | $r_e \neq 0$ (rules out $R_e = \mathcal{O}$ and $S, S_{a,r}, S_{a,s} = \mathcal{O}$; otherwise every ECDH mask in this transfer collapses to a constant function of $\sigma$) |
| T\_a1 | $S_{a,r} = r_e \cdot K_{\text{aud,r}}$ (recipient-auditor ECDH shared secret, reuses ephemeral scalar) |
| T\_a2 | $(m_{v,r}, m_{r,r}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_r}}, S_{a,r}.x, \sigma)$ (recipient-auditor channel masks) |
| T\_a3 | $\tilde{v}_{\text{aud,r}} = v_{\text{tx}} + m_{v,r}$ (recipient-auditor encrypted transfer amount) |
| T\_a4 | $\tilde{r}_{\text{aud,r}} = r_{\text{tx}} + m_{r,r}$ (recipient-auditor encrypted transfer randomness, enables Pedersen-opening reconstruction of $C_{\text{receive}}$, see Section 8.1) |
| T\_a5 | $S_{a,s} = r_e \cdot K_{\text{aud,s}}$ (sender-auditor ECDH shared secret, reuses ephemeral scalar) |
| T\_a6 | $(m_{v,s}, m_{b,s}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_s}}, S_{a,s}.x, \sigma)$ (sender-auditor channel masks) |
| T\_a7 | $\tilde{v}_{\text{aud,s}} = v_{\text{tx}} + m_{v,s}$ (sender-auditor encrypted transfer amount) |
| T\_a8 | $\tilde{b}_{\text{aud,s}} = (v_A - v_{\text{tx}}) + m_{b,s}$ (sender-auditor encrypted balance checkpoint) |

**Public inputs (24 fields, counting each Grumpkin point as two $\mathbb{F}_r$ coordinates):**

| Input | Notes |
|:---|:---|
| $C_{\text{spend}}^A$ | Loaded from sender's `spendable_balance` |
| $Y_A$ | Loaded from sender's `spending_key` |
| $\text{PVK}_B$ | Loaded from recipient's `viewing_public_key`. Recipient must be registered. |
| $\text{wrap}$ | Loaded from instance storage; set once at construction (§3.5) |
| $K_{\text{aud,r}}$ | Fetched from the auditor contract using recipient's `auditor_id` |
| $K_{\text{aud,s}}$ | Fetched from the auditor contract using sender's `auditor_id` |
| $C_{\text{spend}}'$, $C_{\text{tx}}$, $R_e$, $\tilde{v}$, $\tilde{b}$, $\sigma$, $\tilde{v}_{\text{aud,r}}$, $\tilde{r}_{\text{aud,r}}$, $\tilde{v}_{\text{aud,s}}$, $\tilde{b}_{\text{aud,s}}$ | Prover-supplied; $C_{\text{spend}}'$ written to sender's `spendable_balance`, $C_{\text{tx}}$ added to recipient's `receiving_balance`, the rest emitted in event |

**Private witnesses:** $sk_A$, $vk_A$, $v_A$, $r_A$, $v_{\text{tx}}$, $r_e$.

**Post-verification:** The contract verifies the proof, then:
- Sets $A$`.spendable_balance` $= C_{\text{spend}}'$
- Adds to recipient: $B$`.receiving_balance` $\mathrel{+}= C_{\text{tx}}$
- Emits event with $(R_e, \tilde{v}, \sigma, \tilde{b}, \tilde{v}_{\text{aud,r}}, \tilde{r}_{\text{aud,r}}, \tilde{v}_{\text{aud,s}}, \tilde{b}_{\text{aud,s}})$

**Recipient processing.** Upon observing the event, the recipient computes $S = vk \cdot R_e$, derives amount and blinding. The decryption flow is independent of whether the sender was the owner or an operator.

### 7.7 Set Operator

The owner locks funds from their spendable balance into a per-operator escrow. The operator must be a registered account in the contract, so that $Y_{\text{op}}$ (needed for $dvk_i$ escrow) can be looked up from the operator's stored `spending_key`.

**Circuit constraints (SetOperator):**

| # | Constraint |
|:--|:---|
| S1 | $Y = sk \cdot H$ (owner key ownership) |
| S2 | $vk = \text{Poseidon}(\delta_{\text{vk}}, sk, \text{wrap})$ (binds proof to contract) |
| S3 | Prover knows opening $(v, r)$ of $C_{\text{spend}}$ |
| S4 | $v \in [0, 2^{127})$, $v_a \in [0, 2^{127})$, $v - v_a \in [0, 2^{127})$ (range validity, Section 2.6) |
| S5 | $dvk_i = \text{Poseidon}(\delta_{\text{dvk}}, vk, \text{op}_i)$ (delegation key derivation; contract-bound via $vk$) |
| S6 | $r_a = \text{Poseidon}(\delta_{\text{allow\\\_r}}, dvk_i, \sigma_a)$ (allowance blinding) |
| S7 | $C_a = v_a \cdot G + r_a \cdot H$ (allowance commitment) |
| S8 | $\tilde{a} = v_a + \text{Poseidon}(\delta_{\text{enc\\\_allow}}, dvk_i, \sigma_a)$ (encrypted allowance) |
| S9 | $r' = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma)$ (new balance randomness) |
| S10 | $C_{\text{spend}}' = (v - v_a) \cdot G + r' \cdot H$ (new spendable balance) |
| S11 | $\tilde{b} = (v - v_a) + \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$ (encrypted balance) |
| S12 | Escrowed $dvk_i$ correctly encrypts under $Y_{\text{op}}$ via ECDH |
| S13 | $r_e \neq 0$ (rules out $R_e = \mathcal{O}$ and $S_{a,s} = \mathcal{O}$; the same $r_e$ is reused for the $dvk_i$ escrow ECDH in Section 7.11, so this also rules out a trivial escrow shared secret) |
| S\_a1 | $R_e = r_e \cdot H$ (ephemeral key for auditor ECDH) |
| S\_a2 | $S_{a,s} = r_e \cdot K_{\text{aud,s}}$ (owner-auditor ECDH shared secret) |
| S\_a3 | $(m_v, m_b) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_s}}, S_{a,s}.x, \sigma)$ (owner-auditor channel masks) |
| S\_a4 | $\tilde{v}_{\text{aud,s}} = v_a + m_v$ (owner-auditor encrypted escrow amount) |
| S\_a5 | $\tilde{b}_{\text{aud,s}} = (v - v_a) + m_b$ (owner-auditor encrypted balance checkpoint) |

**Public inputs (24 fields):**

| Input | Notes |
|:---|:---|
| $C_{\text{spend}}$ | Loaded from owner's `spendable_balance` |
| $Y$ | Loaded from owner's `spending_key` |
| $Y_{\text{op}}$ | Loaded from operator account's `spending_key`. Operator must be registered. |
| $\text{op}_i$ | $\text{address\\\_to\\\_field}$(`operator` argument), computed per-call by the contract (§2.7) |
| $\text{wrap}$ | Loaded from instance storage; set once at construction (§3.5) |
| $K_{\text{aud,s}}$ | Fetched from the auditor contract using owner's `auditor_id` |
| $C_{\text{spend}}'$, $C_a$, escrowed\_dvk, $\tilde{b}$, $\tilde{a}$, $\sigma$, $\sigma_a$, $R_e$, $\tilde{v}_{\text{aud,s}}$, $\tilde{b}_{\text{aud,s}}$ | Prover-supplied; $C_{\text{spend}}'$ written to owner's `spendable_balance`, the delegation fields written to storage, the rest emitted in event |

**Private witnesses:** $sk$, $vk$, $v$, $r$, $v_a$, $r_e$.

**Post-verification:** The contract verifies the proof, sets `spendable_balance` $= C_{\text{spend}}'$ and stores the `OperatorDelegation`. Emits event with $(R_e, \sigma, \tilde{b}, \tilde{v}_{\text{aud,s}}, \tilde{b}_{\text{aud,s}})$.

### 7.8 Operator Transfer

The operator transfers from the owner's escrowed allowance to a recipient.

**Circuit constraints (OperatorTransfer):**

| # | Constraint |
|:--|:---|
| O1 | $Y_{\text{op}} = sk_{\text{op}} \cdot H$ (operator key ownership) |
| O2 | Prover knows $dvk_i$ and the opening $(v_a, r_a)$ of $C_a$ |
| O3 | $r_a = \text{Poseidon}(\delta_{\text{allow\\\_r}}, dvk_i, \sigma_a)$ (allowance randomness matches stored state) |
| O4 | $v_a \in [0, 2^{127})$, $v_{\text{tx}} \in [0, 2^{127})$, $v_a - v_{\text{tx}} \in [0, 2^{127})$ (range validity, Section 2.6) |
| O5 | $S = r_e \cdot \text{PVK}_{\text{recipient}}$ (ECDH for recipient) |
| O6 | $R_e = r_e \cdot H$ |
| O7 | $r_{\text{tx}} = \text{Poseidon}(\delta_{\text{tx\\\_blind}}, S.x, \sigma_a)$ (transfer blinding) |
| O8 | $C_{\text{tx}} = v_{\text{tx}} \cdot G + r_{\text{tx}} \cdot H$ |
| O9 | $\tilde{v} = v_{\text{tx}} + \text{Poseidon}(\delta_{\text{tx\\\_amount}}, S.x, \sigma_a)$ (encrypted amount) |
| O10 | $r_a' = \text{Poseidon}(\delta_{\text{allow\\\_r}}, dvk_i, \sigma_a')$ (new allowance randomness) |
| O11 | $C_a' = (v_a - v_{\text{tx}}) \cdot G + r_a' \cdot H$ (new allowance) |
| O12 | $\tilde{a}' = (v_a - v_{\text{tx}}) + \text{Poseidon}(\delta_{\text{enc\\\_allow}}, dvk_i, \sigma_a')$ (encrypted allowance) |
| O13 | $r_e \neq 0$ (rules out $R_e = \mathcal{O}$ and $S, S_{a,r}, S_{a,s} = \mathcal{O}$; otherwise every ECDH mask in this transfer collapses to a constant function of $\sigma_a$) |
| O\_a1 | $S_{a,r} = r_e \cdot K_{\text{aud,r}}$ (recipient-auditor ECDH shared secret, reuses ephemeral scalar) |
| O\_a2 | $(m_{v,r}, m_{r,r}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_r}}, S_{a,r}.x, \sigma_a)$ (recipient-auditor channel masks) |
| O\_a3 | $\tilde{v}_{\text{aud,r}} = v_{\text{tx}} + m_{v,r}$ (recipient-auditor encrypted transfer amount) |
| O\_a4 | $\tilde{r}_{\text{aud,r}} = r_{\text{tx}} + m_{r,r}$ (recipient-auditor encrypted transfer randomness, enables Pedersen-opening reconstruction of $C_{\text{receive}}$, see Section 8.1) |
| O\_a5 | $S_{a,s} = r_e \cdot K_{\text{aud,s}}$ (owner-auditor ECDH shared secret, reuses ephemeral scalar) |
| O\_a6 | $(m_{v,s}, m_{a,s}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_s}}, S_{a,s}.x, \sigma_a)$ (owner-auditor channel masks) |
| O\_a7 | $\tilde{v}_{\text{aud,s}} = v_{\text{tx}} + m_{v,s}$ (owner-auditor encrypted transfer amount) |
| O\_a8 | $\tilde{a}_{\text{aud,s}} = (v_a - v_{\text{tx}}) + m_{a,s}$ (owner-auditor encrypted post-transfer allowance) |

**Public inputs (24 fields):**

| Input | Notes |
|:---|:---|
| $C_a$, $\sigma_a$ | Loaded from the `(from, operator)` delegation entry |
| $Y_{\text{op}}$ | Loaded from operator's `spending_key`; matches the auth principal |
| $\text{PVK}_{\text{recipient}}$ | Loaded from recipient's `viewing_public_key` |
| $K_{\text{aud,r}}$ | Fetched from the auditor contract using recipient's `auditor_id` |
| $K_{\text{aud,s}}$ | Fetched from the auditor contract using **owner's** `auditor_id`, not operator's. The visibility model points balance- and allowance-checkpoint ciphertexts at the funds' owner. |
| $C_a'$, $C_{\text{tx}}$, $R_e$, $\tilde{v}$, $\tilde{a}'$, $\sigma_a'$, $\tilde{v}_{\text{aud,r}}$, $\tilde{r}_{\text{aud,r}}$, $\tilde{v}_{\text{aud,s}}$, $\tilde{a}_{\text{aud,s}}$ | Prover-supplied; allowance fields written to delegation storage, $C_{\text{tx}}$ added to recipient's `receiving_balance`, the rest emitted in event |

**Private witnesses:** $sk_{\text{op}}$, $dvk_i$, $v_a$, $r_a$ (single-limb $\mathbb{F}_r$; pinned by O3 to $\text{Poseidon}(\delta_{\text{allow\\\_r}}, dvk_i, \sigma_a)$), $v_{\text{tx}}$, $r_e$.

**Post-verification:** The contract checks `ledger.sequence() <= live_until_ledger`, updates `allowance_commitment`, `encrypted_allowance`, stores `new_allowance_salt`, and adds $C_{\text{tx}}$ to the recipient's `receiving_balance`. Emits event with $(R_e, \tilde{v}, \sigma_a, \tilde{v}_{\text{aud,r}}, \tilde{r}_{\text{aud,r}}, \tilde{v}_{\text{aud,s}}, \tilde{a}_{\text{aud,s}})$.

**Recipient uniformity.** The recipient processes the incoming transfer identically to a direct transfer: compute $S = vk \cdot R_e$, derive amount and blinding. The decryption flow is independent of whether the sender was the owner or an operator.

**Contract binding.** Unlike owner-initiated circuits, the OperatorTransfer circuit does not constrain the $vk$ derivation (the operator has no access to the owner's $sk$). Contract binding is instead inherited indirectly through the allowance commitment chain: the SetOperator circuit derives $dvk_i$ from the contract-specific $vk$ (S2, S5), which determines $r_a$ (S6) and thus $C_a$ (S7). The OperatorTransfer circuit verifies $dvk_i$ against $C_a$ via $\sigma_a$ (O3). Since $C_a$ is a public input and was constructed with contract-specific randomness, a proof generated against one contract's $C_a$ cannot verify against another's.

### 7.9 Revoke Operator

The owner reclaims the remaining escrowed allowance.

**Circuit constraints (RevokeOperator):**

| # | Constraint |
|:--|:---|
| V1 | $Y = sk \cdot H$ (owner key ownership) |
| V2 | $vk = \text{Poseidon}(\delta_{\text{vk}}, sk, \text{wrap})$ (binds proof to contract) |
| V3 | $dvk_i = \text{Poseidon}(\delta_{\text{dvk}}, vk, \text{op}_i)$ |
| V4 | Prover knows opening $(v_a, r_a)$ of $C_a$, with $r_a = \text{Poseidon}(\delta_{\text{allow\\\_r}}, dvk_i, \sigma_a)$ (allowance randomness matches stored state, mirrors O3) |
| V5 | Prover knows opening $(v_s, r_s)$ of $C_{\text{spend}}$ |
| V6 | $r' = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma)$ |
| V7 | $C_{\text{spend}}' = (v_s + v_a) \cdot G + r' \cdot H$ |
| V8 | $\tilde{b} = (v_s + v_a) + \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$ |
| V9 | $v_s \in [0, 2^{127})$, $v_a \in [0, 2^{127})$, $v_s + v_a \in [0, 2^{127})$ (range validity, Section 2.6) |
| V10 | $r_e \neq 0$ (rules out $R_e = \mathcal{O}$ and $S_{a,s} = \mathcal{O}$, which would reduce $m_v$ and $m_b$ to constant functions of $\sigma$) |
| V\_a1 | $R_e = r_e \cdot H$ (ephemeral key for auditor ECDH) |
| V\_a2 | $S_{a,s} = r_e \cdot K_{\text{aud,s}}$ (owner-auditor ECDH shared secret) |
| V\_a3 | $(m_v, m_b) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_s}}, S_{a,s}.x, \sigma)$ (owner-auditor channel masks) |
| V\_a4 | $\tilde{v}_{\text{aud,s}} = v_a + m_v$ (owner-auditor encrypted reclaimed amount) |
| V\_a5 | $\tilde{b}_{\text{aud,s}} = (v_s + v_a) + m_b$ (owner-auditor encrypted balance checkpoint) |

**Public inputs (19 fields):**

| Input | Notes |
|:---|:---|
| $C_{\text{spend}}$ | Loaded from owner's `spendable_balance` |
| $C_a$, $\sigma_a$ | Loaded from the `(account, operator)` delegation entry |
| $Y$ | Loaded from owner's `spending_key` |
| $\text{op}_i$ | $\text{address\\\_to\\\_field}$(`operator` argument), computed per-call by the contract (§2.7) |
| $\text{wrap}$ | Loaded from instance storage; set once at construction (§3.5) |
| $K_{\text{aud,s}}$ | Fetched from the auditor contract using owner's `auditor_id` |
| $C_{\text{spend}}'$, $\tilde{b}$, $\sigma$, $R_e$, $\tilde{v}_{\text{aud,s}}$, $\tilde{b}_{\text{aud,s}}$ | Prover-supplied; $C_{\text{spend}}'$ written to owner's `spendable_balance`, delegation entry deleted, the rest emitted in event |

**Private witnesses:** $sk$, $vk$, $dvk_i$, $v_a$, $r_a$, $v_s$, $r_s$ (input spendable-balance blinding, encoded as a single $\mathbb{F}_r$ `Field`; see §10.4 *Post-merge witness availability* for the acknowledged $2^{-127}$-per-merge case affecting $r_s$), $r_e$.

**Post-verification:** The contract verifies the proof, sets `spendable_balance` $= C_{\text{spend}}'$ and deletes the delegation. Emits event with $(R_e, \sigma, \tilde{b}, \tilde{v}_{\text{aud,s}}, \tilde{b}_{\text{aud,s}})$.

### 7.10 Owner Operations with Active Operators

Owner transfers, withdrawals, and merges proceed identically to the no-operator case. Operator allowances are independently escrowed - no synchronization is needed. The owner's spendable balance and operator allowances are fully isolated.

### 7.11 Delegation Key Escrow

At `set_operator`, the owner escrows $dvk_i$ to the operator on-chain via ECDH, eliminating off-chain key sharing:

1. Owner picks ephemeral $r_e$ (reused from the `set_operator` proof's outer ECDH; see §5.3 <em>Why reusing $r_e$ is safe</em>) and computes $R = r_e \cdot H$.
2. Shared secret: $s = (r_e \cdot Y_{\text{op}}).x$
3. Escrowed key: $\text{escrowed\\\_dvk} = (R.x, \; \text{Poseidon}(\delta_{\text{esc\\\_dvk}}, s, \text{op}_i) + dvk_i)$

**Encoding.** `escrowed_dvk` is a `BytesN<64>` consisting of two 32-byte $\mathbb{F}_r$ representatives: `R_x` (the $x$-coordinate of $R$) followed by `dvk_cipher` (the masked $dvk_i$). $R.y$ is **not** stored. This is sound because ECDH on Grumpkin recovers only the $x$-coordinate of the shared secret: $\pm R$ both have $x = R.x$, and $sk_{\text{op}} \cdot R$ and $sk_{\text{op}} \cdot (-R)$ are inverse points with the same $x$-coordinate. The operator reconstructs the curve point by solving $y^2 = R.x^3 - 17$ in $\mathbb{F}_r$, picks either root, and proceeds; both choices produce the same $s = (sk_{\text{op}} \cdot R).x$ and therefore the same Poseidon mask.

The operator decrypts using $sk_{\text{op}}$. The `set_operator` proof enforces escrow correctness via constraint S12, which expands to three sub-constraints over the prover-supplied `escrowed_dvk = (R_x, dvk_cipher)`:

- $R_x = (r_e \cdot H).x$
- $s_{\text{esc}} = (r_e \cdot Y_{\text{op}}).x$
- $\text{dvk\\\_cipher} = \text{Poseidon}(\delta_{\text{esc\\\_dvk}}, s_{\text{esc}}, \text{op}_i) + dvk_i$

The $r_e$ here is the same scalar S\_a1 commits to ($R_e = r_e \cdot H$), so the escrow's $R_x$ and the auditor channel's $R_e.x$ are forced equal.

### 7.12 Expiry and Revert Safety

Delegations use persistent storage and persist until explicitly revoked. `live_until_ledger` is checked on every operator transfer. Allowance randomness includes `allowance_salt` to prevent deterministic-randomness reuse after reverted transactions.

---

## 8. Auditing

### 8.1 Per-Transfer Auditor Ciphertexts

Each confidential transfer produces ciphertexts under two auditor keys via ECDH, using the same ephemeral scalar $r_e$ used for recipient ECDH. Each auditor channel runs Poseidon2 in sponge mode (Section 2.5), absorbing the channel's domain tag, the ECDH shared scalar, and $\sigma$, and squeezing two masks per call.

**Recipient's auditor** ($K_{\text{aud,r}}$, from the recipient's `auditor_id`) receives the transfer amount and the per-transfer Pedersen randomness:

$$S_{a,r} = r_e \cdot K_{\text{aud,r}}, \qquad s_{a,r} = S_{a,r}.x$$
$$(m_{v,r}, m_{r,r}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_r}}, s_{a,r}, \sigma)$$
$$\tilde{v}_{\text{aud,r}} = v_{\text{tx}} + m_{v,r}, \qquad \tilde{r}_{\text{aud,r}} = r_{\text{tx}} + m_{r,r}$$

**Sender's auditor** ($K_{\text{aud,s}}$, from the sender's `auditor_id`) receives the transfer amount and the sender's post-transfer balance:

$$S_{a,s} = r_e \cdot K_{\text{aud,s}}, \qquad s_{a,s} = S_{a,s}.x$$
$$(m_{v,s}, m_{b,s}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_s}}, s_{a,s}, \sigma)$$
$$\tilde{v}_{\text{aud,s}} = v_{\text{tx}} + m_{v,s}, \qquad \tilde{b}_{\text{aud,s}} = (v_A - v_{\text{tx}}) + m_{b,s}$$

The transfer circuit (constraints T\_a1--T\_a8) enforces correct computation. The contract fetches both auditor keys from the auditor contract using the respective account `auditor_id` fields; neither the sender nor the recipient can substitute a different key.

Each auditor decrypts using their secret key $k$. For example, the sender's auditor:

$$S_{a,s} = k \cdot R_e, \qquad s_{a,s} = S_{a,s}.x$$
$$(m_{v,s}, m_{b,s}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_s}}, s_{a,s}, \sigma)$$
$$v_{\text{tx}} = \tilde{v}_{\text{aud,s}} - m_{v,s}, \qquad v_{\text{new}} = \tilde{b}_{\text{aud,s}} - m_{b,s}$$

where $R_e$ and $\sigma$ are published in the Transfer event. The recipient's auditor follows the same pattern with $\delta_{\text{aud\\\_r}}$ to recover the pair $(v_{\text{tx}}, r_{\text{tx}})$.

**Recipient-auditor opening capability.** Because the recipient-auditor recovers $r_{\text{tx}}$ for every inbound transfer, and because deposits add to `receiving_balance` with $r = 0$ (Section 7.3), the recipient-auditor can reconstruct the full Pedersen opening of $C_{\text{receive}}$ between merges:

$$v_r = \sum_i v_{\text{tx},i} + \sum_j a_j, \qquad r_r = \sum_i r_{\text{tx},i}$$

where $i$ ranges over inbound transfers and operator-transfers since the last merge and $j$ ranges over deposits. This is a full Pedersen *opening* of $C_{\text{receive}}$: both the value and the blinding are reconstructed by the auditor.

The opening capability does not extend to $C_{\text{spend}}$. The auditor knows the *value* $v_s$ at every spend boundary via $\tilde{b}_{\text{aud,s}}$ (Section 5.5), and can extend that with the known $v_r$ contribution at each merge. This bounded opening is what enables the clawback flow specified in [COMPLIANCE.md](./COMPLIANCE.md) §5: the recipient-auditor is the seize-enabling party for inbound flows while $C_{\text{receive}}$ has not yet been merged. After merge the auditor still tracks $C_{\text{spend}}$'s value via $\tilde{b}_{\text{aud,s}}$ updates.

### 8.2 Auditor Visibility Properties

**Transfer amounts.** Both auditors see the transfer amount in real time. The recipient's auditor decrypts $v_{\text{tx}}$ from $\tilde{v}_{\text{aud,r}}$; the sender's auditor decrypts it from $\tilde{v}_{\text{aud,s}}$.

**Balance checkpoints.** The sender's auditor receives an encrypted balance checkpoint at every owner-initiated operation that produces a proof:

- **Outgoing transfer**: auditor decrypts post-transfer balance $(v_A - v_{\text{tx}})$ from $\tilde{b}_{\text{aud,s}}$ (constraints T\_a5--T\_a8).
- **Withdrawal**: auditor decrypts post-withdrawal balance $(v - a)$ from $\tilde{b}_{\text{aud,s}}$ (constraints W\_a1--W\_a4). The withdrawal amount $a$ is also visible as a public input.
- **Set operator**: auditor decrypts escrowed amount $v_a$ from $\tilde{v}_{\text{aud,s}}$ and post-escrow balance $(v - v_a)$ from $\tilde{b}_{\text{aud,s}}$ (constraints S\_a1--S\_a5).
- **Revoke operator**: auditor decrypts reclaimed amount $v_a$ from $\tilde{v}_{\text{aud,s}}$ and post-reclaim balance $(v_s + v_a)$ from $\tilde{b}_{\text{aud,s}}$ (constraints V\_a1--V\_a5).

The recipient's auditor does not see the sender's balance in any of these operations.

**Per-transfer Pedersen randomness (recipient-auditor only).** Beyond the transfer amount, the recipient's auditor also decrypts the per-transfer Pedersen blinding $r_{\text{tx}}$ from $\tilde{r}_{\text{aud,r}}$ on every confidential transfer and operator-transfer (constraints T\_a4 and O\_a4). Combined with $v_{\text{tx}}$ this is a full Pedersen opening of each $C_{\text{tx},i}$ and, by homomorphism, of the recipient's `receiving_balance` $C_{\text{receive}}$ between merges (Section 8.1). The sender's auditor does not see $r_{\text{tx}}$.

**Key rotation.** When the auditor contract sets a new key under the account's `auditor_id` (§8.3), the new key sees the balance checkpoint at the next owner-initiated operation, with no event replay or bootstrapping required. The balance checkpoint is self-contained: it depends only on the auditor's ECDH secret key and the published $(R_e, \sigma)$. Note that `auditor_id` itself is immutable per account (§6.1); only the key under that `auditor_id` rotates.

**No viewing-key escrow on the sender side.** The sender-auditor does not hold any account's viewing key, and compromise of a sender-auditor key exposes only per-operation amounts and balance checkpoints from operations that occurred while the compromised key was active. Historical balances under prior keys, and the recipient's `spendable_balance` (whose blinding derives from $vk_A$, not from any auditor channel), remain opaque.

**Recipient-side opening capability.** The recipient-auditor additionally learns the per-transfer Pedersen randomness $r_{\text{tx}}$ for every inbound transfer and operator-transfer. This is capability-equivalent to holding the opening of every $C_{\text{tx},i}$ and, by summation, of $C_{\text{receive}}$. The capability is bounded in two ways:

- **Forward-only.** Only events emitted while the auditor key was active are decryptable.
- **Receiving-side only (opening).** The full $(v_r, r_r)$ opening covers `receiving_balance`. It does not extend to a full opening of `spendable_balance`, whose blinding $r_s = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk_A, \sigma)$ depends on $vk_A$ and is not derivable from any auditor key. The auditor still knows the *value* $v_s$ of `spendable_balance` at every spend boundary via $\tilde{b}_{\text{aud,s}}$.

This is the trust position that supports the clawback flow in [COMPLIANCE.md](./COMPLIANCE.md) §5: the recipient-auditor is the seize-enabling party for inbound flows, while the sender-auditor remains the seize-enabling party for the spendable-balance side via $\tilde{b}_{\text{aud,s}}$.

### 8.3 Auditor Key Management and Rotation

The auditor contract stores Grumpkin public keys as full affine points $(x, y)$ indexed by `auditor_id`. The contract validates that every inserted key is canonical, on-curve ($y^2 \equiv x^3 - 17 \pmod{r}$), and non-identity at insertion time (Section 3.1, Section 10.8). Each `auditor_id` MAY maintain a sequence of versions, each carrying its activation ledger. Rotation appends a new entry rather than overwriting the previous one.

When building public inputs for any operation that produces auditor ciphertexts (transfers, withdrawals, set/revoke operator), the contract fetches the relevant auditor keys for the recipient's and/or sender's `auditor_id`. The contract passes the full Grumpkin point as a public input; the circuit constrains the ECDH ciphertexts against that exact point. The contract and the circuit are version-agnostic: they verify against whichever key the auditor contract currently exposes.

**In-flight proofs across rotation.** A proof constructed against version $v$ becomes unverifiable the instant the auditor contract activates version $v+1$. The $K_{\text{aud}}$ public input the contract fetches at verification no longer matches the value the prover committed to, so UltraHonk verification fails and the invocation **reverts at the proof-verification boundary**. The caller (sender, owner, or operator) reconstructs the proof against the new $K_{\text{aud}}$ and resubmits. The rejection is benign: the contract's spendable balance, receiving balance, and delegation state are unchanged by the reverted call, $\sigma$ is freshly sampled on retry (Section 9.6), and an observer cannot correlate the rejected attempt with the resubmission.

**Auditor's off-chain obligation.** The auditor MUST retain the secret key for every historical version it has issued. To decrypt an event at ledger $L$, the auditor queries the auditor contract for the version of its `auditor_id` whose activation ledger is the largest value not exceeding $L$, then uses the corresponding off-chain secret key against the $R_e$ and $\sigma$ (or $\sigma_a$) emitted in the event.

### 8.4 Operator Transfer Auditing

Each operator transfer produces auditor ciphertexts under two keys (constraints O\_a1--O\_a8), following the same dual-auditor sponge model as owner transfers. The recipient's auditor decrypts the transfer amount and the per-transfer Pedersen randomness:

$$(m_{v,r}, m_{r,r}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_r}}, s_{a,r}, \sigma_a)$$
$$v_{\text{tx}} = \tilde{v}_{\text{aud,r}} - m_{v,r}, \qquad r_{\text{tx}} = \tilde{r}_{\text{aud,r}} - m_{r,r}$$

The owner's auditor decrypts the transfer amount and post-transfer allowance:

$$(m_{v,s}, m_{a,s}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_s}}, s_{a,s}, \sigma_a)$$
$$v_{\text{tx}} = \tilde{v}_{\text{aud,s}} - m_{v,s}, \qquad v_a' = \tilde{a}_{\text{aud,s}} - m_{a,s}$$

where $s_{a,r}$, $s_{a,s}$, and $\sigma_a$ are recovered from the event as in Section 8.1. The recipient-auditor opening capability stated in Section 8.1 extends to operator-transfer inbound flows: $r_{\text{tx}}$ from operator-transfers contributes to $r_r$ in $C_{\text{receive}}$ identically to owner-transfer inbound flows.

### 8.5 Operator Allowance Auditing

The auditor tracks each allowance's current value through the per-event ciphertexts produced at every state-changing operation: `set_operator` reveals the escrowed amount $v_a$ (Section 8.2), `confidential_transfer_from` reveals the transfer amount and post-transfer allowance $v_a'$ (Section 8.4), and `revoke_operator` reveals the reclaimed amount (Section 8.2).

**Key rotation.** Visibility is forward-only at the event level, matching the spendable-balance model (§8.2). A new key under the account's existing `auditor_id` sees an allowance at the next state-changing operation, when a fresh ciphertext is produced under the new key.

---

## 9. Security Analysis

### 9.1 Griefing Resistance

**Proposition 2** (Spend-proof stability). *No third party can invalidate an honest owner's in-flight spend proof.*

*Proof.* A spend proof (Section 7.6) references $C_{\text{spend}}^A$ as a public input. The contract modifies $C_{\text{spend}}^A$ only through:
1. Owner-initiated operations (transfer, withdrawal, `set_operator`, `revoke_operator`) - all require `account.require_auth()`.
2. Merge - requires `account.require_auth()`.

Incoming transfers modify only $C_{\text{receive}}^A$, which does not appear in the spend proof's public inputs. Therefore, between proof construction and submission, no third-party action can alter $C_{\text{spend}}^A$. The proof remains valid. $\square$

**Corollary.** There is no counter cap on incoming transfers. An account can receive an unbounded number of transfers without any mandatory owner action. The receiving balance is a single point whose committed value grows monotonically; there is no chunk overflow because Pedersen commitments operate over the full scalar field ($|\mathbb{F}_q| \approx 2^{254}$).

### 9.2 Merge Safety

**Proposition 3** (Merge cannot be weaponized). *A third party cannot invoke merge on another account.*

*Proof.* The `merge()` function requires `account.require_auth()`. Only the account holder can authorize it. $\square$

**Proposition 4** (Merge does not create or destroy value). *Follows directly from Proposition 1 and the homomorphic property of Pedersen commitments.*

### 9.3 Balance Conservation

**Invariant.** For any account at any time:

$$\sum_{j} d_j - \sum_{k} w_k = v_{\text{spend}} + v_{\text{receive}} + \sum_{i} v_{\text{allowance}_i}$$

where $d_j$ are deposits, $w_k$ are withdrawals, and the right-hand side sums committed values across the spendable balance, the receiving balance, and every stored (not-yet-revoked) operator allowance. Expired-but-not-revoked allowances are included: expiration prevents the operator from spending the allowance, but the escrowed value still resides on-chain in $C_a$ until `revoke_operator` reclaims it (Section 6.2, Single-slot semantics).

This invariant is maintained by:
- **Deposits** increase $v_{\text{receive}}$ by $d_j$ (Section 7.3).
- **Withdrawals** decrease $v_{\text{spend}}$ by $w_k$, enforced by circuit constraint W4.
- **Transfers** decrease sender's $v_{\text{spend}}$ and increase recipient's $v_{\text{receive}}$ by the same $v_{\text{tx}}$, enforced by circuit constraints T3–T8.
- **Merge** moves value from $v_{\text{receive}}$ to $v_{\text{spend}}$ (Proposition 1); the sum is unchanged.
- **Set operator** moves value from $v_{\text{spend}}$ to $v_{\text{allowance}_i}$; enforced by S3–S7.
- **Operator transfer** decreases $v_{\text{allowance}_i}$ and increases recipient's $v_{\text{receive}}$ by $v_{\text{tx}}$; enforced by O2–O8.
- **Revoke** moves remaining $v_{\text{allowance}_i}$ back to $v_{\text{spend}}$; enforced by V4–V7.

### 9.4 Privacy Properties

**Amount confidentiality.** Transfer amounts are hidden inside Pedersen commitments (computationally hiding under DL). The encrypted amount $\tilde{v}$ is masked by $\text{Poseidon}(\delta_{\text{tx\\\_amount}}, s, \sigma)$, which is pseudorandom to anyone who does not know $s$ (the ECDH shared secret).

**Balance confidentiality.** The spendable balance commitment hides both value and blinding. The encrypted balance scalar $\tilde{b}$ emitted in spend-boundary events is masked by $\text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$, pseudorandom without $vk$.

**Sender-recipient linkage.** Sender and recipient addresses are visible on-chain. The system provides amount and balance confidentiality, not anonymity.

**Viewing key compromise.** Since $vk$ is contract-specific (Section 4.2), compromise of one contract's viewing key does not affect the owner's accounts in other deployments. Within the compromised contract, the attacker can: read all spendable balance snapshots (via $\tilde{b}$ emitted in spend-boundary events), decrypt all incoming transfer amounts (via ECDH with $R_e$ from events), and derive all $dvk_i$ to read operator allowances. The attacker **cannot** authorize any spending operation (requires $sk$, and $vk$ cannot recover $sk$ by Poseidon preimage resistance).

**Auditor key compromise.** If a sender's auditor key is compromised, the attacker can decrypt amounts and balance checkpoints ($\tilde{b}_{\text{aud,s}}$) for all operations (transfers, withdrawals, set/revoke operator) from accounts that used the compromised key, but cannot construct openings of any commitment. If a recipient's auditor key is compromised, the attacker recovers both the transfer amount and the per-transfer Pedersen randomness ($\tilde{v}_{\text{aud,r}}$, $\tilde{r}_{\text{aud,r}}$) for every incoming transfer to accounts that used the compromised key. This is capability-equivalent to holding the opening of every $C_{\text{tx},i}$ and, by summation, of the receiving-balance commitment $C_{\text{receive}}$; see Section 8.2 for the bounded scope (forward-only, receiving-side only). Merge folds $r_r$ into the spendable-balance randomness ($r_{\text{spend}}' = r_s + r_r$, Section 7.4) and emits no checkpoint, so the recipient-auditor's $r_r$ knowledge does not extend to a post-merge opening of $C_{\text{spend}}$: $r_s$ depends on $vk_A$ and is not derivable from any auditor key. In neither case can the attacker recover viewing keys, post-merge spendable-balance openings, historical data from before the key was active, or authorize any spending. After key rotation, new operations are protected by the new key.

### 9.5 State Recovery

The recovery model is built around **checkpoints**: each owner-initiated operation that produces a proof emits $(\tilde{b}, \sigma)$ in its event, creating a point from which the full spendable balance opening is recoverable using $\tilde{b}$, $\sigma$, and $vk$. Event replay is bounded to the window between the most recent checkpoint and the current ledger.

A checkpoint is concretely an event of type `Withdraw`, `Transfer` (as sender), `SetOperator`, or `RevokeOperator`: exactly the events that carry $(\tilde{b}, \sigma)$ for the owner's spendable balance. `Deposit`, incoming `Transfer`, and `OperatorTransfer` do not touch the owner's spendable balance and are therefore not checkpoints. `Merge` does update the spendable balance ($C_{\text{spend}} \leftarrow C_{\text{spend}} + C_{\text{receive}}$, Section 7.4) but is not a checkpoint either: it carries no proof and emits no $(\tilde{b}, \sigma)$, so consistency between $\tilde{b}$ and the post-merge commitment cannot be enforced. Any merge activity is absorbed into the next owner-initiated proof operation, which issues a fresh checkpoint.

**Checkpoint recovery (one event lookup).** At every spend boundary, the spendable balance has deterministic randomness: $r = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma)$. The owner fetches $(\tilde{b}, \sigma)$ from the most recent checkpoint event for their account (`Withdraw`, sender-side `Transfer`, `SetOperator`, or `RevokeOperator`), then recovers $v = \tilde{b} - \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$. Consistency is verifiable: $C_{\text{spend}} \stackrel{?}{=} v \cdot G + r \cdot H$.

**Post-checkpoint recovery (bounded event replay).** Between a checkpoint and the next spend, the receiving balance may have accumulated incoming transfers and deposits, and a merge may have folded them into the spendable balance. The owner reconstructs the current state by replaying only events since the last checkpoint:

1. Start from the checkpoint: set $W_{\text{spend}} \leftarrow (v_n, r_n)$ recovered from $(\tilde{b}, \sigma)$ in the latest checkpoint event and the deterministic blinding derivation. Set $W_{\text{receive}} \leftarrow (0, 0)$.
2. Replay all events since the checkpoint in ledger order. For each: incoming transfers and deposits accumulate into $W_{\text{receive}}$; merge events fold $W_{\text{receive}}$ into $W_{\text{spend}}$ and reset $W_{\text{receive}} \leftarrow (0, 0)$. This correctly handles any number of interleaved events.
3. Verify: $C_{\text{spend}} \stackrel{?}{=} W_{\text{spend}}.v \cdot G + W_{\text{spend}}.r \cdot H$ and $C_{\text{receive}} \stackrel{?}{=} W_{\text{receive}}.v \cdot G + W_{\text{receive}}.r \cdot H$.

The replay window is bounded by the owner's spending frequency. An account that spends or withdraws regularly produces frequent checkpoints, keeping the replay window short. In the worst case (funds received but never spent), the window extends back to registration.

**Data-availability dependency.** Recovery from seed alone (i.e., after the wallet's local cache is destroyed) requires access to the full event history since the last checkpoint, which Stellar RPC does not guarantee. The protocol therefore requires a durable indexer; the data model, retention obligations, and recommended API are specified in [INDEXER.md](INDEXER.md). Without such an indexer a user can still see that their funds exist on-chain (the commitment remains), but cannot reconstruct the opening required to spend.

**Incoming-transfer spam.** A third party can spam an account with confidential transfers (including zero-value transfers, see Section 9.1 Corollary) without invalidating the recipient's spend proofs. The cost to the spammer is the Soroban transaction fee per transfer, which bounds the rate. The cost to the recipient is per-event indexer storage and wallet replay work. Both costs are linear in the number of incoming transfers and bounded by the replay window; neither breaks correctness.

### 9.6 Revert Safety

Because $\sigma$ is sampled fresh via CSPRNG for every operation, a retry after a reverted transaction naturally uses a different $\sigma$. This means the deterministic randomness $r = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma)$ is always fresh, and an observer cannot correlate reverted and retried commitments.

**Retry procedure.** On revert, the wallet simply picks a new random $\sigma$ and recomputes the proof. No special-case logic is needed. The $\sigma$ is a public input and emitted in events so the auditor and owner can reconstruct randomness.

### 9.7 Replay Protection

**Proposition 5** (Proof non-replayability). *A valid proof cannot be replayed to execute the same operation twice.*

*Proof.* Every spending proof includes the current on-chain commitment ($C_{\text{spend}}$ or $C_a$) as a public input. Upon successful verification, the contract replaces this commitment with the proof's output commitment ($C_{\text{spend}}'$ or $C_a'$). A replayed proof references the old commitment, which no longer matches the stored state, so verification fails. The same argument applies to operator transfers via $C_a$. $\square$

**Corollary.** No explicit nullifier or nonce is needed. State binding through commitment chaining provides replay protection as an inherent property of the protocol.

---

## 10. Proof System

### 10.1 Circuits

All proof logic is written in Noir, compiled to UltraHonk circuits. The system uses 6 circuits:

```rust
#[contracttype]
#[repr(u32)]
pub enum CircuitType {
    Register = 0,
    Withdraw = 1,
    Transfer = 2,
    OperatorTransfer = 3,
    SetOperator = 4,
    RevokeOperator = 5,
}
```

### 10.2 Circuit Summary

| Circuit | What it proves |
|:---|:---|
| `Register` | Spending key well-formedness; contract-bound viewing key derivation from $sk$; public viewing key consistency with the derived $vk$ |
| `Withdraw` | Balance sufficiency; new spendable commitment with deterministic randomness; encrypted balance scalar; sender-auditor ECDH ciphertext (balance checkpoint); owner key ownership |
| `Transfer` | Balance conservation; ECDH-derived blinding and encrypted amount for recipient; dual-auditor channel sponges (recipient auditor: amount + per-transfer Pedersen randomness; sender auditor: amount + balance); deterministic randomness for new sender balance; encrypted balance scalar; sender key ownership; range validity (balance $\in [0, 2^{127})$, amount $\in [0, 2^{127})$) |
| `OperatorTransfer` | Allowance sufficiency; ECDH-derived blinding and encrypted amount for recipient; dual-auditor channel sponges (recipient auditor: amount + per-transfer Pedersen randomness; owner auditor: amount + allowance); deterministic randomness for new allowance; encrypted allowance scalar; operator key ownership; contract-bound indirectly via $C_a$ chain (Section 7.8) |
| `SetOperator` | Balance split; $dvk_i$ derivation; ECDH escrow of $dvk_i$; allowance commitment with deterministic randomness; encrypted balance and allowance scalars; owner-auditor ECDH ciphertexts (escrow amount + balance checkpoint); owner key ownership; contract-bound via $vk$ derivation |
| `RevokeOperator` | Allowance decryption via $dvk_i$; balance merge; deterministic randomness for new balance; encrypted balance scalar; owner-auditor ECDH ciphertexts (reclaimed amount + balance checkpoint); owner key ownership; contract-bound via $vk$ derivation |

### 10.3 Circuit Cost Analysis

The dominant cost in Noir circuits is elliptic curve scalar multiplication. With Barretenberg's native Grumpkin support via `multi_scalar_mul`, each scalar multiplication costs approximately 64 UltraPlonk-equivalent constraints (with ECC VM) or 4,700–6,250 without.

The Transfer circuit requires approximately 7 scalar multiplications: spending key verification, spendable balance opening, recipient ECDH shared secret, ephemeral key derivation, transfer commitment construction, recipient-auditor ECDH shared secret, and sender-auditor ECDH shared secret. The OperatorTransfer circuit requires approximately 7 scalar multiplications: operator key verification, allowance commitment opening, recipient ECDH shared secret, ephemeral key derivation, transfer commitment construction, recipient-auditor ECDH shared secret, and owner-auditor ECDH shared secret. The Withdraw, SetOperator, and RevokeOperator circuits each require 2 additional scalar multiplications for auditor ECDH (ephemeral key derivation + auditor shared secret), bringing their totals to approximately 4, 6, and 5 respectively. The ECDH computations add scalar multiplications compared to a random-blinding scheme, but the unchunked design eliminates all per-chunk constraints (which, in a chunked scheme, would involve 8+ scalar multiplications for balance chunks and per-chunk range proofs).

### 10.4 Noir Primitives

```rust
use std::embedded_curve_ops::{EmbeddedCurvePoint, EmbeddedCurveScalar, multi_scalar_mul};

/// Barretenberg's Pedersen generator at index 0 of the "DEFAULT_DOMAIN_SEPARATOR"
/// domain. Equivalent to `derive_generators("DEFAULT_DOMAIN_SEPARATOR", 0)[0]`.
global G: EmbeddedCurvePoint = EmbeddedCurvePoint {
    x: 0x083e7911d835097629f0067531fc15cafd79a89beecb39903f69572c636f4a5a,
    y: 0x1a7f5efaad7f315c25a918f30cc8d7333fccab7ad7c90f14de81bcc528f9935d,
    is_infinite: false,
};

/// Barretenberg's Pedersen generator at index 1 of the "DEFAULT_DOMAIN_SEPARATOR"
/// domain. Equivalent to `derive_generators("DEFAULT_DOMAIN_SEPARATOR", 0)[1]`.
/// No known discrete-log relation to G (each is the output of Barretenberg's
/// hash-to-curve on the domain separator + index).
global H: EmbeddedCurvePoint = EmbeddedCurvePoint {
    x: 0x054aa86a73cb8a34525e5bbed6e43ba1198e860f5f3950268f71df4591bde402,
    y: 0x209dcfbf2cfb57f9f6046f44d71ac6faf87254afc7407c04eb621a6287cac126,
    is_infinite: false,
};

/// Pedersen commitment, used uniformly for every opening witnessed in any
/// circuit (input or output). Both scalars are encoded as single-limb F_r
/// `Field` values: Poseidon outputs or rejection-sampled CSPRNG draws for fresh
/// blindings, and (for the spend-side input opening of C_spend in W3/T3/S3/V5)
/// the canonical F_q reduction of the wallet's post-merge integer blinding,
/// which lies in F_r with probability >= 1 - 2^-127 per merge. The complementary
/// case is acknowledged below in *Post-merge witness availability*.
fn commit(value: Field, randomness: Field) -> EmbeddedCurvePoint {
    multi_scalar_mul(
        [G, H],
        [EmbeddedCurveScalar::from_field(value),
         EmbeddedCurveScalar::from_field(randomness)]
    )
}

/// ECDH: scalar * point. All ECDH scalars in this protocol are F_r-sampled
/// (sk, vk, r_e), so single-limb conversion is sound.
fn ecdh(scalar: Field, point: EmbeddedCurvePoint) -> EmbeddedCurvePoint {
    multi_scalar_mul(
        [point],
        [EmbeddedCurveScalar::from_field(scalar)]
    )
}
```

**Post-merge witness availability.** Every opening witnessed in any circuit is encoded as a single $\mathbb{F}_r$ `Field` via the same `commit` primitive. After a `Merge` (§7.4), the spendable-balance blinding is $r_s + r_r$ over $\mathbb{F}_q$. Its canonical $\mathbb{F}_q$ representative lies in $[0, r)$ -- representable as a Noir `Field` -- with probability $\geq 1 - (q - r)/q \approx 1 - 2^{-127}$ per merge (§2.3). With the complementary probability $\approx 2^{-127}$ it lies in $[r, q)$. In that case the on-chain state remains well-formed (the commitment is a valid Grumpkin point), but the wallet's local opening witness is unencodable as a `Field`, so no spend / transfer / set-operator / revoke-operator proof can be constructed against the affected $C_{\text{spend}}$ until further accumulation shifts the blinding back into $\mathbb{F}_r$.

**Soft recovery.** Every subsequent inbound confidential transfer or operator transfer, once merged, adds a fresh $\mathbb{F}_r$-derived blinding. For each transfer-derived addend, the new canonical $\mathbb{F}_q$ representative falls in $[0, r)$ with probability $\geq 1 - 2^{-127}$ regardless of the current stuck value (worst case: the current value sits at the lower edge of $[r, q)$, requiring the new $\mathbb{F}_r$ addend to cross the mod-$q$ boundary; the probability of failing to do so is bounded by $(q - r)/r \approx 2^{-127}$). For accounts that continue to receive confidential transfers, the unspendable window is self-resolving at the next merge with overwhelming probability; accounts whose only inflows are deposits remain stuck until a confidential transfer arrives.

### 10.5 Verification Flow

1. Contract reads on-chain state (commitments, public keys)
2. Encodes state as public inputs: Grumpkin point coordinates as 32-byte $\mathbb{F}_r$ values
3. Cross-contract call: `verifier.verify_proof(circuit_type, public_inputs, proof)`
4. Verifier deserializes stored VK, runs UltraHonk verification (BN254 G1/G2 pairings, Fiat-Shamir, sumcheck)
5. Contract applies homomorphic balance updates (Grumpkin point arithmetic via $\mathbb{F}_r$ ops)

### 10.6 Structured Reference String {#srs}

UltraHonk is a PLONK-family proving system. Its knowledge soundness guarantee depends on a **Structured Reference String (SRS)** -- a sequence of BN254 G1 and G2 points derived from a secret scalar $\tau$ (the "toxic waste"):

$$\text{SRS} = \bigl([1]_1, [\tau]_1, [\tau^2]_1, \ldots, [\tau^{N-1}]_1, \; [1]_2, [\tau]_2\bigr)$$

where $[x]_1 = x \cdot G_1$ and $[x]_2 = x \cdot G_2$ are BN254 group elements. The SRS is **universal**: a single SRS supports any circuit up to size $N$, and circuit-specific verification keys are derived from it deterministically. The SRS is used during both proof generation (client-side) and verification key derivation (one-time setup).

**Security requirement.** If $\tau$ is known to an attacker, they can forge proofs for arbitrary false statements: minting tokens, draining accounts, bypassing all circuit constraints. The knowledge soundness of the entire system reduces to the assumption that $\tau$ was destroyed after SRS generation.

**Multi-party ceremony.** The standard mitigation is a multi-party computation (MPC) ceremony in which $N$ participants each contribute randomness. The resulting SRS is secure if *at least one* participant honestly destroyed their contribution.

**SRS used in this system.** The Noir/Barretenberg toolchain uses the **Aztec Ignition SRS** by default. Barretenberg downloads the required SRS points from a public transcript on first use. The Ignition ceremony transcript, participant attestations, and verification code are publicly available. The SRS supports circuits up to $2^{28}$ gates, well above the expected circuit sizes for this system ($< 2^{20}$).

**Deployment considerations.** The verifier contract does not store or reference the full SRS. Circuit-specific verification keys are derived offline from the SRS during circuit compilation and embedded in the verifier contract at deployment. The correctness of these VKs can be independently verified by anyone with access to the circuit source code and the public SRS transcript.

**Risk assessment.** The Ignition ceremony had 176 independent participants across multiple jurisdictions, hardware platforms, and operating systems. Compromise requires collusion of *all* 176 participants.

### 10.7 Dependency: CAP-80

[CAP-80](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0080.md) introduces host functions required for efficient UltraHonk verification and on-chain Grumpkin point arithmetic:

- `bn254_g1_msm`: Batched scalar-point multiplication on BN254 G1.
- `bn254_fr_{add, sub, mul, inv, pow}`: $\mathbb{F}_r$ scalar arithmetic.

### 10.8 On-Chain Point Arithmetic

The contract performs Grumpkin affine point addition and subtraction for homomorphic balance updates. Since Grumpkin coordinates are $\mathbb{F}_r^{\text{BN254}}$ elements, these reduce to Fr field operations.

**Curve coefficients.** Grumpkin $y^2 = x^3 - 17$ (Section 2.2) is in short Weierstrass form $y^2 = x^3 + a x + b$ with $a = 0$ and $b = -17$. Only $a$ enters the point arithmetic slope formulas below; $b$ enters only the on-curve check.

The contract distinguishes the following cases when computing $P_3 = P_1 + P_2$:

| Case | Condition | Result |
|:--|:--|:--|
| Left identity | $P_1 = \mathcal{O}$ | $P_3 = P_2$ |
| Right identity | $P_2 = \mathcal{O}$ | $P_3 = P_1$ |
| Inverse | $P_1, P_2 \neq \mathcal{O}$, $x_1 = x_2$, $y_1 = -y_2 \bmod r$ | $P_3 = \mathcal{O}$ |
| Doubling | $P_1, P_2 \neq \mathcal{O}$, $P_1 = P_2$ (so $y_1 \neq 0$) | slope formula with $\lambda_{\text{dbl}}$ below |
| Generic | $P_1, P_2 \neq \mathcal{O}$, $x_1 \neq x_2$ | slope formula with $\lambda_{\text{add}}$ below |

The inverse case must be detected and short-circuited before the generic slope formula, because $x_1 - x_2 = 0$ would otherwise force a division by zero in $\mathbb{F}_r$.

**Slope.**

$$\lambda_{\text{add}} = (y_2 - y_1)(x_2 - x_1)^{-1} \pmod{r}$$

$$\lambda_{\text{dbl}} = (3 x_1^2 + a)(2 y_1)^{-1} = 3 x_1^2 \cdot (2 y_1)^{-1} \pmod{r} \qquad (a = 0 \text{ for Grumpkin})$$

**Resulting coordinates.** With $\lambda$ selected per the case above:

$$x_3 = \lambda^2 - x_1 - x_2 \pmod{r}$$
$$y_3 = \lambda (x_1 - x_3) - y_1 \pmod{r}$$

Requires `bn254_fr_{add, sub, mul, inv}` host calls (CAP-80, Section 10.7).

**Point subtraction** $P_3 = P_1 - P_2$: if $P_2 = \mathcal{O}$ set $-P_2 = \mathcal{O}$, else $-P_2 = (x_2, -y_2 \bmod r)$; then apply the addition cases above. Subtraction of a point from itself yields $\mathcal{O}$ via the inverse case, never the doubling branch.

**Point validation.** Grumpkin points enter the system through three boundaries; on-curve and non-identity checks live at the boundary that owns each one. The contract itself performs no per-call on-curve check.

1. **Proof-constrained points (the dominant case).** Every public input that the corresponding circuit also derives via `multi_scalar_mul` is on-curve by construction -- Noir's embedded-curve operations cannot produce an off-curve Grumpkin point. This covers $Y$ (R1), $\text{PVK}$ (R3), $R_e$ (T6, O6, W_a1, S_a1, V_a1), $C_{\text{tx}}$ (T8, O8), $C_{\text{spend}}'$ (T11, W6, S10, V7), $C_a$ / $C_a'$ (S7, O11), and the ECDH shared secrets. Non-identity is enforced *in-circuit* by explicit nonzero-scalar constraints: $sk \neq 0$ and $vk \neq 0$ at registration (R4, R5), and $r_e \neq 0$ in every circuit that produces an ephemeral key (W8, T13, S13, O13, V10). Without these constraints an adversary could publish $Y = \mathcal{O}$, $\text{PVK} = \mathcal{O}$, or $R_e = \mathcal{O}$ and collapse ECDH (every shared secret becomes $\mathcal{O}$, every Poseidon mask becomes a constant function of $\sigma$, every ciphertext becomes trivially decryptable).
2. **Points read from prior on-chain state.** $C_{\text{spend}}$, $C_{\text{receive}}$, stored $Y$ / $\text{PVK}$, and allowance commitments were validated through path (1) when first written. The contract trusts them on subsequent reads.
3. **Auditor keys (the only proof-less entry point).** $K_{\text{aud}}$ is registered in the auditor contract by the auditor itself, with no accompanying proof. The auditor contract performs canonical encoding, on-curve ($y^2 \equiv x^3 - 17 \pmod{r}$), and non-identity checks at insertion (Section 3.1); the contract trusts the fetched value.

**Canonical encoding** ($x, y \in [0, r)$ as 32-byte representatives) is enforced at the XDR / Soroban host boundary when bytes are deserialized into `BnScalar`; no additional check is needed inside the contract.

---

## 11. Interface

Based on [EIP-7984](https://eips.ethereum.org/EIPS/eip-7984), adapted for Soroban. The `data: Bytes` parameter carries XDR-encoded proof payloads.

**Canonical encoding.** The `data` payloads are `#[contracttype]` structs and enums declared in the contract crate. Their on-chain byte representation is fixed by Soroban's XDR rules, which are canonical: every value has exactly one valid byte encoding. Named struct fields are serialised as an `ScMap` in declaration order; unnamed fields and tuple-enum variants as an `ScVec` in declaration order; map keys are host-enforced into a canonical sorted form. As a consequence, independent implementations that compile against the same `#[contracttype]` definitions produce byte-identical `data` payloads. The authoritative schemas for the underlying `ScVal`, `ScMap`, `ScVec` live in the [stellar/stellar-xdr](https://github.com/stellar/stellar-xdr) repository; the encoding rules are summarised in the [Stellar XDR documentation](https://developers.stellar.org/docs/learn/fundamentals/data-format/xdr) and the [`#[contracttype]` mapping reference](https://developers.stellar.org/docs/learn/fundamentals/contract-development/types/custom-types).

```rust
trait ConfidentialToken {
    fn __constructor(e: Env, admin: Address, token: Address,
                     verifier: Address, auditor: Address);

    fn register(e: Env, account: Address, auditor_id: u32, data: Bytes);

    fn deposit(e: Env, from: Address, to: Address, amount: i128);

    fn merge(e: Env, account: Address);

    fn withdraw(e: Env, from: Address, to: Address, amount: i128, data: Bytes);

    fn confidential_transfer(e: Env, from: Address, to: Address, data: Bytes);

    fn confidential_transfer_from(e: Env, operator: Address,
                                   from: Address, to: Address, data: Bytes);

    fn set_operator(e: Env, account: Address, operator: Address,
                    live_until_ledger: u32, data: Bytes);

    fn revoke_operator(e: Env, account: Address, operator: Address,
                       data: Bytes);

    fn confidential_balance(e: Env, account: Address) -> Bytes;

    fn is_operator(e: Env, account: Address, operator: Address) -> bool;

    fn get_operator(e: Env, account: Address, operator: Address) -> Bytes;
}
```

This table is authoritative: every entry is exactly the set of prover-supplied public inputs from the corresponding Section 7 operation (the contract loads the remaining public inputs from trusted state per §7.1), plus the `proof` blob. Names map directly to the Section 7 symbols.

| Operation | `data` contents |
|:---|:---|
| `register` | $Y$, $\text{PVK}$, `proof` |
| `withdraw` | $C_{\text{spend}}'$, $\tilde{b}$, $R_e$, $\sigma$, $\tilde{b}_{\text{aud,s}}$, `proof` |
| `confidential_transfer` | $C_{\text{spend}}'$, $C_{\text{tx}}$, $R_e$, $\tilde{v}$, $\tilde{b}$, $\sigma$, $\tilde{v}_{\text{aud,r}}$, $\tilde{r}_{\text{aud,r}}$, $\tilde{v}_{\text{aud,s}}$, $\tilde{b}_{\text{aud,s}}$, `proof` |
| `confidential_transfer_from` | $C_a'$, $C_{\text{tx}}$, $R_e$, $\tilde{v}$, $\tilde{a}'$, $\sigma_a'$, $\tilde{v}_{\text{aud,r}}$, $\tilde{r}_{\text{aud,r}}$, $\tilde{v}_{\text{aud,s}}$, $\tilde{a}_{\text{aud,s}}$, `proof` |
| `set_operator` | $C_{\text{spend}}'$, $C_a$, $\text{escrowed\\\_dvk}$, $\tilde{b}$, $\tilde{a}$, $R_e$, $\sigma$, $\sigma_a$, $\tilde{v}_{\text{aud,s}}$, $\tilde{b}_{\text{aud,s}}$, `proof` |
| `revoke_operator` | $C_{\text{spend}}'$, $\tilde{b}$, $R_e$, $\sigma$, $\tilde{v}_{\text{aud,s}}$, $\tilde{b}_{\text{aud,s}}$, `proof` |

For `confidential_transfer_from`, the stored allowance salt $\sigma_a$ is **not** carried in `data`: the contract loads it from the `(from, operator)` delegation entry (§7.8 public-input table). Only the prover-chosen replacement $\sigma_a'$ travels in `data`, gets bound by constraint O10, and is then written back to the delegation entry as the new `allowance_salt` (§6.2). This keeps the trust-boundary rule of §7.1 intact: caller-controlled bytes never overwrite the live $\sigma_a$ used to verify the proof. `set_operator`, by contrast, has no prior delegation entry to load from, so its $\sigma_a$ is prover-supplied and bound by S6.

### 11.1 Authorization Model

Soroban `address.require_auth()` proves that the named principal authorized the current invocation; it binds the full invocation (function name and all arguments) by default. ZK proof verification proves that the prover knows a witness satisfying the circuit's constraints over public inputs the contract itself supplies. The two are complementary: every state-changing operation requires **both** the appropriate `require_auth()` and (where applicable) a valid proof.

| Operation | `require_auth()` principal |
|:---|:---|
| `register(account, auditor_id, data)` | `account` |
| `deposit(from, to, amount)` | `from` |
| `merge(account)` | `account` |
| `withdraw(from, to, amount, data)` | `from` |
| `confidential_transfer(from, to, data)` | `from` |
| `confidential_transfer_from(operator, from, to, data)` | `operator` (not `from`) |
| `set_operator(account, operator, live_until_ledger, data)` | `account` |
| `revoke_operator(account, operator, data)` | `account` |
| `confidential_balance`, `is_operator`, `get_operator` | none (read-only) |

**`register` is single-use.** It reverts if `account` is already registered. Combined with `account.require_auth()`, this prevents a third party from binding attacker-controlled $(Y, \text{PVK})$ to `account`'s `ConfidentialAccount` slot.

**`set_operator` rejects replacement.** It reverts if a non-revoked delegation already exists for `(account, operator)` -- see §6.2.

**`confidential_transfer_from` is operator-authorized.** The owner's authorization was granted out-of-band at `set_operator` and persists in the on-chain delegation entry until expiry or revocation. The operator's `require_auth()` binds `from`, `to`, and `data`.

### 11.2 Event Schema

Each state-modifying operation emits a structured event. Events carry the data needed for recipient decryption, auditor decryption, and wallet recovery.

| Event | Fields |
|:---|:---|
| `Register` | `account`, `auditor_id` |
| `Deposit` | `from`, `to`, `amount` |
| `Merge` | `account` |
| `Withdraw` | `from`, `to`, `amount`, $R_e$, $\sigma$, $\tilde{b}$, $\tilde{b}_{\text{aud,s}}$ |
| `Transfer` | `from`, `to`, $R_e$, $\tilde{v}$, $\sigma$, $\tilde{b}$, $\tilde{v}_{\text{aud,r}}$, $\tilde{r}_{\text{aud,r}}$, $\tilde{v}_{\text{aud,s}}$, $\tilde{b}_{\text{aud,s}}$ |
| `OperatorTransfer` | `operator`, `from`, `to`, $R_e$, $\tilde{v}$, $\sigma_a$, $\tilde{v}_{\text{aud,r}}$, $\tilde{r}_{\text{aud,r}}$, $\tilde{v}_{\text{aud,s}}$, $\tilde{a}_{\text{aud,s}}$ |
| `SetOperator` | `account`, `operator`, `live_until_ledger`, $R_e$, $\sigma$, $\tilde{b}$, $\tilde{v}_{\text{aud,s}}$, $\tilde{b}_{\text{aud,s}}$ |
| `RevokeOperator` | `account`, `operator`, $R_e$, $\sigma$, $\tilde{b}$, $\tilde{v}_{\text{aud,s}}$, $\tilde{b}_{\text{aud,s}}$ |

Amount fields in `Deposit` and `Withdraw` are typed `i128`, matching SEP-41.

**Usage by consumers:**

- **Recipient wallet**: processes `Transfer` and `OperatorTransfer` events using $(R_e, \tilde{v}, \sigma)$ to derive $v_{\text{tx}}$ and $r_{\text{tx}}$ (Section 5.3).
- **Owner wallet**: processes all events for recovery (Section 5.2). The $(\tilde{b}, \sigma)$ pair from the most recent owner-initiated event forms a checkpoint.
- **Auditor**: processes events containing $R_e$ to compute ECDH shared secrets and decrypt amounts and balance checkpoints (Section 8.1, 8.2).

### 11.3 Read Methods

**`confidential_balance(account) -> Bytes`.** Returns the XDR-serialized `ConfidentialAccount` struct for the given account (§6.1), i.e. the tuple `(spending_key, viewing_public_key, spendable_balance, receiving_balance, auditor_id)`. Reverts if `account` is not registered. Wallets bootstrap from this call (single round-trip to obtain both Pedersen commitments plus the keys needed to identify the account and its bound auditor); indexers use it to verify consistency between their replayed accumulators and on-chain state (§5.2 "Consistency check").

**`is_operator(account, operator) -> bool`.** Returns `true` iff a delegation entry exists for `(account, operator)` **and** `ledger.sequence() <= live_until_ledger`. Returns `false` for:

- pairs with no delegation entry,
- pairs whose entry has `ledger.sequence() > live_until_ledger` (expired-but-not-yet-revoked: the escrowed value still resides on-chain in $C_a$ until `revoke_operator` reclaims it -- §6.2 *Single-slot semantics* -- but the operator can no longer spend),
- pairs whose entry was revoked (deleted) by `revoke_operator`.

The function returns the *spending-authority* state, not the *escrow-existence* state. Consumers that need to distinguish "no delegation" from "expired delegation" inspect `get_operator` (below) or replay `SetOperator` / `RevokeOperator` events.

**`get_operator(account, operator) -> Bytes`.** Returns the XDR-serialized `OperatorDelegation` struct (§6.2) for the `(account, operator)` pair, i.e. `(allowance_commitment, encrypted_allowance, escrowed_dvk, allowance_salt, live_until_ledger)`. Reverts if no delegation entry exists for the pair. Unlike `is_operator`, this surfaces the raw on-chain delegation state without applying the expiry filter, so callers can distinguish "no delegation" (revert) from "active delegation" (`ledger.sequence() <= live_until_ledger`) from "expired-but-not-yet-revoked delegation" (`ledger.sequence() > live_until_ledger`, escrowed value still pending reclaim). Primary consumers:

- **Operator wallet:** fetches `allowance_commitment`, `encrypted_allowance`, `escrowed_dvk`, and `allowance_salt` to recover $dvk_i$ via §7.11 decryption, then reads the current allowance via $\tilde{a} = v_a + \text{Poseidon}(\delta_{\text{enc\\\_allow}}, dvk_i, \sigma_a)$ to construct the next `confidential_transfer_from` witness.
- **Owner wallet:** reads the same fields after losing local state, or before calling `revoke_operator`, to confirm the on-chain entry matches its records.
- **Indexers:** verify their replayed delegation state against the live commitment, in the same way `confidential_balance` is used for account state (§5.2 "Consistency check").

The auditor's allowance tracking does **not** use this method: per-event allowance ciphertexts (§8.5) are the auditor's data path; `encrypted_allowance` is keyed to $dvk_i$ and is unreadable without it.

---

## 12. Dependencies

| Dependency | Status | Impact |
|:---|:---|:---|
| **Protocol 25** (BN254 native support) | Available | `bn254.g1_add()`, `g1_mul()`, `pairing_check()`, `BnScalar` Fr arithmetic |
| **CAP-80** (BN254 host functions) | Available | Required for efficient UltraHonk verification and Grumpkin point arithmetic |
| **Modified UltraHonk verifier** | To be built | Multi-VK support (one per circuit type) |
| **Noir circuits** | To be built | 6 circuits using `std::embedded_curve_ops` for Grumpkin |
| **Grumpkin point arithmetic library** | To be built | On-chain point add/sub using BN254 Fr ops, identity handling |
| **Auditor contract** | To be built | Independent key management contract |
| **Nargo / Barretenberg** | Available (`nargo 1.0.0-beta.11`, `bb v0.87.0`) | Off-chain proof generation |
| **Client library** | To be built | ECDH key agreement, Poseidon-based amount encryption/decryption, event processing, off-chain balance tracking |

---

## 13. Domain Separation Constants

Each $\delta$ is a small positive integer in $\mathbb{F}_r$, fixed for the protocol version and used as a Poseidon2 leading-input domain tag. Numeric values are assigned sequentially from 1; the protocol version is implicit in the deployment, and any change to a circuit's constraint that uses these tags requires a new deployment with a fresh verification key (§3.5, §10.6).

| $\delta$ | Value | Context |
|:---|:---:|:---|
| $\delta_{\text{addr}}$ | 1 | Soroban Address compression into a single $\mathbb{F}_r$ Field (§2.7) |
| $\delta_{\text{vk}}$ | 2 | Viewing key derivation from spending key and contract address (§4.2) |
| $\delta_{\text{dvk}}$ | 3 | Delegation viewing key derivation (§4.4) |
| $\delta_{\text{spend\\\_r}}$ | 4 | Deterministic randomness for spendable balance commitments (§5.2 *Update rules*) |
| $\delta_{\text{tx\\\_blind}}$ | 5 | ECDH-derived transfer blinding factor (§5.3 Definition 1) |
| $\delta_{\text{tx\\\_amount}}$ | 6 | ECDH-derived transfer amount encryption (§5.3 Definition 1) |
| $\delta_{\text{enc\\\_bal}}$ | 7 | Encrypted balance scalar masking (§5.5) |
| $\delta_{\text{enc\\\_allow}}$ | 8 | Encrypted allowance scalar masking (§6.2 *encrypted\_allowance*) |
| $\delta_{\text{allow\\\_r}}$ | 9 | Deterministic randomness for operator allowance commitments (§6.2 *allowance\_commitment*) |
| $\delta_{\text{esc\\\_dvk}}$ | 10 | Delegation key escrow (operator ECDH) (§7.11) |
| $\delta_{\text{aud\\\_s}}$ | 11 | Sender / owner-auditor channel sponge (§2.5, §8.1) |
| $\delta_{\text{aud\\\_r}}$ | 12 | Recipient-auditor channel sponge (§2.5, §8.1) |

**Provenance.** Sequential small integers are the simplest assignment that satisfies the requirement of *distinctness* across all Poseidon2 invocations in this protocol -- Poseidon2 is collision-resistant under the assumption of §3.2, so any two distinct leading inputs (independent of size) produce independent outputs. The values themselves carry no semantic meaning; the binding is purely positional and the table is the only authoritative source. Implementations MUST hardcode these exact numeric values; deviations break cross-implementation derivation of $vk$, $dvk_i$, $\tilde{v}$, $\tilde{b}$, $\tilde{a}$, $r_{\text{tx}}$, $r_a$, and all auditor masks.

**Cross-protocol collision.** Future protocols that share Grumpkin / BN254 / Poseidon2 with this protocol -- e.g. an unrelated payments protocol that uses small-integer Poseidon2 domains -- could in principle pick the same numeric values for unrelated purposes. The protocol assumes that the surrounding inputs to Poseidon2 (key material, structural witnesses) sufficiently disambiguate even in such a case; no Poseidon2 invocation in this protocol is keyed solely on a $\delta$ value. If stronger isolation is desired, implementers may instead use the alternate scheme $\delta_X = \text{Poseidon2}(0, \text{ASCII}(\text{"openzeppelin/confidential-token/v1:X"}))$, but this is a deployment-time choice that must be applied uniformly and disclosed in the deployment's circuit-binding documentation.
