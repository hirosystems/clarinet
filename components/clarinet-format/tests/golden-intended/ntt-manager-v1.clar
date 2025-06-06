;; Title: sip10-locking/ntt-manager
;; Version: v1
;; Summary:
;; Description:

;; This contract is for the sBTC fungibile token defined at SM3VDXK3WZZSA84XXFKAFAF15NNZX32CTSG82JFQ4.sbtc-token
;; To deploy for a different SIP-10 token, replace all references to the sBTC contract with references to the chosen token's contract

;;;; Traits

(use-trait transceiver-trait .sip-10-locking-transceiver-trait-v1.transceiver-trait)

;;;; Token Definitions

;;;; Constants

;; Start error codes at 3000 because wormhole-core uses 1000-3000 range

;; Admin function called by non-admin account
(define-constant ERR_UNAUTHORIZED (err u3001))
;; Tried to use an unauthorized transceiver
(define-constant ERR_TRANSCEIVER_UNAUTHORIZED (err u3002))
;; No transceiver found for protocol
(define-constant ERR_TRANSCEIVER_NOT_FOUND (err u3003))
;; Account does not have pending tokens to claim
(define-constant ERR_NO_TOKENS_PENDING (err u3004))
;; No valid recipient found
(define-constant ERR_RECV_NO_RECIPIENT (err u3005))
;; Message has already been processed
(define-constant ERR_RECV_ALREADY_USED (err u3006))

;; Known protocols
(define-constant PROTOCOL_WORMHOLE u1)
(define-constant PROTOCOL_AXELAR u2)

;;;; Data Vars

;;;; Data Maps

;; Tokens that have been released by a VAA, but we don't know where to send them on the Stacks chain
;; Only relevant to protocols that use 32-byte addressing instead of full Stacks addresses
;; Must be claimed by `claim-tokens`
(define-map tokens-pending
  {
    protocol: uint, ;; ID of protocol on which funds were sent
    addr32: (buff 32), ;; Recipient's principal mapped to 32-byte address
  }
  uint ;; Amount unlocked and pending
)

;; Set of protocols and current active transceiver for each
;; Each protocol can only have one transceiver at a time
(define-map protocols
  uint ;; Protocol
  principal ;; Transceiver contract
)

;; Inverse of `protocols`
;; No two contracts in this map should have the same protocol
(define-map transceivers
  principal ;; Transceiver contract
  uint ;; Protocol
)

;; Accounts allowed to call admin functions
;; Defaults to contract deployer
(define-map admins
  principal ;; Admin account
  bool ;; Is approved?
)
(map-set admins tx-sender true)

;; Prevent message replay by tracking messages processed
(define-map consumed-messages
  (buff 32) ;; Unique ID determined by transceiver
  bool ;; Consumed?
)

;;;; Public Functions: Admin

;; ALL FUNCTIONS HERE ARE ADMIN FUNCTIONS AND MUST CALL `check-admin`!

;; @desc Add new admin account for this contract
(define-public (admin-add-admin (account principal))
  (begin
    (try! (check-admin))
    (ok (map-set admins account true))
  )
)

;; @desc Remove admin account for this contract
(define-public (admin-remove-admin (account principal))
  (begin
    (try! (check-admin))
    (ok (map-delete admins account))
  )
)

;; @desc Register transceiver and remove existing transceiver for protocol
(define-public (admin-register-transceiver (transceiver <transceiver-trait>))
  (let (
      (contract (contract-of transceiver))
      (protocol (try! (contract-call? transceiver get-protocol-id)))
    )
    (try! (check-admin))
    (match (get-protocol-transceiver protocol)
      ;; Remove old transceiver for protocol
      old
      (map-delete transceivers old)
      ;; No existing transceiver, do nothing
      true
    )
    (map-set transceivers contract protocol)
    (map-set protocols protocol contract)
    (ok true)
  )
)

;; @desc Unregister transceiver
(define-public (admin-unregister-transceiver (transceiver <transceiver-trait>))
  (let (
      (contract (contract-of transceiver))
      (protocol (try! (contract-call? transceiver get-protocol-id)))
    )
    (try! (check-admin))
    (map-delete transceivers contract)
    ;; This should be safe because there can never be more than one transceiver per protocol
    (map-delete protocols protocol)
    (ok true)
  )
)

;;;; Public Functions: Token transfer

;; ALL FUNCTIONS THAT TAKE <transceiver-trait> MUST CALL `check-transceiver`!

