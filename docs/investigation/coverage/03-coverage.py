"""Coverage of our usage universe by poke-engine 0.0.48 (champions/bss build). Runs on the laptop.

usage:  python 03-coverage.py <workdir>
workdir holds: sd.json (03-dump_showdown.js), engine_src.tar, seen_builds.json, top_builds.json, form-sets.json,
gold_team.txt, ladders/ladder-*.json.  Writes 03-coverage-*.csv / .json into workdir/out.
"""
import csv, json, re, sys, tarfile, collections, glob, os, math
from pathlib import Path

W = Path(sys.argv[1])
OUT = W / "out"
OUT.mkdir(exist_ok=True)


def tid(s):
    return re.sub(r"[^a-z0-9]", "", (s or "").lower())


# ---------------------------------------------------------------- engine source
SRC = {}
with tarfile.open(W / "engine_src.tar") as t:
    for m in t.getmembers():
        if m.isfile() and m.name.endswith(".rs"):
            SRC[m.name] = t.extractfile(m).read().decode("utf-8", "ignore")
print("engine files", len(SRC))


def match_brace(s, i, open_="{", close="}"):
    """s[i] == open_; return index of the matching close."""
    d = 0
    n = len(s)
    while i < n:
        c = s[i]
        if c == open_:
            d += 1
        elif c == close:
            d -= 1
            if d == 0:
                return i
        i += 1
    raise ValueError("unbalanced")


def parse_enum(text, name):
    m = re.search(r"define_enum_with_from_str!\s*\{[^{}]*?\b" + name + r"\s*\{", text, re.S)
    if not m:
        return None, None
    start = m.end() - 1
    end = match_brace(text, start)
    body = re.sub(r"//[^\n]*", "", text[start + 1:end])
    body = re.sub(r"#\[[^\]]*\]", "", body)
    vs = [v.strip() for v in body.split(",") if v.strip()]
    return vs, (m.start(), end)


enums = {}
enum_spans = {}  # file -> list of spans
for fn, text in SRC.items():
    for nm in ("PokemonName", "Abilities", "Items", "Choices"):
        vs, span = parse_enum(text, nm)
        if vs:
            enums[nm] = set(vs)
            enum_spans.setdefault(fn, []).append(span)
print({k: len(v) for k, v in enums.items()})

# strip enum definitions (and nothing else) -> logic text per file
def strip_spans(text, spans):
    out, last = [], 0
    for a, b in sorted(spans):
        out.append(text[last:a])
        last = b + 1
    out.append(text[last:])
    return "".join(out)


# ---- choices.rs: split table / champions modifiers / rest
ch = SRC["src/choices.rs"]
ch_stripped = strip_spans(ch, enum_spans.get("src/choices.rs", []))
t_start = ch_stripped.index("pub static MOVES")
t_end = ch_stripped.index("apply_champions_modifiers(&mut moves);")  # end of table construction
table_txt = ch_stripped[t_start:t_end]
mod_start = ch_stripped.index("fn apply_champions_modifiers")
mod_end = match_brace(ch_stripped, ch_stripped.index("{", mod_start))
mod_txt = ch_stripped[mod_start:mod_end + 1]
ch_rest = ch_stripped[:t_start] + ch_stripped[t_end:mod_start] + ch_stripped[mod_end + 1:]

# table blocks
def eval_cond(cond):
    c = re.sub(r'cfg!\(feature\s*=\s*"(\w+)"\)', lambda m: "True" if m.group(1) == "champions" else "False", cond)
    c = c.replace("||", " or ").replace("&&", " and ").replace("!", " not ")
    return bool(eval(" ".join(c.split()), {"__builtins__": {}}, {}))


def back_match(txt, close_idx):
    d = 0
    i = close_idx
    while i >= 0:
        if txt[i] == "}":
            d += 1
        elif txt[i] == "{":
            d -= 1
            if d == 0:
                return i
        i -= 1
    return -1


def active_at(txt, pos):
    """Is the code at txt[pos] compiled in with only feature "champions" on (if/else-if/else chains of cfg! tests)?"""
    d, i = 0, pos - 1
    while i >= 0:
        c = txt[i]
        if c == "}":
            d += 1
        elif c == "{":
            if d == 0:
                break
            d -= 1
        i -= 1
    if i < 0:
        return True
    j = i - 1
    while j >= 0 and txt[j] not in "};{":
        j -= 1
    header = txt[j + 1:i].strip()
    outer = active_at(txt, i)
    if header.startswith("if "):
        return outer and eval_cond(header[3:])
    if header.startswith("else if "):
        prev_open = back_match(txt, j) if txt[j] == "}" else -1
        prev = active_at_branch(txt, prev_open) if prev_open >= 0 else False
        return outer and (not prev) and eval_cond(header[8:])
    if header == "else":
        prev_open = back_match(txt, j) if txt[j] == "}" else -1
        prev = active_at_branch(txt, prev_open) if prev_open >= 0 else False
        return outer and not prev
    return outer  # LazyLock closure etc.


def active_at_branch(txt, open_idx):
    """Whether the branch whose '{' is at open_idx would be taken (condition only, ignoring outer)."""
    j = open_idx - 1
    while j >= 0 and txt[j] not in "};{":
        j -= 1
    header = txt[j + 1:open_idx].strip()
    if header.startswith("if "):
        return eval_cond(header[3:])
    if header.startswith("else if "):
        prev_open = back_match(txt, j) if txt[j] == "}" else -1
        return (not active_at_branch(txt, prev_open)) and eval_cond(header[8:])
    if header == "else":
        prev_open = back_match(txt, j) if txt[j] == "}" else -1
        return not active_at_branch(txt, prev_open)
    return True


