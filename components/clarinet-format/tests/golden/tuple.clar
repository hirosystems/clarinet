;; max_line_length: 80, indentation: 2
(define-public (set-user-reserve
  (user principal)
  (asset principal) ;; comment
  (state
    (tuple
    (principal-borrow-balance uint)
    (last-variable-borrow-cumulative-index uint)
    (origination-fee uint)
    (stable-borrow-rate uint)
    (last-updated-block uint) ;; comment
    (use-as-collateral bool)
  )
    ))
  (begin
    (asserts! (is-lending-pool contract-caller) ERR_UNAUTHORIZED)
    (contract-call? .pool-reserve-data set-user-reserve-data user asset state)
  )
)
