# Indexing and Off-Chain State Recovery

Companion specification to [DESIGN.md](./DESIGN.md) §5.2 (Off-Chain Opening Maintenance) and [DESIGN_cont.md](./DESIGN_cont.md) §9.5 (State Recovery). It specifies the durable event archive — the *indexer* — that those sections assume: its data model, ingestion contract, retention obligations, and recommended API surface.

The key words MUST, MUST NOT, SHOULD, and MAY are to be interpreted as in RFC 2119. The normative audience is twofold:

- **Indexer operators** MUST satisfy §3–§5 for the deployment to support recovery from seed.
- **Wallets and SDKs** MUST consume an indexer meeting this contract for recovery, and SHOULD apply the client-side verification in §7.

## 1. Why the Indexer Is Load-Bearing

Confidential balances are Pedersen commitments; the on-chain entry alone does not reveal the opening `(v, r)` needed to spend. A wallet that loses its local cache reconstructs the opening deterministically from the master secret plus the account's event history: the latest *checkpoint* event supplies `(b_tilde, sigma)` from which the spendable opening is derived (DESIGN_cont §9.5), while the receiving-side opening is rebuilt by replaying deposits and incoming transfers back to the account's last `Merge`, or to registration if it has never merged (§2).

Stellar RPC retains events for a **7-day window** only. A wallet that loses local state after that window can still see that its funds exist (the commitment remains on-chain) but cannot reconstruct the opening required to spend them — unless a durable archive holds the missing events. That archive is this document's subject. Without a conforming indexer, recovery from seed is not guaranteed, and deployments MUST treat wallet-local state as unrecoverable after the RPC window.

**RPC and the archive compose.** A wallet need not read everything from the archive. Recent history is still live on Stellar RPC, so a client reads the recent tail from RPC directly and only the older portion — everything past the RPC retention floor — from the archive, stitching the two at a *seam* (§2) and deduplicating by event id (§3.4). The archive's obligation is therefore durable retention of everything older than the RPC window, not keeping pace with the chain head. Reading the whole history from the archive is equally conformant; the split is a client optimization. Either way the guarantee holds only if the archive does not fall so far behind that its ingested-through ledger (§6 C4) drops below the seam — see §4.

## 2. Terminology

- **Checkpoint** — an owner-initiated proof-carrying event (`Withdraw`, sender-side `Transfer`, `SetSpender`, `RevokeSpender`) that publishes `(b_tilde, sigma)` for the owner's spendable balance (DESIGN_cont §9.5).
- **Replay window** — for the spendable side, the range from an account's latest checkpoint to the current ledger; for the receiving side, the range from the last `Merge` (or registration, if never merged) to the current ledger.
- **Event id** — the triple `(ledger_seq, tx_hash, event_index)`, unique per emitted event: `tx_hash` is globally unique, and `event_index` is unambiguous because a Soroban transaction carries a single operation. The same event MUST carry the same id whether served from the archive or from RPC, so a hybrid client can deduplicate across the seam. The id does not by itself encode position within a ledger — `tx_hash` conveys no ordering — so the canonical total order is instead `(ledger_seq, tx_application_order, event_index)`, where `tx_application_order` is persisted as its own field (§3.1) and drives §3.4.
- **Seam** — in a hybrid client (§1) that reads the recent tail from Stellar RPC and older history from the archive, the ledger at which it switches sources. A client sets the seam at or below the RPC retention floor (`getHealth().oldestLedger`) so the RPC side is always served from live retention, and requires the archive's ingested-through ledger (§6 C4) to reach the seam.

## 3. Data Model

### 3.1 Archived record

For every in-scope event the indexer MUST persist:

| Field | Contents |
|:--|:--|
| `contract_id` | The confidential token contract address. |
| `ledger_seq` | Ledger sequence the event was emitted in. |
| `ledger_close_time` | Close time of that ledger. |
| `tx_hash` | Transaction hash. |
| `tx_application_order` | Position of the transaction within its ledger; supplies the intra-ledger ordering that `tx_hash` does not (§3.4). |
| `event_index` | Index of the event within the transaction. |
| `topics`, `data` | The event's topics and data payload — verbatim XDR (RECOMMENDED) or a pinned decoded encoding (see note). |

Verbatim XDR is the RECOMMENDED payload: the wallet decodes it directly and, being the on-chain wire form, it survives field renames in the Rust bindings without a schema migration. A decoded representation — for example the JSON a managed indexing pipeline emits — MAY be served instead. Whichever form is served, it MUST reproduce the on-chain event exactly under that decoder. Indexers MAY additionally store decoded columns for querying.

### 3.2 Events in scope

All events emitted by the confidential token (DESIGN_cont §11.2) with the following recovery roles:

| Event | Role in recovery |
|:--|:--|
| `Register` | Start of the account's history; bounds the worst-case replay window. |
| `Deposit` | Receiving-side replay: accumulates `(amount, 0)` into the receiving opening. |
| `Transfer` (recipient side) | Receiving-side replay: carries the recipient-channel ciphertexts for `(v_tx, r_tx)`. |
| `SpenderTransfer` (recipient side) | Receiving-side replay, as above. |
| `Merge` | Folds the receiving opening into the spendable opening; resets the receiving side. |
| `Withdraw`, `Transfer` (sender side), `SetSpender`, `RevokeSpender` | **Checkpoints**: publish `(b_tilde, sigma)` for the owner's spendable balance. `SetSpender`/`RevokeSpender` are in scope as owner checkpoints only — a spender recovers allowance state from the on-chain delegation entry (`allowance_commitment`, `encrypted_allowance`, `escrowed_dvk`, `allowance_salt`), not from the archive. The auditor-channel ciphertexts these events also carry are out of scope for wallet recovery. |

