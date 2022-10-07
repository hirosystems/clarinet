(define-data-var counter int 0)

(define-read-only (get-counter)
  (ok (var-get counter))
)

(define-public (increment)
  (begin
    (var-set counter (+ (var-get counter) 1))
    (ok (var-get counter))
  )
)

(define-public (decrement)
  (begin
    (var-set counter (- (var-get counter) 1))
    (ok (var-get counter))
  )
)

;; n should be int
(define-public (add (n uint))
  (begin
    (asserts! (> n 0) (err u1))
    (var-set counter (+ (var-get counter) n))
    (ok (var-get counter))
  )
)
