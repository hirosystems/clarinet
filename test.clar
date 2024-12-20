;; comment
(slice? "blockstack" u5 u10) ;; Returns (some "stack")
(slice? (list 1 2 3 4 5) u5 u9) ;; Returns none
(slice? (list 1 2 3 4 5) u3 u4) ;; Returns (some (4))
(slice? "abcd" u1 u3) ;; Returns (some "bc")
(slice? "abcd" u2 u2) ;; Returns (some "")
(slice? "abcd" u3 u1) ;; Returns none
;; whatever
;;asdf asdf
