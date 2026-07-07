# Confidential Token: User Flows Overview

## What It Is

The Confidential Token is a Soroban smart contract that adds **private balances and transfers** to any SEP-41 token. Users deposit regular tokens into the contract and, from that point on, balances and transfer amounts are hidden from the public while remaining fully verifiable by the blockchain.

**What is hidden:** how much someone holds and how much moves between two parties.

**What stays visible:** the sender address, the recipient address, and any deposit/withdrawal amounts that cross the boundary between the confidential token contract and the underlying non-confidential token.

The system provides **confidentiality**, not anonymity.

---

## Why a Separate Contract

- **Works with any SEP-41 token**, including XLM via its Stellar Asset Contract. No issuer changes required. (The underlying token must be non-rebasing and free of transfer fees; see `DESIGN.md` §3.4.)
- **Evolves independently** of the token standard - upgrades to the privacy layer do not touch the underlying asset.
- **Clean separation** - the token keeps doing what it does today; confidentiality is an opt-in layer on top.

---

## Core Concepts

| Concept | What it means |
|:---|:---|
| **Spendable balance** | The portion of an account's funds available for sending or withdrawing. Only the account owner can modify it. |
| **Receiving balance** | A separate accumulator where incoming deposits and transfers land. |
| **Merge** | An owner-authorized action that folds the receiving balance into the spendable balance via on-chain point addition. No zero-knowledge proof is required. |
| **Zero-knowledge proof** | A cryptographic artifact attached to a transaction that proves the operation is valid (sufficient funds, correct balance updates, honest encryption) without revealing any amounts. Generated client-side by the sender's wallet. |
| **Spender** | A separate address explicitly authorized by an account owner to spend from a capped, time-limited allowance — enabling automated services, custodians, or DeFi integrations without sharing the owner's spending key. |
| **Wallet** | A client-side tooling responsible for key management, proof generation, encryption/decryption, and local state tracking. In most cases this is a client library embedded directly inside the application the account holder is using. |

---

## Main User Flows

### 1. Account Setup (Registration)

| Step | Who | What happens |
|:-----|:----|:-------------|
| 1 | Account holder | Initiates confidential account registration for a given token via the wallet, selecting an auditor. |
| 2 | Wallet | Generates a spending key (authorizes transactions) and a viewing key (reads balances). Both derive from a single secret. |
| 3 | Wallet | Produces a zero-knowledge proof that all keys are correctly derived and linked. |
| 4 | Contract | Verifies the proof and stores the account's public keys and chosen `auditor_id`. The account initializes with zero spendable and receiving balances. |

**Key point for compliance:** the account's `auditor_id` is bound at registration time, determining which auditor receives per-transfer encrypted ciphertexts for this account's activity.

---

### 2. Deposit (Public to Confidential)

| Step | Who | What happens |
|:-----|:----|:-------------|
| 1 | Depositor | Calls the contract with a regular token amount (e.g., 500 USDC). This amount is publicly visible. |
| 2 | Contract | Transfers the tokens from the depositor into the contract. |
| 3 | Contract | Adds the deposit to the recipient's **receiving balance** using homomorphic math; no proof or action needed from the recipient. |

Anyone can deposit into any registered account; the depositor itself does not need a confidential account, but the recipient must be registered.

---

### 3. Merge (Making Received Funds Spendable)

| Step | Who | What happens |
|:-----|:----|:-------------|
| 1 | Account holder | Authorizes a merge via the wallet. |
| 2 | Contract | Adds the receiving balance commitment to the spendable balance commitment (homomorphic point addition) and resets the receiving balance to the identity point. No proof required. |

Merge is the gate between received funds and spendable funds. It is deliberately lightweight (a single on-chain point addition) so account holders are never blocked by malicious senders who spam the account with incoming transfers and prevent them from constructing a valid proof.

---

### 4. Confidential Transfer

