;; for now this is a fairly centralised Oracle, which is subject to failure.
;; Ideally, we implement a Chainlink Price Feed Oracle ASAP
(define-constant err-not-white-listed u51)

(define-data-var last-price-in-cents uint u0)
(define-data-var last-block uint u0)

(define-constant oracle-owner 'ST31HHVBKYCYQQJ5AQ25ZHA6W2A548ZADDQ6S16GP)

(define-public (update-price (price uint))
  (if (is-eq tx-sender oracle-owner)
    (begin
      (var-set last-price-in-cents price)
      (var-set last-block u0)
      (ok price)
    )
    (err err-not-white-listed)
  )
)

(define-read-only (get-price)
  { price: (var-get last-price-in-cents), height: (var-get last-block) }
)
