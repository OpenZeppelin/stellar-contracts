# Confidential Token: SDK

Companion specification to [DESIGN.md](./DESIGN.md) §4 (Key Hierarchy) and §5.2 (Off-Chain Opening Maintenance), [DESIGN_cont.md](./DESIGN_cont.md) §9.5 (State Recovery) and §11 (Interface), [INDEXER.md](./INDEXER.md), and [SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §5 and §15. It specifies the client layer those documents assume: the crypto core that mirrors the Noir circuits off-chain, the key derivation the protocol leaves open, the witness and payload construction, the wallet state machine, and the auditor, disclosure, and indexer clients.

The key words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are to be interpreted as in RFC 2119. The normative audience is threefold:

- **SDK implementers** MUST satisfy §4–§12.
- **Wallet, auditor, and application integrators** MUST NOT bypass §13; the security properties of the protocol do not survive it.
- **Port authors** (mobile, hardware wallet, a second language) MUST pass §6.

**Scope.** This document specifies obligations, not an API. It does not prescribe function signatures, module names, package layout, or class design, and it does not restate protocol formulas that [DESIGN.md](./DESIGN.md) already fixes — each requirement cites its source section instead.

---

## 1. Why the SDK Is Load-Bearing

Four properties of the protocol place correctness and confidentiality in the client rather than in the contract.

**The opening exists only off-chain.** A balance is a Pedersen commitment $$C = v \cdot G + r \cdot H$$. The chain stores the point; the opening $$(v, r)$$ that authorizes the next spend lives exclusively in client state (DESIGN.md §5.2). A client that loses, misderives, or misaccumulates the opening makes the funds unspendable, and the contract cannot help because it never knew the value.

**Every amount a user sees is client-decrypted.** The contract performs homomorphic point arithmetic and never learns a value. Balances, transfer amounts, allowances, and audit figures are all produced by client-side decryption of event ciphertexts, so a decryption defect yields a plausible wrong number rather than a visible failure.

**The client is an enforcement point for canonicality.** Neither the Soroban host nor the verifier distinguishes a canonical $$\mathbb{F}_r$$ representative from a non-canonical one (DESIGN.md §2.2, *Host deserialiser caveat*); the contract enforces canonicality at its boundary, but the client is where the bytes are produced.

**The client is where all secrets live and all randomness is sampled.** The spending key, the viewing key, every blinding factor, and every per-operation salt originate client-side, so the protocol's confidentiality reduces to the client's key handling and CSPRNG quality. DESIGN.md §2.5 makes salt uniqueness a soundness requirement.

---

## 2. Terminology and Layering

### 2.1 Terminology

- **Root** — the secret an implementation feeds to §5.1's derivation: a SEP-0053 signature by a signer on the account, or a raw 32-byte value for an address with no ed25519 signer of its own (§5).
- **Opening** — the pair $$(v, r)$$ such that $$C = v \cdot G + r \cdot H$$ for an on-chain commitment $$C$$.
- **Checkpoint** — an owner-initiated proof-carrying event publishing $$(\tilde{b}, \sigma)$$ for the owner's spendable balance (DESIGN.md §5.2 *Recovery*, which enumerates the qualifying events).
- **Witness material** — any value that appears as a private witness in any circuit: $$sk$$, $$vk$$, $$dvk_i$$, $$v$$, $$r$$, $$r_e$$, $$v_{\text{transfer}}$$, and every intermediate derived from them.
- **Trust boundary** — the process and storage under the account holder's exclusive control. Witness material inside it is secret; witness material that crosses it is disclosed.
- **In-flight operation** — a submitted operation whose event has not yet been observed. Its projected post-operation opening is known locally but not yet confirmed against chain state.
- **Facade** — a role-scoped interface over the crypto core (§3).

### 2.2 Layering

An implementation MUST separate the following concerns. The boundaries are normative because §15 defines conformance over the lower layers only; the upper layers are deployment-shaped and deliberately unconstrained in structure.

| Layer | Section | Responsibility | Purity |
|:--|:--|:--|:--|
| Crypto core | §4 | Field and curve arithmetic, Poseidon2, derivations, encodings | Deterministic; no I/O; holds no state |
| Key derivation | §5 | Root → $$sk$$ → the DESIGN.md §4 hierarchy | Deterministic; no I/O |
| Conformance vectors | §6 | Fixture and circuit-execution parity | Test-only |
| Witness assembly | §7 | Per-circuit private witnesses and public inputs | Deterministic given randomness |
| Prover | §8 | Circuit artifacts, verification keys, proof generation | I/O; pluggable backend |
| Chain adapter | §9 | Reads, payload encoding, submission, typed errors | I/O |
| Role facades | §10–§12 | Holder wallet, auditor, disclosure, indexer clients | Stateful |

---

## 3. Roles and Capability Separation

Five roles consume the protocol. Each holds distinct key material and MUST be *structurally* incapable of exceeding its capability.

| Role | Holds | Can |
|:--|:--|:--|
| Holder | Root, $$sk$$, $$vk$$ | Spend, withdraw, merge, delegate, disclose, read own balances |
| Spender | Own $$sk_{\text{op}}$$, escrowed $$dvk_i$$ | Spend from the allowance, read allowance state, disclose own spender transfers |
| Auditor | Auditor secret $$k$$ | Decrypt both channels for accounts bound to its `auditor_id` (DESIGN_cont.md §8.1) |
| Disclosure recipient | $$(r_R, P_R)$$ | Verify a disclosure proof and recover the disclosed amount |
| Observer | Nothing | Read commitments, ciphertexts, ephemerals, addresses, public amounts |

---

## 4. Crypto Core

Every requirement in this section is reproduced from `circuits/lib/src/lib.nr`, which is the source of truth wherever this document and the protocol documents disagree.

### 4.1 Fields and the `q` / `p` notation hazard

Two moduli are in play, and confusing them silently corrupts state (§4.6).

| Modulus | Value | Role | Called elsewhere |
|:--|:--|:--|:--|
| $$r$$ | `0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001` | BN254 scalar field. Noir's `Field`; the host's `Bn254Fr`; Grumpkin **coordinate** field | `FR_MODULUS` |
| $$q$$ | `0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47` | BN254 base field. Grumpkin **scalar** (multiplier) field, i.e. the group order | `FP_MODULUS`, and conventionally $$p$$ in the pairing literature |

$$r < q$$, so every $$\mathbb{F}_r$$ element is already a valid Grumpkin scalar with no reduction, which is why a Noir `Field` can be passed to `multi_scalar_mul` unambiguously.

### 4.2 Canonicality

A value is a **canonical** $$\mathbb{F}_r$$ representative iff it is a 32-byte big-endian encoding of an integer in $$[0, r)$$.

- Every $$\mathbb{F}_r$$ value the SDK emits — into a payload, an event assertion, a proof input, or persisted state — MUST be canonical.
- The SDK MUST reject a non-canonical value at its own boundary rather than relying on the contract's check (DESIGN.md §2.2). A client that produces non-canonical bytes has already lost byte-uniqueness in the local state that recovery reads from.
- Points are encoded as `BytesN<64>` = $$\text{be}(x) \\| \text{be}(y)$$, a **flat** 64-byte value. The identity $$\mathcal{O}$$ is all 64 bytes zero, and decodes back to the identity.

### 4.3 Poseidon2 sponge

The sponge construction, its width and rate, the IV placement, the padding rule, and the two-lane form of $$\text{SpongeSqueeze}_2$$ are specified normatively in DESIGN.md §2.5. What follows is what that construction additionally requires of a client.

**Two self-checks are available before any proof is generated.** The absorbed length in $$\text{SpongeSqueeze}_2$$ is always 3, so its IV is fixed at $$3 \cdot 2^{64}$$; and its first lane is identical to $$\text{poseidon\\\_with\\\_domain}(\delta, [s, \sigma])$$ on the same inputs. An implementation that reproduces both has the block layout and the IV lane right.

**The domain-tagged funnel.** Every Poseidon2 invocation in the protocol routes through one entry point that places the domain tag as the **first absorbed element**:

$$\text{poseidon\\\_with\\\_domain}(\delta, [x_1, \ldots, x_n]) = \text{sponge}([\delta, x_1, \ldots, x_n])$$

Squeeze-slot assignment is canonical and MUST be followed: lane 0 is always an amount mask, lane 1 is always a balance, allowance, or per-transfer-randomness mask. Single-ciphertext channels — the `Withdraw` balance checkpoint (DESIGN.md W_a3/W_a4) — take lane **1** and leave lane 0 unused, so a checkpoint pad can never coincide with an amount pad.

### 4.4 Generators and commitments

$$G$$ and $$H$$ are Barretenberg's `derive_generators("DEFAULT_DOMAIN_SEPARATOR")` outputs at indices 0 and 1, fixed in DESIGN_cont.md §10.4.

$$\text{commit}(v, r) = v \cdot G + r \cdot H$$

Scalar multiplication MUST map a zero scalar to the identity rather than erroring, because a registered account's opening commitments are the identity and a zero blinding factor is legitimate (deposits commit with $$r = 0$$, DESIGN.md §7.3).

### 4.5 ECDH

$$\text{ECDH}(a, B) = \text{poseidon\\\_with\\\_domain}(\delta_{\text{ecdh}}, [S.x, S.y]) \quad \text{where} \quad S = a \cdot B$$

**Both coordinates MUST be absorbed.** An x-only extraction is negation-invariant: $$P$$ and $$-P$$ share an x-coordinate, and $$-\text{PVK} = (-vk) \cdot H$$ is itself a valid canonical registration, so an x-only map would collapse each $$(vk, -vk)$$ pair onto one shared secret (DESIGN.md §2.4). The absorb fills exactly one rate-3 block.

The derivation MUST fail rather than proceed if $$S$$ is the identity: with $$\sigma$$ public, an identity shared secret makes every derived ciphertext trivially decryptable, which is why the circuits carry explicit nonzero-scalar constraints (DESIGN_cont.md §10.8).

### 4.6 Blinding accumulation — mod $$q$$, never mod $$r$$

Commitment blinding factors accumulate under homomorphic point addition, so they accumulate in the **Grumpkin scalar field** $$\mathbb{F}_q$$ (§4.1):

$$\text{Com}(v_1, r_1) + \text{Com}(v_2, r_2) = \text{Com}(v_1 + v_2, \\, (r_1 + r_2) \bmod q)$$

Reducing modulo $$r$$ instead yields an opening that is off by $$q - r$$ and no longer matches the on-chain point, and for two full-size blindings the integer sum crosses $$q$$ roughly half the time. Implementations MUST provide distinct, clearly named addition operations for the two moduli and MUST use the $$\mathbb{F}_q$$ one for every blinding accumulation: merge (DESIGN.md §7.4) and receiving-balance credit (DESIGN.md §5.2 *Update rules*).

Committed **values** accumulate as exact integers and MUST NOT be reduced by either modulus; DESIGN.md §2.3 establishes that they never wrap.

### 4.7 Scalar sampling

Secret scalars — $$\sigma$$, $$\sigma_a$$ — MUST be produced by the rejection procedure of DESIGN.md §2.2:

1. Draw 32 bytes from a CSPRNG.
2. Clear the top **2** bits, yielding a 254-bit candidate.
3. Reject and redraw if the candidate is $$\geq r$$, or if it is zero and the call site requires nonzero.

### 4.8 Domain separators

| Tag | Value | Absorbed in a core circuit? |
|:--|:--:|:--|
| $$\delta_{\text{addr}}$$ | 1 | No — absorbed on-chain by the contract (DESIGN.md §2.7) |
| $$\delta_{\text{vk}}$$ | 2 | Yes |
| $$\delta_{\text{dvk}}$$ | 3 | Yes |
| $$\delta_{\text{spend\\\_r}}$$ | 4 | Yes |
| $$\delta_{\text{transfer\\\_blind}}$$ | 5 | Yes |
| $$\delta_{\text{transfer\\\_amount}}$$ | 6 | Yes |
| $$\delta_{\text{enc\\\_bal}}$$ | 7 | Yes |
| $$\delta_{\text{enc\\\_allow}}$$ | 8 | Yes |
| $$\delta_{\text{allow\\\_r}}$$ | 9 | Yes |
| $$\delta_{\text{esc\\\_dvk}}$$ | 10 | Yes |
| $$\delta_{\text{aud\\\_s}}$$ | 11 | Yes |
| $$\delta_{\text{aud\\\_r}}$$ | 12 | Yes |
| $$\delta_{\text{ecdh}}$$ | 13 | Yes |
| $$\delta_{\text{eph}}$$ | 14 | No — derived off-circuit (DESIGN.md §5.3) |
| $$\delta_{\text{disc\\\_bind}}$$ | 15 | No — off-chain disclosure only |
| $$\delta_{\text{disc}}$$ | 16 | No — off-chain disclosure only |

DESIGN_cont.md §13 assigns all sixteen values and is their only source; the right-hand column is this document's addition. $$\delta_{\text{disc\\\_bind}}$$ and $$\delta_{\text{disc}}$$ belong to the off-chain disclosure layer (SELECTIVE_DISCLOSURE.md §2.2). Tag 1 is absorbed by the contract rather than by a circuit — the contract derives $$\text{addr\\\_f}$$ and $$\text{op}_i$$ on-chain and the circuits receive them as opaque public inputs (DESIGN.md §2.7 *Usage sites*) — so it is part of the on-chain wire contract all the same. None of 14–16 is absorbed either in a circuit or on-chain, so none is part of the on-chain wire contract, but all three are part of the cross-client contract because two wallets serving the same account must agree on them (§6.3).

All sixteen values MUST be distinct, and each MUST be used in exactly one sponge mode, per DESIGN.md §2.5 *Mode exclusivity*. Tags 11 and 12 are the two-mask tags; the remaining fourteen, including 1 and 14–16, are single-output tags.

### 4.9 Address compression

$$\text{address\\\_to\\\_field}(a) = \text{poseidon\\\_with\\\_domain}(\delta_{\text{addr}}, [\text{lo}(a), \text{hi}(a)])$$

where $$\text{enc}(a)$$ is the 56-character ASCII strkey (SEP-23), and $$\text{lo}$$ and $$\text{hi}$$ interpret its lower and upper 28 bytes respectively in **little-endian** order (DESIGN.md §2.7). Implementations MUST obtain the strkey from their language's stellar-strkey library.

**Bootstrap check.** On first contact with a deployment, an implementation MAY compute $$\text{addr\\\_f}$$ for the contract's own address and assert equality against the value the contract stores in instance storage (DESIGN.md §3.5).

---

## 5. Key Derivation

DESIGN.md §4 specifies the hierarchy below $$sk$$ — $$vk$$ from $$(sk, \text{addr\\\_f})$$, $$\text{PVK}$$ from $$vk$$, $$dvk_i$$ from $$(vk, \text{op}_i)$$. It does not specify where $$sk$$ itself comes from. This section supplies a derivation, because recovery from backup material is a stated protocol property (DESIGN.md §5.2 *Recovery*, DESIGN_cont.md §9.5, INDEXER.md §1) and two clients given the same backup material would otherwise derive different accounts.

The derivation is a single function (§5.1) over a **root**, and the root's class is determined by what controls the address rather than chosen per client. An address controlled by a Stellar ed25519 key uses a deterministic signature by that key (§5.2). An address with no ed25519 key of its own — a smart account or any other contract address — uses raw bytes from whatever custody mechanism controls it (§5.3). Tying the class to the address rather than to client preference is what keeps two clients from disagreeing about which class produced an account's $$sk$$, a disagreement `register` being single-use (DESIGN_cont.md §11) would make unrepairable. The one residual case is §5.3's fallback, where an ed25519-controlled address has no way to sign the §5.2 message; a client MUST therefore read a $$Y$$ mismatch under §5.2 as evidence that the account uses a root it does not hold, not as a derivation defect.

### 5.1 Derivation

$$sk = \text{RS}\Big(\text{HKDF-SHA-512}\big(\text{IKM} = \text{root}, \\;\\; \text{salt} = \texttt{"openzeppelin/confidential-token/v1/sk"}, \\;\\; \text{info} = \text{be}_{32}(\text{addr\\\_f}) \\,\\|\\, \text{be}_{32}(\text{acct\\\_f}) \\,\\|\\, \text{le}_{4}(j)\big)\Big)$$

where:

| Input | Definition |
|:--|:--|
| $$\text{IKM}$$ | RFC 5869's input keying material: the byte string HKDF-Extract consumes, here the account's root |
| $$\text{root}$$ | Exactly one of — the 64-byte ed25519 signature of §5.2, or a raw 32-byte value (§5.3) |
| $$\text{addr\\\_f}$$ | $$\text{address\\\_to\\\_field}$$ of the confidential token contract (§4.9) |
| $$\text{acct\\\_f}$$ | $$\text{address\\\_to\\\_field}$$ of the address being registered |
| $$j$$ | Rejection counter, starting at 0 |
| $$\text{RS}$$ | The §4.7 procedure applied to the 32-byte HKDF output: clear the top 2 bits, accept iff the result is in $$[1, r)$$, otherwise increment $$j$$ and re-derive |

The candidate MUST also be rejected if the resulting $$vk = \text{poseidon\\\_with\\\_domain}(\delta_{\text{vk}}, [sk, \text{addr\\\_f}])$$ is zero, since registration constraint R5 requires $$vk \neq 0$$.

**The IKM is the root's bytes, verbatim.** HKDF-Extract accepts input keying material of any length, the 64 signature bytes or the 32 raw bytes go in as they are.

Why each element is present:

**HKDF-SHA-512, not Poseidon2.** No circuit constrains how $$sk$$ was obtained — the register circuit constrains only $$Y = sk \cdot H$$ (R1) and $$vk$$'s derivation from $$sk$$ (R2). There is therefore no in-circuit consistency argument for Poseidon2 here, and SHA-512 keeps the custody path on the primitive ed25519 already uses internally, which every Stellar SDK and secure element therefore already implements. Implementations MUST NOT substitute a different KDF, since the choice is arbitrary in isolation but must be identical across clients.

**Bound to $$\text{addr\\\_f}$$.** $$vk$$ is already deployment-scoped by DESIGN.md §4.2, which bounds the blast radius of a *viewing*-key compromise to one deployment. Binding $$sk$$ likewise bounds a *spending*-key compromise, and the contract address is known whenever a client talks to a deployment.

**Bound to $$\text{acct\\\_f}$$.** $$vk$$ depends only on $$(sk, \text{addr\\\_f})$$, so the same $$sk$$ registered under two addresses yields the same $$vk$$, hence identical $$Y$$ and identical $$\text{PVK}$$ published under both accounts and readable by any observer through the account read method, linking two addresses that are otherwise unlinkable. Implementations MUST derive a distinct $$sk$$ per address.

**No account-index input.** Binding $$\text{acct\\\_f}$$ makes a separate SEP-0005 index redundant: the address determines the account, and the index is merely the path that produced the address.

### 5.2 Signer roots

A signer root is a SEP-0053 signature over a message naming this protocol, the deployment, and the account:

$$\text{msg} = \texttt{"openzeppelin/confidential-token/v1/sk"} \\,\\|\\, \texttt{0x0a} \\,\\|\\, \text{enc}(\text{contract}) \\,\\|\\, \texttt{0x0a} \\,\\|\\, \text{enc}(\text{account})$$

$$\text{root} = \text{Ed25519-Sign}\big(sk_{\text{ed}}, \\;\\; \text{SHA-256}(\text{prefix} \\,\\|\\, \text{msg})\big)$$

where `prefix` is SEP-0053's 24 ASCII bytes `Stellar Signed Message:\n`, $$\text{enc}$$ is the 56-character strkey of §4.9, and $$sk_{\text{ed}}$$ is the ed25519 secret of a signer on the account. The message is 151 bytes, printable ASCII apart from its two separators, and carries the strkeys rather than their §4.9 compressions so that a wallet rendering SEP-0053 messages as text shows the user addresses they can compare against the deployment they intend to register on. The signature is the 64-byte RFC 8032 encoding $$R \\,\\|\\, S$$ that every Stellar SDK and SEP-0053 wallet already returns, and those 64 bytes are the IKM.

Binding both addresses into the *message* rather than relying on §5.1's `info` alone is what bounds a harvested signature: a dapp that tricks a user into signing once obtains the root for that account on that deployment, not for every account the key controls on every deployment.

**The SEP-0053 envelope is mandatory even where the secret is extractable.** A client holding the raw ed25519 secret MUST compute this signature itself rather than use the secret's 32 bytes as the IKM directly. One form then covers both custody shapes: an account enrolled through a wallet prompt is reproducible by a client that later imports the secret, and the reverse.

**The ed25519 key is a single point of failure, and this MUST be disclosed.** Whoever obtains the ed25519 secret can recompute the signature and therefore $$sk$$, gaining both view and spend of the confidential account. `register` is single-use, so $$sk$$ cannot be rotated in place, and remediation means registering a fresh address and moving the funds through a transfer. An implementation MUST state, at the point where it offers to create a confidential account, that the account's confidentiality is bounded by the secrecy of the account's signing key.

**Availability is not guaranteed.** A signer root exists only where the custody stack implements SEP-0053 message signing, and support across Stellar wallets and hardware apps is uneven. An implementation MUST treat its absence as an expected outcome and fall back to a raw root (§5.3), which is not reproducible from anything the user already holds and therefore MUST be backed up explicitly, rather than fail enrolment.

**Verify the signature before using it.** An implementation MUST verify the returned signature against the ed25519 public key it expects to have signed, and MUST abort on mismatch. A wallet with a different account selected returns a well-formed signature over the same message, yielding a wrong but entirely usable $$sk$$: registration succeeds, and the account is then unreproducible from the key the user believes controls it.

**Determinism is a precondition.** RFC 8032 ed25519 derives its nonce from the secret and the message, so a conforming signer returns the same 64 bytes forever. Signers that randomise the nonce do not, and threshold and MPC ed25519 are in that category — the nonce is generated per signing session, so a signature does not reproduce in the next one. An implementation MUST obtain the signature twice from independent invocations and MUST abort if they differ. That detects the common case and not every case, since a signer can be deterministic within a session and not across sessions, so an implementation MUST additionally offer $$sk$$ export as a direct-import backup (§5.3) and SHOULD prompt for it before the account first receives funds.

**The IKM MUST NOT be persisted.** The 64-byte signature is equivalent to $$sk$$ for this account and deployment. Implementations MUST derive on demand; where $$sk$$ itself is cached, §13's storage-at-rest rules govern it.

**Record the enrolled signer.** A Stellar address may have signers besides its master key, and $$sk$$ is keyed to the *address* through $$\text{acct\\\_f}$$ rather than to the key that signed for it, so which signer enrolled is not recoverable from the address or from chain state. Implementations MUST record the enrolled ed25519 public key and MUST NOT assume it is the master key. A second client enrolling the same address with a different signer derives a different $$sk$$ whose $$Y$$ does not match the registered spending public key. §5.4's comparison detects that, and an implementation holding more than one candidate signer SHOULD resolve it by trying each and adopting the one whose $$Y$$ matches, rather than reporting a mismatch under whichever it tried first.

**Signer rotation orphans the account.** `set_options` can replace or remove the key controlling an address, and the confidential account survives that rotation while its root does not. An implementation SHOULD compare the recorded signer against the address's current signer set on sync and surface a warning when the enrolled key is no longer among them, because discarding that key after rotation destroys the only path back to $$sk$$.

**Discovery costs one signature per candidate.** §5.4's scan derives $$sk$$ once per candidate index, and each derivation needs a signature by that index's key. A client holding the mnemonic signs locally and scans offline; a client fronted by a custody stack spends a user approval per candidate and reaches nothing for an index whose key that stack will not sign with. An implementation MUST surface which of the two it is doing rather than presenting a bounded scan as exhaustive.

### 5.3 Raw roots and imported keys

Implementations MUST accept a raw 32-byte `root`, those 32 bytes being the IKM. It is the class for every address with no ed25519 key of its own, and three cases require it:

- **Contract addresses.** A confidential account registered by a smart account or other contract address has no ed25519 signer to sign §5.2's message, so its root comes from whatever custody mechanism controls the contract. Where a smart account authorises through signers of its own, the root is supplied by that mechanism rather than derived from any one of them, since the set can change without the address changing.
- **No SEP-0053 path.** An ed25519-controlled account whose custody stack cannot sign arbitrary messages (§5.2) falls back here.
- **Imported keys.** Deployments predating this specification hold $$sk$$ values sampled directly from a CSPRNG with no root behind them. Such a key MUST remain usable as a first-class account secret via direct import, bypassing §5.1 entirely.

A raw root is reproducible from nothing the user already holds, so an implementation that generates one MUST surface it for backup at creation and MUST NOT treat it as recoverable from the account's other credentials.

An implementation MUST record, per account, which of the three forms produced its $$sk$$ — signer root, raw root, or direct import — because they differ in what regenerates the key: a live signer, a stored 32-byte value, or nothing beyond the stored $$sk$$. A user MUST NOT be shown a recovery affordance an account's form cannot satisfy.

### 5.4 Recovery and account discovery

Recovering a signer-root account requires the enrolled signer (§5.2) and the contract address. Where the signer came from a mnemonic, the mnemonic, its passphrase if any, and the SEP-0005 index reproduce it; where it did not, the signer itself must still be held. The set of addresses is discovered rather than remembered:

1. Enumerate candidate Stellar addresses by scanning SEP-0005 indices $$i = 0, 1, 2, \ldots$$ from the seed.
2. For each candidate address, compute $$\text{acct\\\_f}$$, obtain the §5.2 signature from that index's key, derive $$sk$$ per §5.1, and compute $$Y = sk \cdot H$$.
3. Read the account record at that address and compare its stored spending public key against $$Y$$. A match identifies a registered confidential account belonging to this signer.

Step 2 needs a signature rather than a hash, so a client that cannot sign locally SHOULD drive recovery from its own per-account record instead of scanning (§5.2). An account registered under a raw root, or under a signer that is neither held nor reproducible, is not reachable by any scan: only the direct import of a previously exported $$sk$$ recovers it.

---

## 6. Conformance Vectors

### 6.1 Primitive fixtures

`circuits/lib/testdata/*.json` pins one primitive's output per file for a fixed input set, and is the language-agnostic contract for every off-chain consumer. An implementation MUST reproduce every output in every file byte-for-byte.

Its test suite MUST **read** those files rather than transcribe their values into source, so that a change to the Noir library's `print_fixtures` output becomes a test failure rather than a silent divergence.

### 6.2 Circuit-execution parity

Fixtures pin primitives, not witnesses. A client can reproduce every fixture and still assemble a witness with a transposed public input, a wrong field name, or a stale ordering, none of which any fixture covers.

An implementation MUST therefore additionally, for every circuit it supports:

- Build the witness from its own crypto core and have the **real compiled circuit** solve it, asserting success.
- Include **tamper cases** — a witness with one value perturbed — and assert the circuit *rejects* them.

### 6.3 Vectors this specification requires

Three derivations this document specifies or relies on are computed outside the core circuits, so the Noir library's `print_fixtures` does not emit them. The first already has a fixture; the other two require one:

- **$$\text{address\\\_to\\\_field}$$** (§4.9) is pinned by `circuits/lib/testdata/address_to_field.json`, which §6.1's obligation already covers. The circuits receive $$\text{addr\\\_f}$$ as an opaque public input, so this derivation is implemented twice — by the contract on-chain and by every client — making it the sole primitive with two independent implementations and no Noir version, and §4.9's bootstrap check detects a divergence only against an already-deployed contract. Being outside `print_fixtures`, it is also outside the in-Noir `fixtures_match_testdata` guard; the Rust test `address_to_field_matches_testdata_vectors` guards it instead (`circuits/lib/testdata/README.md`).
- **$$\delta_{\text{eph}}$$ derivation** (DESIGN.md §5.3, restated in §10.5). No circuit constrains $$r_e$$, so a fixture is the only mechanism keeping a user's clients in agreement; where two disagree, transfers sent from one are not disclosable from the other.
- **The §5.1 $$sk$$ chain**, from a fixed root, $$\text{addr\\\_f}$$, and $$\text{acct\\\_f}$$ through to $$sk$$, $$vk$$, $$Y$$, and $$\text{PVK}$$, without which recovery from backup material is untestable across implementations. The vector MUST start from a fixed ed25519 secret and run through §5.2's message, its SEP-0053 preimage, and the resulting signature, since the signature is where two clients most plausibly diverge — the only step in the chain whose format is set outside this document. A second vector from a fixed raw root covers §5.3.

---

## 7. Witness Assembly

An implementation MUST provide witness assembly for each circuit it supports, covering the six core circuits of DESIGN_cont.md §10.1: `Register`, `Withdraw`, `Transfer`, `SpenderTransfer`, `SetSpender`, `RevokeSpender`.

**Public-input order is a wire contract.** The verifier sees an ordered vector of field elements with no knowledge of what they denote (DESIGN.md §7.1), so a permutation of two same-typed inputs produces a well-formed vector that verifies a different statement. Each builder MUST assemble public inputs in exactly the order the contract assembles them, and MUST cite the contract function it mirrors at the site of the ordering. The per-operation public-input tables in DESIGN.md §7.2–§7.9 are authoritative for *membership*; the contract's assembly is authoritative for *order*.

**The trust-boundary rule constrains the client too.** DESIGN.md §7.1 requires the contract to load state-derived public inputs itself and never accept them from the caller. The client-side corollary: a builder MUST NOT include a contract-loaded input in the payload it submits. Doing so does not break soundness, since the contract ignores it, but it produces a payload whose fields disagree with the proof's public inputs.

**Prover-supplied values MUST be freshly derived per attempt**, never reused from a previous attempt at the same logical operation (§10.4).

Builders SHOULD return, alongside the witness and payload, the projected post-operation opening and — for transfer-family circuits — the values the recipient will recover, both of which §10 needs.

---

## 8. Prover

### 8.1 Toolchain pinning

Proofs the deployed verifier accepts MUST be generated with the toolchain and non-default flags pinned in `circuits/vks/README.md`. Two flags govern acceptance:

- **A Keccak Fiat-Shamir transcript is mandatory.** The on-chain verifier reconstructs the transcript with Keccak, while proving backends commonly default to Poseidon2, and a default-transcript proof verifies locally then fails on-chain. Implementations MUST set the Keccak transcript for proof generation, local verification, and verification-key derivation alike.
- **Zero-knowledge mode MUST NOT be enabled** while the verifier implements only the non-zk flavour.

Circuit artifacts and verification keys MUST record the toolchain version that produced them. Version drift between a client's vendored artifacts and the deployment's pinned toolchain produces verification keys that differ from the deployed ones while every local test passes.

### 8.2 Verification-key identity

Before submitting a proof, an implementation MUST verify that the verification key its circuit artifact implies matches the one the deployment holds for that circuit type. A mismatch means the client and the chain disagree about what is being proven, and the proof cannot succeed.

Where a deployment permits verification-key rotation (DESIGN.md §3.5), an implementation MUST re-check rather than cache across sessions.

### 8.3 Backend pluggability and the trust boundary

Proving MUST sit behind an interface that admits multiple backends — in-process WASM, a native binary, a remote service — because the viable choice differs per platform. Browser bundlers, for instance, break worker-spawning proving libraries by rewriting the worker URL into a hashed chunk, so an implementation targeting browsers MUST allow the backend to be supplied by the host application rather than resolved statically.

**Witness material MUST NOT cross the trust boundary by default** (§2.1). Remote proving discloses the spending key, the balance, and the transfer amount to the prover. An implementation MAY offer it, but MUST require explicit opt-in, MUST NOT select it as a fallback when a local backend fails, and MUST state precisely which values leave the device.

---

## 9. Chain Adapter

### 9.1 Reads

The adapter MUST expose the account record and the spender delegation record (DESIGN_cont.md §11.3), and MUST treat the contract crate's types as authoritative for their shape. It MUST distinguish the three delegation states that DESIGN_cont.md §11.3 separates — absent, active, and expired-but-not-yet-revoked — and MUST NOT collapse the latter two.

Auditor keys MUST be read from the auditor contract by the account's bound `auditor_id`, never supplied by the caller.

### 9.2 Payload encoding

Proof-carrying entry points take an XDR-encoded payload. Its byte representation is fixed by Soroban's canonical XDR rules, so independent implementations compiling against the same `#[contracttype]` definitions produce byte-identical payloads (DESIGN_cont.md §11).

Two specifics:

- Points are flat 64-byte values, not nested two-field structures (§4.2).
- Struct fields serialise as a map with symbol keys in canonical sorted order.

### 9.3 Authorization and submission

Every state-changing operation requires both the appropriate `require_auth()` principal (DESIGN_cont.md §11.1) and, where applicable, a valid proof. The adapter MUST submit the principal the interface specifies — notably the *spender*, not the owner, for a delegated transfer.

Operations SHOULD be simulated before submission, which catches a stale commitment, a frozen account, or an expired delegation without a fee.

### 9.4 Typed errors

The adapter MUST surface contract failures as typed, distinguishable outcomes. At minimum it MUST separate:

| Class | Meaning | Caller's next step |
|:--|:--|:--|
| Proof verification failed | The proof did not verify against the assembled public inputs | Do not retry blindly; check §8.2 and witness assembly |
| State mismatch | The referenced commitment is no longer current | Re-sync (§10.2) and rebuild the proof |
| Not registered | The account or counterparty has no confidential account | Register, or reject the recipient |
| Compliance rejection | Frozen, policy-denied, or unauthorized by the underlying asset (COMPLIANCE.md §2) | Surface to the user as an administrative state, not an error |
| Delegation state | Absent, duplicate, or expired | Distinguish per §9.1 |

Separating the first two matters most: they present as the same opaque failure on the wire but have opposite remedies.

---

## 10. Holder Wallet

### 10.1 State model

The wallet maintains the two accumulators of DESIGN.md §5.2 — $$W_{\text{spend}}$$ and $$W_{\text{receive}}$$ — plus a sync position and any in-flight projection (§10.3). Values accumulate as exact integers; blindings accumulate modulo $$q$$ (§4.6).

Persistence MUST be pluggable, since the same core serves environments with very different storage. With RPC-only event access, discarding persisted state loses the receiving-side openings permanently (§10.9), so it MUST NOT be treated as an evictable cache.

### 10.2 Event application

Event application MUST be ordered, deduplicated, and idempotent in combination, and an implementation MUST state which layer discharges each obligation. The three are commonly split, with ordering and dedup at the event source and application left non-idempotent, which is conformant only if the split is explicit.

- **Ordering.** Events MUST be applied in emission order, because reconstruction is order-sensitive: a merge and a deposit in the same ledger produce different state depending on which is applied first. The canonical total order is INDEXER.md §3.4's $$(\text{ledger\\\_seq}, \text{tx\\\_application\\\_order}, \text{event\\\_index})$$.
- **Deduplication.** Events MUST be deduplicated by event id, since a hybrid source (§12.4) can deliver the same event twice at its seam.
- **Idempotence.** Either application is idempotent, or dedup provably precedes it. Crediting rules accumulate, so a duplicate inflates a balance.

Application rules are DESIGN.md §5.2's update table and are not restated here. Two properties worth making explicit:

- `Withdraw`, sender-side `Transfer`, `SetSpender`, and `RevokeSpender` **overwrite** $$W_{\text{spend}}$$ from the event's $$(\tilde{b}, \sigma)$$ rather than adjusting it, so a wallet that missed intervening events still converges on the spendable side.
- A self-transfer touches both accumulators in one event; the ordering within the rule matters.

### 10.3 In-flight operations

A wallet MAY project the post-operation opening immediately on successful submission rather than waiting for the event, so that the balance a user sees reflects an action they just took.

The projection MUST still be reconciled against the event, and MUST NOT be treated as confirmed for the purpose of §10.6.

### 10.4 Salt freshness

A fresh $$\sigma$$ MUST be sampled for every **attempt**, including retries after a reverted or dropped transaction.

DESIGN_cont.md §9.6 motivates this as unlinkability: a fresh $$\sigma$$ prevents an observer correlating a reverted attempt with its retry. It is equally a confidentiality requirement, because the salt is the sole freshness input to every derived pad in the operation, the ephemeral scalar included (DESIGN.md §2.5, §5.3). Reuse therefore repeats the ephemeral key and every channel mask that depends on it.

An implementation MUST NOT cache or reuse a salt across attempts, and MUST NOT derive it from anything an observer can predict.

### 10.5 Deterministic ephemeral scalars

An implementation MUST derive the ephemeral scalar of every operation the holder or spender originates:

$$r_e = \text{poseidon\\\_with\\\_domain}(\delta_{\text{eph}}, [vk, \sigma_E])$$

where $$vk$$ is the originator's viewing key and $$\sigma_E$$ the operation's salt. DESIGN.md §5.3 is the normative source: it fixes which viewing key and salt each operation derives from, and the retry rule for the negligible case that the derivation yields zero.

**Scope.** The clawback circuit is the one operation outside the rule, its ephemeral belonging to the auditor: an implementation that constructs clawback witnesses obtains that scalar by the §4.7 procedure, no viewing key being available there (COMPLIANCE.md §5.3).

**Three consequences an implementation MUST handle.**

First, **$$vk$$ carries more authority than balance decryption.** Recomputing $$r_e$$ yields the recipient shared scalar, hence $$r_{\text{transfer}}$$, hence a full Pedersen opening of every transfer commitment the account created — retroactively, and reaching commitments that sit inside recipients' receiving balances (DESIGN_cont.md §9.4, §8.2). An implementation MUST treat $$vk$$ accordingly in §13 and MUST NOT export it as a read-only credential without stating this.

Second, **the salt requirement of §10.4 is a confidentiality requirement**, not only an unlinkability one, the salt being the operation's sole freshness input.

Third, **disclosability is unverifiable from chain data.** No on-chain value distinguishes an ephemeral this derivation produced from one it did not, so an implementation MUST NOT infer disclosability from a stored per-transfer flag and MUST determine it by test (§12.2). Transfers predating this specification may not be disclosable by their sender.

### 10.6 Consistency checking

The wallet MUST verify its openings against on-chain commitments by re-committing and comparing (DESIGN.md §5.2 *Consistency check*), and MUST do so both after every sync and before constructing any proof. A missed event, a duplicate, an expired credit, or a defect then produces a mismatch rather than a plausible wrong balance, and a mismatched state MUST NOT be spent from.

Implementations MUST report which accumulator diverged, since the two have different causes and different remedies.

### 10.7 The unspendable-blinding case

A post-merge spendable blinding can land outside the range a Noir `Field` encodes, leaving no constructible proof against the affected commitment while on-chain state stays well-formed and §10.6's check still passes (DESIGN_cont.md §10.4 *Post-merge witness availability*).

An implementation MUST detect this condition and surface it as a distinct, named state rather than as a generic proof-construction failure.

It MUST also surface the recovery path: the condition resolves at the next merge that folds in an inbound confidential transfer, and an account whose only inflows are deposits stays affected until one arrives (DESIGN_cont.md §10.4 *Soft recovery*).

### 10.8 Merge policy

Received funds are spendable only after a merge (DESIGN.md §7.4). A wallet SHOULD merge automatically, or prompt, ahead of a spend that the spendable balance alone cannot cover. Merging ahead of a spend also bounds the recovery replay window, which starts at the last merge at or before the latest checkpoint (§10.9).

Merge is proof-less and owner-authorized, and neither a merge nor an in-flight spend proof can be disrupted by a third party (DESIGN_cont.md §9.1, §9.2).

### 10.9 Recovery

Recovery follows the procedure of DESIGN.md §5.2 *Recovery*, with the reconstructed state verified per §10.6. Its two anchors differ: the latest checkpoint event pins $$W_{\text{spend}}$$ in one lookup, while $$W_{\text{receive}}$$ restarts at $$T_0$$ — the account's last `Merge` at or before that checkpoint — from which the replay window runs.

Two further obligations follow from data availability:

- Recovery from a root alone depends on a conforming indexer (INDEXER.md). Without one, a client can see that funds exist but cannot reconstruct the opening needed to spend them.
- **With RPC-only event access, a client MUST sync at least once per RPC retention window**, and MUST warn when it has not. The spendable side is robust, since each checkpoint is self-contained, but the receiving side is a running sum from $$T_0$$, so a crediting event that ages out before it is applied takes its opening with it permanently.

### 10.10 Spender-side wallet

A spender reconstructs its allowance state from the on-chain delegation entry rather than from event replay: it recovers $$dvk_i$$ from the escrowed value by ECDH (DESIGN.md §7.11), then reads the current allowance from the entry's encrypted allowance and salt (DESIGN_cont.md §11.3).

Implementations MUST surface the delegation's expiry ledger and SHOULD warn ahead of it. They MUST represent expired-but-unrevoked delegations as still holding escrowed value (DESIGN.md §6.2).

A spender MUST NOT be able to reach the owner's spendable balance through any interface (§3).

---

## 11. Auditor Client

An auditor decrypts from the public event and its own secret $$k$$ alone, with no viewing key, holder cooperation, or extra on-chain read. For each channel it computes the shared scalar against the event's ephemeral point, derives the two lane masks (§4.3), and subtracts.

The two channels differ in what they yield (DESIGN_cont.md §8.1):

| Channel | Lane 0 | Lane 1 |
|:--|:--|:--|
| Sender / owner ($$\delta_{\text{aud\\\_s}}$$) | Transfer amount, or the escrowed amount for `SetSpender` and the reclaimed amount for `RevokeSpender` | Sender's post-operation balance, or post-operation allowance for a spender transfer |
| Recipient ($$\delta_{\text{aud\\\_r}}$$) | Transfer amount | Per-transfer Pedersen randomness $$r_{\text{transfer}}$$ |

`Withdraw`, `SetSpender`, and `RevokeSpender` carry a sender-channel balance checkpoint whose pad is lane **1**. Only `Withdraw` leaves lane 0 unused, its amount being public (DESIGN.md W_a3, §4.3); `SetSpender` and `RevokeSpender` read lane 0 as well, for the escrowed and reclaimed amounts respectively (DESIGN.md S_a4, V_a4).

**Cross-channel agreement.** Where an auditor holds the key for both parties, the amount decrypts independently on each channel and the circuit constrains both to the same value, so the two MUST agree. An implementation SHOULD perform this comparison and treat disagreement as evidence that $$k$$ is not the auditor key for both parties of that event.

**Scope MUST be represented, not implied.** The recipient-channel capability is forward-only, receiving-side only, and reset by merge (DESIGN_cont.md §8.1). Rotation itself needs no replay on the sender side: the next owner-initiated proof operation publishes a fresh balance checkpoint under the new key.

An auditor facade MUST NOT be able to construct a spending witness, and MUST NOT be able to open a post-merge spendable balance, since merge folds the receiving randomness into a blinding that depends on $$vk$$.

---

## 12. Disclosure and Indexer Clients

### 12.1 Disclosure construction

An implementation supporting selective disclosure MUST follow SELECTIVE_DISCLOSURE.md for the holder, sender, and auditor variants, and MUST bind each proof to the requesting recipient's key and nonce so that a proof cannot be replayed against a different recipient or a later request (SELECTIVE_DISCLOSURE.md §2.1).

Disclosure circuits are verified entirely off-chain and MUST NOT be registered with the on-chain verifier set (SELECTIVE_DISCLOSURE.md §15.1).

### 12.2 Disclosure verification

The verifier MUST be distributable independently of any wallet, since its purpose is to let a party who trusts no holder check a claim. It consumes a chain endpoint, the recipient keypair, and the proof bundle with its event reference, and returns the disclosed amount or a **typed** indication of which check failed — proof verification, on-chain state mismatch, or decryption failure. A single boolean is not conformant, because the three outcomes have different meanings to the recipient.

Verification MUST include comparing the circuit's verification key against the pinned key for that disclosure circuit, without which the proof attests to an unknown statement.

Not every historical transfer is disclosable by its sender: one predating §10.5's requirement may carry an ephemeral scalar that does not reproduce. An implementation MUST report that as *not disclosable* rather than as a verification failure, and MUST establish it by test: derive the candidate $$r_e$$ from $$(vk, \sigma_E)$$ and compare $$r_e \cdot H$$ against the event's $$R_e$$. The comparison costs one Poseidon2 call and one scalar multiplication and is authoritative, where a stored per-transfer flag is not (§10.5).

### 12.3 Indexer client

Recovery beyond the RPC retention window requires a conforming durable archive (INDEXER.md). A client MUST propagate the archive's completeness signal to its caller rather than swallowing it, since an incomplete range and a tampered range both end in the same refusal at §10.6 and only that signal distinguishes them.

Clients SHOULD support multiple independent archive endpoints, since withholding is the residual trust the archive retains (INDEXER.md §7).

### 12.4 The hybrid read path and its two failure modes

RPC and archive compose: the RPC serves the recent tail, the archive everything older, and the client stitches them at a seam (INDEXER.md §1). Two requirements are not derivable from INDEXER.md and are specified here.

**The seam MUST sit strictly above the RPC's reported retention floor, by a margin.** The floor advances as ledgers are collected, including between the moment the client reads it and the moment it issues the range query, with the archive request in between taking real time. A seam placed exactly at the observed floor therefore intermittently produces a rejected query. The archive covers everything below the seam, so a margin loses no events. Implementations MUST set the two legs to disjoint ledger ranges so that correctness does not depend on cross-source id equality, and MUST still deduplicate (§10.2) as a guard at the boundary.

**A configured archive's failure MUST fail the whole sync.** If an archive is configured and its request fails, an implementation MUST NOT degrade silently to RPC-only and MUST NOT persist a sync position derived from the RPC leg alone. Persisting it moves the position past the pre-window range, so every later sync takes the warm path that never consults the archive and the openings in the skipped range become unrecoverable. An archive that is *not* configured is a different case and MAY be absent, provided §10.9's warning is surfaced.

---

## 13. Security Requirements

**Secret handling.** The root, $$sk$$, $$vk$$, $$dvk_i$$, every derived $$r_e$$, and every cached opening are secrets. Implementations MUST keep them within the trust boundary (§2.1), SHOULD zeroize buffers holding them once no longer needed, and MUST NOT transmit them to any remote service except under §8.3's explicit opt-in.

$$vk$$ MUST NOT be presented as a safely-shareable read-only credential. It exposes every historical balance checkpoint, every incoming amount, every delegation allowance, and — through the ephemeral-scalar derivation of DESIGN.md §5.3 — the opening of every transfer the account originated. Its only guarantee is that it cannot authorize spending. A party that needs outbound visibility is served with D-sender proofs, which are bound to that party and to a nonce (SELECTIVE_DISCLOSURE.md §13.2), never by handing over the key.

**Storage at rest.** Persisted openings are as sensitive as the amounts they represent, and a persisted root is equivalent to the funds. Implementations MUST document what they persist and where, and SHOULD encrypt both at rest under a key that is not itself derived from persisted material. A signer root (§5.2) MUST NOT be persisted at all, since it is reproducible on demand from the signer.

**Logging and telemetry.** Witness material, decrypted amounts, balances, and openings MUST NOT reach logs, telemetry, analytics, error reports, or crash dumps.

**Randomness.** All sampling MUST use a cryptographically secure generator (§4.7). An implementation MUST NOT fall back to a non-CSPRNG source when one is unavailable; it MUST fail.

**Supply chain.** Soundness depends on the verification keys corresponding to the intended circuits and on the structured reference string having been generated honestly (DESIGN_cont.md §10.6). Implementations MUST pin verification keys and the proving toolchain (§8.1), SHOULD document how a user reproduces a deployed key from circuit source, and MUST NOT fetch either at runtime from a mutable location.

**What the SDK cannot do.** It cannot mitigate a compromised host: malicious code in the process, the bundle, or the storage layer reaches the root. Implementations MUST NOT present software-only custody as equivalent to hardware custody. Hardware custody is additionally constrained here, because proving requires $$sk$$ as a witness, so a device that does not prove internally must expose $$sk$$ to the host.

---

## 14. Non-Functional Requirements

**Proving latency.** Single-digit seconds on contemporary hardware is the design target (OVERVIEW.md). Implementations MUST treat it as user-visible.

**Sync and replay bounds.** The replay window runs from the account's last `Merge` at or before its latest checkpoint, or from registration for an account that has not merged before that checkpoint (DESIGN.md §5.2 *Recovery*), which is unbounded in age. Implementations MUST NOT assume a bounded window, and SHOULD use the archive's checkpoint lookup where available (INDEXER.md §6, C1) so that a dormant account is not obliged to transfer its entire history.

**Storage growth.** Per-account event volume is linear in inbound transfers and unbounded by design, since incoming-transfer spam is rate-limited only by transaction fees (DESIGN_cont.md §9.5). Implementations MUST NOT size local storage on the assumption that inbound volume tracks the user's own activity.

**Offline capability.** Key derivation, witness assembly, and proof generation need no network. Balance display needs synced state. Submission and consistency checking need the chain. Implementations SHOULD make the boundary explicit rather than failing opaquely when offline.

**Version matrix.** An implementation MUST expose, for inspection and for bug reports: the protocol documentation version it targets, the contract address and its `addr_f`, the verification-key set and producing toolchain versions (§8.1), and the domain-separator scheme in use (§4.8, including whether the alternate hashed scheme of DESIGN_cont.md §13 was selected). Each is an interoperability boundary whose mismatch is undiagnosable without knowing its value.

---

## 15. Conformance and Versioning

An implementation conforms to this specification iff it:

1. reproduces every vector of §6.1 byte-for-byte, reading the fixture files rather than transcribing them;
2. passes circuit-execution parity with tamper rejection (§6.2) for every circuit it supports;
3. satisfies §4–§12 for the roles it implements;
4. exposes the version matrix of §14.

Partial role coverage is conformant and MUST be declared: an implementation supporting only the holder role is conformant for that role, and MUST NOT claim spender, auditor, or disclosure conformance. Partial *circuit* coverage MUST likewise be declared.

**The portable surface is §4–§7.** The crypto core, key derivation, and witness assembly are what two implementations must agree on byte-for-byte. The facades of §10–§12 are deployment-shaped, and this document constrains their obligations rather than their structure.

This document is versioned with the protocol documentation set. A change to any primitive in §4, to the derivation in §5, to the ephemeral-scalar derivation (DESIGN.md §5.3, restated in §10.5), or to the domain separators in §4.8 breaks the cross-implementation contract: it MUST bump the protocol documentation version, MUST be called out in release notes, and MUST be accompanied by updated fixtures in `circuits/lib/testdata/`. A change to the numeric value of a domain separator, or to the sponge convention, invalidates every previously derived key and every previously emitted ciphertext, and is a new deployment rather than an upgrade.
