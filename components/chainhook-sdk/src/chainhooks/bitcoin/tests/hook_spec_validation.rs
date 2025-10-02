use std::sync::LazyLock;

use chainhook_types::BitcoinNetwork;
use test_case::test_case;

use super::*;
use crate::chainhooks::bitcoin::InscriptionFeedData;
use crate::chainhooks::types::{ChainhookSpecificationNetworkMap, HttpHook};

static TXID_NO_PREFIX: LazyLock<String> = LazyLock::new(|| "1234567890123456789012345678901234567890123456789012345678901234".into());
static TXID_NOT_HEX: LazyLock<String> = LazyLock::new(|| "0xw234567890123456789012345678901234567890123456789012345678901234".into());
static TXID_SHORT: LazyLock<String> = LazyLock::new(|| "0x234567890123456789012345678901234567890123456789012345678901234".into());
static TXID_LONG: LazyLock<String> = LazyLock::new(|| "0x11234567890123456789012345678901234567890123456789012345678901234".into());
static TXID_VALID: LazyLock<String> = LazyLock::new(|| "0x1234567890123456789012345678901234567890123456789012345678901234".into());

static TXID_PREDICATE_ERR: LazyLock<String> = LazyLock::new(|| "invalid predicate for scope 'txid': txid must be a 32 byte (64 character) hexadecimal string prefixed with '0x'".into());
static INPUT_TXID_ERR: LazyLock<String> = LazyLock::new(|| "invalid predicate for scope 'inputs': txid must be a 32 byte (64 character) hexadecimal string prefixed with '0x'".into());
static DESCRIPTOR_KEY_SHORT_ERR: LazyLock<String> = LazyLock::new(|| "invalid predicate for scope 'outputs': invalid descriptor: unexpected «Key too short (<66 char), doesn't match any format»".into());
static INVALID_DESCRIPTOR_ERR: LazyLock<String> = LazyLock::new(|| "invalid predicate for scope 'outputs': invalid descriptor: Anything but c:pk(key) (P2PK), c:pk_h(key) (P2PKH), and thresh_m(k,...) up to n=3 is invalid by standardness (bare).\n                ".into());
static INVALID_URL_ERR: LazyLock<String> = LazyLock::new(|| "invalid 'http_post' data: url string must be a valid Url: relative URL without a base".into());
static INVALID_HTTP_HEADER_ERR: LazyLock<String> = LazyLock::new(|| "invalid 'http_post' data: auth header must be a valid header value: failed to parse header value".into());
static INVALID_SPEC_NETWORK_MAP_ERR: LazyLock<String> = LazyLock::new(|| "invalid Bitcoin predicate 'test' for network regtest: invalid 'then_that' value: invalid 'http_post' data: url string must be a valid Url: relative URL without a base\ninvalid Bitcoin predicate 'test' for network regtest: invalid 'then_that' value: invalid 'http_post' data: auth header must be a valid header value: failed to parse header value\ninvalid Bitcoin predicate 'test' for network regtest: invalid 'if_this' value: invalid predicate for scope 'txid': txid must be a 32 byte (64 character) hexadecimal string prefixed with '0x'\ninvalid Bitcoin predicate 'test' for network testnet: invalid 'then_that' value: invalid 'http_post' data: url string must be a valid Url: relative URL without a base\ninvalid Bitcoin predicate 'test' for network testnet: invalid 'then_that' value: invalid 'http_post' data: auth header must be a valid header value: failed to parse header value\ninvalid Bitcoin predicate 'test' for network testnet: invalid 'if_this' value: invalid predicate for scope 'txid': txid must be a 32 byte (64 character) hexadecimal string prefixed with '0x'\ninvalid Bitcoin predicate 'test' for network signet: invalid 'then_that' value: invalid 'http_post' data: url string must be a valid Url: relative URL without a base\ninvalid Bitcoin predicate 'test' for network signet: invalid 'then_that' value: invalid 'http_post' data: auth header must be a valid header value: failed to parse header value\ninvalid Bitcoin predicate 'test' for network signet: invalid 'if_this' value: invalid predicate for scope 'txid': txid must be a 32 byte (64 character) hexadecimal string prefixed with '0x'\ninvalid Bitcoin predicate 'test' for network mainnet: invalid 'then_that' value: invalid 'http_post' data: url string must be a valid Url: relative URL without a base\ninvalid Bitcoin predicate 'test' for network mainnet: invalid 'then_that' value: invalid 'http_post' data: auth header must be a valid header value: failed to parse header value\ninvalid Bitcoin predicate 'test' for network mainnet: invalid 'if_this' value: invalid predicate for scope 'txid': txid must be a 32 byte (64 character) hexadecimal string prefixed with '0x'".into());

