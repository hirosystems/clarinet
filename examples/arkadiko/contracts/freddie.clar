(use-trait vault-trait .vault-trait.vault-trait)
(use-trait mock-ft-trait .mock-ft-trait.mock-ft-trait)

;; Freddie - The Vault Manager
;; Freddie is an abstraction layer that interacts with collateral type reserves (initially only STX)
;; Ideally, collateral reserves should never be called from outside. Only manager layers should be interacted with from clients

;; errors
(define-constant err-unauthorized u1)
(define-constant err-transfer-failed u2)
(define-constant err-minter-failed u3)
(define-constant err-burn-failed u4)
(define-constant err-deposit-failed u5)
(define-constant err-withdraw-failed u6)
(define-constant err-mint-failed u7)
(define-constant err-liquidation-failed u8)
(define-constant err-insufficient-collateral u9)
(define-constant err-maximum-debt-reached u10)

;; constants
(define-constant blocks-per-day u144)
(define-constant mint-owner 'ST238B5WSC8B8XETWDXMH7HZC2MJ2RNTYY15YY7SH)

;; Map of vault entries
;; The entry consists of a user principal with their collateral and debt balance
(define-map vaults { id: uint } {
  id: uint,
  owner: principal,
  collateral: uint,
  collateral-type: (string-ascii 12), ;; e.g. STX-A, STX-B, BTC-A etc (represents the collateral class)
  collateral-token: (string-ascii 12), ;; e.g. STX, BTC etc (represents the symbol of the collateral)
  debt: uint,
  created-at-block-height: uint,
  updated-at-block-height: uint,
  stability-fee: uint,
  stability-fee-last-accrued: uint, ;; indicates the block height at which the stability fee was last accrued (calculated)
  is-liquidated: bool,
  auction-ended: bool,
  leftover-collateral: uint
})
(define-map vault-entries { user: principal } { ids: (list 1200 uint) })
(define-data-var last-vault-id uint u0)

;; getters
(define-read-only (get-vault-by-id (id uint))
  (unwrap!
    (map-get? vaults { id: id })
    (tuple
      (id u0)
      (owner 'ST238B5WSC8B8XETWDXMH7HZC2MJ2RNTYY15YY7SH)
      (collateral u0)
      (collateral-type "")
      (collateral-token "")
      (debt u0)
      (created-at-block-height u0)
      (updated-at-block-height u0)
      (stability-fee u0)
      (stability-fee-last-accrued u0)
      (is-liquidated false)
      (leftover-collateral u0)
    )
  )
)

(define-read-only (get-vault-entries (user principal))
  (unwrap! (map-get? vault-entries { user: user }) (tuple (ids (list u0) )))
)

(define-read-only (get-last-vault-id)
  (var-get last-vault-id)
)

(define-read-only (get-vaults (user principal))
  (let ((entries (get ids (get-vault-entries user))))
    (ok (map get-vault-by-id entries))
  )
)

(define-read-only (get-collateral-type-for-vault (vault-id uint))
  (let ((vault (get-vault-by-id vault-id)))
    (get collateral-type vault)
  )
)

(define-read-only (calculate-current-collateral-to-debt-ratio (vault-id uint))
  (let ((vault (get-vault-by-id vault-id)))
    (if (is-eq (get is-liquidated vault) true)
      (ok u0)
      (begin
        (let ((stx-price-in-cents (contract-call? .oracle get-price (get collateral-token vault))))
          (if (> (get debt vault) u0)
            (ok (/ (* (get collateral vault) (get last-price-in-cents stx-price-in-cents)) (get debt vault)))
            (err u0)
          )
        )
      )
    )
  )
)

(define-public (collateralize-and-mint
    (collateral-amount uint)
    (debt uint)
    (sender principal) ;; why?
    (collateral-type (string-ascii 12))
    (collateral-token (string-ascii 12))
    (reserve <vault-trait>)
    (ft <mock-ft-trait>)
  )
  (let ((ratio (unwrap-panic (contract-call? reserve calculate-current-collateral-to-debt-ratio collateral-token debt collateral-amount))))
    (asserts! (is-eq tx-sender sender) (err err-unauthorized))
    (asserts! (>= ratio (unwrap-panic (contract-call? .dao get-liquidation-ratio collateral-type))) (err err-insufficient-collateral))
    (asserts!
      (<
        (unwrap-panic (contract-call? .dao get-total-debt collateral-type))
        (unwrap-panic (contract-call? .dao get-maximum-debt collateral-type))
      )
      (err err-maximum-debt-reached)
    )
    (try! (contract-call? reserve collateralize-and-mint ft collateral-amount debt sender))

    (if (is-ok (as-contract (contract-call? .xusd-token mint debt sender)))
      (begin
        (let ((vault-id (+ (var-get last-vault-id) u1)))
          (let ((entries (get ids (get-vault-entries sender))))
            (map-set vault-entries { user: sender } { ids: (unwrap-panic (as-max-len? (append entries vault-id) u1200)) })
            (map-set vaults
              { id: vault-id }
              {
                id: vault-id,
                owner: sender,
                collateral: collateral-amount,
                collateral-type: collateral-type,
                collateral-token: collateral-token,
                debt: debt,
                created-at-block-height: block-height,
                updated-at-block-height: block-height,
                stability-fee: u0,
                stability-fee-last-accrued: block-height,
                is-liquidated: false,
                auction-ended: false,
                leftover-collateral: u0
              }
            )
            (var-set last-vault-id vault-id)
            (let ((result (contract-call? .dao add-debt-to-collateral-type collateral-type debt)))
              (ok debt)
            )
          )
        )
      )
      (err err-minter-failed)
    )
  )
)

(define-public (deposit (vault-id uint) (uamount uint) (reserve <vault-trait>) (ft <mock-ft-trait>))
  (let ((vault (get-vault-by-id vault-id)))
    (if (unwrap-panic (contract-call? reserve deposit ft uamount))
      (begin
        (let ((new-collateral (+ uamount (get collateral vault))))
          (map-set vaults
            { id: vault-id }
            {
              id: vault-id,
              owner: tx-sender,
              collateral: new-collateral,
              collateral-type: (get collateral-type vault),
              collateral-token: (get collateral-token vault),
              debt: (get debt vault),
              created-at-block-height: (get created-at-block-height vault),
              updated-at-block-height: block-height,
              stability-fee: (get stability-fee vault),
              stability-fee-last-accrued: (get stability-fee-last-accrued vault),
              is-liquidated: false,
              auction-ended: false,
              leftover-collateral: u0
            }
          )
          (ok true)
        )
      )
      (err err-deposit-failed)
    )
  )
)

(define-public (withdraw (vault-id uint) (uamount uint) (reserve <vault-trait>) (ft <mock-ft-trait>))
  (let ((vault (get-vault-by-id vault-id)))
    (asserts! (is-eq tx-sender (get owner vault)) (err err-unauthorized))
    (asserts! (> uamount u0) (err err-insufficient-collateral))
    (asserts! (<= uamount (get collateral vault)) (err err-insufficient-collateral))

    (let ((ratio (unwrap-panic (contract-call? reserve calculate-current-collateral-to-debt-ratio (get collateral-token vault) (get debt vault) (- (get collateral vault) uamount)))))
      (asserts! (>= ratio (unwrap-panic (contract-call? .dao get-collateral-to-debt-ratio "stx"))) (err err-insufficient-collateral))

      (if (unwrap-panic (contract-call? reserve withdraw ft (get owner vault) uamount))
        (begin
          (let ((new-collateral (- (get collateral vault) uamount)))
            (map-set vaults
              { id: vault-id }
              {
                id: vault-id,
                owner: tx-sender,
                collateral: new-collateral,
                collateral-type: (get collateral-type vault),
                collateral-token: (get collateral-token vault),
                debt: (get debt vault),
                created-at-block-height: (get created-at-block-height vault),
                updated-at-block-height: block-height,
                stability-fee: (get stability-fee vault),
                stability-fee-last-accrued: (get stability-fee-last-accrued vault),
                is-liquidated: false,
                auction-ended: false,
                leftover-collateral: u0
              }
            )
            (ok true)
          )
        )
        (err err-withdraw-failed)
      )
    )
  )
)

(define-public (mint (vault-id uint) (extra-debt uint) (reserve <vault-trait>))
  (let ((vault (get-vault-by-id vault-id)))
    (asserts! (is-eq tx-sender (get owner vault)) (err err-unauthorized))
    (asserts!
      (<
        (unwrap-panic (contract-call? .dao get-total-debt (get collateral-type vault)))
        (unwrap-panic (contract-call? .dao get-maximum-debt (get collateral-type vault)))
      )
      (err err-maximum-debt-reached)
    )

    (if (unwrap! (contract-call? reserve mint (get collateral-token vault) (get owner vault) (get collateral vault) (get debt vault) extra-debt (get collateral-type vault)) (err u5))
      (begin
        (let ((new-total-debt (+ extra-debt (get debt vault))))
          (map-set vaults
            { id: vault-id }
            {
              id: vault-id,
              owner: (get owner vault),
              collateral: (get collateral vault),
              collateral-type: (get collateral-type vault),
              collateral-token: (get collateral-token vault),
              debt: new-total-debt,
              created-at-block-height: (get created-at-block-height vault),
              updated-at-block-height: block-height,
              stability-fee: (get stability-fee vault),
              stability-fee-last-accrued: (get stability-fee-last-accrued vault),
              is-liquidated: false,
              auction-ended: false,
              leftover-collateral: u0
            }
          )
          (let ((result (contract-call? .dao add-debt-to-collateral-type (get collateral-type vault) extra-debt)))
            (ok true)
          )
        )
      )
      (err err-mint-failed)
    )
  )
)

(define-public (burn (vault-id uint) (debt uint) (reserve <vault-trait>) (ft <mock-ft-trait>))
  (let ((vault (get-vault-by-id vault-id)))
    (asserts! (is-eq tx-sender (get owner vault)) (err err-unauthorized))
    (asserts! (is-eq u0 (get stability-fee vault)) (err err-unauthorized))

    (if (is-ok (contract-call? .xusd-token burn debt (get owner vault)))
      (if (unwrap-panic (contract-call? reserve burn ft (get owner vault) (get collateral vault)))
        (if (is-eq debt (get debt vault))
          (begin
            (let ((entries (get ids (get-vault-entries (get owner vault)))))
              (let ((result (contract-call? .dao subtract-debt-from-collateral-type (get collateral-type vault) (get debt vault))))
                (map-set vaults
                  { id: vault-id }
                  {
                    id: vault-id,
                    owner: (get owner vault),
                    collateral: u0,
                    collateral-type: (get collateral-type vault),
                    collateral-token: (get collateral-token vault),
                    debt: u0,
                    created-at-block-height: (get created-at-block-height vault),
                    updated-at-block-height: block-height,
                    stability-fee: (get stability-fee vault),
                    stability-fee-last-accrued: (get stability-fee-last-accrued vault),
                    is-liquidated: false,
                    auction-ended: false,
                    leftover-collateral: u0
                  }
                )
                ;; TODO: remove vault ID from vault entries
                ;; (map-set vault-entries { user: tx-sender } { () })

                (ok (map-delete vaults { id: vault-id }))
              )
            )
          )
          (begin
            (map-set vaults
              { id: vault-id }
              {
                id: vault-id,
                owner: (get owner vault),
                collateral: (get collateral vault),
                collateral-type: (get collateral-type vault),
                collateral-token: (get collateral-token vault),
                debt: (- (get debt vault) debt),
                created-at-block-height: (get created-at-block-height vault),
                updated-at-block-height: block-height,
                stability-fee: (get stability-fee vault),
                stability-fee-last-accrued: (get stability-fee-last-accrued vault),
                is-liquidated: false,
                auction-ended: false,
                leftover-collateral: u0
              }
            )
            (ok true)
          )
        )
        (err err-burn-failed)
      )
      (err err-burn-failed)
    )
  )
)

;; Calculate stability fee based on time
;; 144 blocks = 1 day
;; to be fair, this is a very rough approximation
;; the goal is not to get the exact interest,
;; but rather to (dis)incentivize the user to mint stablecoins or not
(define-read-only (get-stability-fee-for-vault (vault-id uint))
  (let ((vault (get-vault-by-id vault-id)))
    (let ((days (/ (- block-height (get stability-fee-last-accrued vault)) blocks-per-day)))
      (let ((debt (/ (get debt vault) u100000))) ;; we can round to 1 number after comma, e.g. 1925000 uxUSD == 1.9 xUSD
        (let ((daily-interest (/ (* debt (unwrap-panic (contract-call? .dao get-stability-fee (get collateral-type vault)))) u100)))
          (ok (tuple (fee (* daily-interest days)) (decimals u12) (days days))) ;; 12 decimals so u5233 means 5233/10^12 xUSD daily interest
        )
      )
    )
  )
)

;; should be called ~weekly per open (i.e. non-liquidated) vault
(define-public (accrue-stability-fee (vault-id uint))
  (let ((fee (unwrap-panic (get-stability-fee-for-vault vault-id))))
    (if (> (get days fee) u7)
      (begin
        (let ((vault (get-vault-by-id vault-id)))
          (map-set vaults
            { id: vault-id }
            {
              id: vault-id,
              owner: (get owner vault),
              collateral: (get collateral vault),
              collateral-type: (get collateral-type vault),
              collateral-token: (get collateral-token vault),
              debt: (get debt vault),
              created-at-block-height: (get created-at-block-height vault),
              updated-at-block-height: block-height,
              stability-fee: (+ (/ (get fee fee) (get decimals fee)) (get stability-fee vault)),
              stability-fee-last-accrued: (+ (get stability-fee-last-accrued vault) (* (get days fee) blocks-per-day)),
              is-liquidated: false,
              auction-ended: false,
              leftover-collateral: (get leftover-collateral vault)
            }
          )
          (ok true)
        )
      )
      (ok true) ;; nothing to accrue
    )
  )
)

(define-public (pay-stability-fee (vault-id uint))
  (let ((vault (get-vault-by-id vault-id)))
    (if (is-ok (contract-call? .xusd-token transfer (get stability-fee vault) tx-sender (as-contract tx-sender)))
      (begin
        (map-set vaults
          { id: vault-id }
          {
            id: vault-id,
            owner: (get owner vault),
            collateral: (get collateral vault),
            collateral-type: (get collateral-type vault),
            collateral-token: (get collateral-token vault),
            debt: (get debt vault),
            created-at-block-height: (get created-at-block-height vault),
            updated-at-block-height: block-height,
            stability-fee: u0,
            stability-fee-last-accrued: (get stability-fee-last-accrued vault),
            is-liquidated: false,
            auction-ended: false,
            leftover-collateral: (get leftover-collateral vault)
          }
        )
        (ok true)
      )
      (err u5)
    )
  )
)

(define-public (liquidate (vault-id uint))
  (if (is-eq contract-caller .liquidator)
    (begin
      (let ((vault (get-vault-by-id vault-id)))
        (begin
          (let ((collateral (get collateral vault)))
            (map-set vaults
              { id: vault-id }
              {
                id: vault-id,
                owner: (get owner vault),
                collateral: u0,
                collateral-type: (get collateral-type vault),
                collateral-token: (get collateral-token vault),
                debt: (get debt vault),
                created-at-block-height: (get created-at-block-height vault),
                updated-at-block-height: block-height,
                stability-fee: (get stability-fee vault),
                stability-fee-last-accrued: (get stability-fee-last-accrued vault),
                is-liquidated: true,
                auction-ended: false,
                leftover-collateral: u0
              }
            )
            (let ((debt (/ (* (unwrap-panic (contract-call? .dao get-liquidation-penalty (get collateral-type vault))) (get debt vault)) u100)))
              (ok (tuple (ustx-amount collateral) (debt (+ debt (get debt vault)))))
            )
          )
        )
      )
    )
    (err err-unauthorized)
  )
)

(define-public (finalize-liquidation (vault-id uint) (leftover-collateral uint) (debt-raised uint))
  (if (is-eq contract-caller .auction-engine)
    (let ((vault (get-vault-by-id vault-id)))
      (if (is-ok (contract-call? .xusd-token burn debt-raised mint-owner))
        (begin
          (map-set vaults
            { id: vault-id }
            {
              id: vault-id,
              owner: (get owner vault),
              collateral: u0,
              collateral-type: (get collateral-type vault),
              collateral-token: (get collateral-token vault),
              debt: (get debt vault),
              created-at-block-height: (get created-at-block-height vault),
              updated-at-block-height: block-height,
              stability-fee: (get stability-fee vault),
              stability-fee-last-accrued: (get stability-fee-last-accrued vault),
              is-liquidated: true,
              auction-ended: true,
              leftover-collateral: leftover-collateral
            }
          )
          (let ((result (contract-call? .dao subtract-debt-from-collateral-type (get collateral-type vault) (get debt vault))))
            (ok true)
          )
        )
        (err err-liquidation-failed)
      )
    )
    (err err-unauthorized)
  )
)

(define-public (withdraw-leftover-collateral (vault-id uint) (reserve <vault-trait>) (ft <mock-ft-trait>))
  (let ((vault (get-vault-by-id vault-id)))
    (asserts! (is-eq tx-sender (get owner vault)) (err err-unauthorized))

    (if (unwrap-panic (contract-call? reserve withdraw ft (get owner vault) (get leftover-collateral vault)))
      (begin
        (map-set vaults
          { id: vault-id }
          {
            id: vault-id,
            owner: tx-sender,
            collateral: (get collateral vault),
            collateral-type: (get collateral-type vault),
            collateral-token: (get collateral-token vault),
            debt: (get debt vault),
            created-at-block-height: (get created-at-block-height vault),
            updated-at-block-height: block-height,
            stability-fee: (get stability-fee vault),
            stability-fee-last-accrued: (get stability-fee-last-accrued vault),
            is-liquidated: true,
            auction-ended: true,
            leftover-collateral: u0
          }
        )
        (ok true)
      )
      (err err-withdraw-failed)
    )
  )
)
