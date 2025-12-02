# Fee Abstraction Utilities

Utilities for implementing fee abstraction, allowing users to pay transaction fees in tokens instead of native XLM.

## Features

- Fee Collection Helpers (eager and lazy approval strategies)
- Inoker Helper with user-side authorizations
- Optional Fee Token Allowlist
- Optional Token Sweeping
- Validation Utilities

## Examples

- [fee-forwarder-permissioned](../../examples/fee-forwarder-permissioned)

  - Only trusted executors can call `forward`.
  - The forwarder contract itself collects the fees, which can be swept later.

- [fee-forwarder-permissionless](../../examples/fee-forwarder-permissionless)
  - Anyone can call `forward`; there is no executor allowlist.
  - The relayer (transaction submitter) receives the collected fee.

Together, these examples show how to combine the auth helper
(`auth_user_and_invoke`) with the fee collection helpers
(`collect_fee_with_eager_approval` and `collect_fee_with_lazy_approval`) in
both models.