;; @desc Lock tokens and send cross-chain message via specified transceiver
(define-public (send-tokens
    (transceiver <transceiver-trait>)
    (amount uint)
    (fee uint)
    (to-chain (buff 16))
    (to (buff 32))
  )
  (begin
    (try! (check-transceiver transceiver))
    ;; Take tokens from user and lock them in this contract
    ;; TODO: Add memo?
    (try! (contract-call? 'SM3VDXK3WZZSA84XXFKAFAF15NNZX32CTSG82JFQ4.sbtc-token
      transfer amount tx-sender (get-contract-principal) none
    ))
    (contract-call? transceiver send-token-transfer amount
      'SM3VDXK3WZZSA84XXFKAFAF15NNZX32CTSG82JFQ4.sbtc-token to to-chain fee
    )
  )
)

;; @desc Lock tokens and send cross-chain message via specified transceiver
(define-public (receive-tokens
    (transceiver <transceiver-trait>)
    (bytes (buff 8192))
  )
  (let (
      (check (try! (check-transceiver transceiver)))
      (result (try! (contract-call? transceiver parse-and-verify-token-transfer bytes)))
      (amount (get amount result))
    )
    ;; Check for message replay
    (asserts! (map-insert consumed-messages (get uid result) true)
      ERR_RECV_ALREADY_USED
    )
    (match (get recipient result)
      ;; Registered, unlock and send to account
      ;; TODO: Add memo?
      recipient
      (try! (as-contract (contract-call? 'SM3VDXK3WZZSA84XXFKAFAF15NNZX32CTSG82JFQ4.sbtc-token
        transfer amount tx-sender recipient none
      )))
      ;; Not registered yet. Allow account to claim later
      (let ((idx {
          protocol: (unwrap! (get-transceiver-protocol transceiver)
            ERR_TRANSCEIVER_NOT_FOUND
          ),
          addr32: (unwrap! (get recipient-addr32 result) ERR_RECV_NO_RECIPIENT),
        }))
        (map-set tokens-pending idx
          (+ amount (default-to u0 (get-tokens-pending idx)))
        )
      )
    )
    (ok true)
  )
)

;; @desc Release any pending tokens for given principal
;;       Returns `(ok amount-transferred)` on success
(define-public (release-tokens-pending
    (transceiver <transceiver-trait>)
    (recipient principal)
  )
  (let (
      (check (try! (check-transceiver transceiver)))
      (idx {
        protocol: (unwrap! (get-transceiver-protocol transceiver) ERR_TRANSCEIVER_NOT_FOUND),
        addr32: (try! (contract-call? transceiver get-32-byte-address recipient)),
      })
      (amount (unwrap! (get-tokens-pending idx) ERR_NO_TOKENS_PENDING))
    )
    (try! (as-contract (contract-call? 'SM3VDXK3WZZSA84XXFKAFAF15NNZX32CTSG82JFQ4.sbtc-token
      transfer amount tx-sender recipient none
    )))
    (map-delete tokens-pending idx)
    (ok amount)
  )
)

;;;; Read-only Functions

(define-read-only (is-admin (account principal))
  (default-to false (map-get? admins account))
)

(define-read-only (get-tokens-locked (account principal))
  (contract-call? 'SM3VDXK3WZZSA84XXFKAFAF15NNZX32CTSG82JFQ4.sbtc-token
    get-balance (get-contract-principal)
  )
)

(define-read-only (get-tokens-pending (idx {
  protocol: uint,
  addr32: (buff 32),
}))
  (map-get? tokens-pending idx)
)

(define-read-only (get-transceiver-protocol (transceiver <transceiver-trait>))
  (map-get? transceivers (contract-of transceiver))
)

(define-read-only (get-protocol-transceiver (protocol uint))
  (map-get? protocols protocol)
)

;; @desc Check transceiver is registered
;;       Returns `(ok protocol)` if so
(define-read-only (check-transceiver (transceiver <transceiver-trait>))
  (ok (unwrap! (get-transceiver-protocol transceiver) ERR_TRANSCEIVER_UNAUTHORIZED))
)

;;;; Private Functions

;; @desc Get this contract's principal
(define-private (get-contract-principal)
  (as-contract tx-sender)
)

;; @desc Check if caller is admin
(define-private (check-admin)
  (ok (asserts! (is-admin contract-caller) ERR_UNAUTHORIZED))
)
