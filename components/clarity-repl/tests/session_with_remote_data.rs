use std::path::PathBuf;

use clarity::types::chainstate::{BlockHeaderHash, StacksBlockId};
use clarity::types::StacksEpochId;
use clarity::util::hash::hex_bytes;
use clarity::vm::{EvaluationResult, Value};
use clarity_repl::repl::settings::{ApiUrl, RemoteDataSettings};
use clarity_repl::repl::{Session, SessionSettings};

#[track_caller]
fn eval_snippet(session: &mut Session, snippet: &str) -> Value {
    let execution_res = session.eval(snippet.to_string(), false).unwrap();
    match execution_res.result {
        EvaluationResult::Contract(_) => unreachable!(),
        EvaluationResult::Snippet(res) => res.result,
    }
}

fn init_session(initial_heigth: u32) -> Session {
    let mut settings = SessionSettings::default();
    let temp_dir = tempfile::tempdir().unwrap();
    settings.cache_location = Some(temp_dir.path().to_path_buf());
    settings.repl_settings.remote_data = RemoteDataSettings {
        enabled: true,
        api_url: ApiUrl("https://api.testnet.hiro.so".to_string()),
        initial_height: Some(initial_heigth),
    };
    Session::new(settings)
}

// the counter contract is delpoyed on testnet at height #41613
// the initial count value is 0 and is incremented by 1 at #56232
const COUNTER_ADDR: &str = "STJCAB2T9TR2EJM7YS4DM2CGBBVTF7BV237Y8KNV.counter";

#[test]
fn it_starts_in_the_right_epoch() {
    let session = init_session(42000);
    assert_eq!(
        session.interpreter.datastore.get_current_epoch(),
        StacksEpochId::Epoch31
    );
}

#[test]
fn it_can_fetch_remote() {
    // count at block 42000 is 0
    let mut session = init_session(42000);
    let snippet = format!("(contract-call? '{COUNTER_ADDR} get-count)");
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(0));

    // count at block 57000 is 1
    let mut session = init_session(57000);
    let snippet = format!("(contract-call? '{COUNTER_ADDR} get-count)");
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(1));
}

#[test]
fn it_can_get_stacks_block_info() {
    let mut session = init_session(57000);

    let snippet = "(get-stacks-block-info? id-header-hash u42000)";
    let result = eval_snippet(&mut session, snippet);
    let hash =
        StacksBlockId::from_hex("b4678e059aa9b82b1473597087876ef61a5c6a0c35910cd4b797201d6b423a07")
            .unwrap();
    assert_eq!(
        result,
        Value::some(Value::buff_from(hash.to_bytes().to_vec()).unwrap()).unwrap()
    );

    let snippet = "(get-stacks-block-info? header-hash u42000)";
    let result = eval_snippet(&mut session, snippet);
    let hash = BlockHeaderHash::from_hex(
        "eabe9273e76a55384be01866e01a72564a1a1772aa1c2d578c4e918875575840",
    )
    .unwrap();
    assert_eq!(
        result,
        Value::some(Value::buff_from(hash.to_bytes().to_vec()).unwrap()).unwrap()
    );

    let snippet = "(get-stacks-block-info? time u42000)";
    let result = eval_snippet(&mut session, snippet);

    assert_eq!(result, Value::some(Value::UInt(1736792439)).unwrap());
}

#[test]
fn it_can_fork_state() {
    let mut session = init_session(57000);
    let snippet_get_count = format!("(contract-call? '{COUNTER_ADDR} get-count)");

    let result = eval_snippet(&mut session, &snippet_get_count);
    assert_eq!(result, Value::UInt(1));

    session.advance_burn_chain_tip(1);
    let snippet = format!("(contract-call? '{COUNTER_ADDR} increment)");
    let _ = eval_snippet(&mut session, &snippet);

    let result = eval_snippet(&mut session, &snippet_get_count);
    assert_eq!(result, Value::UInt(2));
}

