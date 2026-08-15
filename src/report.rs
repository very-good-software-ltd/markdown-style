//! Human-readable, rustc-style rendering of violations.

use std::collections::{HashMap, HashSet};

use crate::rule::{Rule, RuleKind};
use crate::text::split_lines;
use crate::violation::Violation;

const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// Render violations for one file as a rustc-style report.
///
/// Each violation shows the source line with a caret underline and a one-line
/// `why:`. The fuller rationale is shown once per rule per report, on that
/// rule's first violation.
pub fn render(
    path: &str,
    source: &str,
    violations: &[Violation],
    rules: &[Box<dyn Rule>],
    color: bool,
) -> String {
    let reasons = reason_index(rules);
    let lines = split_lines(source);

    let mut ordered: Vec<&Violation> = violations.iter().collect();
    ordered.sort_by_key(|violation| (violation.span.line, violation.span.column));

    let mut out = String::new();
    let mut explained: HashSet<&str> = HashSet::new();
    for (position, violation) in ordered.iter().enumerate() {
        if position > 0 {
            out.push('\n');
        }
        render_one(
            &mut out,
            path,
            &lines,
            violation,
            reasons.get(violation.rule_id).copied().unwrap_or(""),
            explained.insert(violation.rule_id),
            color,
        );
    }
    out
}

/// Render violations as GitHub Actions workflow commands, one `::error` per
/// violation, so a CI run annotates the pull request diff inline. The rule id is
/// the annotation title and its message is the body.
pub fn github(path: &str, violations: &[Violation]) -> String {
    let mut ordered: Vec<&Violation> = violations.iter().collect();
    ordered.sort_by_key(|violation| (violation.span.line, violation.span.column));

    // GitHub matches the annotation against a repository path, which always
    // uses forward slashes, so a Windows runner's backslashes have to be
    // translated or the annotation silently fails to attach to the diff. This
    // is unconditional rather than Windows-only so the behaviour is covered by
    // the tests everywhere; the cost is mistranslating a path on a system that
    // allows a literal backslash in a filename, which at worst leaves that one
    // annotation unattached.
    let path = path.replace('\\', "/");

    let mut out = String::new();
    for violation in ordered {
        let span = &violation.span;
        out.push_str(&format!(
            "::error file={},line={},col={},title={}::{}\n",
            escape_property(&path),
            span.line,
            span.column,
            escape_property(violation.rule_id),
            escape_data(&violation.message),
        ));
    }
    out
}

/// Escape a workflow-command message. Percent must be escaped first, so its
/// replacement's own `%` is not escaped again.
fn escape_data(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escape a workflow-command property value, which additionally may not contain
/// a literal `:` or `,`.
fn escape_property(value: &str) -> String {
    escape_data(value).replace(':', "%3A").replace(',', "%2C")
}

/// The full rationale for one rule, for the `explain` command. `None` when no
/// rule owns the id.
pub fn explain(rule_id: &str, rules: &[Box<dyn Rule>]) -> Option<String> {
    rules
        .iter()
        .find(|rule| rule.id() == rule_id)
        .map(|rule| format!("{rule_id}\n\n{}\n", rule.rationale()))
}

/// The full rule catalog as Markdown, generated from the registry so it cannot
/// drift from the code. The result is run through the formatter, so it is always
/// canonical and passes the tool's own rules. A test asserts `docs/rules.md`
/// matches this, and it is regenerated with `markdown-style rules --markdown`.
pub fn rules_catalog() -> String {
    let rules = crate::default_rules();
    let mut raw = String::new();
    raw.push_str("# Rules\n\n");
    raw.push_str(
        "Every rule is always on. \
         A rule either _fixes_ what it finds or only _flags_ it. \
         A rule is flag-only when fixing it would mean guessing at your intent, \
         so the tool reports it and leaves the change to you. \
         This page is generated from the rules themselves, and the reasoning is \
         the same text `markdown-style explain <rule>` prints.\n\n",
    );

    for rule in &rules {
        raw.push_str(&format!("## {}\n\n", rule.id()));
        raw.push_str(&format!(
            "_{}._ {}\n\n",
            doc_kind(rule.kind()),
            rule.short_reason()
        ));
        raw.push_str(rule.rationale());
        raw.push_str("\n\n");
    }

    crate::format(raw.trim_end(), &rules)
}

fn doc_kind(kind: RuleKind) -> &'static str {
    match kind {
        RuleKind::Fix => "Fix",
        RuleKind::Flag => "Flag",
        RuleKind::Both => "Fix and flag",
    }
}

fn reason_index(rules: &[Box<dyn Rule>]) -> HashMap<&str, &str> {
    rules
        .iter()
        .map(|rule| (rule.id(), rule.rationale()))
        .collect()
}

