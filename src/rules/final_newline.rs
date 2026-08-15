use crate::document::Document;
use crate::rule::Rule;
use crate::text::{dominant_newline, split_lines};
use crate::violation::{Span, Violation};

/// A file ends with a newline and at most one trailing blank line. Two or more
/// trailing blank lines are collapsed to one. The file's existing line-ending
/// style is preserved, and trailing whitespace on the final content line is left
/// to the trailing-whitespace rule. An empty file is left empty.
pub struct FinalNewline;

const ID: &str = "final-newline";

impl Rule for FinalNewline {
    fn id(&self) -> &'static str {
        ID
    }

    fn short_reason(&self) -> &'static str {
        "End files with a newline and at most one trailing blank line."
    }

    fn rationale(&self) -> &'static str {
        "A trailing newline is the POSIX convention that many tools expect, and \
         its absence shows up as a 'no newline at end of file' marker in diffs. A \
         single blank line at the end is harmless and often left by editors, but \
         more than one is just noise, so extra trailing blank lines are collapsed \
         to one."
    }

    fn detect(&self, doc: &Document) -> Vec<Violation> {
        let fixed = fixed_source(&doc.source);
        if fixed == doc.source {
            return Vec::new();
        }
        let message = if doc.source.ends_with('\n') {
            "file ends with extra blank lines"
        } else {
            "file does not end with a newline"
        };
        vec![Violation {
            rule_id: ID,
            message: message.to_string(),
            span: end_span(&doc.source),
        }]
    }

    fn fix(&self, doc: &Document) -> Option<String> {
        Some(fixed_source(&doc.source))
    }
}

fn content_lines(source: &str) -> Vec<crate::text::Line<'_>> {
    let mut lines = split_lines(source);
    while lines
        .last()
        .is_some_and(|line| line.content.trim().is_empty())
    {
        lines.pop();
    }
    lines
}

fn fixed_source(source: &str) -> String {
    let lines = split_lines(source);
    let trailing_blanks = lines
        .iter()
        .rev()
        .take_while(|line| line.content.trim().is_empty())
        .count();
    let content = &lines[..lines.len() - trailing_blanks];
    if content.is_empty() {
        return String::new();
    }

    let convention = dominant_newline(source);
    let last = content.len() - 1;
    let mut out = String::with_capacity(source.len());
    for (i, line) in content.iter().enumerate() {
        out.push_str(line.content);
        if i == last && line.terminator.is_empty() {
            out.push_str(convention);
        } else {
            out.push_str(line.terminator);
        }
    }
    // A single trailing blank line is allowed; anything more collapses to one.
    if trailing_blanks > 0 {
        out.push_str(convention);
    }
    out
}

fn end_span(source: &str) -> Span {
    let lines = content_lines(source);
    match lines.last() {
        None => Span {
            line: 1,
            column: 1,
            length: 0,
        },
        Some(last) => Span {
            line: lines.len(),
            column: last.content.chars().count() + 1,
            length: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(source: &str) -> String {
        FinalNewline.fix(&Document::new(source)).unwrap()
    }

    fn detect(source: &str) -> Vec<Violation> {
        FinalNewline.detect(&Document::new(source))
    }

    #[test]
    fn adds_a_missing_final_newline() {
        assert_eq!(fix("foo"), "foo\n");
        let violations = detect("foo");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].message, "file does not end with a newline");
        assert_eq!(
            violations[0].span,
            Span {
                line: 1,
                column: 4,
                length: 0
            }
        );
    }

    #[test]
    fn collapses_multiple_trailing_blank_lines_to_one() {
        assert_eq!(fix("foo\n\n\n"), "foo\n\n");
        let violations = detect("foo\n\n\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].message, "file ends with extra blank lines");
    }

    #[test]
    fn allows_a_single_trailing_blank_line() {
        assert_eq!(fix("foo\n\n"), "foo\n\n");
        assert!(detect("foo\n\n").is_empty());
    }

    #[test]
    fn cleans_a_trailing_whitespace_only_line_but_keeps_one_blank() {
        assert_eq!(fix("foo\n   \n"), "foo\n\n");
    }

    #[test]
    fn leaves_a_single_trailing_newline() {
        assert_eq!(fix("foo\n"), "foo\n");
        assert!(detect("foo\n").is_empty());
    }

    #[test]
    fn ignores_trailing_whitespace_on_the_final_content_line() {
        assert_eq!(fix("foo   \n"), "foo   \n");
        assert!(detect("foo   \n").is_empty());
    }

    #[test]
    fn leaves_an_empty_file_empty() {
        assert_eq!(fix(""), "");
        assert!(detect("").is_empty());
    }

    #[test]
    fn preserves_crlf_when_adding_a_newline() {
        assert_eq!(fix("foo\r\nbar"), "foo\r\nbar\r\n");
    }

    #[test]
    fn collapses_trailing_crlf_blank_lines_to_one() {
        assert_eq!(fix("foo\r\n\r\n\r\n"), "foo\r\n\r\n");
    }
}
