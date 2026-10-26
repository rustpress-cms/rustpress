//! The filters of a visitor's comment (`pre_comment_content`) against the answers of WordPress 7.1
//! (`dev/kses-golden.py` makes the cases).

use rustpress_core::kses::comment_content_filter;

#[test]
fn comments_are_filtered_like_wordpress() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/kses_cases.json")).expect("the golden file");
    let cases: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let mut failures = Vec::new();
    for case in cases.as_array().expect("a list") {
        let input = case["in"].as_str().expect("input");
        let expected = case["out"].as_str().expect("output");
        let got = comment_content_filter(input, "http://localhost:8080", true);
        if got != expected {
            failures.push(format!("input:    {input:?}\nexpected: {expected:?}\ngot:      {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{} of {} differ:\n{}", failures.len(), cases.as_array().map_or(0, Vec::len), failures.join("\n\n"));
}

#[test]
fn post_content_is_filtered_like_wordpress() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/kses_post_cases.json")).expect("the golden file");
    let cases: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let mut failures = Vec::new();
    for case in cases.as_array().expect("a list") {
        let input = case["in"].as_str().expect("input");
        let expected = case["out"].as_str().expect("output");
        let got = rustpress_core::kses::kses_post(input);
        if got != expected {
            failures.push(format!("input:    {input:?}\nexpected: {expected:?}\ngot:      {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{} of {} differ:\n{}", failures.len(), cases.as_array().map_or(0, Vec::len), failures.join("\n\n"));
}
