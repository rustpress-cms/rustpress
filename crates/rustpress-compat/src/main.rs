//! `rustpress-compat [--reference HOST:PORT] [--host HOST:PORT] [--db URL] [--wp-root DIR] [--report FILE]
//! [--known FILE] [--limit N] [--concurrency N] [--only REGEX] [--dump PATH]`
//!
//! Compares RustPress with the reference WordPress and writes a Markdown report. The exit
//! code is 1 if there are differences that no entry of the known-deviations file covers.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use rustpress_compat::{Known, Options, run};

fn usage() -> ExitCode {
    eprintln!("usage: rustpress-compat [--reference HOST:PORT] [--host HOST:PORT] [--db URL] [--wp-root DIR] [--report FILE] [--known FILE] [--limit N] [--concurrency N] [--only REGEX] [--dump PATH]");
    ExitCode::from(2)
}

#[tokio::main]
async fn main() -> ExitCode {
    let mut options = Options::default();
    let mut report_path: Option<PathBuf> = None;
    let mut dump: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let Some(value) = args.next() else { return usage() };
        match flag.as_str() {
            "--reference" => options.reference = value,
            "--host" => options.host = value,
            "--db" => options.database_url = value,
            "--wp-root" => options.wp_root = PathBuf::from(value),
            "--report" => report_path = Some(PathBuf::from(value)),
            "--known" => options.known = PathBuf::from(value),
            "--limit" => match value.parse() {
                Ok(n) => options.limit = n,
                Err(_) => return usage(),
            },
            "--concurrency" => match value.parse() {
                Ok(n) => options.concurrency = n,
                Err(_) => return usage(),
            },
            "--only" => options.only = Some(value),
            "--dump" => dump = Some(value),
            _ => return usage(),
        }
    }

    if let Some(path) = dump {
        return match rustpress_compat::fetch_both(&options, &path).await {
            Ok((ours, theirs)) => {
                let dir = PathBuf::from("compat/dump");
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(dir.join("ours.txt"), &ours.body);
                let _ = std::fs::write(dir.join("theirs.txt"), &theirs.body);
                eprintln!("{path}: RustPress {} ({} bytes), WordPress {} ({} bytes) -> {}", ours.status, ours.body.len(), theirs.status, theirs.body.len(), dir.display());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(2)
            }
        };
    }

    let report = match run(&options, |done, total, path| {
        eprint!("\r[{done}/{total}] {path:<70.70}");
        let _ = std::io::stderr().flush();
    })
    .await
    {
        Ok(report) => report,
        Err(e) => {
            eprintln!("\n{e}");
            return ExitCode::from(2);
        }
    };
    eprintln!();

    let known = Known::load(&options.known).unwrap_or_default();
    let markdown = report.to_markdown(&known);
    match &report_path {
        Some(path) => {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(path, &markdown) {
                eprintln!("cannot write {}: {e}", path.display());
                return ExitCode::from(2);
            }
            eprintln!("report: {}", path.display());
        }
        None => println!("{markdown}"),
    }
    let unexpected = report.unexpected_count();
    let identical = report.outcomes.iter().filter(|o| o.is_identical()).count();
    eprintln!("{} URLs, {identical} identical, {unexpected} with unexpected differences", report.outcomes.len());
    if unexpected == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
