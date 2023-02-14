;; MAP

(define-private (square (n uint))
  (* n n)
)

(print (map square (list u1 u2 u3 u4)))
;; => (list u1 u4 u9 u16)

;; FILTER

(define-private (is-even (n uint))
  (is-eq (mod n u2) u0)
)

(print (filter is-even (list u1 u2 u10 u51 u42)))
;; => (list u2 u10 u42)

;; FOLD

(define-private (return-biggest (number uint) (result uint))
  (if (> number result) number result)
)

(define-private (find-biggest (numbers (list 10 uint)))
  (fold return-biggest numbers u0)
)

(find-biggest (list u2 u12 u3 u4 u5 u9 u2 u10 u0 u2))
;; u12
