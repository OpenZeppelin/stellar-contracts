# Circuit D-sender: Sender Discloses an Outbound Transfer

The party that **originated** an on-chain confidential transfer proves to a third party that they paid $$v\_{\text{transfer}}$$ to the on-chain recipient address recorded in the event. "Sender" here covers both:

- **`Transfer` events:** the originator is the account holder $$A$$ at `from`. The disclosing key material is the holder's own $$(sk\_A, vk\_A)$$ and the ephemeral scalar $$r\_e$$ used at transfer time.
- **`SpenderTransfer` events:** the originator is the spender at `spender`, **not** the owner at `from`. The disclosing key material is the spender's own $$(sk\_{\text{op}}, vk\_{\text{op}})$$ and the ephemeral scalar $$r\_e$$ used at transfer time.

In both cases the prover must supply the ephemeral scalar $$r\_e$$ as a witness: the sender has no ECDH path through their own $$vk$$ into the event ciphertext $$\tilde{v}$$ (that ciphertext is keyed to the recipient's $$\text{PVK}\_B$$), so $$r\_e$$ is necessary to reconstruct the recipient-side decryption from the sender's side.

**Recovering $$r\_e$$.** The originator recomputes the ephemeral scalar from its own viewing key and the event nonce ([ECDH-Derived Blinding](../../protocol/keys-and-commitments.md#ecdh-derived-blinding)):

$$r\_e = \text{Poseidon2}(\delta\_{\text{eph}}, vk, \sigma\_E)$$

where $$vk$$ is the originator's viewing key ($$vk\_A$$ for `Transfer`, $$vk\_{\text{op}}$$ for `SpenderTransfer`) and $$\sigma\_E$$ is the event nonce ($$\sigma$$ or $$\sigma\_a'$$). The disclosed amount then follows:

$$v\_{\text{transfer}} = \tilde{v} - \text{Poseidon}(\delta\_{\text{transfer\\\_amount}}, \text{ECDH}(r\_e, \text{PVK}\_B), \sigma\_E) \qquad \text{(Elliptic Curve Diffie-Hellman)}$$

with $$\text{PVK}\_B$$ read from the event's `to` address. Both quantities come from the wallet's $$vk$$ and an on-chain read of the event, so D-sender needs **no** per-transfer wallet state, matching the storage-free posture of D-recipient ([Circuit D-recipient: Holder Discloses an Inbound Transfer](d-recipient.md)). Recovery happens off-circuit: $$r\_e$$ enters the proof as a witness pinned by DS3, and no disclosure circuit absorbs $$\delta\_{\text{eph}}$$.

**What this implies for $$vk$$.** Since $$r\_e$$ is recoverable from $$vk$$, so is a full Pedersen opening of every transfer the account originated. [Privacy Properties](../../protocol/security.md#privacy-properties) states the capability a compromised $$vk$$ therefore carries; [Recipient-auditor opening capability](../../protocol/auditing.md#recipient-auditor-opening-capability) records the other holder of those per-transfer openings, so they are exclusive to neither party. The operative consequence for this layer is that a counterparty needing outbound visibility is served with D-sender proofs, which are bound to that counterparty and to a nonce ([Recipient Binding](../security.md#recipient-binding)) — never by handing over $$vk$$ ([Security Requirements](../../sdk/requirements.md#security-requirements)).

In the symbols below, $$A$$ denotes the **originating** address — the holder's address for `Transfer` and the spender's address for `SpenderTransfer`. $$sk\_A$$ is the originator's spending key, $$\text{PVK}\_A$$ is the originator's stored public viewing key, and $$\sigma\_E = \sigma$$ for `Transfer`, $$\sigma\_E = \sigma\_a'$$ for `SpenderTransfer`.

## Public inputs

| Symbol | Source |
|:---|:---|
| $$\text{addr\\\_f}$$ | compressed contract-address Field, loaded from instance storage |
| $$\text{PVK}\_A$$ | originating account's stored `viewing_public_key` (holder for `Transfer`, spender for `SpenderTransfer`) |
| $$R\_e, \sigma\_E, \tilde{v}$$ | from the on-chain event |
| $$\text{PVK}\_B$$ | recipient's stored `viewing_public_key` (looked up from event's `to` address) |
| $$P\_R, \nu$$ | disclosure recipient pubkey and nonce |
| $$R\_{\text{disc}}, \tilde{v}\_{\text{disc}}$$ | disclosure ciphertext |

## Private witnesses

$$sk\_A$$, $$vk\_A$$, $$r\_e$$, $$v\_{\text{transfer}}$$, $$r\_{\text{disc}}$$.

## Constraints

| # | Constraint |
|:--|:---|
| D1 | $$vk\_A = \text{Poseidon}(\delta\_{\text{vk}}, sk\_A, \text{addr\\\_f})$$ |
| D2 | $$\text{PVK}\_A = vk\_A \cdot H$$ |
| DS3 | $$R\_e = r\_e \cdot H$$ (prover knows the ephemeral scalar used at transfer time; same shape as [T6](../../protocol/operations/transfer.md#constraints) for `Transfer` and O6 for `SpenderTransfer`) |
| DS4 | $$s\_B = \text{ECDH}(r\_e, \text{PVK}\_B)$$ (sender-side ECDH shared scalar to recipient, [Elliptic Curve Diffie-Hellman](../../protocol/primitives.md#elliptic-curve-diffie-hellman)) |
| DS5 | $$v\_{\text{transfer}} = \tilde{v} - \text{Poseidon}(\delta\_{\text{transfer\\\_amount}}, s\_B, \sigma\_E)$$ |
| D5 | $$v\_{\text{transfer}} \in [0, 2^{127})$$ |
| U1–U3 | Disclosure ciphertext to recipient ([Disclosure Ciphertext to Recipient](../protocol.md#disclosure-ciphertext-to-recipient)) |

DS3 anchors $$R\_e$$ to the originator by forcing them to know $$r\_e$$. Combined with D1/D2, this proves the prover is the same party that produced the transfer's ephemeral key — the holder for `Transfer`, the spender for `SpenderTransfer`. DS4 and DS5 reconstruct the recipient-side decryption from the originator's perspective.

## Coverage asymmetry: owner cannot D-sender a `SpenderTransfer`

A spender transfer's ephemeral scalar derives from the spender's $$vk\_{\text{op}}$$ ([Spender Transfer](../../protocol/operations/spender-transfer.md)), which the owner does not hold, so the owner cannot recover $$r\_e$$ for the event; nor does the owner have any other ECDH path into $$\tilde{v}$$, which is keyed to $$\text{PVK}\_B$$. The owner therefore cannot independently produce a D-sender disclosure for a `SpenderTransfer`. The owner's cryptographic paths for that event are:

1. **D-auditor ([Circuit D-auditor: Auditor Discloses a Transfer](d-auditor.md))** routed through the owner's auditor key $$K\_{\text{aud,s}}$$, which decrypts $$\tilde{v}\_{\text{aud,s}}$$ for every `SpenderTransfer` from the owner's account ([Spender Transfer Auditing](../../protocol/auditing.md#spender-transfer-auditing)). This is the canonical owner-side path.
2. **D-sender by the cooperating spender.** If the spender is willing, they construct a D-sender proof against the spender's own $$(sk\_{\text{op}}, \text{PVK}\_{\text{op}})$$ and deliver it to the owner, who forwards it (or the owner asks the disclosure recipient to accept proofs originated by the spender). The proof's $$\text{PVK}\_A$$ is the spender's PVK; the verifier looks it up at the event's `spender` address.

A D-sender proof for a `SpenderTransfer` proves that the spender (not the owner) paid the on-chain `to`. If the disclosure recipient additionally needs proof that the owner authorized this spender, they read the `SetSpender` event and observe the on-chain `(owner, spender)` delegation entry; nothing in D-sender attests to delegation provenance.

## Verifier flow

Follow [Verifier Protocol](../protocol.md#verifier-protocol) with `circuit_id = D-sender`. Step 2 looks up two account records: $$\text{PVK}\_A$$ at $$E.\text{from}$$ (for `Transfer`) or $$E.\text{spender}$$ (for `SpenderTransfer`), and $$\text{PVK}\_B$$ at $$E.\text{to}$$ in both cases.

---

Previous: [D-recipient](d-recipient.md) · Up: [Index](../../README.md#companions) · Next: [D-auditor](d-auditor.md)
