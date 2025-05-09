;; private functions
;; #[allow(unchecked_data)]
(define-private (complete-individual-deposits-helper
    (deposit {
      txid: (buff 32),
      vout-index: uint,
      amount: uint,
      recipient: principal,
      burn-hash: (buff 32),
      burn-height: uint,
      sweep-txid: (buff 32),
    })
    (helper-response (response uint uint))
  )
  (match helper-response
    index (begin
      (try! (unwrap!
        (complete-deposit-wrapper (get txid deposit) (get vout-index deposit)
          (get amount deposit) (get recipient deposit) (get burn-hash deposit)
          (get burn-height deposit) (get sweep-txid deposit)
        )
        (err (+ ERR_DEPOSIT_INDEX_PREFIX (+ u10 index)))
      ))
      (ok (+ index u1))
    )
    err-response (err err-response)
  )
)