blocks = {}
n_inactive = 0
for m in re.finditer(r"moves\.insert\(\s*Choices::(\w+),\s*Choice\s*\{", table_txt):
    nm = m.group(1)
    if not active_at(table_txt, m.start()):
        n_inactive += 1
        continue
    b = m.end() - 1
    e = match_brace(table_txt, b)
    blocks[nm] = table_txt[b + 1:e]
print("move table entries", len(blocks), "inactive inserts skipped", n_inactive)


def top_fields(body):
    """name -> raw value text, depth-0 fields of a struct literal body."""
    out = {}
    d = 0
    i = 0
    n = len(body)
    cur = None
    start = 0
    while i < n:
        c = body[i]
        if c in "({[":
            d += 1
        elif c in ")}]":
            d -= 1
        elif d == 0 and c == ":" and body[i + 1:i + 2] != ":" and body[i - 1:i] != ":":
            # identifier before
            j = i - 1
            while j >= 0 and (body[j].isalnum() or body[j] == "_"):
                j -= 1
            ident = body[j + 1:i]
            if ident and not ident[0].isdigit():
                if cur is not None:
                    out[cur] = body[start:j + 1].strip().rstrip(",").strip()
                cur, start = ident, i + 1
        elif d == 0 and body[i:i + 2] == ".." and body[i + 2:i + 9] == "Default":
            if cur is not None:
                out[cur] = body[start:i].strip().rstrip(",").strip()
                cur = None
            break
        i += 1
    if cur is not None:
        out[cur] = body[start:].strip().rstrip(",").strip()
    return out


move_fields = {k: top_fields(v) for k, v in blocks.items()}


def flags_of(fv):
    if not fv:
        return set()
    return set(re.findall(r"(\w+):\s*true", fv))


# champions overrides
overrides = collections.defaultdict(dict)
unparsed = []
i = 0
body = mod_txt
for m in re.finditer(r"moves\.get_mut\(&Choices::(\w+)\)\.unwrap\(\)\.([\w\.]+)\s*=\s*", body):
    nm, fld = m.group(1), m.group(2)
    j = m.end()
    d = 0
    k = j
    while k < len(body):
        c = body[k]
        if c in "({[":
            d += 1
        elif c in ")}]":
            d -= 1
        elif c == ";" and d == 0:
            break
        k += 1
    overrides[nm][fld] = body[j:k].strip()
n_over_stmts = sum(len(v) for v in overrides.values())
other_lines = [l for l in body.split("\n") if "get_mut" in l and "unwrap()." not in l]
print("champions overrides", len(overrides), "moves", n_over_stmts, "stmts; odd lines", len(other_lines))

# ---- logic reference counts
LOGIC_FILES = [fn for fn in SRC if fn != "src/choices.rs" and "gen1" not in fn and "gen2" not in fn and "gen3" not in fn]
text_by_file = {}
for fn in LOGIC_FILES:
    text_by_file[fn] = strip_spans(SRC[fn], enum_spans.get(fn, []))
text_by_file["src/choices.rs(rest)"] = ch_rest
# base_stats data (mega table) is data, not logic: separate it out
bs_fn = "src/genx/base_stats.rs"
base_stats_txt = text_by_file.pop(bs_fn, "")
# tests are not logic
refs = {k: collections.Counter() for k in ("Choices", "Abilities", "Items", "PokemonName", "PokemonVolatileStatus", "PokemonSideCondition")}
refs_files = {k: collections.defaultdict(set) for k in refs}
for fn, txt in text_by_file.items():
    if fn.startswith("tests/"):
        continue
    for k in refs:
        for x in re.findall(r"\b" + k + r"::(\w+)", txt):
            refs[k][x] += 1
            refs_files[k][x].add(os.path.basename(fn))
# PokemonName refs excluding pokemon.rs enum-only: refs found in logic files above.

# mega data
mega_arms = {}
for m in re.finditer(r"\(PokemonName::(\w+),\s*Items::(\w+)\)\s*=>\s*Some\(MegaEvolveData\s*\{\s*id:\s*PokemonName::(\w+),\s*types:\s*\(PokemonType::(\w+),\s*PokemonType::(\w+)\),\s*ability:\s*Abilities::(\w+),\s*base_stats:\s*\(([^)]*)\)", base_stats_txt):
    base, item, mega, t1, t2, ab, st = m.groups()
    mega_arms[(base, item)] = dict(mega=mega, types=(t1, t2), ability=ab, stats=tuple(int(x) for x in st.split(",")))
print("mega arms", len(mega_arms))
bs_entries = set(re.findall(r"PokemonName::(\w+)\s*=>\s*\(\d+", base_stats_txt))
# base_stats() panics for species not listed; list needed for mega recompute
print("base_stats entries", len(bs_entries))

# engine types for move table
def eng_type(v):
    m = re.search(r"PokemonType::(\w+)", v or "")
    return m.group(1).lower() if m else None


def eng_cat(v):
    m = re.search(r"MoveCategory::(\w+)", v or "")
    return m.group(1).lower() if m else None


def eng_num(v):
    try:
        return float(re.sub(r"[^0-9.\-]", "", v))
    except Exception:
        return None


