;; Defines the xUSD Stablecoin according to the SRC20 Standard
(define-fungible-token xusd)

(define-constant mint-owner 'ST31HHVBKYCYQQJ5AQ25ZHA6W2A548ZADDQ6S16GP)
(define-constant err-burn-failed u1)

(define-read-only (total-supply)
  (ok (ft-get-supply xusd))
)

(define-read-only (name)
  (ok "xUSD")
)

(define-read-only (symbol)
  (ok "xUSD")
)

(define-read-only (decimals)
  (ok u6)
)

(define-read-only (balance-of (account principal))
  (ok (ft-get-balance xusd account))
)

(define-public (transfer (recipient principal) (amount uint))
  (begin
    (print "xusd.transfer")
    (print amount)
    (print tx-sender)
    (print recipient)
    (ft-transfer? xusd amount tx-sender recipient)
  )
)

(define-public (mint (amount uint) (recipient principal))
  (begin
    (print recipient)
    (print amount)
    (print tx-sender)
    (print contract-caller)
    (print mint-owner)
    (if
      (and
        (is-eq contract-caller 'ST31HHVBKYCYQQJ5AQ25ZHA6W2A548ZADDQ6S16GP.stx-reserve)
        (is-ok (ft-mint? xusd amount recipient))
      )
      (ok amount)
      (err false)
    )
  )
)

(define-public (burn (amount uint) (sender principal))
  ;; burn the xusd stablecoin and return STX
  (begin
    (if 
      (and
        (is-eq contract-caller 'ST31HHVBKYCYQQJ5AQ25ZHA6W2A548ZADDQ6S16GP.stx-reserve)
        (is-ok (ft-transfer? xusd amount sender mint-owner))
      )
      ;; TODO: burn does not work, so we will transfer for now. Burn tx gets stuck at "pending"
      ;; (ok (as-contract (ft-burn? xusd amount mint-owner)))
      (ok true)
      (err err-burn-failed)
    )
  )
)

;; Initialize the contract
(begin
  (try! (ft-mint? xusd u20 'SP2J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKNRV9EJ7)) ;; alice
  (try! (ft-mint? xusd u10 'S02J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKPVKG2CE)) ;; bob
)
