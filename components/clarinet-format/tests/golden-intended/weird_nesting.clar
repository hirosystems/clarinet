;; max_line_length: 80, indentation: 2
(define-trait export-trait
  (
    (export-state () (response {
      guardian-set-initialized: bool,
      active-guardian-set-id: uint,
      previous-guardian-set: {
        set-id: uint,
        expires-at: uint,
      },
      post-message-fee: uint,
    }
      uint
    ))
  )
)