# ---------------------------------------------------------------- Showdown dump
SD = json.loads((W / "sd.json").read_text(encoding="utf-8"))
sd_sp, sd_mv, sd_it, sd_ab = SD["species"], SD["moves"], SD["items"], SD["abilities"]
legal = {
    "species": {k for k, v in sd_sp.items() if not v.get("isNonstandard")},
    "moves": {k for k, v in sd_mv.items() if not v.get("isNonstandard")},
    "items": {k for k, v in sd_it.items() if not v.get("isNonstandard")},
    "abilities": {k for k, v in sd_ab.items() if not v.get("isNonstandard")},
}
print({k: len(v) for k, v in legal.items()})

# item id -> mega species ids (requiredItem / requiredItems)
item_to_mega = collections.defaultdict(list)
for sid, s in sd_sp.items():
    if s.get("isNonstandard"):
        continue
    if "Mega" in (s.get("forme") or ""):
        for it in ([s["requiredItem"]] if s.get("requiredItem") else []) + (s.get("requiredItems") or []):
            item_to_mega[tid(it)].append(sid)

# ---------------------------------------------------------------- usage
def load(p):
    return json.loads(Path(p).read_text(encoding="utf-8"))


seen_main = load(W / "seen_builds.json")
top = load(W / "top_builds.json")
forms = load(W / "form-sets.json")
ladders = {os.path.basename(p): load(p) for p in sorted(glob.glob(str(W / "ladders" / "ladder-*.json")))}
main_ladder_name = "ladder-20261116.json"
main = ladders[main_ladder_name]
gold_txt = (W / "gold_team.txt").read_text(encoding="utf-8")

unmapped = collections.Counter()


def sp_id(name):
    i = tid(name)
    if i in sd_sp:
        return i
    unmapped[name] += 1
    for base in sorted(sd_sp, key=len, reverse=True):  # game-name variants (Vivillon pattern, Florges colour): base species
        if base and i.startswith(base) and not sd_sp[base].get("isNonstandard") and sd_sp[base].get("baseSpecies", "") == sd_sp[base].get("name"):
            return base
    return i


def mega_of(species_id, item):
    """Showdown id of the Mega form this build turns into, or None."""
    cands = item_to_mega.get(tid(item), [])
    if not cands:
        return None
    base = sd_sp.get(species_id, {}).get("baseSpecies", species_id)
    for c in cands:
        if tid(sd_sp[c].get("baseSpecies", "")) == tid(base) or sd_sp[c].get("changesFrom") == sd_sp.get(species_id, {}).get("name"):
            return c
    return None


class B:
    __slots__ = ("sp", "item", "ability", "moves")

    def __init__(self, sp, item, ability, moves):
        self.sp, self.item, self.ability, self.moves = sp, item, ability, list(moves)


def build_from(e):
    return B(sp_id(e["species"]), e["item"], e["ability"], e["moves"])


# --- training pool: expected builds per roster slot (emulates rl2.teams.Ladder.sets/exact; ignores RECENT weighting)
seen_by_sp = {}
for key, entries in (main["seen_builds"].get("species") or {}).items():
    seen_by_sp[key] = [(build_from(e), int(e["count"])) for e in entries]
cat_by_sp = {}
for name, e in top["species"].items():
    cat_by_sp[sp_id(name)] = [(B(sp_id(name), b["item"], b["ability"], b["moves"]), max(1e-3, 10 ** b["score"])) for b in e["builds"]]
frozen_by_sp = {}
for key, e in forms["sets"].items():
    frozen_by_sp[key] = [(B(sp_id(b["species"]), b.get("item") or "", b.get("ability") or "", b.get("moves") or []), float(b.get("weight", 1.0))) for b in e["builds"]]

slot_dist = []   # per slot: list of (B, p)
slot_species = []
tier_count = collections.Counter()
roster_teams = []
for n, roster in enumerate(main["rosters"]):
    exact = (main.get("roster_builds") or [])[n] if n < len(main.get("roster_builds") or []) else {}
    team = []
    ok = True
    dists = []
    for mname in roster:
        key = sp_id(mname)
        d = None
        if key in exact and 0 <= exact[key] < len(seen_by_sp.get(key, [])):
            d = [(seen_by_sp[key][exact[key]][0], 1.0)]
            tier_count["exact"] += 1
        elif seen_by_sp.get(key):
            d = seen_by_sp[key]
            tier_count["seen"] += 1
        elif cat_by_sp.get(key):
            d = cat_by_sp[key]
            tier_count["catalog"] += 1
        elif frozen_by_sp.get(key):
            d = frozen_by_sp[key]
            tier_count["frozen"] += 1
        else:
            tier_count["none"] += 1
            ok = False
            d = []
        tot = sum(w for _, w in d) or 1.0
        dists.append((key, [(b, w / tot) for b, w in d]))
    roster_teams.append((roster, ok, dists))
print("pool tiers", dict(tier_count), "rosters", len(roster_teams), "complete", sum(1 for r in roster_teams if r[1]))

# accumulate
pool = {"species": collections.Counter(), "moves": collections.Counter(), "items": collections.Counter(), "abilities": collections.Counter()}
pool_team_species = collections.Counter()
slots = 0
for roster, ok, dists in roster_teams:
    if not ok:
        continue
    seen_sp_team = set()
    for key, d in dists:
        slots += 1
        pool["species"][key] += 1
        seen_sp_team.add(key)
        for b, p in d:
            for mv in b.moves:
                pool["moves"][tid(mv)] += p
            if b.item:
                pool["items"][tid(b.item)] += p
            if b.ability:
                pool["abilities"][tid(b.ability)] += p
            mg = mega_of(key, b.item)
            if mg:
                pool["species"][mg] += p
                ma = sd_sp[mg]["abilities"].get("0")
                if ma:
                    pool["abilities"][tid(ma)] += p
    for k in seen_sp_team:
        pool_team_species[k] += 1
