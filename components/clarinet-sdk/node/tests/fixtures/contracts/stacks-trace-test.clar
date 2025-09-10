(define-public (sub-b
    (n uint)
    (m uint)
  )
  (ok (- n m))
)
(define-public (sub-a
    (n uint)
    (m uint)
  )
  (sub-b n m)
)
(define-public (call-sub
    (n uint)
    (m uint)
  )
  (sub-a n m)
)
