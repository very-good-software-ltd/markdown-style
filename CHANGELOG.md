# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]

### Fixed

- The Windows binary is now attached to the release.
  In 0.6.1 the release job printed the archive's checksum before uploading it and failed there, so the zip never reached the release page.


## [0.6.1] - 2026-08-15

### Added

- A prebuilt Windows binary (`x86_64-pc-windows-msvc`) is now published with each release.
  The test suite also runs on Windows in CI.


## [0.6.0] - 2026-08-15

### Added

- A `byte-order-mark` rule.
  A file that starts with a UTF-8 byte order mark, as Windows PowerShell writes, now has it reported and removed.
  The same character elsewhere in the document is a zero width no-break space rather than a byte order mark, so it is left alone.

- A `line-endings` rule.
  A file that mixes LF and CRLF endings now has the minority endings reported and rewritten to the file's own majority style, with LF settling a tie.
  The rule takes no side between the two, so a file that is wholly LF or wholly CRLF is left exactly as it is.


### Fixed

- Fixers that insert a line ending now follow the document's majority style.
  Previously a single CRLF anywhere in an otherwise LF file made every inserted line ending a CRLF.
- `format` no longer carries a leading byte order mark into the document text.
  Any fix that rewrote the first line moved the mark past it, so a title fixed to `# Title` silently began with an invisible character, and the file then passed `lint` clean.
- `lint --format github` now reports paths with forward slashes.
  On a Windows runner the backslashes meant GitHub could not match the path against the repository, so the annotation never attached to the diff.


## [0.5.1] - 2026-08-07

### Added

- A prebuilt Linux binary (`x86_64-unknown-linux-musl`) is now published with each release.
- A prebuilt Docker image is now published to `ghcr.io/very-good-software-ltd/markdown-style` with each release, carrying only the tool so it can run checks in any repository.


## [0.5.0] - 2026-08-07

### Added

- `lint` takes a `--format` option to choose how diagnostics are rendered.
  `human`, the default, is the existing terminal output.
  `github` emits GitHub Actions workflow commands, so a CI run annotates the pull request diff inline.


## [0.4.1] - 2026-08-04

### Fixed

- Sentence-per-line no longer splits after an abbreviation such as `e.g.` when it is preceded by an opening bracket or quote, for example `(e.g. ...)`, and followed by something that starts a sentence.


## [0.4.0] - 2026-08-02

### Changed

- `ordered-list` now accepts the all-same style (`1. 1. 1.`) as well as sequential numbering, since it renders identically and keeps diffs small.
  A list whose numbers vary without being sequential is still renumbered to count up from its first item.


### Fixed

- Sentence-per-line no longer merges a sentence into the line above it when the author already broke the line at a sentence end and the next sentence begins with a lowercase word, for example a command or package name.


## [0.3.0] - 2026-08-02

### Added

- `continuation-indent` rule: flags a paragraph continuation line inside a list item or blockquote that is not indented (or `>`-prefixed) far enough to reach the text it continues, which Markdown would otherwise fold into the wrong block.


## [0.2.1] - 2026-08-02

### Fixed

- Sentence-per-line no longer merges a sentence that begins with inline code, a link, or an autolink into the preceding sentence.


## [0.2.0] - 2026-08-02

### Changed

- The `final-newline` rule now allows a single trailing blank line at the end of a file, collapsing only runs of two or more (previously all trailing blank lines were removed).


### Fixed

- Sentence-per-line no longer joins a bold or italic lead-in into the following sentence when its ending punctuation sits inside the emphasis markers, for example `**Done.**`.


## [0.1.0] - 2026-08-02

### Added

- `lint`, `format`, and `explain` commands, plus `rules` to list the rule set.
- Sentence-per-line formatting as the core feature: prose in paragraphs, blockquotes, and list items is rewritten so every sentence starts on its own line, with no line-length rule.
- An opinionated, always-on set of sixteen rules covering whitespace and file hygiene, heading style and hierarchy, list markers, numbering and indentation, blockquote and emphasis markers, code fences, and blank-line spacing.
- rustc-style diagnostics with a source snippet, a caret at the exact location, and a per-rule `why:` explanation, with long lines windowed so the caret stays aligned.
- `explain <rule>` prints a rule's full reasoning.
- CommonMark and GitHub Flavored Markdown support, with YAML frontmatter preserved.
- Files, directories (walked for Markdown, respecting `.gitignore`), and stdin (`-`) as inputs.
- Exit codes `0` for clean, `1` for violations, and `2` for errors, with fail-fast on the first operational error.
- Idempotent formatting, so running `format` twice makes no further changes.
- `rules --markdown` generates the rule catalogue in `docs/rules.md`, kept current by a test.

[Unreleased]: https://github.com/very-good-software-ltd/markdown-style/compare/0.6.1...HEAD
[0.6.1]: https://github.com/very-good-software-ltd/markdown-style/compare/0.6.0...0.6.1
[0.6.0]: https://github.com/very-good-software-ltd/markdown-style/compare/0.5.1...0.6.0
[0.5.1]: https://github.com/very-good-software-ltd/markdown-style/compare/0.5.0...0.5.1
[0.5.0]: https://github.com/very-good-software-ltd/markdown-style/compare/0.4.1...0.5.0
[0.4.1]: https://github.com/very-good-software-ltd/markdown-style/compare/0.4.0...0.4.1
[0.4.0]: https://github.com/very-good-software-ltd/markdown-style/compare/0.3.0...0.4.0
[0.3.0]: https://github.com/very-good-software-ltd/markdown-style/compare/0.2.1...0.3.0
[0.2.1]: https://github.com/very-good-software-ltd/markdown-style/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/very-good-software-ltd/markdown-style/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/very-good-software-ltd/markdown-style/releases/tag/0.1.0