n_complete = sum(1 for r in roster_teams if r[1])
for c in pool.values():
    for k in c:
        c[k] /= slots
for k in pool_team_species:
    pool_team_species[k] /= n_complete
# the mega-form species weight counted its slot-share via p (probability a slot is the mega form) -> already / slots

# --- live: seen_builds.json (count-weighted live opponent builds)
live = {"species": collections.Counter(), "moves": collections.Counter(), "items": collections.Counter(), "abilities": collections.Counter()}
live_total = 0
for key, entries in seen_main["species"].items():
    for e in entries:
        w = int(e["count"])
        live_total += w
        sid = sp_id(e["species"])
        live["species"][sid] += w
        for mv in e["moves"]:
            live["moves"][tid(mv)] += w
        live["items"][tid(e["item"])] += w
        live["abilities"][tid(e["ability"])] += w
        mg = mega_of(sid, e["item"])
        if mg:
            live["species"][mg] += w
            ma = sd_sp[mg]["abilities"].get("0")
            if ma:
                live["abilities"][tid(ma)] += w
for c in live.values():
    for k in c:
        c[k] /= live_total

# --- ladder: top_builds (157,310 ladder battles; species usage x build weight 10**score)
lad = {"species": collections.Counter(), "moves": collections.Counter(), "items": collections.Counter(), "abilities": collections.Counter()}
usage_tot = sum(e.get("usage", 0) for e in top["species"].values())
for name, e in top["species"].items():
    sid = sp_id(name)
    u = e.get("usage", 0) / usage_tot
    ws = [max(1e-3, 10 ** b["score"]) for b in e["builds"]]
    tw = sum(ws) or 1
    lad["species"][sid] += u
    for b, w in zip(e["builds"], ws):
        p = u * w / tw
        for mv in b["moves"]:
            lad["moves"][tid(mv)] += p
        lad["items"][tid(b["item"])] += p
        lad["abilities"][tid(b["ability"])] += p
        mg = mega_of(sid, b["item"])
        if mg:
            lad["species"][mg] += p
            ma = sd_sp[mg]["abilities"].get("0")
            if ma:
                lad["abilities"][tid(ma)] += p

# --- gold team
gold = {"species": set(), "moves": set(), "items": set(), "abilities": set()}
for blk in re.split(r"\n\s*\n", gold_txt.strip()):
    lines = [l.strip() for l in blk.split("\n") if l.strip()]
    head = lines[0]
    m = re.match(r"(.+?)\s*@\s*(.+)", head)
    sname, item = (m.group(1), m.group(2)) if m else (head, "")
    sid = tid(sname)
    gold["species"].add(sid)
    gold["items"].add(tid(item))
    mg = mega_of(sid, item)
    if mg:
        gold["species"].add(mg)
    for l in lines[1:]:
        if l.startswith("Ability:"):
            gold["abilities"].add(tid(l.split(":", 1)[1]))
            if mg:
                ma = sd_sp[mg]["abilities"].get("0")
                gold["abilities"].add(tid(ma))
        elif l.startswith("- "):
            gold["moves"].add(tid(l[2:]))

# --- used universe: union of all sources (every ladder roster file, seen builds, top_builds usage>0 species' builds are NOT
#     included unless a roster species), gold team
universe_defs = {}
def uni(cat):
    s_pool = set(pool[cat]) | set(live[cat])
    return s_pool
all_ladder_sp = set()
for nm, L in ladders.items():
    for r in L["rosters"]:
        for m in r:
            all_ladder_sp.add(sp_id(m))
used = {}
used["species"] = set(pool["species"]) | set(live["species"]) | gold["species"] | all_ladder_sp
for cat in ("moves", "items", "abilities"):
    used[cat] = set(pool[cat]) | set(live[cat]) | gold[cat]
# alternative: the catalog (top_builds) restricted to species in used
cat_alt = {c: set() for c in ("moves", "items", "abilities")}
for sid in used["species"]:
    for b, _ in cat_by_sp.get(sid, []):
        for mv in b.moves:
            cat_alt["moves"].add(tid(mv))
        cat_alt["items"].add(tid(b.item))
        cat_alt["abilities"].add(tid(b.ability))
# all-ladders pool (every build any ladder roster species could draw): seen-by-species union
seen_all = {c: set() for c in ("moves", "items", "abilities")}
for L in ladders.values():
    for key, entries in (L["seen_builds"].get("species") or {}).items():
        for e in entries:
            for mv in e["moves"]:
                seen_all["moves"].add(tid(mv))
            seen_all["items"].add(tid(e["item"]))
            seen_all["abilities"].add(tid(e["ability"]))
frozen_all = {c: set() for c in ("moves", "items", "abilities")}
for sid in used["species"]:
    for b, _ in frozen_by_sp.get(sid, []):
        for mv in b.moves:
            frozen_all["moves"].add(tid(mv))
        frozen_all["items"].add(tid(b.item))
        frozen_all["abilities"].add(tid(b.ability))
