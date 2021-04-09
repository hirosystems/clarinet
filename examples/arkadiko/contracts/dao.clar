(use-trait vault-trait .vault-trait.vault-trait)
;; Arkadiko DAO
;; 1. See all proposals
;; 2. Vote on a proposal
;; 3. Submit new proposal (hold token supply >= 1%)
;; 4. Initiate Stacking

;; errors
(define-constant err-not-enough-balance u1)
(define-constant err-transfer-failed u2)
(define-constant err-unauthorized u401)
(define-constant status-ok u200)

;; proposal variables
(define-constant diko-reserve 'S02J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKPVKG2CE)
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
    token: (string-ascii 12),
    collateral-type: (string-ascii 12),
    type: (string-ascii 200),
    changes: (list 10 (tuple (key (string-ascii 256)) (new-value uint))),
    details: (string-ascii 256)
  }
)
(define-data-var proposal-count uint u0)
(define-data-var proposal-ids (list 220 uint) (list u0))
(define-map votes-by-member { proposal-id: uint, member: principal } { vote-count: uint })
(define-data-var emergency-shutdown-activated bool false)
(define-data-var stacker-yield uint u80)
(define-data-var governance-token-yield uint u80)
(define-data-var governance-reserve-yield uint u80)
(define-data-var maximum-debt-surplus uint u100000000)

(define-read-only (get-votes-by-member-by-id (proposal-id uint) (member principal))
  (unwrap!
    (map-get? votes-by-member {proposal-id: proposal-id, member: member})
    (tuple
      (vote-count u0)
    )
  )
)

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
      (collateral-type "")
      (type "")
      (changes (list (tuple (key "") (new-value u0))))
      (details (unwrap-panic (as-max-len? "" u256)))
    )
  )
)

(define-read-only (get-proposals)
  (ok (map get-proposal-by-id (var-get proposal-ids)))
)

(define-read-only (get-proposal-ids)
  (ok (var-get proposal-ids))
)

(define-read-only (get-collateral-type-by-token (token (string-ascii 12)))
  (unwrap!
    (map-get? collateral-types { token: token })
    (tuple
      (name "")
      (token "")
      (token-type "")
      (url "")
      (total-debt u0)
      (liquidation-ratio u0)
      (collateral-to-debt-ratio u0)
      (maximum-debt u0)
      (liquidation-penalty u0)
      (stability-fee u0)
      (stability-fee-apy u0)
    )
  )
)

(define-map collateral-types
  { token: (string-ascii 12) }
  {
    name: (string-ascii 256),
    token: (string-ascii 12),
    token-type: (string-ascii 12),
    url: (string-ascii 256),
    total-debt: uint,
    liquidation-ratio: uint,
    collateral-to-debt-ratio: uint,
    maximum-debt: uint,
    liquidation-penalty: uint,
    stability-fee: uint,
    stability-fee-apy: uint
  }
)

(define-map proposal-types
  { type: (string-ascii 200) }
  {
    changes-keys: (list 10 (string-ascii 256))
  }
)

(define-read-only (get-liquidation-ratio (token (string-ascii 12)))
  (ok (get liquidation-ratio (get-collateral-type-by-token token)))
)

(define-read-only (get-collateral-to-debt-ratio (token (string-ascii 12)))
  (ok (get collateral-to-debt-ratio (get-collateral-type-by-token token)))
)

(define-read-only (get-maximum-debt (token (string-ascii 12)))
  (ok (get maximum-debt (get-collateral-type-by-token token)))
)

(define-read-only (get-total-debt (token (string-ascii 12)))
  (ok (get total-debt (get-collateral-type-by-token token)))
)

(define-read-only (get-liquidation-penalty (token (string-ascii 12)))
  (ok (get liquidation-penalty (get-collateral-type-by-token token)))
)

(define-read-only (get-stability-fee (token (string-ascii 12)))
  (ok (get stability-fee (get-collateral-type-by-token token)))
)

(define-read-only (get-stability-fee-apy (token (string-ascii 12)))
  (ok (get stability-fee-apy (get-collateral-type-by-token token)))
)

(define-read-only (get-stacker-yield)
  (ok (var-get stacker-yield)) ;; stacker gets 80% of the yield
)

(define-read-only (get-governance-token-yield)
  (ok (var-get governance-token-yield)) ;; token holders get 10% of the yield
)

