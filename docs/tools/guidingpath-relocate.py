#!/usr/bin/env python3
"""Relocate line ranges of `guidingpath.md` into archive files VERBATIM, and prove it byte for byte.

    python3 docs/tools/guidingpath-relocate.py --self-test
    python3 docs/tools/guidingpath-relocate.py write --plan PLAN.json [--root DIR] [--source PATH] [--dry-run] [--emit-map OUT]
    python3 docs/tools/guidingpath-relocate.py check [--map 2026-09-30] [--root DIR] [--at REV] [--source PATH]

The method of the two earlier relocations (`docs/plans/2026-08-19-docs-consolidation.md` and
`docs/plans/2026-09-23-claude-md-consolidation.md`), committed as a tool at the third
(`docs/plans/2026-09-30-guidingpath-consolidation.md`; R-6.5c2.32), so the byte-check can be re-run.

A PLAN names each output file as an ordered list of pieces:

    {"lines": [a, b]}    source lines a..b (1-based, inclusive), placed VERBATIM
    {"text": "..."}      new text: whole lines, each ending in a newline
    {"text_file": "p"}   new text read from p, relative to the plan file (write mode)
    {"new": n}           n lines of new text whose words the check does not read (a committed map)

The live file is the source's own path, `guidingpath.md`. A plan that does not list it gets the
source with every placed range taken out and nothing inserted, so the empty plan `{"files": {}}` is
the identity.

`write` assembles every file from the source (`--root`'s `guidingpath.md`, or `--source`, which a
re-run needs once the live file is rewritten), refuses BEFORE writing anything when the
plan does not place each line exactly once, writes (unless `--dry-run`), reads every file BACK and
checks what it read. `--emit-map` also writes the plan with each new text replaced by its line
count, the shape `MAPS` holds. `check` runs the same checks on files already written: a built-in
map, the source read from the map's base commit with `git show`, the files from the tree or from
`--at REV`.

The checks, each exiting 1 with a message that names the first line that fails:
  1. every piece is well formed and every range lies inside the source;
  2. every source line is placed exactly once across the plan's files: an overlap names the line
     placed twice, a gap the line placed nowhere;
  3. the live file's ranges ascend, so its kept lines stay in source order; an archive's ranges keep
     the order its plan declares, each range in source order within itself;
  4. each file, read back, is its pieces in order: every placed line byte-identical to its source
     line at the stated position, every new line where the plan puts it, and nothing past the end;
  5. every moved line that is unique and non-blank in the source appears exactly once as a whole
     line across the plan's files, new text included, so a moved line also left behind is caught;
  6. the live file with its new lines taken out is the source with the moved ranges taken out, byte
     for byte: no other line of the live file changed. Check 4 already proves this, given checks 2
     and 3, so check 6 can never be the one that fails; it states the brief's words as an assertion.
On success it prints the check line `N lines moved, byte-identical; M lines kept; I lines inserted`.
I counts the live file's new lines, so `wc -l` before = (the live file's lines after - I) + N.
"""
from __future__ import annotations

import argparse
import contextlib
import hashlib
import io
import json
import os
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
LIVE = "guidingpath.md"

# The committed maps: each relocation's plan with its new text replaced by line counts, and the
# commit whose `guidingpath.md` was its source. `check` re-runs a relocation's byte-check from these.
MAPS: dict[str, dict] = {
    # R-6.5c2.32, M6.5c2 cycle E: the archive by grouping (docs/plans/2026-09-30-guidingpath-consolidation.md).
    "2026-09-30": {
        "base": "19f3feb1",
        "files": {
            "guidingpath.md": [
                {"lines": [1, 17]}, {"new": 2}, {"lines": [18, 20]}, {"new": 14}, {"lines": [37, 38]},
                {"lines": [63, 64]}, {"new": 27}, {"lines": [137, 141]}, {"new": 9}, {"lines": [142, 143]},
                {"lines": [232, 232]},
            ],
            "guidingpath-archive-phase3.md": [
                {"new": 17}, {"lines": [39, 62]}, {"new": 2}, {"lines": [65, 136]}, {"new": 5},
                {"lines": [21, 36]}, {"new": 6}, {"lines": [144, 204]},
            ],
            "guidingpath-archive-web.md": [{"new": 16}, {"lines": [205, 231]}],
        },
    },
}


