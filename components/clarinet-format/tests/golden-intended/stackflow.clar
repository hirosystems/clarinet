;; title: stackflow-token
;; author: brice.btc
;; version: 0.6.0
;; summary: This contract defines a trait that Stackflow contracts for SIP-010
;;   tokens must implement.

;; MIT License

;; Copyright (c) 2024-2025 obycode, LLC

;; Permission is hereby granted, free of charge, to any person obtaining a copy
;; of this software and associated documentation files (the "Software"), to deal
;; in the Software without restriction, including without limitation the rights
;; to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
;; copies of the Software, and to permit persons to whom the Software is
;; furnished to do so, subject to the following conditions:

;; The above copyright notice and this permission notice shall be included in all
;; copies or substantial portions of the Software.

;; THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
;; IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
;; FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
;; AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
;; LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
;; OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
;; SOFTWARE.

(use-trait sip-010 'SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.sip-010-trait-ft-standard.sip-010-trait)

(define-trait stackflow-token
  (
    (fund-pipe
      (
        (optional <sip-010>) ;; token
        uint                 ;; amount
        principal            ;; with
        uint                 ;; nonce
      )
      (response {
        token: (optional principal),
        principal-1: principal,
        principal-2: principal
        } uint
      )
    )
    (close-pipe
      (
        (optional <sip-010>) ;; token
        principal            ;; with
        uint                 ;; my-balance
        uint                 ;; their-balance
        (buff 65)            ;; my-signature
        (buff 65)            ;; their-signature
        uint                 ;; nonce
      )
      (response bool uint)
    )
    (force-cancel
      (
        (optional <sip-010>) ;; token
        principal            ;; with
      )
      (response uint uint)
    )
    (force-close
      (
        (optional <sip-010>) ;; token
        principal            ;; with
        uint                 ;; my-balance
        uint                 ;; their-balance
        (buff 65)            ;; my-signature
        (buff 65)            ;; their-signature
        uint                 ;; nonce
        uint                 ;; action
        principal            ;; actor
        (optional (buff 32)) ;; secret
        (optional uint)      ;; valid-after
      )
      (response uint uint)
    )
    (dispute-closure
      (
        (optional <sip-010>) ;; token
        principal            ;; with
        uint                 ;; my-balance
        uint                 ;; their-balance
        (buff 65)            ;; my-signature
        (buff 65)            ;; their-signature
        uint                 ;; nonce
        uint                 ;; action
        principal            ;; actor
        (optional (buff 32)) ;; secret
        (optional uint)      ;; valid-after
      )
      (response bool uint)
    )
    (finalize
      (
        (optional <sip-010>) ;; token
        principal            ;; with
      )
      (response bool uint)
    )
    (deposit
      (
        uint                 ;; amount
        (optional <sip-010>) ;; token
        principal            ;; with
        uint                 ;; my-balance
        uint                 ;; their-balance
        (buff 65)            ;; my-signature
        (buff 65)            ;; their-signature
        uint                 ;; nonce
      )
      (response {
        token: (optional principal),
        principal-1: principal,
        principal-2: principal
      } uint
      )
    )
    (withdraw
      (
        uint                 ;; amount
        (optional <sip-010>) ;; token
        principal            ;; with
        uint                 ;; my-balance
        uint                 ;; their-balance
        (buff 65)            ;; my-signature
        (buff 65)            ;; their-signature
        uint                 ;; nonce
      )
      (response {
        token: (optional principal),
        principal-1: principal,
        principal-2: principal
      } uint
    )
    )
  )
)