print("species mega-forms in used", sum(1 for x in used["species"] if "Mega" in (sd_sp.get(x, {}).get("forme") or "")))
explore = {
    "used_species_union": len(used["species"]),
    "used_species_excl_megaforms": len([x for x in used["species"] if "Mega" not in (sd_sp.get(x, {}).get("forme") or "")]),
    "pool_species": len(pool["species"]), "live_species": len(live["species"]), "all_ladder_roster_species": len(all_ladder_sp),
    "top_builds_species": len(cat_by_sp), "frozen_species": len(frozen_by_sp),
}
for cat in ("moves", "items", "abilities"):
    explore[cat + "_pool"] = len(pool[cat]); explore[cat + "_live"] = len(live[cat]); explore[cat + "_gold"] = len(gold[cat])
    explore[cat + "_union_pool_live_gold"] = len(used[cat])
    explore[cat + "_catalog_of_used_species"] = len(cat_alt[cat])
    explore[cat + "_seen_all_ladders"] = len(seen_all[cat])
    explore[cat + "_frozen_of_used"] = len(frozen_all[cat])
    explore[cat + "_union_all"] = len(used[cat] | cat_alt[cat] | seen_all[cat] | frozen_all[cat])
# Which definition hits 252/377/147/163?
allspec_catalog_species = set(cat_by_sp)
explore["species_union_incl_catalog_species"] = len(used["species"] | allspec_catalog_species)
explore["species_union_incl_catalog_and_frozen"] = len(used["species"] | allspec_catalog_species | set(frozen_by_sp))
print(json.dumps(explore, indent=1))
print("unmapped names", dict(unmapped))

# final used = union of every source we can enumerate, restricted to the Showdown-legal ones for reporting; keep
# the others flagged (illegal / unmapped)
USED = {
    "species": used["species"],
    "moves": used["moves"] | cat_alt["moves"] | seen_all["moves"] | frozen_all["moves"],
    "items": used["items"] | cat_alt["items"] | seen_all["items"] | frozen_all["items"],
    "abilities": used["abilities"] | cat_alt["abilities"] | seen_all["abilities"] | frozen_all["abilities"],
}
# the pool-reachable species set: builds the sampler could draw for roster species (tier fallback includes catalog/frozen)

# ---------------------------------------------------------------- status classification
def up(x):
    return re.sub(r"[^A-Za-z0-9]", "", x).upper()


# trivial/handler key sets
def handler_keys(o, ignore=()):
    ks = []
    for k, v in o.items():
        if k in ignore:
            continue
        if k.startswith("on") and not re.search(r"(Priority|Order|SubOrder)$", k):
            ks.append(k)
        elif k.endswith("Callback") or k == "condition":
            ks.append(k)
    return ks


def status_ability(aid):
    a = sd_ab.get(aid)
    V = up(aid)
    hk = handler_keys(a) if a else []
    in_enum = V in enums["Abilities"]
    nref = refs["Abilities"][V]
    mega_only = V and any(x["ability"] == V for x in mega_arms.values())
    if not in_enum:
        return "d", "high", "not in Abilities enum (name silently becomes NONE)", []
    if nref > 0:
        return "a", "medium", "referenced in " + ",".join(sorted(refs_files["Abilities"][V])), hk
    if hk:
        return "c", "medium", "in enum, no code reference; Showdown handlers " + ",".join(hk[:5]) + (" (+mega table only)" if mega_only else ""), hk
    return "b", "medium", "in enum, no code, no Showdown handler" + (" (mega table only)" if mega_only else ""), hk


ITEM_IGNORE = {"onPlate", "onDrive", "onMemory"}


def status_item(iid):
    it = sd_it.get(iid)
    V = up(iid)
    if V not in enums["Items"]:
        return "d", "high", "not in Items enum (becomes UNKNOWNITEM)", []
    hk = handler_keys(it, ITEM_IGNORE) if it else []
    nref = refs["Items"][V]
    if it and it.get("megaStone"):
        arms = [(b, i) for (b, i) in mega_arms if i == V]
        if arms:
            return "a", "high", "mega stone arm in mega_evolve_target " + "/".join(b for b, _ in arms), hk
        if nref > 0:
            return "a", "low", "mega stone referenced in logic but no mega_evolve_target arm", hk
        return "c", "high", "mega stone with no mega_evolve_target arm: Mega Evolution not offered", hk
    if nref > 0:
        return "a", "medium", "referenced in " + ",".join(sorted(refs_files["Items"][V])), hk
    if hk:
        trivial = set(hk) <= {"onTakeItem", "onEat", "onStart"}
        return "c", "low" if trivial else "medium", "in enum, no code reference; Showdown handlers " + ",".join(hk[:5]), hk
    return "b", "medium", "in enum, no code, no Showdown handler", hk


MOVE_LOGIC_PROPS = {  # showdown key -> label (needs code beyond the plain table fields)
    "weather": "weather", "terrain": "terrain", "pseudoWeather": "pseudoWeather", "slotCondition": "slotCondition",
    "ohko": "ohko", "damage": "fixed damage", "willCrit": "willCrit", "breaksProtect": "breaksProtect",
    "selfdestruct": "selfdestruct", "mindBlownRecoil": None, "ignoreDefensive": "ignoreDefensive",
    "ignoreAbility": "ignoreAbility", "overrideOffensiveStat": "overrideOffensiveStat",
    "overrideDefensiveStat": "overrideDefensiveStat", "overrideOffensivePokemon": "overrideOffensivePokemon",
    "overrideDefensivePokemon": "overrideDefensivePokemon", "useSourceDefensiveAsOffensive": "useSourceDefensiveAsOffensive",
    "forceSTAB": "forceSTAB", "stallingMove": "stallingMove", "isFutureMove": "futureMove", "thawsTarget": "thawsTarget",
    "multihit": "multihit", "struggleRecoil": "struggleRecoil", "sleepUsable": "sleepUsable", "ignoreEvasion": "ignoreEvasion",
    "smartTarget": "smartTarget", "hasCrashDamage": None, "onTryImmunity": None,
}
FLAG_MAP = {"contact": "contact", "protect": "protect", "sound": "sound", "punch": "punch", "bite": "bite", "bullet": "bullet",
            "pulse": "pulse", "powder": "powder", "slicing": "slicing", "wind": "wind", "reflectable": "reflectable",
            "charge": "charge", "recharge": "recharge", "heal": "heal"}
