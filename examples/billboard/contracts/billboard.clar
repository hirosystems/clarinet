;; billboard v3

;; error consts
(define-constant ERR_INVALID_STRING u0)
(define-constant ERR_STX_TRANSFER   u1)
(define-constant ERR_SET_MESSAGE    u2)

;; data maps/vars
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

        (unwrap! (stx-transfer? cur-price tx-sender (as-contract tx-sender)) (err ERR_STX_TRANSFER))
        (asserts! (var-set price new-price) (err ERR_SET_MESSAGE))
        (asserts! (var-set billboard-message message) (err ERR_SET_MESSAGE))

	(ok new-price)
    )
)
