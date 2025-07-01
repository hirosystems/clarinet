(define-public (finalize
    (token (optional <sip-010>))
    (with principal)
  )
  (let (
      (pipe-key (try! (get-pipe-key (contract-of-optional token) tx-sender with)))
      (pipe (unwrap! (map-get? pipes pipe-key) ERR_NO_SUCH_PIPE))
      (closer (get closer pipe))
      (expires-at (get expires-at pipe))
    )
    ;; A forced closure must be in progress
    (asserts! (is-some closer) ERR_NO_CLOSE_IN_PROGRESS)

    ;; The waiting period must have passed
    (asserts! (> burn-block-height expires-at) ERR_NOT_EXPIRED)

    ;; Reset the pipe in the map.
    (reset-pipe pipe-key (get nonce pipe))

    ;; Emit an event
    (print {
      event: "finalize",
      pipe-key: pipe-key,
      pipe: pipe,
      sender: tx-sender,
    })

    (payout token (get principal-1 pipe-key) (get principal-2 pipe-key)
      (get balance-1 pipe) (get balance-2 pipe)
    )
  )
)
