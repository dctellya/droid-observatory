#!/usr/bin/env python3
"""Merge reports/staging/<feature>.md into the shared files (owner item 19, F-066, rule 27).

A staging file has the sections `## SPEC §9` (also `## SPEC §9 rows`, `## SPEC s9`), `## Decisions`, `## BACKLOG`,
`## Deferred clauses`, `## Library`. Every bullet and every table row of the first three is filed:
  - SPEC §9: each becomes a row F-NNN (next free number) in the table of SPEC.md section 9; a leading `Row (F-NEW):`
    marker is dropped.
  - BACKLOG: each becomes the next B-NNN row of BACKLOG.md (`B-NEW:` or `New:` marker optional; item and reason split
    at `reason:`, otherwise at the first sentence), except a closure (`Closed...`, `Close ...`, `B-NNN: done`), which
    is listed in the report.
  - Decisions: a bullet naming a D-NNN becomes an addendum section of docs/decisions/D-NNN-*.md (the text after the
    `D-NNN (...)` prefix); a bullet that says the decision is new or that there is no amendment, and a bullet naming no
    decision, is listed in the report.
  - Deferred clauses and Library: copied into the report part, per feature.
Prose paragraphs of the first three sections (other than "None.") are listed in the report. A bullet, table row or
paragraph under any other `##` section, or an addendum whose decision file does not exist, is unfiled: the script
prints `unfiled: N` with the items, exits 1 and writes nothing, so the staging file can be fixed and the run repeated.
The staging files are deleted after a successful merge. The report part (markdown) goes to --out, the counts and one
line per filed item to stdout. Files are merged in name order, so row numbers are reproducible.
--dry-run DIR copies SPEC.md, BACKLOG.md and the decisions directory into DIR, merges there, keeps the staging files
and writes the report to DIR/report.md; the repository files are never touched.
"""

import argparse
import glob
import os
import re
import shutil
import sys

SPEC, DECISIONS, BACKLOG, DEFERRED, LIBRARY = "spec", "decisions", "backlog", "deferred", "library"
NEW_ROW = re.compile(r"^(?:Row\s*)?\(?F-NEW\)?:?\s*", re.IGNORECASE)
NEW_BACKLOG = re.compile(r"^(?:B-NEW|New)\b:?\s*", re.IGNORECASE)
CLOSURE = re.compile(r"^(?:Closed?\b|B-\d+\s*:\s*(?:done|closed))", re.IGNORECASE)
NONE_TEXT = re.compile(r"(?:none|nothing to merge|none added)\.?", re.IGNORECASE)
DECISION_REF = re.compile(r"\bD-(\d{3})\b")
DECISION_PREFIX = re.compile(r"^D-\d{3}\s*(?:\([^)]*\)\s*:?|:)\s*")
NOT_AN_AMENDMENT = re.compile(r"\bis new\b|\bno amendment\b", re.IGNORECASE)
REASON = re.compile(r"[;.,]?\s*\b[Rr]eason:\s*")
BULLET = re.compile(r"^[-*] ")
TABLE_SEPARATOR = re.compile(r"^\|?[\s:|-]+\|[\s:|-]*$")
ID_CELL = re.compile(r"^(?:F|B)-NEW$", re.IGNORECASE)


def section_key(name):
    """Map a `## ` heading to a section key; an unknown heading keeps its name behind a `?`."""
    n = name.strip().lower()
    if n.startswith("spec") and re.search(r"(?:§|\bs)\s*9", n):
        return SPEC
    for key, prefix in ((DECISIONS, "decision"), (BACKLOG, "backlog"), (DEFERRED, "deferred"), (LIBRARY, "library")):
        if n.startswith(prefix):
            return key
    return "?" + name.strip()


def parse(path):
    """Return {section key: [lines]} of one staging file."""
    sections = {}
    current = None
    with open(path, encoding="utf-8") as fh:
        for line in fh.read().splitlines():
            if line.startswith("## "):
                current = section_key(line[3:])
                sections.setdefault(current, [])
            elif current is not None:
                sections[current].append(line)
    return sections


def split_cells(line):
    return [c.strip().replace("\\|", "|") for c in line.strip().strip("|").split("|")]


def table_items(rows):
    """Data rows of a markdown table as cell lists; the header and separator rows are dropped."""
    cells = [split_cells(r) for r in rows]
    if len(cells) >= 2 and TABLE_SEPARATOR.match(rows[1].strip()):
        cells = cells[2:]
    cells = [[c for c in row if c] for row in cells]
    return [row[1:] if row and ID_CELL.match(row[0]) else row for row in cells if row]


