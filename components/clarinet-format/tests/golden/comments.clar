;; max_line_length: 80, indentation: 2
;; comment
(define-read-only (get-offer (id uint) (w uint)) (map-get? offers-map id)
)
(define-read-only (get-offer) (ok 1))
;; top comment
;; @ignore-formatting
(define-constant something (+ 1 1)) ;; eol comment

(define-read-only (something-else)
  (begin (+ 1 1)   (ok true)
  ))

(define-public (something-else (a uint))
  (begin
    (+ 1 1) (ok true)))
