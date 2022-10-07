use clarity_repl::clarity::codec::StacksString;
use clarity_repl::clarity::util::hash::{hex_bytes, to_hex};
use clarity_repl::clarity::ClarityName;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hex::{decode, encode};

#[inline]
fn canonical_is_clarity_variable() {
    let function_name = ClarityName::try_from("my-method-name").unwrap();
    StacksString::from(function_name.clone()).is_clarity_variable();
}

#[inline]
fn proposed_is_clarity_variable() {
    let function_name = ClarityName::try_from("my-method-name").unwrap();
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("canonical_is_clarity_variable <my-method-name>", |b| {
        b.iter(|| canonical_is_clarity_variable())
    });
    c.bench_function("proposed_is_clarity_variable <my-method-name>", |b| {
        b.iter(|| proposed_is_clarity_variable())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
