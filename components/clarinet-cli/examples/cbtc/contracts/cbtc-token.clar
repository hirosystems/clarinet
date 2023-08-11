(impl-trait 'SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.sip-010-trait-ft-standard.sip-010-trait)

(define-fungible-token cbtc)

;; get the token balance of owner
(define-read-only (get-balance (owner principal))
  (begin
    (ok (ft-get-balance cbtc owner))))

;; returns the total number of tokens
(define-read-only (get-total-supply)
  (ok (ft-get-supply cbtc)))

;; returns the token name
(define-read-only (get-name)
  (ok "cBTC"))

;; the symbol or "ticker" for this token
(define-read-only (get-symbol)
  (ok "cBTC"))

;; the number of decimals used
(define-read-only (get-decimals)
  (ok u8))

;; Transfers tokens to a recipient
(define-public (transfer (amount uint) (sender principal) (recipient principal) (memo (optional (buff 34))))
  (if (is-eq tx-sender sender)
    (begin
      (try! (ft-transfer? cbtc amount sender recipient))
      (print memo)
      (ok true))
    (err u4)))

(define-public (get-token-uri)
  (ok (some u"https://example.com")))

(define-constant AUTHORITY 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5)

(define-public (mint (amount uint) (recipient principal))
  (begin
    (asserts! (is-eq tx-sender AUTHORITY) (err u0))
    (ft-mint? cbtc amount recipient)))

(define-public (burn (amount uint))
  (begin
    (ft-burn? cbtc amount tx-sender)))