static INVALID_TXID_PREDICATE: LazyLock<BitcoinPredicateType> = LazyLock::new(|| BitcoinPredicateType::Txid(ExactMatchingRule::Equals("test".into())));
static INVALID_HOOK_ACTION: LazyLock<HookAction> = LazyLock::new(|| HookAction::HttpPost(HttpHook { url: "".into(), authorization_header: "\n".into() }));
static ALL_INVALID_SPEC: LazyLock<BitcoinChainhookSpecification> = LazyLock::new(|| BitcoinChainhookSpecification::new(INVALID_TXID_PREDICATE.clone(), INVALID_HOOK_ACTION.clone()));
static ALL_INVALID_SPEC_NETWORK_MAP: LazyLock<ChainhookSpecificationNetworkMap> = LazyLock::new(|| ChainhookSpecificationNetworkMap::Bitcoin(BitcoinChainhookSpecificationNetworkMap {
    uuid: "test".into(),
    owner_uuid: None,
    name: "test".into(),
    version: 1,
    networks: BTreeMap::from([
        (BitcoinNetwork::Regtest, ALL_INVALID_SPEC.clone()),
        (BitcoinNetwork::Signet, ALL_INVALID_SPEC.clone()),
        (BitcoinNetwork::Mainnet, ALL_INVALID_SPEC.clone()),
        (BitcoinNetwork::Testnet, ALL_INVALID_SPEC.clone()),
    ])
}));

