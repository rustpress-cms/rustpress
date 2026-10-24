//! Round-trips output of the real `serialize()` through this crate.
//!
//! Needs a `php` binary on PATH (or `PHP_BIN`); without one the test prints a
//! note and passes, so the suite stays runnable on machines without PHP.

use std::process::Command;

use rustpress_phpser::{serialize, unserialize};

const CASES: &[&str] = &[
    "null",
    "true",
    "false",
    "0",
    "-9223372036854775807 - 1",
    "PHP_INT_MAX",
    "0.1",
    "-0.0",
    "1e25",
    "1.5e-7",
    "123456789012345.0",
    "1e15",
    "1e16",
    "1e17",
    "9.5e16",
    "123456789012345678.0",
    "12345678901234567890.0",
    "0.0001",
    "0.00001",
    "0.00015",
    "''",
    "'äöü \"quoted\"; s:1:\"x\";'",
    "\"line\\r\\n\\0nul\"",
    "[]",
    "[1, 2, 3]",
    "['a' => ['b' => ['c' => null]], 5 => 'five', -3 => 'neg']",
    // WordPress-shaped data
    "['active_plugins' => ['akismet/akismet.php', 'hello.php'], 'widget_text' => [2 => ['title' => 'T', 'text' => \"a\\nb\"], '_multiwidget' => 1]]",
    "(function () { $o = new stdClass; $o->a = 1; $o->b = [true]; return $o; })()",
    "(function () { $o = new stdClass; return [$o, $o]; })()",
    "(function () { $a = 1; return [&$a, &$a]; })()",
    "(function () { $a = ['x' => 1]; $b = [&$a, 'k' => &$a]; return $b; })()",
    "(function () { $o = new ArrayObject([1]); return [$o, [$o]]; })()",
];

fn php() -> Option<String> {
    let bin = std::env::var("PHP_BIN").unwrap_or_else(|_| "php".into());
    Command::new(&bin).arg("-v").output().ok().filter(|o| o.status.success()).map(|_| bin)
}

fn php_serialize(bin: &str, expr: &str) -> Vec<u8> {
    let code = format!("fwrite(STDOUT, serialize({expr}));");
    let out = Command::new(bin).args(["-d", "serialize_precision=-1", "-r", &code]).output().unwrap();
    assert!(out.status.success(), "php failed for {expr}: {}", String::from_utf8_lossy(&out.stderr));
    out.stdout
}

#[test]
fn php_output_round_trips_byte_exact() {
    let Some(bin) = php() else {
        eprintln!("php not found, skipping compatibility test");
        return;
    };
    for expr in CASES {
        let expected = php_serialize(&bin, expr);
        match unserialize(&expected) {
            Ok(value) => assert_eq!(
                serialize(&value),
                expected,
                "round trip differs for `{expr}`: {}",
                String::from_utf8_lossy(&expected)
            ),
            Err(e) => panic!("cannot parse PHP output for `{expr}`: {e}\n{}", String::from_utf8_lossy(&expected)),
        }
    }
}
