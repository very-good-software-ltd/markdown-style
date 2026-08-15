# markdown-style

An opinionated linter and formatter for Markdown.

`markdown-style` has one bigger idea and a handful of small ones.
The bigger idea is _one sentence per line_.
The small ones are conventions that keep a Markdown file tidy, like stripping trailing whitespace and using a single heading style.

It comes with a fixed set of rules and no configuration.
That is deliberate.
You point it at a file, it tells you what is off and why, and it can fix most of it for you.


## Why one sentence per line

When every sentence begins on its own line, two things get easier.
Repetition jumps out at you, because near-identical sentences line up in the source.
Overlong sentences announce themselves, because the line runs long.

That second point is why the tool has _no_ line-length rule.
A long line is not a problem to wrap away, it is feedback.
If a line is too wide to read, the sentence is probably too long to read.

`format` rewrites each paragraph to satisfy this: it joins your soft-wrapped lines back into logical text, then splits again at sentence boundaries.
It is careful about the things that would break if you split them.
Inline code, links, and autolinks are protected, hard line breaks are preserved, and a sentence is never broken where the new line would start with something Markdown reads as a list, heading, or quote.

Sentence detection is fast and deliberately conservative: a small set of rules with a short list of abbreviations.
When it is unsure, it leaves the text joined rather than guess wrong, so a bad split never lands silently in your file.


## Install

With Homebrew, on Apple Silicon macOS:

```sh
brew tap very-good-software-ltd/tap
brew trust very-good-software-ltd/tap
brew install very-good-software-ltd/tap/markdown-style
```

`brew trust` is needed because Homebrew asks you to trust a third-party tap before it will load a formula from it.

Or build from source with Cargo:

```sh
cargo install --path .
```

Or use the prebuilt image.
It is built for CI (see [Continuous integration](#continuous-integration)) but also runs directly, linting the mounted directory by default:

```sh
docker run --rm -v "$PWD:/work" ghcr.io/very-good-software-ltd/markdown-style:latest
```


## Usage

Lint a file and read the explanations:

```sh
markdown-style lint README.md
```

Fix a file in place:

```sh
markdown-style format README.md
```

Both commands accept several paths.
A directory is walked for `*.md` and `*.markdown` files, and `.gitignore` is respected, so pointing at a repo root just works:

```sh
markdown-style lint docs/
```

Use `-` to read from stdin, which makes `format` a handy editor filter:

```sh
cat draft.md | markdown-style format -
```

Ask why a rule exists at any time:

```sh
markdown-style explain sentence-per-line
```


### Continuous integration

The prebuilt image at `ghcr.io/very-good-software-ltd/markdown-style` is the simplest way to run the tool in CI: declare it as the job image and call the tool.

On GitLab CI:

```yaml
lint-markdown:
  image: ghcr.io/very-good-software-ltd/markdown-style:latest
  script:
    - markdown-style lint .
```

On GitHub Actions, run it after checkout and pass `--format github` so each violation annotates the pull request diff inline:

```yaml
jobs:
  markdown:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker://ghcr.io/very-good-software-ltd/markdown-style:latest
        with:
          args: lint --format github .
```

A non-zero exit fails the job, so `lint` is your formatting gate.
To fix rather than check, run `format` instead and commit the result in a follow-up step.


### Exit codes

- `0`: no violations.
- `1`: `lint` found violations.
- `2`: something went wrong, like a missing file or input that is not UTF-8.

A violation never counts as an error.
The `lint` command reports every violation it finds across all readable files and exits `1`.
The first real error stops the run at once and exits `2`.

There is no separate check mode, because you do not need one.
Every fixable rule also detects, so `lint` already reports everything `format` would change.
Run `lint` in CI and it is your formatting gate.


## The rules

Every rule is always on.
Some rules _fix_ what they find, others only _flag_ it, because fixing them would mean guessing at your intent.

| Rule | Fix or flag | In short |
| --- | --- | --- |
| `trailing-whitespace` | fix | Strip trailing whitespace, keep intentional hard breaks. |
| `hard-tabs` | fix | Expand tabs to spaces outside code. |
| `final-newline` | fix | End with a newline and at most one trailing blank line. |
| `line-endings` | fix | One line-ending style per file, the file's own majority. |
| `byte-order-mark` | fix | No byte order mark at the start of a file. |
| `block-spacing` | fix | Consistent blank lines around blocks and headings. |
| `heading-increment` | flag | Heading levels increase one at a time. |
| `heading-style` | fix | ATX headings, not setext underlines. |
| `atx-heading` | fix | One space after `#`, no closing hashes. |
| `single-h1` | flag | One top-level heading per document. |
| `code-fence` | fix and flag | Backtick fences, not tildes or indentation. |
| `list-marker` | fix | `-` for unordered lists. |
| `list-marker-space` | fix | One space after a list marker. |
| `ordered-list` | fix | Number ordered items in sequence or all the same. |
| `nested-indent` | fix | Align nested list items with their parent. |
| `blockquote-marker` | fix | One space after `>`. |
| `continuation-indent` | flag | Indent continuation lines to reach the text they continue. |
| `emphasis` | fix | `_emphasis_` and `**strong**`. |
| `sentence-per-line` | fix | Start each sentence on its own line. |

The full description and reasoning for each rule lives in [docs/rules.md](docs/rules.md).
The reasoning there is also what `markdown-style explain <rule>` prints, which is the living source if the two ever drift.


## How it works

Every rule is one definition with a detector and an optional fixer, so linting and formatting can never disagree about what correct means.
The `format` command runs the fixers in order, each seeing the previous one's output, and the result is idempotent, so formatting an already-formatted file changes nothing.

The tool targets CommonMark plus the GitHub extensions, and it leaves YAML frontmatter alone.
Parsing goes through [comrak](https://github.com/kivikakk/comrak), kept behind an internal representation so the rest of the code never touches it directly.

The design decisions behind all of this are recorded as short ADRs in [docs/adr/](docs/adr/), and the project's vocabulary lives in [CONTEXT.md](CONTEXT.md).


## Configuration

There is none, on purpose.
An opinionated tool with a good default set is more useful than a blank slate you have to configure before it helps.
Configuration may come later if a real need shows up, but the starting point is a set of rules we are willing to defend.
