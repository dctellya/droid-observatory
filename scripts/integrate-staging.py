#!/usr/bin/env python3
"""Merge reports/staging/<feature>.md into the shared files (owner item 19, F-066, rule 27).

A staging file has the sections `## SPEC §9`, `## Decisions`, `## BACKLOG`, `## Deferred clauses`, `## Library`.
  - SPEC §9: every bullet becomes a row F-NNN (next free number) in the table of SPEC.md section 9.
  - Decisions: a bullet `D-NNN (amend...): "text"` becomes an addendum section of docs/decisions/D-NNN-*.md;
    other bullets are listed in the report only.
  - BACKLOG: a bullet starting `B-NEW` becomes the next B-NNN row of BACKLOG.md; other bullets are listed only.
  - Deferred clauses and Library: copied into the report part, per feature.
The staging files are deleted after a successful merge. The report part (markdown) goes to --out, a one-line-per-item
summary to stdout. Files are merged in name order, so row numbers are reproducible.
"""

import argparse
import glob
import os
import re
import sys

SECTION_NAMES = {
    "spec": "SPEC §9",
    "decisions": "Decisions",
    "backlog": "BACKLOG",
    "deferred": "Deferred clauses",
    "library": "Library",
}
NEW_ROW = re.compile(r"^(?:Row\s*)?\(?F-NEW\)?:?\s*", re.IGNORECASE)
NEW_BACKLOG = re.compile(r"^B-NEW:?\s*")
AMEND = re.compile(r"^D-(\d{3})\b[^:]*?\(amend[^)]*\)\s*:?\s*(.*)$", re.DOTALL)
REASON = re.compile(r"[;.,]?\s*\b[Rr]eason:\s*")


def parse(path):
    """Return {section key: [lines]} of one staging file."""
    sections = {}
    current = None
    with open(path, encoding="utf-8") as fh:
        for line in fh.read().splitlines():
            if line.startswith("## "):
                name = line[3:].strip()
                current = next((k for k, v in SECTION_NAMES.items() if v == name), None)
                if current is not None:
                    sections[current] = []
            elif current is not None:
                sections[current].append(line)
    return sections


def bullets(lines):
    """Split lines into (bullets, prose): a bullet runs to the next blank line or bullet."""
    items, prose, cur = [], [], None
    for line in lines:
        if line.startswith("- "):
            cur = [line[2:].strip()]
            items.append(cur)
        elif cur is not None and line.strip():
            cur.append(line.strip())
        else:
            cur = None
            if line.strip():
                prose.append(line.strip())
    return [" ".join(i) for i in items], prose


def cell(text):
    return text.replace("|", "\\|")


def next_number(text, prefix, width=3):
    nums = [int(n) for n in re.findall(r"^\| %s-(\d+) \|" % prefix, text, re.MULTILINE)]
    return (max(nums) if nums else 0) + 1


def insert_after_last_row(text, prefix, rows, start_marker=None):
    """Insert table rows after the last `| PREFIX-NNN |` line (inside the section that starts at start_marker)."""
    lines = text.split("\n")
    begin = 0
    if start_marker is not None:
        begin = next((i for i, l in enumerate(lines) if l.startswith(start_marker)), None)
        if begin is None:
            raise SystemExit("integrate-staging: %r not found" % start_marker)
    last = None
    for i in range(begin, len(lines)):
        if i > begin and start_marker is not None and lines[i].startswith("## "):
            break
        if re.match(r"^\| %s-\d+ \|" % prefix, lines[i]):
            last = i
    if last is None:
        raise SystemExit("integrate-staging: no %s table row found to extend" % prefix)
    lines[last + 1:last + 1] = rows
    return "\n".join(lines)


