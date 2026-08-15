use crate::ast::{Node, NodeKind};
use crate::document::Document;
use crate::rule::Rule;
use crate::text::{Line, dominant_newline, split_lines};
use crate::violation::{Span, Violation};

/// Blank lines between top-level blocks are normalised: two before a heading
/// that follows text, one before a heading that follows another heading, one
/// after any heading, one between other blocks, and none at the top of the file.
///
/// Only all-blank gaps are touched. A gap holding something with no block of its
/// own (a link reference definition, say) is left exactly as written, so nothing
/// is ever lost. Blank lines inside a block, such as within a code block, are
/// left alone too.
pub struct BlockSpacing;

const ID: &str = "block-spacing";

impl Rule for BlockSpacing {
    fn id(&self) -> &'static str {
        ID
    }

    fn short_reason(&self) -> &'static str {
        "Blank lines around blocks and headings are kept consistent."
    }

    fn rationale(&self) -> &'static str {
        "Consistent spacing makes structure scannable: at most one blank line \
         between blocks, two before a heading that follows text so sections stand \
         out, one before a heading that directly follows another and one after \
         any heading, and no blank lines at the very top of the file."
    }

    fn detect(&self, doc: &Document) -> Vec<Violation> {
        rewrite(doc).0
    }

    fn fix(&self, doc: &Document) -> Option<String> {
        Some(rewrite(doc).1)
    }
}

/// Detect and fix in one pass so the two halves can never disagree.
fn rewrite(doc: &Document) -> (Vec<Violation>, String) {
    let lines = split_lines(&doc.source);
    let blocks = &doc.tree().children;
    if blocks.is_empty() {
        return (Vec::new(), doc.source.clone());
    }

    let newline = dominant_newline(&doc.source);
    let mut violations = Vec::new();
    let mut out = String::with_capacity(doc.source.len());

    // Leading region, before the first block.
    let lead_end = blocks[0].span.start - 1;
    if lead_end >= 1 {
        if all_blank(&lines, 1, lead_end) {
            violations.push(violation(1, "remove the blank lines before the first line"));
        } else {
            emit(&mut out, &lines, 1, lead_end);
        }
    }

    emit(&mut out, &lines, blocks[0].span.start, blocks[0].span.end);
    for pair in blocks.windows(2) {
        gap(
            &mut out,
            &mut violations,
            &lines,
            newline,
            &pair[0],
            &pair[1],
        );
        emit(&mut out, &lines, pair[1].span.start, pair[1].span.end);
    }

    // Trailing region is left to the final-newline rule.
    let last_end = blocks[blocks.len() - 1].span.end;
    emit(&mut out, &lines, last_end + 1, lines.len());

    (violations, out)
}

fn gap(
    out: &mut String,
    violations: &mut Vec<Violation>,
    lines: &[Line<'_>],
    newline: &str,
    before: &Node,
    after: &Node,
) {
    let start = before.span.end + 1;
    let end = after.span.start - 1;

    if !all_blank(lines, start, end) {
        emit(out, lines, start, end);
        return;
    }

    let actual = if end >= start { end - start + 1 } else { 0 };
    let desired = desired_gap(before, after);
    if actual != desired {
        violations.push(violation(after.span.start, gap_message(before, after)));
    }
    for _ in 0..desired {
        out.push_str(newline);
    }
}

fn desired_gap(before: &Node, after: &Node) -> usize {
    match after.kind {
        NodeKind::Heading { .. } => match before.kind {
            NodeKind::Heading { .. } | NodeKind::FrontMatter => 1,
            _ => 2,
        },
        _ => 1,
    }
}

fn gap_message(before: &Node, after: &Node) -> &'static str {
    match after.kind {
        NodeKind::Heading { .. } => match before.kind {
            NodeKind::Heading { .. } | NodeKind::FrontMatter => {
                "use one blank line before this heading"
            }
            _ => "use two blank lines before this heading",
        },
        _ => match before.kind {
            NodeKind::Heading { .. } => "use one blank line after a heading",
            _ => "use one blank line between blocks",
        },
    }
}

fn all_blank(lines: &[Line<'_>], from: usize, to: usize) -> bool {
    (from..=to).all(|line| {
        lines
            .get(line - 1)
            .is_none_or(|line| line.content.trim().is_empty())
    })
}

fn emit(out: &mut String, lines: &[Line<'_>], from: usize, to: usize) {
    if from > to {
        return;
    }
    for line in &lines[from - 1..to.min(lines.len())] {
        out.push_str(line.content);
        out.push_str(line.terminator);
    }
}

fn violation(line: usize, message: &str) -> Violation {
    Violation {
        rule_id: ID,
        message: message.to_string(),
        span: Span {
            line,
            column: 1,
            length: 1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(source: &str) -> String {
        BlockSpacing.fix(&Document::new(source)).unwrap()
    }

    fn detect(source: &str) -> Vec<Violation> {
        BlockSpacing.detect(&Document::new(source))
    }

    #[test]
    fn collapses_extra_blank_lines_between_paragraphs() {
        assert_eq!(fix("a\n\n\n\nb\n"), "a\n\nb\n");
    }

    #[test]
    fn uses_two_blank_lines_before_a_heading_after_text() {
        assert_eq!(fix("text\n\n## H\n"), "text\n\n\n## H\n");
        let violations = detect("text\n\n## H\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0].message,
            "use two blank lines before this heading"
        );
    }

    #[test]
    fn uses_one_blank_line_before_a_heading_after_a_heading() {
        assert_eq!(fix("# A\n\n\n## B\n"), "# A\n\n## B\n");
    }

    #[test]
    fn uses_one_blank_line_after_a_heading() {
        assert_eq!(fix("# A\ntext\n"), "# A\n\ntext\n");
    }

    #[test]
    fn removes_blank_lines_at_the_top_of_the_file() {
        assert_eq!(fix("\n\n# A\n"), "# A\n");
        let violations = detect("\n\n# A\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0].message,
            "remove the blank lines before the first line"
        );
    }

    #[test]
    fn preserves_blank_lines_inside_a_code_block() {
        let source = "```\n\n\ncode\n```\n\n\ntext\n";
        assert_eq!(fix(source), "```\n\n\ncode\n```\n\ntext\n");
    }

    #[test]
    fn preserves_a_link_reference_definition_between_blocks() {
        // The link definition has no block node, so its gap is not all-blank and
        // must be copied verbatim rather than dropped.
        let source = "text\n\n[id]: https://example.com\n\n## H\n";
        assert!(fix(source).contains("[id]: https://example.com"));
    }

    #[test]
    fn leaves_a_correctly_spaced_document_untouched() {
        let clean = "# A\n\ntext\n\n\n## B\n\nmore\n";
        assert_eq!(fix(clean), clean);
        assert!(detect(clean).is_empty());
    }

    #[test]
    fn preserves_crlf_line_endings() {
        assert_eq!(fix("a\r\n\r\n\r\n\r\nb\r\n"), "a\r\n\r\nb\r\n");
    }
}
