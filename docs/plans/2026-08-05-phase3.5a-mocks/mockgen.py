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
def _tile_guard(rows, who):
    for r in rows:
        assert vis(r) == TILE_W, f"{who}: tile line {vis(r)} != {TILE_W}: {ANSI_RE.sub('', r)}"
    return rows

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
    return _tile_guard([l1, l2, l3, l4, l5, l6], aid)

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
    L.append(f"  Shophouses  {BOLD}2{R} in hand · 6 in warehouse · 0 built")
    L.append(f"  Trade houses  0 in hand · 4 in warehouse · 0 built {GRAY}(next take ¥4){R}")
    L.append(f"  President {DIM}in hand{R} · 1 order in hand")
    L.append("")
    L.append(f"{BLUE}{BOLD}  BLUE{R}                    {YEN}¥4{R}   {VP}0 VP{R}")
    L.append(f"  Goods  {COPPER}1 Copper{R} · {WHITE}1 Silk{R} · {GREEN}1 Tea{R} · {CYAN}1 Seafood{R} · {DIM}0 Imports{R}")
    L.append(f"  Assistants  {BOLD}8{R} in hand · 0 on board · 12 in warehouse")
    L.append(f"  Shophouses  {BOLD}2{R} in hand · 6 in warehouse · 0 built")
    L.append(f"  Trade houses  0 in hand · 4 in warehouse · 0 built {GRAY}(next take ¥4){R}")
    L.append(f"  President {DIM}in hand{R} · 1 order in hand {GRAY}(hidden){R}")
    L.append("")
    L.append(hdr("TRACKS  (ends at 4 of YOUR pawns · D = dummy, not counted)"))
    church = (f"  Church   faith "
              f"{DIM}[4]{R}{DIM}[5]{R}{DIM}[6]{R}{INV}[7 D]{R}{DIM}[8]{R}{DIM}[9]{R}{DIM}[10]{R}{DIM}[11]{R}"
              f" 0/4 · {VP}★3–14{R}+mv")
    L.append(church)
    customs = (f"  Customs  pay   "
               f"{DIM}[1]{R}{DIM}[1]{R}{DIM}[2]{R}{INV}[2 D]{R}{DIM}[3]{R}{DIM}[3]{R}{DIM}[4]{R} imports"
               f" 0/4 · {VP}★4–22{R}")
    L.append(customs)
    L.append(f"  {GRAY}('tracks' renders every space: cost · VP · rider · occupant){R}")
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
    L.append(f"{GRAY}(only option — press Enter · nothing is final until you apply it · 'back' undoes the last applied entry · ↑ recalls what you typed · 'view' any surface · '?' the card index){R}")
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

# ------------------------------------------------------------ tracks view ---
SP_W = 23
def space_card(top, lines, occupant, dummy=False):
    l1 = pad(f"┌ {BOLD}{top}{R} ", SP_W - 1) + "┐"
    body = [pad(f"│ {ln}", SP_W - 1) + "│" for ln in lines]
    occ = (f"{INV} D · neutral dummy {R}" if dummy else f"{DIM}{occupant}{R}")
    l4 = pad(f"│ {occ}", SP_W - 1) + "│"
    l5 = "└" + "─" * (SP_W - 2) + "┘"
    return [l1] + body + [l4, l5]

CHURCH_SP = [(4,3,"+1 movement"),(5,4,"+1 movement"),(6,5,"+2 movement"),(7,6,"+2 movement"),
             (8,7,"+2 movement"),(9,8,"+2 movement"),(10,11,"+3 movement"),(11,14,"+3 movement")]
CUSTOMS_SP = [(1,1,5,"+2 warehouse takes"),(2,1,4,"+2 yen"),(3,2,10,"+1 warehouse take"),
              (4,2,9,"+1 yen"),(5,3,16,""),(6,3,15,""),(7,4,22,"")]

