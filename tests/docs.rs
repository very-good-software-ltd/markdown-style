use std::fs;

use markdown_style::default_rules;

/// The rule catalog is generated from the registry, so a change to a rule's
/// reasoning or set must be reflected in the committed doc. Regenerate with
/// `markdown-style rules --markdown > docs/rules.md`.
#[test]
fn rules_doc_is_up_to_date() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/rules.md");
    let committed = fs::read_to_string(path).unwrap();
    let generated = markdown_style::report::rules_catalog();

    assert_eq!(
        committed, generated,
        "docs/rules.md is stale; regenerate with `markdown-style rules --markdown > docs/rules.md`"
    );
}

/// The README's rule table is written by hand rather than generated from the
/// registry, so it drifts silently. This catches the drift that actually
/// happens, a rule registered without a row, and nothing finer: it looks for
/// the id anywhere in the file rather than parsing the table, so it says
/// nothing about the wording or the fix-or-flag column.
#[test]
fn readme_mentions_every_rule() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/README.md");
    let readme = fs::read_to_string(path).unwrap();

    let missing: Vec<&str> = default_rules()
        .iter()
        .map(|rule| rule.id())
        .filter(|id| !readme.contains(&format!("`{id}`")))
        .collect();

    assert!(
        missing.is_empty(),
        "README.md has no entry for {missing:?}; add a row to the rules table"
    );
}
