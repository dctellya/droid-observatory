#!/usr/bin/env python3
"""make check-kit: the UI-kit lint (owner item 14b, second half; VAL-UI-001).

Reads apps/observatory-app/ui (src/*.rs, style.css, index.html, theme.css) against ai/design/ui-kit-v2/ and
apps/observatory-app/kit-departures.md, and prints one line `check-kit ok|FAIL VAL-UI-001 ...` (reasons on stderr).

Rules (kit README "Eight rules"):
  1 tokens only      theme.css equals the kit's; no literal colour, no font-family outside theme.css; no inline
                     padding, margin, gap, font or width style
  2 two faces        text that is an identifier, a timestamp or a number sits in an element whose own or inherited
                     classes carry the mono face (a class that sets `font-family: var(--mono)` or `tabular-nums` in
                     theme.css or style.css); a number-shaped table cell also carries `n` (right-aligned)
  3, 6 spacing/scale style.css padding, margin and gap on a kit selector, td/th padding, max-width, inset and a font
                     size off the kit scale are departures: each is a row of kit-departures.md (selector, property,
                     kit value, actual, reason, SPEC row) and each row must still be true of style.css
Known violations of rule 2 that a later feature fixes live in apps/observatory-app/kit-pending.txt; the lint is green
while the list is exact (a fixed entry or a new violation turns it red).
"""

import os
import re
import sys

CHECK = "check-kit"
IDS = "VAL-UI-001"
KIT_SCALE_PX = {"11px", "12px", "14px", "15px", "22px"}
KIT_PAGE_MAX_WIDTH = "1180px"
SPACING_PROPS = ("padding", "margin", "gap", "row-gap", "column-gap")
GEOMETRY_PROPS = ("max-width", "inset")
# Words of a Rust expression that name an identifier, a timestamp or a number (the rt3 scan's three shapes).
ID_WORDS = {"id", "ids", "sha", "kind", "model", "models", "feature", "milestone", "date", "timestamp", "version",
            "mapping_version", "hash", "uuid", "session_id", "event_id", "request_id", "to_rfc3339_millis"}
NUM_WORDS = {"count", "total", "len", "tokens", "input", "output", "cache_read", "cache_write", "turns", "sessions",
             "markers", "duration_ms", "ms", "usd", "cost", "minutes", "idle_minutes", "elapsed_ms", "secs", "seconds",
             "events", "event_count", "turn_input", "turn_output", "turn_cache_read", "pr_number", "percent", "unknown",
             "tray_unknown", "pid", "masked"}
NUM_SUFFIXES = ("_count", "_total", "_ms", "_minutes")
CONTAINER_TAGS = {"tbody", "thead", "section", "main", "ul", "ol", "tr", "form", "article", "header", "nav", "aside"}
UNIT_ONLY = re.compile(r"^[\s\d.,:%+-]*(ms|s|min|usd|USD-eq|tok|k)?$")
# rt3 scan.js: literal text that is a number, a dotted/dashed/underscored id, an `m<digits>` milestone or a uuid.
LITERAL_SHAPE = re.compile(
    r"^(\d[\d,.:\- TZ%]*|[a-z0-9]+(?:[-_.:][a-z0-9.]+){1,}|m\d|[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}.*)$", re.I)
LITERAL_WORDS = {"as declared", "drift", "User testing"}
FORM_TAGS = {"button", "input", "select", "option", "label", "textarea"}
LITERAL_COLOUR = re.compile(r"#[0-9a-fA-F]{3}\b|#[0-9a-fA-F]{6}\b|rgb\(", re.I)


def read(path):
    with open(path, encoding="utf-8") as handle:
        return handle.read()


def line_of(text, offset):
    return text.count("\n", 0, offset) + 1


class Report:
    def __init__(self):
        self.fails = []
        self.notes = []

    def fail(self, message):
        self.fails.append(message)


# ---------------------------------------------------------------- CSS