def tracks_view():
    L = []
    L.append(f"{BOLD}TRACKS — full view{R}   {GRAY}the 'view' pattern: every frame panel is a summary; its full surface renders like this (ports · labs · tracks · sites · goals · hand · log){R}")
    L.append("")
    L.append(f"{BOLD}CHURCH{R} — pay faith, place an assistant on a space, take its printed reward. {GRAY}Ends at 4 REAL assistants (2P; the dummy never advances the end-count) · majority 6 / 3 VP — the dummy competes.{R}")
    cards = []
    for f_, vp, rider in CHURCH_SP:
        cards.append(space_card(f"faith {f_}", [f"{VP}★{vp} VP{R}", f"{GREEN}{rider}{R}"],
                                "empty", dummy=(f_ == 7)))
    for ln in row_of_cards_h(cards, 5):
        L.append(ln)
    L.append("")
    L.append(f"{BOLD}CUSTOMS{R} — pay the printed cost in imported goods (power caps it), take the reward. {GRAY}Ends at 4 REAL assistants — the dummy never counts · majority 8 / 4 VP, dummy competes.{R}")
    cards = []
    for sp, cost, vp, rider in CUSTOMS_SP:
        body = [f"{YELLOW}pay {cost} import{'s' if cost>1 else ''}{R}", f"{VP}★{vp} VP{R}"]
        body.append(f"{GREEN}{rider}{R}" if rider else f"{DIM}—{R}")
        cards.append(space_card(f"space {sp}", body, "empty", dummy=(sp == 4)))
    for ln in row_of_cards_h(cards, 6):
        L.append(ln)
    L.append("")
    L.append(f"{GRAY}A payment must EXACTLY match a space's printed cost; both spaces of a pair filled means that payment is unplaceable.{R}")
    L.append(f"{BOLD}> {R}{GRAY}Enter returns to the game{R}")
    return L

def row_of_cards_h(cards, h):
    lines = ["" for _ in range(h)]
    for cl in cards:
        cl = cl + [pad("", vis(cl[0]))] * (h - len(cl))
        for i in range(h):
            lines[i] += pad(cl[i], vis(cl[0])) + " "
    return [l.rstrip() for l in lines]

# -------------------------------------------------------------- 4P study ----
YEL, GRN = c(93), c(92)
N4 = {"B00":"COPPER MINE","B01":"BANK","B02":"PORT A","B03":"CHINATOWN","B04":"SILK MILL","B05":"LABORATORY A",
      "B06":"FISHING GROUND","B07":"CUSTOMS","B08":"CANAL","B09":"EMPLOYMENT AGENCY","B10":"LABORATORY B",
      "B11":"TEA PLANTATION","B12":"CHURCH","B13":"PORT B","B14":"SILK MILL","B15":"COPPER MINE",
      "B16":"TEA PLANTATION","B17":"FISHING GROUND"}
T4 = {"B00":"1 Import","B01":"3 Tea","B02":"1 Import","B03":"2 Copper","B04":"1 Import","B05":"3 Yen",
      "B06":"3 Seafood","B07":"2 Silk","B09":"1 Import","B10":"3 Yen","B11":"3 Tea","B12":"1 Import",
      "B13":"1 Import","B14":"2 Silk","B15":"2 Copper","B16":"3 Seafood","B17":"1 Import"}

