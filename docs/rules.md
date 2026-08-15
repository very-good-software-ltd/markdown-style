# Rules

Every rule is always on.
A rule either _fixes_ what it finds or only _flags_ it.
A rule is flag-only when fixing it would mean guessing at your intent, so the tool reports it and leaves the change to you.
This page is generated from the rules themselves, and the reasoning is the same text `markdown-style explain <rule>` prints.


## byte-order-mark

_Fix._
A leading byte order mark is invisible and travels into your text.

Some tools, notably Windows PowerShell, write a byte order mark at the start of a UTF-8 file.
Markdown is read as UTF-8 everywhere, so the mark carries no information, but it is a real character sitting in front of the first one you wrote.
Nothing renders it, so it survives every review, and any fix that rewrites the first line carries it along into the middle of the text: a title that reads `# Title` actually begins with an invisible character.
Removing it leaves a file that is still valid UTF-8 and says exactly what it appears to say.


## line-endings

_Fix._
Mixed line endings in one file show up as whole-file diffs.

A file that mixes LF and CRLF line endings is invisible to read but noisy to work with: editors and version control disagree about where lines end, and a tool that rewrites the file can flip every line at once, burying a one-line change in a whole-file diff.
The fix is the file's own majority style rather than a fixed choice, because CRLF is the normal convention on Windows and LF elsewhere, and neither is wrong.
Only the minority endings are rewritten, so a file that is already consistent is left exactly as it is.
LF settles a tie.


## trailing-whitespace

_Fix._
Trailing whitespace is invisible but shows up as diff noise.

Trailing spaces and tabs are invisible in most editors but appear in diffs and version control, adding noise that hides real changes.
They are stripped everywhere, except a run of two or more spaces before a line with text, which Markdown treats as an intentional hard line break and which is normalised to exactly two spaces.


## hard-tabs

_Fix._
Use spaces, not hard tabs.

Hard tabs render at different widths in different tools, so indentation and alignment drift between editors.
They are expanded to spaces at four-column tab stops everywhere except inside code blocks, where a tab may be part of the code.


## final-newline

_Fix._
End files with a newline and at most one trailing blank line.

A trailing newline is the POSIX convention that many tools expect, and its absence shows up as a 'no newline at end of file' marker in diffs.
A single blank line at the end is harmless and often left by editors, but more than one is just noise, so extra trailing blank lines are collapsed to one.


## heading-increment

_Flag._
Heading levels should increase one at a time.

Skipping a heading level, for example jumping from # straight to ###, breaks the document outline that screen readers and tables of contents rely on.
Increase depth one level at a time.
This is reported but never fixed automatically, because only you know which level a heading was meant to be.


## heading-style

_Fix._
Use ATX headings (# Heading), not setext underlines.

ATX headings state their level explicitly on the same line and work for all six levels, while setext underlines only reach two and put the level on a separate line.
One style throughout keeps headings consistent and easy to scan.


## atx-heading

_Fix._
Headings use one space after the marker and no closing #s.

A single space after the # marker and no trailing run of #s is the plain, canonical ATX form.
Closing hashes and extra spaces are decorative, vary between authors, and add nothing the renderer uses.


## code-fence

_Fix and flag._
Use backtick code fences, not tildes or indentation.

Backtick fences are the most widely supported form and let you tag a language for highlighting.
Tilde fences are converted to backticks when it is safe, meaning the code contains no backtick fence of its own.
Indented code blocks are reported but not converted, because turning indentation into a fence can change how nearby text parses.


## list-marker

_Fix._
Use `-` for unordered list markers.

One unordered list marker throughout keeps lists visually consistent.
We use `-`: it is the most common choice and never reads as emphasis the way a leading `*` can.


## list-marker-space

_Fix._
Use one space after a list marker.

A single space after the marker keeps list items aligned predictably and the source tidy.
Wider gaps vary between authors and add nothing, so they are collapsed to one space.


## ordered-list

_Fix._
Number ordered items in sequence or all the same.

An ordered list may be numbered two ways.
Sequential numbers (`1. 2. 3.`) match what the reader sees rendered and are easy to follow.
Repeating one number (`1. 1. 1.`) renders identically and keeps diffs small, because inserting or removing an item never renumbers the rest.
Either is fine, so a list that already uses one is left alone, keeping its starting number and delimiter.
A list whose numbers vary without being sequential is the odd one out, and it is renumbered to count up from its first item.


## blockquote-marker

_Fix._
Put one space after each blockquote marker.

A single space after `>` keeps quotes readable in the source and consistent between authors.
Only the marker spacing changes.
Deeper indentation is content and is left as written.


## nested-indent

_Fix._
Indent nested list items to their parent's content.

Aligning a nested list under the first character of its parent's text keeps the outline readable and matches how the list renders.
The indent is the parent's marker width plus one space: two under a bullet, three under `1.`, and so on.


## emphasis

_Fix._
Use _emphasis_ and **strong**.

One marker for each kind of emphasis keeps prose consistent: `_` for emphasis, which stands out from the surrounding text, and `**` for strong, which works even inside a word.
Conversions that would change how the text renders are left alone.


## sentence-per-line

_Fix._
Start each sentence on its own line.

One sentence per line makes repetition and over-long sentences obvious in the source, keeps diffs to the sentences that actually changed, and is why the tool has no line-length rule: a long line is your cue that a sentence is long.
Hard breaks, inline code, and links are preserved, and a sentence is never split where it would start a line with a block marker.


## block-spacing

_Fix._
Blank lines around blocks and headings are kept consistent.

Consistent spacing makes structure scannable: at most one blank line between blocks, two before a heading that follows text so sections stand out, one before a heading that directly follows another and one after any heading, and no blank lines at the very top of the file.


## single-h1

_Flag._
Use a single top-level heading per document.

A document should have exactly one # heading, its title.
Several top-level headings usually mean the file is really two documents, or that a heading should sit a level deeper.
This is reported but never fixed, because only you know which it is.


## continuation-indent

_Flag._
Indent continuation lines to reach the text they continue.

When a paragraph continues onto another line inside a list item or blockquote, that line has to reach the text it continues, by indentation in a list or a `>` prefix in a quote.
A line that falls short is silently folded into the block above it by Markdown's lazy-continuation rule, so it renders as part of an item it does not appear to belong to.
Indent it to line up, or add a blank line to make it a separate block.
This is reported but never fixed, because only you know which you meant.
