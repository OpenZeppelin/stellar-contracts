# Social Recovery Policy Example

A policy contract that lets a quorum of guardians restore access to a smart account whose owner lost their keys. One deployment serves every smart account on a network, and each account configures its own guardians, threshold, delay and expiry.

For more information, check:
- [`social_recovery` module documentation](../../../packages/accounts/src/policies/social_recovery.rs), which is the API reference
- [Smart Accounts Package README](../../../packages/accounts/README.md)
- [Multisig Smart Account Example](../README.md) for deploying the account this guide recovers

## How It Works

Alice owns smart account `A`. Its rule 0 is a `Default` rule with one signer, her passkey `pk1`. She picks three guardians: Bob and Carol, who use ordinary `G…` accounts, and Dave, whose guardian address is his own smart account. Any two of them can recover `A`. `P` is the shared social recovery policy contract.

**Setup.** Alice signs one call with `pk1`. It adds rule 1 to `A`, scoped to calls on `A` itself, with no signers and `P` as its only policy:

```
A.add_context_rule(CallContract(A), "recovery", None, [], { P: {
    guardians: [Bob, Carol, Dave],
    threshold: 2,
    delay_ledgers: 17_280,     // ~1 day
    expiry_ledgers: 120_960,   // ~7 days
}})
```

`P` stores the configuration for `(A, 1)` and sets its epoch to 1. A recovery always adds a `Default` rule named `recovered` with no policies, and the guardians choose only its signers.

**Recovery.** Alice loses her phone, and her new device creates the passkey `pk2`.

1. At ledger 1,000,000, Bob calls `P.approve(Bob, 1, [pk2], 1, A)`. His approval opens an attempt for the signer set `[pk2]` at epoch 1, and the approvals must reach the threshold before ledger 1,120,960.
2. At ledger 1,010,000, Carol makes the same call. That is 2 of 3 approvals, so the delay starts: the attempt is executable from ledger 1,027,280 until just before ledger 1,148,240.
3. During the delay, Alice can call `P.cancel(1, A)` if she still holds `pk1` and the recovery is not hers. The epoch becomes 2 and the attempt is dead.
4. At ledger 1,027,280 or later, anyone submits `A.add_context_rule(Default, "recovered", None, [pk2], {})` and selects rule 1 with no signatures. Rule 1 has no signers, so `A` defers to `P.enforce`. `P` checks that the call adds exactly that rule for a ready attempt, removes the attempt and moves the epoch to 2. `A` adds rule 2 with `pk2`.
5. Alice signs with `pk2` through rule 2 and removes rule 0. Rule 1 stays in place for any later recovery.

### Attempt States

An attempt is identified by `(smart_account, context_rule_id, epoch, signers_hash)`, where `signers_hash` is the SHA-256 hash of the XDR-encoded signer set, sorted by each signer's XDR encoding. The order of the signers does not change the hash, so `[pk2, pk3]` and `[pk3, pk2]` are the same attempt. `get_attempt` returns the stored fields, and the state follows from them, the configured `expiry_ledgers` (`x`) and the current ledger `L`:

| State | Condition | What can happen |
|---|---|---|
| Collecting | `executable_after` is `None` and `L < opened_at + x` | Guardians approve or revoke |
| Ready | `executable_after` is set and `L < executable_after + x` | Completion when `L ≥ executable_after`; more approvals are recorded and do not move the delay |
| Expired | Past the deadline of its state | The next `approve` opens a fresh attempt for the same signer set |
| Dead | Its epoch is below `get_epoch` | Nothing; never read again |

The epoch increases on `install`, `set_config`, `cancel` and every completion. A guardian's revoke that drops a Ready attempt below the threshold clears `executable_after`, and the next approval that reaches the threshold starts a new delay. The quorum deadline stays `opened_at + x`.

## 1. Build and Deploy the Policy

```bash
stellar contract build --package multisig-social-recovery-policy-example
stellar contract deploy --alias social_recovery_policy \
    --wasm target/wasm32v1-none/release/multisig_social_recovery_policy_example.wasm
```

The contract has no constructor, no admin and no upgrade entry point. Every account that uses a deployment trusts its code, so check that a shared deployment runs this Wasm before relying on it, for example by comparing `stellar contract fetch` output with a local build.

