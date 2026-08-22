use std::collections::HashMap;

use crate::ast::{Node, NodeKind};
use crate::document::Document;
use crate::rule::Rule;
use crate::sentence::{ends_with_sentence_terminator, split_sentences};
use crate::text::{Line, dominant_newline, split_lines};
use crate::violation::{Span, Violation};

/// The tool's core rule: within prose, every sentence begins on its own line.
///
/// A paragraph's soft-wrapped lines are joined into logical text and re-split at
/// sentence boundaries. Author hard breaks (two trailing spaces) are barriers we
/// never join across. Inline code, links, and autolinks are protected so a
/// boundary inside them cannot split them across lines. A boundary that would put
/// a block marker at the start of a line is not taken, so nothing is corrupted.
///
/// Scope is prose: top-level paragraphs, blockquotes, and list items. A
/// paragraph is only rewritten when its sentence structure actually changes, so
/// trailing whitespace and marker spacing stay the concern of their own rules and
/// lint never disagrees with format.
pub struct SentencePerLine;

const ID: &str = "sentence-per-line";
const PH_OPEN: char = '\u{E000}';
const PH_CLOSE: char = '\u{E001}';

impl Rule for SentencePerLine {
    fn id(&self) -> &'static str {
        ID
    }

    fn short_reason(&self) -> &'static str {
        "Start each sentence on its own line."
    }

    fn rationale(&self) -> &'static str {
        "One sentence per line makes repetition and over-long sentences obvious \
         in the source, keeps diffs to the sentences that actually changed, and is \
         why the tool has no line-length rule: a long line is your cue that a \
         sentence is long. Hard breaks, inline code, and links are preserved, and \
         a sentence is never split where it would start a line with a block marker."
    }

    fn detect(&self, doc: &Document) -> Vec<Violation> {
        rewrite(doc).0
    }

    fn fix(&self, doc: &Document) -> Option<String> {
        Some(rewrite(doc).1)
    }
}

/// How a paragraph's lines are prefixed in the source.
enum Prefix {
    /// Top-level paragraph, no prefix.
    Plain,
    /// Inside blockquotes, at the given depth.
    Quote(usize),
    /// Inside a list item; the content starts at the given 1-based column.
    List(usize),
}

/// Detect and fix in one pass so the two halves can never disagree.
fn rewrite(doc: &Document) -> (Vec<Violation>, String) {
    let lines = split_lines(&doc.source);
    let newline = dominant_newline(&doc.source);

    let mut targets = Vec::new();
    collect(doc.tree(), 0, false, &mut targets);

    let mut replacements: HashMap<usize, (usize, String)> = HashMap::new();
    let mut violations = Vec::new();
    for (start, end, prefix) in targets {
        let slice = &lines[start - 1..end];
        let (first_prefix, cont_prefix, stripped) = strip(&prefix, slice);
        let segments = produce(&stripped);

        let before: Vec<String> = stripped
            .iter()
            .map(|line| line.trim().to_string())
            .collect();
        let after: Vec<String> = segments.iter().flatten().cloned().collect();

        if before != after {
            violations.push(Violation {
                rule_id: ID,
                message: "put each sentence on its own line".to_string(),
                span: boundary_span(slice, &stripped, start, &before, &after),
            });
            let last_terminator = slice[slice.len() - 1].terminator;
            let emitted = emit(
                &segments,
                &first_prefix,
                &cont_prefix,
                newline,
                last_terminator,
            );
            replacements.insert(start, (end, emitted));
        } else {
            replacements.insert(start, (end, original(slice)));
        }
    }

    let mut out = String::with_capacity(doc.source.len());
    let mut line_number = 1;
    while line_number <= lines.len() {
        if let Some((end, replacement)) = replacements.get(&line_number) {
            out.push_str(replacement);
            line_number = end + 1;
        } else {
            let line = &lines[line_number - 1];
            out.push_str(line.content);
            out.push_str(line.terminator);
            line_number += 1;
        }
    }

    (violations, out)
}

/// Collect prose paragraphs to reflow, tracking blockquote depth and list-item
/// membership. Paragraphs nested in both a blockquote and a list item are skipped
/// for now.
fn collect(node: &Node, depth: usize, in_item: bool, out: &mut Vec<(usize, usize, Prefix)>) {
    match node.kind {
        NodeKind::Paragraph => {
            let prefix = match (in_item, depth) {
                (false, 0) => Prefix::Plain,
                (false, depth) => Prefix::Quote(depth),
                (true, 0) => Prefix::List(node.start_column),
                (true, _) => return,
            };
            out.push((node.span.start, node.span.end, prefix));
        }
        NodeKind::BlockQuote => {
            for child in &node.children {
                collect(child, depth + 1, in_item, out);
            }
        }
        NodeKind::Item => {
            for child in &node.children {
                collect(child, depth, true, out);
            }
        }
        _ => {
            for child in &node.children {
                collect(child, depth, in_item, out);
            }
        }
    }
}

