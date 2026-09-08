# Indexing and Off-Chain State Recovery

Companion specification to the protocol's [Wallet State and Recovery](protocol/wallet-state.md). It specifies the durable event archive — the *indexer* — that those sections assume: its data model, ingestion contract, retention obligations, and recommended API surface.

The key words MUST, MUST NOT, SHOULD, and MAY are to be interpreted as in RFC 2119. The normative audience is twofold:

- **Indexer operators** MUST satisfy [Data Model](#data-model) through [Retention Obligations](#retention-obligations) for the deployment to support recovery from seed.
- **Wallets and SDKs** MUST consume an indexer meeting this contract for recovery, and SHOULD apply the client-side verification in [Trust Model and Client-Side Verification](#trust-model-and-client-side-verification).

## Why the Indexer Is Load-Bearing

Confidential balances are Pedersen commitments; the on-chain entry alone does not reveal the opening `(v, r)` needed to spend. A wallet that loses its local cache reconstructs both openings deterministically from the master secret plus the account's event history, by the procedure in [Recovery](protocol/wallet-state.md#recovery): the latest checkpoint supplies the spendable opening, and a replay of the window from `T_0` rebuilds the receiving opening and carries the post-checkpoint folds onto the spendable one.

Stellar RPC retains events for a **7-day window** only. A wallet that loses local state after that window can still see that its funds exist (the commitment remains on-chain) but cannot reconstruct the opening required to spend them — unless a durable archive holds the missing events. That archive is this document's subject. Without a conforming indexer, recovery from seed is not guaranteed, and deployments MUST treat wallet-local state as unrecoverable after the RPC window.

**RPC and the archive compose.** A wallet need not read everything from the archive. Recent history is still live on Stellar RPC, so a client reads the recent tail from RPC directly and only the older portion — everything past the RPC retention floor — from the archive, stitching the two at a *seam* ([Terminology](#terminology)) and deduplicating by event id ([Ordering](#ordering)). The archive's obligation is therefore durable retention of everything older than the RPC window, not keeping pace with the chain head. Reading the whole history from the archive is equally conformant; the split is a client optimization. Either way the guarantee holds only if the archive does not fall so far behind that its ingested-through ledger ([API Surface](#api-surface) C4) drops below the seam — see [Ingestion Contract](#ingestion-contract).

## Terminology

- **Checkpoint** and **`T_0`** — as defined in [Recovery](protocol/wallet-state.md#recovery); [Events in scope](#events-in-scope) lists the qualifying event types in ingestion scope.
- **Replay window** — the range from `T_0` to the current ledger. One window serves both sides ([Why the two anchors compose](protocol/wallet-state.md#why-the-two-anchors-compose)).
- **Event id** — the triple `(ledger_seq, tx_hash, event_index)`, unique per emitted event: `tx_hash` is globally unique, and `event_index` is unambiguous because a Soroban transaction carries a single operation. The same event MUST carry the same id whether served from the archive or from RPC, so a hybrid client can deduplicate across the seam. The id does not by itself encode position within a ledger — `tx_hash` conveys no ordering — so the canonical total order is instead `(ledger_seq, tx_application_order, event_index)`, where `tx_application_order` is persisted as its own field ([Archived record](#archived-record)) and drives [Ordering](#ordering).
- **Seam** — in a hybrid client ([Why the Indexer Is Load-Bearing](#why-the-indexer-is-load-bearing)) that reads the recent tail from Stellar RPC and older history from the archive, the ledger at which it switches sources. A client sets the seam above the RPC retention floor (`getHealth().oldestLedger`) so the RPC side is always served from live retention, and requires the archive's ingested-through ledger ([API Surface](#api-surface) C4) to reach the seam.

## Data Model

### Archived record

For every in-scope event the indexer MUST persist:

| Field | Contents |
|:--|:--|
| `contract_id` | The confidential token contract address. |
| `ledger_seq` | Ledger sequence the event was emitted in. |
| `ledger_close_time` | Close time of that ledger. |
| `tx_hash` | Transaction hash. |
| `tx_application_order` | Position of the transaction within its ledger; supplies the intra-ledger ordering that `tx_hash` does not ([Ordering](#ordering)). |
| `event_index` | Index of the event within the transaction. |
| `topics`, `data` | The event's topics and data payload — verbatim XDR (RECOMMENDED) or a pinned decoded encoding (see note). |

Verbatim XDR is the RECOMMENDED payload: the wallet decodes it directly and, being the on-chain wire form, it survives field renames in the Rust bindings without a schema migration. A decoded representation — for example the JSON a managed indexing pipeline emits — MAY be served instead. Whichever form is served, it MUST reproduce the on-chain event exactly under that decoder. Indexers MAY additionally store decoded columns for querying.

### Events in scope

All events emitted by the confidential token ([Event Schema](protocol/interface.md#event-schema)) with the following recovery roles:

| Event | Role in recovery |
|:--|:--|
| `Register` | Start of the account's history; bounds the worst-case replay window. |
| `Deposit` | Receiving-side replay: accumulates `(amount, 0)` into the receiving opening. |
| `Transfer` (recipient side) | Receiving-side replay: carries the recipient-channel ciphertexts for `(v_transfer, r_transfer)`. |
| `SpenderTransfer` (recipient side) | Receiving-side replay, as above. |
| `Merge` | Anchor: folds the receiving opening into the spendable opening; resets the receiving side. |
| `Clawback` | Anchor, as `Merge`, less the public `amount` on the spendable side; not a checkpoint. Emitted only by deployments that enable the compliance extension ([Wallet and Auditor Consequences](compliance.md#wallet-and-auditor-consequences)). |
| `Withdraw`, `Transfer` (sender side), `SetSpender` | **Checkpoints**: publish `(b_tilde, sigma)` for the owner's spendable balance. `SetSpender` is in scope as an owner checkpoint only — a spender recovers allowance state from the on-chain delegation entry (`allowance_commitment`, `a_tilde`, `escrowed_dvk`, `allowance_salt`), not from the archive. |
| `RevokeSpender` | Spendable-side fold, not a checkpoint: the owner adds the reclaimed allowance opening to the spendable opening, deriving it from the event's `a_tilde` and `allowance_salt` ([Revoke Spender](protocol/operations/revoke-spender.md)). May be emitted by the compliance extension's `force_revoke_spender` without an owner-signed transaction; the fold is the same ([Forced Revocation](compliance.md#forced-revocation)). |

A self-transfer — a `Transfer` whose `from` and `to` are the same account — carries both roles at once: it is a sender-side checkpoint and a recipient-side replay event, and recovery applies both ([Wallet State and Recovery](protocol/wallet-state.md)).

The auditor-channel fields these events carry (`v_tilde_aud_*`, `b_tilde_aud_s`, `r_tilde_aud_s`, `a_tilde_aud_s`, `r_a_tilde_aud_s`) are out of scope for wallet recovery; for the auditor they are the only route to an allowance opening, since the escrowed blinding (`r_a_tilde_aud_s` on `SetSpender`, `r_tilde_aud_s` on `SpenderTransfer`) appears nowhere in contract storage ([Archive dependence](protocol/auditing.md#archive-dependence); [Auditor recovery](#auditor-recovery)). They are archived regardless, since the record is the verbatim event ([Archived record](#archived-record)). Event shapes are normative in [Event Schema](protocol/interface.md#event-schema).

Configuration events (`UnderlyingAssetSet`, `VerifierSet`, `AuditorSet`, `AddressAsFieldSet`, verification-key events) and the compliance events `Frozen`, `Unfrozen`, and `ComplianceConfigChanged` are not needed for balance recovery; indexers SHOULD archive them anyway — they are low-volume and useful for deployment forensics.

### Account attribution

Recovery is per-account, and an event belongs to **each** account address appearing in its topics — a `Transfer` to both the sender's and the recipient's history, a `SpenderTransfer` to the owner's, recipient's, and spender's, a `RevokeSpender` to the owner's and the spender's alone, a `Clawback` to the target's alone. Attribution MUST come from the event topics, never from the transaction source account.

The indexer MAY apply this attribution server-side (per-account queries, [API Surface](#api-surface) C2) or serve the whole per-contract stream and leave the client to select the events touching its account; both conform, since attribution is a pure function of the topics. Server-side per-account filtering is RECOMMENDED for high-volume contracts, where downloading the full contract history to every wallet does not scale.

### Ordering

The indexer MUST preserve and expose the total order `(ledger_seq, tx_application_order, event_index)` — all three components are persisted per [Archived record](#archived-record). Replay correctness depends on it: interleaved deposits, transfers, merges, and revokes only reconstruct the right openings when applied in emission order ([Wallet State and Recovery](protocol/wallet-state.md) step 6).

## Ingestion Contract

- **Source.** Any source that yields the complete, final event stream (Stellar RPC `getEvents`, Horizon, or a captive core). Stellar ledgers are final at close; there is no reorg handling.
- **Freshness.** The archive's ingested-through ledger ([API Surface](#api-surface) C4) MUST stay within the source's retention window — at or above the seam a hybrid client would set ([Terminology](#terminology)). It need not track the chain head: the RPC serves the recent tail ([Why the Indexer Is Load-Bearing](#why-the-indexer-is-load-bearing)). But if it falls below the seam, a gap opens that neither source covers. Detect and backfill any gap while its ledgers are still retrievable from a source; a gap that can no longer be filled is permanent, and affected ranges MUST then be reported incomplete ([API Surface](#api-surface) C3).
- **Idempotency.** Ingestion MUST be at-least-once, deduplicated by event id.
- **Gaps.** The indexer MUST track contiguous ingested ledger ranges. If a gap can no longer be backfilled from any source, the indexer MUST NOT silently serve affected histories as complete (see C3 in [API Surface](#api-surface)).
- **Fidelity.** Events MUST be stored faithfully ([Archived record](#archived-record)) — verbatim XDR or a decoded form pinned to the canonical decoder. Decoding for queries is a read-side concern.

## Retention Obligations

The indexer MUST retain the full per-account history of every in-scope event **indefinitely**. No pruning horizon is safe in general:

- The spendable opening is taken from the account's latest checkpoint, which is arbitrarily old for a dormant account.
- The receiving side is replayed from the last `Merge` or `Clawback` at or before that checkpoint, which for an account that receives but never merges is its registration.
- A `RevokeSpender` after the latest checkpoint is the only surviving record of the addend it folded: the delegation entry carrying `a_tilde` and `allowance_salt` is deleted in the same invocation ([Revoke Spender](protocol/operations/revoke-spender.md)).

## API Surface

C2–C4 below are normative; C1 is RECOMMENDED. The REST shape is RECOMMENDED — any transport exposing the same capabilities conforms.

- **C1 — Latest checkpoint (RECOMMENDED).** Return the most recent checkpoint event for `(contract_id, account)` at or before a given ledger. This is an optimization, not a correctness requirement: each checkpoint carries a self-contained `(b_tilde, sigma)` that re-derives the spendable opening as of that checkpoint, so a client can also obtain the latest checkpoint by scanning the ordered history (C2). Exposing C1 lets a dormant account with a long history skip transferring that history. A client additionally needs `T_0`, the last `Merge` or `Clawback` at or before that checkpoint ([Terminology](#terminology)), which C1 does not return and which is obtainable from C2's ordered history.
- **C2 — Ordered history.** Return all in-scope events for `(contract_id, account)` within a ledger range, in the total order of [Ordering](#ordering), paginated, each carrying its event id and payload ([Archived record](#archived-record)). An indexer that serves only the per-contract stream ([Account attribution](#account-attribution)) satisfies C2 by delivering that stream in order for client-side attribution.
- **C3 — Completeness signal.** Every response MUST state whether the served range is complete (`complete: true` only when the indexer holds a gap-free history for the whole requested range).
- **C4 — Ingestion status.** Expose the latest fully-ingested ledger so clients can bound staleness. A hybrid client ([Why the Indexer Is Load-Bearing](#why-the-indexer-is-load-bearing)) compares it against the seam it derives from the RPC retention floor; if the archive has not ingested through the seam, the client MUST treat the crossing range as incomplete (C3).

Recommended shape:

```text
GET /v1/health
  -> { latest_ledger, ingested_through, lag_seconds }

GET /v1/tokens/{contract_id}/accounts/{account}/checkpoint?at_ledger={n}
  -> { event: { ledger_seq, tx_hash, event_index, topics_xdr, data_xdr },
       complete }

GET /v1/tokens/{contract_id}/accounts/{account}/events
      ?from_ledger={n}&to_ledger={m}&types={csv}&cursor={c}&limit={k}
  -> { events: [ ... archived records ... ], cursor, complete }
```

`types` filters by event name; servers MUST apply it after attribution ([Account attribution](#account-attribution)), never by dropping events from storage. The `/checkpoint` endpoint implements the optional C1 and MAY be omitted. A deployment MAY instead expose history as a single per-contract stream (`GET /v1/tokens/{contract_id}/events`) and leave attribution to the client ([Account attribution](#account-attribution)).

## Trust Model and Client-Side Verification

The indexer is trusted for **availability and completeness only** — never for confidentiality or integrity:

- **Confidentiality.** Everything the indexer holds is public chain data: commitments, masked ciphertexts, and ECDH ephemerals. A curious indexer learns nothing beyond what any chain observer sees ([Security Analysis](protocol/security.md)).
- **Integrity fails closed.** Recovery ends with the wallet checking its reconstructed openings against the **on-chain** commitments (`C_spend =? v·G + r·H`, [Wallet State and Recovery](protocol/wallet-state.md) step 7). A tampered or incomplete history cannot produce a wrong balance that verifies; it produces a detectable mismatch.
- **Withholding is the residual risk.** A malicious or broken indexer can deny recovery (a liveness failure, not a soundness one). Two structural mitigations: for the recent window the RPC is an independent source of the same events (the hybrid split of [Why the Indexer Is Load-Bearing](#why-the-indexer-is-load-bearing)), so archive withholding bites only the pre-window history; and for that older history wallets SHOULD support multiple independent archive endpoints, with deployments running or contracting at least two.

### Auditor recovery

An auditor's allowance tracking is strictly event-scoped and has no state-based fallback. The opening of a delegation's `C_a` is escrowed only in the event that wrote it ([Spender Allowance Auditing](protocol/auditing.md#spender-allowance-auditing)), so a missed, reordered, or unarchived `SetSpender` / `SpenderTransfer` leaves the auditor without that opening ([Decryption capability across rotation](protocol/auditing.md#decryption-capability-across-rotation)). An auditor client SHOULD verify each reconstructed allowance opening against the stored `allowance_commitment` — `C_a =? v_a·G + r_a·H` — which is the same fails-closed check [Trust Model and Client-Side Verification](#trust-model-and-client-side-verification) states for wallets. A mismatch is evidence of a missed, reordered, or pruned event rather than of a wrong balance. `live_until_ledger` governs spending authority and is independent of the delegation entry's persistent-entry TTL.

## Conformance and Versioning

An implementation conforms to this specification iff it satisfies [Data Model](#data-model) through [Retention Obligations](#retention-obligations) and exposes the normative capabilities C2, C3, and C4 (C1 is RECOMMENDED). This document is versioned with the protocol documentation set; breaking changes to the archived record shape or the normative capabilities bump the protocol documentation version and MUST be called out in release notes.

---

Up: [Index](README.md#companions)