class Fail(Exception):
    """A check that failed; its message names the first line that fails."""


def split_lines(data: bytes) -> list[bytes]:
    """Split into lines, each keeping its newline; a last line without one is kept as it is."""
    parts = data.split(b"\n")
    lines = [p + b"\n" for p in parts[:-1]]
    if parts[-1]:
        lines.append(parts[-1])
    return lines


def _kind(piece: object, where: str) -> str:
    if not isinstance(piece, dict) or len(piece) != 1:
        raise Fail(f"{where}: a piece is an object with exactly one key, not {piece!r}")
    (k,) = piece
    if k not in ("lines", "text", "new"):
        raise Fail(f"{where}: unknown piece {k!r} (lines | text | text_file | new)")
    return k


def _range(piece: dict, n: int, where: str) -> tuple[int, int]:
    r = piece["lines"]
    if not (isinstance(r, list) and len(r) == 2 and all(type(x) is int for x in r)):
        raise Fail(f"{where}: a range is [first, last], not {r!r}")
    a, b = r
    if not 1 <= a <= b <= n:
        raise Fail(f"{where}: range [{a}, {b}] is not inside the source's lines 1..{n}")
    return a, b


def _text_lines(piece: dict, where: str) -> list[bytes]:
    text = piece["text"]
    if not isinstance(text, str) or (text and not text.endswith("\n")):
        raise Fail(f"{where}: new text is whole lines, each ending in a newline")
    return split_lines(text.encode("utf-8"))


def files_of(src: list[bytes], plan: dict) -> dict[str, list]:
    """The plan's files in its order, the live file derived when the plan does not list it."""
    declared = plan.get("files")
    if not isinstance(declared, dict):
        raise Fail('a plan is an object with a "files" object')
    files = {path: list(pieces) for path, pieces in declared.items()}
    if LIVE not in files:
        placed: set[int] = set()
        for path, pieces in files.items():
            for p in pieces:
                if _kind(p, path) == "lines":
                    a, b = _range(p, len(src), path)
                    placed.update(range(a, b + 1))
        runs: list[dict] = []
        for i in range(1, len(src) + 1):
            if i in placed:
                continue
            if runs and runs[-1]["lines"][1] == i - 1:
                runs[-1]["lines"][1] = i
            else:
                runs.append({"lines": [i, i]})
        files[LIVE] = runs
    return files


def preflight(src: list[bytes], plan: dict) -> dict[str, list]:
    """Checks 1-3, which read no output file: shapes, exactly-once placement, the live order."""
    n = len(src)
    files = files_of(src, plan)
    owner: dict[int, str] = {}
    for path, pieces in files.items():
        for p in pieces:
            k = _kind(p, path)
            if k == "lines":
                a, b = _range(p, n, path)
                for i in range(a, b + 1):
                    if i in owner:
                        raise Fail(f"source line {i} is placed twice: in {owner[i]} and in {path}")
                    owner[i] = path
            elif k == "text":
                _text_lines(p, path)
            elif not (type(p["new"]) is int and p["new"] >= 0):
                raise Fail(f"{path}: a new piece counts lines, not {p!r}")
    for i in range(1, n + 1):
        if i not in owner:
            raise Fail(f"source line {i} is placed nowhere")
    last = 0
    for p in files[LIVE]:
        if "lines" in p:
            a, b = p["lines"]
            if a <= last:
                raise Fail(
                    f"{LIVE}: range [{a}, {b}] is out of order after source line {last}; "
                    "the live file keeps source order"
                )
            last = b
    return files


