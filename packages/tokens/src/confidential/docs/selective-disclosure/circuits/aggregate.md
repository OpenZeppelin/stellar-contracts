# Aggregate Disclosures

For statements of the form "this account received at least $$X$$ from counterparty $$Y$$ during window $$W$$" or "these disbursements total exactly $$X$$", the single-event circuits are vectorized over $$n$$ events. The per-event decryption block of D-recipient, D-sender, or D-auditor runs once per event, and the recovered amounts feed one shared aggregation tail. Each role's aggregate is exposed in the two `circuit_id` shapes of [D-balance](d-balance.md): a **predicate-only** shape that includes THRESH and omits U1–U3, and a **value-revealing** shape that includes U1–U3 and omits THRESH ([Circuits](../security.md#circuits)). For D-auditor the `circuit_id` additionally fixes the channel, $$\delta\_{\text{aud\\\_r}}$$ or $$\delta\_{\text{aud\\\_s}}$$, on which every event in the aggregate is disclosed.

The inputs that bind an event to a key holder are per event: $$\text{PVK}\_{A,i}$$ for D-recipient and D-sender, $$\text{PVK}\_{B,i}$$ for D-sender, $$K\_{\text{aud},i}$$ for D-auditor. An outbound aggregate has a different recipient in every event, so $$\text{PVK}\_B$$ cannot be shared, and a per-event prover-side key lets one aggregate span several accounts held by the same prover, or several key versions of one auditor across a rotation ([Auditor Key Management and Rotation](../../protocol/auditing.md#auditor-key-management-and-rotation)) provided the verifier can resolve each historical key ([Verifier Protocol](../protocol.md#verifier-protocol) step 3). A single-account aggregate is the case where every slot carries the same key and the prover supplies the same secret in every witness slot; the circuit does not require the slots to coincide.

## Public inputs

| Symbol | Source |
|:---|:---|
| Common: $$P\_R$$, $$\nu$$ | as in [Circuit D-recipient: Holder Discloses an Inbound Transfer](d-recipient.md) |
| Common: $$m$$ | active event count, $$1 \le m \le n$$; the number of $$\text{ref}\_{E,i}$$ in the bundle |
| Common, D-recipient and D-sender: $$\text{addr\\\_f}$$ | as in [Circuit D-recipient: Holder Discloses an Inbound Transfer](d-recipient.md) |
| Common, predicate-only shape: $$V\_{\text{threshold}}$$ | aggregate threshold, agreed during the request |
| Common, value-revealing shape: $$R\_{\text{disc}}, \tilde{V}\_{\text{disc}}$$ | disclosure ciphertext of $$V\_{\text{total}}$$ |
| D-recipient, for each $$i \in [1, n]$$: $$(\text{PVK}\_{A,i}, R\_{e,i}, \sigma\_{E,i}, \tilde{v}\_i)$$ | $$\text{PVK}\_{A,i}$$ from the account at $$E\_i.\text{to}$$; the rest from event $$i$$ |
| D-sender, for each $$i \in [1, n]$$: $$(\text{PVK}\_{A,i}, R\_{e,i}, \sigma\_{E,i}, \tilde{v}\_i, \text{PVK}\_{B,i})$$ | $$\text{PVK}\_{A,i}$$ from the account at $$E\_i.\text{from}$$ (`Transfer`) or $$E\_i.\text{spender}$$ (`SpenderTransfer`); $$\text{PVK}\_{B,i}$$ from the account at $$E\_i.\text{to}$$; the rest from event $$i$$ |
| D-auditor, for each $$i \in [1, n]$$: $$(K\_{\text{aud},i}, R\_{e,i}, \sigma\_{E,i}, \tilde{v}\_{\text{aud},i})$$ | $$K\_{\text{aud},i}$$ is the auditor key active at event $$i$$'s ledger on the `circuit_id`'s channel, $$\tilde{v}\_{\text{aud},i}$$ that channel's ciphertext in event $$i$$; the rest from event $$i$$ |

In every variant $$\sigma\_{E,i} = \sigma$$ if event $$i$$ is a `Transfer`, $$\sigma\_a'$$ if `SpenderTransfer`. Each event MUST be identified by its own $$\text{ref}\_{E,i}$$ in the proof bundle ([Proof Bundle](../protocol.md#proof-bundle)) and resolved per [Verifier Protocol](../protocol.md#verifier-protocol). Slots $$i > m$$ are inactive and the verifier fills their tuple with zeros.

## Private witnesses

| Variant | Witnesses |
|:---|:---|
| D-recipient | $$\\{sk\_{A,i}\\}\_{i=1}^n$$, $$\\{vk\_{A,i}\\}\_{i=1}^n$$, $$\\{v\_{\text{transfer},i}\\}\_{i=1}^n$$, and $$r\_{\text{disc}}$$ in the value-revealing shape |
| D-sender | $$\\{sk\_{A,i}\\}\_{i=1}^n$$, $$\\{vk\_{A,i}\\}\_{i=1}^n$$, $$\\{r\_{e,i}\\}\_{i=1}^n$$, $$\\{v\_{\text{transfer},i}\\}\_{i=1}^n$$, and $$r\_{\text{disc}}$$ in the value-revealing shape |
| D-auditor | $$\\{aud\_{sk,i}\\}\_{i=1}^n$$, $$\\{v\_{\text{transfer},i}\\}\_{i=1}^n$$, and $$r\_{\text{disc}}$$ in the value-revealing shape |

## Constraints

| # | Constraint |
|:--|:---|
| D-recipient, for each $$i \le m$$: $$\text{D1}\_i$$ – $$\text{D4}\_i$$ | D1–D4 of [Circuit D-recipient: Holder Discloses an Inbound Transfer](d-recipient.md) on $$(sk\_{A,i}, vk\_{A,i}, \text{PVK}\_{A,i}, R\_{e,i}, \sigma\_{E,i}, \tilde{v}\_i)$$, recovering $$v\_{\text{transfer},i}$$ |
| D-sender, for each $$i \le m$$: $$\text{D1}\_i$$, $$\text{D2}\_i$$, $$\text{DS3}\_i$$ – $$\text{DS5}\_i$$ | D1, D2, DS3–DS5 of [Circuit D-sender: Sender Discloses an Outbound Transfer](d-sender.md) on $$(sk\_{A,i}, vk\_{A,i}, \text{PVK}\_{A,i}, r\_{e,i}, R\_{e,i}, \text{PVK}\_{B,i}, \sigma\_{E,i}, \tilde{v}\_i)$$, recovering $$v\_{\text{transfer},i}$$ |
| D-auditor, for each $$i \le m$$: $$\text{A1}\_i$$ – $$\text{A4}\_i$$ | A1–A4 of [Circuit D-auditor: Auditor Discloses a Transfer](d-auditor.md) on $$(aud\_{sk,i}, K\_{\text{aud},i}, R\_{e,i}, \sigma\_{E,i}, \tilde{v}\_{\text{aud},i})$$ with $$\delta\_{\text{aud}}$$ the `circuit_id`'s channel, recovering $$v\_{\text{transfer},i}$$ |
| For each $$i \le m$$: $$\text{D5}\_i$$ | $$v\_{\text{transfer},i} \in [0, 2^{127})$$ |
| For each $$i > m$$: $$\text{PAD}\_i$$ | $$v\_{\text{transfer},i} = 0$$; the slot's per-event block is not enforced |
| AGG | $$V\_{\text{total}} = \sum\_{i=1}^n v\_{\text{transfer},i}$$ |
| AGG-R | $$V\_{\text{total}} \in [0, 2^{127})$$ (range, [Integer Embedding and Range Proofs](../../protocol/primitives.md#integer-embedding-and-range-proofs)) |
| THRESH (predicate-only shape) | $$V\_{\text{total}} \geq V\_{\text{threshold}}$$, evaluated as $$V\_{\text{total}} - V\_{\text{threshold}} \in [0, 2^{127})$$ on the AGG-R-checked $$V\_{\text{total}}$$, with $$V\_{\text{threshold}} \in [0, 2^{127})$$ |
| U1–U3 (value-revealing shape) | Encrypt the AGG-R-checked $$V\_{\text{total}}$$ (not the individual $$v\_{\text{transfer},i}$$) to the recipient, with $$\delta\_{\text{disc\\\_bind}}$$ replacing $$\delta\_{\text{disc}}$$ to separate domain |

The circuit derives the activity flag $$a\_i = [i \le m]$$ from the public input $$m$$ and enforces every per-event constraint of slot $$i$$ under $$a\_i$$, so an inactive slot carries no decryption obligation and contributes nothing to AGG. Because $$m$$ is public, the prover cannot deactivate a slot the verifier filled from a resolved event.

Over $$\mathbb{F}\_r$$ the comparison in THRESH is undefined and decrypting $$\tilde{V}\_{\text{disc}}$$ recovers $$V\_{\text{total}}$$ only modulo $$r$$, so AGG-R MUST precede THRESH and U1–U3. The slot count $$n$$ is fixed per aggregate `circuit_id` ([Circuits](../security.md#circuits)); every realizable $$n$$ satisfies $$n \cdot 2^{127} < r$$, so the field sum in AGG is the integer sum and AGG-R bounds that integer. An aggregate of $$2^{127}$$ or more exceeds the token's `i128` domain and is not disclosable by this circuit.

The disclosure recipient filters the events off-chain by the criteria they care about (counterparty address, block timestamp) before constructing the verifier's public inputs. They learn $$V\_{\text{total}}$$ in the value-revealing shape and only whether it clears $$V\_{\text{threshold}}$$ in the predicate-only shape; the individual amounts are revealed in neither.

Per-event $$\text{PVK}\_{A,i}$$ matters most for outbound aggregates. Spends from one account serialise, since each spend proof opens the current $$C\_{\text{spend}}$$, so a payer that parallelises disbursements across several accounts would otherwise disclose one aggregate per account, and per-account subtotals over few events approach per-recipient disclosure. Spanning accounts reveals nothing beyond the event stream: the verifier reads every participating address from the events it resolves.

## Verifier flow

Follow [Verifier Protocol](../protocol.md#verifier-protocol) with the aggregate `circuit_id` for the role and shape. Steps 1–3 run once per $$\text{ref}\_{E,i}$$: step 2 resolves $$\text{PVK}\_{A,i}$$ (and, for D-sender, $$\text{PVK}\_{B,i}$$) from event $$i$$'s addresses as for the single-event variant, and step 3 resolves $$K\_{\text{aud},i}$$ at event $$i$$'s ledger on the `circuit_id`'s channel. Step 4 lays out the per-event tuples in bundle order, sets $$m$$ to the number of refs, zero-fills the slots above $$m$$, and in the predicate-only shape enters the $$V\_{\text{threshold}}$$ agreed during the request, never one taken from the bundle. Step 6 decrypts under $$\delta\_{\text{disc\\\_bind}}$$ in place of $$\delta\_{\text{disc}}$$ and applies only to the value-revealing shape; a predicate-only bundle omits $$(R\_{\text{disc}}, \tilde{V}\_{\text{disc}})$$. The verifier MUST reject a bundle in which two $$\text{ref}\_{E,i}$$ resolve to the same event: AGG sums whatever it is given, and a repeated event would count its amount twice.

---

Previous: [D-balance](d-balance.md) · Up: [Index](../../README.md#companions) · Next: [Security and Implementation Notes](../security.md)
