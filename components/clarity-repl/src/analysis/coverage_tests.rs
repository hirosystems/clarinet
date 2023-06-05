use std::convert::TryInto;
use std::fmt::format;

use super::coverage::{CoverageReporter, TestCoverageReport};
use crate::repl::session::{self, Session};
use crate::repl::{
    ClarityCodeSource, ClarityContract, ContractDeployer, SessionSettings, DEFAULT_CLARITY_VERSION,
    DEFAULT_EPOCH,
};

fn get_coverage_report(contract: &str, snippets: Vec<String>) -> (TestCoverageReport, String) {
    let mut session = Session::new(SessionSettings::default());

    let mut report = TestCoverageReport::new("test_scenario".into());
    let _ = session.eval(contract.into(), Some(vec![&mut report]), false);
    for snippet in snippets {
        let _ = session.eval(snippet.into(), Some(vec![&mut report]), false);
    }

    let (contract_id, ast) = session.asts.pop_first().unwrap();
    let coverage_reporter = CoverageReporter::new();

    let mut coverage_reporter = CoverageReporter::new();
    coverage_reporter
        .asts
        .insert(contract_id.clone(), ast.clone());
    coverage_reporter
        .contract_paths
        .insert(contract_id.name.to_string(), "/contract-0.clar".into());
    coverage_reporter.reports.append(&mut vec![report.clone()]);

    let lcov_content = coverage_reporter.build_lcov_file();

    (report, lcov_content)
}

fn get_expected_report(body: String) -> String {
    return format!("TN:test_scenario\nSF:/contract-0.clar\n{body}\nend_of_record\n");
}