fn original(slice: &[Line<'_>]) -> String {
    slice
        .iter()
        .map(|line| format!("{}{}", line.content, line.terminator))
        .collect()
}

/// Point at the sentence that should move onto its own line, not the first
/// sentence, which is already correct. That is the start of the second sentence
/// on the first line that holds more than one. If none does, the paragraph only
/// needs joining, so point at the first line that gets pulled up onto the one
/// above it rather than guessing at the paragraph's second line.
fn boundary_span(
    slice: &[Line<'_>],
    stripped: &[&str],
    start: usize,
    before: &[String],
    after: &[String],
) -> Span {
    for (offset, content) in stripped.iter().enumerate() {
        let sentences: Vec<String> = produce(std::slice::from_ref(content))
            .into_iter()
            .flatten()
            .collect();
        if sentences.len() > 1 {
            let original = slice[offset].content;
            let column = original
                .find(sentences[1].as_str())
                .map_or(1, |byte| original[..byte].chars().count() + 1);
            return Span {
                line: start + offset,
                column,
                length: 1,
            };
        }
    }

    // The first line whose emitted form differs is the one that grew, so the
    // line after it is the one that has to move up.
    if let Some(offset) = first_difference(before, after)
        && offset + 1 < slice.len()
    {
        let moved = offset + 1;
        let original = slice[moved].content;
        let column = original
            .find(stripped[moved].trim_start())
            .map_or(1, |byte| original[..byte].chars().count() + 1);
        return Span {
            line: start + moved,
            column,
            length: 1,
        };
    }

    Span {
        line: start,
        column: 1,
        length: 1,
    }
}

/// The first index at which the paragraph's source lines and its emitted
/// sentences disagree, if any.
fn first_difference(before: &[String], after: &[String]) -> Option<usize> {
    (0..before.len().max(after.len())).find(|&index| before.get(index) != after.get(index))
}

/// Return the prefix for the first emitted line, the prefix for continuation
/// lines, and each source line stripped of its prefix.
fn strip<'a>(prefix: &Prefix, slice: &'a [Line<'a>]) -> (String, String, Vec<&'a str>) {
    match *prefix {
        Prefix::Plain => (
            String::new(),
            String::new(),
            slice.iter().map(|line| line.content).collect(),
        ),
        Prefix::Quote(depth) => {
            let marker = "> ".repeat(depth);
            let stripped = slice
                .iter()
                .map(|line| strip_quote(line.content, depth))
                .collect();
            (marker.clone(), marker, stripped)
        }
        Prefix::List(column) => {
            let width = column.saturating_sub(1);
            let first = slice[0].content;
            let first_prefix: String = first.chars().take(width).collect();
            let mut stripped = vec![&first[first_prefix.len()..]];
            stripped.extend(
                slice[1..]
                    .iter()
                    .map(|line| strip_indent(line.content, width)),
            );
            (first_prefix, " ".repeat(width), stripped)
        }
    }
}

/// Remove up to `depth` leading `>` markers (each with an optional space).
/// Lenient, so lazy continuation lines without a marker pass through unchanged.
fn strip_quote(content: &str, depth: usize) -> &str {
    let mut rest = content;
    for _ in 0..depth {
        match rest.strip_prefix('>') {
            Some(after) => rest = after.strip_prefix(' ').unwrap_or(after),
            None => break,
        }
    }
    rest
}

/// Remove up to `width` leading spaces.
fn strip_indent(content: &str, width: usize) -> &str {
    let mut removed = 0;
    let mut offset = 0;
    for (index, c) in content.char_indices() {
        if removed < width && c == ' ' {
            removed += 1;
            offset = index + 1;
        } else {
            break;
        }
    }
    &content[offset..]
}

/// Split each hard-break segment into sentences. Returns one list of sentences
/// per segment so emission can restore the hard breaks between them.
fn produce(stripped: &[&str]) -> Vec<Vec<String>> {
    segment_on_hard_breaks(stripped)
        .iter()
        .map(|segment| {
            let sentences: Vec<String> = join_groups(segment)
                .iter()
                .flat_map(|group| sentences_of(group))
                .collect();
            merge_block_marker_starts(sentences)
        })
        .collect()
}

/// Split a segment into groups joined only across soft wraps. A line that ends
/// with a real sentence terminator ends its group, so an author's explicit break
/// between sentences is preserved even when the next sentence starts lowercase.
fn join_groups<'a>(segment: &[&'a str]) -> Vec<Vec<&'a str>> {
    let mut groups: Vec<Vec<&str>> = vec![Vec::new()];
    for (index, content) in segment.iter().enumerate() {
        groups.last_mut().unwrap().push(content);
        let is_last = index + 1 == segment.len();
        if !is_last && ends_with_sentence_terminator(content) {
            groups.push(Vec::new());
        }
    }
    groups
}