def items_and_paragraphs(lines):
    """Split section lines into items (text, cells) and prose paragraphs.

    A bullet runs to the next blank line or bullet; a table row is an item whose cells are kept.
    """
    items, paras = [], []
    bullet, para, table = None, [], []

    def end_para():
        if para:
            paras.append(" ".join(para))
            para.clear()

    def end_table():
        for cells in table_items(table):
            items.append((" — ".join(cells), cells))
        table.clear()

    for line in lines:
        text = line.strip()
        if text.startswith("|"):
            end_para()
            bullet = None
            table.append(text)
            continue
        end_table()
        if BULLET.match(line):
            end_para()
            bullet = [line[2:].strip()]
            items.append(bullet)
        elif bullet is not None and text:
            bullet.append(text)
        elif text:
            bullet = None
            para.append(text)
        else:
            bullet = None
            end_para()
    end_table()
    end_para()
    return [(" ".join(i), None) if isinstance(i, list) else i for i in items], paras


def is_none(text):
    return bool(NONE_TEXT.fullmatch(text.strip()))


def cell(text):
    return text.replace("|", "\\|")


def next_number(text, prefix):
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
        reason = first[1] if len(first) > 1 else "no reason stated"
    return item.rstrip(";,. "), reason


def prepare_dry_run(args):
    """Point the shared files at copies under the dry-run directory; the staging files are only read."""
    root = args.dry_run
    os.makedirs(root, exist_ok=True)
    spec, backlog = os.path.join(root, "SPEC.md"), os.path.join(root, "BACKLOG.md")
    decisions = os.path.join(root, "docs", "decisions")
    for copy, original in ((spec, args.spec), (backlog, args.backlog), (decisions, args.decisions)):
        if os.path.realpath(copy) == os.path.realpath(original):
            raise SystemExit("integrate-staging: --dry-run %s would overwrite %s" % (root, original))
    shutil.copyfile(args.spec, spec)
    shutil.copyfile(args.backlog, backlog)
    shutil.copytree(args.decisions, decisions, dirs_exist_ok=True)
    args.spec, args.backlog, args.decisions = spec, backlog, decisions
    args.keep = True
    args.out = args.out or os.path.join(root, "report.md")


class Plan:
    """Everything the staging files ask for, collected before anything is written."""

    def __init__(self, args, spec, backlog):
        self.args = args
        self.f_no, self.b_no = next_number(spec, "F"), next_number(backlog, "B")
        self.f_rows, self.b_rows, self.addenda = [], [], []
        self.notes, self.unfiled, self.summary, self.files = [], [], [], []

    def note(self, feature, kind, text):
        self.notes.append((feature, kind, text))

    def spec_items(self, feature, items, counts):
        for text, _cells in items:
            body = NEW_ROW.sub("", text)
            self.f_rows.append("| F-%03d | %s | %s | %s | done (%s integration) |"
                               % (self.f_no, self.args.date, cell(body), feature, self.args.milestone))
            self.summary.append("F-%03d  %s: %s" % (self.f_no, feature, body[:60]))
            self.f_no += 1
            counts["spec"] += 1

    def backlog_items(self, feature, items, counts):
        for text, cells in items:
            if CLOSURE.match(text):
                self.note(feature, "backlog closure", text)
                counts["closures"] += 1
                continue
            if cells is not None:
                item, reason = cells[0], (cells[1] if len(cells) > 1 else "no reason stated")
            else:
                item, reason = split_backlog(NEW_BACKLOG.sub("", text), feature)
            self.b_rows.append("| B-%03d | %s | %s (%s) | unscheduled |"
                               % (self.b_no, cell(item), cell(reason), feature))
            self.summary.append("B-%03d  %s: %s" % (self.b_no, feature, item[:60]))
            self.b_no += 1
            counts["backlog"] += 1

    def decision_items(self, feature, items, counts):
        for text, _cells in items:
            ref = DECISION_REF.search(text)
            if ref is None or NOT_AN_AMENDMENT.search(text):
                self.note(feature, "decision note", text)
                counts["notes"] += 1
                continue
            body = DECISION_PREFIX.sub("", text).strip().strip('"') or text
            found = sorted(glob.glob(os.path.join(self.args.decisions, "D-%s-*.md" % ref.group(1))))
            if not found:
                self.unfiled.append("%s (Decisions): D-%s has no file in %s: %s"
                                    % (feature, ref.group(1), self.args.decisions, text))
                continue
            self.addenda.append((found[0], ref.group(1), feature, body))
            counts["addenda"] += 1

    def add_file(self, feature, sec):
        counts = dict(spec=0, backlog=0, closures=0, addenda=0, notes=0, deferred=0, library=0)
        handlers = ((SPEC, self.spec_items), (BACKLOG, self.backlog_items), (DECISIONS, self.decision_items))
        for key, handler in handlers:
            items, paras = items_and_paragraphs(sec.get(key, []))
            handler(feature, items, counts)
            for para in paras:
                if not is_none(para):
                    self.note(feature, "prose", para)
        for key in (DEFERRED, LIBRARY):
            items, paras = items_and_paragraphs(sec.get(key, []))
            paras = [p for p in paras if not is_none(p)]
            counts[key] = len(items) + len(paras)
        for key, lines in sec.items():
            if key.startswith("?"):
                items, paras = items_and_paragraphs(lines)
                for text in [t for t, _c in items] + [p for p in paras if not is_none(p)]:
                    self.unfiled.append("%s (## %s): %s" % (feature, key[1:], text))
        self.files.append((feature, counts))

    def totals(self):
        names = ("spec", "backlog", "closures", "addenda", "notes", "deferred", "library")
        return {n: sum(c[n] for _f, c in self.files) for n in names}


