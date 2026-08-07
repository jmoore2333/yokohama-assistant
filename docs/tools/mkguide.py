#!/usr/bin/env python3
"""Regenerate `docs/cli-guide.html` from `docs/cli-guide.md`.

    python3 docs/tools/mkguide.py            # from the repo root
    python3 docs/tools/mkguide.py head.html  # with an explicit head block

The guide ships as a word-identical pair (CLAUDE.md's convention: any change to
CLI behavior or user-visible text updates BOTH formats in the same commit). This
converts the .md half into the .html half so the two cannot drift by hand-editing
one of them.

**A CONVENIENCE, NOT THE GUARANTEE.** `crates/yokohama-cli/tests/guide_sync.rs`
is what enforces the pair -- HELP_TEXT command coverage, the move grammars, the
computed board/frame widths, and the byte-identical §5 and captured-screen blocks
across the two formats. That test is the merge bar; this script only saves the
typing. Regenerating and running `cargo test -p yokohama-cli --test guide_sync`
is the full loop.

The .html head block -- doctype, `<style>`, everything through the `<body>` line
-- is NOT generated: it is hand-maintained CSS. With no argument the script
lifts it from the existing `docs/cli-guide.html`, which makes an argument-free
run reproduce the committed file byte-for-byte. Pass a file to supply a
different head (its last line must be the `<body>` tag).

Markup conventions, reproduced from the hand-maintained file this replaced:
  * `⚠ ` in the .md becomes `<span class="difftag">DIFFERS</span> `
  * a table ROW carrying a ⚠ becomes `<tr class="warn">`
  * a `> ` blockquote becomes `<div class="callout">`
  * the italic lede paragraph becomes `<p class="lede">`
"""
import html
import pathlib
import re
import sys

MD = pathlib.Path("docs/cli-guide.md")
OUT = pathlib.Path("docs/cli-guide.html")

BODY_OPEN = "<body>"


def head_block() -> str:
    """The hand-maintained prelude, through the newline closing `<body>`.

    That trailing newline is part of the block on purpose: the emitter below
    joins head and body with one more, which is what puts the blank line the
    committed file has between `<body>` and the first heading. Slicing one byte
    earlier silently drops that line and the regenerated file stops matching.
    """
    source = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else OUT
    text = source.read_text()
    return text[: text.index(BODY_OPEN) + len(BODY_OPEN) + 1]


DIFF = '<span class="difftag">DIFFERS</span>'


def inline(text: str) -> str:
    """Markdown inline spans -> HTML, entity-escaping everything else.

    Code spans come out FIRST as placeholders so emphasis can be matched over
    the whole line: `**`code`**` is one bold run with a code span inside it, and
    substituting the code first would leave its two `**` markers in different
    pieces (which is exactly the bug the first version of this shipped).
    """
    spans: list[str] = []

    def stash(m):
        spans.append("<code>" + html.escape(m.group(1), quote=False) + "</code>")
        return f"\x00{len(spans) - 1}\x00"

    # DOUBLE-backtick spans first: markdown uses them for a code span whose own
    # content contains a backtick (`` …N more — type `log` ``), and a
    # single-backtick pattern shreds one into three.
    text = re.sub(r"``\s?(.+?)\s?``", stash, text)
    text = re.sub(r"`([^`]+)`", stash, text)
    text = _emph(html.escape(text, quote=False))
    text = re.sub(r"\x00(\d+)\x00", lambda m: spans[int(m.group(1))], text)
    return text.replace("⚠ ", DIFF + " ").replace("⚠", DIFF)


def _emph(text: str) -> str:
    text = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", text, flags=re.S)
    text = re.sub(r"(?<![\w*])\*([^*\n]+?)\*(?![\w*])", r"<em>\1</em>", text)
    return text


def convert(md: str) -> str:
    lines = md.split("\n")
    body: list[str] = []
    i = 0
    n = len(lines)
    while i < n:
        line = lines[i]

        if line.startswith("```"):
            i += 1
            block = []
            while i < n and not lines[i].startswith("```"):
                block.append(lines[i])
                i += 1
            i += 1
            code = html.escape("\n".join(block), quote=False)
            body.append("<pre><code>" + code + "</code></pre>")
            continue

        if line.startswith("# "):
            body.append("<h1>" + inline(line[2:]) + "</h1>")
            i += 1
            continue
        if line.startswith("## "):
            body.append("")
            body.append("<h2>" + inline(line[3:]) + "</h2>")
            i += 1
            continue
        if line.startswith("### "):
            body.append("")
            body.append("<h3>" + inline(line[4:]) + "</h3>")
            i += 1
            continue

        if line.startswith("|"):
            rows = []
            while i < n and lines[i].startswith("|"):
                rows.append(lines[i])
                i += 1
            body.append(table(rows))
            continue

        if line.startswith("> "):
            para = [line[2:]]
            i += 1
            while i < n and lines[i].startswith("> "):
                para.append(lines[i][2:])
                i += 1
            body.append('<div class="callout">' + inline(" ".join(para)) + "</div>")
            continue

        if line.startswith("- ") or re.match(r"^\d+\. ", line):
            ordered = bool(re.match(r"^\d+\. ", line))
            items = []
            while i < n and (lines[i].startswith("- ") or re.match(r"^\d+\. ", lines[i])):
                text = re.sub(r"^(?:- |\d+\. )", "", lines[i])
                items.append("  <li>" + inline(text) + "</li>")
                i += 1
            tag = "ol" if ordered else "ul"
            body.append(f"<{tag}>")
            body.extend(items)
            body.append(f"</{tag}>")
            continue

        if line.strip() == "---":
            i += 1
            continue

        if not line.strip():
            i += 1
            continue

        # A paragraph: one .md line, since this guide writes them that way.
        if line.startswith("*") and line.endswith("*") and not line.startswith("**"):
            body.append('<p class="lede">' + inline(line[1:-1]) + "</p>")
        else:
            body.append("<p>" + inline(line) + "</p>")
        i += 1

    return "\n".join(body)


def table(rows: list[str]) -> str:
    cells = [[c.strip() for c in r.strip().strip("|").split("|")] for r in rows]
    header, body_rows = cells[0], [r for r in cells[2:]]
    out = ["<table>"]
    out.append("  <tr>" + "".join("<th>" + inline(c) + "</th>" for c in header) + "</tr>")
    for row in body_rows:
        warn = any("⚠" in c for c in row)
        tr = '  <tr class="warn">' if warn else "  <tr>"
        out.append(tr + "".join("<td>" + inline(c) + "</td>" for c in row) + "</tr>")
    out.append("</table>")
    return "\n".join(out)


head = head_block()  # read BEFORE the write below clobbers the file it defaults to
OUT.write_text(head + "\n" + convert(MD.read_text()) + "\n\n</body>\n</html>\n")
print(f"wrote {OUT} ({len(OUT.read_text().splitlines())} lines)")
