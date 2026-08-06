#!/usr/bin/env python3
"""3.5a mock generator — two candidate surfaces at the raised-floor size (200x50).

Generates ANSI mock frames (view with `cat` in a terminal >= 200 cols) and
matching HTML fragments for the review artifact. Mocks are DESIGN ARTIFACTS:
they render the owner's actual 2026-08-05 live game (seed 1785980164964555000)
at turn 1, byte-honest to the recorded setup, so the owner can judge them
against the table he just played. Nothing here touches the CLI implementation.

Floor candidate demonstrated: 200 columns x ~50 rows. Every emitted line is
asserted <= FLOOR_W visible columns.
"""
import re, html

FLOOR_W = 200

# ---------------------------------------------------------------- palette ---
R = "\x1b[0m"
def c(code): return f"\x1b[{code}m"
BOLD, DIM, INV = c(1), c(2), c(7)
RED, BLUE = c(91), c(94)              # player colors
GREEN, CYAN, WHITE, YELLOW, MAG, GRAY = c(32), c(96), c(97), c(93), c(95), c(90)
COPPER = c("38;5;172")                 # copper ingot orange-brown
ORANGE = c("38;5;208")                 # NL flag tone
VP = c("38;5;75")                      # VP star blue
YEN = c("38;5;220")                    # yen coin gold
FLAG = {"US": CYAN, "DE": YELLOW, "NL": ORANGE, "GB": MAG, "FR": c("38;5;69")}
GOOD = {"seafood": CYAN, "tea": GREEN, "silk": WHITE, "copper": COPPER, "imports": YELLOW}

ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")
def vis(s): return len(ANSI_RE.sub("", s))
def pad(s, w): return s + " " * (w - vis(s))

# ------------------------------------------------------------------- data ---
ORDERS = {  # id: (country, [(good,n)...], rewards-string-parts)
 1:("US",[("seafood",2),("tea",1)],[("vp",1),("build",1)]),
 2:("FR",[("seafood",1),("tea",2)],[("vp",3)]),
 3:("GB",[("seafood",1),("copper",2)],[("vp",4),("yen",1)]),
 4:("DE",[("seafood",1),("tea",1),("silk",2)],[("vp",5),("build",1)]),
 5:("FR",[("tea",2),("copper",1)],[("vp",1),("imp",1)]),
 6:("NL",[("seafood",2),("tea",1),("copper",1)],[("vp",2),("yen",1),("imp",1)]),
 7:("FR",[("tea",3),("copper",1)],[("vp",3),("imp",1)]),
 8:("NL",[("tea",1),("silk",1),("copper",1)],[("vp",3),("imp",1)]),
 9:("DE",[("seafood",1),("silk",3)],[("vp",2),("yen",1),("imp",1)]),
 10:("GB",[("seafood",1),("tea",1),("copper",2)],[("vp",4),("asst",2)]),
 11:("NL",[("seafood",1),("tea",1),("silk",1)],[("vp",5)]),
 12:("US",[("seafood",3),("tea",1),("silk",2)],[("vp",9),("yen",1)]),
 13:("GB",[("seafood",2),("silk",1),("copper",2)],[("vp",8),("imp",1)]),
 14:("DE",[("tea",2),("silk",2),("copper",1)],[("vp",6),("yen",2)]),
 15:("GB",[("tea",1),("silk",1),("copper",3)],[("vp",9),("imp",1)]),
 16:("US",[("seafood",3),("tea",2),("copper",1)],[("vp",8),("imp",1)]),
 17:("DE",[("seafood",2),("silk",2),("copper",1)],[("vp",8),("yen",1)]),
 18:("US",[("seafood",3),("copper",2)],[("vp",6),("imp",1)]),
 19:("NL",[("seafood",1),("tea",1),("silk",1),("copper",1)],[("vp",8)]),
 20:("GB",[("tea",2),("silk",1),("copper",2)],[("vp",8),("imp",1)]),
 21:("NL",[("seafood",2),("tea",2),("silk",2)],[("vp",11)]),
 22:("US",[("seafood",4),("copper",1)],[("vp",8)]),
 23:("FR",[("tea",3),("silk",2),("copper",1)],[("vp",9),("imp",1)]),
 24:("GB",[("seafood",2),("silk",1),("copper",3)],[("vp",7),("yen",2),("imp",1)]),
 25:("DE",[("tea",1),("silk",3)],[("vp",4),("imp",1)]),
 26:("FR",[("seafood",1),("tea",3)],[("vp",1),("yen",2)]),
 27:("GB",[("seafood",2),("copper",2)],[("vp",1),("build",1),("yen",2)]),
 28:("US",[("seafood",4),("silk",1)],[("vp",3),("imp",1),("build",1)]),
 29:("FR",[("tea",4),("silk",1)],[("vp",8)]),
 30:("FR",[("tea",3),("silk",2)],[("vp",4),("yen",2)]),
 31:("US",[("seafood",3),("copper",1)],[("vp",2),("asst",2)]),
 32:("GB",[("seafood",1),("tea",1),("copper",2)],[("vp",5),("imp",1)]),
 33:("DE",[("tea",3),("silk",3)],[("vp",11)]),
 34:("FR",[("tea",3),("copper",2)],[("vp",9),("imp",1)]),
 35:("US",[("seafood",4),("silk",2)],[("vp",7),("yen",2)]),
 36:("DE",[("tea",2),("silk",4)],[("vp",5),("yen",2),("build",1)]),
}
CNAME = {"US":"USA","DE":"GERMANY","NL":"NETHERLANDS","GB":"GT BRITAIN","FR":"FRANCE"}
GSHORT = {"seafood":"Seafood","tea":"Tea","silk":"Silk","copper":"Copper"}