T_MAP = {"normal": "normal"}


def status_move(mid):
    m = sd_mv.get(mid)
    V = up(mid)
    reasons, conf = [], "high"
    if V not in enums["Choices"]:
        return "d", "high", "not in Choices enum (name silently becomes NONE)", ""
    if V not in blocks:
        return "d", "high", "in enum but no entry in the MOVES table", ""
    f = dict(move_fields[V])
    ov = overrides.get(V, {})
    fl = flags_of(f.get("flags"))
    for k, v in ov.items():
        if k.startswith("flags."):
            if "true" in v:
                fl.add(k.split(".", 1)[1])
            else:
                fl.discard(k.split(".", 1)[1])
        else:
            f[k] = v
    nref = refs["Choices"][V]
    hk = handler_keys(m)
    # --- data checks (high confidence when they differ)
    data = []
    e_bp = eng_num(f.get("base_power", "0")) or 0.0
    if "basePowerCallback" not in m and "damageCallback" not in m:
        if abs((m.get("basePower") or 0) - e_bp) > 0.01 and not (m["category"] == "Status" and e_bp == 0):
            data.append(f"basePower SD {m.get('basePower')} vs engine {e_bp:g}")
    e_type = eng_type(f.get("move_type"))
    if e_type is None:
        e_type = "normal"
    if (m["type"] or "").lower() != e_type:
        data.append(f"type SD {m['type']} vs engine {e_type}")
    e_cat = eng_cat(f.get("category"))
    if e_cat and e_cat != m["category"].lower():
        data.append(f"category SD {m['category']} vs engine {e_cat}")
    e_pri = eng_num(f.get("priority", "0")) or 0
    if int(e_pri) != int(m.get("priority") or 0):
        data.append(f"priority SD {m.get('priority')} vs engine {int(e_pri)}")
    e_acc = eng_num(f.get("accuracy", "100"))
    sd_acc = 100 if m.get("accuracy") is True else m.get("accuracy")
    if e_acc is not None and sd_acc is not None and abs(e_acc - sd_acc) > 0.01 and not (m.get("accuracy") is True and e_acc >= 100):
        data.append(f"accuracy SD {m.get('accuracy')} vs engine {e_acc:g}")
    sdfl = {k for k in m.get("flags", {}) if k in FLAG_MAP}
    engfl = {k for k in fl if k in FLAG_MAP.values()}
    missing_fl = sorted(sdfl - engfl - ({"heal"} if "heal" in fl else set()))
    extra_fl = sorted(engfl - sdfl - {"pivot", "drag"})
    # engine pivot/drag flags represent selfSwitch/forceSwitch
    if missing_fl:
        data.append("flags missing in engine: " + ",".join(missing_fl))
    if extra_fl:
        data.append("flags extra in engine: " + ",".join(extra_fl))
    # --- needed fields
    need = []
    sec = m.get("secondary") or (m.get("secondaries") or None)
    if sec:
        need.append("secondaries")
    self_ = m.get("self")
    if self_:
        if "boosts" in self_:
            need.append("boost")
        if "volatileStatus" in self_:
            need.append("volatile_status")
        if "sideCondition" in self_:
            need.append("side_condition")
        if set(self_) - {"boosts", "volatileStatus", "sideCondition", "chance"}:
            need.append("logic:self." + ",".join(sorted(set(self_) - {"boosts", "volatileStatus", "sideCondition"})))
    if m.get("boosts"):
        need.append("boost")
    if m.get("drain"):
        need.append("drain")
    if m.get("recoil"):
        need.append("recoil")
    if m.get("hasCrashDamage"):
        need.append("crash")
    if m.get("mindBlownRecoil"):
        need.append("heal")  # engine models Mind Blown / Steel Beam self damage as a negative heal
    if m.get("heal"):
        need.append("heal")
    if m.get("status"):
        need.append("status")
    if m.get("volatileStatus"):
        need.append("volatile_status")
    if m.get("sideCondition"):
        need.append("side_condition")
    if m.get("selfSwitch"):
        need.append("flag:pivot")
    if m.get("forceSwitch"):
        need.append("flag:drag")
    cr = m.get("critRatio")
    if cr and cr > 1:
        need.append("logic:critRatio")
    for k, lab in MOVE_LOGIC_PROPS.items():
        if lab and m.get(k):
            need.append("logic:" + lab)
    have = set(k for k, v in f.items() if v not in ("None", ""))
    for k in list(ov):
        if "." not in k:
            have.add(k)
    flag_have = {"pivot": "pivot" in fl, "drag": "drag" in fl}
    miss = []
    logic_need = []
    for n_ in need:
        if n_.startswith("logic:"):
            logic_need.append(n_[6:])
        elif n_.startswith("flag:"):
            if not flag_have[n_[5:]]:
                miss.append(n_)
        elif n_ not in have:
            miss.append(n_)
    # secondary with only `self` or onHit is covered under secondaries; boost on a move whose boost is "self" too
    # volatile / side-condition tokens the table row uses, and whether engine code references them
    toks = re.findall(r"PokemonVolatileStatus::(\w+)", f.get("volatile_status", "") + f.get("secondaries", "") + f.get("boost", "") ) \
        + re.findall(r"PokemonSideCondition::(\w+)", f.get("side_condition", ""))
    tok_ref = [t for t in toks if refs["PokemonVolatileStatus"][t] > 0 or refs["PokemonSideCondition"][t] > 0]
    tok_dead = [t for t in toks if t not in tok_ref]
    if "stallingMove" in logic_need and tok_ref:
        logic_need.remove("stallingMove")
    if ("crash" in have or m.get("mindBlownRecoil")) and "onMoveFail" in hk:
        hk = [x for x in hk if x != "onMoveFail"]
    if data:
        return "c", "high", "data differs: " + "; ".join(data), "|".join(hk)
    if not miss and not logic_need and tok_ref and not tok_dead and nref == 0:
        return "a", "low", "effect via status token(s) with engine code: " + ",".join(tok_ref) + (
            "; Showdown handlers " + ",".join(hk[:4]) if hk else ""), "|".join(hk)
    if tok_dead and nref == 0 and not miss:
        return "c", "medium", "status token(s) never referenced in engine logic: " + ",".join(tok_dead), "|".join(hk)
    if nref > 0:
        why = "referenced in " + ",".join(sorted(refs_files["Choices"][V]))
        return "a", "medium", why, "|".join(hk)
    if miss:
        return "c", "high", "Showdown effect without matching table field: " + ",".join(miss) + (
            "; also needs logic: " + ",".join(logic_need) if logic_need else ""), "|".join(hk)
    if logic_need:
        return "c", "high", "needs special logic, none found: " + ",".join(logic_need), "|".join(hk)
    if hk:
        core = any(x in have for x in ("secondaries", "boost", "status", "volatile_status", "side_condition", "heal", "drain", "recoil"))
        return "c", "low" if core else "medium", "Showdown handlers with no engine code: " + ",".join(hk[:6]), "|".join(hk)
    return "b", "high", "plain table row, nothing special in Showdown", ""


