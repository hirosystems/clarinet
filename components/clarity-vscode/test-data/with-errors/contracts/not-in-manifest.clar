;; this contract has a warning but is not in the manifest

(define-data-var counter int 0)

(define-public (add (n int))
  (begin
    (var-set counter (+ (var-get counter) n))
    (ok (var-get counter))
  )
)