def strip_comments(css):
    return re.sub(r"/\*.*?\*/", lambda m: re.sub(r"[^\n]", " ", m.group(0)), css, flags=re.S)


def parse_css(css):
    """Flat list of (selectors, {property: (value, line)}) with @media blocks flattened."""
    css = strip_comments(css)
    rules = []

    def block(start, end):
        i = start
        while i < end:
            brace = css.find("{", i, end)
            if brace < 0:
                return
            depth, j = 1, brace + 1
            while j < end and depth:
                depth += (css[j] == "{") - (css[j] == "}")
                j += 1
            head = css[i:brace]
            if head.strip().startswith("@"):
                block(brace + 1, j - 1)
            else:
                decls, offset = {}, brace + 1
                for part in css[brace + 1:j - 1].split(";"):
                    if ":" in part:
                        prop, value = part.split(":", 1)
                        decls[prop.strip()] = (" ".join(value.split()), line_of(css, offset + len(part) - len(part.lstrip())))
                    offset += len(part) + 1
                selectors = [" ".join(sel.split()) for sel in head.split(",") if sel.strip()]
                rules.append((selectors, decls, line_of(css, i + len(head) - len(head.lstrip()))))
            i = j

    block(0, len(css))
    return rules


def kit_values(kit_rules):
    values = {}
    for selectors, decls, _ in kit_rules:
        for selector in selectors:
            for prop, (value, _line) in decls.items():
                values.setdefault((selector, prop), value)
    return values


def compound_tail(selector):
    return selector.split()[-1] if selector.split() else selector


def style_departures(style_rules, kit):
    """Every (selector group, property) of style.css that departs from the kit: {(selector, prop): (kit, actual, line)}."""
    found = {}
    for selectors, decls, _ in style_rules:
        group = ", ".join(selectors)
        for prop, (actual, line) in decls.items():
            kit_value = None
            if prop in GEOMETRY_PROPS:
                kit_value = KIT_PAGE_MAX_WIDTH if prop == "max-width" else "none"
                if actual != kit_value:
                    found[(group, prop)] = (kit_value, actual, line)
                continue
            if prop == "font-size":
                if actual not in KIT_SCALE_PX:
                    found[(group, prop)] = ("scale 11, 12, 14, 15, 22 px", actual, line)
                continue
            if not (prop in SPACING_PROPS or prop.startswith(("padding-", "margin-"))):
                continue
            for selector in selectors:
                if compound_tail(selector) in ("th", "td") and prop == "padding":
                    kit_value = kit.get((".lib td", "padding"))
                else:
                    kit_value = kit.get((selector, prop))
                if kit_value is not None and kit_value != actual:
                    found[(group, prop)] = (kit_value, actual, line)
                    break
    return found


def parse_departures(markdown):
    rows = {}
    for number, line in enumerate(markdown.splitlines(), 1):
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) != 6 or cells[0].lower() == "selector" or set(cells[0]) <= set("-: "):
            continue
        selector, prop, kit_value, actual, reason, spec = (c.strip("`").strip() if i < 4 else c for i, c in enumerate(cells))
        rows[(selector, prop)] = (kit_value, actual, reason, spec, number)
    return rows


