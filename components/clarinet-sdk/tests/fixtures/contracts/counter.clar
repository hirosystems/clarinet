(define-data-var counter uint u0)
(define-map participants principal bool)

(define-constant OWNER tx-sender)

(define-read-only  (get-counter)
  (ok { counter: (var-get counter) })
)

(define-public (increment)
  (begin
    (print "call increment")
    (if (is-none (map-get? participants tx-sender))
      (map-insert participants tx-sender true)
      (map-set participants tx-sender true)
    )
    (try! (stx-transfer? u1000000 tx-sender (as-contract tx-sender)))
    (ok (var-set counter (+ (var-get counter) u1)))
  )
)

(define-public (add (n uint))
  (begin
    (print "call add")
    (if (is-none (map-get? participants tx-sender))
      (map-insert participants tx-sender true)
      (map-set participants tx-sender true)
    )
    (try! (stx-transfer? u1000000 tx-sender (as-contract tx-sender)))
    (ok (var-set counter (+ (var-get counter) n)))
  )
)

(define-public (withdraw (amount uint))
  (begin
    (asserts! (is-eq tx-sender OWNER) (err u1))
    (stx-transfer? amount (as-contract tx-sender) OWNER)
  )
)
