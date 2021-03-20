;; addresses
(define-constant auction-reserve 'S02J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKPVKG2CE)

;; errors
(define-constant err-bid-declined u1)
(define-constant err-lot-sold u2)
(define-constant err-poor-bid u3)
(define-constant err-xusd-transfer-failed u4)

(define-map auctions
  { id: uint }
  {
    id: uint,
    collateral-amount: uint,
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
    owner: principal,
    is-accepted: bool
  }
)
(define-data-var last-auction-id uint u0)
(define-data-var auction-ids (list 2000 uint) (list u0))
(define-data-var lot-size uint u100000000) ;; 100 STX

(define-read-only (get-auction-by-id (id uint))
  (unwrap!
    (map-get? auctions { id: id })
    (tuple
      (id u0)
      (collateral-amount u0)
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

(define-read-only (get-auction-id)
  (ok (var-get auction-ids))
)

(define-read-only (get-auctions)
  (ok (map get-auction-by-id (var-get auction-ids)))
)

;; stx-collateral has been posted in stx-liquidation-reserve principal
;; 1. Create auction object in map per 100 STX
;; 2. Add auction ID to list (to show in UI)
;; we wanna sell as little collateral as possible to cover the vault's debt
;; if we cannot cover the vault's debt with the collateral sale,
;; we will have to sell some governance or STX tokens from the reserve
(define-public (start-auction (vault-id uint) (ustx-amount uint) (debt-to-raise uint))
  (let ((auction-id (+ (var-get last-auction-id) u1)))
    ;; 500 collateral => 500 / 100 = 5 lots
    (let ((amount-of-lots (+ u1 (/ ustx-amount (var-get lot-size)))))
      (let ((last-lot (mod ustx-amount (var-get lot-size))))
        (map-set auctions
          { id: auction-id }
          {
            id: auction-id,
            collateral-amount: ustx-amount,
            debt-to-raise: debt-to-raise,
            vault-id: vault-id,
            lot-size: (var-get lot-size),
            lots: amount-of-lots,
            last-lot-size: last-lot,
            lots-sold: u0,
            ends-at: (+ block-height u10000),
            total-collateral-auctioned: u0,
            total-debt-raised: u0,
            is-open: true
          }
        )
        (print "Added new open auction")
        (var-set auction-ids (unwrap-panic (as-max-len? (append (var-get auction-ids) auction-id) u2000)))
        (var-set last-auction-id auction-id)
        (ok true)
      )
    )
  )
)

;; calculates the minimum collateral amount to sell
;; e.g. if we need to cover 10 xUSD debt, and we have 20 STX at $1/STX,
;; we only need to auction off 10 STX
(define-read-only (calculate-minimum-collateral-amount (auction-id uint))
  (let ((stx-price-in-cents (contract-call? .oracle get-price)))
    (let ((auction (get-auction-by-id auction-id)))
      (let ((amount (/ (/ (get debt-to-raise auction) (get price stx-price-in-cents)) (get lots auction))))
        (if (> (get collateral-amount auction) amount)
          (ok amount)
          (ok (get collateral-amount auction))
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
      (owner 'ST31HHVBKYCYQQJ5AQ25ZHA6W2A548ZADDQ6S16GP)
      (is-accepted false)
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

(define-private (accept-bid (auction-id uint) (lot-index uint) (xusd uint) (collateral-amount uint))
  (let ((auction (get-auction-by-id auction-id)))
    (let ((last-bid (get-last-bid auction-id lot-index)))
      (let ((accepted-bid (>= xusd (/ (get debt-to-raise auction) (get lots auction)))))
        ;; if this bid is at least (total debt to raise / lot-size) amount, accept it as final - we don't need to be greedy
        (begin
          ;; (return-collateral (get owner last-bid) (get xusd last-bid))
          (if (unwrap-panic (contract-call? .xusd-token transfer auction-reserve xusd))
            (begin
              (map-set auctions
                { id: auction-id }
                {
                  id: auction-id,
                  collateral-amount: (get collateral-amount auction),
                  debt-to-raise: (get debt-to-raise auction),
                  vault-id: (get vault-id auction),
                  lot-size: (get lot-size auction),
                  lots: (get lots auction),
                  last-lot-size: (get last-lot-size auction),
                  lots-sold: (+ u1 (get lots-sold auction)),
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
                  owner: tx-sender,
                  is-accepted: accepted-bid
                }
              )
              (if
                (or
                  (>= block-height (get ends-at auction))
                  (>= (+ u1 (get lots-sold auction)) (get lots auction))
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

(define-private (return-collateral (owner principal) (xusd uint))
  (if (> u0 xusd)
    (ok (unwrap-panic (as-contract (contract-call? .xusd-token transfer owner xusd))))
    (err false)
  )
)

;; DONE 1. flag auction on map as closed
;; N/A  2a. go over each lot (0 to lot-size) and send collateral to winning address
;; TODO 2b. OR allow person to collect collateral from reserve manually
;; TODO 3. check if vault debt is covered (sum of xUSD in lots >= debt-to-raise)
;; DONE 4. update vault to allow vault owner to withdraw leftover collateral (if any)
;; 5. if not all vault debt is covered: auction off collateral again (if any left)
;; 6. if not all vault debt is covered and no collateral is left: cover xUSD with gov token
;; TODO: maybe keep an extra map with bids and (bidder, auction id, lot id) tuple as key with all their bids
(define-private (close-auction (auction-id uint))
  (let ((auction (get-auction-by-id auction-id)))
    (map-set auctions
      { id: auction-id }
      {
        id: auction-id,
        collateral-amount: (get collateral-amount auction),
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
    (ok
      (unwrap-panic
        (contract-call?
          .freddie
          finalize-liquidation
          (get vault-id auction)
          (- (get collateral-amount auction) (get total-collateral-auctioned auction))
        )
      )
    )
  )
)
