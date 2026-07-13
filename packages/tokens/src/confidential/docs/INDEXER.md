# Indexing and Off-Chain State Recovery

Companion specification to [DESIGN.md](./DESIGN.md) §5.2 (Wallet State and
Recovery) and [DESIGN_cont.md](./DESIGN_cont.md) §9.5 (State Recovery). It
specifies the durable event archive — the *indexer* — that those sections
assume: its data model, ingestion contract, retention obligations, and
recommended API surface.

The key words MUST, MUST NOT, SHOULD, and MAY are to be interpreted as in
RFC 2119. The normative audience is twofold:

- **Indexer operators** MUST satisfy §3–§5 for the deployment to support
  recovery from seed.
- **Wallets and SDKs** MUST consume an indexer meeting this contract for
  recovery, and SHOULD apply the client-side verification in §7.

## 1. Why the Indexer Is Load-Bearing

Confidential balances are Pedersen commitments; the on-chain entry alone
does not reveal the opening `(v, r)` needed to spend. A wallet that loses
its local cache reconstructs the opening deterministically from the master
secret plus the account's event history: the latest *checkpoint* event
supplies `(b_tilde, sigma)` from which the spendable opening is derived,
and replaying subsequent events rebuilds the receiving-side state
(DESIGN_cont §9.5).

Stellar RPC retains events for a **7-day window** only. A wallet that
loses local state after that window can still see that its funds exist
(the commitment remains on-chain) but cannot reconstruct the opening
required to spend them — unless a durable archive holds the missing
events. That archive is this document's subject. Without a conforming
indexer, recovery from seed is not guaranteed, and deployments MUST treat
wallet-local state as unrecoverable after the RPC window.

## 2. Terminology

- **Checkpoint** — an owner-initiated proof-carrying event (`Withdraw`,
  sender-side `Transfer`, `SetSpender`, `RevokeSpender`) that publishes
  `(b_tilde, sigma)` for the owner's spendable balance (DESIGN_cont §9.5).
- **Replay window** — for the spendable side, the range from an account's
  latest checkpoint to the current ledger; for the receiving side, the
  range from the last `Merge` (or registration, if never merged) to the
  current ledger.
- **Event id** — the triple `(ledger_seq, tx_hash, event_index)`, unique
  per emitted event and totally ordered by
  `(ledger_seq, tx_application_order, event_index)`.

## 3. Data Model

### 3.1 Archived record

For every in-scope event the indexer MUST persist:

| Field | Contents |
|:--|:--|
| `contract_id` | The confidential token contract address. |
| `ledger_seq` | Ledger sequence the event was emitted in. |
| `ledger_close_time` | Close time of that ledger. |
| `tx_hash` | Transaction hash. |
| `event_index` | Index of the event within the transaction. |
| `topics_xdr`, `data_xdr` | The event's topics and data, **verbatim XDR**. |

Verbatim XDR is the canonical payload: it is what the wallet decodes and
what survives field renames in the Rust bindings. Indexers MAY additionally
store decoded columns for querying, but the XDR is authoritative and MUST
be returned byte-for-byte.

### 3.2 Events in scope

All events emitted by the confidential token (DESIGN §12) with the
following recovery roles:

| Event | Role in recovery |
|:--|:--|
| `Register` | Start of the account's history; bounds the worst-case replay window. |
| `Deposit` | Receiving-side replay: accumulates `(amount, 0)` into the receiving opening. |
| `Transfer` (recipient side) | Receiving-side replay: carries the recipient-channel ciphertexts for `(v_tx, r_tx)`. |
| `SpenderTransfer` (recipient side) | Receiving-side replay, as above. |
| `Merge` | Folds the receiving opening into the spendable opening; resets the receiving side. |
| `Withdraw`, `Transfer` (sender side), `SetSpender`, `RevokeSpender` | **Checkpoints**: publish `(b_tilde, sigma)` for the spendable balance. `SetSpender`/`SpenderTransfer`/`RevokeSpender` additionally carry the allowance-channel ciphertexts a spender wallet needs. |

Configuration events (`UnderlyingAssetSet`, `VerifierSet`, `AuditorSet`,
`AddressAsFieldSet`, verification-key events) are not needed for balance
recovery; indexers SHOULD archive them anyway — they are low-volume and
useful for deployment forensics.

### 3.3 Account attribution

