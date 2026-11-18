"""Battle-log rows against Showdown's protocol lines (driver.py --rows).

Both are mapped to one common form, a list of items (kind, who, what) per step, and aligned. `who` is
"side:slot" (or "side" for a side condition, "" for none); `what` is the effect id, the HP after, the stat
change, and so on. A Showdown line with "[from] ability: X" or "[from] item: X", and an engine row whose cause
is an ability or item, both add an ("ability"/"item", holder, id) item before their own, since the game writes
an Ability or Held Item row for them.

Showdown lines the game does not show are dropped here and counted (SKIPPED); rows the game writes that
Showdown has no line for (the engine's withdraw rows) are dropped and counted too.
"""

import difflib
import re

SIDES = ("p1", "p2")


def toid(s):
    return "".join(c for c in str(s).lower() if c.isalnum())


def strip_prefix(s):
    s = str(s)
    for p in ("move: ", "ability: ", "item: "):
        if s.startswith(p):
            return s[len(p):]
    return s


# the protect family's lines all name Protect
PROTECTS = {"protect", "kingsshield", "spikyshield", "banefulbunker", "burningbulwark", "silktrap", "obstruct"}
# Showdown lines the phone game does not write
SKIPPED_KINDS = {"-crit", "-hint", "-anim", "upkeep", "t:", "", "-message", "-center", "-combine", "-waiting",
                 "-swapsideconditions",
                 "-zpower", "split", "gametype", "player", "teamsize", "gen", "tier", "rule", "start", "teampreview",
                 "poke", "clearpoke", "-notarget", "j", "raw", "debug"}


class Names:
    """Showdown idents ('p1a: Garchomp') to engine Pokémon ('0:1'), from the snapshot's species."""

    def __init__(self, state, teams=None, picks=None):
        self.slots = {}
        # the oracle names a Pokémon by its set's name, else its species' base name
        self.named = {}
        for si, sd in enumerate(SIDES):
            self.slots[sd] = [(i, m.get("baseSpecies", ""), m.get("species", ""))
                              for i, m in enumerate(state["sides"][sd]["pokemon"])]
            if teams and picks:
                for i, k in enumerate(picks[sd]):
                    n = teams[sd][k].get("name")
                    if n:
                        self.named[(sd, toid(n))] = i
        self.cache = {}

    def who(self, ident):
        ident = ident.strip()
        if ident in self.cache:
            return self.cache[ident]
        m = re.match(r"(p[12])[a-z]?(?::\s*(.*))?$", ident)
        if not m:
            return ""
        sd, name = m.group(1), toid(m.group(2) or "")
        si = SIDES.index(sd)
        if not name:
            return str(si)
        if (sd, name) in self.named:
            out = f"{si}:{self.named[(sd, name)]}"
            self.cache[ident] = out
            return out
        # the species brought (a transformed Pokémon's form is another's); exact before prefix
        best = next((i for i, base, _ in self.slots[sd] if base == name), None)
        if best is None:
            best = next((i for i, base, _ in self.slots[sd] if base.startswith(name)), None)
        out = f"{si}:{best}" if best is not None else f"{si}:?{name}"
        self.cache[ident] = out
        return out


def attrs(parts):
    """The [from] and [of] attributes of a line."""
    frm, of, flags = "", "", set()
    for p in parts:
        if p.startswith("[from]"):
            frm = p[6:].strip()
        elif p.startswith("[of]"):
            of = p[4:].strip()
        elif p.startswith("["):
            flags.add(p.split("]")[0] + "]")
    return frm, of, flags


# line kinds whose [of] is the holder of the ability or item named in [from]
OF_HOLDS = {"-damage", "-status", "-start", "-weather", "-fieldstart", "-sidestart", "-enditem",
            "-clearallboost", "-curestatus", "-end", "-activate", "-immune"}


def hp_after(s):
    s = str(s).split(" ")[0]
    if "/" in s:
        return int(s.split("/")[0])
    try:
        return int(s)
    except ValueError:
        return 0


