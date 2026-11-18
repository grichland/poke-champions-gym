"""Prints the row disagreements of a driver.py --rows run: per key, the count and the first examples as the two
common-form sequences side by side (Showdown's lines and the engine's rows around the first difference).

usage: python tools/rig/rows_report.py RUN.jsonl.gz [MAX_KEYS] [EXAMPLES_PER_KEY]
"""

import difflib
import gzip
import json
import sys


def main():
    path = sys.argv[1]
    max_keys = int(sys.argv[2]) if len(sys.argv) > 2 else 40
    per = int(sys.argv[3]) if len(sys.argv) > 3 else 1
    op = gzip.open if path.endswith(".gz") else open
    summary = None
    exs = []
    with op(path, "rt", encoding="utf-8") as f:
        for line in f:
            r = json.loads(line)
            if r.get("kind") == "rows_examples":
                exs.append(r)
            elif r.get("kind") == "summary":
                summary = r
    if summary and "rows" in summary:
        rs = summary["rows"]
        print(f"steps {rs['steps']}, equal {rs['steps_equal']}")
        for k, v in sorted(rs["kinds"].items()):
            tot = v["both"] + max(v["showdown_only"], v["engine_only"])
            print(f"  {k:16} both {v['both']:6}  showdown_only {v['showdown_only']:5}  engine_only {v['engine_only']:5}  "
                  f"agree {100.0 * v['both'] / tot if tot else 100:.1f}%")
        print("dropped showdown lines:", rs["showdown_lines_dropped"])
        print("dropped engine rows:", rs["engine_rows_dropped"])
    for r in exs[:max_keys]:
        print(f"\n=== {r['key']}  ({r['count']})")
        for st in r["steps"][:per]:
            sd = [tuple(x) for x in st["sd_items"]]
            en = [tuple(x) for x in st["en_items"]]
            print("  step", st["step"])
            sm = difflib.SequenceMatcher(None, sd, en, autojunk=False)
            for tag, i1, i2, j1, j2 in sm.get_opcodes():
                if tag == "equal":
                    continue
                print(f"   {tag}: SD {sd[i1:i2]}  EN {en[j1:j2]}")
            print("   lines:", " ".join(x for x in st["showdown"] if not x.startswith("|t:") and x)[:1500])


if __name__ == "__main__":
    main()