Recovery queries are per-account. The indexer MUST index every event under
**each** account address appearing in its topics — a `Transfer` belongs to
both the sender's and the recipient's history, a `SpenderTransfer` to the
owner's, recipient's, and spender's. Attribution MUST come from the event
topics, not from transaction source accounts.

### 3.4 Ordering

The indexer MUST preserve and expose the total order
`(ledger_seq, tx_application_order, event_index)`. Replay correctness
depends on it: interleaved deposits, transfers, and merges only reconstruct
the right openings when applied in emission order (DESIGN §5.2 step 5).

## 4. Ingestion Contract

- **Source.** Any source that yields the complete, final event stream
  (Stellar RPC `getEvents`, Horizon, or a captive core). Stellar ledgers
  are final at close; there is no reorg handling.
- **Deadline.** Ingestion MUST complete before events age out of the
  source's retention window. With the standard 7-day RPC window, an
  indexer SHOULD run with a lag of minutes and MUST detect and backfill
  any gap while the gap's ledgers are still retrievable.
- **Idempotency.** Ingestion MUST be at-least-once, deduplicated by event
  id.
- **Gaps.** The indexer MUST track contiguous ingested ledger ranges. If a
  gap can no longer be backfilled from any source, the indexer MUST NOT
  silently serve affected histories as complete (see `complete` in §5).
- **Fidelity.** Events MUST be stored verbatim (§3.1). Decoding is a read
  side concern.

## 5. Retention Obligations

The indexer MUST retain the full per-account history of every in-scope
event **indefinitely**. No pruning horizon is safe in general:

- The spendable-side window reaches back to the account's latest
  checkpoint, which is arbitrarily old for a dormant account.
- The receiving-side window reaches back to the last `Merge`, which for an
  account that receives but never spends is its registration.

Incoming-transfer spam makes per-account storage linear in the number of
inbound events; operators SHOULD provision for this and MAY rate-limit
*serving* (never retention) per DESIGN_cont §9.5.

## 6. API Surface

The capabilities below are normative; the REST shape is RECOMMENDED — any
transport exposing the same capabilities conforms.

Normative capabilities:

- **C1 — Latest checkpoint.** Return the most recent checkpoint event for
  `(contract_id, account)` at or before a given ledger.
- **C2 — Ordered history.** Return all in-scope events for
  `(contract_id, account)` within a ledger range, in the total order of
  §3.4, paginated, each carrying its event id and verbatim XDR.
- **C3 — Completeness signal.** Every response MUST state whether the
  served range is complete (`complete: true` only when the indexer holds a
  gap-free history for the whole requested range).
- **C4 — Ingestion status.** Expose the latest fully-ingested ledger so
  clients can bound staleness.

Recommended shape:

```text
GET /v1/health
  -> { latest_ledger, ingested_through, lag_seconds }

GET /v1/tokens/{contract_id}/accounts/{account}/checkpoint?at_ledger={n}
  -> { event: { ledger_seq, tx_hash, event_index, topics_xdr, data_xdr },
       complete }

GET /v1/tokens/{contract_id}/accounts/{account}/events
      ?from_ledger={n}&to_ledger={m}&types={csv}&cursor={c}&limit={k}
  -> { events: [ ... §3.1 records ... ], cursor, complete }
```

`types` filters by event name; servers MUST apply it after attribution
(§3.3), never by dropping events from storage.

## 7. Trust Model and Client-Side Verification

The indexer is trusted for **availability and completeness only** — never
for confidentiality or integrity:

- **Confidentiality.** Everything the indexer holds is public chain data:
  commitments, masked ciphertexts, and ECDH ephemerals. A curious indexer
  learns nothing beyond what any chain observer sees (DESIGN_cont §9).
- **Integrity fails closed.** Recovery ends with the wallet checking its
  reconstructed openings against the **on-chain** commitments
  (`C_spend =? v·G + r·H`, DESIGN_cont §9.5 step 3). A tampered or
  incomplete history cannot produce a wrong balance that verifies; it
  produces a detectable mismatch.
- **Withholding is the residual risk.** A malicious or broken indexer can
  deny recovery (a liveness failure, not a soundness one). Wallets SHOULD
  support multiple independent indexer endpoints, and deployments SHOULD
  run or contract at least two independent archives.

## 8. Conformance and Versioning

An implementation conforms to this specification iff it satisfies §3–§5
and exposes capabilities C1–C4. This document is versioned with the
protocol documentation set; breaking changes to the archived record shape
or the normative capabilities bump the protocol documentation version and
MUST be called out in release notes.