def check_departures(report, style_rules, kit_rules, departures_md):
    found = style_departures(style_rules, kit_values(kit_rules))
    rows = parse_departures(departures_md)
    for key, (kit_value, actual, line) in sorted(found.items(), key=lambda kv: kv[1][2]):
        row = rows.get(key)
        if row is None:
            report.fail(f"style.css:{line} `{key[0]}` {key[1]}: {actual} departs from the kit ({kit_value}) and is not "
                        f"a row of kit-departures.md")
            continue
        if row[1] != actual:
            report.fail(f"kit-departures.md:{row[4]} `{key[0]}` {key[1]}: row says {row[1]}, style.css:{line} says {actual}")
        if row[0] != kit_value:
            report.fail(f"kit-departures.md:{row[4]} `{key[0]}` {key[1]}: row says kit value {row[0]}, the kit has {kit_value}")
    pending_spec = 0
    for key, (kit_value, actual, reason, spec, number) in rows.items():
        if key not in found:
            report.fail(f"kit-departures.md:{number} `{key[0]}` {key[1]} is not in style.css any more: delete the row")
        if not reason.strip():
            report.fail(f"kit-departures.md:{number} `{key[0]}` {key[1]} has no reason")
        if re.fullmatch(r"F-NEW", spec.strip()):
            pending_spec += 1
        elif not re.search(r"\bF-\d+\b", spec):
            report.fail(f"kit-departures.md:{number} `{key[0]}` {key[1]} names no SPEC row (F-nnn)")
    if pending_spec:
        report.notes.append(f"{pending_spec} departure rows wait for a SPEC row number (F-NEW)")
    return len(rows)


# ---------------------------------------------------------------- Rust views

TAG_OPEN = re.compile(r"<([a-z][a-z0-9-]*)")
VIEW_OPEN = re.compile(r"\bview!\s*([{(])")
CLASS_ATTR = re.compile(r"\bclass\s*=\s*")


def skip_string(text, i, n):
    """Index of the closing quote of the string literal that opens at text[i]."""
    i += 1
    while i < n and text[i] != '"':
        i += 2 if text[i] == "\\" else 1
    return i


def match_close(text, i, open_ch, close_ch, limit=None):
    """Index after the delimiter that closes text[i] == open_ch; strings are skipped."""
    depth, n = 0, len(text) if limit is None else limit
    while i < n:
        ch = text[i]
        if ch == '"':
            i = skip_string(text, i, n)
        elif ch == open_ch:
            depth += 1
        elif ch == close_ch:
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n


class Node:
    def __init__(self, tag, attrs, classes, line):
        self.tag, self.attrs, self.classes, self.line = tag, attrs, classes, line
        self.texts = []  # (kind, text) direct content: "lit" or "expr"


def classes_of(attrs):
    found = set(re.findall(r"\bclass:([\w-]+)", attrs))
    for match in CLASS_ATTR.finditer(attrs):
        rest = attrs[match.end():]
        if rest.startswith('"'):
            found.update(rest[1:skip_string(rest, 0, len(rest))].split())
        else:  # class=format!("a {x}") or class=move || ...: the literals up to the next attribute
            nxt = re.search(r"\s[\w:-]+\s*=", rest)
            for lit in re.findall(r'"([^"]*)"', rest[: nxt.start() if nxt else len(rest)]):
                found.update(w for w in re.split(r"[\s{}]+", lit) if w)
    return found


def label_of(attrs):
    for name in ("aria-label", "data-testid"):
        m = re.search(name + r'\s*=\s*"([^"]*)"', attrs)
        if m:
            return m.group(1)
    return ""


def find_views(text, lo, hi):
    """Absolute spans (start, end) of the outermost view! { ... } bodies in text[lo:hi]."""
    spans, i = [], lo
    while True:
        m = VIEW_OPEN.search(text, i, hi)
        if not m:
            return spans
        close = match_close(text, m.end() - 1, m.group(1), "}" if m.group(1) == "{" else ")", hi)
        spans.append((m.end(), close - 1, m.start()))
        i = close