#[test]
fn line_is_executed() {
    let contract = "(define-read-only (add) (+ 1 2))";
    let snippet = "(contract-call? .contract-0 add)";
    let (report, cov) = get_coverage_report(contract, vec![snippet.into()]);

    let expect = get_expected_report(
        vec![
            "FN:1,add",
            "FNDA:1,add",
            "FNF:1",
            "FNH:1",
            "DA:1,1",
            "BRF:0",
            "BRH:0",
        ]
        .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn line_is_executed_twice() {
    let contract = "(define-read-only (add) (+ 1 2))";
    // call it twice
    let snippet = "(contract-call? .contract-0 add) (contract-call? .contract-0 add)";
    let (_, cov) = get_coverage_report(contract.into(), vec![snippet.into()]);

    let expect = get_expected_report(
        vec![
            "FN:1,add",
            "FNDA:1,add",
            "FNF:1",
            "FNH:1",
            "DA:1,2",
            "BRF:0",
            "BRH:0",
        ]
        .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn line_count_in_iterator() {
    let contract = vec![
        "(define-private (add-1 (n uint)) (+ n u1))",
        "(define-public (map-add-1)",
        "  (ok (map add-1 (list u2 u3)))",
        ")",
    ]
    .join("\n");
    let snippet = "(contract-call? .contract-0 map-add-1)";
    let (_, cov) = get_coverage_report(contract.as_str(), vec![snippet.into()]);

    let expect = get_expected_report(
        vec![
            "FN:1,add-1",
            "FN:2,map-add-1",
            "FNDA:1,add-1",
            "FNDA:1,map-add-1",
            "FNF:2",
            "FNH:2",
            "DA:1,2",
            "DA:3,1",
            "BRF:0",
            "BRH:0",
        ]
        .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn multiple_line_execution() {
    let contract = vec![
        "(define-read-only (add)",
        "  (begin",
        "    (+ (+ 1 1) (+ 1 2))",
        "    (+ 1 2 3)",
        "  )",
        ")",
    ]
    .join("\n");

    let snippet = "(contract-call? .contract-0 add)";
    let (_, cov) = get_coverage_report(contract.as_str(), vec![snippet.into()]);

    let expect = get_expected_report(
        vec![
            "FN:1,add",
            "FNDA:1,add",
            "FNF:1",
            "FNH:1",
            "DA:2,1",
            "DA:3,1",
            "DA:4,1",
            "BRF:0",
            "BRH:0",
        ]
        .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn let_binding() {
    let contract = vec![
        "(define-public (add-print)",
        "  (let (",
        "    (c (+ 1 1))",
        "  )",
        "    (ok c)",
        "  )",
        ")",
    ]
    .join("\n");

    let snippet = "(contract-call? .contract-0 add-print)";
    let (_, cov) = get_coverage_report(contract.as_str(), vec![snippet.into()]);

    let expect = get_expected_report(
        vec![
            "FN:1,add-print",
            "FNDA:1,add-print",
            "FNF:1",
            "FNH:1",
            "DA:2,1",
            "DA:3,1",
            "DA:5,1",
            "BRF:0",
            "BRH:0",
        ]
        .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn simple_if_branching() {
    let contract = vec![
        "(define-read-only (one-or-two (one bool))",
        "  (if one 1 2)",
        ")",
    ]
    .join("\n");

    let expect_base = vec![
        "FN:1,one-or-two",
        "FNDA:1,one-or-two",
        "FNF:1",
        "FNH:1",
        "DA:2,1",
        "BRF:2",
        "BRH:1",
    ];

    // left path
    let snippet = "(contract-call? .contract-0 one-or-two true)";
    let (_, cov) = get_coverage_report(contract.as_str(), vec![snippet.into()]);

    let expect = get_expected_report(
        [&expect_base[..], &["BRDA:2,8,0,1", "BRDA:2,8,1,0"]]
            .concat()
            .join("\n"),
    );
    assert_eq!(cov, expect);

    // right path
    let snippet = "(contract-call? .contract-0 one-or-two false)";
    let (_, cov) = get_coverage_report(contract.as_str(), vec![snippet.into()]);

    let expect = get_expected_report(
        [&expect_base[..], &["BRDA:2,8,0,0", "BRDA:2,8,1,1"]]
            .concat()
            .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn simple_if_branches_with_exprs() {
    let contract = vec![
        "(define-read-only (add-or-sub (add bool))",
        "  (if add (+ 1 1) (- 1 1))",
        ")",
    ]
    .join("\n");
    let snippet = "(contract-call? .contract-0 add-or-sub true)";
    let (report, cov) = get_coverage_report(contract.as_str(), vec![snippet.into()]);

    let expect = get_expected_report(
        vec![
            "FN:1,add-or-sub",
            "FNDA:1,add-or-sub",
            "FNF:1",
            "FNH:1",
            "DA:2,1",
            "BRF:2",
            "BRH:1",
            "BRDA:2,8,0,1",
            "BRDA:2,8,1,0",
        ]
        .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn hit_all_if_branches() {
    let contract = vec![
        "(define-read-only (add-or-sub (add bool))",
        "  (if add (+ 1 1) (- 1 1))",
        ")",
    ]
    .join("\n");

    // hit left branch 3 times and right branch 2
    let snippets: Vec<String> = vec![
        "(contract-call? .contract-0 add-or-sub true)".into(),
        "(contract-call? .contract-0 add-or-sub true)".into(),
        "(contract-call? .contract-0 add-or-sub true)".into(),
        "(contract-call? .contract-0 add-or-sub false)".into(),
        "(contract-call? .contract-0 add-or-sub false)".into(),
    ];
    let (report, cov) = get_coverage_report(&contract, snippets);

    let expect = get_expected_report(
        vec![
            "FN:1,add-or-sub",
            "FNDA:1,add-or-sub",
            "FNF:1",
            "FNH:1",
            "DA:2,5", // 3 + 2
            "BRF:2",
            "BRH:2",
            "BRDA:2,8,0,3",
            "BRDA:2,8,1,2",
        ]
        .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn simple_asserts_branching() {
    let contract = vec![
        "(define-read-only (is-one (v int))",
        "  (ok (asserts! (is-eq v 1) (err u1)))",
        ")",
    ]
    .join("\n");

    // no hit on (err u1)
    let snippets: Vec<String> = vec!["(contract-call? .contract-0 is-one 1)".into()];
    let (report, cov) = get_coverage_report(&contract, snippets);

    let expect = get_expected_report(
        vec![
            "FN:1,is-one",
            "FNDA:1,is-one",
            "FNF:1",
            "FNH:1",
            "DA:2,1",
            "BRF:1",
            "BRH:0",
            "BRDA:2,10,0,0",
        ]
        .join("\n"),
    );
    assert_eq!(cov, expect);

    // hit on (err u1)
    let snippets: Vec<String> = vec!["(contract-call? .contract-0 is-one 2)".into()];
    let (report, cov) = get_coverage_report(&contract, snippets);

    let expect = get_expected_report(
        vec![
            "FN:1,is-one",
            "FNDA:1,is-one",
            "FNF:1",
            "FNH:1",
            "DA:2,1",
            "BRF:1",
            "BRH:1",
            "BRDA:2,10,0,1",
        ]
        .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn branch_if_plus_and() {
    let contract = vec![
        "(define-read-only (unecessary-ifs (v int))",
        "  (if (and (> v 0) (> v 1) (> v 2) (> v 3))",
        "    (ok \"greater\")",
        "    (ok \"lower\")",
        "  )",
        ")",
    ]
    .join("\n");
    // calling with `2`, so that evualuation should stop at (> v 2) (which is false)
    let snippet = "(contract-call? .contract-0 unecessary-ifs 2)";
    let (report, cov) = get_coverage_report(contract.as_str(), vec![snippet.into()]);

    let expect = get_expected_report(
        vec![
            "FN:1,unecessary-ifs",
            "FNDA:1,unecessary-ifs",
            "FNF:1",
            "FNH:1",
            "DA:2,1",
            "DA:3,0", // left if path
            "DA:4,1", // right if path
            "BRF:6",
            "BRH:4",
            "BRDA:2,10,0,1",
            "BRDA:2,10,1,1",
            "BRDA:2,10,2,1",
            "BRDA:2,10,3,0", // (> v 3) not hit
            "BRDA:3,8,0,0",  // left if path not hit
            "BRDA:4,8,1,1",
        ]
        .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn branch_if_plus_or() {
    let contract = vec![
        "(define-read-only (unecessary-ors (v int))",
        "  (if (or (is-eq v 0) (is-eq v 1) (is-eq v 2) (is-eq v 3))",
        "    (ok \"match\")",
        "    (ok \"no match\")",
        "  )",
        ")",
    ]
    .join("\n");
    // calling with 1, so that evualuation should stop at (is-eq v 1)
    let snippet = "(contract-call? .contract-0 unecessary-ors 1)";
    let (report, cov) = get_coverage_report(contract.as_str(), vec![snippet.into()]);

    let expect = get_expected_report(
        vec![
            "FN:1,unecessary-ors",
            "FNDA:1,unecessary-ors",
            "FNF:1",
            "FNH:1",
            "DA:2,1",
            "DA:3,1", // left if path
            "DA:4,0", // right if path
            "BRF:6",
            "BRH:3",
            "BRDA:2,10,0,1",
            "BRDA:2,10,1,1", // stop
            "BRDA:2,10,2,0",
            "BRDA:2,10,3,0",
            "BRDA:3,8,0,1",
            "BRDA:4,8,1,0",
        ]
        .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn match_opt_oneline() {
    let contract = vec![
        "(define-public (match-opt (opt? (optional int)))",
        "  (match opt? opt (ok opt) (err u1))",
        ")",
    ]
    .join("\n");

    let expect_base = [
        "FN:1,match-opt",
        "FNDA:1,match-opt",
        "FNF:1",
        "FNH:1",
        "DA:2,1",
        "BRF:2",
        "BRH:1",
    ];

    // left path
    let snippets: Vec<String> = vec!["(contract-call? .contract-0 match-opt (some 1))".into()];
    let (report, cov) = get_coverage_report(&contract, snippets);

    let expect = get_expected_report(
        vec![&expect_base[..], &["BRDA:2,10,0,1", "BRDA:2,10,1,0"]]
            .concat()
            .join("\n"),
    );
    assert_eq!(cov, expect);

    // right path
    let snippets: Vec<String> = vec!["(contract-call? .contract-0 match-opt none)".into()];
    let (report, cov) = get_coverage_report(&contract, snippets);

    let expect = get_expected_report(
        vec![&expect_base[..], &["BRDA:2,10,0,0", "BRDA:2,10,1,1"]]
            .concat()
            .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn match_opt_multiline() {
    let contract = vec![
        "(define-public (match-opt (opt? (optional int)))",
        "  (match opt?",
        "    opt",
        "    (ok opt)",
        "    (err u1)",
        "  )",
        ")",
    ]
    .join("\n");

    let expect_base = [
        "FN:1,match-opt",
        "FNDA:1,match-opt",
        "FNF:1",
        "FNH:1",
        "DA:2,1",
    ];

    // left path
    let snippets: Vec<String> = vec!["(contract-call? .contract-0 match-opt (some 1))".into()];
    let (report, cov) = get_coverage_report(&contract, snippets);

    let expect = get_expected_report(
        vec![
            &expect_base[..],
            &[
                "DA:4,1",
                "DA:5,0",
                "BRF:2",
                "BRH:1",
                "BRDA:4,10,0,1",
                "BRDA:5,10,1,0",
            ],
        ]
        .concat()
        .join("\n"),
    );
    assert_eq!(cov, expect);

    // right path
    let snippets: Vec<String> = vec!["(contract-call? .contract-0 match-opt none)".into()];
    let (report, cov) = get_coverage_report(&contract, snippets);

    let expect = get_expected_report(
        vec![
            &expect_base[..],
            &[
                "DA:4,0",
                "DA:5,1",
                "BRF:2",
                "BRH:1",
                "BRDA:4,10,0,0",
                "BRDA:5,10,1,1",
            ],
        ]
        .concat()
        .join("\n"),
    );
    assert_eq!(cov, expect);
}

#[test]
fn match_res_oneline() {
    // very similar to match opt
    // lighter test strategy, only test one liner and call both paths in same session
    let contract = vec![
        "(define-public (match-res (res (response int uint)))",
        "  (match res o (ok o) e (err e))",
        ")",
    ]
    .join("\n");

    // call left path twice and right path once
    let snippets: Vec<String> = vec![
        "(contract-call? .contract-0 match-res (ok 1))".into(),
        "(contract-call? .contract-0 match-res (ok 2))".into(),
        "(contract-call? .contract-0 match-res (err u1))".into(),
    ];
    let (report, cov) = get_coverage_report(&contract, snippets);

    let expect = get_expected_report(
        vec![
            "FN:1,match-res",
            "FNDA:1,match-res",
            "FNF:1",
            "FNH:1",
            "DA:2,3",
            "BRF:2",
            "BRH:2",
            "BRDA:2,11,0,2",
            "BRDA:2,11,1,1",
        ]
        .join("\n"),
    );

    assert_eq!(cov, expect);
}