fn render_one(
    out: &mut String,
    path: &str,
    lines: &[crate::text::Line<'_>],
    violation: &Violation,
    rationale: &str,
    show_rationale: bool,
    color: bool,
) {
    let span = &violation.span;
    let gutter = " ".repeat(span.line.to_string().len());

    let heading = format!("{}: {}", violation.rule_id, violation.message);
    out.push_str(&paint(&heading, &format!("{BOLD}{RED}"), color));
    out.push('\n');
    out.push_str(&format!(
        "{gutter}--> {path}:{}:{}\n",
        span.line, span.column
    ));
    out.push_str(&format!("{gutter} |\n"));

    let source_line = lines.get(span.line - 1).map_or("", |line| line.content);
    let (shown, caret_offset, caret_len) = window(source_line, span.column, span.length);
    out.push_str(&format!("{} | {shown}\n", span.line));

    let caret_pad = " ".repeat(caret_offset);
    let carets = "^".repeat(caret_len);
    out.push_str(&format!(
        "{gutter} | {caret_pad}{}\n",
        paint(&carets, RED, color)
    ));

    if show_rationale && !rationale.is_empty() {
        for (index, line) in rationale.lines().enumerate() {
            let label = if index == 0 { " = why: " } else { "        " };
            out.push_str(&format!("{gutter}{label}{}\n", line.trim()));
        }
    }
}

/// The most source we show on one line, so a long line and its caret stay
/// aligned instead of wrapping in the terminal.
const MAX_WIDTH: usize = 100;

/// Return the source excerpt to show, the caret's offset within it, and the
/// caret length. Short lines are shown whole; long lines are windowed around the
/// span with `...` marking each trimmed end.
fn window(line: &str, column: usize, length: usize) -> (String, usize, usize) {
    let chars: Vec<char> = line.chars().collect();
    let span_start = column.saturating_sub(1);
    if chars.len() <= MAX_WIDTH {
        return (line.to_string(), span_start, length.max(1));
    }

    let end = (span_start + MAX_WIDTH - 24).min(chars.len());
    let start = end.saturating_sub(MAX_WIDTH);
    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < chars.len() { "..." } else { "" };

    let mut shown = String::from(prefix);
    shown.extend(&chars[start..end]);
    shown.push_str(suffix);

    let caret_offset = prefix.len() + (span_start - start);
    let caret_len = length.max(1).min(end - span_start);
    (shown, caret_offset, caret_len)
}

fn paint(text: &str, code: &str, color: bool) -> String {
    if color {
        format!("{code}{text}{RESET}")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;
    use crate::default_rules;

    fn detect(source: &str) -> Vec<Violation> {
        crate::lint(&Document::new(source), &default_rules())
    }

    #[test]
    fn renders_a_caret_snippet_with_reason() {
        let source = "# Title\n\ntext   \n";
        let report = render("a.md", source, &detect(source), &default_rules(), false);
        // The source line is shown verbatim (with its trailing spaces), and the
        // carets sit under them starting at the flagged column. `line3` keeps the
        // trailing spaces off the end of a physical source line here.
        let line3 = "text   ";
        let expected = format!(
            "trailing-whitespace: trailing whitespace\n --> a.md:3:5\n  |\n3 | {line3}\n  |     ^^^\n  = why: Trailing spaces"
        );
        assert!(report.starts_with(&expected), "got:\n{report}");
    }

    #[test]
    fn shows_the_why_only_once_per_rule() {
        let source = "text  \n\nmore  \n";
        let report = render("a.md", source, &detect(source), &default_rules(), false);
        // Two violations of the same rule, but the reasoning is shown once.
        assert_eq!(report.matches("= why:").count(), 1);
        assert!(!report.contains("= help:"));
    }

    #[test]
    fn adds_colour_only_when_asked() {
        let source = "text   \n";
        let plain = render("a.md", source, &detect(source), &default_rules(), false);
        let coloured = render("a.md", source, &detect(source), &default_rules(), true);
        assert!(!plain.contains('\x1b'));
        assert!(coloured.contains('\x1b'));
    }

    #[test]
    fn windows_a_long_line_and_keeps_the_caret_aligned() {
        use crate::violation::Span;
        let long = format!("{}*x*{}", "a".repeat(200), "b".repeat(200));
        let violation = Violation {
            rule_id: "emphasis",
            message: "m".to_string(),
            span: Span {
                line: 1,
                column: 201,
                length: 1,
            },
        };
        let report = render(
            "f.md",
            &long,
            std::slice::from_ref(&violation),
            &default_rules(),
            false,
        );
        let lines: Vec<&str> = report.lines().collect();
        let source_index = lines.iter().position(|l| l.starts_with("1 | ")).unwrap();
        let source = lines[source_index];
        let caret = lines[source_index + 1];

        assert!(source.chars().count() < 120, "line should be windowed");
        assert!(source.contains("..."));
        let caret_position = caret.find('^').unwrap();
        assert_eq!(source.chars().nth(caret_position), Some('*'));
    }

    #[test]
    fn github_emits_an_error_annotation_per_violation() {
        let source = "# Title\n\ntext   \n";
        let out = github("a.md", &detect(source));
        assert!(
            out.contains("::error file=a.md,line=3,col=5,title=trailing-whitespace::"),
            "got:\n{out}"
        );
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn github_reports_a_windows_path_with_forward_slashes() {
        let source = "# Title\n\ntext   \n";
        let out = github("docs\\guide\\a.md", &detect(source));
        assert!(out.contains("file=docs/guide/a.md,"), "got:\n{out}");
    }

    #[test]
    fn github_escapes_reserved_characters() {
        use crate::violation::Span;
        let violation = Violation {
            rule_id: "demo",
            message: "50% off, now\nlater".to_string(),
            span: Span {
                line: 2,
                column: 3,
                length: 1,
            },
        };
        let out = github("dir,name:1.md", std::slice::from_ref(&violation));
        // Properties escape the comma and colon; the message escapes the percent
        // and newline but keeps its comma.
        assert!(out.contains("file=dir%2Cname%3A1.md"), "got:\n{out}");
        assert!(out.contains("::50%25 off, now%0Alater\n"), "got:\n{out}");
    }

    #[test]
    fn explain_returns_a_known_rule_rationale() {
        let text = explain("final-newline", &default_rules()).unwrap();
        assert!(text.starts_with("final-newline\n"));
        assert!(text.contains("POSIX"));
    }

    #[test]
    fn explain_is_none_for_an_unknown_rule() {
        assert!(explain("no-such-rule", &default_rules()).is_none());
    }
}