def assemble(src: list[bytes], plan: dict) -> dict[str, bytes]:
    """Every file's bytes, from the source and the plan. Refuses before building anything."""
    files = preflight(src, plan)
    out: dict[str, bytes] = {}
    for path, pieces in files.items():
        chunks: list[bytes] = []
        for p in pieces:
            k = _kind(p, path)
            if k == "lines":
                a, b = p["lines"]
                chunks.extend(src[a - 1 : b])
            elif k == "text":
                chunks.extend(_text_lines(p, path))
            else:
                raise Fail(f"{path}: a plan to write carries its new text, not a line count")
        out[path] = b"".join(chunks)
    return out


def check(src: list[bytes], plan: dict, written: dict[str, bytes]) -> str:
    """Checks 1-6 over files already written; returns the check line and a line per file.

    Read from the TREE, a live file that has grown since the relocation fails check 4 with "a line
    past the plan's end": expected once the tree has moved on after the relocation. The proof is
    `check --at 054b1e4f`, which reads the files at the relocation's own commit.
    """
    files = preflight(src, plan)
    moved = kept = inserted = 0
    report: list[str] = []
    live_new: set[int] = set()  # 0-based indices of the live file's new lines
    for path, pieces in files.items():
        if path not in written:
            raise Fail(f"{path}: the plan names it and it was not read")
        got = split_lines(written[path])
        pos = 0
        placed_here = new_here = 0
        for p in pieces:
            k = _kind(p, path)
            if k == "lines":
                a, b = p["lines"]
                for i in range(a, b + 1):
                    if pos >= len(got):
                        raise Fail(f"{path}:{pos + 1}: expected source line {i} verbatim, found the end of the file")
                    if got[pos] != src[i - 1]:
                        raise Fail(f"{path}:{pos + 1}: expected source line {i} verbatim, found a different line")
                    pos += 1
                placed_here += b - a + 1
            else:
                if k == "text":
                    want = _text_lines(p, path)
                    for j, w in enumerate(want):
                        if pos + j >= len(got) or got[pos + j] != w:
                            raise Fail(f"{path}:{pos + j + 1}: expected the plan's new line {j + 1} of this piece")
                    count = len(want)
                else:
                    count = p["new"]
                    if pos + count > len(got):
                        raise Fail(f"{path}:{len(got) + 1}: expected {count} new lines from line {pos + 1}, found the end of the file")
                if path == LIVE:
                    live_new.update(range(pos, pos + count))
                pos += count
                new_here += count
        if pos != len(got):
            raise Fail(
                f"{path}:{pos + 1}: a line past the plan's end — expected once the tree has moved on "
                "after the relocation; the proof is `check --at 054b1e4f`"
            )
        if path == LIVE:
            kept, inserted = placed_here, new_here
        else:
            moved += placed_here
        role = "kept" if path == LIVE else "moved"
        report.append(f"  {path}: {len(got)} lines = {placed_here} {role} + {new_here} new")

    # Check 5: a moved line appears exactly once, wherever the plan's files are read.
    in_source: dict[bytes, int] = {}
    for line in src:
        in_source[line] = in_source.get(line, 0) + 1
    everywhere: dict[bytes, int] = {}
    for path in files:
        for line in split_lines(written[path]):
            everywhere[line] = everywhere.get(line, 0) + 1
    moved_lines: set[int] = set()
    for path, pieces in files.items():
        if path == LIVE:
            continue
        for p in pieces:
            if "lines" in p:
                a, b = p["lines"]
                moved_lines.update(range(a, b + 1))
    for i in sorted(moved_lines):
        line = src[i - 1]
        if line.strip() and in_source[line] == 1 and everywhere.get(line, 0) != 1:
            raise Fail(
                f"source line {i} appears {everywhere.get(line, 0)} times across the plan's files; "
                "a moved line appears exactly once"
            )

    # Check 6: the live file without its new lines is the source without its moved lines. Checks 2-4
    # already imply it, so it never fails first; it asserts the brief's words ("no other line changed").
    live_got = split_lines(written[LIVE])
    remainder = [line for k, line in enumerate(live_got) if k not in live_new]
    expected = [src[i - 1] for i in range(1, len(src) + 1) if i not in moved_lines]
    for k, (g, e) in enumerate(zip(remainder, expected)):
        if g != e:
            raise Fail(f"{LIVE}: kept line {k + 1} differs from the source's")
    if len(remainder) != len(expected):
        raise Fail(f"{LIVE}: {len(remainder)} kept lines where the source leaves {len(expected)}")

    head = f"CHECK  {moved} lines moved, byte-identical; {kept} lines kept; {inserted} lines inserted"
    return "\n".join([head, f"  source: {len(src)} lines", *report])


