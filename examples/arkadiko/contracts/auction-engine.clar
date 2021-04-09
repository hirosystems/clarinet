(use-trait vault-trait .vault-trait.vault-trait)
(use-trait mock-ft-trait .mock-ft-trait.mock-ft-trait)

;; addresses
(define-constant auction-reserve 'ST238B5WSC8B8XETWDXMH7HZC2MJ2RNTYY15YY7SH)

;; errors
(define-constant err-bid-declined u1)
(define-constant err-lot-sold u2)
(define-constant err-poor-bid u3)
(define-constant err-xusd-transfer-failed u4)
(define-constant err-auction-not-allowed u5)
(define-constant err-insufficient-collateral u6)
(define-constant err-not-authorized u7)

(define-map auctions
  { id: uint }
  {
    id: uint,
    collateral-amount: uint,
    collateral-token: (string-ascii 12),
    debt-to-raise: uint,
    vault-id: uint,
    lot-size: uint,
    lots: uint,
    last-lot-size: uint,
    lots-sold: uint,
    is-open: bool,
    total-collateral-auctioned: uint,
    total-debt-raised: uint,
    ends-at: uint
  }
)
(define-map bids
  { auction-id: uint, lot-index: uint }
  {
    xusd: uint,
    collateral-amount: uint,
    collateral-token: (string-ascii 12),
    owner: principal,
    is-accepted: bool
  }
)
(define-map winning-lots
  { user: principal }
  { ids: (list 100 (tuple (auction-id uint) (lot-index uint))) }
)
(define-map redeeming-lot
  { user: principal }
  { auction-id: uint, lot-index: uint }
)

(define-data-var last-auction-id uint u0)
(define-data-var auction-ids (list 1800 uint) (list u0))
(define-data-var lot-size uint u100000000) ;; 100 xUSD

(define-read-only (get-auction-by-id (id uint))
  (unwrap!
    (map-get? auctions { id: id })
    (tuple
      (id u0)
      (collateral-amount u0)
      (collateral-token "")
      (debt-to-raise u0)
      (vault-id u0)
      (lot-size u0)
      (lots u0)
      (last-lot-size u0)
      (lots-sold u0)
      (is-open false)
      (total-collateral-auctioned u0)
      (total-debt-raised u0)
      (ends-at u0)
    )
  )
)

(define-read-only (get-auctions)
  (ok (map get-auction-by-id (var-get auction-ids)))
)

;; 1. Create auction object in map per 100 xUSD
;; 2. Add auction ID to list (to show in UI)
;; we wanna sell as little collateral as possible to cover the vault's debt
;; if we cannot cover the vault's debt with the collateral sale,
;; we will have to sell some governance or STX tokens from the reserve
(define-public (start-auction (vault-id uint) (uamount uint) (debt-to-raise uint))
  (let ((vault (contract-call? .freddie get-vault-by-id vault-id)))
    (asserts! (is-eq contract-caller .liquidator) (err err-not-authorized))
    (asserts! (is-eq (get is-liquidated vault) true) (err err-auction-not-allowed))

    (let ((auction-id (+ (var-get last-auction-id) u1)))
      ;; 500 xUSD debt => 500 / 100 = 5 lots
      (let ((amount-of-lots (/ debt-to-raise (var-get lot-size))))
        (if (< (* amount-of-lots (var-get lot-size)) debt-to-raise)
          (begin
            ;; need to add +1 to amount of lots
            (let ((last-lot-size (mod debt-to-raise (var-get lot-size))))
              (map-set auctions
                { id: auction-id }
                {
                  id: auction-id,
                  collateral-amount: uamount,
                  collateral-token: (get collateral-token vault),
                  debt-to-raise: debt-to-raise,
                  vault-id: vault-id,
                  lot-size: (var-get lot-size),
                  lots: (+ u1 amount-of-lots),
                  last-lot-size: last-lot-size,
                  lots-sold: u0,
                  ends-at: (+ block-height u10000),
                  total-collateral-auctioned: u0,
                  total-debt-raised: u0,
                  is-open: true
                }
              )
              (print "Added new open auction")
              (var-set auction-ids (unwrap-panic (as-max-len? (append (var-get auction-ids) auction-id) u1800)))
              (var-set last-auction-id auction-id)
              (ok true)
            )
          )
          (begin
            ;; the collateral amount is exactly divisible by lot-size (no remainder after division)
            (map-set auctions
              { id: auction-id }
              {
                id: auction-id,
                collateral-amount: uamount,
                collateral-token: (get collateral-token vault),
                debt-to-raise: debt-to-raise,
                vault-id: vault-id,
                lot-size: (var-get lot-size),
                lots: amount-of-lots,
                last-lot-size: u0,
                lots-sold: u0,
                ends-at: (+ block-height u10000),
                total-collateral-auctioned: u0,
                total-debt-raised: u0,
                is-open: true
              }
            )
            (print "Added new open auction")
            (var-set auction-ids (unwrap-panic (as-max-len? (append (var-get auction-ids) auction-id) u1800)))
            (var-set last-auction-id auction-id)
            (ok true)
          )
        )
      )
    )
  )
)

;; start an auction to sell off DIKO gov tokens
;; this is a private function since it should only be called
;; when a normal collateral liquidation auction can't raise enough debt
(define-private (start-debt-auction (vault-id uint) (debt-to-raise uint))
  (let ((vault (contract-call? .freddie get-vault-by-id vault-id)))
    (asserts! (is-eq (get is-liquidated vault) true) (err err-auction-not-allowed))
    (let ((collateral-uamount u5))
      (let ((auction-id (+ (var-get last-auction-id) u1)))
        (map-set auctions
          { id: auction-id }
          {
            id: auction-id,
            collateral-amount: collateral-uamount,
            collateral-token: "diko",
            debt-to-raise: debt-to-raise,
            vault-id: vault-id,
            lot-size: (var-get lot-size),
            lots: (+ u1 (/ debt-to-raise (var-get lot-size))), ;; lot-size / price of diko
            last-lot-size: u0,
            lots-sold: u0,
            ends-at: (+ block-height u10000),
            total-collateral-auctioned: u0,
            total-debt-raised: u0,
            is-open: true
          }
        )
      )
    )
    (ok true)
  )
)

;; calculates the minimum collateral amount to sell
;; e.g. if we need to cover 10 xUSD debt, and we have 20 STX at $1/STX,
;; we only need to auction off 10 STX
(define-read-only (calculate-minimum-collateral-amount (auction-id uint))
  (let ((auction (get-auction-by-id auction-id)))
    (let ((price-in-cents (contract-call? .oracle get-price (get collateral-token auction))))
      (let ((amount (/ (/ (get debt-to-raise auction) (get last-price-in-cents price-in-cents)) (get lots auction))))
        (if (> (/ (get collateral-amount auction) (get lots auction)) (* u100 amount))
          (ok (* u100 amount))
          (ok (/ (get collateral-amount auction) (get lots auction)))
        )
      )
    )
  )
)

(define-read-only (get-last-bid (auction-id uint) (lot-index uint))
  (unwrap!
    (map-get? bids { auction-id: auction-id, lot-index: lot-index })
    (tuple
      (xusd u0)
      (collateral-amount u0)
      (collateral-token "")
      (owner 'ST238B5WSC8B8XETWDXMH7HZC2MJ2RNTYY15YY7SH)
      (is-accepted false)
    )
  )
)

(define-read-only (get-winning-lots (owner principal))
  (unwrap!
    (map-get? winning-lots { user: owner })
    (tuple
      (ids (list (tuple (auction-id u0) (lot-index u0))))
    )
  )
)

(define-public (bid (auction-id uint) (lot-index uint) (xusd uint) (collateral-amount uint))
  (let ((auction (get-auction-by-id auction-id)))
    (if
      (and
        (< lot-index (get lots auction))
        (is-eq (get is-open auction) true)
        (<= collateral-amount (/ (get collateral-amount auction) (get lots auction)))
      )
      (ok (register-bid auction-id lot-index xusd collateral-amount))
      (err err-bid-declined) ;; just silently exit
    )
  )
)

(define-private (register-bid (auction-id uint) (lot-index uint) (xusd uint) (collateral-amount uint))
  (let ((auction (get-auction-by-id auction-id)))
    (let ((last-bid (get-last-bid auction-id lot-index)))
      (if (not (get is-accepted last-bid))
        (if (> xusd (get xusd last-bid)) ;; we have a better bid and the previous one was not accepted!
          (ok (accept-bid auction-id lot-index xusd collateral-amount))
          (err err-poor-bid) ;; don't care cause either the bid is already over or it was a poor bid
        )
        (err err-lot-sold) ;; lot is already sold
      )
    )
  )
)

(define-private (is-lot-sold (accepted-bid bool))
  (if accepted-bid
    (ok u1)
    (ok u0)
  )
)

(define-private (accept-bid (auction-id uint) (lot-index uint) (xusd uint) (collateral-amount uint))
  (let ((auction (get-auction-by-id auction-id)))
    (let ((last-bid (get-last-bid auction-id lot-index)))
      (let ((accepted-bid (>= xusd (/ (get debt-to-raise auction) (get lots auction)))))
        ;; if this bid is at least (total debt to raise / lot-size) amount, accept it as final - we don't need to be greedy
        (begin
          ;; (return-collateral (get owner last-bid) (get xusd last-bid)) ;; return xUSD of last bid to (now lost) bidder
          (if (unwrap-panic (contract-call? .xusd-token transfer xusd tx-sender auction-reserve))
            (begin
              (map-set auctions
                { id: auction-id }
                {
                  id: auction-id,
                  collateral-amount: (get collateral-amount auction),
                  collateral-token: (get collateral-token auction),
                  debt-to-raise: (get debt-to-raise auction),
                  vault-id: (get vault-id auction),
                  lot-size: (get lot-size auction),
                  lots: (get lots auction),
                  last-lot-size: (get last-lot-size auction),
                  lots-sold: (+ (unwrap-panic (is-lot-sold accepted-bid)) (get lots-sold auction)),
                  ends-at: (get ends-at auction),
                  total-collateral-auctioned: (- (+ collateral-amount (get total-collateral-auctioned auction)) (get collateral-amount last-bid)),
                  total-debt-raised: (- (+ xusd (get total-debt-raised auction)) (get xusd last-bid)),
                  is-open: true
                }
              )
              (map-set bids
                { auction-id: auction-id, lot-index: lot-index }
                {
                  xusd: xusd,
                  collateral-amount: collateral-amount,
                  collateral-token: (get collateral-token auction),
                  owner: tx-sender,
                  is-accepted: accepted-bid
                }
              )
              (if accepted-bid
                (begin
                  (let ((lots (get-winning-lots tx-sender)))
                    (map-set winning-lots
                      { user: tx-sender }
                      {
                        ids: (unwrap-panic (as-max-len? (append (get ids lots) (tuple (auction-id auction-id) (lot-index lot-index))) u100))
                      }
                    )
                  )
                )
                true
              )
              (if
                (or
                  (>= block-height (get ends-at auction))
                  (>= (+ (unwrap-panic (is-lot-sold accepted-bid)) (get lots-sold auction)) (get lots auction))
                )
                ;; auction is over - close all bids
                ;; send collateral to winning bidders
                (ok (unwrap-panic (close-auction auction-id)))
                (err u0)
              )
            )
            (err err-xusd-transfer-failed)
          )
        )
      )
    )
  )
)

(define-private (remove-winning-lot (lot (tuple (auction-id uint) (lot-index uint))))
  (let ((current-lot (unwrap-panic (map-get? redeeming-lot { user: tx-sender }))))
    (if 
      (and
        (is-eq (get auction-id lot) (get auction-id current-lot))
        (is-eq (get lot-index lot) (get lot-index current-lot))
      )
      false
      true
    )
  )
)

(define-public (redeem-lot-collateral (ft <mock-ft-trait>) (reserve <vault-trait>) (auction-id uint) (lot-index uint))
  (let ((last-bid (get-last-bid auction-id lot-index)))
    (if
      (and
        (is-eq tx-sender (get owner last-bid))
        (get is-accepted last-bid)
      )
      (begin
        (let ((lots (get-winning-lots tx-sender)))
          (map-set redeeming-lot { user: tx-sender } { auction-id: auction-id, lot-index: lot-index})
          (if (map-set winning-lots { user: tx-sender } { ids: (filter remove-winning-lot (get ids lots)) })
            (ok (contract-call? reserve redeem-collateral ft (get collateral-amount last-bid) tx-sender))
            (err false)
          )
        )
      )
      (err false)
    )
  )
)

(define-private (return-collateral (owner principal) (xusd uint))
  (if (> u0 xusd)
    (ok (unwrap-panic (as-contract (contract-call? .xusd-token transfer xusd tx-sender owner))))
    (err false)
  )
)

;; DONE     1. flag auction on map as closed
;; SCRIPT   2a. go over each lot (0 to lot-size) and send collateral to winning address
;; DONE     2b. OR allow person to collect collateral from reserve manually
;; TODO     3. check if vault debt is covered (sum of xUSD in lots >= debt-to-raise)
;; DONE     4. update vault to allow vault owner to withdraw leftover collateral (if any)
;; DONE     5. if not all vault debt is covered: auction off collateral again (if any left)
;; TODO     6. if not all vault debt is covered and no collateral is left: cover xUSD with gov token
(define-public (close-auction (auction-id uint))
  (let ((auction (get-auction-by-id auction-id)))
    (asserts!
      (or
        (>= block-height (get ends-at auction))
        (is-eq (get lots-sold auction) (get lots auction))
      )
      (err err-not-authorized)
    )
    (asserts! (is-eq (get is-open auction) true) (err err-not-authorized))

    (map-set auctions
      { id: auction-id }
      {
        id: auction-id,
        collateral-amount: (get collateral-amount auction),
        collateral-token: (get collateral-token auction),
        debt-to-raise: (get debt-to-raise auction),
        vault-id: (get vault-id auction),
        lot-size: (get lot-size auction),
        lots: (get lots auction),
        last-lot-size: (get last-lot-size auction),
        lots-sold: (get lots-sold auction),
        ends-at: (get ends-at auction),
        total-collateral-auctioned: (get total-collateral-auctioned auction),
        total-debt-raised: (get total-debt-raised auction),
        is-open: false
      }
    )
    (if (>= (get total-debt-raised auction) (get debt-to-raise auction))
      (contract-call?
        .freddie
        finalize-liquidation
        (get vault-id auction)
        (- (get collateral-amount auction) (get total-collateral-auctioned auction))
        (get total-debt-raised auction)
      )
      (begin
        (if (or
          (<= (get lots-sold auction) (get lots auction)) ;; not all lots are sold
          (<= (get total-collateral-auctioned auction) (get collateral-amount auction)) ;; we have some collateral left to auction
        ) ;; if any collateral left to auction
          (ok (unwrap-panic (start-auction
            (get vault-id auction)
            (- (get collateral-amount auction) (get total-collateral-auctioned auction))
            (- (get debt-to-raise auction) (get total-debt-raised auction))
          )))
          (begin
            ;; no collateral left and/or all current lots are sold. Need to sell governance token to raise more xUSD
            (ok (unwrap-panic (start-debt-auction
              (get vault-id auction)
              (get debt-to-raise auction)
            )))
          )
        )
      )
    )
  )
)
