---
title: Analyze with Check-Checker
---

The check-checker is a static analysis pass you can use to help find potential vulnerabilities in your contracts.

*Topics covered in this guide*:

* [Enable static analysis pass](#enable-static-analysis-pass)
* [Check checker options](#options)
* [Annotations](#annotations)

## Enable static analysis pass

To enable the static analysis pass, add the following lines to your Clarinet.toml file:

```toml
[repl.analysis]
passes = ["check_checker"]
```

The check-checker pass analyzes your contract to identify places where untrusted inputs might be used in a potentially dangerous way. 
Since anyone can call public functions, any arguments passed to these functions should be considered untrusted. 
This analysis pass takes the opinion that all untrusted data must be checked before being used to modify the state of a blockchain. 
Modifying the state includes any operations that affect wallet balances or any data stored in your contracts.

- Actions on Stacks wallets:
  - stx-burn?
  - stx-transfer?
- Actions on fungible tokens:
  - ft-burn?
  - ft-mint?
  - ft-transfer?
- Actions on non-fungible tokens:
  - nft-burn?
  - nft-mint?
  - nft-transfer?
- Actions on persisted data:
  - Maps:
    - map-delete
    - map-insert
    - map-set
  - Variables:
    - var-set

In addition to those operations, the check-checker is opinionated and prefers that untrusted data be checked near the source,
making the code more readable and maintainable. For this reason,the check-checker also requires that arguments of private functions and the return values be checked.

- Calls to private functions
- Return values

Finally, another opportunity for exploits appears when contracts call functions from traits. Those traits are untrusted, just like other parameters to public functions, so they must also be checked.

- Dynamic contract calls (through traits)

When an untrusted input is used in one of these ways, you will see a warning like this:

```
bank:27:37: warning: use of potentially unchecked data
        (as-contract (stx-transfer? (to-uint amount) tx-sender customer))
                                    ^~~~~~~~~~~~~~~~
bank:21:36: note: source of untrusted input here
(define-public (withdrawal-unsafe (amount int))
```

In the case where an operation affects only the sender's wallet (e.g., calling `stx-transfer?` with the sender 
set to `tx-sender`), there is no need to generate a warning because the untrusted input affects only the sender, 
who is the source of that input. In other words, the sender should be able to safely specify parameters in an 
operation that affects only themselves. This sender is also potentially protected by post-conditions.

For a video walkthrough on how to check for smart contract vulnerabilities, please see the [Catch Smart Contract Vulnerabilities With Clarinet's Check-Checker Feature](https://www.youtube.com/watch?v=v2qXFL2owC8) video.

### Options

The check-checker provides various options that can be specified in `Clarinet.toml` to handle common usage scenarios that
may reduce false positives from the analysis:

```toml
[repl.analysis.check_checker]
strict = false
trusted_sender = true
trusted_caller = true
callee_filter = true
```

If `strict` is set to `true`, all other options are ignored, and the analysis proceeds with the most strict interpretation of the rules.

The `trusted_sender` and `trusted_caller` options handle a common practice in smart contracts where there is a concept of a 
trusted transaction sender (or transaction caller), which is treated like an admin user. Once a check has been performed 
to validate the sender (or caller), all inputs should be trusted.

In the example below, the `asserts!` on line 3 verifies the `tx-sender`. Because of that check, all inputs are trusted 
(if the `trusted_sender` option is enabled):

```clarity
(define-public (take (amount int) (from principal))
    (let ((balance (- (default-to 0 (get amount (map-get? accounts {holder: from}))) amount)))
        (asserts! (is-eq tx-sender (var-get bank-owner)) err-unauthorized)
        (map-set accounts {holder: from} {amount: balance})
        (stx-transfer? (to-uint amount) (as-contract tx-sender) tx-sender)
    )
)
```

The `callee_filter` option loosens the restriction on passing untrusted data to private functions. This option
enables checks in a called function to propagate to the caller, enabling developers to 
define input checks in a function that can be reused.

In the example below, the private function `validate` checks its parameter. The public function `save` calls `validate`, 
and when the `callee_filter` option is enabled, that call to `validate` will count as a check for the untrusted 
input `amount` resulting in no warnings from the check-checker.

```clarity
(define-public (save (amount uint))
    (begin
        (try! (validate amount))
        (var-set saved amount)
        (ok amount)
    )
)
(define-private (validate (amount uint))
    (let ((current (var-get saved)))
        (asserts! (> amount current) err-too-low)
        (asserts! (<= amount (* current u2)) err-too-high)
        (ok amount)
    )
)
```

### Annotations

Sometimes, there is code that the check-checker analysis cannot validate as safe. However, as a developer, 
you know the code is safe and want to pass that information to the check-checker to turn off such false positive warnings. Check-checker supports several annotations, implemented using "magic comments" in the contract code, to handle such cases.

**`#[allow(unchecked_params)]`**

This annotation tells the check-checker that the associated private function is allowed to receive unchecked arguments. 
The check-checker will not generate a warning for calls to this function that pass unchecked inputs. Inside the private function, 
the parameters are considered unchecked and could generate warnings.

```clarity
;; #[allow(unchecked_params)]
(define-private (my-func (amount uint))
    ...
)
```

**`#[allow(unchecked_data)]`**

This annotation tells the check-checker that the following expression is allowed to use unchecked data without warnings.
It should be used carefully, as it will turnoff all warnings from the associated expression.

```clarity
(define-public (dangerous (amount uint))
    (let ((sender tx-sender))
        ;; #[allow(unchecked_data)]
        (as-contract (stx-transfer? amount tx-sender sender))
    )
)
```

**`#[filter(var1, var2)]`**

This annotation will tell the check-checker to consider the specified variables to be checked by the following expression.
This is useful for the case where your contract performs some indirect check that validates that an input is safe, 
but there is no way for the analysis to recognize this. In place of the list of variable names in the annotation, an `*` 
may be used to filter all inputs.

_This is the safest and preferred way to silence warnings that you consider false positives._

```clarity
(define-public (filter_one (amount uint))
    (let ((sender tx-sender))
        ;; #[filter(amount)]
        (asserts! (> block-height u1000) (err u400))
        (as-contract (stx-transfer? amount tx-sender sender))
    )
)
```
