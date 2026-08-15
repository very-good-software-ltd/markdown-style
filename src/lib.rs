pub mod ast;
pub mod cli;
pub mod document;
pub mod parser;
pub mod report;
pub mod rule;
pub mod rules;
pub mod sentence;
pub mod text;
pub mod violation;

pub use document::Document;
pub use rule::Rule;
pub use violation::{Span, Violation};

/// The built-in, opinionated rule set. Every rule is always on.
pub fn default_rules() -> Vec<Box<dyn Rule>> {
    vec![
        // These two come first, so every later fixer works on a document whose
        // bytes are already normalised. The byte order mark in particular has
        // to go before any rule can rewrite the first line and carry it into
        // the text.
        Box::new(rules::byte_order_mark::ByteOrderMark),
        Box::new(rules::line_endings::LineEndings),
        Box::new(rules::trailing_whitespace::TrailingWhitespace),
        Box::new(rules::hard_tabs::HardTabs),
        Box::new(rules::final_newline::FinalNewline),
        Box::new(rules::heading_increment::HeadingIncrement),
        Box::new(rules::heading_style::HeadingStyle),
        Box::new(rules::atx_heading::AtxHeading),
        Box::new(rules::code_fence::CodeFence),
        Box::new(rules::list_marker::ListMarker),
        Box::new(rules::list_marker_space::ListMarkerSpace),
        Box::new(rules::ordered_list::OrderedList),
        Box::new(rules::blockquote_marker::BlockquoteMarker),
        Box::new(rules::nested_indent::NestedIndent),
        Box::new(rules::emphasis::Emphasis),
        Box::new(rules::sentence_per_line::SentencePerLine),
        Box::new(rules::block_spacing::BlockSpacing),
        Box::new(rules::single_h1::SingleH1),
        Box::new(rules::continuation_indent::ContinuationIndent),
    ]
}

/// Run every rule's detector against a document and collect the violations.
pub fn lint(doc: &Document, rules: &[Box<dyn Rule>]) -> Vec<Violation> {
    rules.iter().flat_map(|rule| rule.detect(doc)).collect()
}

/// Apply every rule's fixer in order, each seeing the previous fixer's output.
///
/// A fresh `Document` per rule means each fixer parses the current source, so
/// structural fixers work against up-to-date structure. Detect-only rules
/// return no fix and are skipped. The result is idempotent: formatting it again
/// changes nothing.
pub fn format(source: &str, rules: &[Box<dyn Rule>]) -> String {
    let mut current = source.to_string();
    for rule in rules {
        if let Some(fixed) = rule.fix(&Document::new(&current)) {
            current = fixed;
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_runs_the_default_rules() {
        let doc = Document::new("foo   \n");
        let violations = lint(&doc, &default_rules());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_id, "trailing-whitespace");
    }

    #[test]
    fn format_applies_several_fixers_together() {
        let source = "Title\n=====\n\nHello   world  \n";
        assert_eq!(
            format(source, &default_rules()),
            "# Title\n\nHello   world\n"
        );
    }

    #[test]
    fn format_leaves_a_clean_document_untouched() {
        let clean = "# Title\n\nHello world\n";
        assert_eq!(format(clean, &default_rules()), clean);
    }

    #[test]
    fn format_strips_a_byte_order_mark_before_anything_rewrites_the_line() {
        // The mark used to survive as text: rewriting the setext heading
        // carried it past the `# ` and into the middle of the title.
        let source = "\u{feff}Title\n=====\n\nAlpha one.\n";
        assert_eq!(format(source, &default_rules()), "# Title\n\nAlpha one.\n");
    }

    #[test]
    fn format_is_idempotent() {
        let source = "Title\n=====\n\nHello   world  \n\n\n";
        let once = format(source, &default_rules());
        let twice = format(&once, &default_rules());
        assert_eq!(once, twice);
    }
}