| Step | Who | What happens |
|:-----|:----|:-------------|
| 1 | Sender | Specifies the recipient address and amount in the wallet. |
| 2 | Wallet | Generates a zero-knowledge proof covering: balance sufficiency, correct computation of the new sender commitment, ECDH-derived blinding for the transfer commitment (so the recipient can decrypt), dual-auditor encrypted ciphertexts, and range validity of all values. |
| 3 | Wallet | Encrypts the transfer amount under an ephemeral shared secret with the recipient's public viewing key. Also produces encrypted ciphertexts for both auditors: the recipient's auditor receives the transfer amount, and the sender's auditor receives the transfer amount plus the sender's post-transfer balance. |
| 4 | Wallet | Submits the transaction containing the proof, the new sender commitment, the transfer commitment, the ephemeral public key, the salt, the encrypted amount, the encrypted balance scalar (for owner recovery), and the auditor ciphertexts. |
| 5 | Contract | Verifies the proof, replaces the sender's spendable balance commitment, and adds the transfer commitment to the recipient's receiving balance via homomorphic addition. Emits an event carrying the ephemeral public key, the salt, the encrypted amount, the encrypted balance scalar, and the auditor ciphertexts. |
| 6 | Recipient wallet | Observes the event, performs ECDH with the ephemeral public key to recover the shared secret, decrypts the transfer amount, derives the blinding factor, and updates its local receiving-balance accumulator. |

**On-chain, an observer sees:** that address A transacted with address B. The transfer amount and both parties' balances remain hidden.

---

### 5. Withdrawal (Confidential to Public)

| Step | Who | What happens |
|:-----|:----|:-------------|
| 1 | Account holder | Specifies the withdrawal amount in the wallet. This amount will be publicly visible on-chain once the transaction executes. |
| 2 | Wallet | Generates a zero-knowledge proof demonstrating balance sufficiency, correct construction of the new spendable balance commitment with deterministic randomness, and a sender-auditor encrypted balance checkpoint produced via ephemeral ECDH with the sender's auditor key. |
| 3 | Contract | Verifies the proof, replaces the spendable balance commitment, and transfers the corresponding amount of regular tokens from the contract back to the account holder. Emits an event carrying the ephemeral public key, the salt, the encrypted balance scalar (for owner recovery), and the sender-auditor balance ciphertext. |

---

### 6. Delegated Spending (Spenders)

Spenders enable use cases like automated trading bots, payment processors, or custodial services.

| Step | Who | What happens |
|:-----|:----|:-------------|
| **Setup** | | |
| 1 | Owner | Specifies the spender address (which must already be a registered account in the contract, so its spending public key can be looked up for delegation key escrow), the allowance amount, and a `live_until_ledger` expiration. |
| 2 | Wallet | Generates a proof that the allowance is correctly carved out of the owner's spendable balance. The proof also covers derivation and ECDH escrow of a delegation viewing key (`dvk`) so the spender can independently track and decrypt its allowance state, and produces ciphertexts for the owner's auditor (escrow amount and post-operation balance checkpoint). |
| 3 | Contract | Verifies the proof, deducts the allowance from the owner's spendable balance commitment, and stores the spender delegation (allowance commitment, encrypted allowance, escrowed `dvk`, allowance salt, expiration). Emits an event with the owner's post-operation balance checkpoint and the owner-auditor ciphertexts. |
| **Operation** | | |
| 4 | Spender | Initiates a confidential transfer from the escrowed allowance to any registered recipient. A proof accompanies each transfer, covering allowance sufficiency, ECDH-derived encryption for the recipient, and dual-auditor ciphertexts for the recipient's and owner's auditors. |
| 5 | Contract | Verifies the proof, updates the allowance commitment, and adds the transfer commitment to the recipient's receiving balance. The owner's spendable balance is not involved. Emits an event with the ephemeral public key, the salt, and the auditor ciphertexts. |
| **Revocation** | | |
| 6 | Owner | Revokes the delegation at any time via a proof. The remaining escrowed allowance is folded back into the owner's spendable balance, and the proof produces ciphertexts for the owner's auditor (reclaimed amount and post-revocation balance checkpoint). The contract emits an event carrying these ciphertexts alongside the owner's balance checkpoint. |

