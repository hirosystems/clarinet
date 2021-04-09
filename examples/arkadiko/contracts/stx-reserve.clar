(impl-trait .vault-trait.vault-trait)
(use-trait mock-ft-trait .mock-ft-trait.mock-ft-trait)

;; errors
(define-constant err-unauthorized u1)
(define-constant err-transfer-failed u2)
(define-constant err-minter-failed u3)
(define-constant err-burn-failed u4)
(define-constant err-deposit-failed u5)
(define-constant err-withdraw-failed u6)
(define-constant err-mint-failed u7)

(define-read-only (get-risk-parameters)
  (ok (contract-call? .dao get-collateral-type-by-token "stx"))
)

;; MAIN LOGIC

;; calculate the amount of stablecoins to mint, based on posted STX amount
;; ustx-amount * stx-price-in-cents == dollar-collateral-posted-in-cents
;; (dollar-collateral-posted-in-cents / collateral-to-debt-ratio) == stablecoins to mint
(define-read-only (calculate-xusd-count (token (string-ascii 12)) (ustx-amount uint) (collateral-type (string-ascii 12)))
  (let ((stx-price-in-cents (contract-call? .oracle get-price token)))
    (let ((amount
      (/
        (* ustx-amount (get last-price-in-cents stx-price-in-cents))
        (unwrap-panic (contract-call? .dao get-collateral-to-debt-ratio collateral-type))
      )))
      (ok amount)
    )
  )
)

(define-read-only (calculate-current-collateral-to-debt-ratio (token (string-ascii 12)) (debt uint) (ustx uint))
  (let ((stx-price-in-cents (contract-call? .oracle get-price token)))
    (if (> debt u0)
      (ok (/ (* ustx (get last-price-in-cents stx-price-in-cents)) debt))
      (err u0)
    )
  )
)

;; accept collateral in STX tokens
;; save STX in stx-reserve-address
;; calculate price and collateralisation ratio
(define-public (collateralize-and-mint (token <mock-ft-trait>) (ustx-amount uint) (debt uint) (sender principal))
  (match (print (stx-transfer? ustx-amount sender (as-contract tx-sender)))
    success (ok debt)
    error (err err-transfer-failed)
  )
)

;; deposit extra collateral in vault
(define-public (deposit (token <mock-ft-trait>) (additional-ustx-amount uint))
  (match (print (stx-transfer? additional-ustx-amount tx-sender (as-contract tx-sender)))
    success (ok true)
    error (err err-deposit-failed)
  )
)

;; withdraw collateral (e.g. if collateral goes up in value)
(define-public (withdraw (token <mock-ft-trait>) (vault-owner principal) (ustx-amount uint))
  (begin
    (asserts! (is-eq contract-caller .freddie) (err err-unauthorized))

    (match (print (as-contract (stx-transfer? ustx-amount (as-contract tx-sender) vault-owner)))
      success (ok true)
      error (err err-withdraw-failed)
    )
  )
)

;; mint new tokens when collateral to debt allows it (i.e. > collateral-to-debt-ratio)
(define-public (mint (token (string-ascii 12)) (vault-owner principal) (ustx-amount uint) (current-debt uint) (extra-debt uint) (collateral-type (string-ascii 12)))
  (begin
    (asserts! (is-eq contract-caller .freddie) (err err-unauthorized))

    (let ((max-new-debt (- (unwrap-panic (calculate-xusd-count token ustx-amount collateral-type)) current-debt)))
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

;; burn stablecoin to free up STX tokens
;; method assumes position has not been liquidated
;; and thus collateral to debt ratio > liquidation ratio
(define-public (burn (token <mock-ft-trait>) (vault-owner principal) (collateral-to-return uint))
  (begin
    (asserts! (is-eq contract-caller .freddie) (err err-unauthorized))

    (match (print (as-contract (stx-transfer? collateral-to-return (as-contract tx-sender) vault-owner)))
      transferred (ok true)
      error (err err-transfer-failed)
    )
  )
)

;; liquidate a vault-address' vault
;; should only be callable by the liquidator smart contract address
;; the xUSD in the vault need to be covered & burnt
;; by xUSD earned through auctioning off the collateral in the current vault
;; 1. Mark vault as liquidated?
;; 2. Send collateral into the liquidator's liquidation reserve
(define-public (liquidate (token (string-ascii 12)) (stx-collateral uint) (current-debt uint))
  (if (is-eq contract-caller .freddie)
    (begin
      (let ((new-debt (/ (* (unwrap-panic (contract-call? .dao get-liquidation-penalty token)) current-debt) u100)))
        (ok (tuple (ustx-amount stx-collateral) (debt (+ new-debt current-debt))))
      )
    )
    (err err-unauthorized)
  )
)

(define-public (redeem-collateral (token <mock-ft-trait>) (stx-collateral uint) (owner principal))
  (begin
    (asserts! (is-eq contract-caller .auction-engine) (err err-unauthorized))
    (as-contract (stx-transfer? stx-collateral (as-contract tx-sender) owner))
  )
)
