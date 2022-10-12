(define-data-var counter uint u1)
(define-constant STRING u"String with escaped quote\" in the middle")

(define-read-only (get-counter)
  (ok (var-get counter))
)

(define-public (add (n uint))
  (begin
    (asserts! (> n u1) (err u1))
    (var-set counter (+ (var-get counter) n))
    (ok (var-get counter))
  )
)

(define-public (call-bns)
  (contract-call? 'SP000000000000000000002Q6VF78.bns can-namespace-be-registered 0x627463)
)
