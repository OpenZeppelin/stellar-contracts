# Proof System

## Circuits

All proof logic is written in Noir, compiled to UltraHonk circuits. The system uses six circuits, the last belonging to the optional compliance extension:

```rust
#[contracttype]
#[repr(u32)]
pub enum CircuitType {
    Register = 0,
    Withdraw = 1,
    Transfer = 2,
    SpenderTransfer = 3,
    SetSpender = 4,
    Clawback = 100,
}
```

`deposit`, `merge`, and `revoke_spender` carry no proof and have no circuit ([Deposit](operations/deposit.md), [Merge](operations/merge.md), [Revoke Spender](operations/revoke-spender.md)). The discriminants are part of the on-chain interface and are never renumbered; core circuits occupy `0..=4` and extension circuits start at `100`.

## Circuit Summary

| Circuit | What it proves |
|:---|:---|
| `Register` | Spending key well-formedness; contract-bound viewing key derivation from $$sk$$; public viewing key consistency with the derived $$vk$$ |
| `Withdraw` | Balance sufficiency; new spendable commitment with deterministic randomness; encrypted balance scalar; sender-auditor ECDH ciphertexts (balance checkpoint + `lane[2]` escrow of the new spendable blinding); owner key ownership |
| `Transfer` | Balance conservation; ECDH-derived blinding and encrypted amount for recipient; dual-auditor channel sponges (recipient auditor: amount + per-transfer Pedersen randomness; sender auditor: amount + balance + `lane[2]` escrow of the new spendable blinding); deterministic randomness for new sender balance; encrypted balance scalar; sender key ownership; range validity (balance $$\in [0, 2^{127})$$, amount $$\in [0, 2^{127})$$) |
| `SpenderTransfer` | Allowance sufficiency; ECDH-derived blinding and encrypted amount for recipient; dual-auditor channel sponges (recipient auditor: amount + per-transfer Pedersen randomness; owner auditor: amount + allowance + `lane[2]` escrow of the new allowance blinding); deterministic randomness for new allowance; encrypted allowance scalar; spender key ownership; contract-bound indirectly via $$C\_a$$ chain ([Spender Transfer](operations/spender-transfer.md)) |
| `SetSpender` | Balance split; $$dvk\_i$$ derivation; ECDH escrow of $$dvk\_i$$ to the spender and of the allowance blinding to the owner's auditor; allowance commitment with deterministic randomness; encrypted balance and allowance scalars; owner-auditor ECDH ciphertexts (escrow amount + balance checkpoint + `lane[2]` escrow of the new spendable blinding); owner key ownership; contract-bound via $$vk$$ derivation |
| `Clawback` | Knowledge of the openings of the target's $$C\_{\text{spend}}$$ and $$C\_{\text{receive}}$$; public seize amount bounded by their sum, remainder in range; no key ownership, no ephemeral ([Circuit](../compliance.md#circuit)) |

## Circuit Cost Analysis

The dominant cost in Noir circuits is elliptic curve scalar multiplication. With Barretenberg's native Grumpkin support via `multi_scalar_mul`, each scalar multiplication costs approximately 64 UltraPlonk-equivalent constraints (with ECC VM) or 4,700–6,250 without.

