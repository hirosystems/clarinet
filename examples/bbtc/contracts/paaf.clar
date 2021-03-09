;; Pox as a Foundation
;; The goal of this contract is to provide a framework, aiming at easing implementations of 
;; DeFi protocols relying on PoX.
;; The idea is the following: protocols can emit events, 
;; (define-map contracts principal (list 5 { key: (string-ascii 16), mandatory: bool }))
;; Note: it would be great if this contract could be used as a way to protect actors from contracts misbehaving.
;; Note: not-behaving ≠ misbehaving
(define-constant ERR_UNABLE_TO_LOCK_COLLATERAL u1)

(define-map funding { contract: principal, actor: principal} { amount: uint })
(define-non-fungible-token paaf-cycle-share { contract: principal, actor: principal, share: uint, cycle: uint })

(define-public (register-contract (contract principal) (params (list 3 { key: (string-ascii 16), mandatory: bool })))
    (ok true))

(define-public (register-actor (contract principal) (actor principal) (collateral uint) (num-of-cycles uint) (params (list 3 { value: int })))
    (let (
        (self (as-contract tx-sender))
        (current-funding (default-to 0 (map-get? funding { contract: contract, actor: actor }))))
    ;; Transfer funds
    (unwrap! 
        (stx-transfer? collateral actor self) 
        (err ERR_UNABLE_TO_LOCK_COLLATERAL))
    ;; Keep track of this transfer
    (map-set funding { contract: contract, actor: actor } { amount: (+ current-funding collateral) })
    (ok true)))

(define-public (update-actor-registration (contract principal) (actor principal) (additional-collateral uint) (additonal-num-of-cycles uint) (params (list 3 { value: int })))
    (let (
        (self (as-contract tx-sender))
        (current-funding (default-to 0 (map-get? funding { contract: contract, actor: actor }))))
    ;; Transfer funds
    (unwrap! 
        (stx-transfer? collateral actor self) 
        (err ERR_UNABLE_TO_LOCK_COLLATERAL))
    ;; Keep track of this transfer
    (map-set funding { contract: contract, actor: actor } { amount: (+ current-funding collateral) })
    (ok true)))


(define-public ( (contract principal) (actor principal) (num-of-cycles uint) (collateral uint) (params (list 3 { value: int })))
    (ok true))

;; Actors selection
(define-public (select-1-actor (contract principal) (predicat (list 3 { min: (option int), max: (option int)}))) 
    (ok (list)))

(define-public (select-2-actors (contract principal) (predicat (list 3 { min: (option int), max: (option int)})))
    (ok (list)))

(define-public (select-4-actors (contract principal) (predicat (list 3 { min: (option int), max: (option int)}))) 
    (ok (list)))

(define-public (select-8-actors (contract principal) (predicat (list 3 { min: (option int), max: (option int)}))) 
    (ok (list)))

(define-public (select-16-actors (contract principal) (predicat (list 3 { min: (option int), max: (option int)}))) 
    (ok (list)))

(define-public (select-32-actors (contract principal) (predicat (list 3 { min: (option int), max: (option int)}))) 
    (ok (list)))

(define-public (select-64-actors (contract principal) (predicat (list 3 { min: (option int), max: (option int)}))) 
    (ok (list)))

(define-public (select-128-actors (contract principal) (predicat (list 3 { min: (option int), max: (option int)}))) 
    (ok (list)))

(define-public (select-256-actors (contract principal) (predicat (list 3 { min: (option int), max: (option int)}))) 
    (ok (list)))