def reward_str(parts):
    out = []
    for k, n in parts:
        if k == "vp":    out.append(f"{VP}★{n} VP{R}")
        elif k == "yen": out.append(f"{YEN}¥{n}{R}")
        elif k == "imp": out.append(f"{YELLOW}{n} Import{R}")
        elif k == "build": out.append(f"{WHITE}+Shophouse{R}")
        elif k == "asst":  out.append(f"{WHITE}{n} Assistants{R}")
    return "  ".join(out)

def goods_str(reqs):
    return " ".join(f"{GOOD[g]}{n} {GSHORT[g]}{R}" for g, n in reqs)

# ------------------------------------------------------- mini order card ----
CARD_W = 24
def order_card(oid, dim=False, note=None, sel=None):
    co, reqs, rew = ORDERS[oid]
    d = DIM if dim else ""
    fl = FLAG[co]
    top_label = f"{d}{fl}{BOLD}{CNAME[co]}{R}{d}"
    idtag = f"{GRAY}O{oid:02d}{R}{d}"
    seltag = f"{d}{BOLD}[{sel}]{R}{d} " if sel else ""
    l1 = pad(f"{d}┌ {seltag}{top_label}", CARD_W - 6) + f" {idtag}{d} ┐{R}"
    def strip_if_dim(s): return d + ANSI_RE.sub("", s) if dim else s
    ga = goods_str(reqs[:2]); gb = goods_str(reqs[2:]) if len(reqs) > 2 else ""
    l2 = f"{d}│ {pad(strip_if_dim(ga), CARD_W-4)}{d} │{R}"
    l3 = f"{d}│ {pad(strip_if_dim(gb), CARD_W-4)}{d} │{R}"
    parts = [reward_str([p]) for p in rew]
    ra, rb = "", ""
    for p in parts:
        if not ra or vis(ra) + 2 + vis(p) <= CARD_W - 4:
            ra = p if not ra else f"{ra}  {p}"
        else:
            rb = p if not rb else f"{rb}  {p}"
    l4 = f"{d}│ {pad(strip_if_dim(ra), CARD_W-4)}{d} │{R}"
    line5 = d + (note or "")[:CARD_W-4] if (dim or note) else strip_if_dim(rb)
    l5 = f"{d}│ {pad(line5, CARD_W-4)}{d} │{R}"
    l6 = f"{d}└{'─'*(CARD_W-2)}┘{R}"
    return [l1, l2, l3, l4, l5, l6]

