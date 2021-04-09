;; implements a trait that allows collateral of any token (e.g. stx, bitcoin)
(use-trait mock-ft-trait .mock-ft-trait.mock-ft-trait)

(define-trait vault-trait
  (
    ;; calculate stablecoin count to mint from posted collateral
    (calculate-xusd-count ((string-ascii 12) uint (string-ascii 12)) (response uint uint))

    ;; calculate the current collateral to debt ratio against USD value of collateral
    (calculate-current-collateral-to-debt-ratio ((string-ascii 12) uint uint) (response uint uint))

    ;; collateralize tokens and mint stablecoin according to collateral-to-debt ratio
    (collateralize-and-mint (<mock-ft-trait> uint uint principal) (response uint uint))

    ;; deposit extra collateral
    (deposit (<mock-ft-trait> uint) (response bool uint))

    ;; withdraw excess collateral
    (withdraw (<mock-ft-trait> principal uint) (response bool uint))

    ;; mint additional stablecoin
    (mint ((string-ascii 12) principal uint uint uint (string-ascii 12)) (response bool uint))

    ;; burn all the stablecoin in the vault of tx-sender and return collateral
    (burn (<mock-ft-trait> principal uint) (response bool uint))

    ;; liquidate the vault of principal. only callable by liquidator smart contract
    ;; (liquidate (uint uint) (response (tuple (ustx-amount uint) (debt uint)) uint))

    ;; redeem collateral after an auction ran
    (redeem-collateral (<mock-ft-trait> uint principal) (response bool uint))
  )
)
