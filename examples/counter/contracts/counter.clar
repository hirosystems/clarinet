
;; counter
;; let's get started with smart contracts
(define-data-var counter uint u1)

(define-public (increment (step uint))
    (let ((new-val (+ step (var-get counter)))) 
        (var-set counter new-val)
        (ok new-val)))