CARD_H = 6
def row_of_cards(cards):
    lines = ["" for _ in range(CARD_H)]
    for cl in cards:
        for i in range(CARD_H):
            lines[i] += pad(cl[i], vis(cl[i])) + " "
    return [l.rstrip() for l in lines]

# ------------------------------------------------------------- tile boxes ---
TILE_W = 30
TOKENS = {"A00":"1 Import","A01":"3 Tea","A02":"3 Seafood","A03":"3 Yen","A04":"2 Copper",
          "A05":"2 Silk","A06":"1 Import","A07":"3 Tea","A08":"3 Seafood","A09":"3 Yen"}
TOKCOL = {"Import":YELLOW,"Tea":GREEN,"Seafood":CYAN,"Yen":YEN,"Copper":COPPER,"Silk":WHITE}
SITES = {"A00":2,"A01":3,"A02":4,"A03":5,"A04":6,"A05":7,"A06":8,"A07":9,"A08":0,"A09":23}
NAMES = {"A00":"COPPER MINE","A01":"BANK","A02":"PORT A","A03":"LABORATORY A",
         "A04":"EMPLOYMENT AGENCY","A05":"CUSTOMS","A06":"SILK MILL","A07":"CHURCH",
         "A08":"TEA PLANTATION","A09":"FISHING GROUND"}

def tile(aid):
    name = NAMES[aid]; tok = TOKENS[aid]; tokc = TOKCOL[tok.split()[1]]
    l1 = f"┌ {GRAY}{aid}{R} {BOLD}{name}{R} " + "─" * (TILE_W - 5 - len(aid) - len(name)) + "┐"
    l2 = f"│ {pad(f'5-Power: {tokc}{tok}{R}  {GRAY}Site {SITES[aid]}{R}', TILE_W-4)} │"
    l3 = f"│ {pad(f'President: {DIM}none{R}', TILE_W-4)} │"
    l4 = f"│ {pad(f'{RED}Red 0{R} · {BLUE}Blue 0{R} assistants', TILE_W-4)} │"
    l5 = f"│ {pad(f'Shophouses {DIM}····{R} Trade {DIM}·{R}', TILE_W-4)} │"
    l6 = "└" + "─" * (TILE_W - 2) + "┘"
    return [l1, l2, l3, l4, l5, l6]

