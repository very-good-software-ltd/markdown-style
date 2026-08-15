/// A single source line: its content without the terminator, and the terminator
/// itself (`"\n"`, `"\r\n"`, or `""` for a final line with no trailing newline).
pub struct Line<'a> {
    pub content: &'a str,
    pub terminator: &'a str,
}

/// The line ending the source mostly uses, with LF settling a tie.
///
/// Fixers that insert a line ending ask for this rather than looking for a
/// single `\r\n`, so one stray CRLF in an LF file cannot spread its style
/// through the rest of the document. The `line-endings` rule normalises the
/// stragglers using the same majority.
pub fn dominant_newline(source: &str) -> &'static str {
    let mut crlf = 0;
    let mut lf = 0;
    for line in split_lines(source) {
        match line.terminator {
            "\r\n" => crlf += 1,
            "\n" => lf += 1,
            _ => {}
        }
    }
    if crlf > lf { "\r\n" } else { "\n" }
}

/// Split source into lines, preserving each line's terminator so the original
/// text, including its line-ending style, can be reconstructed exactly.
pub fn split_lines(source: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut rest = source;
    loop {
        match rest.find('\n') {
            Some(idx) => {
                let raw = &rest[..idx];
                let (content, terminator) = match raw.strip_suffix('\r') {
                    Some(without_cr) => (without_cr, "\r\n"),
                    None => (raw, "\n"),
                };
                lines.push(Line {
                    content,
                    terminator,
                });
                rest = &rest[idx + 1..];
                if rest.is_empty() {
                    break;
                }
            }
            None => {
                lines.push(Line {
                    content: rest,
                    terminator: "",
                });
                break;
            }
        }
    }
    lines
}
