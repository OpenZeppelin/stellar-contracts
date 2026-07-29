# Confidential Token: SDK

Companion specification to [DESIGN.md](./DESIGN.md) §4 (Key Hierarchy) and §5.2 (Off-Chain Opening Maintenance), [DESIGN_cont.md](./DESIGN_cont.md) §9.5 (State Recovery) and §11 (Interface), [INDEXER.md](./INDEXER.md), and [SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §15. It specifies the client layer those documents assume: the crypto core that mirrors the Noir circuits off-chain, the key derivation the protocol leaves open, the witness and payload construction, the wallet state machine, and the auditor, disclosure, and indexer clients.

The key words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are to be interpreted as in RFC 2119. The normative audience is threefold:

- **SDK implementers** MUST satisfy §4–§12.
- **Wallet, auditor, and application integrators** MUST NOT bypass §13; the security properties of the protocol do not survive it.
- **Port authors** (mobile, hardware wallet, a second language) MUST pass §6 and expose §15.

**Scope.** This document specifies obligations, not an API. It does not prescribe function signatures, module names, package layout, or class design, and it does not restate protocol formulas that [DESIGN.md](./DESIGN.md) already fixes — each requirement cites its source section instead.

---

## 1. Why the SDK Is Load-Bearing

Four properties of the protocol place correctness and confidentiality in the client rather than in the contract.

**The opening exists only off-chain.** A balance is a Pedersen commitment $$C = v \cdot G + r \cdot H$$. The chain stores the point; the opening $$(v, r)$$ that authorizes the next spend lives exclusively in client state (DESIGN.md §5.2). A client that loses, mis-derives, or mis-accumulates the opening makes the funds unspendable, and the contract cannot help because it never knew the value.

**Every amount a user sees is client-decrypted.** The contract performs homomorphic point arithmetic and never learns a value. Balances, transfer amounts, allowances, and audit figures are all produced by client-side decryption of event ciphertexts, so a decryption defect yields a plausible wrong number rather than a visible failure.

**The client is the only enforcement point for canonicality.** The Soroban host's `bn254_fr_from_u256val` silently reduces any 32-byte representative $$x \geq r$$ modulo $$r$$ rather than rejecting it (DESIGN.md §2.2, *Host deserialiser caveat*). Two distinct byte strings therefore deserialise to the same field element, and a verifier alone cannot tell canonical from non-canonical input. The contract enforces canonicality at its boundary, but the client is where the bytes are produced.

**The client is where all secrets live and all randomness is sampled.** The spending key, the viewing key, every blinding factor, and every per-operation salt originate client-side, so the protocol's confidentiality reduces to the client's key handling and CSPRNG quality. DESIGN.md §2.5 makes salt uniqueness a soundness requirement.

---

## 2. Terminology and Layering

### 2.1 Terminology

- **Root** — the secret an implementation feeds to §5.1's derivation: a BIP-39 seed, a SEP-0053 signature by a signer on the account, or a raw 32-byte value (§5).
- **Opening** — the pair $$(v, r)$$ such that $$C = v \cdot G + r \cdot H$$ for an on-chain commitment $$C$$.
- **Checkpoint** — an owner-initiated proof-carrying event publishing $$(\tilde{b}, \sigma)$$ for the owner's spendable balance: `Withdraw`, sender-side `Transfer`, `SetSpender`, `RevokeSpender` (DESIGN_cont.md §9.5). Defined identically in INDEXER.md §2.
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
- The SDK MUST reject a non-canonical value at its own boundary rather than relying on the contract's check. The contract does check, but a client that produces non-canonical bytes has already lost byte-uniqueness in the local state that recovery reads from.
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

Reducing modulo $$r$$ instead yields an opening that is off by $$q - r$$ and no longer matches the on-chain point, and for two full-size blindings the integer sum crosses $$q$$ roughly half the time. Implementations MUST provide distinct, clearly named addition operations for the two moduli and MUST use the $$\mathbb{F}_q$$ one for every blinding accumulation: merge (DESIGN.md §7.4) and receiving-balance credit (§5.2 *Update rules*).

Committed **values** accumulate as exact integers and MUST NOT be reduced by either modulus; DESIGN.md §2.3 establishes that they never wrap.

### 4.7 Scalar sampling

Secret scalars — $$sk$$, $$r_e$$ when sampled, $$\sigma$$, $$\sigma_a$$ — MUST be produced by the rejection procedure of DESIGN.md §2.2:

1. Draw 32 bytes from a CSPRNG.
2. Clear the top **2** bits, yielding a 254-bit candidate.
3. Reject and redraw if the candidate is $$\geq r$$, or if it is zero and the call site requires nonzero.

### 4.8 Domain separators

| Tag | Value | Absorbed in a core circuit? |
|:--|:--:|:--|
| $$\delta_{\text{addr}}$$ | 1 | Yes |
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
| $$\delta_{\text{disc\\\_bind}}$$ | 14 | No — off-chain disclosure only |
| $$\delta_{\text{eph}}$$ | 15 | No — client convention (§10.5) |
| $$\delta_{\text{disc}}$$ | 16 | No — off-chain disclosure only |

Values 1–13 are defined in DESIGN_cont.md §13., SELECTIVE_DISCLOSURE.md §2.2 introduces the remaining three without assigning numbers, and this document assigns them 14–16. Tags 14–16 are never absorbed inside a core circuit, so they are not part of the on-chain wire contract, but they are part of the cross-client contract because two wallets serving the same account must agree on them (§6.3).

All sixteen values MUST be distinct, and each MUST be used in exactly one sponge mode, per DESIGN.md §2.5 *Mode exclusivity*. Tags 11 and 12 are the two-mask tags; the remaining fourteen, including 14–16, are single-output tags.

### 4.9 Address compression

$$\text{address\\\_to\\\_field}(a) = \text{poseidon\\\_with\\\_domain}(\delta_{\text{addr}}, [\text{lo}(a), \text{hi}(a)])$$

where $$\text{enc}(a)$$ is the 56-character ASCII strkey (SEP-23), and $$\text{lo}$$ and $$\text{hi}$$ interpret its lower and upper 28 bytes respectively in **little-endian** order (DESIGN.md §2.7). Implementations MUST obtain the strkey from their language's stellar-strkey library.

**Bootstrap check.** On first contact with a deployment, an implementation MAY compute $$\text{addr\\\_f}$$ for the contract's own address and assert equality against the value the contract stores in instance storage (DESIGN.md §3.5).

---

## 5. Key Derivation

DESIGN.md §4 specifies the hierarchy below $$sk$$ — $$vk$$ from $$(sk, \text{addr\\\_f})$$, $$\text{PVK}$$ from $$vk$$, $$dvk_i$$ from $$(vk, \text{op}_i)$$. It does not specify where $$sk$$ itself comes from. This section supplies a derivation, because recovery from a seed is a stated protocol property (DESIGN.md §5.2 *Recovery*, DESIGN_cont.md §9.5, INDEXER.md §1) and two clients given the same backup material would otherwise derive different accounts.

The derivation is a single function (§5.1) over a **root**. For an account controlled by a Stellar ed25519 key that root may come from either of two classes: the BIP-39 seed the account's key was itself derived from (§5.3), or a deterministic signature by the key that controls the account (§5.4). §5.2 states which to choose and what each costs.

### 5.1 Derivation

$$sk = \text{RS}\Big(\text{HKDF-SHA-512}\big(\text{ikm} = \text{root}, \\;\\; \text{salt} = \texttt{"openzeppelin/confidential-token/v1/sk"}, \\;\\; \text{info} = \text{be}_{32}(\text{addr\\\_f}) \\,\\|\\, \text{be}_{32}(\text{acct\\\_f}) \\,\\|\\, \text{le}_{4}(j)\big)\Big)$$

where:

| Input | Definition |
|:--|:--|
| $$\text{root}$$ | The account's root, per its class: a 64-byte BIP-39 seed (§5.3), a 64-byte signer signature (§5.4), or a raw 32-byte value (§5.5) |
| $$\text{addr\\\_f}$$ | $$\text{address\\\_to\\\_field}$$ of the confidential token contract (§4.9) |
| $$\text{acct\\\_f}$$ | $$\text{address\\\_to\\\_field}$$ of the address being registered |
| $$j$$ | Rejection counter, starting at 0 |
| $$\text{RS}$$ | The §4.7 procedure applied to the 32-byte HKDF output: clear the top 2 bits, accept iff the result is in $$[1, r)$$, otherwise increment $$j$$ and re-derive |

The candidate MUST also be rejected if the resulting $$vk = \text{poseidon\\\_with\\\_domain}(\delta_{\text{vk}}, [sk, \text{addr\\\_f}])$$ is zero, since registration constraint R5 requires $$vk \neq 0$$.

Why each element is present:

**HKDF-SHA-512, not Poseidon2.** No circuit constrains how $$sk$$ was obtained — the register circuit constrains only $$Y = sk \cdot H$$ (R1) and $$vk$$'s derivation from $$sk$$ (R2). There is therefore no in-circuit consistency argument for Poseidon2 here, and SHA-512 keeps the seed-custody path on a primitive that every BIP-39 library and secure element already implements. Implementations MUST NOT substitute a different KDF, since the choice is arbitrary in isolation but must be identical across clients.

**Bound to $$\text{addr\\\_f}$$.** $$vk$$ is already deployment-scoped by DESIGN.md §4.2, which bounds the blast radius of a *viewing*-key compromise to one deployment. Binding $$sk$$ likewise bounds a *spending*-key compromise, and the contract address is known whenever a client talks to a deployment.

**Bound to $$\text{acct\\\_f}$$.** $$vk$$ depends only on $$(sk, \text{addr\\\_f})$$, so the same $$sk$$ registered under two addresses yields the same $$vk$$, hence identical $$Y$$ and identical $$\text{PVK}$$ published under both accounts and readable by any observer through the account read method, linking two addresses that are otherwise unlinkable. Implementations MUST derive a distinct $$sk$$ per address.

**No account-index input.** Binding $$\text{acct\\\_f}$$ makes a separate SEP-0005 index redundant: the address determines the account, and the index is merely the path that produced the address. Implementations MUST NOT introduce one.

**One salt across all root classes.** The classes differ in their ikm, and a signer root's domain separation lives in the message it signs (§5.4) rather than in the salt, so a per-class salt would separate nothing the ikm does not already separate. The class is recorded per account (§5.5) because the classes differ in what regenerates the key, not because the derivation needs to tell them apart.

### 5.2 Choosing a root class

For an account whose key came from a mnemonic, both classes reproduce $$sk$$ from that mnemonic, so neither forfeits seed recovery; a signer root additionally covers accounts with no mnemonic behind them. The classes differ in what the confidential client must hold and in whether the ed25519 signing key becomes a single point of failure for confidential funds.

**The rule.** An implementation that already holds the mnemonic for an account SHOULD use a seed root: a signer root buys nothing there, because the seed is in the process either way, and it costs the coupling described below. An implementation that does not hold the mnemonic, and should not — an account fronted by a hardware wallet, or one that exists only as an exported `S…` secret with no mnemonic behind it — SHOULD use a signer root. An implementation MAY support both, and MUST record which produced each account (§5.5).

| | Seed root (§5.3) | Signer root (§5.4) |
|:--|:--|:--|
| ikm | 64-byte BIP-39 seed | 64-byte SEP-0053 signature |
| Requires of the custody stack | mnemonic or seed export | SEP-0053 message signing |
| What the confidential client holds | authority over every account and asset under the mnemonic | authority over one confidential account on one deployment |
| If the ed25519 secret leaks | confidential balances unaffected | view and spend of the confidential account, unrecoverably |
| Reproducible from | the mnemonic and its passphrase | the enrolled signer, or the mnemonic and passphrase plus the recorded SEP-0005 index |
| Survives signer rotation | yes | no |
| Needs a deterministic signer | no | yes — excludes randomised and threshold ed25519 signers |
| Account discovery (§5.6) | offline, full index scan | one signature per candidate index |

**The coupling is the whole cost of a signer root.** Whoever obtains the ed25519 secret can recompute the signature and therefore $$sk$$, gaining both view and spend of the confidential account. Stellar's operational culture makes this live rather than theoretical: exporting and pasting an `S…` secret is a routine act, and the confidential account does not recover from it. `register` is single-use (DESIGN_cont.md §11), so $$sk$$ cannot be rotated in place; remediation means registering a fresh address and moving the funds through a transfer that links the old address to the new one. Under a seed root the signing key can leak with no consequence for confidential balances, because the two keys are siblings rather than ancestor and descendant.

**The host exposure is the whole benefit of a signer root.** §13 establishes that $$sk$$ is necessarily host-resident: proving takes it as a private witness, so a device that does not prove internally must hand it to the host. Hardware custody therefore cannot protect $$sk$$ under either class; what it can still protect is the mnemonic. A seed root puts authority over every account and every asset under that mnemonic into the confidential client's process, while a signer root puts one 64-byte value there whose authority is one confidential account on one deployment. A seed root consequently requires a hardware-wallet user to put the mnemonic into software, exposing every account under it to a client that needs one.

**Neither class dominates.** A seed root minimises credential-leak risk; a signer root minimises host-exposure risk. Which of the two is live is a property of how the account is already held, so the choice is keyed to custody shape rather than settled here for every deployment.

### 5.3 Seed roots

A seed root is the 64-byte BIP-39 seed produced from a mnemonic and optional passphrase — the same seed SEP-0005 derives account keys from. Implementations SHOULD accept 24-word mnemonics (256-bit entropy) and MAY accept 12-word.

The seed itself is the ikm, never a SLIP-0010 child of it. Deriving $$sk$$ beneath `m/44'/148'/i'` would place it under the ed25519 account key rather than beside it, since SLIP-0010 hardened children are derivable from the parent *private* key. That reproduces §5.2's coupling without the SEP-0053 envelope that bounds a harvested signature to one account, and without the enrolment record that makes the coupling visible to recovery. Implementations MUST NOT derive a seed root from any node of the SEP-0005 path.

### 5.4 Signer roots

A signer root is a SEP-0053 signature over a message naming this protocol, the deployment, and the account:

$$\text{msg} = \texttt{"openzeppelin/confidential-token/v1/sk"} \\,\\|\\, \texttt{0x0a} \\,\\|\\, \text{enc}(\text{contract}) \\,\\|\\, \texttt{0x0a} \\,\\|\\, \text{enc}(\text{account})$$

$$\text{root} = \text{Ed25519-Sign}\big(sk_{\text{ed}}, \\;\\; \text{SHA-256}(\text{prefix} \\,\\|\\, \text{msg})\big)$$

where `prefix` is SEP-0053's 24 ASCII bytes `Stellar Signed Message:\n`, $$\text{enc}$$ is the 56-character strkey of §4.9, and $$sk_{\text{ed}}$$ is the ed25519 secret of a signer on the account. The message is 151 bytes, printable ASCII apart from its two separators, and carries the strkeys rather than their §4.9 compressions so that a wallet rendering SEP-0053 messages as text shows the user addresses they can compare against the deployment they intend to register on. The 64-byte signature is HKDF input material unchanged.

Binding both addresses into the *message* rather than relying on §5.1's `info` alone is what bounds a harvested signature: a dapp that tricks a user into signing once obtains the root for that account on that deployment, not for every account the key controls on every deployment.

**The SEP-0053 envelope is mandatory even where the secret is extractable.** A client holding the raw ed25519 secret MUST compute this signature itself rather than use the secret as ikm directly. One form then covers both custody shapes: an account enrolled through a wallet prompt is reproducible by a client that later imports the secret, and the reverse.

**Availability is not guaranteed.** A signer root exists only where the custody stack implements SEP-0053 message signing, and support across Stellar wallets and hardware apps is uneven. An implementation MUST treat its absence as an expected outcome and fall back to §5.3 or §5.5, not fail enrolment.

**Verify the signature before using it.** An implementation MUST verify the returned signature against the ed25519 public key it expects to have signed, and MUST abort on mismatch. A wallet with a different account selected returns a well-formed signature over the same message, yielding a wrong but entirely usable $$sk$$: registration succeeds, and the account is then unreproducible from the key the user believes controls it.

**Determinism is a precondition.** RFC 8032 ed25519 derives its nonce from the secret and the message, so a conforming signer returns the same 64 bytes forever. Signers that randomise the nonce do not, and threshold and MPC ed25519 are in that category — the nonce is generated per signing session, so a signature does not reproduce in the next one. An implementation MUST obtain the signature twice from independent invocations and MUST abort if they differ. That detects the common case and not every case, since a signer can be deterministic within a session and not across sessions, so an implementation MUST additionally offer $$sk$$ export as a direct-import backup (§5.5) and SHOULD prompt for it before the account first receives funds.

**The ikm MUST NOT be persisted.** The 64-byte signature is equivalent to $$sk$$ for this account and deployment. Implementations MUST derive on demand; where $$sk$$ itself is cached, §13's storage-at-rest rules govern it.

**Record the enrolled signer.** A Stellar address may have signers besides its master key, and $$sk$$ is keyed to the *address* through $$\text{acct\\\_f}$$ rather than to the key that signed for it, so which signer enrolled is not recoverable from the address or from chain state. Implementations MUST record the enrolled ed25519 public key and MUST NOT assume it is the master key. A second client enrolling the same address with a different signer derives a different $$sk$$ whose $$Y$$ does not match the registered spending public key; §5.6's comparison detects that and cannot repair it.

**Signer rotation orphans the account.** `set_options` can replace or remove the key controlling an address, and the confidential account survives that rotation while its root does not. An implementation SHOULD compare the recorded signer against the address's current signer set on sync and surface a warning when the enrolled key is no longer among them, because discarding that key after rotation destroys the only seedless path back to $$sk$$.

**Discovery degrades.** §5.6's scan derives $$sk$$ once per candidate index, which under a signer root means one signature per candidate — a user approval each on a hardware wallet, and nothing at all for an index whose key the custody stack will not sign with. An implementation MUST NOT present signer-root discovery as equivalent to seed-root discovery.

### 5.5 Raw roots and imported keys

Implementations MUST also accept a raw 32-byte `root`, used unchanged as HKDF input material. Two cases require it:

- **Contract addresses.** A confidential account registered by a smart account or other contract address has no mnemonic and no ed25519 signer of its own, and its root comes from whatever custody mechanism controls the contract.
- **Imported keys.** Deployments predating this specification hold $$sk$$ values sampled directly from a CSPRNG with no root behind them. Such a key MUST remain usable as a first-class account secret via direct import, bypassing §5.1 entirely.

An implementation MUST record, per account, which of the four forms produced its $$sk$$ — seed root, signer root, raw root, or direct import — because they differ in what regenerates the key: backup words, a live signer, a stored 32-byte value, or nothing beyond the stored $$sk$$. A user MUST NOT be shown a recovery-phrase affordance for an account whose key cannot be regenerated from a phrase, and a signer-root account MUST NOT be presented as phrase-recoverable unless its enrolled SEP-0005 index is recorded alongside the signer (§5.4).

### 5.6 Recovery and account discovery

Recovering a seed-root account family requires the mnemonic, its passphrase if any, and the contract address. The set of addresses is discovered rather than remembered:

1. Enumerate candidate Stellar addresses by scanning SEP-0005 indices $$i = 0, 1, 2, \ldots$$ from the seed.
2. For each candidate address, compute $$\text{acct\\\_f}$$, derive $$sk$$ per §5.1, and compute $$Y = sk \cdot H$$.
3. Read the account record at that address and compare its stored spending public key against $$Y$$. A match identifies a registered confidential account belonging to this root.

Implementations exposing this scan MUST pin a gap limit — a number of consecutive unregistered indices after which scanning stops — and MUST surface it, since an account beyond the limit is invisible to recovery even though its funds exist. Scanning MUST NOT be presented as exhaustive.

Recovering a signer-root account requires the enrolled signer, either directly or by re-deriving it from the mnemonic at the recorded SEP-0005 index (§5.5). Step 2 then costs one signature per candidate rather than one hash, so an implementation SHOULD drive signer-root recovery from its own per-account record and SHOULD NOT scan blindly. An account whose enrolled signer is neither held nor re-derivable is unrecoverable, and only the direct import of a previously exported $$sk$$ reaches it.

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

Three derivations this document specifies or relies on have no fixture in `circuits/lib/testdata/`, because none of them is computed inside a core circuit. Each requires one:

- **$$\text{address\\\_to\\\_field}$$** (§4.9). The circuits receive $$\text{addr\\\_f}$$ as an opaque public input, so this derivation is implemented twice — by the contract on-chain and by every client — and the existing fixtures pin $$\text{addr\\\_f}$$ only as a fixed constant. It is the sole primitive with two independent implementations, and §4.9's bootstrap check detects a divergence only against an already-deployed contract.
- **$$\delta_{\text{eph}}$$ derivation** (§10.5). Nothing in any circuit constrains $$r_e$$, so a fixture is the only mechanism keeping a user's clients in agreement; where they disagree, transfers sent from one are not disclosable from another.
- **The §5.1 $$sk$$ chain**, from a fixed root, $$\text{addr\\\_f}$$, and $$\text{acct\\\_f}$$ through to $$sk$$, $$vk$$, $$Y$$, and $$\text{PVK}$$, without which recovery from backup material is untestable across implementations. Each root class needs one: the seed-root vector from a fixed mnemonic and passphrase, and the signer-root vector from a fixed ed25519 secret through §5.4's message, its SEP-0053 preimage, and the resulting signature. The signature step is where two clients most plausibly diverge, being the only step in the chain whose format is set outside this document.

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

Proving MUST sit behind an interface that admits multiple backends — in-process WASM, a native binary, a remote service — because the viable choice differs per platform and because proving-library packaging is itself a portability hazard. Browser bundlers, for instance, break worker-spawning proving libraries by rewriting the worker URL into a hashed chunk, so an implementation targeting browsers MUST allow the backend to be supplied by the host application rather than resolved statically.

**Witness material MUST NOT cross the trust boundary by default** (§2.1). Remote proving discloses the spending key, the balance, and the transfer amount to the prover. An implementation MAY offer it, but MUST require explicit opt-in, MUST NOT select it as a fallback when a local backend fails, and MUST state precisely which values leave the device.

### 8.4 Latency

Proof generation is a multi-second, CPU-bound, user-blocking operation. Implementations MUST expose progress and cancellation, and MUST NOT hold a lock over wallet state for the duration, since an incoming transfer arriving mid-proof is expected and harmless (DESIGN_cont.md §9.1).

Backends SHOULD be constructed once per circuit and reused; initialisation loads the proving system and its reference string and dominates the cost of a single proof.

---

## 9. Chain Adapter

### 9.1 Reads

The adapter MUST expose the account record and the spender delegation record (DESIGN_cont.md §11.3), and MUST treat the contract crate's types as authoritative for their shape. It MUST distinguish the three delegation states that DESIGN_cont.md §11.3 separates — absent, active, and expired-but-not-yet-revoked — and MUST NOT collapse the latter two, since an expired delegation still holds escrowed funds that only revocation reclaims.

Auditor keys MUST be read from the auditor contract by the account's bound `auditor_id`, never supplied by the caller.

### 9.2 Payload encoding

Proof-carrying entry points take an XDR-encoded payload. Its byte representation is fixed by Soroban's canonical XDR rules, so independent implementations compiling against the same `#[contracttype]` definitions produce byte-identical payloads (DESIGN_cont.md §11).

Payloads MUST be produced by encoding those definitions. Implementations SHOULD generate the encoder from the contract's interface rather than hand-assembling the structure, and where hand-assembly is unavoidable MUST pin it with a round-trip test against the contract's own decoder. Two specifics:

- **Points are flat 64-byte values**, not nested two-field structures (§4.2). The nested form encodes without error and decodes to nothing usable.
- **Struct fields serialise as a map with symbol keys in canonical sorted order.** Hand-sorting with a host language's default string comparison agrees with the canonical order for the current lowercase-and-underscore field names, but nothing preserves that agreement across a field rename.

### 9.3 Authorization and submission

Every state-changing operation requires both the appropriate `require_auth()` principal (DESIGN_cont.md §11.1) and, where applicable, a valid proof. The adapter MUST submit the principal the interface specifies — notably the *spender*, not the owner, for a delegated transfer.

Operations SHOULD be simulated before submission, which catches a stale commitment, a frozen account, or an expired delegation without a fee.

### 9.4 Typed errors

The adapter MUST surface contract failures as typed, distinguishable outcomes rather than opaque host errors. At minimum it MUST separate:

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

- **Ordering.** Events MUST be applied in emission order, because reconstruction is order-sensitive: a merge and a deposit in the same ledger produce different state depending on which is applied first. The canonical total order is INDEXER.md §3.4's $$(\text{ledger\\\_seq}, \text{tx\\\_application\\\_order}, \text{event\\\_index})$$. Implementations MUST NOT order by event id string, whose components are not ordering keys.
- **Deduplication.** Events MUST be deduplicated by event id, since a hybrid source (§12.4) can deliver the same event twice at its seam.
- **Idempotence.** Either application is idempotent, or dedup provably precedes it. Crediting rules accumulate, so a duplicate inflates a balance.

Application rules are DESIGN.md §5.2's update table and are not restated here. Two properties worth making explicit:

- Sender-side `Withdraw` and `Transfer` **overwrite** $$W_{\text{spend}}$$ from the event's $$(\tilde{b}, \sigma)$$ rather than adjusting it, so a wallet that missed intervening events still converges on the spendable side.
- A self-transfer touches both accumulators in one event; the ordering within the rule matters.

### 10.3 In-flight operations

A wallet MAY project the post-operation opening immediately on successful submission rather than waiting for the event, so that the balance a user sees reflects an action they just took.

The projection MUST still be reconciled against the event, and MUST NOT be treated as confirmed for the purpose of §10.6.

### 10.4 Salt freshness

A fresh $$\sigma$$ MUST be sampled for every **attempt**, including retries after a reverted or dropped transaction.

DESIGN_cont.md §9.6 motivates this as unlinkability: a fresh $$\sigma$$ prevents an observer correlating a reverted attempt with its retry. Under §10.5 it also becomes a confidentiality requirement, because $$\sigma$$ is then the sole freshness input to every derived pad in the operation. DESIGN.md §2.5 already requires the pair $$(r_e, \sigma)$$ to be unique per proof; with $$r_e$$ derived from $$\sigma$$ that reduces to $$\sigma$$ alone, and reuse repeats the ephemeral key and every channel mask that depends on it.

An implementation MUST NOT cache or reuse a salt across attempts, and MUST NOT derive it from anything an observer can predict.

### 10.5 Deterministic ephemeral scalars

The ephemeral scalar for an outgoing transfer SHOULD be derived rather than sampled:

$$r_e = \text{poseidon\\\_with\\\_domain}(\delta_{\text{eph}}, [vk, \sigma_E])$$

where $$vk$$ is the originator's viewing key and $$\sigma_E$$ the operation's salt (SELECTIVE_DISCLOSURE.md §7, §15.2). The derivation MUST be re-attempted with a fresh salt in the negligible case that it yields zero, which the circuits forbid.

**Why.** No circuit constrains $$r_e$$ beyond $$R_e = r_e \cdot H$$ and $$r_e \neq 0$$, so deriving it changes nothing on-chain. It lets the originator recompute $$r_e$$ for any past outgoing transfer from $$vk$$ and the event's public salt, so sender-side disclosure needs no per-transfer state. The alternative is retaining $$(r_e, v_{\text{transfer}})$$ for every outbound transfer indefinitely, and a transfer whose randomly-sampled $$r_e$$ was not retained is permanently undisclosable.

**Three consequences an implementation MUST handle.**

First, **it widens the viewing-key blast radius, retroactively.** DESIGN_cont.md §9.4 states that a $$vk$$ holder cannot construct openings of any commitment. Under this convention a $$vk$$ holder recomputes $$r_e$$, hence the recipient shared scalar, hence $$r_{\text{transfer}}$$ — a full opening of every transfer commitment the account ever created, including transfers predating the compromise, and those commitments sit inside recipients' receiving balances. The *amounts* were already inferable by differencing checkpoints and netting merges, so that capability is not new; the openings are, and they extend a capability DESIGN_cont.md §8.2 otherwise scopes to the recipient's auditor (§16.1). An implementation MUST treat $$vk$$ accordingly in §13 and MUST NOT export it as a read-only credential without stating this.

Second, **the salt requirement of §10.4 is promoted from unlinkability to confidentiality.**

Third, **disclosability is unverifiable on-chain.** Nothing distinguishes a derived $$r_e$$ from a sampled one, so a client cannot determine whether a given historical transfer is disclosable without attempting it, and a user moving between clients can accumulate a mixed history. An implementation that supports both MUST record, per account, the ledger range over which the derived convention applied, and MUST use that record when presenting disclosability (§12.2). An implementation offering a sampled-$$r_e$$ path MUST either retain $$(r_e, v_{\text{transfer}})$$ per outbound transfer or state that those transfers are permanently undisclosable.

### 10.6 Consistency checking

The wallet MUST verify its openings against on-chain commitments by re-committing and comparing (DESIGN.md §5.2 *Consistency check*), and MUST do so both after every sync and before constructing any proof. A missed event, a duplicate, an expired credit, or a defect then produces a mismatch rather than a plausible wrong balance, and a mismatched state MUST NOT be spent from.

Implementations MUST report which accumulator diverged, since the two have different causes and different remedies.

### 10.7 The unspendable-blinding case

After a merge the spendable blinding is $$r_s + r_r$$ over $$\mathbb{F}_q$$. With probability approximately $$2^{-127}$$ per merge its canonical representative lands in $$[r, q)$$, where it remains a valid Grumpkin scalar — so on-chain state is well-formed and §10.6's check still passes — but is not encodable as a Noir `Field`, so no proof can be constructed against the affected commitment (DESIGN_cont.md §10.4).

An implementation MUST detect this condition and surface it as a distinct, named state rather than as a generic proof-construction failure.

It MUST also surface the recovery path: every subsequent inbound confidential transfer contributes a fresh $$\mathbb{F}_r$$ blinding, so the next merge resolves the condition with overwhelming probability. An account whose only inflows are deposits stays affected until a confidential transfer arrives.

### 10.8 Merge policy

Received funds are spendable only after a merge (DESIGN.md §7.4). A wallet SHOULD merge automatically, or prompt, ahead of a spend that the spendable balance alone cannot cover.

Merge is proof-less and owner-authorized. Incoming transfers touch only the receiving balance, so they cannot invalidate an in-flight spend proof, and no third party can front-run a merge (DESIGN_cont.md §9.1). Implementations MUST NOT introduce a spend path that references the receiving balance directly, and MUST NOT gate spending on the absence of incoming activity.

### 10.9 Recovery

Recovery follows DESIGN.md §5.2 and DESIGN_cont.md §9.5: locate the latest checkpoint, recover $$W_{\text{spend}}$$ from its $$(\tilde{b}, \sigma)$$ and $$vk$$, then replay subsequent crediting and merge events to rebuild $$W_{\text{receive}}$$, and verify per §10.6.

Two obligations follow from data availability:

- Recovery from a root alone depends on a conforming indexer (INDEXER.md). Without one, a client can see that funds exist but cannot reconstruct the opening needed to spend them.
- **With RPC-only event access, a client MUST sync at least once per RPC retention window**, and MUST warn when it has not. The spendable side is robust, since each checkpoint is self-contained, but the receiving side is a running sum, so a crediting event that ages out before it is applied takes its opening with it permanently. Implementations MUST NOT present RPC-only operation as equivalent to indexed operation.

### 10.10 Spender-side wallet

A spender reconstructs its allowance state from the on-chain delegation entry rather than from event replay: it recovers $$dvk_i$$ from the escrowed value by ECDH (DESIGN.md §7.11), then reads the current allowance from the entry's encrypted allowance and salt (DESIGN_cont.md §11.3).

Implementations MUST surface the delegation's expiry ledger and SHOULD warn ahead of it, since a spender transfer after expiry is rejected. They MUST represent expired-but-unrevoked delegations as still holding escrowed value, since deleting the entry would destroy the escrow and only the owner's revocation reclaims it.

A spender MUST NOT be able to reach the owner's spendable balance through any interface (§3).

---

## 11. Auditor Client

An auditor decrypts from the public event and its own secret $$k$$ alone, with no viewing key, holder cooperation, or extra on-chain read. For each channel it computes the shared scalar against the event's ephemeral point, derives the two lane masks (§4.3), and subtracts.

The two channels differ in what they yield (DESIGN_cont.md §8.1):

| Channel | Lane 0 | Lane 1 |
|:--|:--|:--|
| Sender / owner ($$\delta_{\text{aud\\\_s}}$$) | Transfer amount | Sender's post-operation balance, or post-operation allowance for a spender transfer |
| Recipient ($$\delta_{\text{aud\\\_r}}$$) | Transfer amount | Per-transfer Pedersen randomness $$r_{\text{transfer}}$$ |

`Withdraw`, `SetSpender`, and `RevokeSpender` carry a sender-channel checkpoint whose pad is lane **1**; lane 0 is unused because the amount is public or separately carried (§4.3).

**Cross-channel agreement.** Where an auditor holds the key for both parties, the amount decrypts independently on each channel and the circuit constrains both to the same value, so the two MUST agree. An implementation SHOULD perform this comparison and treat disagreement as evidence that $$k$$ is not the auditor key for both parties of that event.

**Scope MUST be represented, not implied.** The recipient-channel capability is forward-only, receiving-side only, and reset by merge (DESIGN_cont.md §8.2). An auditor's data model MUST distinguish "no activity in this period" from "activity not decryptable because it predates the current key", and MUST NOT present a reconstructed receiving balance as covering a period before the key was active. Rotation itself needs no replay on the sender side: the next owner-initiated proof operation publishes a fresh balance checkpoint under the new key.

An auditor facade MUST NOT be able to construct a spending witness, and MUST NOT be able to open a post-merge spendable balance, since merge folds the receiving randomness into a blinding that depends on $$vk$$.

---

## 12. Disclosure and Indexer Clients

### 12.1 Disclosure construction

An implementation supporting selective disclosure MUST follow SELECTIVE_DISCLOSURE.md for the holder, sender, and auditor variants, and MUST bind each proof to the requesting recipient's key and nonce so that a proof cannot be replayed against a different recipient or a later request (SELECTIVE_DISCLOSURE.md §2.1).

Disclosure circuits are verified entirely off-chain and MUST NOT be registered with the on-chain verifier set (SELECTIVE_DISCLOSURE.md §15.1).

### 12.2 Disclosure verification

The verifier MUST be distributable independently of any wallet, since its purpose is to let a party who trusts no holder check a claim. It consumes a chain endpoint, the recipient keypair, and the proof bundle with its event reference, and returns the disclosed amount or a **typed** indication of which check failed — proof verification, on-chain state mismatch, or decryption failure. A single boolean is not conformant, because the three outcomes have different meanings to the recipient.

Verification MUST include comparing the circuit's verification key against the pinned key for that disclosure circuit, without which the proof attests to an unknown statement.

Where §10.5's epoch record shows a transfer predates the derived-$$r_e$$ convention, an implementation MUST report it as not disclosable rather than attempting and reporting a failure.

### 12.3 Indexer client

Recovery beyond the RPC retention window requires a conforming durable archive (INDEXER.md). A client MUST propagate the archive's completeness signal to its caller rather than swallowing it, since an incomplete range and a tampered range both end in the same refusal at §10.6 and only that signal distinguishes them.

Clients SHOULD support multiple independent archive endpoints, since withholding is the residual trust the archive retains (INDEXER.md §7).

### 12.4 The hybrid read path and its two failure modes

RPC and archive compose: the RPC serves the recent tail, the archive everything older, and the client stitches them at a seam (INDEXER.md §1). Two requirements are not derivable from INDEXER.md and are specified here.

**The seam MUST sit strictly above the RPC's reported retention floor, by a margin.** The floor advances as ledgers are collected, including between the moment the client reads it and the moment it issues the range query, with the archive request in between taking real time. A seam placed exactly at the observed floor therefore intermittently produces a rejected query. The archive covers everything below the seam, so a margin loses no events. Implementations MUST set the two legs to disjoint ledger ranges so that correctness does not depend on cross-source id equality, and MUST still deduplicate (§10.2) as a guard at the boundary.

**A configured archive's failure MUST fail the whole sync.** If an archive is configured and its request fails, an implementation MUST NOT degrade silently to RPC-only and MUST NOT persist a sync position derived from the RPC leg alone. Persisting it moves the position past the pre-window range, so every later sync takes the warm path that never consults the archive and the openings in the skipped range become unrecoverable. An archive that is *not* configured is a different case and MAY be absent, provided §10.9's warning is surfaced.

---

## 13. Security Requirements

**Secret handling.** The root, $$sk$$, $$vk$$, $$dvk_i$$, and every cached opening are secrets. Implementations MUST keep them within the trust boundary (§2.1), SHOULD zeroize buffers holding them once no longer needed, and MUST NOT transmit them to any remote service except under §8.3's explicit opt-in.

$$vk$$ MUST NOT be presented as a safely-shareable read-only credential. It exposes every historical balance checkpoint, every incoming amount, every delegation allowance, and — under §10.5 — the opening of every transfer the account originated. Its only guarantee is that it cannot authorize spending.

**Storage at rest.** Persisted openings are as sensitive as the amounts they represent, and a persisted root is equivalent to the funds. Implementations MUST document what they persist and where, and SHOULD encrypt both at rest under a key that is not itself derived from persisted material. A signer root (§5.4) MUST NOT be persisted at all, since it is reproducible on demand from the signer.

**Logging and telemetry.** Witness material, decrypted amounts, balances, and openings MUST NOT reach logs, telemetry, analytics, error reports, or crash dumps.

**Randomness.** All sampling MUST use a cryptographically secure generator (§4.7). An implementation MUST NOT fall back to a non-CSPRNG source when one is unavailable; it MUST fail.

**Supply chain.** Soundness depends on the verification keys corresponding to the intended circuits and on the structured reference string having been generated honestly (DESIGN_cont.md §10.6). Implementations MUST pin verification keys and the proving toolchain (§8.1), SHOULD document how a user reproduces a deployed key from circuit source, and MUST NOT fetch either at runtime from a mutable location.

**What the SDK cannot do.** It cannot mitigate a compromised host: malicious code in the process, the bundle, or the storage layer reaches the root. Implementations MUST NOT present software-only custody as equivalent to hardware custody. Hardware custody is additionally constrained here, because proving requires $$sk$$ as a witness, so a device that does not prove internally must expose $$sk$$ to the host.

---

## 14. Non-Functional Requirements

**Proving latency.** Single-digit seconds on contemporary hardware is the design target (OVERVIEW.md). Implementations MUST treat it as user-visible; §8.4's progress and cancellation are requirements.

**Sync and replay bounds.** The replay window runs from the account's latest checkpoint, or from registration for an account that has received but never spent, which is unbounded in age. Implementations MUST NOT assume a bounded window, and SHOULD use the archive's checkpoint lookup where available (INDEXER.md §6, C1) so that a dormant account is not obliged to transfer its entire history.

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

This document is versioned with the protocol documentation set. A change to any primitive in §4, to the derivation in §5, or to the domain separators in §4.8 breaks the cross-implementation contract: it MUST bump the protocol documentation version, MUST be called out in release notes, and MUST be accompanied by updated fixtures in `circuits/lib/testdata/`. A change to the numeric value of a domain separator, or to the sponge convention, invalidates every previously derived key and every previously emitted ciphertext, and is a new deployment rather than an upgrade.

---

## 16. Open Protocol Questions

Each item below is a place where the protocol documents are silent, mutually inconsistent, or behind the circuits. This document states the assumption it makes so that implementations agree in the interim; each still needs a decision in the document that owns it.

**16.1 — Deterministic $$r_e$$ contradicts DESIGN.md §7.6 and revises DESIGN_cont.md §9.4.** DESIGN.md §7.6 step 1 instructs the sender to sample $$r_e$$; SELECTIVE_DISCLOSURE.md §7 and §15.2 instruct deriving it. These are opposite, and a client built to §7.6 alone permanently forecloses sender-side disclosure. §10.5 adopts derivation. Adopting it protocol-wide also requires amending DESIGN_cont.md §9.4, whose claim that a $$vk$$ holder "cannot construct openings of any commitment" no longer holds, and DESIGN_cont.md §8.2, whose scoping of receiving-side opening capability to the recipient's auditor becomes incomplete.

**16.2 — Optional hardening: bind $$\text{acct\\\_f}$$ into $$vk$$.** §5.1's $$\text{acct\\\_f}$$ binding prevents the identical-$$Y$$-and-$$\text{PVK}$$ linkage by client convention. Binding the registering address into $$vk$$ at DESIGN.md §4.2 would make it impossible by construction, at the cost of changing constraint R2 and every verification key.
