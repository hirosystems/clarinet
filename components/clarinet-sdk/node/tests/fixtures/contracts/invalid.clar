(define-data-var count uint u0)

(define-read-only  (get-count)
  (ok { count: (var-get count) })
)) ;; extra `)`