def sd_items(lines, names, skipped):
    """Common-form items of Showdown's lines for one step."""
    out = []
    for li, line in enumerate(lines):
        parts = line.split("|")
        if len(parts) < 2:
            continue
        k = parts[1]
        a = parts[2:]
        if k in SKIPPED_KINDS:
            skipped[k] = skipped.get(k, 0) + 1
            continue
        frm, of, flags = attrs(a)
        if "[silent]" in flags and k not in ("-damage", "-heal", "-sethp"):
            skipped[k + " [silent]"] = skipped.get(k + " [silent]", 0) + 1
            continue
        w = names.who(a[0]) if a and a[0].startswith("p") else ""
        items = []
        if k == "move":
            items.append(("move", w, toid(a[1])))
        elif k in ("switch", "drag"):
            items.append(("enter", w, "forced" if k == "drag" else ""))
        elif k == "replace":
            skipped[k] = skipped.get(k, 0) + 1
        elif k == "cant":
            items.append(("cant", w, toid(strip_prefix(a[1]))))
        elif k == "faint":
            items.append(("faint", w, ""))
        elif k in ("-damage", "-heal", "-sethp"):
            if frm == "ability: Regenerator":
                skipped["-heal Regenerator [silent]"] = skipped.get("-heal Regenerator [silent]", 0) + 1
                continue
            if frm == "move: Revival Blessing":
                items.append(("revive", w, ""))
            else:
                items.append(("hp", w, str(hp_after(a[1]))))
        elif k == "-status":
            items.append(("status", w, a[1]))
        elif k == "-curestatus":
            items.append(("cure", w, a[1]))
        elif k in ("-boost", "-unboost"):
            n = int(a[2]) if len(a) > 2 and a[2].lstrip("-").isdigit() else 0
            stat = a[1]
            if n == 0:
                items.append(("boost_limit", w, stat + ("+" if k == "-boost" else "-")))
            else:
                items.append(("boost", w, f"{stat}{'+' if k == '-boost' else '-'}{n}"))
        elif k == "-setboost":
            items.append(("boost", w, f"{a[1]}="))
        elif k == "-clearallboost":
            items.append(("boost_reset", "", "field"))
        elif k == "-clearboost":
            items.append(("boost_reset", w, "all"))
        elif k == "-clearnegativeboost":
            items.append(("boost_reset", w, "negative"))
        elif k == "-clearpositiveboost":
            items.append(("boost_reset", w, "positive"))
        elif k == "-copyboost":
            items.append(("boost_reset", w, "copy"))
        elif k == "-swapboost":
            items.append(("boost_reset", w, "swap"))
        elif k == "-invertboost":
            items.append(("boost_reset", w, "invert"))
        elif k == "-weather":
            if "[upkeep]" in flags:
                skipped["-weather [upkeep]"] = skipped.get("-weather [upkeep]", 0) + 1
                continue
            wid = toid(a[0])
            items.append(("weather", "", "" if wid == "none" else wid))
        elif k in ("-fieldstart", "-fieldend"):
            items.append(("field", "", toid(strip_prefix(a[0])) + (":start" if k == "-fieldstart" else ":end")))
        elif k == "-fieldactivate":
            e = toid(strip_prefix(a[0]))
            items.append(("once", "", e) if e in ("perishsong", "teatime") else ("field", "", e + ":start"))
        elif k in ("-sidestart", "-sideend"):
            side = str(SIDES.index(a[0][:2]))
            items.append(("side", side, toid(strip_prefix(a[1])) + (":start" if k == "-sidestart" else ":end")))
        elif k == "-swapsideconditions":
            items.append(("side", "", "courtchange"))
        elif k == "-start":
            eff = toid(strip_prefix(a[1]))
            if eff in ("futuresight", "doomdesire") or (eff == "uproar" and "[upkeep]" in flags):
                items.append(("once", w, eff))
            elif eff == "slowstart":
                items.append(("ability", w, eff))
            elif eff == "typechange":
                items.append(("types", w, ""))
            elif eff == "typeadd":
                items.append(("types", w, "added"))
            else:
                for pre in ("perish", "stockpile", "protosynthesis", "quarkdrive", "fallen"):
                    if eff.startswith(pre):
                        eff = {"perish": "perishsong", "fallen": "supremeoverlord"}.get(pre, pre)
                items.append(("volatile", w, eff))
        elif k == "-end":
            eff = toid(strip_prefix(a[1]))
            if eff in ("futuresight", "doomdesire", "slowstart"):
                items.append(("once", w, eff))
            elif "[partiallytrapped]" in flags:
                items.append(("volatile_end", w, "partiallytrapped"))
            else:
                items.append(("volatile_end", w, eff))
        elif k == "-activate":
            eff = a[1] if len(a) > 1 else ""
            e = toid(strip_prefix(eff))
            if eff.startswith("ability: "):
                items.append(("ability", w, e))
            elif eff.startswith("item: "):
                items.append(("item", w, e))
            elif e in ("protect", "quickguard", "wideguard", "matblock"):
                items.append(("protect", w, ""))
            elif e in ("substitute", "confusion", "endure", "destinybond", "attract"):
                items.append(("volatile_acted", w, e))
            elif e == "stickyweb":
                items.append(("side", w.split(":")[0], "stickyweb:acted"))
            elif e == "trapped":
                items.append(("volatile", w, "trapped"))
            elif "[broken]" in flags or e == "feint":
                items.append(("once", w, e))
            elif e in ("electricterrain", "mistyterrain", "psychicterrain", "safeguard", "ingrain"):
                items.append(("blocked", w, e))
            elif e == "orichalcumpulse":
                items.append(("ability", w, e))
            elif e == "poltergeist":
                items.append(("item", w, toid(a[2]) if len(a) > 2 else ""))
            elif e in ("spite", "eeriespell"):
                items.append(("pp", w, toid(a[2]) if len(a) > 2 else ""))
            elif any(x in eff for x in ("Bind", "Wrap", "Fire Spin", "Whirlpool", "Sand Tomb", "Magma Storm",
                                        "Infestation", "Snap Trap", "Thunder Cage", "Clamp")):
                items.append(("volatile", w, "partiallytrapped"))
            elif eff.startswith("move: ") or e == "skillswap":
                items.append(("once", w, e))
            else:
                items.append(("activate", w, e))
        elif k in ("-singleturn", "-singlemove"):
            e = toid(strip_prefix(a[1]))
            items.append(("once", w, "protect" if e in PROTECTS else e))
        elif k in ("-supereffective", "-resisted"):
            n = int(a[1]) if len(a) > 1 and a[1].isdigit() else 1
            how = ("super" if n == 1 else "extreme") if k == "-supereffective" else ("resisted" if n == 1 else "mostly_ineffective")
            items.append(("result", w, how))
        elif k == "-immune":
            items.append(("result", w, "immune"))
        elif k == "-ohko":
            items.append(("result", "", "ohko"))
        elif k == "-miss":
            items.append(("miss", names.who(a[1]) if len(a) > 1 and a[1] else "", ""))
        elif k == "-fail":
            if len(a) > 1 and a[1] == "unboost":
                items.append(("blocked", w, toid(strip_prefix(frm)) if frm else ""))
            else:
                items.append(("fail", w, ""))
        elif k == "-block":
            eff = a[1] if len(a) > 1 else ""
            if eff.startswith("ability: "):
                items.append(("ability", names.who(of) if of else w, toid(strip_prefix(eff))))
            items.append(("blocked", w, toid(strip_prefix(eff))))
        elif k == "-hitcount":
            items.append(("hits", w, a[1]))
        elif k == "-prepare":
            mid = toid(a[1])
            items.append(("once", w, mid) if mid == "chillyreception" else ("prepare", w, mid))
        elif k == "-mustrecharge":
            items.append(("volatile", w, "mustrecharge"))
        elif k == "-ability":
            items.append(("ability", w, toid(a[1])))
        elif k == "-endability":
            items.append(("volatile", w, "gastroacid"))
        elif k in ("-item", "-enditem"):
            items.append(("item", w, toid(a[1])))
        elif k == "-mega":
            items.append(("mega_stone", w, ""))
            items.append(("mega", w, ""))
        elif k == "detailschange":
            nxt = lines[li + 1].split("|") if li + 1 < len(lines) else []
            if len(nxt) > 1 and nxt[1] == "-mega":
                skipped["detailschange (mega)"] = skipped.get("detailschange (mega)", 0) + 1
                continue
            items.append(("form", w, ""))
        elif k == "-formechange":
            items.append(("form", w, ""))
        elif k == "-transform":
            items.append(("transform", w, ""))
        elif k == "turn":
            items.append(("turn", "", a[0]))
        elif k in ("win", "tie"):
            items.append(("end", "", ""))
        elif k == "-nothing":
            items.append(("once", "", "nothing"))
        elif k == "-zbroken":
            items.append(("once", w, "zbroken"))
        else:
            items.append(("other:" + k, w, ""))
        # the ability or item that made this line
        src = []
        if frm.startswith("ability: ") or frm.startswith("item: "):
            kind = "ability" if frm.startswith("ability: ") else "item"
            holds = k in OF_HOLDS or (k == "-item" and toid(strip_prefix(frm)) == "frisk")
            holder = names.who(of) if (of and holds) else w
            if k == "-ability" or (k == "-activate" and items and items[0][0] == kind):
                pass
            elif k in ("-item", "-enditem") and kind == "item":
                pass
            else:
                src.append((kind, holder, toid(strip_prefix(frm))))
        out.extend(src + items)
    return out


