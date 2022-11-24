(define-data-var an-uint uint u1)

(define-public (args-and-lets (var0 uint))
  (let ((var1 (let ((var1_1 (var-get an-uint)) (var1_2 u2)) (+ var1_1 var1_2))))
    (if (> var1 var0)
      (let ((var2 u2)) (ok (+ var1 var2)))
      (let ((var2 u3)) (ok (+ var1 var2)))
    )
  )
)

(define-public (args-and-lets2 (var0 uint))
  (let ((var1 (let ((var1_1 (var-get an-uint)) (var1_2 u2)) (+ var1_1 var1_2))))
    (if (> var1 u2)
      (let ((var2 u2)) (ok (+ var1 var2)))
      (let ((var2 u3)) (ok (+ var1 var2)))
    )
  )
)


(define-private (match-senarios (optional-var (optional uint)) (response-var (response uint uint)))
  (begin
    (match optional-var some-var (print some-var) (print u0))

    (match response-var ok-var (print ok-var) err-var (print err-var))

    (ok true)
  )
)
