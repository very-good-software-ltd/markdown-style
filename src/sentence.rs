//! Heuristic sentence splitting for the sentence-per-line rule.
//!
//! Works on a single logical line of plain text (no newlines). A boundary is a
//! run of `.`/`!`/`?` (with any closing quotes) followed by whitespace and the
//! start of a new sentence. A curated abbreviation list and single-letter
//! initials suppress false boundaries, and an abbreviation that doubles as an
//! ordinary word only counts when a number follows it. It is deliberately
//! conservative: when in doubt it does not split, so it never runs together with
//! per-run churn.

/// Common abbreviations that end in a period but do not end a sentence. Stored
/// lowercase, with the trailing period.
const ABBREVIATIONS: &[&str] = &[
    "e.g.", "i.e.", "etc.", "vs.", "cf.", "al.", "esp.", "approx.", "dr.", "mr.", "mrs.", "ms.",
    "prof.", "sr.", "jr.", "st.", "vol.", "fig.", "pp.", "inc.", "ltd.", "co.", "u.s.", "u.k.",
    "a.m.", "p.m.",
];

/// Abbreviations that only abbreviate when a number follows, as in "No. 5".
/// They are ordinary words otherwise, and an ordinary word can end a sentence.
const NUMBER_ABBREVIATIONS: &[&str] = &["no."];

/// Whether `text` ends with a terminator a boundary check would honor: a real
/// `.`/`!`/`?` (past any closing quotes or emphasis markers) whose final word is
/// not an abbreviation or initial. Lets a caller preserve an author's line break
/// between sentences even when the next sentence begins with a lowercase word.
pub fn ends_with_sentence_terminator(text: &str) -> bool {
    let chars: Vec<char> = text.trim_end().chars().collect();
    let mut end = chars.len();
    while end > 0 && is_closing(chars[end - 1]) {
        end -= 1;
    }
    if end == 0 || !is_terminator(chars[end - 1]) {
        return false;
    }
    let mut word_start = end;
    while word_start > 0 && !chars[word_start - 1].is_whitespace() {
        word_start -= 1;
    }
    let token: String = chars[word_start..end].iter().collect();
    !is_abbreviation(&token, None)
}

/// Split `text` into sentences, each trimmed of surrounding whitespace.
pub fn split_sentences(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut sentences = Vec::new();
    let mut start = 0;
    let mut index = 0;

    while index < chars.len() {
        if !is_terminator(chars[index]) {
            index += 1;
            continue;
        }

        let mut run_end = index + 1;
        while run_end < chars.len() && is_terminator(chars[run_end]) {
            run_end += 1;
        }
        let mut after = run_end;
        while after < chars.len() && is_closing(chars[after]) {
            after += 1;
        }

        if is_boundary(&chars, index, run_end, after) {
            push_sentence(&mut sentences, &chars[start..after]);
            let next = after
                + chars[after..]
                    .iter()
                    .take_while(|c| c.is_whitespace())
                    .count();
            start = next;
            index = next;
        } else {
            index = run_end;
        }
    }

    if start < chars.len() {
        push_sentence(&mut sentences, &chars[start..]);
    }
    if sentences.is_empty() {
        sentences.push(text.trim().to_string());
    }
    sentences
}

/// A boundary needs whitespace then the start of a new sentence after the
/// punctuation, and the word being ended must not be an abbreviation or initial.
fn is_boundary(chars: &[char], term_start: usize, run_end: usize, after: usize) -> bool {
    let Some(&whitespace) = chars.get(after) else {
        return false;
    };
    if !whitespace.is_whitespace() {
        return false;
    }
    let next = after
        + chars[after..]
            .iter()
            .take_while(|c| c.is_whitespace())
            .count();
    match chars.get(next) {
        Some(&start) if starts_sentence(start) => {}
        _ => return false,
    }

    let mut word_start = term_start;
    while word_start > 0 && !chars[word_start - 1].is_whitespace() {
        word_start -= 1;
    }
    let token: String = chars[word_start..run_end].iter().collect();
    !is_abbreviation(&token, chars.get(next).copied())
}

/// Whether `token` ends in a period that does not end a sentence. `following` is
/// the first character of what comes next, which decides the number
/// abbreviations, and is `None` when nothing follows.
fn is_abbreviation(token: &str, following: Option<char>) -> bool {
    // Drop any leading punctuation, like an opening paren or quote, so a token
    // such as "(e.g." is still recognised as the abbreviation "e.g.".
    let token = token.trim_start_matches(|c: char| !c.is_alphanumeric());
    let lower = token.to_lowercase();
    if ABBREVIATIONS.contains(&lower.as_str()) {
        return true;
    }
    if NUMBER_ABBREVIATIONS.contains(&lower.as_str()) {
        return matches!(following, Some(c) if c.is_ascii_digit());
    }
    // A single letter followed by a period is an initial, like "J." in a name.
    let core = token.trim_end_matches(['.', '!', '?']);
    core.chars().count() == 1 && core.chars().all(char::is_alphabetic)
}

fn push_sentence(sentences: &mut Vec<String>, chars: &[char]) {
    let sentence: String = chars.iter().collect();
    let trimmed = sentence.trim();
    if !trimmed.is_empty() {
        sentences.push(trimmed.to_string());
    }
}

