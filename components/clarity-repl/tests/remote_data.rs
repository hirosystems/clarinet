use clarity::{
    types::StacksEpochId,
    vm::{EvaluationResult, Value},
};
use clarity_repl::repl::{
    settings::{ApiUrl, RemoteDataSettings},
    Session, SessionSettings,
};

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

    settings.repl_settings.remote_data = RemoteDataSettings {
        enabled: true,
        api_url: ApiUrl("https://api.testnet.hiro.so".to_string()),
        initial_height: Some(initial_heigth),
    };

    let mut session = Session::new(settings);
    session.update_epoch(StacksEpochId::Epoch30);
    session
}

// the counter contract is delpoyed on testnet at height #41613
// the initial count value is 0 and is incremented by 1 at #56232

const COUNTER_ADDR: &str = "STJCAB2T9TR2EJM7YS4DM2CGBBVTF7BV237Y8KNV.counter";

#[ignore]
#[test]
fn it_handles_not_found_contract() {
    let mut session = init_session(40000);

    let snippet = format!("(contract-call? '{} get-count)", COUNTER_ADDR);
    let result = eval_snippet(&mut session, &snippet);
    println!("result: {:?}", result);
}

#[test]
fn it_can_fetch_remote() {
    // count at block 42000 is 0
    let mut session = init_session(42000);
    let snippet = format!("(contract-call? '{} get-count)", COUNTER_ADDR);
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(0));

    // count at block 57000 is 1
    let mut session = init_session(57000);
    let snippet = format!("(contract-call? '{} get-count)", COUNTER_ADDR);
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(1));
}

#[test]
fn it_can_fork_state() {
    let mut session = init_session(57000);
    let snippet_get_count = format!("(contract-call? '{} get-count)", COUNTER_ADDR);
    let result = eval_snippet(&mut session, &snippet_get_count);
    assert_eq!(result, Value::UInt(1));

    let snippet = format!("(contract-call? '{} increment)", COUNTER_ADDR);
    let _ = eval_snippet(&mut session, &snippet);
    session.advance_burn_chain_tip(1);

    let result = eval_snippet(&mut session, &snippet_get_count);
    assert_eq!(result, Value::UInt(2));
}

#[test]
fn it_handles_at_block() {
    let mut session = init_session(60000);

    // block 42000 hash
    let id_header_hash = "0xb4678e059aa9b82b1473597087876ef61a5c6a0c35910cd4b797201d6b423a07";

    let snippet = format!("(at-block {} stacks-block-height)", id_header_hash);
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(42000));

    let snippet_get_count_at_10k = format!(
        "(contract-call? '{} get-count-at-block u10000)",
        COUNTER_ADDR
    );
    let result = eval_snippet(&mut session, &snippet_get_count_at_10k);
    assert_eq!(result, Value::okay(Value::none()).unwrap());

    let snippet_get_count_at_42k = format!(
        "(contract-call? '{} get-count-at-block u42000)",
        COUNTER_ADDR
    );
    let result = eval_snippet(&mut session, &snippet_get_count_at_42k);
    assert_eq!(result, Value::okay(Value::UInt(0)).unwrap());

    let snippet_get_count_at_57k = format!(
        "(contract-call? '{} get-count-at-block u57000)",
        COUNTER_ADDR
    );
    let result = eval_snippet(&mut session, &snippet_get_count_at_57k);
    assert_eq!(result, Value::okay(Value::UInt(1)).unwrap());
}

#[test]
fn correctly_keeps_track_of_historical_data() {
    let mut session = init_session(57000);

    let snippet = format!(
        "(contract-call? '{} get-count-at-block u42000)",
        COUNTER_ADDR
    );
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::okay(Value::UInt(0)).unwrap());

    let snippet = format!("(contract-call? '{} get-count)", COUNTER_ADDR);
    let result = eval_snippet(&mut session, &snippet);
    assert_eq!(result, Value::UInt(1));
}
