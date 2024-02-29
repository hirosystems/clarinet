(impl-trait .multiplier-trait.multiplier)

(define-read-only (multiply (a uint) (b uint))
  (ok (* a b))
)
