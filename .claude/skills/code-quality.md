---
name: code-quality
description: >
  Review and (optionally) fix Rust source files against the OpenZeppelin
  Stellar Contracts Code Quality Checklist. Use this skill whenever the user
  wants to audit, review, or improve code quality of a Stellar/Soroban
  contract or library module, or mentions "/code-quality". Triggers on
  phrases like "check code quality", "review stellar style", "review soroban
  contract", "fix stellar style".
user_invocable: true
---

# OpenZeppelin Stellar Contracts — Code Quality Checklist

Reviews `.rs` files in this workspace for convention violations and either
reports them or fixes them in place — the user picks.

This is a local maintainer tool. It is not wired into CI.

## Usage

- `/code-quality` — review the file(s) changed on the current branch
  (`git diff main...HEAD --name-only`, plus any uncommitted edits).
- `/code-quality <path>` — review a specific file or directory.
- `/code-quality <package-name>` — review a workspace member by package name
  (e.g. `stellar-tokens`).

## Workflow

### 1. Check working tree

```bash
git rev-parse --abbrev-ref HEAD
git status --short
```

If the working tree is dirty, warn the user and ask whether to:

- Continue (the quality-fix commit will mix with their unrelated changes).
- Stash first, run the skill, then unstash.
- Abort.

Do not silently mix unrelated changes. If the current branch is `main`, do
not commit there — require a new branch (handled in step 6).

### 2. Discover the file set

If a path or package name was provided, expand it:

- Path → glob `**/*.rs` under it, excluding `target/`.
- Package name → look up the package in the workspace `Cargo.toml`, then
  glob `**/*.rs` under its `src/`.

If no argument was given, derive the file set from git:

```bash
git merge-base main HEAD                                          # base
git diff $(git merge-base main HEAD) HEAD --name-only             # committed
git diff --name-only ; git ls-files --others --exclude-standard   # uncommitted
```

Filter to `*.rs` under `packages/` and `examples/`. If empty, report "no
Rust files in scope" and stop.

Read every file in scope before checking rules — partial reads produce
partial reviews.

### 3. Identify violations & discrepancies

Walk the file set against **two** reference points:

1. **The rules in the `Rules` section below.** These are the codified
   conventions — explicit, named, and stable.
2. **The closest existing sibling in the repo.** For each file under
   review, pick the most relevant established counterpart and read it
   in full as the working "ground truth". Anything the new file does
   differently from its sibling is a *discrepancy* — possibly a
   violation, possibly a deliberate choice, but always worth surfacing.

How to pick the sibling: judge by context and relevancy.
Read the file under review, understand what it is doing (what kind of
module, which traits it touches, what state it manages), then scan the
repo for the existing file whose purpose is closest. For example, if it
is a token, a good example may be `packages/tokens/src/fungible`.
Always compare file type to file type — a `mod.rs` against another
`mod.rs`, a `storage.rs` against another `storage.rs`, a `test.rs`
against another `test.rs`, an example `contract.rs` against another example
`contract.rs`.

If no good sibling exists take `packages/tokens/src/fungible` for the base
example for styling (ordering of sections, inline documentation style, etc.).

Build a numbered list of findings. Each entry has:

- **file path** (repo-relative)
- **line number** (or line range)
- **kind** — either `rule:<short-rule-name>` or
  `discrepancy:<reference-file-path>`
- **finding** — what differs and why it matters
- **fix** — one or two sentences describing what should change

### 4. Choose an action

Tell the user how many findings were collected and ask which mode to
run in:

1. **Apply all** — apply every fix in one pass without further prompts.
   Stop only on a tool error or a finding the skill cannot fix on its
   own.