# Showdown's name of an ability row
ABILITY_ALIAS = {"asoneglastrier": "asone", "asonespectrier": "asone"}


def w_of(p):
    return f"{p[0]}:{p[1]}" if p is not None else ""


def fx_src(fx):
    """('ability'|'item', id) of an effect string, or None."""
    if fx.startswith("ability:"):
        return "ability", fx[8:]
    if fx.startswith("item:"):
        return "item", fx[5:]
    return None


def en_items(rows, extra):
    """Common-form items of the engine's rows for one step."""
    out = []
    for r in rows:
        k = r["k"]
        w = w_of(r.get("p"))
        items = []
        src = None
        if k == "withdraw":
            extra["withdraw"] = extra.get("withdraw", 0) + 1
            continue
        if k == "turn":
            items.append(("turn", "", str(r["n"])))
        elif k == "enter":
            items.append(("enter", w, "forced" if r.get("forced") else ""))
        elif k == "move":
            items.append(("move", w, r["move"]))
            cb = r.get("called_by", "")
            if cb.startswith("ability:"):
                src = ("ability", w, cb[8:])
        elif k == "prepare":
            items.append(("prepare", w, r["move"]))
        elif k == "cant":
            why = r.get("why", "")
            why = why.split(":", 1)[1] if ":" in why else why
            items.append(("cant", w, {"mustrecharge": "recharge", "": "nopp"}.get(why, why)))
        elif k == "protect":
            items.append(("protect", w, ""))
        elif k == "miss":
            items.append(("miss", w, ""))
        elif k == "fail":
            items.append(("fail", w, ""))
        elif k == "result":
            items.append(("result", "" if r["how"] == "ohko" else w, r["how"]))
        elif k == "hits":
            items.append(("hits", w, str(r["n"])))
        elif k == "hp":
            items.append(("hp", w, str(r["after"])))
            s = fx_src(r.get("cause", ""))
            if s:
                holder = w_of(r.get("of")) if (r.get("of") and r["after"] < r["before"]) else w
                src = (s[0], holder, s[1])
        elif k == "faint":
            items.append(("faint", w, ""))
        elif k == "status":
            items.append(("status", w, r["status"]))
            s = fx_src(r.get("by", ""))
            # Synchronize's status line names no ability (its own row comes before)
            if s and s[1] != "synchronize":
                src = (s[0], w_of(r.get("of")) or w, s[1])
        elif k == "cure":
            # an item's cure follows its own item row; Showdown's cure line names nothing
            items.append(("cure", w, r["status"]))
        elif k == "volatile":
            eff = r["effect"]
            if eff in ("trapped",):
                items.append(("volatile", w, eff))
            else:
                items.append(("volatile", w, eff))
            s = fx_src(r.get("by", ""))
            if s:
                src = (s[0], w_of(r.get("of")) or w, s[1])
        elif k == "volatile_end":
            items.append(("volatile_end", w, r["effect"] or ("illusion" if r.get("by") == "ability:illusion" else "")))
            s = fx_src(r.get("by", ""))
            if s and s[1] != "illusion":
                src = (s[0], w, s[1])
        elif k == "volatile_acted":
            items.append(("volatile_acted", w, r["effect"]))
        elif k == "boost":
            by = r.get("by", "")
            for st, v in r["stats"].items():
                if by in ("move:bellydrum", "ability:angerpoint"):
                    items.append(("boost", w, f"{st}="))
                else:
                    items.append(("boost", w, f"{st}{'+' if v > 0 else '-'}{abs(v)}"))
            s = fx_src(by)
            if s and s[0] == "item":
                src = (s[0], w, s[1])
        elif k == "boost_limit":
            for st in r["stats"]:
                items.append(("boost_limit", w, st + ("+" if r["dir"] == "up" else "-")))
        elif k == "boost_reset":
            items.append(("boost_reset", w if r["what"] != "field" else "", r["what"]))
        elif k == "blocked":
            what = r.get("by", "")
            items.append(("blocked", w, what.split(":", 1)[1] if ":" in what else what))
            s = fx_src(what)
            if s:
                src = (s[0], w, s[1])
        elif k == "ability":
            items.append(("ability", w, ABILITY_ALIAS.get(r["ability"], r["ability"])))
        elif k == "ability_change":
            if r.get("by") in ("ability:mummy", "ability:lingeringaroma"):
                extra["ability_change (mummy)"] = extra.get("ability_change (mummy)", 0) + 1
                continue
            items.append(("ability", w, r["to"]))
        elif k == "item":
            items.append(("item", w, r["item"]))
            s = fx_src(r.get("by", ""))
            if s:
                src = (s[0], w_of(r.get("of")) if s[1] == "frisk" else w, s[1])
        elif k == "mega_stone":
            items.append(("mega_stone", w, ""))
        elif k == "mega":
            items.append(("mega", w, ""))
        elif k == "form":
            items.append(("form", w, ""))
            s = fx_src(r.get("by", ""))
            if s:
                src = (s[0], w, s[1])
        elif k == "transform":
            items.append(("transform", w, ""))
        elif k == "types":
            items.append(("types", w, "added" if r.get("added") else ""))
            s = fx_src(r.get("by", ""))
            if s:
                src = (s[0], w, s[1])
        elif k == "weather":
            items.append(("weather", "", r["weather"]))
            s = fx_src(r.get("by", ""))
            if s:
                src = (s[0], w_of(r.get("of")), s[1])
        elif k == "field":
            items.append(("field", "", f"{r['effect']}:{r['what']}"))
            s = fx_src(r.get("by", ""))
            if s:
                src = (s[0], w_of(r.get("of")), s[1])
        elif k == "side":
            items.append(("side", str(r["side"]) if r["what"] != "acted" else str(r["side"]), f"{r['effect']}:{r['what']}"))
        elif k == "once":
            eff = r.get("effect", "")
            # a protection broken by an ability (Unseen Fist, Piercing Drill)
            eff = "zbroken" if eff in ("ability:unseenfist", "ability:piercingdrill") else (
                eff.split(":", 1)[1] if ":" in eff else eff)
            items.append(("once", w, "protect" if eff in PROTECTS else eff))
        elif k == "pp":
            items.append(("pp", w, r["move"]))
        elif k == "revive":
            items.append(("revive", w, ""))
        elif k == "end":
            items.append(("end", "", ""))
        else:
            items.append(("other:" + k, w, ""))
        out.extend(([src] if src else []) + items)
    return out


