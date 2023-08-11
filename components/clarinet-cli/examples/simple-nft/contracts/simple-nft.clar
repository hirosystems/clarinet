(define-constant CONTRACT_OWNER tx-sender)
(define-constant CONTRACT_ADDRESS (as-contract tx-sender))
(define-constant DEPLOYED_AT block-height)

(define-constant ERR_NOT_AUTHORIZED (err u1001))

(impl-trait 'SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.nft-trait.nft-trait)

(define-data-var lastId uint u0)
(define-map CFG_BASE_URI bool (string-ascii 256))

(define-non-fungible-token nft uint)

(define-read-only (get-last-token-id)
  (ok (var-get lastId))
)

(define-read-only (get-owner (id uint))
  (ok (nft-get-owner? nft id))
)

(define-read-only (get-token-uri (id uint))
  (ok (map-get? CFG_BASE_URI true))
)

(define-public (transfer (id uint) (sender principal) (recipient principal))
  (begin
    (asserts! (is-eq tx-sender sender) ERR_NOT_AUTHORIZED)
    (nft-transfer? nft id sender recipient)
  )
)

;; extra functions
(define-public (set-base-uri (uri (optional (string-ascii 256))))
  (begin
    (asserts! (is-eq contract-caller CONTRACT_OWNER) ERR_NOT_AUTHORIZED)
    (ok (match uri uriVal
      (map-set CFG_BASE_URI true uriVal)
      (map-delete CFG_BASE_URI true)
    ))
  )
)

;; test functions
(define-public (test-mint (recipient principal))
  (let
    ((newId (+ (var-get lastId) u1)))
    (asserts! (is-eq DEPLOYED_AT u1) ERR_NOT_AUTHORIZED)
    (var-set lastId newId)
    (nft-mint? nft newId recipient)
  )
)

(define-public (test-burn (id uint) (sender principal))
  (begin
    (asserts! (is-eq tx-sender sender) ERR_NOT_AUTHORIZED)
    (nft-burn? nft id sender)
  )
)
