(impl-trait .mock-ft-trait.mock-ft-trait)

;; Defines the Arkadiko Governance Token according to the SRC20 Standard
(define-fungible-token diko)

;; errors
(define-constant err-unauthorized u1)

(define-read-only (get-total-supply)
  (ok (ft-get-supply diko))
)

(define-read-only (get-name)
  (ok "Arkadiko")
)

(define-read-only (get-symbol)
  (ok "DIKO")
)

(define-read-only (get-decimals)
  (ok u6)
)

(define-read-only (get-balance-of (account principal))
  (ok (ft-get-balance diko account))
)

;; TODO - finalize before mainnet deployment
(define-read-only (get-token-uri)
  (ok none)
)

(define-public (transfer (amount uint) (sender principal) (recipient principal))
  (begin
    (ft-transfer? diko amount sender recipient)
  )
)

;; TODO - finalize before mainnet deployment
(define-public (mint (amount uint) (recipient principal))
  (err err-unauthorized)
)

(define-public (burn (amount uint) (sender principal))
  (ok (ft-burn? diko amount sender))
)

;; Initialize the contract
(begin
  ;; mint 1 million tokens
  (try! (ft-mint? diko u890000000000 'S02J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKPVKG2CE))
  (try! (ft-mint? diko u150000000000 'ST1QV6WVNED49CR34E58CRGA0V58X281FAS1TFBWF))
  (try! (ft-mint? diko u150000000000 'ST238B5WSC8B8XETWDXMH7HZC2MJ2RNTYY15YY7SH))
  (try! (ft-mint? diko u1000000000 'ST2ZRX0K27GW0SP3GJCEMHD95TQGJMKB7G9Y0X1MH))
)