(define-read-only (get-governance-reserve-yield)
  (ok (var-get governance-reserve-yield)) ;; reserve gets 10% of the yield
)

(define-read-only (get-emergency-shutdown-activated)
  (ok (var-get emergency-shutdown-activated))
)

(define-read-only (get-maximum-debt-surplus)
  (ok (var-get maximum-debt-surplus))
)

;; setters accessible only by DAO contract
(define-public (add-collateral-type (token (string-ascii 12)) (collateral-type (string-ascii 12)))
  (if (is-eq contract-caller .dao)
    (begin
      (map-set collateral-types
        { token: collateral-type }
        {
          name: "Stacks",
          token: token,
          token-type: collateral-type,
          url: "https://www.stacks.co/",
          total-debt: u0,
          liquidation-ratio: u150,
          collateral-to-debt-ratio: u200,
          maximum-debt: u100000000000000,
          liquidation-penalty: u13,
          stability-fee: u1363, ;; 0.001363077% daily percentage == 1% APY
          stability-fee-apy: u50 ;; 50 basis points
        }
      )
      (ok true)
    )
    (err false)
  )
)

(define-public (add-debt-to-collateral-type (token (string-ascii 12)) (debt uint))
  (let ((collateral-type (get-collateral-type-by-token token)))
    (map-set collateral-types
      { token: token }
      {
        name: (get name collateral-type),
        token: (get token collateral-type),
        token-type: (get token-type collateral-type),
        url: (get url collateral-type),
        total-debt: (+ debt (get total-debt collateral-type)),
        liquidation-ratio: (get liquidation-ratio collateral-type),
        collateral-to-debt-ratio: (get collateral-to-debt-ratio collateral-type),
        maximum-debt: (get maximum-debt collateral-type),
        liquidation-penalty: (get liquidation-penalty collateral-type),
        stability-fee: (get stability-fee collateral-type),
        stability-fee-apy: (get stability-fee-apy collateral-type)
      }
    )
    (ok debt)
  )
)

(define-public (subtract-debt-from-collateral-type (token (string-ascii 12)) (debt uint))
  (let ((collateral-type (get-collateral-type-by-token token)))
    (map-set collateral-types
      { token: token }
      {
        name: (get name collateral-type),
        token: (get token collateral-type),
        token-type: (get token-type collateral-type),
        url: (get url collateral-type),
        total-debt: (- debt (get total-debt collateral-type)),
        liquidation-ratio: (get liquidation-ratio collateral-type),
        collateral-to-debt-ratio: (get collateral-to-debt-ratio collateral-type),
        maximum-debt: (get maximum-debt collateral-type),
        liquidation-penalty: (get liquidation-penalty collateral-type),
        stability-fee: (get stability-fee collateral-type),
        stability-fee-apy: (get stability-fee-apy collateral-type)
      }
    )
    (ok debt)
  )
)

(define-public (set-liquidation-ratio (token (string-ascii 12)) (ratio uint))
  (if (is-eq contract-caller .dao)
    (begin
      (let ((params (get-collateral-type-by-token token)))
        (map-set collateral-types
          { token: token }
          {
            name: (get name params),
            token: (get token params),
            token-type: (get token-type params),
            url: (get url params),
            total-debt: (get total-debt params),
            liquidation-ratio: ratio,
            collateral-to-debt-ratio: (get collateral-to-debt-ratio params),
            maximum-debt: (get maximum-debt params),
            liquidation-penalty: (get liquidation-penalty params),
            stability-fee: (get stability-fee params),
            stability-fee-apy: (get stability-fee-apy params)
          }
        )
        (ok (get-liquidation-ratio token))
      )
    )
    (ok (get-liquidation-ratio token))
  )
)

(define-public (set-collateral-to-debt-ratio (token (string-ascii 12)) (ratio uint))
  (if (is-eq contract-caller .dao)
    (begin
      (let ((params (get-collateral-type-by-token token)))
        (map-set collateral-types
          { token: token }
          {
            name: (get name params),
            token: (get token params),
            token-type: (get token-type params),
            url: (get url params),
            total-debt: (get total-debt params),
            liquidation-ratio: (get liquidation-ratio params),
            collateral-to-debt-ratio: ratio,
            maximum-debt: (get maximum-debt params),
            liquidation-penalty: (get liquidation-penalty params),
            stability-fee: (get stability-fee params),
            stability-fee-apy: (get stability-fee-apy params)
          }
        )
        (ok (get-liquidation-ratio token))
      )
    )
    (ok (get-liquidation-ratio token))
  )
)

