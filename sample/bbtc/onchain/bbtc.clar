;; bBTC
;; bBTC is a low level contract enabling BTC on the Stacks Blockchain.
;; Depending on how frequently this contract end up being used, it can
;; also be used as an onchain STX/BTC price feed 

;; Constants
;;

;; Data maps and vars
(define-data-var box-ids uint u0)
(define-data-var sealing-commitees-ids uint u0)
(define-non-fungible-token boxed-btc { box-id: uint })
(define-map sealers principal { collateral: { max: uint, current: uint }})
(define-map sealing-commitees { sealing-committee-id: uint } { sealers: (list 10 principal), status: uint })
(define-map sealing-commitee-calls { sealing-committee-id: uint } (list 10 (option bool)))
(define-map btc-boxes { box-id: uint } { state: uint, txid: (option (buff 32)) })
(define-map collected-fees { box-id: uint } uint)

;; Private functions
(define-private (new-box-id)
    (+ (var-get box-ids) u1))

(define-private (new-sealing-commitees-id)
    (+ (var-get sealing-commitees-ids) u1))

(define-private (get-some-randomness (derivation uint))
    (let ((vrf (unwrap-panic! (get-block-info? vrf block-height))))
        derivation))

;; Public functions
;; Called by the boxer
(define-public (create-box (size uint) (fee uint))
    (let 
        ((box-id (new-box-id))
        ;; Get some randomness
        (randomness (get-some-randomness box-id))
        ;; Create a valid sealing committee, with enough collateral
        (sealing-committee (try! (new-sealing-committee box-size box-id tx-id randomness))))
        ;; Take a fee: will be used for rewarding sealers once the signature is submitted
        (asserts! (stx-transfer? fee tx-sender (as-contract tx-sender)) 
            (err ERR_INSUFFICIENT_FUNDS))
        ;; Emit an event to wake up the elected sealers
        (print { type: "create-box", box-id: box-id, sealing-committee: sealing-committee })
        ;; Register new box
        (map-set btc-boxes { box-id: box-id } { state: u0 })
        ;; Register new committee
        (map-set btc-boxes { box-id: box-id } { state: u0 })
        (ok true)))

;; Called by the sealers
(define-public (secure-box (box-id uint)))

;; Called by the boxer
(define-public (box-btc (box-id uint) (txid (buff 32)))
    (let ((box-props (unwrap-panic (map-get? box-id ))))
        (map-set btc-boxes { box-id: box-id } (merge box-props { txid: (some txid) }))))

;; Called by anyone
(define-public (watch-box))

;; Called by the owner
(define-public (transfer (btc-box-id uint) (recipient principal))
    (try! (nft-transfer? boxed-btc { box-id: box-id } tx-sender principal))

;; Called by the unboxer
(define-public (unbox-btc))

;; Called by the sealers
(define-read-only (get-box-status (box-id uint))
    (ok true))

(define-public (register-sealer (collateral uint))
    (let (sealer-id (new-sealer-id)))
        ;; Ensure uniqueness
        (map-set sealers sealer-id { collateral: collateral }))