Spenders never access the owner's spendable balance directly. Exposure from a compromised or malicious spender is bounded by the granted allowance amount.

---

## Compliance and Auditing

### How Auditing Works

The system supports **real-time auditing** via a dual-auditor model. Each account selects an auditor at registration. Every confidential transfer produces encrypted ciphertexts for both the sender's and recipient's auditors. Withdrawals, spender setup, and spender revocation also produce ciphertexts for the sender's (or owner's) auditor. All ciphertexts are enforced by the zero-knowledge proof of each operation.

| What | Recipient's auditor sees | Sender's auditor sees |
|:-----|:------------------------|:---------------------|
| Transfer amount | Yes | Yes |
| Per-transfer Pedersen randomness $r_{\text{tx}}$ | Yes (enables Pedersen-opening reconstruction of the recipient's receiving balance) | No |
| Sender's post-transfer balance | No | Yes |
| Withdrawal amount | n/a | Yes (publicly visible) |
| Post-withdrawal balance | n/a | Yes |
| Spender escrow / reclaim amount | n/a | Yes (owner's auditor) |
| Post-transfer spender allowance | No (for spender transfers) | Yes (owner's auditor) |

Each auditor decrypts its ciphertexts by running the channel sponge (recipient-auditor channel for recipients, sender-auditor channel for senders/owners) with its private key, the ephemeral public key, and the per-operation salt published in the operation's event.

### Compliance Properties

- **Per-account auditor selection.** Each account selects an auditor at registration. The `auditor_id` is immutable and determines which auditor receives ciphertexts for the account's activity.
- **Dual-auditor ciphertexts.** The ciphertexts each operation produces are enforced by its zero-knowledge proof, so they cannot be omitted or malformed, and no extra action is needed from users.
- **Per-account scope.** Auditing one account reveals nothing about any other account.
- **Recipient-side opening capability.** The recipient's auditor holds the per-transfer Pedersen blinding $r_{\text{tx}}$ on every inbound transfer and spender-transfer. Combined with the transfer amount this is the full Pedersen opening of every received transfer commitment, and by summation of the recipient's receiving-balance commitment between merges. The capability is forward-only (only events while the auditor key was active are decryptable), receiving-side only (it does not extend to the recipient's spendable balance after merge), and reset by merge. This is what enables the seizure/clawback flow specified in `COMPLIANCE.md` §5 without an on-chain accumulator or per-transfer contract hook. The sender's auditor remains restricted to amounts and balance checkpoints; it does not see openings.
- **Seamless auditor rotation.** When an auditor key is rotated, the new key immediately receives ciphertexts on subsequent operations. For the sender's auditor, the balance checkpoint at the next owner-initiated proof operation (transfer, withdrawal, set spender, or revoke spender) provides the current balance with no event replay or bootstrapping.
- **Spender visibility.** The owner's auditor sees spender transfer amounts and post-transfer allowances via the same dual-auditor mechanism, and additionally sees escrowed and reclaimed amounts at `set_spender` and `revoke_spender`.
- **Viewing vs. spending separation.** Even with the viewing key, an auditor or anyone who obtains it **cannot move or spend funds**. Spending requires the separate spending key, which is never shared.

### Auditor Configuration

The auditor is a **separate contract**, independent of the token contract. This means:

- A single auditor contract can serve multiple wrapped tokens.
- Auditor keys can be managed (rotated, added) without redeploying the token contract.
- Different tokens can point to different auditors, allowing jurisdiction-specific configurations.

---

## Required Tooling

### For Users (Wallet)

The wallet abstracts all cryptographic operations. Account holders interact with standard actions (send, receive, merge, withdraw) without direct exposure to keys, proofs, or commitments.

The wallet must:

- **Generate and store keys** - derive the full key hierarchy (spending key, viewing key, public viewing key, delegation viewing keys) from a single master secret.
- **Produce zero-knowledge proofs** - the heaviest client-side computation. Proof generation time depends on the circuit complexity but targets single-digit seconds on modern hardware. The Transfer circuit involves approximately 7 elliptic-curve scalar multiplications (including two auditor ECDH exchanges); the Register circuit is lighter.
- **Track local state** - maintain running commitment openings (value and blinding factor pairs) for the spendable and receiving balances by processing on-chain events. This is comparable to wallet sync in UTXO-based privacy systems (Zcash, Monero).
- **Handle recovery** - if local state is lost, reconstruct balances from on-chain data using the viewing key: fetch the encrypted balance scalar and salt from the most recent spend-boundary event, derive the deterministic blinding factor, and replay subsequent incoming transfer events. Recovery requires the master secret plus access to a durable event archive (Stellar RPC retains only 7 days of history); the indexer this archive must satisfy is specified in the companion Indexing and Off-Chain State Recovery document (`INDEXER.md`).

### For Developers (Integration)

| Component | Description |
|:----------|:------------|
| **Token contract** | The main Soroban contract that holds SEP-41 tokens and manages encrypted account state. |
| **Verifier contract** | Validates zero-knowledge proofs on-chain. Stores one verification key per operation type. |
| **Auditor contract** | Manages auditor public keys. Shared across tokens. |
| **Noir circuits** | Six proof circuits (register, withdraw, transfer, spender transfer, set spender, revoke spender). Written in Noir, compiled to UltraHonk. |
| **Client library** | SDK for wallets: key management, proof generation, event processing, balance tracking, encryption/decryption. |

---

## UX Considerations

### What Account Holders Experience

The goal is that confidential transfers are perceived similarly to regular transfers, with two notable differences:

1. **Proof generation latency** when sending or withdrawing. The wallet generates a zero-knowledge proof on the local device before submitting the transaction. Expected duration is a few seconds, depending on hardware.
2. **The merge step** before spending received funds. Received deposits and transfers accumulate in the receiving balance and must be merged into the spendable balance before they can be spent. Wallets should encourage this and prompt a merge transaction ahead of any spend, making it almost automatic. The merge itself is inexpensive (no proof, just a contract call).

### What the Wallet Abstracts

- Cryptographic keys, Pedersen commitments, and zero-knowledge proofs.
- Blinding factors, salts, and encrypted scalars.
- Auditor ciphertexts - generated automatically by every operation that requires them. The account holder selects an auditor once at registration; no further interaction is needed.

### Wallet Recovery

If the wallet is lost or reinstalled on a new device:

1. The account holder restores from the master secret (seed phrase or equivalent).
2. The wallet re-derives the full key hierarchy.
3. The wallet fetches the latest spend-boundary event for the account, reads the encrypted balance scalar and salt from it, recovers the spendable balance opening using the viewing key, then replays subsequent incoming transfer events to reconstruct the receiving balance.

The recovery process is fully deterministic given the master secret and access to the account's full event history since the last spend boundary. Because Stellar RPC retains only the last 7 days of events, recovery from seed alone depends on a durable indexer (`INDEXER.md`) that retains the per-account event log; without one, the on-chain commitments remain visible but their openings cannot be reconstructed.

### Edge Cases the Wallet Handles

- **Spam resistance.** Incoming transfers cannot block or delay spending. They modify only the receiving balance; spend proofs reference only the spendable balance, so in-flight proofs remain valid regardless of incoming activity.
- **Failed transactions.** If a transaction reverts, the wallet uses a fresh random salt on retry, producing different deterministic randomness. This prevents an observer who saw the reverted transaction from correlating the retried commitment. The salt is a public input so the auditor can still reconstruct state.
- **Spender expiry.** Delegations carry a `live_until_ledger` after which spender transfers are rejected. The wallet should surface upcoming expirations and facilitate renewal or revocation. Expired delegations persist in storage until explicitly revoked by the owner, since automatic cleanup would destroy escrowed funds.
