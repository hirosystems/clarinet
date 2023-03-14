(define-data-var counter int 0)

(define-read-only (get-counter)
  (ok (var-get counter))
)

(define-private (set-counter (n int))
  (var-set counter n)
)


(define-public (increment)
  (let ((new-value (+ (var-get counter) 1)))
    (set-counter new-value)
    (ok (var-get counter))
  )
)


(define-public (decrement)
  (begin
    (set-counter (- (var-get counter) 1))
    (ok (var-get counter))
  )
)

(contract-call? .contract get-counter)
