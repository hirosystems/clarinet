;; Stacks the STX tokens in POX
;; pox contract: SP000000000000000000002Q6VF78.pox
;; https://explorer.stacks.co/txid/0x41356e380d164c5233dd9388799a5508aae929ee1a7e6ea0c18f5359ce7b8c33?chain=mainnet

;; v1
;;  Stack for 1 cycle a time
;;  This way we miss each other cycle (i.e. we stack 1/2) but we can stack everyone's STX.
;;  We cannot stack continuously right now
;; v2
;;  Ideally we can stack more tokens on the same principal
;;  to stay eligible for future increases of reward slot thresholds.
(define-public (pox-stack-stx (amount-ustx uint)
                              (pox-addr (tuple (version (buff 1)) (hashbytes (buff 20))))
                              (start-burn-ht uint)
                              (lock-period uint))
  ;; 1. check `get-stacking-minimum` to see if we have > minimum tokens
  ;; 2. call `stack-stx` for 1 `lock-period` fixed
)