2. **One-by-one** — walk the findings in order. For each, describe it
   (file, line, what's wrong, what the proposed edit would do), then
   ask the user to approve before editing. Findings the user skips are
   recorded in the final report so they can be revisited.
3. **Report only** — print the full list of findings and stop. Do not
   edit anything. Skip directly to step 7.

The user may also cancel entirely; in that case stop without further
action.

If the list is empty, say so plainly and stop:

> ✅ No violations or discrepancies found in <N> file(s).

### 5. Apply fixes

Use the `Edit` tool. After all edits, run from the repo root:

```bash
# Format (NIGHTLY required — rustfmt.toml uses unstable_features)
cargo +nightly fmt --all

# Clippy with all-targets, warnings as errors
cargo clippy --release --locked --all-targets --package <name> -- -D warnings
```

Both must succeed. If clippy reports a warning the skill cannot reasonably
fix on its own, stop and escalate to the user. Do not paper it over with
`#[allow(...)]` — that is itself a violation (see the "Lint suppression"
rule).

### 6. Build, test, doc

```bash
# WASM release build — catches no_std and target-specific issues
cargo build --target wasm32v1-none --release --package <name>

# Unit + integration tests
cargo test --package <name>

# Doc check (no_deps so we don't pull in the world)
cargo doc --no-deps --package <name>
```

CI runs `cargo llvm-cov --workspace --fail-under-lines 90`. If a fix
removes test coverage, restore it before finishing.

### 7. Report

Summarise what changed, grouped by file. If nothing was edited (report-only
or no violations), there is nothing to summarise — say so and stop.

## Rules

The rules below are derived from existing modules in `packages/access`,
`packages/contract-utils`, `packages/tokens`, and `packages/governance`.
Those packages are the source of truth — a "violation" only counts if the
existing code consistently follows the opposite pattern. When two
established modules disagree, surface the discrepancy in the report rather
than picking a winner.

### Module file layout

A library module that owns state lives in its own directory and follows
this shape:

```
<module>/
├── mod.rs       # trait, errors, constants, events, public re-exports
├── storage.rs   # storage keys + free functions implementing the logic
└── test.rs      # #[cfg(test)] mod test;
```

Common ordering inside `mod.rs`:

1. Module-level docstring (`//!`) explaining purpose, structure, design.
2. Sub-`mod` declarations (`mod storage;`, then `#[cfg(test)] mod test;`).
3. `use` imports (single grouped block — see "Imports").
4. `pub use` re-exports of items from `storage` (and any sub-modules) that
   form the module's public surface.
5. The trait definition with `#[contracttrait]`.
6. `// ################## ERRORS ##################` — `#[contracterror]`
   enum.
7. `// ################## CONSTANTS ##################` — TTL constants
   etc.
8. `// ################## EVENTS ##################` — `#[contractevent]`
   structs paired with `pub fn emit_*` helpers.

Common ordering inside `storage.rs`:

1. `use` imports.
2. The storage-key type(s) (`#[contracttype]`).
3. `// ################## QUERY STATE ##################` — read functions.
4. `// ################## CHANGE STATE ##################` — write
   functions (high-level then low-level `_no_auth` variants).
5. `// ################## LOW-LEVEL HELPERS ##################` — internal
   helpers used by the section above.

The `// ################## NAME ##################` delimiter is the
canonical form (18 hashes on each side). Variations like `// === NAME ===`
or `// --- NAME ---` should be rewritten to the canonical form.

A package crate's top-level `lib.rs` is `#![no_std]` and contains only
`pub mod` declarations and the crate-level docstring. No business logic.

An example crate's top-level `lib.rs` is `#![no_std] #![allow(dead_code)]`
followed by `mod contract;` and `#[cfg(test)] mod test;`.

### Naming

- **Errors**: PascalCase variants in a `#[contracterror]` enum named
  `<Module>Error` (`FungibleTokenError`, `AccessControlError`,
  `OwnableError`, `PausableError`). The enum is `#[derive(Copy, Clone,
  Debug, Eq, PartialEq, PartialOrd, Ord)] #[repr(u32)]`. Numeric codes are
  sequential within the package's existing range (pausable 1000s, fungible
  100s, access-control 2000s, ownable 2100s). New errors should stay in
  the same range as their enum's siblings — do not invent new ranges.
