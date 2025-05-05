(use-trait sip-010 'SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.sip-010-trait-ft-standard.sip-010-trait)
(define-trait stackflow-token (
  (fund-pipe
    (
      (optional <sip-010>) ;; token
      uint                 ;; amount
      principal            ;; with
      uint                 ;; nonce
    )
    (response {
      token: (optional principal),
      principal-1: principal,
      principal-2: principal,
    } uint)
  )
  (close-pipe
    (
      (optional <sip-010>) ;; token
      principal            ;; with
      uint                 ;; my-balance
      uint                 ;; their-balance
      (buff 65)            ;; my-signature
      (buff 65)            ;; their-signature
      uint                 ;; nonce
    )
    (response bool uint)
  )
  (force-cancel
    (
      (optional <sip-010>) ;; token
      principal            ;; with
    )
    (response uint uint)
  )
  (force-close
    (
      (optional <sip-010>) ;; token
      principal            ;; with
      uint                 ;; my-balance
      uint                 ;; their-balance
      (buff 65)            ;; my-signature
      (buff 65)            ;; their-signature
      uint                 ;; nonce
      uint                 ;; action
      principal            ;; actor
      (optional (buff 32)) ;; secret
      (optional uint)      ;; valid-after
    )
    (response uint uint)
  )
  (dispute-closure
    (
      (optional <sip-010>) ;; token
      principal            ;; with
      uint                 ;; my-balance
      uint                 ;; their-balance
      (buff 65)            ;; my-signature
      (buff 65)            ;; their-signature
      uint                 ;; nonce
      uint                 ;; action
      principal            ;; actor
      (optional (buff 32)) ;; secret
      (optional uint)      ;; valid-after
    )
    (response bool uint)
  )
  (finalize
    (
      (optional <sip-010>) ;; token
      principal            ;; with
    )
    (response bool uint)
  )
  (deposit
    (
      uint                 ;; amount
      (optional <sip-010>) ;; token
      principal            ;; with
      uint                 ;; my-balance
      uint                 ;; their-balance
      (buff 65)            ;; my-signature
      (buff 65)            ;; their-signature
      uint                 ;; nonce
    )
    (response {
      token: (optional principal),
      principal-1: principal,
      principal-2: principal,
    } uint)
  )
  (withdraw
    (
      uint                 ;; amount
      (optional <sip-010>) ;; token
      principal            ;; with
      uint                 ;; my-balance
      uint                 ;; their-balance
      (buff 65)            ;; my-signature
      (buff 65)            ;; their-signature
      uint                 ;; nonce
    )
    (response {
      token: (optional principal),
      principal-1: principal,
      principal-2: principal,
    } uint)
  )
))
