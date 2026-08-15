use crate::document::Document;
use crate::rule::Rule;
use crate::text::{dominant_newline, split_lines};
use crate::violation::{Span, Violation};

/// Every line ending in a file matches the file's own majority style.
///
/// The rule takes no side between LF and CRLF. A wholly CRLF file stays CRLF
/// and a wholly LF file stays LF, so only the odd line out is ever reported.
pub struct LineEndings;

const ID: &str = "line-endings";

impl Rule for LineEndings {
    fn id(&self) -> &'static str {
        ID
    }

    fn short_reason(&self) -> &'static str {
        "Mixed line endings in one file show up as whole-file diffs."
    }

    fn rationale(&self) -> &'static str {
        "A file that mixes LF and CRLF line endings is invisible to read but \
         noisy to work with: editors and version control disagree about where \
         lines end, and a tool that rewrites the file can flip every line at \
         once, burying a one-line change in a whole-file diff. The fix is the \
         file's own majority style rather than a fixed choice, because CRLF is \
         the normal convention on Windows and LF elsewhere, and neither is \
         wrong. Only the minority endings are rewritten, so a file that is \
         already consistent is left exactly as it is. LF settles a tie."
    }

    fn detect(&self, doc: &Document) -> Vec<Violation> {
        analyze(&doc.source).0
    }

    fn fix(&self, doc: &Document) -> Option<String> {
        Some(analyze(&doc.source).1)
    }
}

/// Detect and fix in one pass so the two halves can never disagree.
fn analyze(source: &str) -> (Vec<Violation>, String) {
    let newline = dominant_newline(source);
    let mut violations = Vec::new();
    let mut out = String::with_capacity(source.len());

    for (i, line) in split_lines(source).iter().enumerate() {
        out.push_str(line.content);
        // A final line with no terminator has nothing to normalise; the
        // final-newline rule owns that case.
        if line.terminator.is_empty() || line.terminator == newline {
            out.push_str(line.terminator);
            continue;
        }
        violations.push(Violation {
            rule_id: ID,
            message: format!(
                "{} line ending in a file that mostly uses {}",
                label(line.terminator),
                label(newline),
            ),
            span: Span {
                line: i + 1,
                // The terminator is invisible, so the caret sits just past the
                // last character of the line.
                column: line.content.chars().count() + 1,
                length: 1,
            },
        });
        out.push_str(newline);
    }

    (violations, out)
}

fn label(newline: &str) -> &'static str {
    if newline == "\r\n" { "CRLF" } else { "LF" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(source: &str) -> String {
        LineEndings.fix(&Document::new(source)).unwrap()
    }

    fn detect(source: &str) -> Vec<Violation> {
        LineEndings.detect(&Document::new(source))
    }

    #[test]
    fn leaves_a_wholly_lf_file_untouched() {
        let clean = "alpha\nbravo\ncharlie\n";
        assert_eq!(fix(clean), clean);
        assert!(detect(clean).is_empty());
    }

    #[test]
    fn leaves_a_wholly_crlf_file_untouched() {
        let clean = "alpha\r\nbravo\r\ncharlie\r\n";
        assert_eq!(fix(clean), clean);
        assert!(detect(clean).is_empty());
    }

    #[test]
    fn rewrites_a_lone_crlf_in_a_mostly_lf_file() {
        assert_eq!(fix("alpha\nbravo\r\ncharlie\n"), "alpha\nbravo\ncharlie\n");
    }

    #[test]
    fn rewrites_a_lone_lf_in_a_mostly_crlf_file() {
        assert_eq!(
            fix("alpha\r\nbravo\ncharlie\r\n"),
            "alpha\r\nbravo\r\ncharlie\r\n"
        );
    }

    #[test]
    fn reports_the_minority_line_with_a_span() {
        let violations = detect("alpha\nbravo\r\ncharlie\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_id, "line-endings");
        assert_eq!(
            violations[0].span,
            Span {
                line: 2,
                column: 6,
                length: 1
            }
        );
    }

    #[test]
    fn names_both_styles_in_the_message() {
        let violations = detect("alpha\nbravo\r\ncharlie\n");
        assert_eq!(
            violations[0].message,
            "CRLF line ending in a file that mostly uses LF"
        );

        let violations = detect("alpha\r\nbravo\ncharlie\r\n");
        assert_eq!(
            violations[0].message,
            "LF line ending in a file that mostly uses CRLF"
        );
    }

    #[test]
    fn reports_every_minority_line() {
        let violations = detect("alpha\r\nbravo\ncharlie\ndelta\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].span.line, 1);
    }

    #[test]
    fn settles_a_tie_towards_lf() {
        assert_eq!(fix("alpha\r\nbravo\n"), "alpha\nbravo\n");
    }

    #[test]
    fn leaves_a_final_line_without_a_terminator_alone() {
        assert_eq!(fix("alpha\nbravo"), "alpha\nbravo");
        assert_eq!(fix("alpha\r\nbravo"), "alpha\r\nbravo");
    }

    #[test]
    fn handles_a_document_with_no_line_endings_at_all() {
        assert_eq!(fix("alpha"), "alpha");
        assert!(detect("alpha").is_empty());
    }

    #[test]
    fn handles_an_empty_document() {
        assert_eq!(fix(""), "");
        assert!(detect("").is_empty());
    }

    #[test]
    fn counts_blank_lines_towards_the_majority() {
        // The blank lines are CRLF, so the single LF line is the minority even
        // though the lines with text are evenly split.
        assert_eq!(
            fix("alpha\n\r\n\r\nbravo\r\n"),
            "alpha\r\n\r\n\r\nbravo\r\n"
        );
    }
}