**Counting unit.** The totals below count `multi_scalar_mul` *call sites*, matching how the circuits are written (`circuits/lib/src/lib.nr`): a Pedersen commitment is one call over two scalars, and each `ecdh` is one call plus a Poseidon2 absorb ([Elliptic Curve Diffie-Hellman](primitives.md#elliptic-curve-diffie-hellman)). A per-scalar count is higher, since every commitment contributes two. Each row lists every site, so the constraint identifiers are the audit trail.

| Circuit | Calls | Sites |
|:---|:--:|:---|
| `Register` | 2 | $$Y$$ (R1), $$\text{PVK}$$ (R3) |
| `Withdraw` | 5 | $$Y$$ (W1), $$C\_{\text{spend}}$$ opening (W3), $$C\_{\text{spend}}'$$ (W6), $$R\_e$$ (W\_a1), sender-auditor ECDH (W\_a2) |
| `Transfer` | 8 | $$Y\_A$$ (T1), $$C\_{\text{spend}}^A$$ opening (T3), recipient ECDH (T5), $$R\_e$$ (T6), $$C\_{\text{transfer}}$$ (T8), $$C\_{\text{spend}}'$$ (T11), recipient-auditor ECDH (T\_a1), sender-auditor ECDH (T\_a5) |
| `SpenderTransfer` | 8 | $$Y\_{\text{op}}$$ (O1), $$C\_a$$ opening (O2), recipient ECDH (O5), $$R\_e$$ (O6), $$C\_{\text{transfer}}$$ (O8), $$C\_a'$$ (O11), recipient-auditor ECDH (O\_a1), owner-auditor ECDH (O\_a5) |
| `SetSpender` | 7 | $$Y$$ (S1), $$C\_{\text{spend}}$$ opening (S3), $$C\_a$$ (S7), $$C\_{\text{spend}}'$$ (S10), $$R\_e$$ (S\_a1), $$dvk\_i$$ escrow ECDH (S12, [Delegation Key Escrow](operations/set-spender.md#delegation-key-escrow)), owner-auditor ECDH (S\_a2) |
| `Clawback` | 2 | $$C\_{\text{spend}}$$ opening (CB1), $$C\_{\text{receive}}$$ opening (CB2) |

`SetSpender` is the one circuit with a third ECDH beyond the auditor channel: the $$dvk\_i$$ handoff of [Delegation Key Escrow](operations/set-spender.md#delegation-key-escrow) reuses $$r\_e$$ but multiplies it against $$Y\_{\text{op}}$$, so it is a separate call, not a reuse of the S\_a2 shared secret. The auditor-side escrow of the allowance blinding $$r\_a$$ (S14) reuses the S\_a2 shared scalar and adds one Poseidon evaluation. The `lane[2]` escrows (W\_a5, T\_a9, S\_a6, O\_a9) read `lane[2]` of a permutation each circuit already computes and cost one field addition apiece. The ordering these totals imply is consistent with the committed ACIR opcode counts in `circuits/constraints.baseline`.

The ECDH computations add scalar multiplications compared to a random-blinding scheme, but the unchunked design eliminates all per-chunk constraints (which, in a chunked scheme, would involve 8+ scalar multiplications for balance chunks and per-chunk range proofs).

## Noir Primitives

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
/// blindings, and (for the spend-side input opening of C_spend in W3/T3/S3 and
/// of both balance commitments in CB1/CB2)
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

/// ECDH shared-secret scalar (primitives.md#elliptic-curve-diffie-hellman): S = scalar * point, then
/// s = Poseidon2(delta_ecdh, S.x, S.y). Absorbing both coordinates binds
/// the full shared point, removing the (P, -P) negation invariance of an
/// x-only extraction. All ECDH scalars in this protocol are F_r-sampled
/// (sk, vk, r_e_point), so single-limb conversion is sound.
fn ecdh(scalar: Field, point: EmbeddedCurvePoint) -> Field {
    let s = multi_scalar_mul(
        [point],
        [EmbeddedCurveScalar::from_field(scalar)]
    );
    poseidon_with_domain(ECDH_SHARED_SECRET, [s.x, s.y])
}
```

### Post-merge witness availability

Every opening witnessed in any circuit is encoded as a single $$\mathbb{F}\_r$$ `Field` via the same `commit` primitive. After a proofless fold -- `Merge` ([Merge](operations/merge.md)), `revoke_spender` ([Revoke Spender](operations/revoke-spender.md)), or `clawback` ([Contract Flow](../compliance.md#contract-flow), where the compliance extension is enabled) -- the spendable-balance blinding is a sum over $$\mathbb{F}\_q$$: $$r\_s + r\_r$$ for a merge or a clawback, $$r\_s + r\_a$$ for a revoke. Its canonical $$\mathbb{F}\_q$$ representative lies in $$[0, r)$$ -- representable as a Noir `Field` -- with probability $$\geq 1 - (q - r)/q \approx 1 - 2^{-127}$$ per fold ([Pedersen Commitments](primitives.md#pedersen-commitments)). With the complementary probability $$\approx 2^{-127}$$ it lies in $$[r, q)$$. In that case the on-chain state remains well-formed (the commitment is a valid Grumpkin point), but the wallet's local opening witness is unencodable as a `Field`, so no spend / transfer / set-spender proof can be constructed against the affected $$C\_{\text{spend}}$$ until further accumulation shifts the blinding back into $$\mathbb{F}\_r$$.

### Soft recovery

Every subsequent inbound confidential transfer or spender transfer, once merged, adds a fresh $$\mathbb{F}\_r$$-derived blinding. For each transfer-derived addend, the new canonical $$\mathbb{F}\_q$$ representative falls in $$[0, r)$$ with probability $$\geq 1 - 2^{-127}$$ regardless of the current stuck value (worst case: the current value sits at the lower edge of $$[r, q)$$, requiring the new $$\mathbb{F}\_r$$ addend to cross the mod $$q$$ boundary; the probability of failing to do so is bounded by $$(q - r)/r \approx 2^{-127}$$). For accounts that continue to receive confidential transfers, the unspendable window is self-resolving at the next merge with overwhelming probability; accounts whose only inflows are deposits remain stuck until a confidential transfer arrives.

## Verification Flow

1. Contract reads on-chain state (commitments, public keys)
2. Encodes state as public inputs: Grumpkin point coordinates as 32-byte $$\mathbb{F}\_r$$ values
3. Cross-contract call: `verifier.verify_proof(circuit_type, public_inputs, proof)`
4. Verifier deserializes stored VK, runs UltraHonk verification (BN254 G1/G2 pairings, Fiat-Shamir, sumcheck)
5. Contract applies homomorphic balance updates (Grumpkin point arithmetic via $$\mathbb{F}\_r$$ ops)

The proofless operations (`deposit`, `merge`, `revoke_spender`) perform step 5 only.

## Structured Reference String

UltraHonk is a PLONK-family proving system. Its knowledge soundness guarantee depends on a **Structured Reference String (SRS)** -- a sequence of BN254 G1 and G2 points derived from a secret scalar $$\tau$$ (the "toxic waste"):

$$\text{SRS} = \bigl([1]\_1, [\tau]\_1, [\tau^2]\_1, \ldots, [\tau^{N-1}]\_1, \\; [1]\_2, [\tau]\_2\bigr)$$

where $$[x]\_1 = x \cdot G\_1$$ and $$[x]\_2 = x \cdot G\_2$$ are BN254 group elements. The SRS is **universal**: a single SRS supports any circuit up to size $$N$$, and circuit-specific verification keys are derived from it deterministically. The SRS is used during both proof generation (client-side) and verification key derivation (one-time setup).

**Security requirement.** If $$\tau$$ is known to an attacker, they can forge proofs for arbitrary false statements: minting tokens, draining accounts, bypassing all circuit constraints. The knowledge soundness of the entire system reduces to the assumption that $$\tau$$ was destroyed after SRS generation.

**Multi-party ceremony.** The standard mitigation is a multi-party computation (MPC) ceremony in which $$N$$ participants each contribute randomness. The resulting SRS is secure if *at least one* participant honestly destroyed their contribution.

**SRS used in this system.** The Noir/Barretenberg toolchain uses the **Aztec Ignition SRS** by default. Barretenberg downloads the required SRS points from a public transcript on first use. The Ignition ceremony transcript, participant attestations, and verification code are publicly available. The SRS supports circuits up to $$2^{28}$$ gates, well above the expected circuit sizes for this system ($$< 2^{20}$$).

**Deployment considerations.** The verifier contract does not store or reference the full SRS. Circuit-specific verification keys are derived offline from the SRS during circuit compilation and embedded in the verifier contract at deployment. The correctness of these VKs can be independently verified by anyone with access to the circuit source code and the public SRS transcript.

**Risk assessment.** The Ignition ceremony had 176 independent participants across multiple jurisdictions, hardware platforms, and operating systems. Compromise requires collusion of *all* 176 participants.

## Dependency: CAP-80

[CAP-80](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0080.md) introduces the host functions required for efficient UltraHonk verification and on-chain Grumpkin point arithmetic. The rollout spans two protocols; both are required, so **protocol 26 is the effective minimum**.

- **Protocol 25:** `bn254_g1_{add, mul}`, `bn254_multi_pairing_check`.
- **Protocol 26:** `bn254_g1_msm`, `bn254_g1_is_on_curve`, `bn254_fr_{add, sub, mul, inv, pow}` -- the $$\mathbb{F}\_r$$ scalar arithmetic underpinning Grumpkin point operations.

## On-Chain Point Arithmetic

The contract performs Grumpkin affine point addition and subtraction for homomorphic balance updates. Since Grumpkin coordinates are $$\mathbb{F}\_r^{\text{BN254}}$$ elements, these reduce to Fr field operations.

**Curve coefficients.** Grumpkin $$y^2 = x^3 - 17$$ ([Grumpkin-BN254 Cycle](primitives.md#grumpkin-bn254-cycle)) is in short Weierstrass form $$y^2 = x^3 + a x + b$$ with $$a = 0$$ and $$b = -17$$. Only $$a$$ enters the point arithmetic slope formulas below; $$b$$ enters only the on-curve check.

The contract distinguishes the following cases when computing $$P\_3 = P\_1 + P\_2$$:

| Case | Condition | Result |
|:--|:--|:--|
| Left identity | $$P\_1 = \mathcal{O}$$ | $$P\_3 = P\_2$$ |
| Right identity | $$P\_2 = \mathcal{O}$$ | $$P\_3 = P\_1$$ |
| Inverse | $$P\_1, P\_2 \neq \mathcal{O}$$, $$x\_1 = x\_2$$, $$y\_1 = -y\_2 \bmod r$$ | $$P\_3 = \mathcal{O}$$ |
| Doubling | $$P\_1, P\_2 \neq \mathcal{O}$$, $$P\_1 = P\_2$$ (so $$y\_1 \neq 0$$) | slope formula with $$\lambda\_{\text{dbl}}$$ below |
| Generic | $$P\_1, P\_2 \neq \mathcal{O}$$, $$x\_1 \neq x\_2$$ | slope formula with $$\lambda\_{\text{add}}$$ below |

The inverse case must be detected and short-circuited before the generic slope formula, because $$x\_1 - x\_2 = 0$$ would otherwise force a division by zero in $$\mathbb{F}\_r$$.

**Slope.**

$$\lambda\_{\text{add}} = (y\_2 - y\_1)(x\_2 - x\_1)^{-1} \pmod{r}$$

$$\lambda\_{\text{dbl}} = (3 x\_1^2 + a)(2 y\_1)^{-1} = 3 x\_1^2 \cdot (2 y\_1)^{-1} \pmod{r} \qquad (a = 0 \text{ for Grumpkin})$$

**Resulting coordinates.** With $$\lambda$$ selected per the case above:

$$x\_3 = \lambda^2 - x\_1 - x\_2 \pmod{r}$$
$$y\_3 = \lambda (x\_1 - x\_3) - y\_1 \pmod{r}$$

Requires `bn254_fr_{add, sub, mul, inv}` host calls (CAP-80, [Dependency: CAP-80](#dependency-cap-80)).

**Point subtraction** $$P\_3 = P\_1 - P\_2$$: if $$P\_2 = \mathcal{O}$$ set $$-P\_2 = \mathcal{O}$$, else $$-P\_2 = (x\_2, -y\_2 \bmod r)$$; then apply the addition cases above. Subtraction of a point from itself yields $$\mathcal{O}$$ via the inverse case, never the doubling branch.

**Point validation.** Grumpkin points enter the system through three boundaries; on-curve and non-identity checks live at the boundary that owns each one. The contract itself performs no per-call on-curve check.

1. **Proof-constrained points (the dominant case).** Every public input that the corresponding circuit also derives via `multi_scalar_mul` is on-curve by construction -- Noir's embedded-curve operations cannot produce an off-curve Grumpkin point. This covers $$Y$$ (R1), $$\text{PVK}$$ (R3), $$R\_e$$ (T6, O6, W_a1, S_a1), $$C\_{\text{transfer}}$$ (T8, O8), $$C\_{\text{spend}}'$$ (T11, W6, S10), $$C\_a$$ / $$C\_a'$$ (S7, O11), and the ECDH shared secrets. Non-identity is enforced *in-circuit* by explicit nonzero-scalar constraints: $$sk \neq 0$$ and $$vk \neq 0$$ at registration (R4, R5), and $$r\_e \neq 0$$ in every circuit that produces an ephemeral key (W8, T13, S13, O13). Without these constraints an adversary could publish $$Y = \mathcal{O}$$, $$\text{PVK} = \mathcal{O}$$, or $$R\_e = \mathcal{O}$$ and collapse ECDH (every shared secret becomes $$\mathcal{O}$$, every Poseidon mask becomes a constant function of the operation's salt, every ciphertext becomes trivially decryptable).
2. **Points read from prior on-chain state.** $$C\_{\text{spend}}$$, $$C\_{\text{receive}}$$, stored $$Y$$ / $$\text{PVK}$$, and allowance commitments were validated through path (1) when first written. The contract trusts them on subsequent reads.
3. **Auditor keys (the only proof-less entry point).** $$K\_{\text{aud}}$$ is registered in the auditor contract by the auditor itself, with no accompanying proof. The auditor contract performs canonical encoding, on-curve ($$y^2 \equiv x^3 - 17 \pmod{r}$$), and non-identity checks at insertion ([Components](system-model.md#components)); the contract trusts the fetched value.

**Canonical encoding** ($$x, y \in [0, r)$$ as 32-byte representatives) is enforced **by the contract** at the verifier boundary, not by the Soroban host, which reduces non-canonical inputs rather than rejecting them ([Host deserialiser caveat](primitives.md#host-deserialiser-caveat)). Every prover-supplied scalar and coordinate that reaches the verifier — and therefore every byte string that gets persisted or emitted downstream — is the unique canonical representative of its field element.

---

Previous: [Security Analysis](security.md) · Up: [Index](../README.md#protocol) · Next: [Interface](interface.md)
