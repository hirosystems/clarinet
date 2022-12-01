
key alice {
}

key bob {
}

key carol {
}

key_derivation alice_derived {
    xprv = alice.xprv
    path = "m/84'/1'/0'/0"
}

key_derivation bob_derived {
    xprv = bob.xprv
    path = "m/84'/1'/0'/0"
}

key_derivation carol_derived {
    xprv = carol.xprv
    path = "m/84'/1'/0'/0"
}

timelock at_least_2_blocks {
    input_older_than = 2
}

threshold quorum_abc {
    min = 3
    conditions = [
        alice_derived.public_key_hash,
        bob_derived.public_key_hash,
        pkh(carol_derived),
        at_least_2_blocks,
    ]
}

fund funding_quorum_abc {
    network = testnet
    amount = 10000
    sender = alice
    recipient = quorum_abc
}