def canon_runs(items):
    """Sorts each run of boost and boost-limit items on one Pokémon: the engine's tables keep no key order."""
    out = list(items)
    i = 0
    while i < len(out):
        if out[i][0] in ("boost", "boost_limit"):
            j = i
            while j < len(out) and out[j][0] in ("boost", "boost_limit") and out[j][1] == out[i][1]:
                j += 1
            out[i:j] = sorted(out[i:j])
            i = j
        else:
            i += 1
    return out


# revival: Showdown writes an HP line, the engine a revive row
def revive_fix(sd, en):
    return sd, en


class RowStats:
    def __init__(self):
        self.kinds = {}
        self.skipped = {}
        self.extra = {}
        self.examples = {}
        self.steps = 0
        self.steps_equal = 0
        self.illusion_steps = 0
        # matched items per kind and effect: which rows the run reached
        self.seen = {}

    def kind(self, k):
        return self.kinds.setdefault(k, {"both": 0, "showdown_only": 0, "engine_only": 0})

    def compare(self, lines, rows, state, step_id, max_examples=12, teams=None, picks=None):
        names = Names(state, teams, picks)
        sd = sd_items(lines, names, self.skipped)
        en = en_items(rows, self.extra)
        # Illusion: Showdown names the disguise, the engine the Pokémon; on a side that brought it, only
        # the side is compared
        ill = set()
        if teams and picks:
            for si, side in enumerate(SIDES):
                if any(toid(teams[side][k].get("ability", "")) == "illusion" for k in picks[side]):
                    ill.add(str(si))
        if ill:
            self.illusion_steps += 1
            fix = lambda xs: [(k, w.split(":")[0] + ":*" if w.split(":")[0] in ill and ":" in w else w, v) for k, w, v in xs]
            sd, en = fix(sd), fix(en)
        sd, en = canon_runs(sd), canon_runs(en)
        self.steps += 1
        if sd == en:
            self.steps_equal += 1
        sm = difflib.SequenceMatcher(None, sd, en, autojunk=False)
        for tag, i1, i2, j1, j2 in sm.get_opcodes():
            if tag == "equal":
                for x in sd[i1:i2]:
                    self.kind(x[0])["both"] += 1
                    sk = x[0] + "|" + (x[2] if x[0] in EX_BY_WHAT else "")
                    self.seen[sk] = self.seen.get(sk, 0) + 1
                continue
            for x in sd[i1:i2]:
                self.kind(x[0])["showdown_only"] += 1
                self.example(("showdown_only", x[0], x[2] if x[0] in EX_BY_WHAT else ""), step_id, lines, rows, sd, en, max_examples)
            for x in en[j1:j2]:
                self.kind(x[0])["engine_only"] += 1
                self.example(("engine_only", x[0], x[2] if x[0] in EX_BY_WHAT else ""), step_id, lines, rows, sd, en, max_examples)

    def example(self, key, step_id, lines, rows, sd, en, n):
        k = "|".join(key)
        ex = self.examples.setdefault(k, {"count": 0, "steps": []})
        ex["count"] += 1
        if len(ex["steps"]) < n:
            ex["steps"].append({"step": step_id, "showdown": lines, "rows": rows,
                                "sd_items": [list(x) for x in sd], "en_items": [list(x) for x in en]})

    def summary(self):
        return {"steps": self.steps, "steps_equal": self.steps_equal, "illusion_steps": self.illusion_steps,
                "kinds": self.kinds, "seen": self.seen,
                "showdown_lines_dropped": self.skipped, "engine_rows_dropped": self.extra,
                "disagreements": {k: v["count"] for k, v in sorted(self.examples.items(), key=lambda kv: -kv[1]["count"])}}


# kinds whose example key also carries the effect
EX_BY_WHAT = {"volatile", "volatile_end", "once", "activate", "ability", "item", "blocked", "cant", "field", "side",
              "boost_reset", "volatile_acted", "result", "weather", "status", "cure", "other:-activate"}
