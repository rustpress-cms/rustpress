//! `make_clickable()` against the output of WordPress for the inputs in `data/make_clickable.json` (made with
//! `make_clickable()` of the reference outside a filter, so without `ugc`).

use rustpress_core::text::{Clickable, make_clickable};

#[test]
fn matches_wordpress() {
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!("data/make_clickable.json")).expect("golden data");
    let opts = Clickable { home_host: "localhost", comment: false };
    let mut failed = Vec::new();
    for case in &cases {
        let (input, expected) = (case["input"].as_str().unwrap_or_default(), case["output"].as_str().unwrap_or_default());
        let got = make_clickable(input, &opts);
        if got != expected {
            failed.push(format!("input:    {input:?}\nexpected: {expected:?}\ngot:      {got:?}"));
        }
    }
    assert!(failed.is_empty(), "{} of {} differ:\n{}", failed.len(), cases.len(), failed.join("\n\n"));
}
