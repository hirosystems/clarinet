;; max_line_length: 80, indentation: 2
;; comment
;; comment

;; comment
(define-read-only (get-offer (id uint) (w uint)) (map-get? offers-map id)
)
(define-read-only (get-offer) (ok 1))
;; top comment
;; @format-ignore
(define-constant something (list
   1     2  3 ;; comment
   4 5 ))

(define-read-only (something-else)
  (begin (+ 1 1)   (ok true)
  ))

(define-public (something-else (a uint))
  (begin
    (+ 1 1) (ok true)))
