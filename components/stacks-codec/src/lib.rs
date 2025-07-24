// Compatibility layer for stacks-codec
// Re-exports transaction types from stackslib

pub mod codec {
    // Re-export the basic codec functionality from clarity, but avoid Error conflicts
    pub use clarity::codec::{
        read_next, read_next_exact, write_next, StacksMessageCodec, MAX_MESSAGE_LEN,
    };
    // Re-export types needed for the helper function
    pub use clarity::types::chainstate::StacksAddress;
    pub use clarity::util::secp256k1::{Secp256k1PrivateKey, Secp256k1PublicKey};
    pub use clarity::vm::types::{
        PrincipalData, QualifiedContractIdentifier, StandardPrincipalData, Value,
    };
    pub use clarity::vm::{ClarityName, ContractName};
    pub use stacks_common::util::hash::Hash160;
    pub use stackslib::chainstate::stacks::*;
    // Re-export StacksString from util_lib
    pub use stackslib::util_lib::strings::StacksString;

    /// Build a contract call transaction
    pub fn build_contract_call_transaction(
        contract_id: String,
        function_name: String,
        args: Vec<Value>,
        nonce: u64,
        fee: u64,
        sender_secret_key: &[u8],
    ) -> StacksTransaction {
        let private_key = Secp256k1PrivateKey::from_slice(sender_secret_key).unwrap();
        let public_key = Secp256k1PublicKey::from_private(&private_key);

        let contract_parts: Vec<&str> = contract_id.split('.').collect();
        if contract_parts.len() != 2 {
            panic!("Invalid contract ID format: {contract_id}");
        }

        let standard_principal =
            PrincipalData::parse_standard_principal(contract_parts[0]).unwrap();
        let address = StacksAddress::new(
            standard_principal.version(),
            Hash160::from_data(&standard_principal.1),
        )
        .unwrap();
        let contract_name = ContractName::from(contract_parts[1]);
        let function_name = ClarityName::try_from(function_name).unwrap();

        let contract_call = TransactionContractCall {
            address,
            contract_name,
            function_name,
            function_args: args,
        };

        let payload = TransactionPayload::ContractCall(contract_call);

        let signer_addr = StacksAddress::from_public_keys(
            0,
            &clarity::address::AddressHashMode::SerializeP2PKH,
            1,
            &vec![public_key],
        )
        .unwrap();

        let spending_condition =
            TransactionSpendingCondition::Singlesig(SinglesigSpendingCondition {
                signer: *signer_addr.bytes(),
                nonce,
                tx_fee: fee,
                hash_mode: SinglesigHashMode::P2PKH,
                key_encoding: TransactionPublicKeyEncoding::Compressed,
                signature: clarity::util::secp256k1::MessageSignature::empty(),
            });

        let auth = TransactionAuth::Standard(spending_condition);
        let unsigned_tx = StacksTransaction {
            version: TransactionVersion::Testnet,
            chain_id: 0x80000000,
            auth,
            anchor_mode: TransactionAnchorMode::Any,
            post_condition_mode: TransactionPostConditionMode::Allow,
            post_conditions: vec![],
            payload,
        };

        let mut tx_signer = StacksTransactionSigner::new(&unsigned_tx);
        tx_signer.sign_origin(&private_key).unwrap();
        tx_signer.get_tx().unwrap()
    }
}