For this example, the shell variables below hold the deployed addresses and the new key:

```bash
ACCOUNT=C...           # the smart account, `A`
POLICY=C...            # the social recovery policy, `P`
VERIFIER=C...          # the WebAuthn verifier of the new passkey
NEW_KEY=04...          # the new passkey's public key and credential ID, hex
```

## 2. Add the Recovery Rule

The owner calls `add_context_rule` on the account, authorized through its admin rule. The policy's install parameters are an untyped `Val` in the account's `policies` map, so the CLI takes them as raw `ScVal` JSON, with the field names as map keys in sorted order:

```bash
PARAMS='{"map":[
  {"key":{"symbol":"delay_ledgers"},"val":{"u32":17280}},
  {"key":{"symbol":"expiry_ledgers"},"val":{"u32":120960}},
  {"key":{"symbol":"guardians"},"val":{"vec":[
    {"address":"G...BOB"},{"address":"G...CAROL"},{"address":"C...DAVE"}]}},
  {"key":{"symbol":"threshold"},"val":{"u32":2}}]}'

stellar contract invoke --id $ACCOUNT -- add_context_rule \
    --context_type "{\"CallContract\":\"$ACCOUNT\"}" \
    --name recovery \
    --signers '[]' \
    --policies "{\"$POLICY\": $PARAMS}"
```