def status_species(sid):
    s = sd_sp.get(sid)
    V = up(sid)
    if V not in enums["PokemonName"]:
        return "d", "high", "not in PokemonName enum"
    if s and "Mega" in (s.get("forme") or ""):
        arms = [(k, a) for k, a in mega_arms.items() if a["mega"] == V]
        if not arms:
            return "c", "high", "Mega form exists as a name but no mega_evolve_target arm uses it (cannot Mega Evolve into it)"
        probs = []
        for (b, it), a in arms:
            ts = {t.lower() for t in a["types"] if t != "TYPELESS"}
            if ts != {t.lower() for t in s["types"]}:
                probs.append(f"types engine {sorted(ts)} vs SD {sorted(x.lower() for x in s['types'])}")
            sdab = tid(s["abilities"].get("0", ""))
            if up(sdab) != a["ability"]:
                probs.append(f"ability engine {a['ability']} vs SD {sdab}")
            bs = s["baseStats"]
            sdst = (bs["hp"], bs["atk"], bs["def"], bs["spa"], bs["spd"], bs["spe"])
            if sdst != a["stats"]:
                probs.append(f"stats engine {a['stats']} vs SD {sdst}")
            if V not in bs_entries:
                probs.append("no base_stats() row")
            if b not in bs_entries:
                pass
        if probs:
            return "c", "high", "Mega data differs: " + "; ".join(sorted(set(probs)))
        return "a", "high", "mega_evolve_target arm " + "/".join(f"{b}+{i}" for (b, i), _ in arms)
    nref = refs["PokemonName"][V]
    # battle-only formes tied to this species
    bf = [o for o in (s.get("otherFormes") or []) if sd_sp.get(tid(o), {}).get("battleOnly") or sd_sp.get(tid(o), {}).get("changesFrom")
          if "Mega" not in (sd_sp.get(tid(o), {}).get("forme") or "") and "Gmax" not in (sd_sp.get(tid(o), {}).get("forme") or "")
          and "Primal" not in (sd_sp.get(tid(o), {}).get("forme") or "")] if s else []
    bf = [o for o in bf if not sd_sp.get(tid(o), {}).get("isNonstandard")]
    if nref > 0:
        return "a", "medium", "species-specific code in " + ",".join(sorted(refs_files["PokemonName"][V]))
    if bf:
        engine_has = [o for o in bf if up(o) in enums["PokemonName"]]
        handled = [o for o in engine_has if refs["PokemonName"][up(o)] > 0]
        if not handled:
            return "c", "medium", "has battle-only forme(s) " + ",".join(bf) + " with no engine code switching to them"
    return "b", "medium", "enum entry only (stats/types are supplied by the caller)"


results = {}
def run(cat, ids, fn):
    out = {}
    for i in ids:
        out[i] = fn(i)
    return out


for _c in USED:
    USED[_c].discard("")
all_ids = {
    "species": (legal["species"] | USED["species"]),
    "moves": (legal["moves"] | USED["moves"]),
    "items": (legal["items"] | USED["items"]),
    "abilities": (legal["abilities"] | USED["abilities"]),
}
res = {
    "species": {i: status_species(i) for i in all_ids["species"]},
    "moves": {i: status_move(i) for i in all_ids["moves"]},
    "items": {i: status_item(i) for i in all_ids["items"]},
    "abilities": {i: status_ability(i)[:3] for i in all_ids["abilities"]},
}