def tile4(aid):
    name = N4[aid]
    if name == "CANAL":
        l1 = f"┌ {GRAY}{aid}{R} {BOLD}CANAL{R} " + "─" * (TILE_W - 5 - len(aid) - 5) + "┐"
        return _tile_guard([l1,
                f"│ {pad(f'{DIM}transit only — no pawns,{R}', TILE_W-4)} │",
                f"│ {pad(f'{DIM}no tokens, no buildings{R}', TILE_W-4)} │",
                f"│ {pad(f'{YEN}¥1{R} to the bank / crossing', TILE_W-4)} │",
                f"│ {pad('', TILE_W-4)} │",
                "└" + "─" * (TILE_W - 2) + "┘"], aid)
    tok = T4[aid]; tokc = TOKCOL[tok.split()[1]]
    l1 = f"┌ {GRAY}{aid}{R} {BOLD}{name}{R} " + "─" * (TILE_W - 5 - len(aid) - len(name)) + "┐"
    return _tile_guard([l1,
            f"│ {pad(f'5-Power: {tokc}{tok}{R} {GRAY}Site 12{R}', TILE_W-4)} │",
            f"│ {pad(f'President: {DIM}none{R}', TILE_W-4)} │",
            f"│ {pad(f'{RED}R0{R} {BLUE}B0{R} {YEL}Y0{R} {GRN}G0{R} assistants', TILE_W-4)} │",
            f"│ {pad(f'Shophouses {DIM}····{R} Trade {DIM}·{R}', TILE_W-4)} │",
            "└" + "─" * (TILE_W - 2) + "┘"], aid)

def player_col(color, name, lead, yen, hidden=True):
    COLW = 48
    mark = f"{color}{BOLD}▶ {name} — your turn{R}" if lead else f"{color}{BOLD}  {name}{R}"
    return [pad(mark, COLW - 14) + f"{YEN}¥{yen}{R}   {VP}0 VP{R}" + " " * 2,
            pad(f"  Goods  {COPPER}1 Copper{R} · {WHITE}1 Silk{R} · {GREEN}1 Tea{R}", COLW),
            pad(f"         {CYAN}1 Seafood{R} · {DIM}0 Imports{R}", COLW),
            pad(f"  Assistants  8 hand · 0 board · 12 warehouse", COLW),
            pad(f"  Shophouses  2 hand · 6 warehouse · 0 built", COLW),
            pad(f"  Trade houses 0 hand · 4 warehouse · 0 built", COLW),
            pad(f"  President in hand · 1 order" + (" (hidden)" if hidden else ""), COLW)]

