# Key Hierarchy and Commitment Scheme

## Key Hierarchy

All keys derive from a single spending secret $$sk \in \mathbb{F}_r$$.

### Spending Key

$$Y = sk \cdot H$$

The spending public key is stored on-chain at registration. Knowledge of $$sk$$ is required to authorize transfers, withdrawals, spender delegations, and merges.

### Viewing Key

$$vk = \text{Poseidon}(\delta_{\text{vk}}, sk, \text{addr\\\_f})$$

A scalar in $$\mathbb{F}_r$$, unique per $$(sk, \text{addr\\\_f})$$ pair. Enables balance decryption without spending authority, and — through the ephemeral-scalar derivation of [ECDH-Derived Blinding](#ecdh-derived-blinding) — reconstruction of the Pedersen openings of transfers the account originated; [Privacy Properties](security.md#privacy-properties) states the full capability of a compromised $$vk$$. Cannot recover $$sk$$ (Poseidon preimage resistance). Because $$\text{addr\\\_f}$$ is bound into the derivation, proofs that constrain $$vk$$ (R2, W2, T2, S2) are inherently bound to the contract, eliminating the need for explicit per-circuit context binding.

### Public Viewing Key

$$\text{PVK} = vk \cdot H$$

A Grumpkin point stored on-chain at registration. Serves as the recipient's ECDH public key for incoming transfers. The registration proof constrains $$\text{PVK} = vk \cdot H$$ where $$vk = \text{Poseidon}(\delta_{\text{vk}}, sk, \text{addr\\\_f})$$ and $$Y = sk \cdot H$$, preventing a user from registering an unrelated $$\text{PVK}$$.

### Delegation Viewing Key

For spender $$i$$ with address $$\text{op}_i$$, the owner derives:

$$dvk_i = \text{Poseidon}(\delta_{\text{dvk}}, vk, \text{op}_i)$$

Properties:
- $$dvk_i$$ reveals only this spender's allowance state in this contract's context ($$vk$$ is contract-specific, [Viewing Key](#viewing-key)).
- $$dvk_i$$ cannot recover $$vk$$ (preimage resistance).
- Different $$(vk, \text{op}_i)$$ tuples yield independent keys.

---

## Commitment Scheme

The following symbols are used throughout this section:

| Symbol | Definition |
|:---|:---|
| $$C_{\text{spend}}$$ | On-chain spendable balance commitment (Pedersen point) |
| $$C_{\text{receive}}$$ | On-chain receiving balance commitment (Pedersen point) |
| $$C_{\text{transfer}}$$ | Transfer commitment added to recipient's $$C_{\text{receive}}$$ |
| $$\text{Com}(v, r)$$ | Pedersen commitment $$v \cdot G + r \cdot H$$ |
| $$v_s, r_s$$ | Value and blinding factor of $$C_{\text{spend}}$$ (off-chain wallet state) |
| $$v_r, r_r$$ | Value and blinding factor of $$C_{\text{receive}}$$ (off-chain wallet state) |
| $$v_{\text{transfer}}$$ | Transfer amount (private) |
| $$r_{\text{transfer}}$$ | ECDH-derived blinding factor for $$C_{\text{transfer}}$$ |
| $$W_{\text{spend}}, W_{\text{receive}}$$ | Wallet-side accumulators: $$(v, r)$$ pairs tracking commitment openings |
| $$r_e$$ | Ephemeral scalar, derived per operation from the $$vk$$ and the operation salt ([ECDH-Derived Blinding](#ecdh-derived-blinding)) |
| $$R_e$$ | Ephemeral public key $$r_e \cdot H$$ (published in event data) |
| $$S$$ | ECDH shared secret point $$r_e \cdot \text{PVK}_B$$ |
| $$s$$ | Scalar shared secret $$\text{ECDH}(r_e, \text{PVK}_B) = \text{Poseidon}(\delta_{\text{ecdh}}, S.x, S.y)$$ ([Elliptic Curve Diffie-Hellman](primitives.md#elliptic-curve-diffie-hellman)) |
| $$\tilde{v}$$ | Encrypted transfer amount: $$v_{\text{transfer}} + \text{Poseidon}(\delta_{\text{transfer\\\_amount}}, s, \sigma)$$ |
| $$\tilde{b}$$ | Encrypted balance scalar: $$v_{\text{new}} + \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$$ |
| $$\sigma$$ | Prover-chosen random salt, sampled per operation via the rejection sampling procedure of [Grumpkin-BN254 Cycle](primitives.md#grumpkin-bn254-cycle); canonical $$\mathbb{F}_r$$ representative encoded as `BytesN<32>` |

### Balance Commitments

Each balance is a single Pedersen commitment $$C = \text{Com}(v, r) \in \mathbb{G}$$, represented on-chain as an uncompressed affine point $$(x, y) \in \mathbb{F}_r^2$$ (64 bytes). The identity $$\mathcal{O}$$ is encoded as $$(0, 0)$$ and handled as a special case in point arithmetic.

The committed value $$v$$ can represent the full range of practical balances (up to $$2^{127} - 1$$, bounded by the SEP-41 `i128` interface) without discrete logarithm concerns, because the owner maintains the commitment opening off-chain ([Wallet State and Recovery](wallet-state.md)) and the auditor reads an encrypted scalar ([Per-Transfer Auditor Ciphertexts](auditing.md#per-transfer-auditor-ciphertexts)).

### ECDH-Derived Blinding

When a sender (spending key $$sk_A$$) transfers to a recipient with public viewing key $$\text{PVK}_B$$, the transfer commitment uses blinding derived from an ephemeral ECDH exchange.

**Definition 1** (Transfer blinding derivation). The sender samples $$\sigma \in \mathbb{F}_r$$ via the rejection sampling procedure ([Grumpkin-BN254 Cycle](primitives.md#grumpkin-bn254-cycle)), then computes:

$$r_e = \text{Poseidon}(\delta_{\text{eph}}, vk_A, \sigma)$$
$$R_e = r_e \cdot H$$
$$S = r_e \cdot \text{PVK}_B$$
$$s = \text{Poseidon}(\delta_{\text{ecdh}}, S.x, S.y) \in \mathbb{F}_r \qquad \text{(Elliptic Curve Diffie-Hellman)}$$
$$r_{\text{transfer}} = \text{Poseidon}(\delta_{\text{transfer\\\_blind}}, s, \sigma)$$
$$\tilde{v} = v_{\text{transfer}} + \text{Poseidon}(\delta_{\text{transfer\\\_amount}}, s, \sigma)$$

where $$v_{\text{transfer}}$$ is the transfer amount. The transfer commitment is $$C_{\text{transfer}} = \text{Com}(v_{\text{transfer}}, r_{\text{transfer}})$$. The ephemeral public key $$R_e$$, encrypted amount $$\tilde{v}$$, and $$\sigma$$ are published in the transaction event data so recipients can derive both $$v_{\text{transfer}}$$ and $$r_{\text{transfer}}$$ during replay.

Since $$vk_B \cdot R_e = r_e \cdot \text{PVK}_B = S$$ by ECDH commutativity, both sender and recipient can independently derive $$r_{\text{transfer}}$$ and decrypt $$v_{\text{transfer}} = \tilde{v} - \text{Poseidon}(\delta_{\text{transfer\\\_amount}}, s, \sigma)$$, provided they know $$\sigma$$ emitted with the event. The auditor decrypts the transfer amount via a separate ECDH channel ([Per-Transfer Auditor Ciphertexts](auditing.md#per-transfer-auditor-ciphertexts)).

**Deterministic ephemeral scalar.** $$r_e$$ is derived from the originator's own viewing key and the operation salt. Every operation that has an ephemeral derives it this way: `Transfer`, `Withdraw`, and `SetSpender` from the owner's viewing key, `SpenderTransfer` from the spender's own ([Spender Transfer](operations/spender-transfer.md)). The derivation MUST be re-attempted with a fresh salt in the negligible case that it yields zero.

Because $$\sigma$$ is published in the event and $$vk$$ is held by the originator, the derivation lets the originator recompute $$r_e$$ for any past transfer from the event alone, which is what makes sender-side selective disclosure possible with no per-transfer wallet state ([D-sender](../selective-disclosure/circuits/d-sender.md)). A transfer whose $$r_e$$ was sampled and not retained is permanently undisclosable by its sender. No circuit constrains $$r_e$$ beyond $$R_e = r_e \cdot H$$ and $$r_e \neq 0$$.

#### Why reusing `r_e` is safe

A single $$r_e$$ keys every ECDH channel an operation opens: the recipient's ($$\text{PVK}_B$$), both auditors' ($$K_{\text{aud,r}}$$ and $$K_{\text{aud,s}}$$, [Per-Transfer Auditor Ciphertexts](auditing.md#per-transfer-auditor-ciphertexts)), and, at `set_spender`, the $$dvk_i$$ escrow to the spender ($$Y_{\text{op}}$$, [Delegation Key Escrow](operations/set-spender.md#delegation-key-escrow)). Three properties separate them. *Distinct shared scalars*: those four counterparty points are independent Grumpkin points, none derivable from another. *Distinct domains*: each channel absorbs its own tag ([Domain Separation Constants](domain-separators.md) enumerates them), so masks across channels are independent under the PRF assumption on Poseidon ([Threat Model](system-model.md#threat-model)). *Fresh per-operation nonce*: every channel sponge re-absorbs the operation salt, which [Pad freshness](primitives.md#pad-freshness) requires to be fresh. Together they close the standard ECDH key-reuse attack surface. One derivation stands outside the argument: the auditor-side allowance-blinding escrow (S14, $$\delta_{\text{esc\\\_allow\\\_r\\\_aud}}$$) opens no channel of its own, reusing the sender-auditor shared scalar and absorbing the per-delegation constant $$\text{op}_i$$ rather than a nonce. Only the distinct-domain leg holds there in its own right; its pad freshness is inherited from $$r_e$$, hence from the salt ([Spender Allowance Auditing](auditing.md#spender-allowance-auditing)).

### Anti-Poisoning Constraint

The transfer circuit enforces that $$C_{\text{transfer}}$$ was constructed using the ECDH-derived $$r_{\text{transfer}}$$:

$$C_{\text{transfer}} = v_{\text{transfer}} \cdot G + r_{\text{transfer}} \cdot H \quad \text{where} \quad r_{\text{transfer}} = \text{Poseidon}(\delta_{\text{transfer\\\_blind}}, s, \sigma)$$

This prevents a malicious sender from committing with arbitrary blinding, which would cause the recipient to lose track of their accumulated blinding factor and be unable to spend.

### Encrypted Balance Scalar

Owner-initiated operations (transfers, withdrawals) produce a new spendable balance commitment with deterministic randomness ([Operations](operations/README.md)). To enable wallet recovery without full event replay, the proof also outputs an **encrypted balance scalar**:

$$\tilde{b} = v_{\text{new}} + \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$$

where $$v_{\text{new}}$$ is the new spendable balance and $$\sigma$$ is the prover-chosen random salt. The contract emits $$\tilde{b}$$ in the operation's event ([Event Schema](interface.md#event-schema)) rather than storing it on-chain; the contract never reads it after the proof has bound it to $$C_{\text{spend}}$$. Anyone with $$vk$$ recovers $$v_{\text{new}} = \tilde{b} - \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$$ from the event. The primary consumer is the owner's wallet for checkpoint recovery ([Wallet State and Recovery](wallet-state.md)); auditors do not hold $$vk$$ and instead read balances via per-transfer ECDH ciphertexts ([Per-Transfer Auditor Ciphertexts](auditing.md#per-transfer-auditor-ciphertexts)). The circuit enforces consistency between $$\tilde{b}$$ and the committed value in $$C_{\text{spend}}$$.
