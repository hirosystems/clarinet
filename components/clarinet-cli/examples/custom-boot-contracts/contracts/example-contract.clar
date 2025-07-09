;; Example contract that demonstrates interaction with custom boot contracts

(define-constant contract-owner principal tx-sender)

;; Function to test custom pox-4 contract
(define-public (test-custom-pox-info)
    (begin
        ;; Call the custom get-pox-info function from the overridden pox-4 contract
        (contract-call? 'SP000000000000000000002Q6VF78.pox-4 get-pox-info)
    )
)

;; Function to test custom costs contract
(define-public (test-custom-costs (function-name (string-ascii 128)))
    (begin
        ;; Call the custom get-cost function from the overridden costs contract
        (contract-call? 'SP000000000000000000002Q6VF78.costs get-cost function-name)
    )
)

;; Function to test custom costs-2 contract
(define-public (test-custom-costs-2 (function-name (string-ascii 128)))
    (begin
        ;; Call the custom get-cost-2 function from the overridden costs-2 contract
        (contract-call? 'SP000000000000000000002Q6VF78.costs-2 get-cost-2 function-name)
    )
)

;; Function to demonstrate that custom boot contracts are being used
(define-read-only (get-boot-contract-info)
    (ok {
        pox-4-min-amount: (unwrap! (contract-call? 'SP000000000000000000002Q6VF78.pox-4 get-pox-info) (err u1))
        costs-runtime: (unwrap! (contract-call? 'SP000000000000000000002Q6VF78.costs get-cost "test") (err u2))
    })
)
