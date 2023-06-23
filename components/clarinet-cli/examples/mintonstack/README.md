# Using Chainhook to Mint Automatic NFTs based on bitcoin txns to a wallet address ("mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC")

A Clarity smart contract that implements the `sip009-nft-trait` trait and defines a non-fungible token (NFT) called `bitbadge`. 

It includes functions for minting and transferring the NFT, as well as a `mint-to-bitcoin-address` function that takes in a `scriptSig` argument, extracts the corresponding stacks address using the `p2pkh-to-principal` function, and mints an NFT to it.

The `slice?` function defined in this code takes in three arguments: an input buffer, a start index, and an end index. It returns a new buffer containing the slice of the input buffer from the start index to the end index. If the start and end indices are not valid, it returns `none`.

This function is used in the `p2pkh-to-principal` function to extract a slice of the `scriptSig` buffer. The `slice?` function is called with three arguments: the `scriptSig` buffer, the start index `(- (len scriptSig) u33)`, and the end index `(len scriptSig)`.