(let (
    (ids-to-recipient (if (is-eq no-to-recipient u0)
        (list)
        (unwrap-panic (slice? new-available-ids (- (len new-available-ids) no-to-recipient) (len new-available-ids))))
    )
    (sender tx-sender)
)
    (ok true)
)