The stellar CLI cannot sign for a smart account, so submit the call through the tool that signs for the account (see Next Steps in the [account example](../README.md#next-steps)), selecting the admin rule. The same call from Rust:

```rust
account_client.add_context_rule(
    &ContextRuleType::CallContract(account.clone()),
    &String::from_str(&e, "recovery"),
    &None,
    &Vec::new(&e),
    &map![&e, (policy.clone(), SocialRecoveryAccountParams {
        guardians: vec![&e, bob, carol, dave],
        threshold: 2,
        delay_ledgers: 17_280,
        expiry_ledgers: 120_960,
    }
    .into_val(&e))],
);
```

`install` rejects the configuration with `InvalidConfig` (3232) unless:

| Parameter | Rule |
|---|---|
| The rule | Scoped to `CallContract(smart_account)`. The policy does not check its signers or other policies, so keep it at no signers and this policy alone. |
| `guardians` | 1 to 32 distinct addresses, excluding the smart account and the policy contract |
| `threshold` | From 1 to the number of guardians |
| `delay_ledgers` | At least 1. This is the owner's veto window, and at ~5 seconds per ledger, 17,280 is about a day. |
| `expiry_ledgers` | At least 1. Approvals must reach the threshold within it, and a ready attempt must complete within it. |
| Ledger headroom | Current ledger + `delay_ledgers` + 2 × `expiry_ledgers` fits in a `u32` |

Leave the recovery rule's `valid_until` unset, since a recovery rule that expires turns recovery off with no event. Accounts built on `stellar-accounts` 0.8.0 or later can add the rule without an upgrade.

## 3. Approve a Recovery

Each guardian reads the current epoch and approves the new signer set. A `G…` guardian signs with the stellar CLI, here with the `bob` identity:

```bash
stellar contract invoke --id $POLICY -- get_epoch --context_rule_id 1 --smart_account $ACCOUNT

stellar contract invoke --id $POLICY --source bob -- approve \
    --guardian bob \
    --epoch 1 \
    --signers "[{\"External\":[\"$VERIFIER\",\"$NEW_KEY\"]}]" \
    --context_rule_id 1 \
    --smart_account $ACCOUNT
```

The guardian's signature covers all five arguments, so a relayer cannot swap the signer set, and an approval signed for an earlier epoch fails with `StaleEpoch` (3235). The other guardians make the same call with the same signers in any order, ideally copied from the `signers` field of the `recovery_approval_recorded` event the first approval emitted. A guardian that is a smart account authorizes `approve` through one of its own rules that admits a call on `P`. A guardian contract that calls `approve` with its own address as `guardian` authorizes as the direct invoker.

Before approving, the guardian's wallet checks the set. The policy cannot tell a genuine signer from a malicious verifier, or from a delegated signer that authorizes back into the account, and either one lets anyone use the recovered rule:

- An `External` signer's verifier is a known deployment for the network, and its key parses in that verifier's format.
- A `Delegated` signer is a `G…` address, or a `C…` address whose Wasm hash is a known smart account build.
- The set's hash matches the one the recovering person's wallet shows. The guardian compares at least the first 20 hex characters (80 bits) of the set's `hash_signers` value with that person, on the same call that confirms who they are.

The guardians who approve later run the same checks on the `signers` of the opening event, and refuse or revoke when one fails.

`approve` rejects an empty set, more than 15 signers, an external key over 256 bytes, a delegated signer or a verifier that is the smart account or the policy contract, and repeated signers with `InvalidSigners` (3233). It then calls `batch_canonicalize_key` on each verifier the set names, and two keys with the same canonical form fail with `SmartAccountError::DuplicateSigner` (3007). A verifier that fails blocks approvals of every set that names it, with the verifier's own error, and of no other set.

To compute the hash locally and read the attempt:

```bash
SIGNERS_HASH=$(echo "{\"vec\":[{\"vec\":[{\"symbol\":\"External\"},{\"address\":\"$VERIFIER\"},{\"bytes\":\"$NEW_KEY\"}]}]}" \
    | stellar xdr encode --type ScVal --output single | shasum -a 256 | cut -d' ' -f1)

stellar contract invoke --id $POLICY -- get_attempt \
    --epoch 1 --signers_hash $SIGNERS_HASH --context_rule_id 1 --smart_account $ACCOUNT
```

A set of several signers is sorted by each signer's XDR encoding before it is hashed. Rust clients get the same value from `stellar_accounts::policies::social_recovery::hash_signers`. Every approval event also carries `signers_hash`. The policy's `hash_signers` view returns the hash of any set for queries such as `get_attempt`, but the guardian's comparison does not rely on it:

```bash
stellar contract invoke --id $POLICY -- hash_signers \
    --signers "[{\"External\":[\"$VERIFIER\",\"$NEW_KEY\"]}]"
```

A guardian withdraws an approval with `revoke_approval`, which takes the same five arguments. It calls no verifier, so a revoke works even when a verifier in the set fails.

## 4. Veto a Recovery

The `recovery_quorum_reached` event marks the start of the delay and carries `executable_after` and `expires_at`. Until a completion lands, the owner can cancel every open attempt:

```bash
stellar contract invoke --id $POLICY -- cancel --context_rule_id 1 --smart_account $ACCOUNT
```

`cancel` requires the smart account's authorization for a call on `P`, which the admin rule gives. The recovery rule cannot give it, and neither can a `spending_limit` session key. Submit it through the account's signing tool, as in step 2. The cancel has no deadline, but an attacker can complete and remove the old rule in one transaction, so in practice the veto window is `delay_ledgers`. Notification services should alert the owner on `recovery_quorum_reached`.

## 5. Complete the Recovery

From `executable_after` until `expires_at`, anyone can submit the completion: a call to `add_context_rule(Default, "recovered", None, signers, {})` on the account, with the approved signers in any order. The smart account's authorization entry carries `AuthPayload { signers: {}, context_rule_ids: [1] }`, with no signatures. The relayer pays the fee and signs only as the transaction source.

The flow with `@stellar/stellar-sdk`: simulate to record the account's authorization entry, fill in the payload, simulate again so the footprint includes what `__check_auth` and the policy read, then submit.

```ts
import {
  Account,
  Address,
  BASE_FEE,
  Keypair,
  Networks,
  Operation,
  TransactionBuilder,
  rpc,
  xdr,
} from "@stellar/stellar-sdk";

const server = new rpc.Server("https://soroban-testnet.stellar.org");
const networkPassphrase = Networks.TESTNET;
const relayer = Keypair.fromSecret(process.env.RELAYER_SECRET!);

const smartAccount = "C..."; // the account being recovered
const recoveryRuleId = 1; // the rule that carries the policy
const verifier = "C..."; // verifier of the new key
const newKey = Buffer.from("...", "hex"); // the new public key

// `add_context_rule(Default, "recovered", None, signers, {})`, with the
// approved signers in any order.
const args = [
  xdr.ScVal.scvVec([xdr.ScVal.scvSymbol("Default")]),
  xdr.ScVal.scvString("recovered"),
  xdr.ScVal.scvVoid(),
  xdr.ScVal.scvVec([
    xdr.ScVal.scvVec([
      xdr.ScVal.scvSymbol("External"),
      new Address(verifier).toScVal(),
      xdr.ScVal.scvBytes(newKey),
    ]),
  ]),
  xdr.ScVal.scvMap([]),
];

// `AuthPayload { signers: {}, context_rule_ids: [recoveryRuleId] }`, with the
// map keys in sorted order.
const authPayload = xdr.ScVal.scvMap([
  new xdr.ScMapEntry({
    key: xdr.ScVal.scvSymbol("context_rule_ids"),
    val: xdr.ScVal.scvVec([xdr.ScVal.scvU32(recoveryRuleId)]),
  }),
  new xdr.ScMapEntry({
    key: xdr.ScVal.scvSymbol("signers"),
    val: xdr.ScVal.scvMap([]),
  }),
]);

function addressCredentials(entry: xdr.SorobanAuthorizationEntry): xdr.SorobanAddressCredentials {
  const credentials = entry.credentials();
  switch (credentials.switch().name) {
    case "sorobanCredentialsAddress":
      return credentials.address();
    case "sorobanCredentialsAddressV2":
      return credentials.addressV2();
    default:
      throw new Error(`unexpected credentials: ${credentials.switch().name}`);
  }
}

async function completeRecovery(): Promise<void> {
  const source = await server.getAccount(relayer.publicKey());
  const build = (auth: xdr.SorobanAuthorizationEntry[]) =>
    new TransactionBuilder(new Account(source.accountId(), source.sequenceNumber()), {
      fee: BASE_FEE,
      networkPassphrase,
    })
      .addOperation(
        Operation.invokeContractFunction({
          contract: smartAccount,
          function: "add_context_rule",
          args,
          auth,
        }),
      )
      .setTimeout(60)
      .build();

  // 1. Simulation records the smart account's authorization entry.
  const recorded = await server.simulateTransaction(build([]));
  if (!rpc.Api.isSimulationSuccess(recorded) || !recorded.result) {
    throw new Error(`simulation failed: ${JSON.stringify(recorded)}`);
  }
  const { sequence } = await server.getLatestLedger();
  const auth = recorded.result.auth.map((entry) => {
    const credentials = addressCredentials(entry);
    credentials.signatureExpirationLedger(sequence + 60);
    credentials.signature(authPayload);
    return entry;
  });

  // 2. Simulating again with the filled entry runs `__check_auth` and the
  //    policy, which adds their reads to the footprint.
  const enforced = await server.simulateTransaction(build(auth));
  if (!rpc.Api.isSimulationSuccess(enforced)) {
    throw new Error(`recovery not executable: ${enforced.error}`);
  }

  // 3. The relayer signs only as the fee payer.
  const tx = rpc.assembleTransaction(build(auth), enforced).build();
  tx.sign(relayer);
  const { hash } = await server.sendTransaction(tx);
  const result = await server.pollTransaction(hash);
  if (result.status !== "SUCCESS") {
    throw new Error(`recovery failed: ${result.status}`);
  }
}

completeRecovery();
```

When the policy rejects the completion inside `__check_auth`, simulation fails with `Error(Auth, InvalidAction)`, and the diagnostic events show the policy's code, usually `NotAllowed` (3238). The `authorize_completion` helper in the [account example's tests](../account/src/test.rs) builds the same authorization entry in Rust.

After the policy admits the completion, the account checks the new signers for canonical duplicates again, which calls their verifiers. If a verifier in the set started failing after the approvals, the completion reverts with that verifier's error and the attempt stays as it was.

A successful completion emits the policy's `recovery_authorized`, then the account's `signer_registered` for each new signer and `context_rule_added`. `recovery_authorized` alone does not mean a rule was added: a direct `enforce` call under the account's authorization retires the attempt without adding one. A recovery is complete only when `context_rule_added` follows in the same transaction.

## 6. Clean Up

The recovered rule gets a new ID, rule 2 in the example. The new owner removes the old rule through it, signing with the new key and selecting rule 2 in the `AuthPayload` through the account's signing tool:

```bash
stellar contract invoke --id $ACCOUNT -- remove_context_rule --context_rule_id 0
```

Until then, anyone holding the old key keeps full authority over the account. Policy state keyed by the old rule's ID, such as a spending window, does not carry over to the new rule.

The recovered rule has no policies, so every one of its signers must sign through it. The new owner adds any policies it should have, such as a threshold for a set of several keys, with `add_policy` through the same rule.

## Safety

- **Lost keys only.** Anyone holding the owner's keys can remove the policy, cancel any attempt and replace the guardians immediately. The policy does not protect an account whose keys were stolen.
- **Veto window.** The owner's veto lasts as long as `delay_ledgers` in practice, and only rules that admit a call on the policy contract can cast it.
- **Rules on the policy contract.** A rule that admits a call on the policy contract can also call `set_config` and `uninstall`, so it can replace the guardians and then recover the account. It holds the owner's full authority unless a policy on that rule limits it to `cancel`.
- **Guardians.** A quorum of guardians acting together cannot be told apart from the owner, except by the owner's cancel. Guardians must confirm off-chain that a new signer set belongs to the person recovering.
- **Signers.** The policy cannot tell a genuine signer from a malicious verifier, or from a delegated signer that authorizes back into the account, and either one lets anyone use the recovered rule. Guardian wallets check each signer and compare the set's hash with the person recovering before approving, as step 3 describes.
- **The old rule.** A recovery adds a rule and removes nothing. The lost key keeps its authority until the new owner removes the old rule.
- **Upgrades.** The example contract is not upgradeable. Whoever can upgrade a deployment of this policy controls recovery for every smart account that uses it.
- **Guardian accounts.** A smart-account guardian with `Delegated` signers inherits the delegation caveats in the [Smart Accounts README](../../../packages/accounts/README.md#delegated-signers).

## Common Mistakes

| Mistake | What happens | How to fix |
|---|---|---|
| Giving the recovery rule signers or other policies | `install` accepts it; extra policies add checks that can block completion | Keep the rule at no signers and this policy alone |
| Setting `valid_until` on the recovery rule | Recovery stops working after that ledger, with no event | Leave it `None` unless recovery should be time-boxed |
| Approving a signer without checking its verifier or delegate | A malicious verifier, or a delegate that authorizes back into the account, lets anyone use the recovered rule | Resolve each signer to a known verifier deployment or account build before approving |
| Confirming only the new key with the person recovering | A verifier swapped on the way to the guardians passes the key check and lets anyone use the recovered rule | Compare at least 80 bits of `hash_signers`, computed by the guardian's own wallet, with that person |
| Completing with a context type, name, `valid_until` or policies other than `Default`, `"recovered"`, `None` and `{}` | `NotAllowed` (3238) inside `__check_auth`, surfaced as `Error(Auth, InvalidAction)` | Submit `add_context_rule(Default, "recovered", None, signers, {})` |
| Adding signatures to the completion's `AuthPayload` | The account rejects signers that are not on the selected rule (`UnauthorizedSigner`, 3016) | Send an empty `signers` map and `context_rule_ids: [recovery rule]` |
| Expecting the recovered rule to carry the old rule's policies | The recovered rule has none, so every signer of a multi-key set must sign through it | Add policies with `add_policy` through the recovered rule after the recovery |
| Treating `recovery_authorized` as a completed recovery | An indexer reports a rule that does not exist | Wait for the account's `context_rule_added` in the same transaction |
| Leaving the old rule after recovery | Whoever finds the lost key keeps full control | Remove the old rule through the recovered one right away |
| Not watching `recovery_quorum_reached` | The owner misses the veto window | Alert the owner on that event for their account |
| Relying on a session key to cancel | `cancel` fails, because a `spending_limit` rule never admits a call on the policy | Cancel through the admin rule |
| Using a deployment that someone can upgrade | The upgrader controls recovery for every account using it | Deploy this crate, which has no upgrade path, and verify a shared deployment's Wasm |