- **Constants**: `UPPER_SNAKE_CASE`. TTL extension constants follow the
  pattern:
  ```rust
  const DAY_IN_LEDGERS: u32 = 17280;
  pub const FOO_EXTEND_AMOUNT: u32 = N * DAY_IN_LEDGERS;
  pub const FOO_TTL_THRESHOLD: u32 = FOO_EXTEND_AMOUNT - DAY_IN_LEDGERS;
  ```
  `DAY_IN_LEDGERS` is local (non-`pub`); `*_EXTEND_AMOUNT` and
  `*_TTL_THRESHOLD` are `pub`.
- **Storage keys**: enum named `<Module>StorageKey`, annotated
  `#[contracttype]`. Parameterised variants either inline their data
  (`Balance(Address)`) or wrap a named struct (`Allowance(AllowanceKey)`
  where `AllowanceKey` is `#[contracttype] pub struct AllowanceKey { ... }`).
- **Events**: `#[contractevent]` struct named in PascalCase, past-tense or
  noun form (`Transfer`, `Approve`, `RoleGranted`,
  `OwnershipTransferCompleted`, `Paused`). Topic fields use `#[topic]` and
  appear before non-topic fields. Always paired with a snake_case
  `pub fn emit_<event>(e: &Env, ...)` helper that constructs and
  `.publish(e)`s the event — even for empty events
  (`Paused {}.publish(e)`).
- **`_no_auth` suffix**: low-level write functions that bypass
  authorisation checks (`grant_role_no_auth`, `set_role_admin_no_auth`,
  `revoke_role_no_auth`). These must carry a "Security Warning" doc
  section explaining the missing check and naming the recommended caller
  context.
- **`__constructor`**: contract constructors are named exactly
  `__constructor` (double underscore prefix), defined inside the
  contract's `#[contractimpl] impl <Contract> { ... }` block, and take
  `e: &Env` as the first parameter.

### Module & package

- Crate name: `stellar-<scope>` (`stellar-access`, `stellar-tokens`).
  Internal Rust module names are snake_case.
- All library packages are `#![no_std]`. `[dependencies]` must not pull in
  `std`-only crates. The `[dev-dependencies]` block may
  (`stellar-event-assertion`, `soroban-sdk` with `testutils`).
- Workspace fields use the `field.workspace = true` shorthand
  (`edition.workspace = true`, `version.workspace = true`,
  `license.workspace = true`, `repository.workspace = true`,
  `authors.workspace = true`).
- Library packages declare `[lib] crate-type = ["lib", "cdylib"], doctest =
  false`, set `publish = true`, and have a `description = "..."`.
- Example packages declare `[lib] crate-type = ["cdylib"], doctest = false`,
  set `publish = false`, and have no `description`.
- Every package declares `[package.metadata.stellar] cargo_inherit = true`.
- Workspace-level dependencies are pulled in with `{ workspace = true }` —
  no inline versions in member `Cargo.toml` files.

### Imports

- `rustfmt.toml` sets `imports_granularity = "Crate"` and
  `group_imports = "StdExternalCrate"`. Hand-written import blocks must fit
  that style: one grouped `use` per crate, with std/external/crate groups
  separated by a blank line.
- Prefer importing items used in the file directly rather than re-pathing
  through the crate at every call site — e.g.
  `use stellar_access::access_control::{self as access_control, AccessControl};`
  when both the module path and the trait are needed.
- Wildcard imports (`use foo::bar::*;`) are not used anywhere in the
  codebase.

### `Super` keyword

- `super` keyword is frowned upon and should be avoided for the sake of full concrete path, both in code and in inline documentation. Use the full path always instead of `super`.

### Errors and panics

- Always raise contract errors with `panic_with_error!(e, EnumName::Variant)`.
  Bare `panic!`, `unreachable!`, and `unwrap()` are violations in non-test
  code. (`clippy.toml` sets `allow-unwrap-in-tests = true`, so test code
  is fine.)
- `expect("...")` is acceptable for invariants the surrounding code has
  just established. The message must explain *why* the value is guaranteed
  to be present, not what it is.