def split_backlog(text, feature):
    m = REASON.search(text)
    if m:
        item, reason = text[:m.start()].strip(), text[m.end():].strip()
    else:
        first = re.split(r"(?<=\.)\s+", text, maxsplit=1)
        item = first[0].rstrip(".")
        reason = first[1] if len(first) > 1 else "Filed by " + feature + "."
    return item.rstrip(";,. "), reason


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--milestone", required=True)
    ap.add_argument("--date", required=True)
    ap.add_argument("--staging", default="reports/staging")
    ap.add_argument("--spec", default="SPEC.md")
    ap.add_argument("--backlog", default="BACKLOG.md")
    ap.add_argument("--decisions", default="docs/decisions")
    ap.add_argument("--out", required=True, help="markdown report part")
    ap.add_argument("--keep", action="store_true", help="do not delete the staging files")
    args = ap.parse_args()

    files = sorted(glob.glob(os.path.join(args.staging, "*.md")))
    parsed = [(os.path.splitext(os.path.basename(f))[0], f, parse(f)) for f in files]

    with open(args.spec, encoding="utf-8") as fh:
        spec = fh.read()
    with open(args.backlog, encoding="utf-8") as fh:
        backlog = fh.read()
    f_no, b_no = next_number(spec, "F"), next_number(backlog, "B")

    f_rows, b_rows, amendments, notes, summary = [], [], [], [], []
    per_feature = []
    for feature, _path, sec in parsed:
        n_f = n_b = n_d = 0
        items, prose = bullets(sec.get("spec", []))
        for text in items:
            body = NEW_ROW.sub("", text)
            f_rows.append("| F-%03d | %s | %s | %s | done (%s integration) |"
                          % (f_no, args.date, cell(body), feature, args.milestone))
            summary.append("F-%03d  %s: %s" % (f_no, feature, body[:60]))
            f_no += 1
            n_f += 1
        items, prose = bullets(sec.get("decisions", []))
        for text in items:
            m = AMEND.match(text)
            body = m.group(2).strip().strip('"') if m else ""
            if m and body:
                amendments.append((m.group(1), feature, body))
                n_d += 1
            else:
                notes.append((feature, "decision", text))
        items, prose = bullets(sec.get("backlog", []))
        for text in items:
            if NEW_BACKLOG.match(text):
                item, reason = split_backlog(NEW_BACKLOG.sub("", text), feature)
                b_rows.append("| B-%03d | %s | %s (%s) | unscheduled |"
                              % (b_no, cell(item), cell(reason), feature))
                summary.append("B-%03d  %s: %s" % (b_no, feature, item[:60]))
                b_no += 1
                n_b += 1
            else:
                notes.append((feature, "backlog", text))
        per_feature.append((feature, n_f, n_b, n_d))

    # Resolve the decision files first: a missing file is reported, never a half-written merge.
    applied = []
    for num, feature, body in amendments:
        found = sorted(glob.glob(os.path.join(args.decisions, "D-%s-*.md" % num)))
        if not found:
            notes.append((feature, "decision D-%s (file not found, not applied)" % num, body))
            continue
        applied.append((found[0], num, feature, body))

    if f_rows:
        spec = insert_after_last_row(spec, "F", f_rows, "## 9.")
        with open(args.spec, "w", encoding="utf-8") as fh:
            fh.write(spec)
    if b_rows:
        backlog = insert_after_last_row(backlog, "B", b_rows)
        with open(args.backlog, "w", encoding="utf-8") as fh:
            fh.write(backlog)
    for path, num, feature, body in applied:
        with open(path, encoding="utf-8") as fh:
            text = fh.read().rstrip("\n")
        text += "\n\n## Addendum %s (%s integration, %s)\n\n%s\n" % (args.date, args.milestone, feature, body)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(text)
        summary.append("D-%s  %s: addendum in %s" % (num, feature, os.path.basename(path)))

    out = ["### Staging files merged", ""]
    if not parsed:
        out += ["No staging files.", ""]
    else:
        out += ["| Feature | SPEC §9 rows | BACKLOG rows | Decision addenda |", "|---|---|---|---|"]
        out += ["| `%s` | %d | %d | %d |" % row for row in per_feature]
        out.append("")
    if notes:
        out += ["### Staging bullets not filed", ""]
        out += ["- `%s` (%s): %s" % (feat, kind, text) for feat, kind, text in notes]
        out.append("")
    for key, title in (("deferred", "Deferred clauses"), ("library", "Library notes for the orchestrator")):
        blocks = []
        for feature, _path, sec in parsed:
            body = "\n".join(sec.get(key, [])).strip()
            if body and body.lower().rstrip(".") not in ("none", "nothing to merge"):
                blocks.append("Feature `%s`:\n\n%s\n" % (feature, body))
        out += ["### " + title, ""]
        out += blocks if blocks else ["None.", ""]
        if blocks:
            out.append("")
    with open(args.out, "w", encoding="utf-8") as fh:
        fh.write(re.sub(r"\n{3,}", "\n\n", "\n".join(out)).rstrip("\n") + "\n")

    if not args.keep:
        for _f, path, _s in parsed:
            os.remove(path)
    print("staging files merged: %d" % len(parsed))
    for line in summary:
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