def opaque(plan: dict) -> dict:
    """The plan with each new text replaced by its line count: the shape `MAPS` holds."""
    out = json.loads(json.dumps(plan))
    for pieces in out["files"].values():
        for i, p in enumerate(pieces):
            if isinstance(p, dict) and "text" in p:
                pieces[i] = {"new": len(split_lines(p["text"].encode("utf-8")))}
    return out


def _git_show(root: pathlib.Path, rev: str, path: str) -> bytes:
    run = subprocess.run(["git", "-C", str(root), "show", f"{rev}:{path}"], capture_output=True)
    if run.returncode != 0:
        raise Fail(f"git show {rev}:{path} failed: {run.stderr.decode(errors='replace').strip()}")
    return run.stdout


def _load_plan(args: argparse.Namespace) -> dict:
    if args.plan_json is not None:
        plan, base = json.loads(args.plan_json), pathlib.Path.cwd()
    else:
        path = pathlib.Path(args.plan)
        plan, base = json.loads(path.read_text(encoding="utf-8")), path.parent
    for pieces in (plan.get("files") or {}).values():
        for i, p in enumerate(pieces):
            if isinstance(p, dict) and set(p) == {"text_file"}:
                pieces[i] = {"text": (base / p["text_file"]).read_text(encoding="utf-8")}
    return plan


def cmd_write(args: argparse.Namespace) -> int:
    root = pathlib.Path(args.root)
    src = split_lines(pathlib.Path(args.source).read_bytes() if args.source else (root / LIVE).read_bytes())
    plan = _load_plan(args)
    built = assemble(src, plan)  # checks 1-3 run first: a refused plan writes nothing
    if args.dry_run:
        readback = built
    else:
        for path, data in built.items():
            (root / path).write_bytes(data)
        readback = {path: (root / path).read_bytes() for path in built}
    print(check(src, plan, readback))
    if args.emit_map:
        pathlib.Path(args.emit_map).write_text(json.dumps(opaque(plan), indent=1) + "\n", encoding="utf-8")
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    if args.map not in MAPS:
        raise Fail(f"no built-in map {args.map!r}; the maps are {sorted(MAPS)}")
    m = MAPS[args.map]
    root = pathlib.Path(args.root)
    src_bytes = pathlib.Path(args.source).read_bytes() if args.source else _git_show(root, m["base"], LIVE)
    src = split_lines(src_bytes)
    plan = {"files": m["files"]}
    written = {}
    for path in files_of(src, plan):
        written[path] = _git_show(root, args.at, path) if args.at else (root / path).read_bytes()
    where = args.source or f"{LIVE} at {m['base']}"
    print(f"map {args.map}: source {where}; files {'at ' + args.at if args.at else 'in the tree'}")
    print(check(src, plan, written))
    return 0