# ---------------------------------------------------------------- write CSVs
names = {"species": sd_sp, "moves": sd_mv, "items": sd_it, "abilities": sd_ab}
summary = {}
for cat in ("species", "moves", "items", "abilities"):
    rows = []
    for i in all_ids[cat]:
        r = res[cat][i]
        st, conf, why = r[0], r[1], r[2]
        d = names[cat].get(i, {})
        rows.append(dict(
            name=d.get("name", i), id=i, used=int(i in USED[cat]), legal=int(i in legal[cat]),
            pool_share=round(pool[cat].get(i, 0), 6), live_share=round(live[cat].get(i, 0), 6), ladder_share=round(lad[cat].get(i, 0), 6),
            team_share=round(pool_team_species.get(i, 0), 4) if cat == "species" else "",
            in_gold=int(i in gold[cat]), status=st, confidence=conf, reason=why))
    rows.sort(key=lambda r: (-r["used"], -r["pool_share"], -r["live_share"], -r["ladder_share"], r["name"]))
    with open(OUT / f"03-coverage-{cat}.csv", "w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    used_rows = [r for r in rows if r["used"]]
    cnt = collections.Counter(r["status"] for r in used_rows)
    cnt_legal = collections.Counter(r["status"] for r in rows if r["legal"])
    summary[cat] = dict(used=len(used_rows), by_status=dict(cnt), legal_by_status=dict(cnt_legal))
    for key in ("pool_share", "live_share", "ladder_share"):
        tot = sum(r[key] for r in used_rows)
        cov = sum(r[key] for r in used_rows if r["status"] in ("a", "b"))
        covA = sum(r[key] for r in used_rows if r["status"] == "a")
        summary[cat][key + "_total"] = round(tot, 4)
        summary[cat][key + "_covered_ab"] = round(cov, 4)
        summary[cat][key + "_status_share"] = {s: round(sum(r[key] for r in used_rows if r["status"] == s), 4) for s in "abcd"}
    gaps = [r for r in rows if r["status"] in ("c", "d") and r["used"]]
    json.dump(gaps, open(OUT / f"03-coverage-gaps-{cat}.json", "w"), indent=0)
summary["_explore"] = explore
summary["_pool"] = dict(rosters=len(roster_teams), complete=n_complete, slots=slots, tiers=dict(tier_count))
summary["_live"] = dict(builds=sum(len(v) for v in seen_main["species"].values()), observations=live_total,
                        battles_listed=len(seen_main.get("battles", [])))
summary["_ladder"] = dict(ladder_battles=top.get("ladder_battles"), species=len(top["species"]), usage_total=usage_tot)
summary["_ladders_files"] = {k: len(v["rosters"]) for k, v in ladders.items()}
summary["_engine"] = dict(enum_sizes={k: len(v) for k, v in enums.items()}, move_table=len(blocks), overrides=len(overrides),
                          mega_arms=len(mega_arms), logic_refs={k: len(v) for k, v in refs.items()})
summary["_legal"] = {k: len(v) for k, v in legal.items()}
summary["_unmapped"] = dict(unmapped)
json.dump(summary, open(OUT / "03-coverage-summary.json", "w"), indent=1)
print(json.dumps({k: v for k, v in summary.items() if not k.startswith("_")}, indent=1))

# ---------------------------------------------------------------- markdown fragments
def md_table(rows, cols):
    L = ["| " + " | ".join(cols) + " |", "|" + "|".join("---" for _ in cols) + "|"]
    for r in rows:
        L.append("| " + " | ".join(str(r[c]) for c in cols) + " |")
    return "\n".join(L)

frag = []
allrows = {}
for cat in ("species", "moves", "items", "abilities"):
    rows = list(csv.DictReader(open(OUT / f"03-coverage-{cat}.csv", encoding="utf-8")))
    rows = [r for r in rows if r["used"] == "1"]
    allrows[cat] = rows
    top50 = []
    for r in rows[:50]:
        top50.append(dict(name=r["name"], pool=f"{float(r['pool_share'])*100:.2f}%", live=f"{float(r['live_share'])*100:.2f}%",
                          ladder=f"{float(r['ladder_share'])*100:.2f}%", gold="yes" if r["in_gold"] == "1" else "", status=r["status"],
                          conf=r["confidence"], note=r["reason"][:110].replace("|", "/")))
    frag.append(f"### Top 50 {cat}\n\n" + md_table(top50, ["name", "pool", "live", "ladder", "gold", "status", "conf", "note"]) + "\n")
gaps = []
for cat, rows in allrows.items():
    for r in rows:
        if r["status"] in ("c", "d"):
            gaps.append(dict(cat=cat, name=r["name"], pool=float(r["pool_share"]), live=float(r["live_share"]), ladder=float(r["ladder_share"]),
                             gold="yes" if r["in_gold"] == "1" else "", status=r["status"], conf=r["confidence"], note=r["reason"][:120].replace("|", "/")))
gaps.sort(key=lambda g: -g["pool"])
json.dump(gaps, open(OUT / "03-coverage-gaplist.json", "w"), indent=0)
g30 = [dict(rank=i + 1, **{**g, "pool": f"{g['pool']*100:.2f}%", "live": f"{g['live']*100:.2f}%", "ladder": f"{g['ladder']*100:.2f}%"}) for i, g in enumerate(gaps[:30])]
frag.append("### Top 30 gaps\n\n" + md_table(g30, ["rank", "cat", "name", "pool", "live", "ladder", "gold", "status", "conf", "note"]) + "\n")
(OUT / "03-coverage-fragments.md").write_text("\n".join(frag), encoding="utf-8")
