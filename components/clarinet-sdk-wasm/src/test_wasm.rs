use super::core::DeployContractArgs;
use crate::core::{CallFnArgs, ContractOptions, EpochString, TransactionRes, SDK};
use clarity::vm::Value as ClarityValue;
use js_sys::Function as JsFunction;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

async fn init_sdk() -> SDK {
    let js_noop = JsFunction::new_no_args("return");
    let mut sdk = SDK::new(js_noop.clone(), js_noop, None);
    let _ = sdk.init_empty_session(JsValue::undefined()).await;
    sdk.set_epoch(EpochString::new("3.0"));
    sdk
}

#[track_caller]
fn deploy_basic_contract(sdk: &mut SDK) -> TransactionRes {
    let contract = DeployContractArgs::new(
        "basic-contract".into(),
        "(define-private (two) (+ u1 u1))".into(),
        ContractOptions::new(None),
        "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM".into(),
    );
    sdk.deploy_contract(&contract).unwrap()
}

#[wasm_bindgen_test]
async fn it_can_execute_clarity_code() {
    let mut sdk = init_sdk().await;
    let tx = sdk.execute("(+ u41 u1)".into()).unwrap();
    let expected = format!("0x{}", ClarityValue::UInt(42).serialize_to_hex().unwrap());
    assert_eq!(tx.result, expected);
}

#[wasm_bindgen_test]
async fn it_can_set_epoch() {
    let mut sdk = init_sdk().await;
    assert_eq!(sdk.block_height(), 1);
    assert_eq!(sdk.current_epoch(), "3.0");
}

#[wasm_bindgen_test]
async fn it_can_deploy_contract() {
    let mut sdk = init_sdk().await;
    let tx = deploy_basic_contract(&mut sdk);
    let expected = format!("0x{}", ClarityValue::Bool(true).serialize_to_hex().unwrap());
    assert_eq!(tx.result, expected);
}

#[wasm_bindgen_test]
async fn it_can_call_a_private_function() {
    let mut sdk = init_sdk().await;
    let _ = deploy_basic_contract(&mut sdk);
    let tx = sdk
        .call_private_fn(&CallFnArgs::new(
            "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.basic-contract".into(),
            "two".into(),
            vec![],
            "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM".into(),
        ))
        .unwrap();
    let expected = format!("0x{}", ClarityValue::UInt(2).serialize_to_hex().unwrap());
    assert_eq!(tx.result, expected);
}

// #[wasm_bindgen_test]
// async fn it_can_call_remote_data() {
//     let mut sdk = init_sdk().await;
//     let _ = deploy_basic_contract(&mut sdk);
//     let tx = sdk
//         .call_private_fn(&CallFnArgs::new(
//             "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.basic-contract".into(),
//             "two".into(),
//             vec![],
//             "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM".into(),
//         ))
//         .unwrap();
//     let expected = format!("0x{}", ClarityValue::UInt(2).serialize_to_hex().unwrap());
//     assert_eq!(tx.result, expected);
// }
