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