def main(argv: list[str]) -> int:
    if argv == ["--self-test"]:
        return self_test()
    parser = argparse.ArgumentParser(prog="guidingpath-relocate.py", description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    w = sub.add_parser("write", help="assemble, write, read back and check")
    given = w.add_mutually_exclusive_group(required=True)
    given.add_argument("--plan", help="the plan, a JSON file")
    given.add_argument("--plan-json", help="the plan, inline")
    w.add_argument("--root", default=str(ROOT))
    w.add_argument("--source", help="read the source from this path rather than --root's guidingpath.md")
    w.add_argument("--dry-run", action="store_true", help="assemble and check; write nothing")
    w.add_argument("--emit-map", help="also write the plan with its new text as line counts")
    c = sub.add_parser("check", help="check files already written against a built-in map")
    c.add_argument("--map", default="2026-09-30")
    c.add_argument("--root", default=str(ROOT))
    c.add_argument("--at", help="read the files at this commit rather than from the tree")
    c.add_argument("--source", help="read the source from this path rather than the map's base commit")
    try:
        args = parser.parse_args(argv)
    except SystemExit as e:
        return int(e.code or 0)
    try:
        return cmd_write(args) if args.cmd == "write" else cmd_check(args)
    except Fail as e:
        print(f"FAIL  {e}")
        return 1
    except (OSError, json.JSONDecodeError) as e:
        print(f"FAIL  {type(e).__name__}: {e}")
        return 1


# ----------------------------------------------------------------------------------------------
# The self-test. The two cases the brief names run the CLI itself against the tree, and the
# non-identity dry run runs it in a directory of its own; the rest run the core on a synthetic
# source, so a check that passes vacuously is caught by its own case.
# ----------------------------------------------------------------------------------------------

SYNTH = b"".join(b"line %d\n" % i for i in range(1, 11))  # ten distinct lines


def _run(argv: list[str]) -> tuple[int, str]:
    out = io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
        code = main(argv)
    return code, out.getvalue()


def _tree_snapshot(root: pathlib.Path) -> dict[str, str]:
    snap = {}
    for name in sorted(os.listdir(root)):
        p = root / name
        snap[name] = hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else "dir"
    return snap


def _expect_fail(fn, needle: str) -> None:
    try:
        fn()
    except Fail as e:
        if needle not in str(e):
            raise AssertionError(f"failed, but the message {str(e)!r} does not name {needle!r}")
        return
    raise AssertionError(f"expected a failure naming {needle!r}; the check passed")


def st_empty_map_dry_run_changes_nothing() -> None:
    before = _tree_snapshot(ROOT)
    code, out = _run(["write", "--plan-json", '{"files": {}}', "--root", str(ROOT), "--dry-run"])
    assert code == 0, f"exit {code}: {out}"
    assert "0 lines moved, byte-identical" in out, out
    assert _tree_snapshot(ROOT) == before, "the dry run changed a file at the root"


def st_overlapping_map_exits_1() -> None:
    before = _tree_snapshot(ROOT)
    plan = {"files": {"x-archive.md": [{"lines": [2, 4]}], "y-archive.md": [{"lines": [4, 6]}]}}
    code, out = _run(["write", "--plan-json", json.dumps(plan), "--root", str(ROOT)])
    assert code == 1, f"exit {code}: {out}"
    assert "source line 4 is placed twice" in out, out
    assert _tree_snapshot(ROOT) == before, "a refused plan wrote a file"


def st_non_identity_dry_run_writes_nothing() -> None:
    # The empty map is the identity, so it cannot see --dry-run ignored: this plan would change files.
    # It runs in a directory of its own, so a broken --dry-run never reaches the tree.
    d = pathlib.Path(tempfile.mkdtemp(prefix="guidingpath-relocate-selftest-"))
    (d / LIVE).write_bytes(SYNTH)
    before = _tree_snapshot(d)
    plan = {"files": {"x-archive.md": [{"text": "# x\n"}, {"lines": [1, 3]}]}}
    code, out = _run(["write", "--plan-json", json.dumps(plan), "--root", str(d), "--dry-run"])
    assert code == 0, f"exit {code}: {out}"
    assert "3 lines moved, byte-identical; 7 lines kept" in out, out
    after = _tree_snapshot(d)
    assert after == before, f"--dry-run wrote into {d} (left in place for a look): {sorted(after)}"
    (d / LIVE).unlink()
    d.rmdir()


def st_range_past_the_end_is_a_named_refusal() -> None:
    src = split_lines(SYNTH)
    plan = {"files": {"a.md": [{"lines": [9, 11]}]}}
    _expect_fail(lambda: assemble(src, plan), "range [9, 11] is not inside the source's lines 1..10")


def st_changed_new_text_is_refused() -> None:
    src = split_lines(SYNTH)
    plan = _round_trip_plan()
    files = assemble(src, plan)
    files["arch-1.md"] = files["arch-1.md"].replace(b"mid\n", b"mud\n")
    _expect_fail(lambda: check(src, plan, files), "arch-1.md:5: expected the plan's new line 1")


def _round_trip_plan() -> dict:
    return {
        "files": {
            LIVE: [{"lines": [1, 2]}, {"text": "new a\n"}, {"lines": [6, 7]}, {"lines": [10, 10]}],
            "arch-1.md": [{"text": "# header\n\n"}, {"lines": [8, 9]}, {"text": "mid\n"}, {"lines": [3, 5]}],
        }
    }


def st_round_trip_counts_and_arithmetic() -> None:
    src = split_lines(SYNTH)
    plan = _round_trip_plan()
    files = assemble(src, plan)
    line = check(src, plan, files)
    assert "5 lines moved, byte-identical; 5 lines kept; 1 lines inserted" in line, line
    live_after = len(split_lines(files[LIVE]))
    assert len(src) == (live_after - 1) + 5, "wc -l before != (live after - inserted) + moved"


def st_gap_is_refused() -> None:
    src = split_lines(SYNTH)
    plan = {"files": {LIVE: [{"lines": [1, 4]}, {"lines": [6, 10]}]}}
    _expect_fail(lambda: check(src, plan, assemble(src, plan)), "source line 5 is placed nowhere")


def st_live_out_of_order_is_refused() -> None:
    src = split_lines(SYNTH)
    plan = {"files": {LIVE: [{"lines": [6, 10]}, {"lines": [1, 5]}]}}
    _expect_fail(lambda: check(src, plan, assemble(src, plan)), "out of order")


def st_tampered_moved_line_is_named() -> None:
    src = split_lines(SYNTH)
    plan = _round_trip_plan()
    files = assemble(src, plan)
    files["arch-1.md"] = files["arch-1.md"].replace(b"line 4\n", b"line 4 \n")
    _expect_fail(lambda: check(src, plan, files), "arch-1.md:7: expected source line 4")


def st_moved_line_left_in_new_text_is_refused() -> None:
    src = split_lines(SYNTH)
    plan = _round_trip_plan()
    plan["files"][LIVE][1] = {"text": "line 8\n"}
    _expect_fail(lambda: check(src, plan, assemble(src, plan)), "source line 8 appears 2 times")


def st_extra_line_past_the_plan_is_refused() -> None:
    src = split_lines(SYNTH)
    plan = _round_trip_plan()
    files = assemble(src, plan)
    files[LIVE] += b"stray\n"
    _expect_fail(lambda: check(src, plan, files), "past the plan's end")


def st_opaque_new_lines_check_like_text() -> None:
    src = split_lines(SYNTH)
    plan = _round_trip_plan()
    files = assemble(src, plan)
    opaque = json.loads(json.dumps(plan))
    for pieces in opaque["files"].values():
        for i, p in enumerate(pieces):
            if "text" in p:
                pieces[i] = {"new": len(split_lines(p["text"].encode()))}
    assert "5 lines moved" in check(src, opaque, files)


SELF_TESTS = [
    st_empty_map_dry_run_changes_nothing,
    st_overlapping_map_exits_1,
    st_non_identity_dry_run_writes_nothing,
    st_range_past_the_end_is_a_named_refusal,
    st_changed_new_text_is_refused,
    st_round_trip_counts_and_arithmetic,
    st_gap_is_refused,
    st_live_out_of_order_is_refused,
    st_tampered_moved_line_is_named,
    st_moved_line_left_in_new_text_is_refused,
    st_extra_line_past_the_plan_is_refused,
    st_opaque_new_lines_check_like_text,
]


def self_test() -> int:
    failed = 0
    for t in SELF_TESTS:
        try:
            t()
            print(f"ok    {t.__name__}")
        except Exception as e:  # noqa: BLE001 - a self-test reports every failure by name
            failed += 1
            print(f"FAIL  {t.__name__}: {type(e).__name__}: {e}")
    print(f"self-test: {len(SELF_TESTS) - failed} passed, {failed} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
