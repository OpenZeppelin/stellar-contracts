# Confidential Token: RWA Integrator Guide

Non-normative. This guide shows how issuers, wallets, compliance services and venues use the RWA profile specified in [RWA Profile](compliance.md#rwa-profile). The reference deployment is [`examples/confidential/rwa-token`](../../../examples/confidential/rwa-token/src/contract.rs).

## What the profile is

A confidential token for a regulated asset has to answer two kinds of rule. Rules about who the parties are (KYC, jurisdiction, freeze) concern public addresses, and the contract checks them. Rules about how much a party holds or moves (holding caps, volume limits, lockups, holder counts) concern hidden values, and no party to a transfer can prove them about the other side. The profile gives the second kind to the issuer's compliance service, which holds the designated auditor key and therefore sees every amount:

| Tier | Balance-dependent rules | Online party for a transfer |
|:---|:---|:---|
| 0, monitor and remediate (`approver: None`) | Detected after settlement; remedied with `freeze` and `forced_transfer` | None |
| 1, approval gate (`approver: Some`) | Enforced before settlement by the approver | The issuer's approval service, for every value-moving operation |

Tier 0 fits rules whose breach can be cured afterwards. A holder can pass an excess on to another eligible holder, or withdraw it where the public side is open, before the breach is detected. Tier 1 fits rules that must hold at every instant, at the price of an online dependency and of trusting the approver for the correctness of those rules. In both tiers the proofs still guarantee that nobody, the issuer included, can create value, and that a holder's balance moves only by the holder's own proof or by an administrative override, which needs a role holder, the auditor's proof and a frozen account ([Overrides and recovery](#overrides-and-recovery)).

## Issuer

**Base asset.** The confidential token holds an issuer-controlled SEP-41 asset. Two variants:

- *Closed public side.* Only the treasury and the confidential token hold the base asset. Eligibility comes from a policy contract ([Policy Contract](compliance.md#policy-contract)), typically an adapter over the identity registry; `sac_passthrough` is off.
- *Open public side.* Investors are also authorized holders of the base asset, a Stellar Asset Contract with `AUTH_REQUIRED`. `sac_passthrough` is on, so the issuer's authorization list gates the confidential layer too ([SAC Authorization Passthrough](compliance.md#sac-authorization-passthrough)). Public transfers of the base asset do not pass the approval gate, so balance and volume rules cover public holdings only by monitoring.

A token whose own compliance modules depend on balances (for example a SEP-57 token with a maximum-balance module) is not a suitable base asset: its modules would see the pool as one investor.

**Deployment.** Wire `type Hooks = RwaHooks`, and set both configurations in the constructor; until both exist every operation reverts. Give the `freezer`, `config_manager`, `clawback` and `forced_transfer` duties to different signers ([Admin Authority](compliance.md#admin-authority)). Keep the auditor decryption key, the approver signing key and the admin roles in separate custody: the approver's mistakes should be visible to someone who does not hold its key. The approver is a dedicated address: it cannot be the sender, recipient or spender of an operation it approves, so the treasury cannot be its own approver.

**Issuance.** There is no confidential mint. Mint base units to the treasury, `deposit(treasury, treasury, amount)`, `merge`, then `confidential_transfer` to each investor. The mint and deposit amounts are public; the per-investor amounts are not. Funding the treasury in bulk keeps individual subscriptions from being matched to deposits. Redemption is the reverse: a confidential transfer to the treasury, then `withdraw` and burn.

**Identity changes.** The identity registry cannot see confidential balances. Do not remove or reassign an identity while its accounts hold confidential value.

**Adopting the profile on a live token.** Accounts registered under another auditor cannot send, receive or delegate once `RwaHooks` is active ([Designated Auditor](compliance.md#designated-auditor)). Drain them first.

## Wallet and custodian

| Step | Tier 0 | Tier 1 |
|:---|:---|:---|
| Onboard | Pass the issuer's KYC | Same |
| Register | `register` with the designated `auditor_id` from `rwa_config()` | Same |
| Preflight | `can_send(from)`, `can_receive(to)` | Same; `rwa_config().approver` tells the wallet an approval is needed |
| Send | Build the proof, sign, submit | Build the proof, sign a detached authorization entry, send both to the issuer's approval service |
| Receive | Ingest the event; no signature or acceptance | Same |

`can_send` and `can_receive` report public eligibility only. They cannot promise that an operation will settle: that also needs a valid proof and, in Tier 1, an approval.

### Tier 1 submission

*Service submits (default).*

1. The wallet builds the invocation and its proof, and signs its own authorization entry as a detached address credential with a short expiry.
2. It sends the invocation and that entry to the approval service.
3. The service decrypts the amount from the recipient-auditor ciphertexts, checks it against `c_transfer`, evaluates its rules and reserves capacity.
4. The approver signer signs the approver's entry over the arguments of [Approval Gate](compliance.md#approval-gate), and the service simulates the complete transaction with authorization enforced. This simulation is what checks the sender's signature and the proof.
5. The service submits the transaction with its own source account and returns the hash.

*Wallet submits.* As above, except that the service returns the signed approver entry and the wallet attaches it, signs the envelope and submits. Routers and settlement venues need this form.

A request the service cannot decide at once stays pending and holds its reservation. The proof behind it stays valid until the sender spends, merges or changes a delegation, the auditor key rotates, the sender's entry expires, or the approval epoch advances. After any of those the wallet builds a new proof, which is a new request.

Each refusal tells the sender one bit about the rule that refused it. Services should authenticate requests, prefer submitting themselves so that an approval is not handed out unused, and return generic reasons.

## Approval service

The service is the approver's signing component plus the auditor's ledger. Requirements for a correct one:

- **Ledger.** Per identity: spendable, receiving and escrowed allowance values over all its accounts, plus public base-asset balances where the public side is open. `ForcedTransfer` and `Clawback` events adjust it.
- **Reservations.** Serialize decisions per identity and per global limit. Confirm a reservation only after the enforcing simulation passes. Release it on observed settlement or once the approval's expiry ledger has closed, never on a submission error alone. Credit outgoing value back only when it is observed on-chain.
- **Freshness.** Refuse while the indexer is behind. Evaluate time-dependent rules for every ledger in the approval's validity window.
- **Issue only detached entries.** A contract approver that invokes the token itself is not bound by the approval epoch ([Approval Gate](compliance.md#approval-gate)). A smart-account approver should forbid generic execution calls that target the token.
- **Keys.** Retain every historical key of the designated auditor id; a rotated key cannot open earlier ciphertexts ([Auditor Key Management and Rotation](protocol/auditing.md#auditor-key-management-and-rotation)).

While the service is down, value-moving operations on a Tier 1 token stop. The `config_manager` can replace the approver or disable the gate; both void outstanding approvals.

## Overrides and recovery

`forced_transfer` and `clawback` need three things: a role holder, a proof from the auditor the source account is bound to, and a frozen source. They are not subject to approval. [Forced Transfer](compliance.md#forced-transfer) gives the recovery procedure for a lost account and the restriction on using either entry point before the verifier accepts zero-knowledge proofs; until then a public-network deployment has freeze as its only remedy.

## Venues and contracts

- **Atomic settlement.** A router contract can call two `confidential_transfer` legs in one transaction. Each party proves its own leg and checks the counterparty's leg off-chain from the ciphertext addressed to it. A leg is never invalidated by activity on the recipient's account; it is invalidated by the sender's own outgoing activity, a key rotation or an epoch change, so a venue needs timeouts.
- **Binding legs together.** Approver entries are rooted at the token and do not bind each other. A router that needs inseparable legs requires each trader's and each approver's authorization over the settlement terms itself; the token-level authorizations then sit under that entry.
- **Contract-owned accounts.** A contract can own a confidential account and authorize spends as the caller, while an off-chain agent holds the spending key and builds proofs. The agent cannot move funds without the contract; it can withhold service and sees that account's balance. This serves escrow, settlement custody and collateral lockboxes with amounts agreed off-chain. A contract cannot read or compare a hidden balance, so lending, vaults and automated market makers need a different design.

## Standards mapping

The base token's mapping to ERC-7984 is unchanged by the profile. The profile adds the ERC-8424 surface where a proof-based token can provide it:

| Standard element | Here | Status |
|:---|:---|:---|
| ERC-8424 `canSend`, `canReceive` | `can_send`, `can_receive` | Adopted: plain eligibility, no quantitative rule, `false` instead of a contract error |
| ERC-8424 `forceConfidentialTransferFrom` | `forced_transfer` | Adapted: ignores the source's eligibility, requires the destination's, reaches frozen funds. The amount is public, the source must be frozen, an insufficient balance reverts, and only `ForcedTransfer` is emitted |
| ERC-8424 `confidentialCanTransfer` | Tier 1: a dry run at the approval service. Tier 0: none | Not on-chain: the contract cannot compute a hidden boolean |
| ERC-8424 `confidentialAvailableBalanceOf` | None | Not carried over; the profile has whole-account freeze only |
| ERC-8424 transfer of zero on failure | Revert | The standard permits reverting; in Tier 1 the refusal reaches the wallet before any transaction exists |
| ERC-3643 `isVerified`, SEP-57 `verify_identity` | `Policy::is_authorized` over the identity registry | Through a policy adapter |
| ERC-3643 `setAddressFrozen` | `freeze`, `unfreeze` | Whole-address freezing |
| ERC-3643 `freezePartialTokens` | None | Not available on hidden balances |
| ERC-3643 `forcedTransfer`, `recoveryAddress` | `forced_transfer` | The movement only; relinking the identity and carrying over restrictions are the issuer's procedure |
| ERC-3643 compliance modules | Identity modules behind the policy; balance and volume modules in the approval service | Split by what the chain can see |
| SEP-8 approval server | Approval service issuing Soroban authorization entries | Same role, different mechanism |

ERC-7984 and ERC-8424 are technology-agnostic. The differences above come from this implementation: it matches the names and the public-eligibility functions, and it cannot provide behaviours that need a party computing on hidden values. Describe the token as aligned with those standards, not as conforming to them.

## Trade-offs

- Tier 1 trusts the approver for balance and volume rules, and makes it an online dependency.
- Tier 0 detects a breach after it has settled.
- `forced_transfer` and recovery reveal the amount.
- With an open public side, public transfers bypass the approval gate.
- Wallets and custodians carry the base token's burden unchanged (proving, key derivation, event ingestion), plus the approval exchange in Tier 1.

---

Up: [Index](README.md#companions)