def frame_4p():
    L = []
    t = f"{BOLD}Y O K O H A M A{R}   {GRAY}4-PLAYER LAYOUT STUDY — illustrative board · the dashboard moves BELOW the board at this count{R}"
    L.append(pad("", max((FLOOR_W - vis(t)) // 2, 0)) + t)
    L.append("")
    rows = [["B00","B01","B02","B03","B04","B05"], ["B06","B07","B08","B09","B10"],
            ["B11","B12","B13","B14"], ["B15","B16","B17"]]
    for i, row in enumerate(rows):
        offset = " " * (i * (TILE_W // 2))
        tiles = [tile4(a) for a in row]
        for li in range(6):
            L.append(offset + "".join(tt[li] for tt in tiles))
    L.append("")
    L.append(hdr_wide("PLAYERS"))
    cols = [player_col(RED, "RED", True, 3, hidden=False),
            player_col(BLUE, "BLUE", False, 4),
            player_col(YEL, "YELLOW", False, 4),
            player_col(GRN, "GREEN", False, 4)]
    for li in range(7):
        L.append(" ".join(pad(col[li], 48) for col in cols).rstrip())
    L.append("")
    tr = (f"{GRAY}─ {R}{BOLD}TRACKS{R}  Church {DIM}[4][5][6][7][8][9][10][11]{R} 0/6 · "
          f"Customs {DIM}[1][1][2][2][3][3][4]{R} imports 0/6 {GRAY}(no dummy at 4P · 'tracks' for rewards){R}"
          f"   {BOLD}ACHIEVEMENTS{R} A·Silk 6 B·Orders 5 C·Prod 2+Comm 3 {GRAY}('goals'){R}")
    L.append(tr)
    L.append(f"{GRAY}{'─'*FLOOR_W}{R}")
    phases = ["Additional", "Placement", "Movement", "Area Action", "5-Power", "Building", "Recovery", "Additional", "End"]
    chain = f"{BOLD}TURN 1 · {RED}RED{R}{BOLD}'s turn{R}   "
    for i, p in enumerate(phases):
        chain += (f"{INV}{BOLD} {p.upper()} {R}" if i == 0 else f"{DIM}{p}{R}")
        if i < len(phases) - 1: chain += f"{GRAY} ▸ {R}"
    L.append(chain)
    L.append(f"{BOLD}YOU ARE DECIDING{R} · an additional action before Placement.")
    L.append(f"   {BOLD}[1]{R} End the additional-action phase and move on to {BOLD}Placement{R}")
    L.append(f"{BOLD}> {R}")
    return L


# ================================================================ ROUND 3 ====
# Mid-game 2P state (ILLUSTRATIVE — turn 9, Movement phase, Red to move).
# Same board as the owner's game; pawns/builds/goods invented to exercise the
# rendering. Site rewards are the REAL S-card data from 09-building-sites.yaml.

SITE_R = {  # site index -> (TL, TR, ML, MR, TH)  as compact tokens
 2:("★1","f3","—","f2",7), 3:("★1","★4","—","t2",7), 4:("★1","⌂2","—","t2",8),
 5:("★1","s2","—","c1",9), 6:("★1","c2","—","s2",9), 7:("★1","*2","—","*1",7),
 8:("★1","*2","—","*1",6), 9:("★1","★4","—","a2",8), 0:("★1","f3","—","f2",7),
 23:("★1","★4","—","★3",9),
}
TOKC = {"★":VP,"¥":YEN,"f":CYAN,"t":GREEN,"s":WHITE,"c":COPPER,"*":MAG,"a":WHITE,"⌂":WHITE,"—":GRAY}
def tok(t):
    return f"{TOKC.get(t[0],GRAY)}{t}{R}"

# per-area mid-game presence: aid -> (president_color|None, red_asst, blue_asst, built list)
MID = {
 "A00": (None,0,1,[("TL",BLUE)]), "A01": (BLUE,0,2,[]), "A02": (None,1,0,[]),
 "A03": (None,0,0,[]), "A04": (None,0,0,[]), "A05": (None,2,1,[]),
 "A06": (RED,2,1,[]), "A07": (None,0,2,[]), "A08": (None,0,0,[]), "A09": (None,0,0,[]),
}
REACHABLE = {"A05", "A02"}

def pawn_run(color, n):
    if n == 0: return ""
    dots = "●" * min(n, 4) + (f"+{n-4}" if n > 4 else "")
    return f"{color}{dots}{R}"

def tile_mid(aid, style="v3"):
    name = NAMES[aid]; tok_ = TOKENS[aid]; tokc = TOKCOL[tok_.split()[1]]
    pres, ra, ba, built = MID[aid]
    reach = aid in REACHABLE
    mark = f" {pres}▲{R}" if pres else ""
    rtag = f" {GREEN}◈{R}" if reach else ""
    head = f"┌ {GRAY}{aid}{R} {BOLD}{name}{R}{mark}{rtag} "
    l1 = pad(head, TILE_W - 1) + "┐"
    l1 = l1[:len(l1)]
    fill = TILE_W - 1 - vis(head)
    l1 = head + "─" * fill + "┐" if fill > 0 else pad(head, TILE_W - 1) + "┐"
    l2 = f"│ {pad(f'5-Power: {tokc}{tok_}{R}  {GRAY}Site {SITES[aid]}{R}', TILE_W-4)} │"
    pawns = "  ".join(x for x in [pawn_run(RED, ra), pawn_run(BLUE, ba)] if x)
    if not pawns: pawns = f"{DIM}no pawns{R}"
    l3 = f"│ {pad(pawns, TILE_W-4)} │"
    tl, tr, ml, mr, th = SITE_R[SITES[aid]]
    slots = []
    built_keys = {k for k, _ in built}
    for key, t in (("TL",tl),("TR",tr),("ML",ml),("MR",mr)):
        if key in built_keys:
            owner = dict(built)[key]
            slots.append(f"{owner}⌂{R}")
        else:
            slots.append(tok(t))
    l4 = f"│ {pad('sites ' + ' '.join(slots) + f'  {GRAY}│{R} T' + str(th), TILE_W-4)} │"
    l5 = "└" + "─" * (TILE_W - 2) + "┘"
    return _tile_guard([l1, l2, l3, l4, l5], aid)

def iconography_study():
    L = []
    L.append(f"{BOLD}TILE PRESENCE — three treatments of the same mid-game tile{R}   {GRAY}(Silk Mill: Red president + 2 Red assistants + 1 Blue assistant){R}")
    L.append("")
    v1 = ["┌ A06 SILK MILL ─────────────┐",
          f"│ {pad(f'President: {RED}Red{R}', TILE_W-4)} │",
          f"│ {pad(f'{RED}Red 2{R} · {BLUE}Blue 1{R} assistants', TILE_W-4)} │",
          f"│ {pad(f'Shophouses {DIM}····{R} Trade {DIM}·{R}', TILE_W-4)} │",
          "└────────────────────────────┘"]
    v2 = ["┌ A06 SILK MILL ─────────────┐",
          f"│ {pad(f'{RED}▲{R} {RED}●●{R}  {BLUE}●{R}', TILE_W-4)} │",
          f"│ {pad(f'{GRAY}▲ president · ● assistants{R}', TILE_W-4)} │",
          f"│ {pad(f'Shophouses {DIM}····{R} Trade {DIM}·{R}', TILE_W-4)} │",
          "└────────────────────────────┘"]
    v3 = tile_mid("A06")
    labels = [pad(f"{GRAY}V1 — today's letters/counts{R}", TILE_W + 2),
              pad(f"{GRAY}V2 — pawn glyph line{R}", TILE_W + 2),
              pad(f"{GRAY}V3 — marker on the title + slots{R}", TILE_W + 2)]
    L.append("".join(labels).rstrip())
    rows = max(len(v1), len(v2), len(v3))
    for v in (v1, v2, v3):
        while len(v) < rows: v.append(pad("", TILE_W))
    for i in range(rows):
        L.append(pad(v1[i], TILE_W + 2) + pad(v2[i], TILE_W + 2) + v3[i])
    L.append("")
    L.append(f"{GRAY}V3 frees a whole tile line for the site slots by moving the president marker (▲, in its owner's color) onto the title — scanning the board for ▲ answers")
    L.append(f"{GRAY}\"where are the presidents\" at a glance, and {R}{GREEN}◈{R}{GRAY} tints tiles the engine's own legal list marks as reachable during Movement (see the mid-game frame).{R}")
    return L

def dashboard_mid():
    L = []
    L.append(hdr("PLAYERS"))
    L.append(f"{RED}{BOLD}▶ RED — your turn{R}        {YEN}¥6{R}   {VP}11 VP{R}")
    L.append(f"  Goods  {COPPER}0 Copper{R} · {WHITE}2 Silk{R} · {GREEN}1 Tea{R} · {CYAN}0 Seafood{R} · {YELLOW}1 Import{R}")
    L.append(f"  Assistants  2 hand · {BOLD}5 on board{R} · 13 warehouse")
    L.append(f"  Shophouses  2 in hand · 5 in warehouse · 1 built")
    L.append(f"  Trade houses  0 in hand · 4 in warehouse · 0 built {GRAY}(next take ¥4){R}")
    L.append(f"  Tech  {BOLD}Telegram{R} · Mercenaries {BOLD}1 unused{R} · 0 used")
    L.append(f"  President {RED}▲{R} on SILK MILL · 1 order in hand")
    L.append("")
    L.append(f"{BLUE}{BOLD}  BLUE{R}                    {YEN}¥3{R}   {VP}8 VP{R}")
    L.append(f"  Goods  {COPPER}1 Copper{R} · {WHITE}0 Silk{R} · {GREEN}2 Tea{R} · {CYAN}1 Seafood{R} · {DIM}0 Imports{R}")
    L.append(f"  Assistants  4 hand · 4 on board · 12 warehouse")
    L.append(f"  Shophouses  1 in hand · 6 in warehouse · 1 built")
    L.append(f"  Trade houses  0 in hand · 4 in warehouse · 0 built {GRAY}(next take ¥4){R}")
    L.append(f"  Tech  {DIM}none yet{R} · Mercenaries 0 unused · 0 used")
    L.append(f"  President {BLUE}▲{R} on BANK · 1 order in hand {GRAY}(hidden){R}")
    L.append("")
    L.append(hdr("TRACKS  (ends at 4 of YOUR pawns · D = dummy, not counted)"))
    church = (f"  Church   faith "
              f"{DIM}[4]{R}{BLUE}[5●]{R}{DIM}[6]{R}{INV}[7 D]{R}{DIM}[8]{R}{DIM}[9]{R}{DIM}[10]{R}{DIM}[11]{R}"
              f" 1/4 · {VP}★3–14{R}+mv")
    L.append(church)
    customs = (f"  Customs  pay   "
               f"{DIM}[1]{R}{DIM}[1]{R}{DIM}[2]{R}{INV}[2 D]{R}{DIM}[3]{R}{DIM}[3]{R}{DIM}[4]{R} imports"
               f" 0/4 · {VP}★4–22{R}")
    L.append(customs)
    L.append(f"  {GRAY}Mercenary supply: 5 tiles left · ('tracks' renders every space){R}")
    L.append("")
    L.append(hdr("ACHIEVEMENTS  (one per category)"))
    L.append(f"  {BOLD}A{R} · Copper 5      own 5 copper                {VP}8 / 6 VP{R}   {DIM}unclaimed{R}")
    L.append(f"  {BOLD}B{R} · Money 10      own 10 yen                  {VP}9 / 7 VP{R}   {DIM}unclaimed{R}")
    L.append(f"  {BOLD}C{R} · Production 4  buildings in 4 production   {VP}10 / 8 VP{R}  {DIM}unclaimed{R}")
    L.append("")
    L.append(hdr("YOUR HAND  ('hand' expands · 'privacy' hides)"))
    for ln in order_card(2, note="your starting order"):
        L.append("  " + ln)
    return L

def main_frame_mid():
    L = []
    title = f"{BOLD}Y O K O H A M A{R}   {GRAY}MID-GAME STUDY (illustrative state, turn 9) · live tracking · autosaved{R}"
    L.append(pad("", max((FLOOR_W - vis(title)) // 2, 0)) + title)
    L.append("")
    rows = [["A00","A01","A02","A03"], ["A04","A05","A06"], ["A07","A08"], ["A09"]]
    board = []
    for i, row in enumerate(rows):
        offset = " " * (i * (TILE_W // 2))
        tiles = [tile_mid(a) for a in row]
        for li in range(5):
            board.append(offset + "".join(t[li] for t in tiles))
    dash = dashboard_mid()
    n = max(len(board), len(dash))
    board += [""] * (n - len(board)); dash += [""] * (n - len(dash))
    for b, d in zip(board, dash):
        L.append(pad(b, 124) + d)
    L.append(f"{GRAY}site slots: {R}{WHITE}⌂{R}{GRAY} built (owner color) · {R}{VP}★n{R}{GRAY} VP · {R}{CYAN}f{R}{GREEN}t{R}{WHITE}s{R}{COPPER}c{R}{GRAY}=goods · {R}{MAG}*{R}{GRAY}=any good · a=assistants · T=trading-house VP — 'sites' renders the full cards{R}")
    L.append("")
    L.append(f"{GRAY}{'─'*FLOOR_W}{R}")
    phases = ["Additional", "Placement", "Movement", "Area Action", "5-Power", "Building", "Recovery", "Additional", "End"]
    chain = f"{BOLD}TURN 9 · {RED}RED{R}{BOLD}'s turn{R}   "
    for i, p in enumerate(phases):
        chain += (f"{INV}{BOLD} {p.upper()} {R}" if i == 2 else f"{DIM}{p}{R}")
        if i < len(phases) - 1: chain += f"{GRAY} ▸ {R}"
    L.append(chain)
    L.append("")
    L.append(f"{BOLD}YOU ARE DECIDING{R} · where your President moves — destinations need one of your assistants to stop on ({GREEN}◈{R} on the board).")
    L.append(f"   {BOLD}[1]{R} Move to {BOLD}CUSTOMS{R} {GREEN}◈{R} — 1 step · free")
    L.append(f"   {BOLD}[2]{R} Move to {BOLD}PORT A{R} {GREEN}◈{R} — 1 step · free")
    L.append(f"   {BOLD}[3]{R} Return the President to your hand")
    L.append(f"   {DIM}(not offered: TEA PLANTATION — you have no assistant there to stop on){R}")
    L.append(f"{GRAY}(pick a number · 'back' undoes the last applied entry · ↑ recalls what you typed · 'view' any surface · '?' the card index){R}")
    L.append(f"{BOLD}> {R}")
    return L

def sites_view():
    W = 44
    def scard(idx, area, spaces, built, th, th_note):
        rows = [pad(f"┌ {BOLD}SITE {idx}{R} {GRAY}— on {area}{R} ", W - 1) + "┐"]
        names = {"TL":"top-left","TR":"top-right","ML":"mid-left","MR":"mid-right"}
        for key, desc in spaces:
            b = built.get(key)
            state = f"{b}⌂ built{R}" if b else f"{DIM}empty{R}"
            rows.append(pad(f"│ {names[key]:<10} {desc:<16} {state}", W - 1) + "│")
        rows.append(pad(f"│ trading house  {VP}★{th} VP{R}  {th_note}", W - 1) + "│")
        rows.append("└" + "─" * (W - 2) + "┘")
        return rows
    L = []
    L.append(f"{BOLD}BUILDING SITES — full view{R}   {GRAY}every area's site card: each space's printed reward, what is built, and by whom ('sites'){R}")
    L.append("")
    c1 = scard(2, "COPPER MINE", [("TL","1 VP"),("TR","3 Seafood"),("ML","— blank"),("MR","2 Seafood")],
               {"TL": BLUE}, 7, f"{DIM}open{R}")
    c2 = scard(8, "SILK MILL", [("TL","1 VP"),("TR","2 any goods"),("ML","— blank"),("MR","1 any good")],
               {}, 6, f"{DIM}open{R}")
    c3 = scard(9, "CHURCH", [("TL","1 VP"),("TR","4 VP"),("ML","— blank"),("MR","2 assistants")],
               {}, 8, f"{DIM}open{R}")
    for i in range(len(c1)):
        L.append(pad(c1[i], W + 2) + pad(c2[i], W + 2) + c3[i])
    L.append("")
    L.append(f"{GRAY}A shophouse built on a space takes that space's printed reward once; the trading house builds FREE from hand and scores its VP. Seven more cards page with n / p.{R}")
    L.append(f"{BOLD}> {R}{GRAY}Enter returns to the game{R}")
    return L

if __name__ == "__main__":
    emit("main-frame", main_frame())
    emit("order-index", order_index())
    emit("tracks-view", tracks_view())
    emit("frame-4p", frame_4p())
    emit("iconography", iconography_study())
    emit("main-frame-mid", main_frame_mid())
    emit("sites-view", sites_view())