#[test]
fn it_handles_at_block() {
    let mut session = init_session(60000);

    // block 42000 hash
    let hash = "0xb4678e059aa9b82b1473597087876ef61a5c6a0c35910cd4b797201d6b423a07";

    let snippet = format!("(at-block {hash} stacks-block-height)");
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(42000));

    let snippet_get_count_at_10k =
        format!("(contract-call? '{COUNTER_ADDR} get-count-at-block u10000)");
    let result = eval_snippet(&mut session, &snippet_get_count_at_10k);
    assert_eq!(result, Value::okay(Value::none()).unwrap());

    let snippet_get_count_at_42k =
        format!("(contract-call? '{COUNTER_ADDR} get-count-at-block u42000)");
    let result = eval_snippet(&mut session, &snippet_get_count_at_42k);
    assert_eq!(result, Value::okay(Value::UInt(0)).unwrap());

    let snippet_get_count_at_57k =
        format!("(contract-call? '{COUNTER_ADDR} get-count-at-block u57000)");
    let result = eval_snippet(&mut session, &snippet_get_count_at_57k);
    assert_eq!(result, Value::okay(Value::UInt(1)).unwrap());
}

#[test]
fn it_can_get_heights() {
    let mut session = init_session(57000);

    let result = eval_snippet(&mut session, "stacks-block-height");
    assert_eq!(result, Value::UInt(57000));
    let result = eval_snippet(&mut session, "burn-block-height");
    assert_eq!(result, Value::UInt(6603));
    let result = eval_snippet(&mut session, "tenure-height");
    assert_eq!(result, Value::UInt(4251));

    let hash = "0xb4678e059aa9b82b1473597087876ef61a5c6a0c35910cd4b797201d6b423a07";
    let snippet = format!("(at-block {hash} stacks-block-height)");
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(42000));
    let snippet = format!("(at-block {hash} burn-block-height)");
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(6603));
    let snippet = format!("(at-block {hash} tenure-height)");
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(3302));
}

#[test]
fn it_can_fetch_burn_chain_info() {
    let mut session = init_session(800000);

    let result = eval_snippet(&mut session, "burn-block-height");
    assert_eq!(result, Value::UInt(30850));
    let result = eval_snippet(&mut session, "(get-burn-block-info? header-hash u30850)");
    let expected_header_hash =
        hex_bytes("0bf01726f390e2d61d22ac1b8468a33b7802966efbdb7c85861763ca0d9e29b8").unwrap();
    assert_eq!(
        result,
        Value::some(Value::buff_from(expected_header_hash).unwrap()).unwrap()
    );
    let result = eval_snippet(&mut session, "(get-burn-block-info? header-hash u30849)");
    let expected_header_hash =
        hex_bytes("7593042f0e4229276f7d5a71c86b5c7f59db7cef106d29f2542b0ec1461bba15").unwrap();
    assert_eq!(
        result,
        Value::some(Value::buff_from(expected_header_hash).unwrap()).unwrap()
    );

    // test for a bug where a burn block height higher than the current stacks block height would return invalid data
    let mut session = init_session(2000);
    let result = eval_snippet(&mut session, "burn-block-height");
    assert_eq!(result, Value::UInt(2836));
    let result = eval_snippet(&mut session, "(get-burn-block-info? header-hash u2832)");
    let expected_header_hash =
        hex_bytes("088722e90bf5c04639aa91cc30585b068883693a8ecc95a12aab71be2c7252ed").unwrap();
    assert_eq!(
        result,
        Value::some(Value::buff_from(expected_header_hash).unwrap()).unwrap()
    );

    // advance the burn chain tip will result in a fork, bitcoin data is now mocked
    session.advance_burn_chain_tip(10);
    let result = eval_snippet(&mut session, "burn-block-height");
    assert_eq!(result, Value::UInt(2846));
    let result = eval_snippet(&mut session, "(get-burn-block-info? header-hash u2840)");
    let expected_mocked_header_hash =
        hex_bytes("0224cd36a1bb63d40c62a249d8e05153ba4f6411e3024ad569ac28e0b50b41f2").unwrap();
    assert_eq!(
        result,
        Value::some(Value::buff_from(expected_mocked_header_hash).unwrap()).unwrap()
    );
}

