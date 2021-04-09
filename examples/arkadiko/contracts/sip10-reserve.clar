(impl-trait .vault-trait.vault-trait)
(use-trait mock-ft-trait .mock-ft-trait.mock-ft-trait)

;; errors
(define-constant err-unauthorized u1)
(define-constant err-transfer-failed u2)
(define-constant err-deposit-failed u5)
(define-constant err-withdraw-failed u6)
(define-constant err-mint-failed u7)

(define-read-only (calculate-xusd-count (token (string-ascii 12)) (ucollateral-amount uint) (collateral-type (string-ascii 12)))
  (let ((price-in-cents (contract-call? .oracle get-price token)))
    (let ((amount
      (/
        (* ucollateral-amount (get last-price-in-cents price-in-cents))
        (unwrap-panic (contract-call? .dao get-collateral-to-debt-ratio collateral-type))
      )))
      (ok amount)
    )
  )
)

(define-read-only (calculate-current-collateral-to-debt-ratio (token (string-ascii 12)) (debt uint) (ucollateral uint))
  (let ((price-in-cents (contract-call? .oracle get-price token)))
    (if (> debt u0)
      (ok (/ (* ucollateral (get last-price-in-cents price-in-cents)) debt))
      (err u0)
    )
  )
)

;; (match (print (ft-transfer? token ucollateral-amount sender (as-contract tx-sender)))
(define-public (collateralize-and-mint (token <mock-ft-trait>) (ucollateral-amount uint) (debt uint) (sender principal))
  (begin
    (asserts! (is-eq contract-caller .freddie) (err err-unauthorized))

    ;; token should be a trait e.g. 'SP3GWX3NE58KXHESRYE4DYQ1S31PQJTCRXB3PE9SB.arkadiko-token
    (match (contract-call? token transfer ucollateral-amount sender (as-contract tx-sender))
      success (ok debt)
      error (err err-transfer-failed)
    )
  )
)

(define-public (deposit (token <mock-ft-trait>) (additional-ucollateral-amount uint))
  (begin
    (asserts! (is-eq contract-caller .freddie) (err err-unauthorized))

    (match (print (contract-call? token transfer additional-ucollateral-amount tx-sender (as-contract tx-sender)))
      success (ok true)
      error (err err-deposit-failed)
    )
  )
)

(define-public (withdraw (token <mock-ft-trait>) (vault-owner principal) (ucollateral-amount uint))
  (begin
    (asserts! (is-eq contract-caller .freddie) (err err-unauthorized))

    (match (print (as-contract (contract-call? token transfer ucollateral-amount (as-contract tx-sender) vault-owner)))
      success (ok true)
      error (err err-withdraw-failed)
    )
  )
)

(define-public (mint (token (string-ascii 12)) (vault-owner principal) (ucollateral-amount uint) (current-debt uint) (extra-debt uint) (collateral-type (string-ascii 12)))
  (begin
    (asserts! (is-eq contract-caller .freddie) (err err-unauthorized))

    (let ((max-new-debt (- (unwrap-panic (calculate-xusd-count token ucollateral-amount collateral-type)) current-debt)))
      (if (>= max-new-debt extra-debt)
        (match (print (as-contract (contract-call? .xusd-token mint extra-debt vault-owner)))
          success (ok true)
          error (err err-mint-failed)
        )
        (err err-mint-failed)
      )
    )
  )
)

(define-public (burn (token <mock-ft-trait>) (vault-owner principal) (collateral-to-return uint))
  (begin
    (asserts! (is-eq contract-caller .freddie) (err err-unauthorized))

    (match (print (as-contract (contract-call? token transfer collateral-to-return (as-contract tx-sender) vault-owner)))
      transferred (ok true)
      error (err err-transfer-failed)
    )
  )
)

(define-public (redeem-collateral (token <mock-ft-trait>) (ucollateral uint) (owner principal))
  (begin
    (asserts! (is-eq contract-caller .auction-engine) (err err-unauthorized))
    (as-contract (contract-call? token transfer ucollateral (as-contract tx-sender) owner))
  )
)
