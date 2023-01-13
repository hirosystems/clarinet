(define-data-var counter uint u1)
(define-constant FORBIDDEN (err u1))

(define-read-only (get-counter)
  (ok (var-get counter))
)

(define-public (add (n uint))
  (begin
    (asserts! (> n u1) FORBIDDEN)
    (var-set counter (+ (var-get counter) n))
    (ok (var-get counter))
  )
)

(define-public (call-bns)
  (contract-call? 'SP000000000000000000002Q6VF78.bns can-namespace-be-registered 0x627463)
)
