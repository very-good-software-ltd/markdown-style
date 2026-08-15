use crate::document::Document;
use crate::rule::Rule;
use crate::violation::{Span, Violation};

/// A file does not start with a byte order mark.
///
/// Only a leading mark is a byte order mark. The same character elsewhere in
/// the document is a zero width no-break space, which is content, so it is left
/// alone.
pub struct ByteOrderMark;

const ID: &str = "byte-order-mark";

/// U+FEFF, three bytes (`EF BB BF`) when encoded as UTF-8.
const BOM: char = '\u{feff}';

impl Rule for ByteOrderMark {
    fn id(&self) -> &'static str {
        ID
    }

    fn short_reason(&self) -> &'static str {
        "A leading byte order mark is invisible and travels into your text."
    }

    fn rationale(&self) -> &'static str {
        "Some tools, notably Windows PowerShell, write a byte order mark at the \
         start of a UTF-8 file. Markdown is read as UTF-8 everywhere, so the \
         mark carries no information, but it is a real character sitting in \
         front of the first one you wrote. Nothing renders it, so it survives \
         every review, and any fix that rewrites the first line carries it \
         along into the middle of the text: a title that reads `# Title` \
         actually begins with an invisible character. Removing it leaves a file \
         that is still valid UTF-8 and says exactly what it appears to say."
    }

    fn detect(&self, doc: &Document) -> Vec<Violation> {
        if !doc.source.starts_with(BOM) {
            return Vec::new();
        }
        vec![Violation {
            rule_id: ID,
            message: "file starts with a byte order mark".to_string(),
            span: Span {
                line: 1,
                column: 1,
                length: 1,
            },
        }]
    }

    fn fix(&self, doc: &Document) -> Option<String> {
        Some(
            doc.source
                .strip_prefix(BOM)
                .unwrap_or(&doc.source)
                .to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(source: &str) -> String {
        ByteOrderMark.fix(&Document::new(source)).unwrap()
    }

    fn detect(source: &str) -> Vec<Violation> {
        ByteOrderMark.detect(&Document::new(source))
    }

    #[test]
    fn strips_a_leading_byte_order_mark() {
        assert_eq!(fix("\u{feff}# Title\n"), "# Title\n");
    }

    #[test]
    fn reports_a_leading_byte_order_mark_with_a_span() {
        let violations = detect("\u{feff}# Title\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_id, "byte-order-mark");
        assert_eq!(violations[0].message, "file starts with a byte order mark");
        assert_eq!(
            violations[0].span,
            Span {
                line: 1,
                column: 1,
                length: 1
            }
        );
    }

    #[test]
    fn reports_it_only_once_however_long_the_file_is() {
        let violations = detect("\u{feff}# Title\n\nAlpha one.\nBravo two.\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn leaves_a_file_without_one_untouched() {
        let clean = "# Title\n\nAlpha one.\n";
        assert_eq!(fix(clean), clean);
        assert!(detect(clean).is_empty());
    }

    #[test]
    fn leaves_the_same_character_alone_inside_the_text() {
        // Only a leading mark is a byte order mark; elsewhere it is a zero
        // width no-break space, which is content.
        let source = "# Title\n\nAlpha\u{feff}one.\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn strips_only_the_first_of_two_leading_marks() {
        // The second is no longer at the start of the file, so it is text.
        assert_eq!(fix("\u{feff}\u{feff}# Title\n"), "\u{feff}# Title\n");
    }

    #[test]
    fn handles_a_file_that_is_only_a_byte_order_mark() {
        assert_eq!(fix("\u{feff}"), "");
    }

    #[test]
    fn handles_an_empty_document() {
        assert_eq!(fix(""), "");
        assert!(detect("").is_empty());
    }
}
