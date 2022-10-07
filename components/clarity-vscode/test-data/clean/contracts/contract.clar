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
