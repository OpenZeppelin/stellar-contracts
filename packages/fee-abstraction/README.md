# Fee Abstraction Utilities

Utilities for implementing fee abstraction, allowing users to pay transaction fees in tokens instead of native XLM.

## Features

- Optional Fee Token Allowlist
- Optional Token Sweeping
- Validation Utilities

### Events
- `FeeCollected`: Emitted when a fee is collected
- `ForwardExecuted`: Emitted when a call is forwarded
- `TokensSwept`: Emitted when tokens are swept

## Example

See the [fee-forwarder example](../../examples/fee-forwarder) for a complete implementation.
