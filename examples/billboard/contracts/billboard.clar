;; billboard contract

;; error consts
(define-constant ERR_STX_TRANSFER   u0)
(define-constant ERR_SET_MESSAGE    u1)
(define-constant ERR_SET_PRICE      u2)

;; data vars
(define-data-var billboard-message (string-utf8 500) u"Hello World!")
(define-data-var price uint u100)

;; public functions
(define-read-only (get-price)
    (var-get price)
)

(define-read-only (get-message)
    (var-get billboard-message)
)

(define-public (set-message (message (string-utf8 500)))
    (let ((cur-price (var-get price))
          (new-price (+ u10 cur-price)))

        ;; pay the contract
        (unwrap! (stx-transfer? cur-price tx-sender (as-contract tx-sender)) (err ERR_STX_TRANSFER))

	;; update the billboard's message
        (asserts! (var-set billboard-message message) (err ERR_SET_MESSAGE))

        ;; update the price of setting a message
        (asserts! (var-set price new-price) (err ERR_SET_PRICE))

        ;; return the updated price
	(ok new-price)
    )
)
