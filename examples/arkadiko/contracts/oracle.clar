;; for now this is a fairly centralised Oracle, which is subject to failure.
;; Ideally, we implement a Chainlink Price Feed Oracle ASAP
(define-constant err-not-white-listed u51)

(define-data-var last-price-in-cents uint u0)
(define-data-var last-block uint u0)

(define-constant oracle-owner 'ST238B5WSC8B8XETWDXMH7HZC2MJ2RNTYY15YY7SH)

(define-map prices
  { token: (string-ascii 12) }
  {
    last-price-in-cents: uint,
    last-block: uint
  }
)

(define-public (update-price (token (string-ascii 12)) (price uint))
  (if (is-eq tx-sender oracle-owner) ;; use assert instead
    (begin
      (map-set prices { token: token } { last-price-in-cents: price, last-block: u0 })
      (ok price)
    )
    (err err-not-white-listed)
  )
)

(define-read-only (get-price (token (string-ascii 12)))
  (unwrap! (map-get? prices {token: token }) { last-price-in-cents: u0, last-block: u0 })
)
