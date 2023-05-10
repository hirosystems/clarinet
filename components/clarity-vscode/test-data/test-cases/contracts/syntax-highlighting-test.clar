(define-private (tst
  (test-block-height uint)
  (test-false uint)
  (false-test uint)
  (test-none uint)
  (none-false uint)
  (test-none uint)
  (block-height-test uint)
  (test-tx-sender uint)
  (test-3 uint)
  (test-u3 uint)
  (test-0x4567 uint)
)
  (begin
    (print test-block-height)
    (print test-false)
    (print false-test)
    (print test-none)
    (print none-false)
    (print test-none)
    (print block-height-test)
    (print test-tx-sender)
    (print test-u3)
    (print test-3)
    (print test-0x4567)

    (print true)
    (print false)
    (print none)
    (print u3)
    (print 3)
    (print 0x4567)
  )
)


(define-read-only (test-true) (ok true))
(test-true)

(define-read-only (true-test) (ok true))
(true-test)

(define-read-only (err-test) (ok (err u1)))
(err-test)

(define-read-only (test-err) (ok (err u1)))
(test-err)