/// Join a group's soft-wrapped lines and split the result into sentences.
fn sentences_of(group: &[&str]) -> Vec<String> {
    let joined = group
        .iter()
        .map(|content| content.trim())
        .filter(|content| !content.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let (masked, protected) = mask(&joined);
    split_sentences(&masked)
        .into_iter()
        .map(|sentence| restore(&sentence, &protected))
        .filter(|sentence| !sentence.is_empty())
        .collect()
}

fn emit(
    segments: &[Vec<String>],
    first_prefix: &str,
    cont_prefix: &str,
    newline: &str,
    last_terminator: &str,
) -> String {
    let mut out_lines: Vec<String> = Vec::new();
    let segment_count = segments.len();
    for (segment_index, sentences) in segments.iter().enumerate() {
        let is_last_segment = segment_index + 1 == segment_count;
        let last_sentence = sentences.len().saturating_sub(1);
        for (sentence_index, sentence) in sentences.iter().enumerate() {
            let prefix = if out_lines.is_empty() {
                first_prefix
            } else {
                cont_prefix
            };
            let mut line = format!("{prefix}{sentence}");
            if !is_last_segment && sentence_index == last_sentence {
                line.push_str("  ");
            }
            out_lines.push(line);
        }
    }

    let mut out = String::new();
    let last = out_lines.len().saturating_sub(1);
    for (index, line) in out_lines.into_iter().enumerate() {
        out.push_str(&line);
        out.push_str(if index == last {
            last_terminator
        } else {
            newline
        });
    }
    out
}

/// Split a paragraph's (prefix-stripped) lines into segments at author hard
/// breaks (two or more trailing spaces), which we never join across.
fn segment_on_hard_breaks<'a>(stripped: &[&'a str]) -> Vec<Vec<&'a str>> {
    let mut segments: Vec<Vec<&str>> = vec![Vec::new()];
    for (index, content) in stripped.iter().enumerate() {
        segments.last_mut().unwrap().push(content);
        let is_last = index + 1 == stripped.len();
        if !is_last && ends_with_hard_break(content) {
            segments.push(Vec::new());
        }
    }
    segments
}

fn ends_with_hard_break(content: &str) -> bool {
    let trimmed = content.trim_end_matches(' ');
    content.len() - trimmed.len() >= 2
}

/// Merge back any sentence that would begin a line with a block marker, so the
/// re-emitted text cannot be reparsed as a list, heading, quote, or rule.
fn merge_block_marker_starts(sentences: Vec<String>) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();
    for sentence in sentences {
        match merged.last_mut() {
            Some(previous) if starts_with_block_marker(&sentence) => {
                previous.push(' ');
                previous.push_str(&sentence);
            }
            _ => merged.push(sentence),
        }
    }
    merged
}

fn starts_with_block_marker(sentence: &str) -> bool {
    sentence.starts_with('#')
        || sentence.starts_with('>')
        || starts_with_bullet(sentence)
        || starts_with_ordered(sentence)
        || is_rule_line(sentence)
}

fn starts_with_bullet(sentence: &str) -> bool {
    let mut chars = sentence.chars();
    match chars.next() {
        Some('-' | '*' | '+') => matches!(chars.next(), Some(' ') | None),
        _ => false,
    }
}

fn starts_with_ordered(sentence: &str) -> bool {
    let bytes = sentence.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 || i >= bytes.len() || (bytes[i] != b'.' && bytes[i] != b')') {
        return false;
    }
    i + 1 == bytes.len() || bytes[i + 1] == b' '
}

fn is_rule_line(sentence: &str) -> bool {
    let trimmed = sentence.trim();
    trimmed.chars().count() >= 3 && trimmed.chars().all(|c| matches!(c, '-' | '=' | '*' | '_'))
}

