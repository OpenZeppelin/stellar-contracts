# Key Derivation

DESIGN.md §4 specifies the hierarchy below $$sk$$ — $$vk$$ from $$(sk, \text{addr\\\_f})$$, $$\text{PVK}$$ from $$vk$$, $$dvk_i$$ from $$(vk, \text{op}_i)$$. It does not specify where $$sk$$ itself comes from. This section supplies a derivation, because recovery from backup material is a stated protocol property (DESIGN.md §5.2 *Recovery*, DESIGN_cont.md §9.5, INDEXER.md §1) and two clients given the same backup material would otherwise derive different accounts.

The derivation is a single function (§5.1) over a **root**, and the root's class is determined by what controls the address rather than chosen per client. An address controlled by a Stellar ed25519 key uses a deterministic signature by that key (§5.2). An address with no ed25519 key of its own — a smart account or any other contract address — uses raw bytes from whatever custody mechanism controls it (§5.3). Tying the class to the address rather than to client preference is what keeps two clients from disagreeing about which class produced an account's $$sk$$, a disagreement `register` being single-use (DESIGN_cont.md §11) would make unrepairable. The one residual case is §5.3's fallback, where an ed25519-controlled address has no way to sign the §5.2 message; a client MUST therefore read a $$Y$$ mismatch under §5.2 as evidence that the account uses a root it does not hold, not as a derivation defect.

## Derivation

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

## Signer roots

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

## Raw roots and imported keys

Implementations MUST accept a raw 32-byte `root`, those 32 bytes being the IKM. It is the class for every address with no ed25519 key of its own, and three cases require it:

- **Contract addresses.** A confidential account registered by a smart account or other contract address has no ed25519 signer to sign §5.2's message, so its root comes from whatever custody mechanism controls the contract. Where a smart account authorises through signers of its own, the root is supplied by that mechanism rather than derived from any one of them, since the set can change without the address changing.
- **No SEP-0053 path.** An ed25519-controlled account whose custody stack cannot sign arbitrary messages (§5.2) falls back here.
- **Imported keys.** Deployments predating this specification hold $$sk$$ values sampled directly from a CSPRNG with no root behind them. Such a key MUST remain usable as a first-class account secret via direct import, bypassing §5.1 entirely.

A raw root is reproducible from nothing the user already holds, so an implementation that generates one MUST surface it for backup at creation and MUST NOT treat it as recoverable from the account's other credentials.

An implementation MUST record, per account, which of the three forms produced its $$sk$$ — signer root, raw root, or direct import — because they differ in what regenerates the key: a live signer, a stored 32-byte value, or nothing beyond the stored $$sk$$. A user MUST NOT be shown a recovery affordance an account's form cannot satisfy.

## Recovery and account discovery

Recovering a signer-root account requires the enrolled signer (§5.2) and the contract address. Where the signer came from a mnemonic, the mnemonic, its passphrase if any, and the SEP-0005 index reproduce it; where it did not, the signer itself must still be held. The set of addresses is discovered rather than remembered:

1. Enumerate candidate Stellar addresses by scanning SEP-0005 indices $$i = 0, 1, 2, \ldots$$ from the seed.
2. For each candidate address, compute $$\text{acct\\\_f}$$, obtain the §5.2 signature from that index's key, derive $$sk$$ per §5.1, and compute $$Y = sk \cdot H$$.
3. Read the account record at that address and compare its stored spending public key against $$Y$$. A match identifies a registered confidential account belonging to this signer.

Step 2 needs a signature rather than a hash, so a client that cannot sign locally SHOULD drive recovery from its own per-account record instead of scanning (§5.2). An account registered under a raw root, or under a signer that is neither held nor reproducible, is not reachable by any scan: only the direct import of a previously exported $$sk$$ recovers it.
