;;RUN: cargo run check | filecheck %s

(define-public (tainted (amount uint))
;; CHECK: checker:[[# @LINE + 6 ]]:20: warning: use of potentially unchecked data
;; CHECK-NEXT:     (stx-transfer? amount (as-contract tx-sender) tx-sender)
;; CHECK-NEXT:                    ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:25: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted (amount uint))
;; CHECK-NEXT:                          ^~~~~~
    (stx-transfer? amount (as-contract tx-sender) tx-sender)
)

(define-public (expr-tainted (amount uint))
;; CHECK: checker:[[# @LINE + 6 ]]:20: warning: use of potentially unchecked data
;; CHECK-NEXT:     (stx-transfer? (+ u10 amount) (as-contract tx-sender) tx-sender)
;; CHECK-NEXT:                    ^~~~~~~~~~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:30: note: source of untrusted input here
;; CHECK-NEXT: (define-public (expr-tainted (amount uint))
;; CHECK-NEXT:                               ^~~~~~
    (stx-transfer? (+ u10 amount) (as-contract tx-sender) tx-sender)
)

(define-public (let-tainted (amount uint))
    (let ((x amount))
;; CHECK: checker:[[# @LINE + 6 ]]:24: warning: use of potentially unchecked data
;; CHECK-NEXT:         (stx-transfer? x (as-contract tx-sender) tx-sender)
;; CHECK-NEXT:                        ^
;; CHECK-NEXT: checker:[[# @LINE - 5 ]]:29: note: source of untrusted input here
;; CHECK-NEXT: (define-public (let-tainted (amount uint))
;; CHECK-NEXT:                              ^~~~~~
        (stx-transfer? x (as-contract tx-sender) tx-sender)
    )
)

(define-public (filtered (amount uint))
    (begin
        (asserts! (< amount u100) (err u100))
;; CHECK-NOT: checker:[[# @LINE + 1 ]]:24: warning:
        (stx-transfer? amount (as-contract tx-sender) tx-sender)
    )
)

(define-public (filtered-expr (amount uint))
    (begin
        (asserts! (< (+ amount u10) u100) (err u100))
;; CHECK-NOT: checker:[[# @LINE + 1 ]]:24: warning:
        (stx-transfer? amount (as-contract tx-sender) tx-sender)
    )
)

(define-public (let-filtered (amount uint))
    (let ((x amount))
        (asserts! (< x u100) (err u100))
;; CHECK-NOT: checker:[[# @LINE + 1 ]]:24: warning:
        (stx-transfer? x (as-contract tx-sender) tx-sender)
    )
)

(define-public (let-filtered-parent (amount uint))
    (let ((x amount))
        (asserts! (< amount u100) (err u100))
;; CHECK-NOT: checker:[[# @LINE + 1 ]]:24: warning:
        (stx-transfer? x (as-contract tx-sender) tx-sender)
    )
)

(define-public (let-tainted-twice (amount1 uint) (amount2 uint))
    (let ((x (+ amount1 amount2)))
;; CHECK: checker:[[# @LINE + 9 ]]:24: warning: use of potentially unchecked data
;; CHECK-NEXT:         (stx-transfer? x (as-contract tx-sender) tx-sender)
;; CHECK-NEXT:                        ^
;; CHECK-NEXT: checker:[[# @LINE - 5 ]]:35: note: source of untrusted input here
;; CHECK-NEXT: (define-public (let-tainted-twice (amount1 uint) (amount2 uint))
;; CHECK-NEXT:                                    ^~~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 8 ]]:50: note: source of untrusted input here
;; CHECK-NEXT: (define-public (let-tainted-twice (amount1 uint) (amount2 uint))
;; CHECK-NEXT:                                                   ^~~~~~~
        (stx-transfer? x (as-contract tx-sender) tx-sender)
    )
)

(define-public (let-tainted-twice-filtered-once (amount1 uint) (amount2 uint))
    (let ((x (+ amount1 amount2)))
        (asserts! (< amount1 u100) (err u100))
;; CHECK: checker:[[# @LINE + 6 ]]:24: warning: use of potentially unchecked data
;; CHECK-NEXT:         (stx-transfer? x (as-contract tx-sender) tx-sender)
;; CHECK-NEXT:                        ^
;; CHECK-NEXT: checker:[[# @LINE - 6 ]]:64: note: source of untrusted input here
;; CHECK-NEXT: (define-public (let-tainted-twice-filtered-once (amount1 uint) (amount2 uint))
;; CHECK-NEXT:                                                                 ^~~~~~~
        (stx-transfer? x (as-contract tx-sender) tx-sender)
    )
)

(define-public (let-tainted-twice-filtered-twice (amount1 uint) (amount2 uint))
    (let ((x (+ amount1 amount2)))
        (asserts! (< amount1 u100) (err u100))
        (asserts! (< amount2 u100) (err u101))
;; CHECK-NOT: checker:[[# @LINE + 1 ]]:24: warning:
        (stx-transfer? x (as-contract tx-sender) tx-sender)
    )
)

(define-public (let-tainted-twice-filtered-together (amount1 uint) (amount2 uint))
    (let ((x (+ amount1 amount2)))
        (asserts! (< (+ amount1 amount2) u100) (err u100))
;; CHECK-NOT: checker:[[# @LINE + 1 ]]:24: warning:
        (stx-transfer? x (as-contract tx-sender) tx-sender)
    )
)

(define-public (if-filter (amount uint))
;; CHECK-NOT: checker:[[# @LINE + 1 ]]:40: warning:
    (stx-transfer? (if (< amount u100) amount u100) (as-contract tx-sender) tx-sender)
)

(define-public (if-not-filtered (amount uint))
;; CHECK: checker:[[# @LINE + 6 ]]:20: warning: use of potentially unchecked data
;; CHECK-NEXT:     (stx-transfer? (if (< u50 u100) amount u100) (as-contract tx-sender) tx-sender)
;; CHECK-NEXT:                    ^~~~~~~~~~~~~~~~~~~~~~~~~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:33: note: source of untrusted input here
;; CHECK-NEXT: (define-public (if-not-filtered (amount uint))
;; CHECK-NEXT:                                  ^~~~~~
    (stx-transfer? (if (< u50 u100) amount u100) (as-contract tx-sender) tx-sender)
)

(define-public (and-tainted (amount uint))
    (ok (and
;; CHECK: checker:[[# @LINE + 6 ]]:38: warning: use of potentially unchecked data
;; CHECK-NEXT:         (unwrap-panic (stx-transfer? amount (as-contract tx-sender) tx-sender))
;; CHECK-NEXT:                                      ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 5 ]]:29: note: source of untrusted input here
;; CHECK-NEXT: (define-public (and-tainted (amount uint))
;; CHECK-NEXT:                              ^~~~~~
        (unwrap-panic (stx-transfer? amount (as-contract tx-sender) tx-sender))
    ))
)

(define-public (and-filter (amount uint))
    (ok (and
        (< amount u100)
;; CHECK-NOT: checker:[[# @LINE + 1 ]]:38: warning:
        (unwrap-panic (stx-transfer? amount (as-contract tx-sender) tx-sender))
    ))
)

(define-public (and-filter-after (amount uint))
    (ok (and
;; CHECK: checker:[[# @LINE + 6 ]]:38: warning: use of potentially unchecked data
;; CHECK-NEXT:         (unwrap-panic (stx-transfer? amount (as-contract tx-sender) tx-sender))
;; CHECK-NEXT:                                      ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 5 ]]:34: note: source of untrusted input here
;; CHECK-NEXT: (define-public (and-filter-after (amount uint))
;; CHECK-NEXT:                                   ^~~~~~
        (unwrap-panic (stx-transfer? amount (as-contract tx-sender) tx-sender))
        (< amount u100)
    ))
)

(define-public (or-tainted (amount uint))
    (ok (or
;; CHECK: checker:[[# @LINE + 6 ]]:38: warning: use of potentially unchecked data
;; CHECK-NEXT:         (unwrap-panic (stx-transfer? amount (as-contract tx-sender) tx-sender))
;; CHECK-NEXT:                                      ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 5 ]]:28: note: source of untrusted input here
;; CHECK-NEXT: (define-public (or-tainted (amount uint))
;; CHECK-NEXT:                             ^~~~~~
        (unwrap-panic (stx-transfer? amount (as-contract tx-sender) tx-sender))
    ))
)

(define-public (or-filter (amount uint))
    (ok (or
        (>= amount u100)
;; CHECK-NOT: checker:[[# @LINE + 1 ]]:38: warning:
        (unwrap-panic (stx-transfer? amount (as-contract tx-sender) tx-sender))
    ))
)

(define-public (or-filter-after (amount uint))
    (ok (or
;; CHECK: checker:[[# @LINE + 6 ]]:38: warning: use of potentially unchecked data
;; CHECK-NEXT:         (unwrap-panic (stx-transfer? amount (as-contract tx-sender) tx-sender))
;; CHECK-NEXT:                                      ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 5 ]]:33: note: source of untrusted input here
;; CHECK-NEXT: (define-public (or-filter-after (amount uint))
;; CHECK-NEXT:                                  ^~~~~~
        (unwrap-panic (stx-transfer? amount (as-contract tx-sender) tx-sender))
        (>= amount u100)
    ))
)

(define-public (tainted-stx-burn (amount uint))
;; CHECK: checker:[[# @LINE + 6 ]]:16: warning: use of potentially unchecked data
;; CHECK-NEXT:     (stx-burn? amount (as-contract tx-sender))
;; CHECK-NEXT:                ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:34: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-stx-burn (amount uint))
;; CHECK-NEXT:                                   ^~~~~~
    (stx-burn? amount (as-contract tx-sender))
)

(define-fungible-token stackaroo)

(define-public (tainted-ft-burn (amount uint))
;; CHECK: checker:[[# @LINE + 6 ]]:25: warning: use of potentially unchecked data
;; CHECK-NEXT:     (ft-burn? stackaroo amount (as-contract tx-sender))
;; CHECK-NEXT:                         ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:33: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-ft-burn (amount uint))
;; CHECK-NEXT:                                  ^~~~~~
    (ft-burn? stackaroo amount (as-contract tx-sender))
)

(define-public (tainted-ft-transfer (amount uint))
;; CHECK: checker:[[# @LINE + 6 ]]:29: warning: use of potentially unchecked data
;; CHECK-NEXT:     (ft-transfer? stackaroo amount (as-contract tx-sender) tx-sender)
;; CHECK-NEXT:                             ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:37: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-ft-transfer (amount uint))
;; CHECK-NEXT:                                      ^~~~~~
    (ft-transfer? stackaroo amount (as-contract tx-sender) tx-sender)
)

(define-public (tainted-ft-mint (amount uint))
;; CHECK: checker:[[# @LINE + 6 ]]:25: warning: use of potentially unchecked data
;; CHECK-NEXT:     (ft-mint? stackaroo amount (as-contract tx-sender))
;; CHECK-NEXT:                         ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:33: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-ft-mint (amount uint))
;; CHECK-NEXT:                                  ^~~~~~
    (ft-mint? stackaroo amount (as-contract tx-sender))
)

(define-non-fungible-token stackaroo2 uint)

(define-public (tainted-nft-burn (amount uint))
;; CHECK: checker:[[# @LINE + 6 ]]:27: warning: use of potentially unchecked data
;; CHECK-NEXT:     (nft-burn? stackaroo2 amount (as-contract tx-sender))
;; CHECK-NEXT:                           ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE -4 ]]:34: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-nft-burn (amount uint))
;; CHECK-NEXT:                                   ^~~~~~
    (nft-burn? stackaroo2 amount (as-contract tx-sender))
)

(define-public (tainted-nft-transfer (amount uint))
;; CHECK: checker:[[# @LINE + 6 ]]:31: warning: use of potentially unchecked data
;; CHECK-NEXT:     (nft-transfer? stackaroo2 amount (as-contract tx-sender) tx-sender)
;; CHECK-NEXT:                               ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:38: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-nft-transfer (amount uint))
;; CHECK-NEXT:                                       ^~~~~~
    (nft-transfer? stackaroo2 amount (as-contract tx-sender) tx-sender)
)

(define-public (tainted-nft-mint (amount uint))
;; CHECK: checker:[[# @LINE + 6 ]]:27: warning: use of potentially unchecked data
;; CHECK-NEXT:     (nft-mint? stackaroo2 amount (as-contract tx-sender))
;; CHECK-NEXT:                           ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:34: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-nft-mint (amount uint))
;; CHECK-NEXT:                                   ^~~~~~
    (nft-mint? stackaroo2 amount (as-contract tx-sender))
)

(define-data-var myvar uint u0)

(define-public (tainted-var-set (amount uint))
;; CHECK: checker:[[# @LINE + 6 ]]:24: warning: use of potentially unchecked data
;; CHECK-NEXT:     (ok (var-set myvar amount))
;; CHECK-NEXT:                        ^~~~~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:33: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-var-set (amount uint))
;; CHECK-NEXT:                                 ^~~~~~
    (ok (var-set myvar amount))
)

(define-map mymap { key-name-1: uint } { val-name-1: int })

(define-public (tainted-map-set (key uint) (value int))
;; CHECK: checker:[[# @LINE + 12 ]]:37: warning: use of potentially unchecked data
;; CHECK-NEXT:     (ok (map-set mymap {key-name-1: key} {val-name-1: value}))
;; CHECK-NEXT:                                     ^~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:33: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-map-set (key uint) (value int))
;; CHECK-NEXT:                                  ^~~
;; CHECK-NEXT: checker:[[# @LINE + 6 ]]:55: warning: use of potentially unchecked data
;; CHECK-NEXT:     (ok (map-set mymap {key-name-1: key} {val-name-1: value}))
;; CHECK-NEXT:                                                       ^~~~~
;; CHECK-NEXT: checker:[[# @LINE - 10 ]]:44: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-map-set (key uint) (value int))
;; CHECK-NEXT:                                             ^~~~~
    (ok (map-set mymap {key-name-1: key} {val-name-1: value}))
)

(define-public (tainted-map-insert (key uint) (value int))
;; CHECK: checker:[[# @LINE + 12 ]]:40: warning: use of potentially unchecked data
;; CHECK-NEXT:     (ok (map-insert mymap {key-name-1: key} {val-name-1: value}))
;; CHECK-NEXT:                                        ^~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:36: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-map-insert (key uint) (value int))
;; CHECK-NEXT:                                     ^~~
;; CHECK-NEXT: checker:[[# @LINE + 6 ]]:58: warning: use of potentially unchecked data
;; CHECK-NEXT:     (ok (map-insert mymap {key-name-1: key} {val-name-1: value}))
;; CHECK-NEXT:                                                          ^~~~~
;; CHECK-NEXT: checker:[[# @LINE - 10 ]]:47: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-map-insert (key uint) (value int))
;; CHECK-NEXT:                                                ^~~~~
    (ok (map-insert mymap {key-name-1: key} {val-name-1: value}))
)

(define-public (tainted-map-delete (key uint))
;; CHECK: checker:[[# @LINE + 6 ]]:40: warning: use of potentially unchecked data
;; CHECK-NEXT:     (ok (map-delete mymap {key-name-1: key}))
;; CHECK-NEXT:                                        ^~~
;; CHECK-NEXT: checker:[[# @LINE - 4 ]]:36: note: source of untrusted input here
;; CHECK-NEXT: (define-public (tainted-map-delete (key uint))
;; CHECK-NEXT:                                     ^~~
    (ok (map-delete mymap {key-name-1: key}))
)
