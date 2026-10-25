//! `wp_text_diff()` against the reference WordPress: `$TEMP/diff_cases.json` holds `[left, right]` pairs and
//! `$TEMP/diff_ref.json` the results of `wp_text_diff()` for them (see `dev/text-diff-check.py`). Skipped without them.

use rustpress_core::text_diff::{DiffArgs, wp_text_diff};

#[test]
fn matches_the_reference() {
    let Ok(temp) = std::env::var("TEMP") else {
        eprintln!("no TEMP");
        return;
    };
    let (Ok(cases), Ok(expected)) = (std::fs::read_to_string(format!("{temp}/diff_cases.json")), std::fs::read_to_string(format!("{temp}/diff_ref.json"))) else {
        eprintln!("no files in {temp}");
        return;
    };
    let cases: Vec<(String, String)> = serde_json::from_str(&cases).expect("cases");
    let expected: Vec<serde_json::Value> = serde_json::from_str(&expected).expect("expected");
    assert_eq!(cases.len(), expected.len());
    let args = DiffArgs { title_left: "Removed".into(), title_right: "Added".into(), ..Default::default() };
    let mut failures = 0;
    for (i, (left, right)) in cases.iter().enumerate() {
        // `wp_text_diff()` answers `false`-ish with an empty string; the JSON of PHP has "" for it.
        let want = expected[i].as_str().unwrap_or_default();
        let got = wp_text_diff(left, right, &args);
        if got != want {
            failures += 1;
            eprintln!("case {i}: {left:?} -> {right:?}\n--- want\n{want}\n--- got\n{got}\n");
        }
    }
    eprintln!("checked {} cases", cases.len());
    assert_eq!(failures, 0, "{failures} of {} cases differ", cases.len());
}
