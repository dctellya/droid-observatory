"""Usage report tables that split cost by token class and read the declarations beside the log.

Imported by scripts/usage-report.py (same directory); Python 3 standard library only.
"""

# (name, Acc attribute, price-table rate key), in the order cost follows the class: fresh input, cache write, cache read, output.
TOKEN_CLASSES = (("input", "i", "input"), ("cache write", "cw", "cache_write"), ("cache read", "cr", "cache_read"), ("output", "o", "output"))


def new_totals():
    return {name: [0, 0.0] for name, _, _ in TOKEN_CLASSES}


def add_totals(totals, acc, rates):
    """Add one Acc priced at `rates` (per million tokens by class) to the per-class [tokens, usd] totals."""
    for name, attr, key in TOKEN_CLASSES:
        n = getattr(acc, attr)
        totals[name][0] += n
        totals[name][1] += n * rates[key] / 1e6


def share(part, whole):
    return "%.1f" % (100.0 * part / whole) if whole else "-"


def class_rows(scope, totals):
    tokens = sum(t for t, _ in totals.values())
    cost = sum(c for _, c in totals.values())
    return [[scope, name, n, share(n, tokens), "%.2f" % c, share(c, cost)] for name, (n, c) in totals.items()]