// BitcoinPredicateType::Block
#[test_case(&BitcoinPredicateType::Block, None; "block")]
// BitcoinPredicateType::Txid
#[test_case(
    &BitcoinPredicateType::Txid(ExactMatchingRule::Equals((*TXID_NO_PREFIX).clone())),
    Some(vec![(*TXID_PREDICATE_ERR).clone()]); "txid without 0x"
)]
#[test_case(
    &BitcoinPredicateType::Txid(ExactMatchingRule::Equals((*TXID_NOT_HEX).clone())),
    Some(vec![(*TXID_PREDICATE_ERR).clone()]); "txid not hex"
)]
#[test_case(
    &BitcoinPredicateType::Txid(ExactMatchingRule::Equals((*TXID_SHORT).clone())),
    Some(vec![(*TXID_PREDICATE_ERR).clone()]); "txid too short"
)]
#[test_case(
    &BitcoinPredicateType::Txid(ExactMatchingRule::Equals((*TXID_LONG).clone())),
    Some(vec![(*TXID_PREDICATE_ERR).clone()]); "txid too long"
)]
#[test_case(
    &BitcoinPredicateType::Txid(ExactMatchingRule::Equals((*TXID_VALID).clone())),
    None; "txid just right"
)]
// BitcoinPredicateType::Inputs
#[test_case(
    &BitcoinPredicateType::Inputs(InputPredicate::Txid(TxinPredicate { txid: (*TXID_NO_PREFIX).clone(), vout: 0})),
    Some(vec![(*INPUT_TXID_ERR).clone()]); "inputs txid without 0x"
)]
#[test_case(
    &BitcoinPredicateType::Inputs(InputPredicate::Txid(TxinPredicate { txid: (*TXID_NOT_HEX).clone(), vout: 0})),
    Some(vec![(*INPUT_TXID_ERR).clone()]); "inputs txid not hex"
)]
#[test_case(
    &BitcoinPredicateType::Inputs(InputPredicate::Txid(TxinPredicate { txid: (*TXID_SHORT).clone(), vout: 0})),
    Some(vec![(*INPUT_TXID_ERR).clone()]); "inputs txid too short"
)]
#[test_case(
    &BitcoinPredicateType::Inputs(InputPredicate::Txid(TxinPredicate { txid: (*TXID_LONG).clone(), vout: 0})),
    Some(vec![(*INPUT_TXID_ERR).clone()]); "inputs txid too long"
)]
#[test_case(
    &BitcoinPredicateType::Inputs(InputPredicate::Txid(TxinPredicate { txid: (*TXID_VALID).clone(), vout: 0})),
    None; "inputs txid just right"
)]
// BitcoinPredicateType::Outputs
#[test_case(
    &BitcoinPredicateType::Outputs(OutputPredicate::OpReturn(MatchingRule::Equals("".into()))),
    None; "outputs opreturn"
)]
#[test_case(
    &BitcoinPredicateType::Outputs(OutputPredicate::P2pkh(ExactMatchingRule::Equals("".into()))),
    None; "outputs p2pkh"
)]
#[test_case(
    &BitcoinPredicateType::Outputs(OutputPredicate::P2sh(ExactMatchingRule::Equals("".into()))),
    None; "outputs p2sh"
)]
#[test_case(
    &BitcoinPredicateType::Outputs(OutputPredicate::P2wpkh(ExactMatchingRule::Equals("".into()))),
    None; "outputs p2wpkh"
)]
#[test_case(
    &BitcoinPredicateType::Outputs(OutputPredicate::P2wsh(ExactMatchingRule::Equals("".into()))),
    None; "outputs p2wsh"
)]
#[test_case(
    &BitcoinPredicateType::Outputs(OutputPredicate::Descriptor(
        DescriptorMatchingRule {
            expression: "wpkh(02f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9)".into(),
            range: None
        }
    )),
    None; "outputs descriptor ok"
)]
#[test_case(
    &BitcoinPredicateType::Outputs(OutputPredicate::Descriptor(
        DescriptorMatchingRule {
            expression: "wpkh(0)".into(),
            range: None
        }
    )),
    Some(vec![(*DESCRIPTOR_KEY_SHORT_ERR).clone()]); "outputs descriptor too short"
)]
#[test_case(
    &BitcoinPredicateType::Outputs(OutputPredicate::Descriptor(
        DescriptorMatchingRule {
            expression: "0".into(),
            range: None
        }
    )),
    Some(vec![(*INVALID_DESCRIPTOR_ERR).clone()]); "outputs invalid descriptor"
)]
// BitcoinPredicateType::StacksProtocol
#[test_case(&BitcoinPredicateType::StacksProtocol(StacksOperations::StackerRewarded), None; "stacks protocol")]
// BitcoinPredicateType::OrdinalsProtocol
#[test_case(&BitcoinPredicateType::OrdinalsProtocol(OrdinalOperations::InscriptionFeed(InscriptionFeedData { meta_protocols: None})), None; "ordinals protocol")]
fn it_validates_bitcoin_predicates(
    predicate: &BitcoinPredicateType,
    expected_err: Option<Vec<String>>,
) {
    if let Err(e) = predicate.validate() {
        if let Some(expected) = expected_err {
            assert_eq!(e, expected);
        } else {
            panic!("Unexpected error in predicate validation: {:?}", predicate);
        }
    } else if expected_err.is_some() {
        panic!(
            "Missing expected error for predicate validation: {:?}",
            predicate
        );
    }
}

#[test_case(&*INVALID_HOOK_ACTION, Some(vec![(*INVALID_URL_ERR).clone(), (*INVALID_HTTP_HEADER_ERR).clone()]); "invalid http_post action"
)]
fn it_validates_hook_actions(action: &HookAction, expected_err: Option<Vec<String>>) {
    if let Err(e) = action.validate() {
        if let Some(expected) = expected_err {
            assert_eq!(e, expected);
        } else {
            panic!("Unexpected error in predicate validation: {:?}", action);
        }
    } else if expected_err.is_some() {
        panic!(
            "Missing expected error for predicate validation: {:?}",
            action
        );
    }
}

#[test_case(&*ALL_INVALID_SPEC_NETWORK_MAP, (*INVALID_SPEC_NETWORK_MAP_ERR).clone())]
fn it_validates_bitcoin_chainhook_specs(
    predicate: &ChainhookSpecificationNetworkMap,
    expected_err: String,
) {
    if let Err(e) = predicate.validate() {
        assert_eq!(e, expected_err);
    } else {
        panic!(
            "Missing expected error for predicate validation: {:?}",
            predicate
        );
    }
}
