# System Model

## Components

The system comprises three contracts deployed on Soroban:

**Token contract.** Holds SEP-41 token balances, manages encrypted account state, and delegates proof verification via cross-contract calls. Performs Grumpkin point arithmetic through $$\mathbb{F}_r$$ host operations for homomorphic balance updates.

**Verifier contract.** An UltraHonk verifier storing one verification key per circuit type. Accepts a circuit identifier, serialized public inputs, and a proof blob; returns success or failure. Builds on top of [NethermindEth/rs-soroban-ultrahonk](https://github.com/NethermindEth/rs-soroban-ultrahonk).

**Auditor contract.** Manages auditor encryption keys independently of the contract. One auditor contract serves multiple token contracts. Stores Grumpkin public keys as full affine points $$(x, y)$$ indexed by `auditor_id`. The contract validates that stored keys are non-identity curve points; a zero or identity key would make ECDH-derived ciphertexts trivially decryptable (since $$\sigma$$ is public). The contract fetches the active auditor key at operation time and passes it as a public input to the relevant circuit.

## Threat Model

- The contract execution environment is trusted for correctness but not for privacy: all on-chain state and invocation inputs are public.
- Proof verification is sound: a valid proof guarantees the proven statement holds. This depends on the UltraHonk knowledge soundness assumption *and* the integrity of the Structured Reference String ([Structured Reference String](proof-system.md#structured-reference-string)).
- The discrete logarithm problem on Grumpkin is hard.
- Poseidon2 ([Poseidon2 Hash](primitives.md#poseidon2-hash)) is a pseudorandom function (PRF) and is preimage-resistant over $$\mathbb{F}_r$$ at the parameterized round count ($$R_F = 8$$, $$R_P = 56$$, 128-bit security target).
- Third parties may submit arbitrary transactions, including spam transfers to any registered account.

## Trust Assumptions

The contract, verifier, and auditor contracts are trusted code. Users trust that the verification keys embedded in the verifier correspond to the correct circuits and were derived from a honestly generated Structured Reference String ([Structured Reference String](proof-system.md#structured-reference-string)). The auditor is trusted to protect its decryption key and exercise access only upon legitimate regulatory request.

## Underlying Token Assumptions

The contract holds units of an underlying SEP-41 token on behalf of its users. The confidential accounting invariant ([Balance Conservation](security.md#balance-conservation)) implicitly assumes:

$$\sum_i v_{\text{committed},i} \\;\le\\; \text{token.balance}(\text{contract})$$

i.e., the total committed value across all confidential accounts never exceeds the public token balance held by the contract. The deployer's choice of underlying token determines whether that invariant is actually preserved over time. The contract itself does not, and cannot, defend against every misbehavior of the wrapped asset.

**Required properties of the underlying token.**

- *Non-rebasing.* The token's balance attributed to the contract address changes only as a result of explicit operations that the contract itself originated. Tokens whose balances change as a function of supply, oracle data, or external triggers break the accounting invariant and are unsupported.
- *No fee-on-transfer.* `token.transfer(from, to, amount)` MUST move exactly `amount` units. A fee deducted in transit would leave the contract's confidential accounting larger than its public backing.
- *Deterministic revert.* A failed `token.transfer` MUST cause the enclosing contract invocation (`deposit` or `withdraw`) to revert atomically, so confidential state is never updated against a token transfer that did not happen.
- *Underlying clawback / freeze / deauthorization.* These are surfaces of the Stellar Asset Contract (`StellarAssetInterface`), not the generic SEP-41 (`TokenInterface`). If the underlying token is a SAC whose issuer can clawback, freeze, or deauthorize the contract's holdings, confidential accounting at the contract layer may temporarily or permanently exceed the contract's accessible backing. This is an operational risk borne by the deployer's choice of underlying token. The token layer offers its own freeze and per-account clawback flows that operate inside the confidential surface; see [Contract-Level Freeze](../compliance.md#contract-level-freeze) (contract-level freeze) and [Commitment Scheme](keys-and-commitments.md#commitment-scheme) (admin + auditor clawback). [SAC Authorization Passthrough](../compliance.md#sac-authorization-passthrough) additionally specifies SAC authorization passthrough, which composes the contract's freeze with the issuer's freeze without requiring the admin to mirror state.

**Non-negativity check.** The contract's public interface uses `i128` end-to-end, matching SEP-41. Every entrypoint that accepts a public amount (`deposit`, `withdraw`) MUST reject `amount < 0` and revert. The in-circuit range constraint ([Integer Embedding and Range Proofs](primitives.md#integer-embedding-and-range-proofs)) bounds the same value at $$2^{127}$$ from above; together they pin the contract's value domain to $$[0, 2^{127}) = [0, \text{i128::MAX}]$$, matching SEP-41 exactly. No conversion at the SEP-41 boundary is needed.

## Governance and Upgradeability

The constructor binds the contract to fixed `admin`, `token`, `verifier`, and `auditor` addresses. It additionally computes and stores `addr_f = address_to_field(env.current_contract_address())` ([Address-to-Field Encoding](primitives.md#address-to-field-encoding)) in **instance storage** as a single canonical $$\mathbb{F}_r$$ Field; this is the value every owner-initiated proof references via constraints R2 / W2 / T2 / S2. The compressed `addr_f` Field is computed once at construction (not recomputed per call) to ensure all proofs across the contract's lifetime bind to the same Field representative of the contract's address. Beyond that, this specification does not prescribe a governance policy for upgrading these components or for rotating per-circuit verification keys. Concrete deployments differ widely in spender structure, regulatory posture, and emergency-response requirements, so these decisions are deliberately left to implementers.

Questions an implementer must answer:

- May `admin` replace the `verifier` contract, or the `auditor` contract after deployment?
- Are per-circuit verification keys immutable for the lifetime of the deployment, or may they be updated?
- If any of the above is upgradeable, what authorization (single key, multisig), timelock, and event-emission rules apply?
- How do users independently reproduce a deployed VK from circuit source, toolchain, and SRS ([Structured Reference String](proof-system.md#structured-reference-string))?

**Recommendation.** The strongest soundness posture is full immutability: `token`, `verifier`, `auditor`, and per-circuit verification keys all fixed at deployment, with any circuit or verifier change requiring a fresh deployment and an explicit user-side migration. Where operational realities make full immutability impractical (for example, a discovered soundness bug in a circuit or verifier that needs a fast fix), implementers may expose admin-guarded upgrade entrypoints for the `verifier` address or per-circuit VKs. In that case the upgrade path should be gated.

---

Previous: [Primitives](primitives.md) · Up: [Index](../README.md#protocol) · Next: [Key Hierarchy and Commitment Scheme](keys-and-commitments.md)