def report_text(plan, parsed):
    out = ["### Staging files merged", ""]
    if not parsed:
        out += ["No staging files.", ""]
    else:
        out += ["| Feature | SPEC §9 rows | BACKLOG rows | Decision addenda | Closures | Decision notes |",
                "|---|---|---|---|---|---|"]
        out += ["| `%s` | %d | %d | %d | %d | %d |"
                % (f, c["spec"], c["backlog"], c["addenda"], c["closures"], c["notes"]) for f, c in plan.files]
        out.append("")
    if plan.notes:
        out += ["### Staging notes (closures, decision notes, prose)", ""]
        out += ["- `%s` (%s): %s" % note for note in plan.notes]
        out.append("")
    for key, title in ((DEFERRED, "Deferred clauses"), (LIBRARY, "Library notes for the orchestrator")):
        blocks = []
        for feature, _path, sec in parsed:
            body = "\n".join(sec.get(key, [])).strip()
            if body and not is_none(body):
                blocks.append("Feature `%s`:\n\n%s\n" % (feature, body))
        out += ["### " + title, ""]
        out += blocks if blocks else ["None.", ""]
        if blocks:
            out.append("")
    return re.sub(r"\n{3,}", "\n\n", "\n".join(out)).rstrip("\n") + "\n"


def write_changes(plan, spec, backlog, args):
    if plan.f_rows:
        with open(args.spec, "w", encoding="utf-8") as fh:
            fh.write(insert_after_last_row(spec, "F", plan.f_rows, "## 9."))
    if plan.b_rows:
        with open(args.backlog, "w", encoding="utf-8") as fh:
            fh.write(insert_after_last_row(backlog, "B", plan.b_rows))
    for path, num, feature, body in plan.addenda:
        with open(path, encoding="utf-8") as fh:
            text = fh.read().rstrip("\n")
        text += "\n\n## Addendum %s (%s integration, %s)\n\n%s\n" % (args.date, args.milestone, feature, body)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(text)
        plan.summary.append("D-%s  %s: addendum in %s" % (num, feature, os.path.basename(path)))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--milestone", required=True)
    ap.add_argument("--date", required=True)
    ap.add_argument("--staging", default="reports/staging")
    ap.add_argument("--spec", default="SPEC.md")
    ap.add_argument("--backlog", default="BACKLOG.md")
    ap.add_argument("--decisions", default="docs/decisions")
    ap.add_argument("--out", help="markdown report part (default with --dry-run: DIR/report.md)")
    ap.add_argument("--keep", action="store_true", help="do not delete the staging files")
    ap.add_argument("--dry-run", metavar="DIR", help="merge into a copy of the shared files under DIR")
    args = ap.parse_args()
    if args.dry_run:
        prepare_dry_run(args)
    elif not args.out:
        ap.error("--out is required without --dry-run")

    files = sorted(glob.glob(os.path.join(args.staging, "*.md")))
    parsed = [(os.path.splitext(os.path.basename(f))[0], f, parse(f)) for f in files]
    with open(args.spec, encoding="utf-8") as fh:
        spec = fh.read()
    with open(args.backlog, encoding="utf-8") as fh:
        backlog = fh.read()

    plan = Plan(args, spec, backlog)
    for feature, _path, sec in parsed:
        plan.add_file(feature, sec)

    if plan.unfiled:
        print("staging files read: %d, nothing written" % len(parsed))
        print("unfiled: %d" % len(plan.unfiled))
        for line in plan.unfiled:
            print("  " + line)
        return 1

    write_changes(plan, spec, backlog, args)
    with open(args.out, "w", encoding="utf-8") as fh:
        fh.write(report_text(plan, parsed))
    if not args.keep:
        for _f, path, _s in parsed:
            os.remove(path)

    total = plan.totals()
    prose = sum(1 for _f, kind, _t in plan.notes if kind == "prose")
    print("staging files merged: %d%s" % (len(parsed), " (dry run in %s)" % args.dry_run if args.dry_run else ""))
    for line in plan.summary:
        print(line)
    print("SPEC §9 rows: %d" % total["spec"])
    print("BACKLOG rows: %d" % total["backlog"])
    print("closures: %d" % total["closures"])
    print("decision addenda: %d" % total["addenda"])
    print("decision notes: %d" % total["notes"])
    print("prose notes: %d" % prose)
    print("deferred clauses: %d" % total["deferred"])
    print("library: %d" % total["library"])
    for feature, c in plan.files:
        print("file %s: spec=%d backlog=%d closures=%d addenda=%d notes=%d deferred=%d library=%d"
              % (feature, c["spec"], c["backlog"], c["closures"], c["addenda"], c["notes"], c["deferred"], c["library"]))
    print("unfiled: 0")
    return 0


if __name__ == "__main__":
    sys.exit(main())
