;; Arkadiko DAO
;; 1. See all proposals
;; 2. Vote on a proposal
;; 3. Submit new proposal (hold token supply >= 1%)

;; errors
(define-constant err-not-enough-balance u1)
(define-constant err-transfer-failed u2)

;; proposal variables
(define-constant proposal-reserve 'S02J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKPVKG2CE)
(define-constant emergency-lockup-address 'S02J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKPVKG2CE)
(define-map proposals
  { id: uint }
  {
    id: uint,
    proposer: principal,
    is-open: bool,
    start-block-height: uint,
    end-block-height: uint,
    yes-votes: uint,
    no-votes: uint,
    token: (string-ascii 4),
    type: (string-ascii 200),
    changes: (list 10 (tuple (key (string-ascii 256)) (new-value uint))),
    details: (string-ascii 256)
  }
)
(define-data-var proposal-count uint u0)
(define-map votes-by-member { proposal-id: uint, member: principal } { has-voted: bool })
(define-data-var emergency-shutdown-activated bool false)

(define-read-only (get-proposal-by-id (proposal-id uint))
  (unwrap!
    (map-get? proposals {id: proposal-id})
    (tuple
      (id u0)
      (proposer 'S02J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKPVKG2CE)
      (is-open false)
      (start-block-height u0)
      (end-block-height u0)
      (yes-votes u0)
      (no-votes u0)
      (token "")
      (type "")
      (changes (list (tuple (key "") (new-value u0))))
      (details (unwrap-panic (as-max-len? "" u256)))
    )
  )
)

;; risk parameters
(define-map risk-parameters
  { token: (string-ascii 4) }
  {
    liquidation-ratio: uint,
    collateral-to-debt-ratio: uint,
    maximum-debt: uint,
    liquidation-penalty: uint,
    stability-fee: uint
  }
)

(define-map collateral-types
  { token: (string-ascii 4) }
  {
    name: (string-ascii 256),
    total-debt: uint
  }
)

(define-map proposal-types
  { type: (string-ascii 200) }
  {
    changes-keys: (list 10 (string-ascii 256))
  }
)

(define-read-only (get-risk-parameters (token (string-ascii 4)))
  (unwrap!
    (map-get? risk-parameters { token: token })
    (tuple
      (liquidation-ratio u0)
      (collateral-to-debt-ratio u0)
      (maximum-debt u0)
      (liquidation-penalty u0)
      (stability-fee u0)
    )
  )
)

(define-read-only (get-liquidation-ratio (token (string-ascii 4)))
  (ok (get liquidation-ratio (get-risk-parameters token)))
)

(define-read-only (get-collateral-to-debt-ratio (token (string-ascii 4)))
  (ok (get collateral-to-debt-ratio (get-risk-parameters token)))
)

(define-read-only (get-maximum-debt (token (string-ascii 4)))
  (ok (get maximum-debt (get-risk-parameters token)))
)

(define-read-only (get-liquidation-penalty (token (string-ascii 4)))
  (ok (get liquidation-penalty (get-risk-parameters token)))
)

(define-read-only (get-stability-fee (token (string-ascii 4)))
  (ok (get stability-fee (get-risk-parameters token)))
)

(define-read-only (get-stacker-yield)
  (ok u80) ;; stacker gets 80% of the yield
)

(define-read-only (get-governance-token-yield)
  (ok u10) ;; token holders get 10% of the yield
)

(define-read-only (get-governance-reserve-yield)
  (ok u10) ;; reserve gets 10% of the yield
)

(define-read-only (get-emergency-shutdown-activated)
  (ok (var-get emergency-shutdown-activated))
)

;; setters accessible only by DAO contract
(define-public (set-liquidation-ratio (token (string-ascii 4)) (ratio uint))
  (if (is-eq contract-caller 'ST2ZRX0K27GW0SP3GJCEMHD95TQGJMKB7G9Y0X1MH.dao)
    (begin
      (let ((params (get-risk-parameters token)))
        (map-set risk-parameters
          { token: token }
          {
            liquidation-ratio: ratio,
            collateral-to-debt-ratio: (get collateral-to-debt-ratio params),
            maximum-debt: (get maximum-debt params),
            liquidation-penalty: (get liquidation-penalty params),
            stability-fee: (get stability-fee params)
          }
        )
        (ok (get-liquidation-ratio token))
      )
    )
    (ok (get-liquidation-ratio token))
  )
)

(define-public (set-collateral-to-debt-ratio (token (string-ascii 4)) (ratio uint))
  (if (is-eq contract-caller 'ST2ZRX0K27GW0SP3GJCEMHD95TQGJMKB7G9Y0X1MH.dao)
    (begin
      (let ((params (get-risk-parameters token)))
        (map-set risk-parameters
          { token: token }
          {
            liquidation-ratio: (get liquidation-ratio params),
            collateral-to-debt-ratio: ratio,
            maximum-debt: (get maximum-debt params),
            liquidation-penalty: (get liquidation-penalty params),
            stability-fee: (get stability-fee params)
          }
        )
        (ok (get-liquidation-ratio token))
      )
    )
    (ok (get-liquidation-ratio token))
  )
)

(define-public (set-maximum-debt (token (string-ascii 4)) (debt uint))
  (if (is-eq contract-caller 'ST2ZRX0K27GW0SP3GJCEMHD95TQGJMKB7G9Y0X1MH.dao)
    (begin
      (let ((params (get-risk-parameters token)))
        (map-set risk-parameters
          { token: token }
          {
            liquidation-ratio: (get liquidation-ratio params),
            collateral-to-debt-ratio: (get collateral-to-debt-ratio params),
            maximum-debt: debt,
            liquidation-penalty: (get liquidation-penalty params),
            stability-fee: (get stability-fee params)
          }
        )
        (ok (get-liquidation-ratio token))
      )
    )
    (ok (get-liquidation-ratio token))
  )
)

(define-public (set-liquidation-penalty (token (string-ascii 4)) (penalty uint))
  (if (is-eq contract-caller 'ST2ZRX0K27GW0SP3GJCEMHD95TQGJMKB7G9Y0X1MH.dao)
    (begin
      (let ((params (get-risk-parameters token)))
        (map-set risk-parameters
          { token: token }
          {
            liquidation-ratio: (get liquidation-ratio params),
            collateral-to-debt-ratio: (get collateral-to-debt-ratio params),
            maximum-debt: (get maximum-debt params),
            liquidation-penalty: penalty,
            stability-fee: (get stability-fee params)
          }
        )
        (ok (get-liquidation-ratio token))
      )
    )
    (ok (get-liquidation-ratio token))
  )
)

(define-public (set-stability-fee (token (string-ascii 4)) (fee uint))
  (if (is-eq contract-caller 'ST2ZRX0K27GW0SP3GJCEMHD95TQGJMKB7G9Y0X1MH.dao)
    (begin
      (let ((params (get-risk-parameters token)))
        (map-set risk-parameters
          { token: token }
          {
            liquidation-ratio: (get liquidation-ratio params),
            collateral-to-debt-ratio: (get collateral-to-debt-ratio params),
            maximum-debt: (get maximum-debt params),
            liquidation-penalty: (get liquidation-penalty params),
            stability-fee: fee
          }
        )
        (ok (get-liquidation-ratio token))
      )
    )
    (ok (get-liquidation-ratio token))
  )
)

;; Start a proposal
;; Requires 1% of the supply in your wallet
;; Default voting period is 10 days (144 * 10 blocks)
;; 
(define-public (propose
    (start-block-height uint)
    (details (string-ascii 256))
    (type (string-ascii 200))
    (changes (list 10 (tuple (key (string-ascii 256)) (new-value uint))))
  )
  (let ((proposer-balance (unwrap-panic (contract-call? .arkadiko-token balance-of tx-sender))))
    (let ((supply (unwrap-panic (contract-call? .arkadiko-token total-supply))))
      (let ((proposal-id (+ u1 (var-get proposal-count))))
        (if (>= (* proposer-balance u100) supply)
          (begin
            (map-set proposals
              { id: proposal-id }
              {
                id: proposal-id,
                proposer: tx-sender,
                is-open: true,
                start-block-height: start-block-height,
                end-block-height: (+ start-block-height u1440),
                yes-votes: u0,
                no-votes: u0,
                token: "stx",
                type: type,
                changes: changes,
                details: details
              }
            )
            (ok true)
          )
          (err err-not-enough-balance) ;; need at least 1% 
        )
      )
    )
  )
)

(define-public (vote-for (proposal-id uint) (amount uint))
  (let ((proposal (get-proposal-by-id proposal-id)))
    (if 
      (and
        (is-none (map-get? votes-by-member { proposal-id: proposal-id, member: tx-sender }))
        (unwrap-panic (contract-call? .arkadiko-token transfer proposal-reserve amount))
      )
      (begin
        (map-set proposals
          { id: proposal-id }
          {
            id: proposal-id,
            proposer: (get proposer proposal),
            is-open: true,
            start-block-height: (get start-block-height proposal),
            end-block-height: (get end-block-height proposal),
            yes-votes: (+ amount (get yes-votes proposal)),
            no-votes: (get no-votes proposal),
            token: (get token proposal),
            type: (get type proposal),
            changes: (get changes proposal),
            details: (get details proposal)
          }
        )
        (map-set votes-by-member { proposal-id: proposal-id, member: tx-sender } { has-voted: true })
        (ok true)
      )
      (err err-transfer-failed)
    )
  )
)

(define-public (vote-against (proposal-id uint) (amount uint))
  (let ((proposal (get-proposal-by-id proposal-id)))
    (if 
      (and
        (is-none (map-get? votes-by-member { proposal-id: proposal-id, member: tx-sender }))
        (unwrap-panic (contract-call? .arkadiko-token transfer proposal-reserve amount))
      )
      (begin
        (map-set proposals
          { id: proposal-id }
          {
            id: proposal-id,
            proposer: (get proposer proposal),
            is-open: true,
            start-block-height: (get start-block-height proposal),
            end-block-height: (get end-block-height proposal),
            yes-votes: (get yes-votes proposal),
            no-votes: (+ amount (get no-votes proposal)),
            token: (get token proposal),
            type: (get type proposal),
            changes: (get changes proposal),
            details: (get details proposal)
          }
        )
        (map-set votes-by-member { proposal-id: proposal-id, member: tx-sender } { has-voted: true })
        (ok true)
      )
      (err err-transfer-failed)
    )
  )
)

;; Initialize the contract
(begin
  (try!
    (if (map-set risk-parameters
      { token: "stx" }
      {
        liquidation-ratio: u150,
        collateral-to-debt-ratio: u200,
        maximum-debt: u10000000,
        liquidation-penalty: u13,
        stability-fee: u0
      }
    )
      (ok true)
      (err false)
    )
  )
  (map-set collateral-types
    { token: "stx" }
    {
      name: "Stacks",
      total-debt: u0
    }
  )
  (map-set proposal-types
    { type: "change_risk_parameter" }
    {
      changes-keys: (list "token" "liquidation-ratio" "collateral-to-debt-ratio" "maximum-debt" "liquidation-penalty" "stability-fee")
    }
  )
  (map-set proposal-types
    { type: "add_collateral_type" }
    {
      changes-keys: (list "collateral_token" "collateral_name" "liquidation-ratio" "collateral-to-debt-ratio" "maximum-debt" "liquidation-penalty" "stability-fee")
    }
  )
  (map-set proposal-types
    { type: "stacking_distribution" }
    {
      changes-keys: (list "stacker_yield" "governance_token_yield" "governance_reserve_yield")
    }
  )
  (map-set proposal-types
    { type: "stacking_distribution" }
    {
      changes-keys: (list "stacker_yield" "governance_token_yield" "governance_reserve_yield")
    }
  )
  (map-set proposal-types
    { type: "emergency_shutdown" }
    {
      changes-keys: (list "")
    }
  )
  (print (get-liquidation-ratio "stx"))
)
