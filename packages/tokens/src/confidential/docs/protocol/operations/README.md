# Operations

## Public Input Sources

UltraHonk verifies the relation between a proof and its public-input vector. The verifier sees only field elements -- it has no knowledge of which account, contract, or auditor those values are supposed to describe. Binding each public input to the correct provenance is the contract's responsibility. If the contract takes a value that should come from trusted state (e.g. the sender's `spending_public_key`) and instead reads it from caller-controlled invocation inputs, a soundly proven statement can verify for the wrong account.

Each operation below lists, for every public input, where the contract loads it from -- persistent account storage, the delegation entry, the contract's own contract address, an auditor-contract lookup, an invocation argument, or a prover-supplied value that the circuit binds.

**Trust-boundary rule.** Public inputs that derive from trusted state (account storage, delegation storage, the current contract address, or auditor-contract lookups) MUST be loaded by the contract itself. The contract MUST NOT accept these values from the caller's `data` payload. Only invocation arguments (which are bound under `require_auth()` per [Authorization Model](../interface.md#authorization-model)) and prover-supplied values (which the circuit binds to its constraints) may originate from the caller. Violating this rule breaks soundness even with a perfectly sound circuit.

## Owner Operations with Active Spenders

Owner transfers, withdrawals, and merges proceed identically to the no-spender case. Spender allowances are independently escrowed - no synchronization is needed. The owner's spendable balance and spender allowances are fully isolated.