def board_rows():
    rows = [["A00","A01","A02","A03"], ["A04","A05","A06"], ["A07","A08"], ["A09"]]
    out = []
    for i, row in enumerate(rows):
        offset = " " * (i * (TILE_W // 2))
        tiles = [tile(a) for a in row]
        for li in range(6):
            out.append(offset + "".join(t[li] for t in tiles))
    return out

# -------------------------------------------------------- dashboard panels --
DASH_W = 74
def hdr(t):  return f"{GRAY}─ {R}{BOLD}{t}{R} {GRAY}{'─'*(DASH_W-4-len(t))}{R}"

def dashboard():
    L = []
    L.append(hdr("PLAYERS"))
    L.append(f"{RED}{BOLD}▶ RED — your turn{R}        {YEN}¥3{R}   {VP}0 VP{R}")
    L.append(f"  Goods  {COPPER}1 Copper{R} · {WHITE}1 Silk{R} · {GREEN}1 Tea{R} · {CYAN}1 Seafood{R} · {DIM}0 Imports{R}")
    L.append(f"  Assistants  {BOLD}8{R} in hand · 0 on board · 12 in warehouse")
    L.append(f"  Built  0 shophouses · 0 trading houses {GRAY}(next take costs ¥4){R}")
    L.append(f"  President {DIM}in hand{R} · 1 order in hand")
    L.append("")
    L.append(f"{BLUE}{BOLD}  BLUE{R}                    {YEN}¥4{R}   {VP}0 VP{R}")
    L.append(f"  Goods  {COPPER}1 Copper{R} · {WHITE}1 Silk{R} · {GREEN}1 Tea{R} · {CYAN}1 Seafood{R} · {DIM}0 Imports{R}")
    L.append(f"  Assistants  {BOLD}8{R} in hand · 0 on board · 12 in warehouse")
    L.append(f"  Built  0 shophouses · 0 trading houses {GRAY}(next take costs ¥4){R}")
    L.append(f"  President {DIM}in hand{R} · 1 order in hand {GRAY}(hidden){R}")
    L.append("")
    L.append(hdr("TRACKS  (2-player: filled at 4 · D = neutral dummy)"))
    church = (f"  Church   faith "
              f"{DIM}[4]{R}{DIM}[5]{R}{DIM}[6]{R}{INV}[7 D]{R}{DIM}[8]{R}{DIM}[9]{R}{DIM}[10]{R}{DIM}[11]{R}"
              f"   1/4 filled")
    L.append(church)
    customs = (f"  Customs  cost  "
               f"{DIM}[¥1]{R}{DIM}[¥1]{R}{DIM}[¥2]{R}{INV}[¥2 D]{R}{DIM}[¥3]{R}{DIM}[¥3]{R}{DIM}[¥4]{R}"
               f"      1/4 filled")
    L.append(customs)
    L.append("")
    L.append(hdr("ACHIEVEMENTS  (one per category — first / later VP)"))
    L.append(f"  {BOLD}A{R} · Copper 5      own 5 copper                {VP}8 / 6 VP{R}   {DIM}unclaimed{R}")
    L.append(f"  {BOLD}B{R} · Money 10      own 10 yen                  {VP}9 / 7 VP{R}   {DIM}unclaimed{R}")
    L.append(f"  {BOLD}C{R} · Production 4  buildings in 4 production   {VP}10 / 8 VP{R}  {DIM}unclaimed{R}")
    L.append("")
    L.append(hdr("YOUR HAND  ('hand' expands · 'privacy' hides)"))
    for ln in order_card(2, note="your starting order"):
        L.append("  " + ln)
    return L

# ------------------------------------------------------------ main frame ----
def main_frame():
    L = []
    title = f"{BOLD}Y O K O H A M A{R}   {GRAY}live tracking · seed …964555000 · autosaved{R}"
    L.append(pad("", (FLOOR_W - vis(title)) // 2) + title)
    L.append("")
    board = board_rows()
    dash = dashboard()
    n = max(len(board), len(dash))
    board += [""] * (n - len(board)); dash += [""] * (n - len(dash))
    for b, d in zip(board, dash):
        L.append(pad(b, 124) + d)
    L.append("")
    # port strip
    L.append(hdr_wide("PORT A — order cards on display  ('ports' expands)"))
    for ln in row_of_cards([order_card(1), order_card(3), order_card(7),
                            order_card(14), order_card(15), order_card(16)]):
        L.append(ln)
    lab = (f"{GRAY}─ {R}{BOLD}LAB A — technology on display{R}  "
           f"[1] Language School {YEN}¥3{R} · [2] Telegram {YEN}¥6{R} · [3] Telephone {YEN}¥7{R} · "
           f"[4] Telephone {YEN}¥7{R} · [5] Newspaper {YEN}¥7{R} · [6] Newspaper {YEN}¥7{R}"
           f"   {GRAY}('labs' expands){R}")
    L.append(lab)
    L.append("")
    L.append(f"{GRAY}{'─'*FLOOR_W}{R}")
    phases = ["Additional", "Placement", "Movement", "Area Action", "5-Power", "Building", "Recovery", "Additional", "End"]
    chain = f"{BOLD}TURN 1 · {RED}RED{R}{BOLD}'s turn{R}   "
    for i, p in enumerate(phases):
        chain += (f"{INV}{BOLD} {p.upper()} {R}" if i == 0 else f"{DIM}{p}{R}")
        if i < len(phases) - 1: chain += f"{GRAY} ▸ {R}"
    L.append(chain)
    L.append("")
    L.append(f"{BOLD}YOU ARE DECIDING{R} · an additional action before Placement — you own no technologies yet, so there is nothing to use.")
    L.append(f"   {BOLD}[1]{R} End the additional-action phase and move on to {BOLD}Placement{R}")
    L.append(f"{GRAY}(only option — press Enter to apply · 'back' undoes · 'view' opens any panel · '?' opens the card index){R}")
    L.append(f"{BOLD}> {R}")
    return L

def hdr_wide(t, rest=""):
    lead = f"{GRAY}─ {R}{BOLD}{t}{R} {GRAY}"
    fill = FLOOR_W - vis(lead) - vis(rest) - 1
    return lead + "─" * max(fill, 1) + R + " " + rest

# ------------------------------------------------------------ order index ---
def order_index():
    L = []
    t = f"{BOLD}ORDER CARD INDEX{R} — all 36, grouped by flag · {GRAY}small tag = engine id (for saves & reports; never required){R}"
    L.append(t)
    L.append(f"{GRAY}Recording: Port A, slot 3 of 6 — which card is physically in the slot? Type its [number]. 'f silk' filters by a good · 'f 8vp' by reward · Enter cancels.{R}")
    L.append("")
    placed = {2: "Seat 0's deal", 4: "Seat 0's deal", 6: "Seat 1's deal", 8: "Seat 1's deal",
              1: "Port A · slot 1", 3: "Port A · slot 2"}
    sel = 0
    by_country = {}
    for oid in sorted(ORDERS):
        by_country.setdefault(ORDERS[oid][0], []).append(oid)
    for co in ["DE", "FR", "GB", "NL", "US"]:
        ids = by_country[co]
        L.append(f"{FLAG[co]}{BOLD}■ {CNAME[co]}{R}  {GRAY}({len(ids)} cards){R}")
        cards = []
        for oid in ids:
            sel += 1
            if oid in placed:
                cards.append(order_card(oid, dim=True, note=f"= {placed[oid]}", sel=sel))
            else:
                cards.append(order_card(oid, sel=sel))
        for ln in row_of_cards(cards):
            L.append(ln)
        L.append("")
    L.append(f"{GRAY}Dimmed cards are already recorded elsewhere and cannot be entered twice — each says where it is.{R}")
    L.append(f"{BOLD}> {R}")
    return L

# ------------------------------------------------------------- html emit ----
CSS_MAP = {"1":"font-weight:bold","2":"opacity:.45","7":"background:#d8dee9;color:#10151c",
           "32":"color:#69c25f","90":"color:#5c6773","91":"color:#ff5f56","93":"color:#f2c14e",
           "94":"color:#6f9bff","95":"color:#d78ef0","96":"color:#5fd7e8","97":"color:#e8ecf2",
           "38;5;172":"color:#d78742","38;5;208":"color:#ff9640","38;5;75":"color:#5fa8ff",
           "38;5;220":"color:#ffd75f","38;5;69":"color:#5f87ff"}
def ansi_to_html(lines):
    out = []
    for line in lines:
        res, open_spans = "", 0
        idx = 0
        for m in ANSI_RE.finditer(line):
            res += html.escape(line[idx:m.start()])
            code = m.group(0)[2:-1]
            if code in ("0", ""):
                res += "</span>" * open_spans; open_spans = 0
            else:
                css = CSS_MAP.get(code)
                if css is None and ";" not in code:
                    css = CSS_MAP.get(code.lstrip("0"))
                res += f'<span style="{css or ""}">'; open_spans += 1
            idx = m.end()
        res += html.escape(line[idx:]) + "</span>" * open_spans
        out.append(res)
    return "\n".join(out)

# ------------------------------------------------------------------ emit ----
def emit(name, lines):
    for i, ln in enumerate(lines):
        assert vis(ln) <= FLOOR_W, f"{name}:{i} is {vis(ln)} cols: {ANSI_RE.sub('', ln)[:80]}"
    with open(f"docs/plans/2026-08-05-phase3.5a-mocks/{name}.ans", "w") as f:
        f.write("\n".join(lines) + "\n")
    with open(f"docs/plans/2026-08-05-phase3.5a-mocks/{name}.html.frag", "w") as f:
        f.write(ansi_to_html(lines))
    print(f"{name}: {len(lines)} rows, max width {max(vis(l) for l in lines)}")

if __name__ == "__main__":
    emit("main-frame", main_frame())
    emit("order-index", order_index())