- Document errors in the `# Errors` section of every public function that
  can panic. Each bullet uses
  ``[`ModuleError::Variant`] - <one-sentence reason>`` form. For functions
  that delegate, write ``refer to [`other_function`] errors.``
- Each public function's doc comment uses the section order:
  `# Arguments` → `# Errors` → `# Events` → `# Notes` →
  `# Security Warning`. Skip any section that doesn't apply, but do not
  reorder them.

### Events

- All events are defined with `#[contractevent]` and
  `#[derive(Clone, Debug, Eq, PartialEq)]`. Topic fields are tagged
  `#[topic]` and appear first in the struct.
- For SEP-41 / cross-event-symbol compatibility, `topics = ["..."]` and
  `data_format = "single-value"` attributes are allowed (e.g.
  `MuxedTransfer` reuses the `"transfer"` topic; `Transfer`'s data is a
  bare `i128`). Use these only when matching an external standard — do not
  introduce them for new internal events.
- Every event struct is paired with a `pub fn emit_<snake_case>(e: &Env,
  ...)` helper in `mod.rs` that constructs the struct (`.clone()`-ing
  reference args), then calls `.publish(e)`. Inline
  `Event { ... }.publish(e)` calls scattered through the storage layer
  should be funneled through the helper.
- Document events in the `# Events` section of any function that emits
  one, using the form:
  ```
  /// # Events
  ///
  /// * topics - `["transfer", from: Address, to: Address]`
  /// * data - `[amount: i128]`
  ```

### Storage and TTL

- Three storage tiers, each with a clear purpose:
  - **`instance`** — contract-wide singletons that live as long as the
    contract: metadata, total supply, owner/admin, pause flag.
  - **`persistent`** — long-lived per-key data: balances, role
    memberships, role enumeration, NFT ownership.
  - **`temporary`** — time-bounded data with a caller-specified expiry:
    allowances, pending admin/ownership transfers.
- **Library-managed entries extend TTL on read, not on write.** When the
  library owns a `persistent` or `temporary` entry, every read function
  that successfully fetches the entry must call `extend_ttl` immediately
  after. The canonical pattern is:
  ```rust
  if let Some(value) = e.storage().persistent().get::<_, T>(&key) {
      e.storage().persistent().extend_ttl(
          &key,
          FOO_TTL_THRESHOLD,
          FOO_EXTEND_AMOUNT,
      );
      value
  } else {
      Default::default()
  }
  ```
  or, for `Option<T>` returns:
  ```rust
  e.storage().persistent().get(&key).inspect(|_| {
      e.storage().persistent().extend_ttl(
          &key,
          FOO_TTL_THRESHOLD,
          FOO_EXTEND_AMOUNT,
      )
  })
  ```
  Argument order is always `(&key, TTL_THRESHOLD, EXTEND_AMOUNT)`.
  Reversing it is a silent bug.
- **`instance` storage TTL is the contract developer's responsibility, not
  the library's.** Library modules expose `INSTANCE_TTL_THRESHOLD` and
  `INSTANCE_EXTEND_AMOUNT` as sane defaults but should not call
  `e.storage().instance().extend_ttl(...)` themselves.
- **Utility modules with caller-managed instance state do NOT extend TTL
  on read.** The pausable module is the canonical example: its `paused()`
  reader explicitly does not call `extend_ttl` because the contract using
  pausable already manages its instance TTL. New utilities storing a
  single `instance` flag should follow this pattern and add an
  explanatory `// NOTE: ...` comment.
- The storage-key enum is defined once in `storage.rs` and re-exported via
  `pub use` from `mod.rs`. Duplicate definitions across files are a
  violation.
- Reads must default sensibly when the entry is missing — `0` for balances
  and counts, empty `Vec` for collections, `None` for optional pointers,
  default struct for allowances. Returning `panic_with_error!` on a
  missing read is only correct for required setup data (metadata, admin)
  where absence is a real error.

### Functions and authorization