Configuration events (`UnderlyingAssetSet`, `VerifierSet`, `AuditorSet`, `AddressAsFieldSet`, verification-key events) are not needed for balance recovery; indexers SHOULD archive them anyway — they are low-volume and useful for deployment forensics.

### 3.3 Account attribution

Recovery is per-account, and an event belongs to **each** account address appearing in its topics — a `Transfer` to both the sender's and the recipient's history, a `SpenderTransfer` to the owner's, recipient's, and spender's. Attribution MUST come from the event topics, never from the transaction source account.

The indexer MAY apply this attribution server-side (per-account queries, §6 C2) or serve the whole per-contract stream and leave the client to select the events touching its account; both conform, since attribution is a pure function of the topics. Server-side per-account filtering is RECOMMENDED for high-volume contracts, where downloading the full contract history to every wallet does not scale.

### 3.4 Ordering

The indexer MUST preserve and expose the total order `(ledger_seq, tx_application_order, event_index)` — all three components are persisted per §3.1. Replay correctness depends on it: interleaved deposits, transfers, and merges only reconstruct the right openings when applied in emission order (DESIGN §5.2 step 5).

## 4. Ingestion Contract

- **Source.** Any source that yields the complete, final event stream (Stellar RPC `getEvents`, Horizon, or a captive core). Stellar ledgers are final at close; there is no reorg handling.
- **Freshness.** The archive's ingested-through ledger (§6 C4) MUST stay within the source's retention window — at or above the seam a hybrid client would set (§2). It need not track the chain head: the RPC serves the recent tail (§1). But if it falls below the seam, a gap opens that neither source covers. Detect and backfill any gap while its ledgers are still retrievable from a source; a gap that can no longer be filled is permanent, and affected ranges MUST then be reported incomplete (§6 C3).
- **Idempotency.** Ingestion MUST be at-least-once, deduplicated by event id.
- **Gaps.** The indexer MUST track contiguous ingested ledger ranges. If a gap can no longer be backfilled from any source, the indexer MUST NOT silently serve affected histories as complete (see C3 in §6).
- **Fidelity.** Events MUST be stored faithfully (§3.1) — verbatim XDR or a decoded form pinned to the canonical decoder. Decoding for queries is a read-side concern.

## 5. Retention Obligations

The indexer MUST retain the full per-account history of every in-scope event **indefinitely**. No pruning horizon is safe in general:

- The spendable-side window reaches back to the account's latest checkpoint, which is arbitrarily old for a dormant account.
- The receiving-side window reaches back to the last `Merge`, which for an account that receives but never spends is its registration.

Incoming-transfer spam makes per-account storage linear in the number of inbound events; operators SHOULD provision for this and MAY rate-limit *serving* (never retention) per DESIGN_cont §9.5.

## 6. API Surface

C2–C4 below are normative; C1 is RECOMMENDED. The REST shape is RECOMMENDED — any transport exposing the same capabilities conforms.

- **C1 — Latest checkpoint (RECOMMENDED).** Return the most recent checkpoint event for `(contract_id, account)` at or before a given ledger. This is an optimization, not a correctness requirement: each checkpoint carries a self-contained `(b_tilde, sigma)` that fully re-derives the spendable opening, so a client can also obtain the latest checkpoint by scanning the ordered history (C2). Exposing C1 lets a dormant account with a long history skip transferring that history.
- **C2 — Ordered history.** Return all in-scope events for `(contract_id, account)` within a ledger range, in the total order of §3.4, paginated, each carrying its event id and payload (§3.1). An indexer that serves only the per-contract stream (§3.3) satisfies C2 by delivering that stream in order for client-side attribution.
- **C3 — Completeness signal.** Every response MUST state whether the served range is complete (`complete: true` only when the indexer holds a gap-free history for the whole requested range).
- **C4 — Ingestion status.** Expose the latest fully-ingested ledger so clients can bound staleness. A hybrid client (§1) compares it against the seam it derives from the RPC retention floor; if the archive has not ingested through the seam, the client MUST treat the crossing range as incomplete (C3).

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

`types` filters by event name; servers MUST apply it after attribution (§3.3), never by dropping events from storage. The `/checkpoint` endpoint implements the optional C1 and MAY be omitted. A deployment MAY instead expose history as a single per-contract stream (`GET /v1/tokens/{contract_id}/events`) and leave attribution to the client (§3.3).

## 7. Trust Model and Client-Side Verification

The indexer is trusted for **availability and completeness only** — never for confidentiality or integrity:

- **Confidentiality.** Everything the indexer holds is public chain data: commitments, masked ciphertexts, and ECDH ephemerals. A curious indexer learns nothing beyond what any chain observer sees (DESIGN_cont §9).
- **Integrity fails closed.** Recovery ends with the wallet checking its reconstructed openings against the **on-chain** commitments (`C_spend =? v·G + r·H`, DESIGN_cont §9.5 step 3). A tampered or incomplete history cannot produce a wrong balance that verifies; it produces a detectable mismatch.
- **Withholding is the residual risk.** A malicious or broken indexer can deny recovery (a liveness failure, not a soundness one). Two structural mitigations: for the recent window the RPC is an independent source of the same events (the hybrid split of §1), so archive withholding bites only the pre-window history; and for that older history wallets SHOULD support multiple independent archive endpoints, with deployments running or contracting at least two.

## 8. Conformance and Versioning

An implementation conforms to this specification iff it satisfies §3–§5 and exposes the normative capabilities C2, C3, and C4 (C1 is RECOMMENDED). This document is versioned with the protocol documentation set; breaking changes to the archived record shape or the normative capabilities bump the protocol documentation version and MUST be called out in release notes.
