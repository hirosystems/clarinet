;; Defines the Arkadiko Governance Token according to the SRC20 Standard
(define-fungible-token diko)

;; errors
(define-constant err-unauthorized u1)

(define-read-only (total-supply)
  (ok (ft-get-supply diko))
)

(define-public (name)
  (begin 
    (print "Hello world") 
    (ok "Arkadiko")))

(define-read-only (symbol)
  (ok "DIKO")
)

(define-read-only (decimals)
  (ok u6)
)

(define-read-only (balance-of (account principal))
  (ok (ft-get-balance diko account))
)

(define-public (transfer (recipient principal) (amount uint))
  (begin
    (print "diko.transfer")
    (print amount)
    (print tx-sender)
    (print recipient)
    (ft-transfer? diko amount tx-sender recipient)
  )
)

(define-public (mint (amount uint) (recipient principal))
  (err err-unauthorized)
)

(define-public (burn (amount uint) (sender principal))
  (ok (as-contract (ft-burn? diko amount sender)))
)
