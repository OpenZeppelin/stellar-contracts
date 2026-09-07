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
- Confidential Token: [Indexing and Off-Chain State Recovery](./INDEXER.md)
- Confidential Token: [SDK](./SDK.md)

---

## Introduction

### Background

Confidential transfers on blockchain require balances and amounts to be hidden from public observers while remaining verifiable by the contract. The standard approach uses additively homomorphic encryption - the contract operates on ciphertexts (adding deposits, subtracting transfers) without learning the underlying values, and zero-knowledge proofs guarantee that operations are valid (sufficient funds, consistent encryption, non-negative balances).

This document defines a Soroban contracts suite that provides confidential balances and transfers features on top of SEP-41 tokens. It is not an extension to the fungible token standard; it defines a separate contract that holds tokens on behalf of users and manages encrypted state independently. The wrapping approach is chosen over native token integration for three reasons: it works with existing assets, it can evolve independently of the token standard, and it keeps confidentiality complexity separate from the underlying token.

### Design Goals

**Amount and balance confidentiality.** An observer can see that account $$A$$ transferred to account $$B$$, and how much each party deposited or withdrew, but not how much moved between them or what their balances are. The system provides confidentiality, not anonymity - sender and recipient addresses remain visible on-chain.

**Griefing resistance.** A third party must not be able to prevent an account owner from spending by spamming transfers, so the balance model must isolate incoming funds from the state that spend proofs reference.

**No mandatory maintenance operations.** Receiving funds should not require the owner to perform a costly ZK proof before those funds become accessible. The merge operation that makes received funds spendable must be lightweight and non-frontrunnable.

**Selective auditing.** Each account selects an auditor at registration. Each transfer produces ciphertexts under two auditor keys: the recipient's auditor receives the transfer amount, while the sender's auditor receives the transfer amount and sender's post-transfer balance. This dual-auditor model enables real-time visibility for both parties' auditors without granting access to uninvolved accounts' historical balances.

**Delegated spending.** Account owners can authorize spenders (separate addresses) to spend from escrowed allowances, enabling use cases like automated market makers and custodial services without sharing the owner's secret key.

### Approach

The design is built on three interlocking mechanisms:

1. **Pedersen commitments.** Each balance is a single elliptic curve point $$C = v \cdot G + r \cdot H$$. There is no chunking, no discrete logarithm to solve for decryption, and no overflow from repeated homomorphic additions. The owner maintains the commitment opening $$(v, r)$$ as local wallet state, updated incrementally from on-chain events.

2. **ECDH-derived blinding.** When a sender transfers to a recipient, the blinding factor of the transfer commitment is derived from an ephemeral Diffie-Hellman key exchange with the recipient's public viewing key. The circuit enforces correct derivation, ensuring the recipient can always compute the blinding. The same ephemeral scalar is reused for an ECDH exchange with auditors' public key.

3. **Proof-less merge.** Incoming funds accumulate in a receiving balance that is separate from the spendable balance. To make received funds spendable, the owner authorizes a merge - no ZK proof is required. Since merge requires owner authorization and incoming transfers touch only the receiving balance, neither the spend path nor the merge path can be front-run by a third party.

Five Noir/UltraHonk circuits cover registration, withdrawal, confidential transfer, spender transfer, and spender delegation. The proof system leverages the Grumpkin–BN254 curve cycle: Grumpkin point arithmetic is native inside Noir circuits (no field emulation), while Soroban natively supports BN254 operations for UltraHonk proof verification.