def parse_view(text, start, end, ancestors, nodes):
    stack = list(ancestors)
    open_nodes = []
    i = start
    while i < end:
        ch = text[i]
        if ch == "<" and text.startswith("</", i):
            close = text.find(">", i, end)
            if open_nodes:
                open_nodes.pop()
                stack.pop()
            i = close + 1 if close >= 0 else end
        elif ch == "<" and TAG_OPEN.match(text, i):
            m = TAG_OPEN.match(text, i)
            j = m.end()
            while j < end and text[j] != ">":
                if text[j] == '"':
                    j = skip_string(text, j, end)
                elif text[j] == "{":
                    j = match_close(text, j, "{", "}", end) - 1
                j += 1
            attrs = text[m.end():j]
            node = Node(m.group(1), attrs, classes_of(attrs), line_of(text, i))
            nodes.append((node, [set(s) for s in stack]))
            if not attrs.rstrip().endswith("/"):
                open_nodes.append(node)
                stack.append(node.classes)
            i = j + 1
        elif ch == "{":
            close = match_close(text, i, "{", "}", end)
            nested = find_views(text, i + 1, close - 1)
            for lo, hi, _begin in nested:
                parse_view(text, lo, hi, stack, nodes)
            if open_nodes and not nested:  # an expression that builds elements is judged through them
                open_nodes[-1].texts.append(("expr", text[i + 1:close - 1]))
            i = close
        elif ch == '"':
            j = skip_string(text, i, end)
            if open_nodes:
                open_nodes[-1].texts.append(("lit", text[i + 1:j]))
            i = j + 1
        else:
            i += 1


def view_nodes(source):
    nodes = []
    for lo, hi, _begin in find_views(source, 0, len(source)):
        parse_view(source, lo, hi, [], nodes)
    return nodes


def mono_rules(*css_texts):
    """Rules that set the mono face or tabular figures: classes (anywhere) and (tag, ancestor class) pairs."""
    classes, pairs = set(), set()
    for css in css_texts:
        for selectors, decls, _ in parse_css(css):
            font = decls.get("font-family", ("", 0))[0]
            numeric = decls.get("font-variant-numeric", ("", 0))[0]
            if "var(--mono)" not in font and "tabular-nums" not in numeric:
                continue
            for selector in selectors:
                parts = selector.split()
                tail = parts[-1]
                names = re.findall(r"\.([\w-]+)", tail)
                if names:
                    classes.update(names)
                elif len(parts) > 1 and re.fullmatch(r"[a-z]+", tail):
                    pairs.update((tail, anc) for anc in re.findall(r"\.([\w-]+)", parts[-2]))
    return classes, pairs


def expr_shape(expr):
    template = re.search(r'format!\s*\(\s*"([^"]*)"', expr)
    if template:
        residue = re.sub(r"\{[^}]*\}", "", template.group(1))
        return "number" if UNIT_ONLY.match(residue) and "{" in template.group(1) else None
    if "." not in expr and "(" not in expr:
        return None  # a bare variable: its shape is decided where it is built
    words = set(re.findall(r"[a-z_][a-z0-9_]*", re.sub(r'"[^"]*"', '""', expr)))
    if words & NUM_WORDS or any(w.endswith(NUM_SUFFIXES) for w in words):
        return "number"
    if words & ID_WORDS:
        return "identifier"
    return None


def is_prose(node):
    return any(kind == "lit" and re.search(r"[A-Za-z]{2,}", text) for kind, text in node.texts)


def node_key(source_name, node):
    label = label_of(node.attrs)
    if label:
        return f"{source_name} {node.tag}[{label}]"
    snippet = " ".join(" ".join(t for _, t in node.texts).split())[:48]
    return f"{source_name} {node.tag} `{snippet}`"


