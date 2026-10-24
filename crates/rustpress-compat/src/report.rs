//! Results of a run and their Markdown report.

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::compare::Difference;
use crate::known::Known;
use crate::urls::{Kind, Target};

/// A difference together with the documented deviation that covers it, if any.
#[derive(Debug, Clone)]
pub struct Finding {
    pub difference: Difference,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub target: Target,
    pub ours_ms: u128,
    pub theirs_ms: u128,
    pub findings: Vec<Finding>,
}

impl Outcome {
    pub fn is_identical(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn is_unexpected(&self) -> bool {
        self.findings.iter().any(|f| f.reason.is_none())
    }
}

#[derive(Debug, Clone)]
pub struct Report {
    pub label: String,
    pub wp_version: String,
    pub generated: String,
    pub outcomes: Vec<Outcome>,
}

/// Attaches the documented deviations to the raw differences.
pub fn classify(differences: Vec<Difference>, path: &str, theme: &str, known: &Known) -> Vec<Finding> {
    differences
        .into_iter()
        .map(|difference| {
            let reason = known.matching(path, &difference.aspect, theme).map(|d| d.reason.clone());
            Finding { difference, reason }
        })
        .collect()
}

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values.get(values.len() / 2).copied().unwrap_or(0)
}

impl Report {
    pub fn unexpected(&self) -> impl Iterator<Item = &Outcome> {
        self.outcomes.iter().filter(|o| o.is_unexpected())
    }

    pub fn unexpected_count(&self) -> usize {
        self.unexpected().count()
    }

    /// The Markdown report.
    pub fn to_markdown(&self, known: &Known) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Kompatibilitätsbericht: {}\n", self.label);
        let _ = writeln!(out, "Stand: {} · Referenz: WordPress {} · Theme: `{}`\n", self.generated, self.wp_version, self.label);
        let total = self.outcomes.len();
        let identical = self.outcomes.iter().filter(|o| o.is_identical()).count();
        let unexpected = self.unexpected_count();
        let _ = writeln!(out, "**{total} URLs verglichen: {identical} identisch, {} mit bekannten Abweichungen, {unexpected} mit unerwarteten Abweichungen.**\n", total - identical - unexpected);

        let _ = writeln!(out, "| Art | URLs | identisch | bekannt | unerwartet | Median RustPress | Median WordPress |");
        let _ = writeln!(out, "|---|---:|---:|---:|---:|---:|---:|");
        for kind in Kind::ALL {
            let of_kind: Vec<&Outcome> = self.outcomes.iter().filter(|o| o.target.kind == kind).collect();
            if of_kind.is_empty() {
                continue;
            }
            let same = of_kind.iter().filter(|o| o.is_identical()).count();
            let bad = of_kind.iter().filter(|o| o.is_unexpected()).count();
            let ours = median(of_kind.iter().map(|o| o.ours_ms).collect());
            let theirs = median(of_kind.iter().map(|o| o.theirs_ms).collect());
            let _ = writeln!(out, "| {} | {} | {same} | {} | {bad} | {ours} ms | {theirs} ms |", kind.label(), of_kind.len(), of_kind.len() - same - bad);
        }

        let _ = writeln!(out, "\n## Unerwartete Abweichungen\n");
        if unexpected == 0 {
            let _ = writeln!(out, "Keine.\n");
        }
        for outcome in self.unexpected() {
            let _ = writeln!(out, "### `{}` ({})\n", outcome.target.path, outcome.target.kind.label());
            for finding in outcome.findings.iter().filter(|f| f.reason.is_none()) {
                if finding.difference.detail.contains('\n') {
                    let _ = writeln!(out, "- **{}**\n\n```text\n{}\n```\n", finding.difference.aspect, finding.difference.detail);
                } else {
                    let _ = writeln!(out, "- **{}**: {}", finding.difference.aspect, finding.difference.detail);
                }
            }
            let _ = writeln!(out);
        }

        // Known deviations that were actually seen, grouped by reason.
        let mut seen: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for outcome in &self.outcomes {
            for finding in &outcome.findings {
                if let Some(reason) = &finding.reason {
                    let list = seen.entry(reason.as_str()).or_default();
                    let entry = format!("`{}` ({})", outcome.target.path, finding.difference.aspect);
                    if !list.contains(&entry) {
                        list.push(entry);
                    }
                }
            }
        }
        let _ = writeln!(out, "## Bekannte Abweichungen\n");
        if seen.is_empty() {
            let _ = writeln!(out, "Keine aufgetreten.\n");
        }
        for (reason, urls) in &seen {
            let shown: Vec<&String> = urls.iter().take(12).collect();
            let more = if urls.len() > shown.len() { format!(" … und {} weitere", urls.len() - shown.len()) } else { String::new() };
            let _ = writeln!(out, "- **{reason}**: {}{more}", shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
        }
        let unused: Vec<_> = known.entries().filter(|d| !seen.contains_key(d.reason.as_str())).collect();
        if !unused.is_empty() {
            let _ = writeln!(out, "\nEingetragen, aber in diesem Lauf nicht aufgetreten:\n");
            for d in unused {
                let _ = writeln!(out, "- `{}` ({}): {}", d.path, d.aspect, d.reason);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(path: &str, kind: Kind, findings: Vec<Finding>) -> Outcome {
        Outcome { target: Target { path: path.into(), kind }, ours_ms: 10, theirs_ms: 40, findings }
    }

    fn finding(aspect: &str, reason: Option<&str>) -> Finding {
        Finding { difference: Difference { aspect: aspect.into(), detail: "x".into() }, reason: reason.map(str::to_owned) }
    }

    #[test]
    fn outcomes_are_identical_known_or_unexpected() {
        assert!(outcome("/", Kind::Page, vec![]).is_identical());
        let known = outcome("/", Kind::Page, vec![finding("status", Some("why"))]);
        assert!(!known.is_identical() && !known.is_unexpected());
        let bad = outcome("/", Kind::Page, vec![finding("status", Some("why")), finding("body", None)]);
        assert!(bad.is_unexpected());
    }

    #[test]
    fn the_markdown_lists_unexpected_and_known_differences() {
        let report = Report {
            label: "twentytwentyfive".into(),
            wp_version: "7.1.3".into(),
            generated: "2026-10-24".into(),
            outcomes: vec![
                outcome("/", Kind::Page, vec![]),
                outcome("/login", Kind::Other, vec![finding("status", Some("Anmeldung fehlt"))]),
                outcome("/kaputt/", Kind::Page, vec![Finding { difference: Difference { aspect: "body".into(), detail: "a\nb".into() }, reason: None }]),
            ],
        };
        let md = report.to_markdown(&Known::default());
        assert!(md.contains("3 URLs verglichen: 1 identisch, 1 mit bekannten Abweichungen, 1 mit unerwarteten Abweichungen"), "{md}");
        assert!(md.contains("### `/kaputt/`"));
        assert!(md.contains("**Anmeldung fehlt**: `/login` (status)"));
        assert_eq!(report.unexpected_count(), 1);
    }
}