(define-public (set-maximum-debt (token (string-ascii 12)) (debt uint))
  (if (is-eq contract-caller .dao)
    (begin
      (let ((params (get-collateral-type-by-token token)))
        (map-set collateral-types
          { token: token }
          {
            name: (get name params),
            token: (get token params),
            token-type: (get token-type params),
            url: (get url params),
            total-debt: (get total-debt params),
            liquidation-ratio: (get liquidation-ratio params),
            collateral-to-debt-ratio: (get collateral-to-debt-ratio params),
            maximum-debt: debt,
            liquidation-penalty: (get liquidation-penalty params),
            stability-fee: (get stability-fee params),
            stability-fee-apy: (get stability-fee-apy params)
          }
        )
        (ok (get-liquidation-ratio token))
      )
    )
    (ok (get-liquidation-ratio token))
  )
)

(define-public (set-liquidation-penalty (token (string-ascii 12)) (penalty uint))
  (if (is-eq contract-caller .dao)
    (begin
      (let ((params (get-collateral-type-by-token token)))
        (map-set collateral-types
          { token: token }
          {
            name: (get name params),
            token: (get token params),
            token-type: (get token-type params),
            url: (get url params),
            total-debt: (get total-debt params),
            liquidation-ratio: (get liquidation-ratio params),
            collateral-to-debt-ratio: (get collateral-to-debt-ratio params),
            maximum-debt: (get maximum-debt params),
            liquidation-penalty: penalty,
            stability-fee: (get stability-fee params),
            stability-fee-apy: (get stability-fee-apy params)
          }
        )
        (ok (get-liquidation-ratio token))
      )
    )
    (ok (get-liquidation-ratio token))
  )
)

(define-public (set-stability-fee (token (string-ascii 12)) (fee uint) (fee-apy uint))
  (if (is-eq contract-caller .dao)
    (begin
      (let ((params (get-collateral-type-by-token token)))
        (map-set collateral-types
          { token: token }
          {
            name: (get name params),
            token: (get token params),
            token-type: (get token-type params),
            url: (get url params),
            total-debt: (get total-debt params),
            liquidation-ratio: (get liquidation-ratio params),
            collateral-to-debt-ratio: (get collateral-to-debt-ratio params),
            maximum-debt: (get maximum-debt params),
            liquidation-penalty: (get liquidation-penalty params),
            stability-fee: fee,
            stability-fee-apy: fee-apy
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
    (token (string-ascii 12))
    (collateral-type (string-ascii 12))
  )
  (let ((proposer-balance (unwrap-panic (contract-call? .arkadiko-token get-balance-of tx-sender))))
    (let ((supply (unwrap-panic (contract-call? .arkadiko-token get-total-supply))))
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
                token: token,
                collateral-type: collateral-type,
                type: type,
                changes: changes,
                details: details
              }
            )
            (var-set proposal-count proposal-id)
            (var-set proposal-ids (unwrap-panic (as-max-len? (append (var-get proposal-ids) proposal-id) u220)))
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
    (asserts! (is-eq (get is-open proposal) true) (err err-unauthorized))
    (asserts! (>= block-height (get start-block-height proposal)) (err err-unauthorized))

    (let ((vote-count (get vote-count (get-votes-by-member-by-id proposal-id tx-sender))))
      (if (unwrap-panic (contract-call? .arkadiko-token transfer amount tx-sender proposal-reserve))
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
              collateral-type: (get collateral-type proposal),
              type: (get type proposal),
              changes: (get changes proposal),
              details: (get details proposal)
            }
          )
          (map-set votes-by-member { proposal-id: proposal-id, member: tx-sender } { vote-count: (+ vote-count amount) })
          (ok status-ok)
        )
        (err err-transfer-failed)
      )
    )
  )
)