fn is_terminator(c: char) -> bool {
    matches!(c, '.' | '!' | '?')
}

fn is_closing(c: char) -> bool {
    // Includes `*` and `_` so terminal punctuation inside emphasis, like the
    // period in `**Done.**`, still ends a sentence.
    matches!(
        c,
        '"' | '\'' | ')' | ']' | '*' | '_' | '\u{201D}' | '\u{2019}'
    )
}

fn starts_sentence(c: char) -> bool {
    c.is_uppercase()
        || c.is_ascii_digit()
        || matches!(c, '"' | '\'' | '(' | '[' | '\u{201C}' | '\u{2018}')
        // The sentence-per-line rule masks inline code and links with private-use
        // placeholder characters, so a sentence may begin with one.
        || ('\u{E000}'..='\u{F8FF}').contains(&c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_sentence_stays_whole() {
        assert_eq!(split_sentences("One sentence."), vec!["One sentence."]);
    }

    #[test]
    fn splits_on_sentence_boundaries() {
        assert_eq!(
            split_sentences("One. Two. Three."),
            vec!["One.", "Two.", "Three."]
        );
    }

    #[test]
    fn splits_on_all_terminators() {
        assert_eq!(
            split_sentences("One! Two? Three."),
            vec!["One!", "Two?", "Three."]
        );
    }

    #[test]
    fn does_not_split_on_a_lowercase_abbreviation() {
        assert_eq!(
            split_sentences("See e.g. this one."),
            vec!["See e.g. this one."]
        );
    }

    #[test]
    fn does_not_split_an_abbreviation_before_a_capital() {
        assert_eq!(
            split_sentences("Ask Dr. Smith now."),
            vec!["Ask Dr. Smith now."]
        );
    }

    #[test]
    fn does_not_split_a_parenthesised_abbreviation() {
        // The leading "(" must not stop "e.g." from being recognised as an
        // abbreviation when a capitalised example follows.
        assert_eq!(
            split_sentences("Some frameworks (e.g. Foo) work here."),
            vec!["Some frameworks (e.g. Foo) work here."]
        );
    }

    #[test]
    fn splits_after_a_sentence_that_ends_in_no() {
        assert_eq!(
            split_sentences("The world said no. Then it moved on."),
            vec!["The world said no.", "Then it moved on."]
        );
    }

    #[test]
    fn does_not_split_a_number_reference() {
        assert_eq!(
            split_sentences("See No. 5 for details."),
            vec!["See No. 5 for details."]
        );
    }

    #[test]
    fn does_not_split_initials() {
        assert_eq!(
            split_sentences("J. R. R. Tolkien wrote it."),
            vec!["J. R. R. Tolkien wrote it."]
        );
    }

    #[test]
    fn does_not_split_a_decimal_number() {
        assert_eq!(
            split_sentences("Pi is 3.14 today."),
            vec!["Pi is 3.14 today."]
        );
    }

    #[test]
    fn keeps_a_closing_quote_with_the_sentence() {
        assert_eq!(
            split_sentences("He said \"Go.\" Then left."),
            vec!["He said \"Go.\"", "Then left."]
        );
    }

    #[test]
    fn collapses_extra_spaces_between_sentences() {
        assert_eq!(split_sentences("One.   Two."), vec!["One.", "Two."]);
    }

    #[test]
    fn ends_a_sentence_when_the_period_is_inside_emphasis() {
        assert_eq!(
            split_sentences("**Bold lead.** Next sentence here."),
            vec!["**Bold lead.**", "Next sentence here."]
        );
        assert_eq!(
            split_sentences("_Italic lead._ Then more."),
            vec!["_Italic lead._", "Then more."]
        );
    }

    #[test]
    fn a_masked_placeholder_can_begin_a_sentence() {
        // Callers mask inline spans with private-use placeholders; a boundary
        // before one must still be found.
        assert_eq!(
            split_sentences("First. \u{E000}0\u{E001} second."),
            vec!["First.", "\u{E000}0\u{E001} second."]
        );
    }

    #[test]
    fn does_not_split_before_a_lowercase_word() {
        assert_eq!(
            split_sentences("Version 1.0 works. and more"),
            vec!["Version 1.0 works. and more"]
        );
    }

    #[test]
    fn recognises_a_line_that_ends_a_sentence() {
        assert!(ends_with_sentence_terminator("A short sentence."));
        assert!(ends_with_sentence_terminator("Is it, though?"));
        assert!(ends_with_sentence_terminator("He said \"go.\""));
        assert!(ends_with_sentence_terminator("**Bold lead.**"));
    }

    #[test]
    fn does_not_treat_a_soft_wrap_as_a_sentence_end() {
        assert!(!ends_with_sentence_terminator("this clause keeps going"));
    }

    #[test]
    fn treats_a_line_ending_in_no_as_a_sentence_end() {
        assert!(ends_with_sentence_terminator("The world said no."));
    }

    #[test]
    fn does_not_treat_a_trailing_abbreviation_as_a_sentence_end() {
        assert!(!ends_with_sentence_terminator("foo, bar, etc."));
        assert!(!ends_with_sentence_terminator("Ask Dr."));
    }
}