- All public library functions take `e: &Env` as the first parameter, then
  borrowed `&Address` arguments. Trait methods exposed to contracts take
  owned `Address` and convert to a borrow when delegating:
  `Self::ContractType::transfer(e, &from, &to, amount)`.
- High-level functions handle their own auth via
  `<address>.require_auth()` and emit events. Low-level `_no_auth`
  siblings do neither — they accept a `caller: &Address` purely for event
  emission. The split must be honoured: `require_auth()` inside a
  `_no_auth` function, or omitting it from the high-level entry point, is
  a violation.
- **Never call `require_auth()` twice on the same address inside one
  invocation** — Soroban panics. This is why
  `#[has_role]` / `#[has_any_role]` exist alongside
  `#[only_role]` / `#[only_any_role]`: pick `has_*` when the function
  body (or the `Base::` helper it delegates to) already requires auth, and
  `only_*` otherwise. Combining `require_auth()` with `#[only_*]` macros
  on the same account is a violation.
- Functions that lack auth on purpose (`Base::mint`, `pausable::pause`,
  `set_admin`, `set_owner`, `Base::set_metadata`) must carry a
  `# Security Warning` block in their docs explaining the missing check
  and naming the recommended caller context.

### Macros

- The macros in `stellar-macros` are the canonical way to express access
  control on contract entry points:
  - `#[only_admin]` — admin-only, calls `enforce_admin_auth`.
  - `#[only_owner]` — owner-only, calls `enforce_owner_auth`.
  - `#[only_role(addr, "role")]` — role check + `require_auth()` on `addr`.
  - `#[has_role(addr, "role")]` — role check, **no** `require_auth()`.
  - `#[only_any_role(addr, [...])]`, `#[has_any_role(addr, [...])]` —
    same idea, multiple roles.
  - `#[when_paused]`, `#[when_not_paused]` — pausable guards.
- Pick the version that matches the auth flow already inside the function.
  Re-implementing these checks by hand (e.g. inlining
  `caller.require_auth(); if owner != caller { panic_with_error!(...) }`)
  when a macro fits is a violation.
- **First-arg requirement**: all of these macros require the function's
  first argument to be `Env` or `&Env`. Putting another argument first is
  the most common bug.

### Traits and contract types

- The methods that are closely relevant with the new implemented module,
  that is expected to be implemented by the contracts, should be placed
  under a trait, they shouldn't be implemented on the contract itself directly
  using the storage functions. The trait should be implemented on the contract
  instead.
- If there is no good reason on not to, all the trait methods should have a
  default implementation (utilize the relevant storage functions for that).
- Library traits that contracts implement are annotated `#[contracttrait]`.
  Default method bodies delegate either to a free function in the same
  module (`storage::has_role(e, &account, &role)`) or to the associated
  contract type (`Self::ContractType::transfer(e, &from, &to, amount)`).
- Token-style traits use the `type ContractType: ContractOverrides;`
  associated-type pattern to enable mutually-exclusive extensions
  (`Base`, `AllowList`, `BlockList`, `Enumerable`, `Consecutive`, ...).
  Adding a new variant requires:
  1. A unit struct (`pub struct MyVariant;`).
  2. An `impl ContractOverrides for MyVariant { /* overrides */ }`.
  3. Compatibility documentation in the trait docstring listing which
     extensions can/can't co-exist with it.
- In examples and downstream contracts, use
  `#[contractimpl(contracttrait)]` when implementing a `#[contracttrait]`
  trait so the macro picks up default method bodies. Use plain
  `#[contractimpl]` for the contract's own inherent `impl` block (the one
  containing `__constructor` and any helpers that aren't part of a trait).
  Reversing this is a violation.

### Documentation

- Triple-slash `///` for items rendered in rustdoc; double-slash `//` for
  inline notes. JavaDoc-style `/** */` is not used.
- Every public item gets at least a one-line summary. Functions that take
  or produce non-trivial values get the full
  `# Arguments` / `# Errors` / `# Events` / `# Notes` /
  `# Security Warning` block in that order.