(define-public (vote-against (proposal-id uint) (amount uint))
  (let ((proposal (get-proposal-by-id proposal-id)))
    (asserts! (is-eq (get is-open proposal) true) (err err-unauthorized))
    (asserts! (>= block-height (get start-block-height proposal)) (err err-unauthorized))

    (let ((vote-count (get vote-count (get-votes-by-member-by-id proposal-id tx-sender))))
      (if (unwrap-panic (contract-call? .arkadiko-token transfer amount tx-sender proposal-reserve))
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
              collateral-type: (get collateral-type proposal),
              type: (get type proposal),
              changes: (get changes proposal),
              details: (get details proposal)
            }
          )
          (map-set votes-by-member { proposal-id: proposal-id, member: tx-sender } { vote-count: (+ vote-count amount) })
          (ok status-ok)
        )
        (err err-transfer-failed)
      )
    )
  )
)

(define-public (end-proposal (proposal-id uint))
  (let ((proposal (get-proposal-by-id proposal-id)))
    (asserts! (not (is-eq (get id proposal) u0)) (err err-unauthorized))
    (asserts! (is-eq (get is-open proposal) true) (err err-unauthorized))
    (asserts! (>= block-height (get end-block-height proposal)) (err err-unauthorized))

    (map-set proposals
      { id: proposal-id }
      {
        id: proposal-id,
        proposer: (get proposer proposal),
        is-open: false,
        start-block-height: (get start-block-height proposal),
        end-block-height: (get end-block-height proposal),
        yes-votes: (get yes-votes proposal),
        no-votes: (get no-votes proposal),
        token: (get token proposal),
        collateral-type: (get collateral-type proposal),
        type: (get type proposal),
        changes: (get changes proposal),
        details: (get details proposal)
      }
    )

    (ok status-ok)
  )
)

;; (define-private (return-diko (data (tuple (proposal-id uint) (member principal))))
;;   (map-set votes-by-member { proposal-id: proposal-id, member: principal } { vote-count: (+ vote-count amount) })
;;   (ok true)
;; )

;; DAO can initiate stacking for the STX reserve
(define-public (stack)
  (ok true)
)

;; Pay all parties:
;; - Owners of vaults
;; - DAO Reserve
;; - Owners of gov tokens
(define-public (payout)
  (ok true)
)

;; Initialize the contract
(begin
  (map-set collateral-types
    { token: "stx-a" }
    {
      name: "Stacks",
      token: "STX",
      token-type: "STX-A",
      url: "https://www.stacks.co/",
      total-debt: u0,
      liquidation-ratio: u150,
      collateral-to-debt-ratio: u200,
      maximum-debt: u100000000000000,
      liquidation-penalty: u13,
      stability-fee: u1363, ;; 0.001363077% daily percentage == 1% APY
      stability-fee-apy: u50 ;; 50 basis points
    }
  )
  (map-set collateral-types
    { token: "stx-b" }
    {
      name: "Stacks",
      token: "STX",
      token-type: "STX-B",
      url: "https://www.stacks.co/",
      total-debt: u0,
      liquidation-ratio: u110,
      collateral-to-debt-ratio: u200,
      maximum-debt: u10000000000000,
      liquidation-penalty: u25,
      stability-fee: u2726, ;; 0.002726155% daily percentage == 1% APY
      stability-fee-apy: u100 ;; 100 basis points
    }
  )
  (map-set collateral-types
    { token: "diko-a" }
    {
      name: "Arkadiko",
      token: "DIKO",
      token-type: "DIKO-A",
      url: "https://www.arkadiko.finance/",
      total-debt: u0,
      liquidation-ratio: u200,
      collateral-to-debt-ratio: u300,
      maximum-debt: u10000000000000,
      liquidation-penalty: u13,
      stability-fee: u2726, ;; 0.002726155% daily percentage == 1% APY
      stability-fee-apy: u100
    }
  )
  (map-set proposal-types
    { type: "change_risk_parameter" }
    {
      changes-keys: (list "liquidation-ratio" "collateral-to-debt-ratio" "maximum-debt" "liquidation-penalty" "stability-fee-apy" "minimum-vault-debt")
    }
  )
  (map-set proposal-types
    { type: "add_collateral_type" }
    {
      changes-keys: (list
        "collateral_token"
        "collateral_name"
        "liquidation-ratio"
        "collateral-to-debt-ratio"
        "maximum-debt"
        "liquidation-penalty"
        "stability-fee-apy"
        "minimum-vault-debt"
      )
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