/// Replace inline code, links, images, and autolinks with placeholders that
/// contain no sentence punctuation, so the splitter cannot break inside them.
fn mask(text: &str) -> (String, Vec<String>) {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut protected: Vec<String> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            let run = run_length(&chars, i, '`');
            if let Some(close) = closing_backticks(&chars, i + run, run) {
                protect(&mut out, &mut protected, &chars[i..close + run]);
                i = close + run;
                continue;
            }
        } else if c == '<' {
            if let Some(gt) = chars[i + 1..]
                .iter()
                .position(|&c| c == '>')
                .map(|p| i + 1 + p)
                && !chars[i + 1..gt].iter().any(|c| c.is_whitespace())
            {
                protect(&mut out, &mut protected, &chars[i..gt + 1]);
                i = gt + 1;
                continue;
            }
        } else if c == '[' || (c == '!' && chars.get(i + 1) == Some(&'[')) {
            let bracket = if c == '!' { i + 1 } else { i };
            if let Some(end) = link_end(&chars, bracket) {
                protect(&mut out, &mut protected, &chars[i..end]);
                i = end;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }

    (out, protected)
}

fn link_end(chars: &[char], bracket: usize) -> Option<usize> {
    let close = matching(chars, bracket, '[', ']')?;
    let mut end = close + 1;
    match chars.get(end) {
        Some('(') => {
            if let Some(paren) = matching(chars, end, '(', ')') {
                end = paren + 1;
            }
        }
        Some('[') => {
            if let Some(reference) = matching(chars, end, '[', ']') {
                end = reference + 1;
            }
        }
        _ => {}
    }
    Some(end)
}

fn matching(chars: &[char], open: usize, open_char: char, close_char: char) -> Option<usize> {
    let mut depth = 0;
    for (offset, &c) in chars[open..].iter().enumerate() {
        if c == open_char {
            depth += 1;
        } else if c == close_char {
            depth -= 1;
            if depth == 0 {
                return Some(open + offset);
            }
        }
    }
    None
}

fn run_length(chars: &[char], from: usize, c: char) -> usize {
    chars[from..].iter().take_while(|&&x| x == c).count()
}

fn closing_backticks(chars: &[char], from: usize, run: usize) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == '`' {
            let length = run_length(chars, i, '`');
            if length == run {
                return Some(i);
            }
            i += length;
        } else {
            i += 1;
        }
    }
    None
}

fn protect(out: &mut String, protected: &mut Vec<String>, span: &[char]) {
    out.push(PH_OPEN);
    out.push_str(&protected.len().to_string());
    out.push(PH_CLOSE);
    protected.push(span.iter().collect());
}