#[test]
fn it_keeps_track_of_historical_data() {
    let mut session = init_session(57000);

    let snippet = format!("(contract-call? '{COUNTER_ADDR} get-count-at-block u42000)");
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::okay(Value::UInt(0)).unwrap());

    let snippet = format!("(contract-call? '{COUNTER_ADDR} get-count)");
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(1));
}

#[test]
fn it_can_get_tenure_info_time() {
    let mut session = init_session(57000);
    let result = eval_snippet(&mut session, "(get-tenure-info? time u56999)");
    assert_eq!(result, Value::some(Value::UInt(1737053962)).unwrap());
    let result = eval_snippet(&mut session, "(get-tenure-info? time u50999)");
    assert_eq!(result, Value::some(Value::UInt(1736980481)).unwrap());
}

#[test]
fn it_can_get_tenure_info_bhh() {
    let mut session = init_session(57000);
    let result = eval_snippet(
        &mut session,
        "(get-tenure-info? burnchain-header-hash u56888)",
    );
    let expected_header_hash =
        hex_bytes("026c12afb50b4baabb5cac8b940eda8b437f979b9819eef4cdd14c9f1a78133c").unwrap();
    assert_eq!(
        result,
        Value::some(Value::buff_from(expected_header_hash).unwrap()).unwrap()
    );
}

#[test]
fn it_can_get_tenure_info_vrf_seed() {
    let mut session = init_session(57000);
    let result = eval_snippet(&mut session, "(get-tenure-info? vrf-seed u51897)");
    let expected_vrf_seed =
        hex_bytes("5b2e70683c3d155621ce0f12fc905f571fc8ce487f3ffa0e9c7e75cea10b0990").unwrap();
    assert_eq!(
        result,
        Value::some(Value::buff_from(expected_vrf_seed).unwrap()).unwrap()
    );
    let result = eval_snippet(&mut session, "(get-tenure-info? vrf-seed u51896)");
    let expected_vrf_seed =
        hex_bytes("5b2e70683c3d155621ce0f12fc905f571fc8ce487f3ffa0e9c7e75cea10b0990").unwrap();
    assert_eq!(
        result,
        Value::some(Value::buff_from(expected_vrf_seed).unwrap()).unwrap()
    );
    let result = eval_snippet(&mut session, "(get-tenure-info? vrf-seed u50896)");
    let expected_vrf_seed =
        hex_bytes("33d5f1e379fc9779933f6b8a6d242b520103ceb1d71a75864665d0e036934df3").unwrap();
    assert_eq!(
        result,
        Value::some(Value::buff_from(expected_vrf_seed).unwrap()).unwrap()
    );
}

#[test]
fn it_handles_chain_constants() {
    let mut session = init_session(57000);
    let result = eval_snippet(&mut session, "is-in-mainnet");
    assert_eq!(result, Value::Bool(false));
    let result = eval_snippet(&mut session, "chain-id");
    assert_eq!(result, Value::UInt(2147483648));

    let mut settings = SessionSettings::default();
    settings.repl_settings.remote_data = RemoteDataSettings {
        enabled: true,
        api_url: ApiUrl("https://api.hiro.so".to_string()),
        initial_height: Some(535000),
    };
    let mut session = Session::new(settings);
    let result = eval_snippet(&mut session, "is-in-mainnet");
    assert_eq!(result, Value::Bool(true));
    let result = eval_snippet(&mut session, "chain-id");
    assert_eq!(result, Value::UInt(1));
}

#[test]
fn it_saves_metadata_to_cache() {
    let mut session = init_session(57000);
    let snippet = format!("(contract-call? '{COUNTER_ADDR} get-count)");
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(1));

    let cache_location = session.settings.cache_location.unwrap();
    let cache_file_path = cache_location
        .join(PathBuf::from(
            "datastore/STJCAB2T9TR2EJM7YS4DM2CGBBVTF7BV237Y8KNV_counter_vm-metadata__9__contract_645949d1e1701aea7bd5ca574bf26a7828a26a068ea6409134bc8a9b1329b4fd",
        ))
        .with_extension("json");
    assert!(cache_file_path.exists());
}
