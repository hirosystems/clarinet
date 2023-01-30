
;; counter
;; let's get started with smart contracts
(define-data-var counter uint u1)

(define-public (increment (step uint))
    (let ((new-val (+ step (var-get counter)))) 
        (var-set counter new-val)
        (print { object: "counter", action: "incremented", value: new-val, chain: (slice? "blockstack" u5 u10) })
        (ok new-val)))

(define-public (decrement (step uint))
    (let ((new-val (- step (var-get counter)))) 
        (var-set counter new-val)
        (print { object: "counter", action: "decremented", value: new-val, chain: (slice? "blockstack" u5 u10) })
        (ok new-val)))

(define-read-only (read-counter)
    (ok (var-get counter)))