- Trait methods in `mod.rs`, and functions in `storage.rs` need to have a
  rigid inline documentation style. Follow the `packages/tokens/src/fungible` directory's
  `mod.rs` and `storage.rs` files as examples for that.
- The module-level `//!` docstring at the top of each `mod.rs` explains
  purpose, structure, and any non-obvious design choices (the dual-layer
  high/low-level split, mutual exclusivity of extensions, the
  contract-developer-owned `instance` TTL, etc.). New library modules
  without one are a violation.
- `cargo doc --no-deps` should run cleanly with no broken intra-doc links.
  When a public item is renamed, every `[`OldName`]` reference in
  docstrings must be updated.

### Testing

- Tests live in a `test.rs` file alongside the module they exercise, gated
  behind `#[cfg(test)] mod test;` in the parent `mod.rs` (or `lib.rs` for
  examples).
- Test files start with `extern crate std;` followed by the imports.
- Test setup uses `Env::default()`, `Address::generate(&e)`, and
  `e.register(MockContract, ())` (or `e.register(ExampleContract, (...))`
  with constructor args). For library-internal tests, wrap calls in
  `e.as_contract(&address, || { ... })`. Additionally, crates can generate
  a client (`ExampleContractClient::new(&e, &address)`) and
  test through it.
- Authorisation in tests uses `e.mock_all_auths()` — never hand-build auth
  payloads unless explicitly testing the auth machinery.
- Event assertions use `EventAssertion::new(&e, address.clone())` from the
  `stellar-event-assertion` dev-dependency. Hand-decoding
  `e.events().all()` is a violation when an `assert_*` helper exists.
- Panic tests use
  `#[should_panic(expected = "Error(Contract, #<code>)")]` — the numeric
  code must match the `#[repr(u32)]` value of the expected enum variant.
  String-pattern panics
  (`#[should_panic(expected = "InsufficientBalance")]`) are a violation.
- Coverage threshold is 90% (`cargo llvm-cov --fail-under-lines 90`). New
  public functions without test coverage are a violation.

### Architecture and style

- **Reads are free; writes are expensive.** Optimise for fewer writes,
  not fewer reads. A read-then-decide-then-write pattern is preferred over
  always-write.
- **Computation is cheap; developer experience is expensive.** Idiomatic,
  declarative Rust beats hand-rolled loops and bit-fiddling unless there
  is a measured cost reason. The `enumerable` and `consecutive` extensions
  are the reference for the right balance.
- **Two-step transfers for sensitive ownership.** Admin and owner
  transfers are always 2-step: an `initiate` call writes a pending entry
  to `temporary` storage with a `live_until_ledger`, and a separate
  `accept` call by the recipient completes it. Direct, single-call
  transfers of admin/owner are not allowed for new modules — reuse
  `role_transfer`.
- **Constructor responsibilities.** `__constructor` is the only place
  where `set_owner`, `set_admin`, and similar one-shot setters can be
  called without auth. Calling them outside the constructor (without an
  auth check) is a violation.
- **Composability over self-transfers.** Library functions return objects
  / values rather than calling `transfer::*` internally. The caller
  decides what to do with the result.

### Lint suppression

- `#[allow(...)]` attributes silencing clippy or compiler warnings are a
  violation. Every warning should be resolved by fixing the underlying
  code.
- If a warning genuinely cannot be addressed (e.g. an upstream
  `soroban-sdk` macro emits something unavoidable), stop and escalate to
  the user instead of suppressing.

---

## Important notes

- Never add a `Co-Authored-By` trailer to commits.
- The PR body must follow the repo's `pull_request_template.md` shape:
  `Fixes #???` line, a short description, and the `#### PR Checklist`
  block. Do not invent a "Test plan" section — it is not part of the
  template.
- When unsure whether a piece of code is "wrong" or just "different", read
  two or three sibling modules first. The library is consistent enough
  that real violations stand out; if a pattern appears in at least one
  established module, it is the convention, even if it differs from what
  the rule list above suggests — surface the discrepancy to the user
  instead of silently rewriting.