fn restore(sentence: &str, protected: &[String]) -> String {
    let mut restored = sentence.to_string();
    for (index, original) in protected.iter().enumerate() {
        let placeholder = format!("{PH_OPEN}{index}{PH_CLOSE}");
        restored = restored.replace(&placeholder, original);
    }
    restored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(source: &str) -> String {
        SentencePerLine.fix(&Document::new(source)).unwrap()
    }

    fn detect(source: &str) -> Vec<Violation> {
        SentencePerLine.detect(&Document::new(source))
    }

    #[test]
    fn leaves_a_single_sentence_paragraph_alone() {
        let source = "Hello world.\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn splits_a_multi_sentence_line() {
        assert_eq!(fix("One. Two.\n"), "One.\nTwo.\n");
        assert_eq!(detect("One. Two.\n").len(), 1);
    }

    #[test]
    fn points_at_the_second_sentence_not_the_first() {
        let violations = detect("First sentence. Second sentence.\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].span.line, 1);
        // "Second" begins at column 17, after "First sentence. ".
        assert_eq!(violations[0].span.column, 17);
    }

    #[test]
    fn points_at_the_continuation_line_that_must_move_up() {
        // Only the last two lines belong together, so the flag must land on the
        // line that gets pulled up, not on the paragraph's second line.
        let violations =
            detect("Alpha stands alone.\nBeta stands alone.\nGamma keeps\ngoing to the end.\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].span.line, 4);
        assert_eq!(violations[0].span.column, 1);
    }

    #[test]
    fn points_past_a_blockquote_marker_when_joining() {
        let violations = detect("> Alpha stands alone.\n> Gamma keeps\n> going to the end.\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].span.line, 3);
        // Column 3 is the text, past the "> " marker.
        assert_eq!(violations[0].span.column, 3);
    }

    #[test]
    fn joins_soft_wrapped_lines_of_one_sentence() {
        assert_eq!(fix("One two\nthree four.\n"), "One two three four.\n");
    }

    #[test]
    fn keeps_a_hard_break_as_a_barrier() {
        // The hard break after the first line is preserved; the second segment
        // still splits into one sentence per line.
        assert_eq!(fix("first  \nfoo. Bar\n"), "first  \nfoo.\nBar\n");
    }

    #[test]
    fn protects_an_inline_code_span() {
        assert_eq!(
            fix("Call `a. b` now. Then stop.\n"),
            "Call `a. b` now.\nThen stop.\n"
        );
    }

    #[test]
    fn protects_a_link() {
        assert_eq!(
            fix("See [a. b](http://x). Next.\n"),
            "See [a. b](http://x).\nNext.\n"
        );
    }

    #[test]
    fn does_not_break_before_a_block_marker() {
        let source = "See below. - not a list.\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn leaves_code_blocks_untouched() {
        let source = "```\nx. y. z\n```\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn preserves_crlf_line_endings() {
        assert_eq!(fix("One. Two.\r\n"), "One.\r\nTwo.\r\n");
    }

    #[test]
    fn splits_a_blockquote_paragraph() {
        assert_eq!(fix("> One. Two.\n"), "> One.\n> Two.\n");
        assert_eq!(detect("> One. Two.\n").len(), 1);
    }

    #[test]
    fn joins_a_soft_wrapped_blockquote() {
        assert_eq!(fix("> One two\n> three four.\n"), "> One two three four.\n");
    }

    #[test]
    fn splits_a_nested_blockquote() {
        assert_eq!(fix("> > One. Two.\n"), "> > One.\n> > Two.\n");
    }

    #[test]
    fn canonicalises_a_lazy_continuation_when_splitting() {
        assert_eq!(
            fix("> One two\nthree. Four.\n"),
            "> One two three.\n> Four.\n"
        );
    }

    #[test]
    fn leaves_a_well_formed_blockquote_sentence_alone() {
        let source = "> Hello world.\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn does_not_flag_blockquote_marker_spacing_alone() {
        let source = ">  Hello world.\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn does_not_touch_trailing_whitespace_when_structure_is_correct() {
        let source = "Hello world.   \n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn splits_a_bulleted_list_item() {
        assert_eq!(fix("- One. Two.\n"), "- One.\n  Two.\n");
        assert_eq!(detect("- One. Two.\n").len(), 1);
    }

    #[test]
    fn splits_an_ordered_list_item_aligning_continuations() {
        assert_eq!(fix("1. One. Two.\n"), "1. One.\n   Two.\n");
    }

    #[test]
    fn joins_a_soft_wrapped_list_item() {
        assert_eq!(
            fix("- One two\n  three. Four.\n"),
            "- One two three.\n  Four.\n"
        );
    }

    #[test]
    fn leaves_a_well_formed_list_item_alone() {
        let source = "- Hello world.\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn keeps_a_sentence_that_starts_with_inline_code_on_its_own_line() {
        // The second sentence starts with a masked code span; it must not be
        // merged into the first.
        let source = "First line here.\n`code` starts the second.\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn keeps_an_author_break_before_a_lowercase_sentence() {
        // Each line already ends a sentence, so an author's break before a
        // sentence that starts with a lowercase word must be preserved rather
        // than merged into the line above.
        let source =
            "This tool depends on that one.\nnpm sees it is already present.\nThe rest follows.\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn still_joins_a_soft_wrap_before_a_lowercase_word() {
        // The first line does not end a sentence, so it is a soft wrap and the
        // lines join as before.
        assert_eq!(
            fix("this clause keeps\ngoing to the end.\n"),
            "this clause keeps going to the end.\n"
        );
    }

    #[test]
    fn keeps_a_sentence_that_ends_in_no_on_its_own_line() {
        // "no." is only an abbreviation before a number, so a line ending in the
        // word itself must not be joined with the line below.
        let source = "The world said no.\nYou cannot argue with a refusal.\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn keeps_a_parenthesised_abbreviation_before_a_code_span_together() {
        // "(e.g. `foo`)" must not split before the code span: the abbreviation
        // guard has to see through the leading paren.
        let source = "Routes (e.g. `foo`) are matched, so both `a` and `b` work.\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }

    #[test]
    fn keeps_a_bold_lead_in_on_its_own_line() {
        // The period sits inside the emphasis, so the lead-in is its own sentence
        // and must not be joined with the line below.
        let source = "- **Bold lead.**\n  First point here.\n  Second point here.\n";
        assert_eq!(fix(source), source);
        assert!(detect(source).is_empty());
    }
}