def check_mono(report, sources, mono, pending_keys):
    classes, pairs = mono
    violations = {}
    for name, source in sources.items():
        for node, ancestors in view_nodes(source):
            if node.tag == "table" and not any("lib" in a for a in ancestors):
                violations.setdefault(f"{name} table[{label_of(node.attrs)}]", []).append(
                    f"{name}:{node.line} <table> is not inside the kit's .lib box (rule 6)")
            if node.tag in FORM_TAGS | CONTAINER_TAGS or not node.texts:
                continue
            ancestor_classes = set().union(*ancestors) if ancestors else set()
            is_mono = bool((node.classes | ancestor_classes) & classes) or any(
                (node.tag, anc) in pairs for anc in ancestor_classes)
            shape = None
            if node.tag == "time":
                shape = "timestamp"
            elif not is_prose(node):
                for kind, text in node.texts:
                    if kind == "lit":
                        own = text.strip()
                        if own and LITERAL_SHAPE.match(own) and own not in LITERAL_WORDS:
                            shape = "identifier"
                    else:
                        shape = shape or expr_shape(text)
            if shape is None:
                continue
            key = node_key(name, node)
            if not is_mono:
                violations.setdefault(key, []).append(
                    f"{name}:{node.line} <{node.tag}> {shape}-shaped text is not in the mono face (add class=\"mono\")")
            elif node.tag == "td" and shape == "number" and "n" not in node.classes:
                violations.setdefault(key, []).append(
                    f"{name}:{node.line} <td> number-shaped cell lacks class=\"n\" (right-aligned tabular figures)")
    pending = 0
    for key, messages in sorted(violations.items()):
        if key in pending_keys:
            pending += 1
            continue
        for message in messages:
            report.fail(message + f" [{key}]")
    for key in sorted(pending_keys - set(violations)):
        report.fail(f"kit-pending.txt: `{key}` no longer violates the kit: delete the line")
    return pending


def read_pending(path):
    keys = set()
    if os.path.exists(path):
        for line in read(path).splitlines():
            line = line.split("  #", 1)[0].rstrip()
            if line and not line.startswith("#"):
                keys.add(line)
    return keys


# ---------------------------------------------------------------- main


def main():
    root = os.environ.get("KIT_ROOT") or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    ui = os.path.join(root, "apps/observatory-app/ui")
    report = Report()

    kit_css = read(os.path.join(root, "ai/design/ui-kit-v2/theme.css"))
    theme = read(os.path.join(ui, "theme.css"))
    style = read(os.path.join(ui, "style.css"))
    index = read(os.path.join(ui, "index.html"))
    src_dir = os.path.join(ui, "src")
    sources = {f: read(os.path.join(src_dir, f)) for f in sorted(os.listdir(src_dir)) if f.endswith(".rs")}
    departures_md = read(os.path.join(root, "apps/observatory-app/kit-departures.md"))
    pending_keys = read_pending(os.path.join(root, "apps/observatory-app/kit-pending.txt"))

    if theme != kit_css:
        report.fail("ui/theme.css differs from ai/design/ui-kit-v2/theme.css (rule 1: the kit's file is the app's stylesheet)")

    files = {f"src/{n}": t for n, t in sources.items()}
    files.update({"style.css": style, "index.html": index})
    for name, text in files.items():
        body = strip_comments(text) if name.endswith(".css") else text
        for m in LITERAL_COLOUR.finditer(body):
            report.fail(f"{name}:{line_of(body, m.start())} literal colour `{m.group(0)}` (rule 1: tokens only)")
        for m in re.finditer(r"font-family", body):
            report.fail(f"{name}:{line_of(body, m.start())} font-family outside theme.css (rule 2: faces come from the tokens)")
        for m in re.finditer(r"\bstyle\s*=\s*\"([^\"]*)\"", body):
            if re.search(r"padding|margin|gap|font|width|inset", m.group(1)):
                report.fail(f"{name}:{line_of(body, m.start())} inline style `{m.group(1)}`: spacing and type live in a class")

    style_rules = parse_css(style)
    kit_rules = parse_css(kit_css)
    rows = check_departures(report, style_rules, kit_rules, departures_md)
    mono = mono_rules(kit_css, style)
    pending = check_mono(report, sources, mono, pending_keys)

    if report.fails:
        print(f"{CHECK} FAIL {IDS}")
        for message in report.fails:
            print(f"  {CHECK}: {message}", file=sys.stderr)
        return 1
    notes = [f"{rows} departures recorded"] + report.notes
    if pending:
        notes.append(f"pending: {pending} known mono violations in kit-pending.txt")
    print(f"{CHECK} ok {IDS} [{'; '.join(notes)}]")
    return 0


if __name__ == "__main__":
    sys.exit(main())
