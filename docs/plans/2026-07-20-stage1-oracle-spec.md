# Stage 1 oracle spec — independent legal-move reference (`crates/yokohama-oracle`)

**Date:** 2026-07-20 · **Phase:** 2.9 Stage 1 (plan `docs/plans/2026-07-20-phase-2.9-differential-oracle-plan.md`)
**Author role:** independent reference author. This spec was written from the rules sources ONLY —
the rulebook PDF, `analysis-1/yokohama-kb/01-rules-reference.md`, the three verified card YAMLs,
`guidingpath.md`'s decision log, CLAUDE.md's Rules-knowledge section, the `types/` vocabulary, and
`game/rules.rs`'s accessor surface. **No engine logic file was opened** (no `legal_moves.rs`,
`apply_move.rs`, `movement.rs`, `scoring.rs`, `choices.rs`, `actions/`, `data.rs`, no engine-mapped
reference, no tests). The full attestation is §8.

**Citation key** used throughout:
- `[RB p.N]` — rulebook page N (`analysis-1/00-yokohama-rulebook.pdf`; final authority on rules).
- `[KB §…]` — `analysis-1/yokohama-kb/01-rules-reference.md` section (rulebook wins on conflict
  unless a decision-log row adjudicates).
- `[DL YYYY-MM-DD …]` — `guidingpath.md` decision-log row (normative for every ruling AND for every
  engine ENCODING convention: choice queue, Skip, canonical representations).
- `[CM]` — CLAUDE.md Rules-knowledge section (summary of adjudicated rulings; the DL row is primary).
- `[types::file]` — the shared type vocabulary in `crates/yokohama-core/src/types/`.

---

## 1. Contract & scope

### 1.1 Function contract (pinned by the orchestrator; restated)

```rust
pub struct RouteLabel {
    /// Where the President ends the Movement step.
    pub destination: AreaId,
    /// Every Yen the mover pays on the effective route, keyed by recipient:
    /// `Some(player)` = 1-Yen president fees credited to that player;
    /// `None` = bank (Canal crossing tolls). Absent key = 0.
    /// This mirrors the engine's L2 `RouteVector` keying
    /// (`BTreeMap<Option<PlayerId>, u32>` by fee recipient — types/ids.rs doc comment).
    pub fees: std::collections::BTreeMap<Option<PlayerId>, u32>,
}
impl RouteLabel {
    /// Total Yen the mover spends = player credits + bank tolls.
    pub fn mover_cost(&self) -> u32 { self.fees.values().sum() }
}

pub struct OracleMoves {
    /// Every legal move in the engine's exact `Move` encoding, EXCEPT the
    /// on-board president-movement family (`Move::MovePresident`).
    pub exact: Vec<Move>,
    /// The president-movement family, expressed as one label per distinct
    /// (destination, fee-vector). Non-empty only when `turn_phase == Movement`,
    /// the pending-choice queue is empty, and the President is on the board.
    pub movement: Vec<RouteLabel>,
}

pub fn oracle_moves(state: &GameState, rules: &Rules) -> OracleMoves;
```

Rationale for the label form: the engine canonicalizes president routes to **one representative
`MovePresident` per distinct (destination, mover-cost, recipient-credit-vector) over simple paths**
[DL 2026-07-19 "§8 veto pass … I3 SUPERSEDED by the L2 route-freedom ruling"], and legality is
judged on the **effective route** — retracing declarations are not real movements, so simple paths
are the complete table-legality surface and same-destination routes are distinct exactly when their
(cost, credit-vector) differ, "which the L2 label already encodes" [DL 2026-07-20 "Movement path
semantics RULED … effective route"]. Comparing `path` vectors would test an engine-side canonical
representative the rulebook has no opinion about; comparing labels tests exactly the rules content.
`PlacePresident` and `ReturnPresidentToHand` are NOT part of the label family — they are exact
moves (§3.3).

### 1.2 Input assumptions

- **States are engine-reachable and normalized.** In particular, singleton auto-resolution has
  already been applied to the pending-choice queue [DL 2026-07-19 "CHOICE ENCODING LANDED" close-out
  row: queue-first `get_legal_moves`, singleton auto-resolve with the pinned two-prong Decline
  invariant]: if `state.pending_choices` is non-empty, its head is a real multi-option choice. The
  oracle still derives each head's option set from the rules (§2) — it does not assume the count.
- **Static data is shared, not independent.** Card/site/token data arrives via the `Rules`
  accessors (`rules.orders`, `rules.techs`, `rules.building_sites`, `rules.five_power_tokens` —
  `game/rules.rs` public fields) and via `GameState.board_layout` /
  the port/lab/track structures. Independence in this phase is about **logic**, per the plan's
  honesty caveat; the coverage map must declare this sharedness.
- The oracle assumes the engine's state invariants hold (counts consistent, decks conserved). It
  is still written as a total function: any reachable `(phase, state)` combination gets a defined
  answer, including the dormant ones flagged in §1.4.

### 1.3 Output conventions

- `exact` and `movement` are **duplicate-free**; element order carries no meaning — the Stage-1
  runner compares them as sets. Where the engine's canonical intra-move ordering of a `Vec` field
  could not be pinned from allowed sources (listed in §7.2/`encoding_unpinned`), the oracle emits a
  documented normal form (sorted ascending) and the runner must normalize both sides the same way
  before set comparison.
- Brute force is the design: exhaustive enumeration over ≤18 areas, ≤6 slots, ≤5 power. No
  affordability shortcut is taken anywhere except the movement DFS's monotone cost cutoff, which is
  justified from the rules in §4.4.

### 1.4 Turn structure and what the oracle emits per phase

The turn is: **A. Additional Action phase (first half) → B. Main Action phase steps 1–6
(Placement, Movement, Area Action, 5-Power bonus (optional), Shophouse and Trading House
Construction (optional), Recovery) → C. Additional Action phase (second half)** [RB p.5 "Flow of a
turn"]. Steps 1–3 and 6 are compulsory, 4–5 optional; if Step 2's Option B is taken, steps 3–6 are
skipped [RB p.5–6]. The engine's `TurnPhase` [types::moves] maps these 1:1:
`AdditionalActionFirst, Placement, Movement, AreaAction, FivePowerBonus, Building, Recovery,
AdditionalActionSecond, TurnEnd`.

Mercenary use runs a nested sub-turn AreaAction→FivePowerBonus→Building→Recovery at
`mercenary_target_area` with `is_mercenary_action` set, then returns to `mercenary_return_phase`
[RB p.8 "you continue with the same steps of the main phase in order, i.e. 3…6";
types::state fields].

| State | `exact` contains | `movement` |
|---|---|---|
| `pending_choices` non-empty (any phase) | head choice's resolution moves ONLY (§2) | empty |
| game over (`is_game_over()`) | empty | empty |
| `AdditionalActionFirst` / `AdditionalActionSecond` (incl. mercenary-return windows — they ARE these phases [DL 2026-07-19 §5.3-4]) | §3.1 additional-action family | empty |
| `Placement` | §3.2 placement family | empty |
| `Movement`, President in hand | `PlacePresident` destinations + retreat family (§3.3) | empty |
| `Movement`, President on board | retreat family (§3.3) | route labels (§4) |
| `AreaAction` | §3.4 per-area family for the acting area | empty |
| `FivePowerBonus` | §3.5 | empty |
| `Building` | §3.6 | empty |
| `Recovery`, queue empty | empty (Recovery is automatic; the only decision is the queued `RecoveryLeave`, §2.6/§3.7 — a choice-free Recovery state is believed transient, §7.2) | empty |
| `TurnEnd` | empty (terminal rest state at game over; non-terminal `TurnEnd` believed transient, §7.2) | empty |

**Acting area** (used by AreaAction/FivePowerBonus/Building/Recovery): `mercenary_target_area` when
`is_mercenary_action`, else the President's `OnBoard` area [RB p.6 "the player carries out the
action where their President pawn is located… if you utilized Foreign Mercenaries… you may carry
out an action in the specified area"].

**Power** of the acting player in the acting area = (1 if own President is in that area — never the
case in a mercenary sub-turn, since the target may not contain your President [RB p.8]) + own
assistants there + 1 if own Shophouse there + 1 if own Trading House there + 1 if the Station token
is there (Station powers ALL players [RB p.15]), capped at 5 [RB p.6–7 power table; "If the Power
value tallies 6 or higher, it is still considered as a maximum value of 5"]. Shophouses/Trading
Houses never count as assistants for movement [RB p.6].

### 1.5 Out of scope for Stage 1

Applying moves (the engine drives self-play), scoring (`final_result` — Stage 2), reveal/record
machinery, setup/`reconcile_setup`, and the CLI. Trading-House passive income [RB p.7] and all
reward *effects* are apply-side and appear here only where they gate legality (never — income
happens during resolution and cannot fund the move that triggers it).

---

## 2. Queue-first choice semantics

When `state.pending_choices` is non-empty, the legal set is **exactly the head choice's resolution
moves** — nothing else, in any phase; even `EndAdditionalPhase` is rejected while choices are
queued [DL 2026-07-19 "CHOICE ENCODING LANDED" close-out row (queue-first `get_legal_moves`); DL
2026-07-19 "FINAL WHOLE-BRANCH REVIEW" I1 fix (`EndAdditionalPhase` rejected at the apply layer
while choices are queued)]. Choices belong to the current player; the queue is FIFO and empty at
every hand-off [types::state doc]. The oracle inspects `pending_choices[0]` only.

Per `PendingChoice` variant [types::choices]:

### 2.1 `AnyGood` → `ChooseAnyGood { good }`
Option space: exactly the four `TradeGood`s (Copper, Silk, Tea, Seafood). No decline — the gain is
unconditional ("Gain 1 Trade good of your choice", Any-Trade-good icon [RB p.10]; no-Decline pinned:
"AnyGood 4-fixed" is structurally never a singleton [DL 2026-07-19 close-out row, two-prong
invariant]). Always 4 moves, regardless of state.

### 2.2 `WarehouseTake` → `ChooseWarehouseTake { item }` ∪ `DeclineWarehouseTake`
Warehouse icon: "Take a token from the Warehouse tile to your hand. The cost is the same as the
Employment Agency action" [RB p.10]. Option space, one move per `WarehouseItem` that is both
**available** and **affordable**:
- `Assistant`: `assistants.in_warehouse ≥ 1`; cost 0 [RB p.13 "Assistant pawns: Free"].
- `Shophouse`: `shophouses.in_warehouse ≥ 1`; cost 2 Yen [RB p.13].
- `TradingHouse`: `trading_houses.in_warehouse ≥ 1`; cost = the printed 4/5/6/7 progression indexed
  by tokens already out of the warehouse (`in_hand + built`) [RB p.13 "4 Yen for the 1st, 5 for the
  2nd, 6 for the 3rd, 7 for the 4th"; staging warehouse→hand at the Employment Agency ruled DL
  2026-07-17 "Trading House staging corrected"; `TradingHouseState::take_cost/next_cost`
  is the single source, types::player].
- `DeclineWarehouseTake` is ALWAYS offered — a player who can pay cannot be compelled to spend
  [DL 2026-07-19 "§5.3-3: Warehouse-icon takes are DECLINABLE"].

### 2.3 `AssistantMove` → `ChooseAssistantMove { from, to }` ∪ `DeclineAssistantMove`
Movement icon: "Move an Assistant pawn from your hand or from an Area to another Area. You may
treat each movement separately. You do not need to pay other President pawns" [RB p.10; DL
2026-07-17 icon-semantics row]. Option space:
- `from ∈ { MoveSource::Hand }` iff `assistants.in_hand ≥ 1`, plus `MoveSource::Area(a)` for every
  area `a` with `assistants.in_area(a) ≥ 1`.
- `to`: every board area with `area_type.allows_placement()` (i.e. every non-Canal area — no pawns
  may ever be placed in the Canal [RB p.14; types::board]); for `from = Area(a)`, `to ≠ a`
  ("to **another** Area" [RB p.10]). Opponent presidents at `to` are irrelevant (no fees — this is
  not President movement [RB p.10; types::choices doc]).
- `DeclineAssistantMove` is ALWAYS offered — declining a movement icon is a legal play
  [DL 2026-07-17 "v1 reward-choice simplifications" (declining is legal); Decline co-listing pinned
  by the two-prong invariant, DL 2026-07-19 close-out row].

### 2.4 `ChurchSpace { total_faith }` → `ChooseChurchSpace { space }`
Queued only after a Church action has validated ≥2 eligible spaces at `total_faith`; exactly one
eligible space places inline and never reaches the queue [types::choices doc; DL 2026-07-19
close-out row]. Option space: every space value `s` in 4..=11 with `church_track` slot empty,
`s ≤ total_faith` ("any empty space with value ≤ total faith, excess faith lost" [RB p.13; DL
2026-07-17 Church placement row]), excluding the 2-player printed-dummy block on faith 7
[DL 2026-07-17 "2P games: church faith-7 space … PRE-BLOCKED"]. No decline.

### 2.5 `CustomsSpace { cost }` → `ChooseCustomsSpace { space }`
Queued only when BOTH spaces of the printed cost pair are empty and unblocked; a lone eligible
space places inline [types::choices doc]. Option space: the empty, unblocked spaces among
`CustomsTrack::spaces_for_cost(cost)` (cost 1 → {1,2}; 2 → {3,4}; 3 → {5,6}; 4 → {7})
[DL 2026-07-17 Customs re-model row; types::tracks], excluding the 2-player dummy block on space 4
[DL 2026-07-17]. No decline. (Cost 4 has a single space, so `CustomsSpace{4}` can never queue.)

### 2.6 `RecoveryLeave` → `ChooseRecoveryLeave { leave_one: false }` ∪ `{ leave_one: true }`
Exactly two fixed options, queued only when the current player owns Telephone AND has ≥1 own
assistant in the acting area [RB p.16 Telephone "you may leave 1 Assistant pawn in the Area";
types::choices doc]. `leave_one: true` with a LONE assistant recovers ZERO — the kept pawn may be
the only one [DL 2026-07-19 "§5.3-5: Telephone may keep a LONE assistant"]. Structurally never a
singleton [DL 2026-07-19 close-out row].

### 2.7 `StationPlace` → `ChooseStationPlace { area }`
Queued when the Station tech is acquired; the token must land — NO decline [types::choices doc;
RB p.15 "When implementing this Technology, place the Station token in an Area of your choice"].
Option space: every non-Canal area (the Canal takes no tokens [RB p.14]); occupancy by pawns,
buildings, or presidents of any player does not restrict it ("an Area of your choice") [CM: "every
non-Canal area is eligible"]. Documented exception to the singleton invariant: a lone eligible area
places inline with real effect — design-sanctioned, unreachable on any generated board [CM].

---

## 3. Per-phase enumeration rules

Throughout: "opponent president in area X" means a president pawn of any OTHER player located
`OnBoard(X)`. Fees to a president's owner are 1 Yen each; a player's own president never generates
a fee against themself (paying yourself is a net no-op; the rulebook's fee text targets "the player
owning the President pawn" [RB p.5] and its examples price only other players' presidents).

### 3.1 Additional-action phases (`AdditionalActionFirst` / `AdditionalActionSecond`)

Any number of the following, in any order, all optional [RB p.8 "you may do the following actions
as many times as you like and in any order"] — so the enumeration per state is:

1. **`EndAdditionalPhase`** — always legal (proceeding is always the player's right; every listed
   action is "may"). [RB p.8; encoding: types::moves "End additional action phase and proceed".]
2. **`UseMercenary { country, target_area }`** — for each `country` with
   `player.mercenaries.available[country] ≥ 1` (face-up only — used mercenaries are flipped
   face-down [RB p.8]) × each area with ≥1 own assistant AND not containing the player's own
   President; opponent presidents allowed and NEVER paid [RB p.8 "It is possible to choose an Area
   that contains another player's President pawn … you do not need to pay them any money"; CM
   mercenaries-bypass line]. Gated by `!additional_action_state.mercenary_used` — once per TURN,
   across both windows [RB p.8 "You may only use Foreign Mercenaries once on your turn"].
3. **`FulfillOrder { order_id }`** — for each order card in `order_cards_in_hand` whose
   `requirements` (copper/silk/tea/seafood, via `rules.orders`) are all ≤ the player's holdings
   [RB p.8 "Place an Order card from your hand face-up in front of you. Pay the corresponding
   Trade Goods"]. Repeatable within the phase; rewards/mercenary-matching are apply-side.
4. **`FulfillAchievement { achievement }`** — for each of the three in-play `achievements` where
   (a) the player has not already claimed it (each player may fulfill each Achievement only once
   [RB p.8 Note]), (b) `assistants.in_hand ≥ 1` (an assistant is placed from hand onto the card
   [RB p.8]), and (c) the requirement is met at claim time — nothing is paid, ownership is
   demonstrated [RB p.14 "You do not need to pay or throw away these items"]:
   - Category A: own ≥5 Copper / ≥6 Raw Silk / ≥7 Tea / ≥7 Seafood [RB p.14].
   - Category B: own ≥10 Yen; own ≥5 total 5-Power + Foreign Mercenary tokens (see ambiguity
     §7.1-A2 for face-down counting); have ≥5 completed Orders; own ≥4 Technology cards [RB p.14–15].
   - Category C: Shophouses and Trading Houses BOTH count, multiple buildings in one area count
     once [RB p.15 "If you have built more than 1 Shophouse and/or Trading House in the same Area,
     it counts for only 1 icon"; DL 2026-07-18 owner-confirmed both-types row]: buildings in ≥4
     Production areas / ≥4 Commercial areas / ≥3 Production + ≥2 Commercial / ≥2 Production + ≥3
     Commercial (`AreaType::is_production`/`is_commercial`; Canal is neither).
5. **`UseUniversity`** — iff the player owns University, `!university_used`, `money ≥ 1`, and
   `assistants.in_warehouse ≥ 1` [RB p.16 "Once per turn, you may gain an Assistant pawn from the
   Warehouse tile by paying 1 Yen to the supply"]. Offered in the additional-action windows ONLY —
   a mid-turn use is state-equivalent to the nearest window [DL 2026-07-19 "§5.3-4"].
6. **`UseSpinningMill`** — iff the player owns Spinning Mill (card name "Textile Mill"),
   `!spinning_mill_used`, and `silk ≥ 1` [RB p.15 "Once per turn, you may discard 1 Raw Silk to
   gain 2 Yen"]. Window-only, same ruling [DL 2026-07-19 "§5.3-4"].

(System of Stock is a passive turn-start refill, no move [RB p.16; DL 2026-07-17 Task-20c row].)

### 3.2 Placement (`Placement`)

Choose ONE of two options [RB p.5]. Placement is from HAND only [RB p.5 "You may not place any
Assistant pawns from the Warehouse tile or the Management boards"]; never into the Canal [RB p.5].
Fee rule: **1 Yen per assistant placed, to EACH opponent-president owner in the target area**
[RB p.5 "you have to pay 1 Yen for each Assistant pawn you place to the player owning the President
pawn. If there is more than one President pawn in the area, you have to pay 1 Yen to each"; the
per-assistant reading is the adjudicated C1 fix, DL 2026-07-17 final-review C1]. Insufficient money
⇒ the placement is illegal [RB p.5 "If you have insufficient money, you may not place"].

1. **Option A — `PlaceThreeAreas { placements }`**: every subset of distinct placeable
   (non-Canal) areas of size 0..=N, where N = min(hand assistants, 3, or 4 with Newspaper
   [RB p.16 Newspaper "you may place up to 4 Assistant pawns in different Areas instead of 3"]).
   Placing 0, 1, 2, or 3 (4) is legal — the empty placement `placements: vec![]` is always legal
   and free [RB p.5 "you may place 1, 2 or 3 Assistant pawns"; zero ruled legal DL 2026-07-18
   final-review I5, expanded owner wording DL 2026-07-19 §8 veto row; types::moves doc "0-3 areas,
   or 0-4 with Newspaper"]. Affordability: Σ over chosen areas of (#opponent presidents there)
   ≤ `money`. Oracle normal form: `placements` sorted ascending by `AreaId` (§7.2).
2. **Option B — `PlaceTwoSameArea { area, adjacent_bonus }`**: for every placeable area,
   requires 2 assistants in hand; fee = 2 × (#opponent presidents at `area`) — per assistant, not
   per area [DL 2026-07-17 C1: "Option B's 2-into-one-area costs 2, not 1"]. With Telegram, ALSO
   enumerate `adjacent_bonus: Some(adj)` for each area `adj` physically adjacent to `area`
   (`board_layout.adjacent_areas`), EXCLUDING canal-crossing pairs — crossings are movement-transit
   edges only, placement adjacency is physical tile contiguity [DL 2026-07-18 "Canal geometry
   RE-CONFIRMED … Telegram's adjacent-bonus placement excludes crossing pairs"], and excluding the
   Canal itself; requires a 3rd assistant in hand; adds 1 × (#opponent presidents at `adj`) to the
   fee ("Pay the usual cost if another President pawn occupies this adjacent Area" [RB p.16
   Telegram]). The bonus is optional per move — the `None` form is enumerated regardless of
   Telegram ownership ("may additional place" [RB p.16]).
   Option B has no 0/1-assistant form — the zero-placement ruling attaches to Option A's empty
   vector only [DL 2026-07-18 I5, quoting `PlaceThreeAreas { placements: vec![] }`].

### 3.3 Movement (`Movement`) — exact-move part

Choose ONE of two options; the step is required and Option B is always performable, so Movement
never dead-ends [RB p.6; DL 2026-07-17 "Movement Option B is legal with the President already in
hand"].

1. **`PlacePresident { destination }`** (President in hand only): every **Possible Action Area** —
   ≥1 own assistant AND no other player's President [RB p.6 definition]. With **Postal System**,
   additionally every area with ≥1 own assistant that contains opponent president(s); fee = 1 Yen
   per opponent president at the destination, waived entirely by also owning **Stagecoach**
   [RB p.15 Postal System "You may end your movement in an Area occupied by another President pawn
   … You must still pay these players (unless you also own the Stagecoach)"; from-hand placement
   explicitly included, DL 2026-07-17 final-review I2 row]. Affordability-gated. The own-assistant
   requirement is never lifted — Postal System modifies only the president clause.
   From-hand Tram does NOT exist (board-only, owner-ruled [DL 2026-07-19 "from-hand Tram is
   BOARD-ONLY"]); from-hand Station teleport is unruled — the oracle takes the conservative
   board-only reading and the question is logged (§7.1-A1).
2. **`ReturnPresidentToHand { recover }`** — the any-number retreat family: President returns to
   hand (legal from ON BOARD, and also with the President ALREADY IN HAND — "stay in hand, recover
   any assistants" [DL 2026-07-17 Option-B-from-hand ruling]); `recover` is a per-area count map —
   every map where each key `a` has `1 ≤ recover[a] ≤ assistants.in_area(a)` (areas with 0
   recovered are omitted from the map), INCLUDING the empty map [RB p.6 "you may also return any
   Assistant pawns from any Area to your hand"; any-number-per-area encoding ruled DL 2026-07-19 §8
   veto row I5 expanded wording; types::moves doc "per-area counts; empty = recover none"].
   Management-board assistants (church/customs/achievements) are NOT recoverable [RB p.6 "You may
   not return Assistant pawns placed on Management Boards"] — structurally excluded because they are
   not in `on_board`. No fees of any kind. Family size = Π over own-assistant areas of
   (count + 1); the oracle materializes it (brute force); if a comparison-perf problem arises the
   runner may verify this family analytically as the exact product set (it is one — no per-map
   condition exists beyond the bounds).

The on-board `MovePresident` family is emitted as labels — §4.

### 3.4 Area Action (`AreaAction`)

The acting area's action is carried out at the computed power (§1.4). Step 3 is MANDATORY in
sequence; the six choice areas are worded permissively, so `AreaAction::Skip` — carry out the
action declining all optional components — is always legal at Port A/B, Laboratory A/B, Employment
Agency, Chinatown, Church, Customs, and ILLEGAL at production areas and the Bank, whose imperative
"Gain …" move is their only legal action [DL 2026-07-17 "Area-action optionality" row; RB p.7
steps 4/5 marked optional and step 3 not; RB p.11–12 per-area zero-effect wordings;
`AreaType::allows_area_action_skip`, types::board]. Empty substantive encodings (Port `[]`,
Chinatown `[]`, Employment 0/0/0, Customs 0) are validator-ILLEGAL — Skip is the sole canonical
decline; `Church` all-false stays substantive (faith = power) [same DL row]. Skip grants no
area-attached bonuses — at 5 Power on a Port, Skip forfeits the +3 VP (take-gated, box-verified
[DL 2026-07-18 BOX CHECK (b)]).

Per area type (all moves are `ExecuteAreaAction { action }`):

- **Production areas** — exactly one legal move: `ProduceCopper` / `ProduceSilk` / `ProduceTea` /
  `ProduceSeafood` by area type. Yields (apply-side, listed for fixture use): Copper 0/0/1/2/3,
  Silk 0/1/2/3/4, Tea 1/2/3/4/5, Seafood 1/2/3/4/5 at power 1–5 [RB p.10–11]. No Skip.
- **Bank** — exactly one legal move: `Bank` (gain Yen = power [RB p.11]). No Skip.
- **Port A/B** (the port board matching the acting area) — `Skip` plus:
  - Eligible slots by power: power 1 → none ("may not take" [RB p.11]); 2 → Low (slots 0–1);
    3 → Low/Mid (0–3); 4–5 → all six [RB p.11; six-slot layout LOW,LOW,MID,MID,HIGH,HIGH,
    DL 2026-07-17 six-slot row; `PortBoard::zone_of`, types::cards]. Power 5 also grants +3 VP with
    the take [RB p.11; take-gated, DL 2026-07-18 BOX CHECK (b)].
  - **Take 1** (free): `Port { cards_taken: [c], second_card_payment: None }` for each card `c` in
    an eligible occupied slot — iff `order_cards_in_hand.len() + 1 ≤ 3` (hand limit 3; orders can
    never be discarded, so a take that would exceed is illegal [RB p.5 General Rules, p.11 Note]).
  - **Take 2** (pay 2 Yen OR 1 Imported Good [RB p.11]): for each unordered pair of distinct
    eligible cards, `second_card_payment: Some(PortPayment::TwoYen)` iff `money ≥ 2`, and
    separately `Some(PortPayment::OneImportedGood)` iff `imported_goods ≥ 1` — both moves when both
    affordable; iff `hand + 2 ≤ 3`. Oracle normal form for `cards_taken`: slot order ascending
    (§7.2). Replenishment (slide-left, refill rightmost [RB p.12; DL 2026-07-17 port-replenishment
    row]) is apply-side.
- **Laboratory A/B** — `Skip` plus, for each occupied slot `s` holding tech card `c`:
  - **One-per-title gate**: skip if the player already owns a tech of the same name — effect is
    1:1 with title in the standard data [RB p.5 General Rules "players may only own 1 card of the
    same name"; DL 2026-07-17 final-review C2; KB §Laboratory ownership limit]. (Different players
    may each own one of a 2-copy title [KB §Laboratory].)
  - Required Industry = printed `industry_cost` + position modifier 0,0,1,1,2,2 for slots 1–6
    [RB p.12; DL 2026-07-17 six-slot row; `LabBoard::position_modifier`, types::cards].
  - Industry sources: power (max 5 from power), 2 Yen = 1 Industry, 1 Imported Good = 1 Industry
    [RB p.12]. Let `d = required − min(power,5)` (0 if negative; excess Industry is lost
    [RB p.12]). Enumerate exact covers: for each `i` in 0..=min(d, imported_goods) with
    `extra_imported = i`, `extra_yen = 2·(d − i)` — legal iff the total money/imported outlay
    (incl. the 5th-tech surcharge below) is affordable. Overpaying combinations are not enumerated
    (§7.2 — enumeration-convention unpinned; over-cover adds nothing at the table).
  - **5th-tech penalty**: if the player already owns ≥4 techs, each acquisition costs an ADDITIONAL
    2 Yen or 1 Imported Good [RB p.12 Note]; enumerate `fifth_tech_payment: Some(true)` (2 Yen)
    and `Some(false)` (1 Imported) as separately affordable variants; `None` otherwise.
  - Affordability: `money ≥ extra_yen + (2 if fifth = Some(true))`;
    `imported_goods ≥ extra_imported + (1 if fifth = Some(false))`.
  - Station's queued placement, Exposition's instant VP, Patent/Letterpress riders: apply-side.
- **Employment Agency** — `Skip` plus every `EmploymentAgency { assistants: a, shophouses: s,
  trading_houses: t }` with:
  - `1 ≤ a+s+t ≤ power − 1` (power→tokens 0/1/2/3/4 [RB p.12]); 0/0/0 is validator-ILLEGAL
    [DL 2026-07-17 optionality row].
  - Warehouse availability: `a ≤ assistants.in_warehouse`, `s ≤ shophouses.in_warehouse`,
    `t ≤ trading_houses.in_warehouse`.
  - Cost = `2·s` + Σ_{j=0..t−1} `TradingHouseState::take_cost(already_taken + j)` ≤ `money`
    (assistants free; shophouses 2 Yen; trading houses 4/5/6/7 by tokens already out of the
    warehouse [RB p.13; DL 2026-07-17 staging row]). Tokens go to HAND; building later is free
    (Building step, §3.6).
- **Chinatown** (absent in 2P) — `Skip` plus exchange sequences
  `Chinatown { exchanges }` with `1 ≤ len ≤ power` [RB p.13 "The number of times you may exchange
  depends on the Action Power"; power cap CM]. The nine `ChinatownExchange` rates [RB p.13;
  types::moves]: Copper→2¥, Silk→2¥, Tea→1¥, Seafood→1¥, 2¥→Copper, 2¥→Silk, 1¥→Tea, 1¥→Seafood,
  4¥→Imported. 2¥→Copper yields 1 + (owned Mining Technology copies) Copper [08-tech-cards.yaml
  Mining; DL 2026-07-18 every-copper-receipt row; KB 01:596] (added at round-2 triage: the
  original spec missed this cross-cutting rider; class-2 oracle bug, misreading recorded in the
  ruling list). Imported Goods can never be sold [RB p.13 Note]. The oracle validates a sequence
  left-to-right with running balances (proceeds of an earlier exchange may fund a later one — the
  sequential reading; see ambiguity §7.1-A3) and enumerates ALL feasible ordered sequences;
  sequence-vs-multiset canonicalization is an unpinned encoding (§7.2) the runner must normalize.
  `[]` is validator-ILLEGAL [DL 2026-07-17 optionality row].
- **Church** — `Skip` plus every payment combo `Church { copper, silk, tea, seafood, yen,
  imported_good }` (booleans; max 1 of each kind per turn [RB p.13 Note]) such that:
  - each `true` flag is backed by ≥1 of that resource (yen ⇒ `money ≥ 1`);
  - `total_faith = min(power,5) + #true` — faith from power is LINEAR 1..5; the rulebook's
    "2 Power: 1 Faith" is a TYPO [RB p.13 as corrected by DL 2026-07-17 "RESOLVED by Jeremy …
    LINEAR 1,2,3,4,5"; KB §Church correction];
  - an assistant is available from hand OR from the acting area
    (`in_hand ≥ 1 || in_area(acting) ≥ 1`) [RB p.13 "place 1 Assistant pawn from the Church Area or
    your hand"; DL 2026-07-18 final-review I3];
  - ≥1 eligible space exists: empty, value ≤ `total_faith`, value ≥ 4, not the 2P faith-7 dummy
    [RB p.13; DL 2026-07-17 Church placement + 2P dummy rows]. Combos failing any of these are
    ILLEGAL (validation error, zero mutation — goods are never paid for a placement that doesn't
    happen), not no-ops [DL 2026-07-17 "Church executor v1 semantic"]. All-false is substantive
    (faith = power) and legal when power ≥ 4 with an eligible space [DL 2026-07-17 optionality
    row]. Space choice: ≥2 eligible ⇒ `ChurchSpace` queues; exactly 1 ⇒ inline (§2.4).
- **Customs** — `Skip` plus `Customs { imported_goods_paid: n }` for each `n` with:
  - `1 ≤ n ≤ min(power − 1, 4)` (power→max-payable 0/1/2/3/4 [RB p.14]); `n ≤ imported_goods`;
  - `n` exactly matches a printed space cost with an empty, unblocked space: costs are
    [1,1,2,2,3,3,4] across spaces 1–7; both spaces of a pair filled ⇒ that payment unplaceable;
    2P space 4 dummy-blocked [RB p.14 + payment-must-match ruling, DL 2026-07-17 Customs re-model
    row];
  - assistant from hand OR the acting area [RB p.14 "from the Customs Area or your hand"; DL
    2026-07-18 I3].
  - `n = 0` is validator-ILLEGAL (Skip canonical) [DL 2026-07-17 Customs executor + optionality
    rows]. Space choice: both of the pair empty ⇒ `CustomsSpace` queues; else inline (§2.5).

### 3.5 5-Power bonus (`FivePowerBonus`)

Optional step, requires a 5-Power action in step 3 [RB p.7]. Enumeration:
- `Take5PowerToken` iff `current_action_power == Some(5)` AND
  `board_layout.five_power_tokens[acting_area] == Some(Some(_))` (token still present — it may have
  been claimed in an earlier turn [RB p.7 "If the 5-Power token in the corresponding Area is still
  present"]).
- `Skip5PowerBonus` — always legal in this phase (the step is optional [RB p.7]).
Whether the engine even presents this phase when ineligible (power < 5 or token gone) is an
unpinned reachability question (§7.2); the oracle's answer there is `{Skip5PowerBonus}`, which is
correct under the rules either way. Token effects (six faces: imported/copper/silk/tea/seafood/
money — no Movement or VP faces exist [DL 2026-07-18 BOX CHECK (a)]) resolve apply-side.

### 3.6 Building (`Building`)

Optional step, requires a 4+ Power action in step 3; ONE building, either type [RB p.7; DL
2026-07-18 C3 addendum "ONE Building action at 4+ Power … a Shophouse OR a Trading House"].
Enumeration (site = `board_layout.building_sites[acting_area]`; the Canal has no site
[types::board]):
- `BuildShophouse { slot }` for EACH empty slot (0–3) — the player chooses the space and gains
  that space's printed reward, so every empty slot is a distinct legal move [RB p.7 "Place a
  Shophouse token from your hand onto an empty Shophouse space … Gain the benefits stated on the
  space that you built your Shophouse on"; spaces carry different printed rewards, 09-building-
  sites.yaml] — iff `current_action_power ≥ 4`, `shophouses.in_hand ≥ 1`, and the player does NOT
  already have a Shophouse on this site [RB p.7 "only if they do not have a Shophouse already on
  the Building Site card"].
- `BuildTradingHouse` iff `current_action_power ≥ 4` (the 4+ gate applies to BOTH building types —
  there is no separate Power-5 tier [DL 2026-07-17 final-review C3; DL 2026-07-18 C3 addendum]),
  `trading_houses.in_hand ≥ 1` (tokens reach hand via the Employment Agency take; the build itself
  is FREE [RB p.7 "Place a Trading House token from your hand"; DL 2026-07-17 staging row]), and
  the site's Trading House space is empty [RB p.7].
- `SkipBuilding` — always legal in this phase (optional step [RB p.7]). Presentation at power < 4
  is the same unpinned reachability question as §3.5 (§7.2); the oracle answers `{SkipBuilding}`.
Gaslight/Electrical Light VP riders are apply-side [RB p.15–16].

### 3.7 Recovery (`Recovery`)

Automatic — ALL own assistants in the acting area return to hand, no move exists [RB p.7 "return
them all to your hand"; types::moves "Recovery is automatic"]. The only decision point is the
queued `RecoveryLeave` (§2.6), i.e. covered by queue-first §2. A `Recovery` state with an empty
queue yields the empty set (believed transient, §7.2).

### 3.8 Turn end (`TurnEnd`)

No moves. At game over this is the terminal rest state (the empty legal list is the engine's
post-game-over safety [DL 2026-07-18 Session-boundary row]); a non-terminal `TurnEnd` is believed
transient (§7.2).

### 3.9 Mercenary sub-turns

`UseMercenary` (§3.1) enters AreaAction→FivePowerBonus→Building→Recovery at the target area with
`is_mercenary_action = true` [RB p.8; types::state]. Everything in §3.4–3.7 applies verbatim with
acting area = `mercenary_target_area`; power counts NO president (the target may not contain the
player's own President [RB p.8], and opponent presidents contribute nothing to the actor's power
[RB p.6 power table is per-owner]); opponent presidents in the target cost nothing [RB p.8]. The
Church/Customs from-area assistant source matters especially here — the target holds ≥1 own
assistant by definition, so hand-0 placements are legal [DL 2026-07-18 I3 row]. After Recovery the
turn returns to `mercenary_return_phase` (apply-side).

---

## 4. Movement labels (`MovePresident` as `RouteLabel`)

Emitted only when: queue empty, `turn_phase == Movement`, President `OnBoard(start)`.

### 4.1 Walking routes

Enumerate every **simple path** `start → … → dest` over `board_layout.adjacency` such that:

1. **Every stepped-into area (intermediates AND destination) holds ≥1 of the mover's own
   assistants** [RB p.6 Movement rules "The President pawn may move 1 step from its current Area to
   an adjacent Area. At least one of that player's Assistant pawns must be in the destination
   area" — applied per step; the p.6 example rejects the Laboratory-A route for exactly this
   reason]. Shophouses/Trading Houses do not count [RB p.6].
2. **Destination**: no opponent president (Possible Action Area [RB p.6]) — OR the mover owns
   Postal System, which permits stopping there [RB p.15]. Destination ≠ `start` [RB p.6 "The
   President pawn may not return to the Area that it started in"] — the ONLY prohibited revisit;
   the effective route itself never self-intersects [DL 2026-07-20 effective-route row].
3. **Intermediate areas containing opponent president(s)**: passing is allowed; the mover "must
   immediately pay each other President pawn in the Area 1 Yen and continue" [RB p.6] — may not
   stop there (without Postal System). Fee: 1 Yen per opponent president in the area, credited to
   each owner, WAIVED by Stagecoach [RB p.15 Stagecoach "you do not need to pay other President
   pawns occupying Areas that you move over"].
4. **Destination fees under Postal System**: 1 Yen per opponent president at the destination,
   credited to each owner, waived by also owning Stagecoach [RB p.15 Postal System; DL 2026-07-17
   I2 row: empty-path teleports exempt].
5. **Canal crossings**: an edge `(a,b)` with `board_layout.is_canal_crossing(a,b)` costs 1 Yen to
   the BANK per crossing [RB p.14 "you may pay 1 Yen to the supply to move across the Canal"; DL
   2026-07-18 BOX CHECK (c): colinear through-line pairs only, toll to the bank]. The Canal is not
   a node — crossings are transit-only adjacency edges between flank tiles; bent transits are
   structurally impossible [DL 2026-07-18/19 canal-geometry rows]. Count the toll on EVERY traversed
   crossing edge, including the first edge `start → path[0]` [DL 2026-07-18 I4 row noted the
   start-edge as a required fix].
6. **Affordability**: total fees (player credits + bank tolls) ≤ `money` (fees are mandatory
   payments; the placement analog is explicit that insufficient money forbids the move [RB p.5–6]).

Each accepted path yields the label `{ destination, fees }` where `fees` accumulates
`Some(owner) → +1` per president fee and `None → +1` per canal toll. Deduplicate labels by
structural equality — this IS the L2 identity (destination, mover-cost, credit-vector) [DL
2026-07-19 §8 veto row], since mover-cost is determined by the fee map.

### 4.2 Teleports

- **Tram** (owner only): a zero-fee label for every non-Canal area that contains no other player's
  President, other than the current area; NO assistant needed at the destination; Tram is the
  entire movement [RB p.16 Tram "move your President pawn to any Area as long as that Area does not
  contain another player's President pawn. If you move your President by Tram this way, you may not
  move it again"; destination scope per DL 2026-07-17 I1 row: "every president-free, non-Canal area
  other than the current one"]. Tram does NOT bypass opponent-president blocking (destination must
  be president-free) [DL 2026-07-17 I1].
- **Station** (owner of the Station tech only, `station_location == Some(st)`, `st ≠` current
  area): a zero-fee label to `st` — free, regardless of other President pawns there [RB p.15
  Station "You may move your President pawn to this area … for free, and you do not need to pay
  even if there are other President pawns in the Area. However, you may only move to this Area if
  you are doing this Area action"; DL 2026-07-17 I2 row calls both Station and Tram empty-path
  free teleports]. Whether the Station area must hold an own assistant is a genuine open reading —
  §7.1-A1; the oracle AS SPECIFIED emitted the label WITHOUT the assistant requirement (card-text
  reading, Tram analog) so that any engine disagreement would surface as a divergence for triage
  rather than being silently harmonized. **[SUPERSEDED at round-1 triage, 2026-07-20 — see §9-D:
  the probe worked. The divergence surfaced organically (3/60 sweep games), was adversarially
  verified, and was triaged class-4 genuine ambiguity; pending the owner's ruling the CODE
  temporarily mirrors the engine's Reading 2 behind the `PENDING-RULING §7.1-A1` marker in
  `movement.rs`, so this sentence describes the probe design, not current behavior. The ruling
  list carries the full entry.]**

Teleport labels merge with walking labels under the same dedup (a free walking route and a teleport
to the same destination produce one identical `{dest, {}}` label).

### 4.3 What labels deliberately do not encode

The chosen path itself. Route choice is unencoded even engine-side (a deliberately more expensive
route to the same (dest, fee-vector) is the same label; the residual same-label route-choice limit
is accepted [DL 2026-07-19 final-review I3 residual note, superseded into L2 by the §8 veto row]).
Income-denial routing IS captured — it changes the credit vector, hence the label [DL 2026-07-19 §8
veto row].

### 4.4 Why simple paths + a monotone cost cutoff are complete and sound

- **Completeness**: legality is judged on the EFFECTIVE route; retracing/self-intersecting
  declarations are not real movements and cannot multiply fees, so every legal table movement's
  effective path is a simple path [DL 2026-07-20 effective-route row — recorded owner ruling,
  closing the external review's I-6]. Enumerating all simple paths therefore enumerates the whole
  movement surface.
- **Soundness of pruning**: every fee on a route is a nonnegative Yen amount (president fees +1¥
  each [RB p.6], canal tolls +1¥ [RB p.14]); nothing on a route refunds or earns money (no rule
  grants income during the Movement step). Accumulated cost is therefore monotone nondecreasing in
  path extension, so a prefix whose cost already exceeds `money` can never extend to an affordable
  route — pruning it loses nothing. Boards are ≤18 areas and the step-requires-own-assistant rule
  restricts the walk graph to the mover's assistant-occupied areas, so exhaustive DFS is trivially
  tractable.

---

## 5. Module decomposition (`crates/yokohama-oracle/src/`)

Dependency rule (plan-binding): `Cargo.toml` depends on `yokohama-core` for TYPES only; every
`use` is from `yokohama_core::types::*` plus `yokohama_core::game::Rules` (the static-data bundle —
data, not logic). No other engine import is permitted; stage review re-checks imports.

Modules are cut along rules boundaries so 3–5 authors implement in parallel against THIS SPEC
without talking to each other. Every function below takes `&GameState` (+ `&Rules` where card data
is needed) and returns owned values; no module calls into another except via the signatures listed.

### `lib.rs` — types + dispatch (Author A)
```rust
pub struct RouteLabel { pub destination: AreaId, pub fees: BTreeMap<Option<PlayerId>, u32> }
impl RouteLabel { pub fn mover_cost(&self) -> u32; }
pub struct OracleMoves { pub exact: Vec<Move>, pub movement: Vec<RouteLabel> }

pub fn oracle_moves(state: &GameState, rules: &Rules) -> OracleMoves;
// Dispatch order: game-over => empty; queue non-empty => choices::head_choice_moves;
// else per-phase per the §1.4 table. Dedup + stable sort of outputs for determinism.
```

### `common.rs` — shared rules-arithmetic helpers (Author A)
```rust
/// §1.4: mercenary target when is_mercenary_action, else President's OnBoard area.
pub(crate) fn acting_area(state: &GameState) -> Option<AreaId>;
/// §1.4 power table, capped at 5. `count_president` false during mercenary sub-turns.
pub(crate) fn action_power(state: &GameState, player: PlayerId, area: AreaId) -> u8;
/// Opponent presidents OnBoard(area), as (owner, count=1) pairs.
pub(crate) fn opponent_presidents_in(state: &GameState, mover: PlayerId, area: AreaId) -> Vec<PlayerId>;
/// True iff `player` owns a tech with the given effect (via rules.techs id lookup).
pub(crate) fn owns_effect(state: &GameState, rules: &Rules, player: PlayerId, effect: TechEffect) -> bool;
/// §3.4 Church/Customs assistant source: hand OR acting area (DL 2026-07-18 I3).
pub(crate) fn track_assistant_available(state: &GameState, area: AreaId) -> bool;
/// Non-Canal areas of the board, ascending AreaId.
pub(crate) fn placeable_areas(state: &GameState) -> Vec<AreaId>;
```

### `choices.rs` — §2 queue-head resolution (Author A)
```rust
pub(crate) fn head_choice_moves(state: &GameState, rules: &Rules) -> Vec<Move>;
// One arm per PendingChoice variant, exactly per §2.1–2.7. Match must be wildcard-free
// so a future 8th variant breaks the build here.
```

### `additional_actions.rs` — §3.1 (Author E)
```rust
pub(crate) fn additional_action_moves(state: &GameState, rules: &Rules) -> Vec<Move>;
pub(crate) fn achievement_met(state: &GameState, player: PlayerId, a: AchievementType) -> bool;
pub(crate) fn order_fulfillable(state: &GameState, rules: &Rules, player: PlayerId, id: OrderCardId) -> bool;
pub(crate) fn mercenary_targets(state: &GameState, player: PlayerId) -> Vec<AreaId>;
```

### `placement.rs` — §3.2 (Author B)
```rust
pub(crate) fn placement_moves(state: &GameState, rules: &Rules) -> Vec<Move>;
/// 1 Yen × assistants placed × opponent presidents, per target area (§3.2 fee rule).
pub(crate) fn placement_fee(state: &GameState, player: PlayerId, per_area_counts: &[(AreaId, u8)]) -> u32;
/// Physical-contiguity neighbors minus canal-crossing pairs minus Canal (Telegram scope, §3.2).
pub(crate) fn telegram_adjacent(state: &GameState, area: AreaId) -> Vec<AreaId>;
```

### `movement.rs` — §3.3 + §4 (Author B)
```rust
pub(crate) fn movement_moves(state: &GameState, rules: &Rules) -> (Vec<Move>, Vec<RouteLabel>);
/// §3.3-2 retreat family: every per-area count map incl. empty.
pub(crate) fn retreat_family(state: &GameState) -> Vec<Move>;
/// §3.3-1 PlacePresident destinations with fees priced (Postal/Stagecoach aware).
pub(crate) fn place_president_moves(state: &GameState, rules: &Rules) -> Vec<Move>;
/// §4.1–4.2: simple-path DFS with monotone cutoff + Tram/Station labels, deduped.
pub(crate) fn route_labels(state: &GameState, rules: &Rules) -> Vec<RouteLabel>;
```

### `area_actions_market.rs` — §3.4 production, Bank, Employment Agency, Chinatown (Author D)
```rust
pub(crate) fn production_or_bank_move(area_type: AreaType) -> Option<Move>;
pub(crate) fn employment_moves(state: &GameState, power: u8) -> Vec<Move>;
// Round-2 correction: was `(state, power)`. `rules` threaded in because the
// Mining Technology rider (§3.4 Chinatown, added at round-2 triage) makes the
// 2¥→Copper delta ownership-dependent — effect lookup via `rules.techs` (§9-C
// data access only).
pub(crate) fn chinatown_moves(state: &GameState, rules: &Rules, power: u8) -> Vec<Move>;
```

### `area_actions_cards.rs` — §3.4 Port, Laboratory (Author C)
```rust
pub(crate) fn port_moves(state: &GameState, rules: &Rules, area_type: AreaType, power: u8) -> Vec<Move>;
pub(crate) fn laboratory_moves(state: &GameState, rules: &Rules, area_type: AreaType, power: u8) -> Vec<Move>;
```

### `area_actions_tracks.rs` — §3.4 Church, Customs (Author D)
```rust
pub(crate) fn church_moves(state: &GameState, power: u8, acting: AreaId) -> Vec<Move>;
pub(crate) fn customs_moves(state: &GameState, power: u8, acting: AreaId) -> Vec<Move>;
/// Eligible empty church spaces ≤ faith, ≥4, 2P-dummy-aware (shared with choices.rs §2.4).
pub(crate) fn eligible_church_spaces(state: &GameState, total_faith: u8) -> Vec<u8>;
/// Eligible empty customs spaces for an exact cost, 2P-dummy-aware (shared with §2.5).
pub(crate) fn eligible_customs_spaces(state: &GameState, cost: u8) -> Vec<u8>;
```

`area_action_moves` (in `lib.rs` or a thin `area_actions.rs` facade, Author A) dispatches on the
acting area's `AreaType`, appends `AreaAction::Skip` per `allows_area_action_skip`.

### `buildings_and_bonus.rs` — §3.5 + §3.6 (Author E)
```rust
pub(crate) fn five_power_moves(state: &GameState) -> Vec<Move>;
pub(crate) fn building_moves(state: &GameState, rules: &Rules) -> Vec<Move>;
```

**Author split (5 parallel lanes):** A = lib/common/choices/dispatch; B = placement + movement;
C = port + laboratory; D = employment/chinatown/production/bank + church/customs;
E = additional_actions + buildings_and_bonus. Each lane's fixtures (§6) live beside it in
`crates/yokohama-oracle/tests/` (or `#[cfg(test)]` modules); no lane needs another lane's code to
compile its tests against the signatures above (Author A lands the skeleton first with `todo!()`
bodies so signatures exist — the one serialization point).

Registry-accessor note: order/tech lookups go through the PUBLIC surface of `rules.orders` /
`rules.techs` (declared in engine files outside this spec's allowlist). Module authors use the
public lookup methods as published in rustdoc — data access only; taking logic from those files
remains forbidden. Recorded as an unpinned surface in §7.2.

---

## 6. Fixture-test plan

Every fixture: build a minimal `GameState` by hand (via `BoardLayoutBuilder` + the public type
constructors; `Rules::empty_for_tests()` unless card data matters, else `Rules::standard()`), call
`oracle_moves`, assert presence/absence of specific moves/labels. Each fixture cites its source.

### 6.1 Rulebook-example fixtures (one per printed example)

| # | Source | Fixture asserts |
|---|---|---|
| F1 | [RB p.5] Hisashi placement example | On a board where Copper Mine is president-free, Employment Agency holds Blue's president, Laboratory A holds Green+Yellow presidents, and the mover has 1 Yen: `PlaceThreeAreas [CopperMine, Employment, X]` present with the Employment fee 1 affordable; every Option-A set including Laboratory A absent (2-Yen fee unaffordable); with money = 2 the Lab-A set appears. |
| F2 | [RB p.6] Hisashi movement example | President on Tea Plantation; own assistants on Employment Agency and Silk Mill only; Blue president on Employment Agency. Labels: `{SilkMill, {Some(Blue):1}}` present (pass-through fee); NO label with destination Employment Agency (blocked, no Postal); no route via Laboratory A (no assistant). With Stagecoach: `{SilkMill, {}}`. With Postal System: `{Employment, {Some(Blue):1}}` appears. |
| F3 | [RB p.7] power-tally example | Own president + 3 assistants + 1 shophouse on Silk Mill ⇒ `action_power` = 5; production move is `ProduceSilk` alone (no Skip at a production area). |
| F4 | [RB p.7] Seiji building example | Power 4 at Church, zero shophouse and trading-house tokens in hand ⇒ Building phase = `{SkipBuilding}` only. |
| F5 | [RB p.8] mercenary example | `UseMercenary{English, CopperMine}` legal iff ≥1 own assistant at Copper Mine, own president elsewhere, face-up English mercenary, `!mercenary_used`; illegal when the president stands there; opponent president at the target does not price or block the move. |
| F6 | [RB p.8] achievement example | 5 Copper held, Copper5 in play, unclaimed-by-player, assistant in hand ⇒ `FulfillAchievement{Copper5}` present; absent after the player's own claim (claimed_by contains player). |
| F7 | [RB p.12] Hisashi port example | Power 3 at Port A, hand ≤1, money ≥2: 2-card takes enumerate all Low/Mid pairs with `Some(TwoYen)`; no pair touches High slots; with `imported ≥ 1` the `Some(OneImportedGood)` twin appears; at hand = 2 all 2-card takes vanish (limit 3); at hand = 3 only `Skip`. |
| F8 | [RB p.12] Tram laboratory example | Tram (printed cost 5, `Rules::standard()`) in slot 3 (+1 ⇒ 6); power 5, money ≥2 ⇒ `Laboratory{card, extra_yen: 2, extra_imported: 0, fifth_tech_payment: None}` present; with money 1 and imported 1 the `extra_imported: 1` cover is present and the yen cover absent. |
| F9 | [RB p.13] church faith example | Power 3, pay copper+silk+yen ⇒ faith 6; combo legal iff space ≤6 empty; with spaces 4,5 filled and 6 empty (singleton) the combo stays legal (inline place, no queue on this state's enumeration); eligibility asserts via `eligible_church_spaces`. Linear-faith pin: power 2, no goods ⇒ faith 2, every combo reaching <4 illegal [DL 2026-07-17 linear ruling]. |
| F10 | [RB p.14] customs example | Power 2, 1 imported good ⇒ `Customs{1}` legal (spaces 1,2 empty); `Customs{2}` absent (power caps at 1); power 5 + 4 imported ⇒ `Customs{4}` legal iff space 7 empty. |
| F11 | [RB p.14] Canal + [DL 2026-07-18 BOX CHECK (c)] | 3-row board with a Canal cell whose H/V/diagonal flank pairs exist: crossing label carries `{None:1}` bank toll; a first-edge crossing (start adjacent via crossing pair) also tolls; money 0 ⇒ crossing labels absent (affordability). |
| F12 | [RB p.15] Station text | Station token at area S, mover owns Station, no own assistant at S, opponent president at S ⇒ label `{S, {}}` present (documented §7.1-A1 reading); non-owner gets no such label. |
| F13 | [RB p.16] Telegram | Option B at area X with Telegram: `Some(adj)` enumerated for each contiguous neighbor, canal-crossing pair excluded [DL 2026-07-18 Telegram exclusion]; fee adds 1 per opponent president at `adj`; without a 3rd hand assistant the bonus forms vanish. |
| F14 | [RB p.16] Newspaper | With Newspaper: 4-distinct-area sets appear; without: capped at 3. Zero-placement `PlaceThreeAreas{[]}` present in both [DL 2026-07-18 I5]. |

### 6.2 Constructed fixtures per module (targeted, each cites its governing row)

- **choices.rs**: each variant's exact option space — AnyGood always 4 [§2.1]; WarehouseTake
  affordability matrix incl. TH progression at `already_taken` 0..3 and Decline always [§2.2, DL
  §5.3-3]; AssistantMove from-hand/from-area × non-Canal targets, `to ≠ from`, Decline [§2.3];
  ChurchSpace multi-space + 2P faith-7 dummy [§2.4]; CustomsSpace pair + 2P space-4 dummy [§2.5];
  RecoveryLeave two-option incl. lone-assistant semantics doc [§2.6, DL §5.3-5]; StationPlace =
  all non-Canal areas [§2.7].
- **placement.rs**: per-assistant fee — Option B in a president-area costs 2 not 1 [DL 2026-07-17
  C1]; multi-president area prices per president per assistant [RB p.5]; hand-0 ⇒ only `[]`.
- **movement.rs**: retreat family = exact product set over (count+1) per area, empty map included,
  president-in-hand variant legal [DL 2026-07-17 Option-B-from-hand; DL 2026-07-19 I5 expanded];
  no-return-to-start (`destination ≠ start` in every label) [RB p.6]; Tram excludes president-
  occupied areas and the current area, needs no destination assistant [DL 2026-07-17 I1];
  Postal+Stagecoach 2×2 fee matrix at destination and pass-through [RB p.15]; label dedup: two
  same-fee routes to one destination ⇒ one label [DL 2026-07-19 L2 row]; income-denial: two
  different-credit routes to one destination ⇒ two labels [DL 2026-07-19 §8 veto row].
- **area_actions_cards.rs**: one-per-title lab gate (own copy of a 2-copy title blocks the second)
  [DL 2026-07-17 C2]; 5th-tech penalty variants [RB p.12 Note]; port power-1 ⇒ Skip only [RB
  p.11]; empty `Port{[]}` never emitted [DL 2026-07-17 optionality].
- **area_actions_market.rs**: Employment token cap power−1 and TH take-cost summation [RB p.12–13];
  Chinatown sequential feasibility (0¥ + 1 copper, power 2 ⇒ `[CopperToYen, YenToTea]` legal —
  pins the §7.1-A3 reading); power cap on sequence length [CM Chinatown cap].
- **area_actions_tracks.rs**: church all-false substantive at power ≥4 [DL 2026-07-17 optionality];
  goods-backing and one-per-kind flags [RB p.13]; hand-0-but-area-assistant legal (mercenary shape)
  [DL 2026-07-18 I3]; customs exact-cost matching and pair-filled unplaceability [DL 2026-07-17
  Customs re-model].
- **buildings_and_bonus.rs**: every empty shophouse slot is a separate move [RB p.7]; own-shophouse-
  on-site blocks all slots [RB p.7]; TH build free-from-hand at power 4 [DL 2026-07-18 C3
  addendum]; `Take5PowerToken` requires power exactly 5 AND unclaimed token [RB p.7].
- **additional_actions.rs**: mercenary once-per-turn across windows [RB p.8]; University/Spinning
  Mill gating incl. `*_used` flags [DL 2026-07-19 §5.3-4]; order fulfillment requirement arithmetic
  against `Rules::standard()` ids (e.g. order #19 needs 1 of each good, 07-order-cards.yaml).
- **dispatch**: queue-first — with any pending head, `exact` is exactly §2's set and `movement` is
  empty in the Movement phase too [DL 2026-07-19 close-out row]; game-over ⇒ all-empty.

---

## 7. Ambiguity list & unpinned encodings

### 7.1 Genuine rules ambiguities (neither rulebook, KB, nor decision log settles them; collected, not guessed)

- **A1 — Station teleport preconditions.** Text: "You may move your President pawn to this area
  during the Movement Step for free, and you do not need to pay even if there are other President
  pawns in the Area. However, you may only move to this Area if you are doing this Area action"
  [RB p.15]. Two open readings: (i) does the Station area need ≥1 of the owner's assistants?
  Reading 1: no — the card text imposes only the must-act condition, and the Tram analog was ruled
  assistant-free [DL 2026-07-17 I1]; Reading 2: yes — the general "may stop in a Possible Action
  Area" rule is only partially overridden (president clause explicitly, assistant clause silently).
  Distinguishing state: owner's president on board, Station area holds zero own assistants —
  Reading 1 offers the teleport, Reading 2 does not. (ii) may the teleport be taken from HAND
  (President in hand)? Tram was ruled board-only on a "changes nothing" rationale [DL 2026-07-19
  from-hand-Tram row] that does NOT transfer: from hand, Station WOULD change something (landing
  with no assistant / on an opponent president). Distinguishing state: president in hand, Station
  area not a Possible Action Area. The oracle implements Reading 1 + board-only and flags both
  (§4.2) so divergence surfaces for triage.
- **A2 — Tokens5 achievement and face-down tokens.** Text: "Own a total of 5 5-Power and/or
  Foreign Mercenary tokens" [RB p.14]. Used mercenaries are flipped face-down but "keep … in their
  possession" is only stated for 5-Power tokens [RB p.7/p.8]. Readings: count all owned tokens
  (face-up + face-down; `MercenaryState::total() + five_power_tokens`), or count usable/face-up
  mercenaries only (`unused_count`). Distinguishing state: player with 3 5-Power tokens + 2 used
  mercenaries — claimable under the first reading only. End-game remainder scoring separately says
  "Unused Foreign Mercenary tokens" [RB p.10], which shows the rulebook distinguishes the notions
  when it wants to — weak evidence for the count-all reading here. Oracle implements count-all;
  flagged for the ruling list.
- **A3 — Chinatown: sequential proceeds.** Text: "The number of times you may exchange depends on
  the Action Power … For each exchange, you may do one of the following" [RB p.13]. Readings:
  exchanges are performed one at a time and later exchanges may spend earlier proceeds (sequential
  reading), vs. all exchanges must be payable from the pre-action holdings (budget reading).
  Distinguishing state: 0 Yen, 1 Copper, power 2 — "sell copper (+2¥), buy silk (−2¥)" is legal
  only under the sequential reading. The oracle implements the sequential reading (the natural
  sense of "each exchange" as a step); flagged because no source states it.

### 7.2 Unpinned ENCODING semantics (allowed sources do not fix the engine's convention; oracle emits a documented normal form; runner must normalize)

Mirrors the `encoding_unpinned` list in the structured report:

1. **`Move::PlaceThreeAreas.placements` element order** — multiset semantics are pinned (the CLI
   matches "by placement-multiset equality" [DL 2026-07-18 placement-grammar row]) but the
   generator's canonical element order is not. Oracle: sorted ascending `AreaId`; runner: compare
   under multiset normalization.
2. **`AreaAction::Port.cards_taken` order for 2-card takes** — oracle: ascending slot index;
   runner normalizes as a set.
3. **`AreaAction::Laboratory` payment enumeration** — `extra_yen`'s denomination (the field doc
   says "Extra Yen paid" ⇒ Yen units, 2 per Industry point — adopted) and whether over-covering
   payments are enumerated (oracle: exact covers only).
4. **`AreaAction::Chinatown.exchanges` canonicalization** — sequences vs multisets; whether
   feasible permutations are distinct legal moves. Oracle: all feasible ordered sequences; the
   runner may need multiset-with-feasibility normalization if the engine canonicalizes.
5. **`Skip5PowerBonus` / `SkipBuilding` at ineligible power** — whether the engine PRESENTS these
   phases when power < 5 / < 4 (auto-advance vs. singleton skip). Oracle defines the skip-only
   answer either way; a reachability mismatch here is a presentation question, not a rules one.
6. **Non-terminal `TurnEnd` / choice-free `Recovery` presentation** — believed transient (Recovery
   is automatic [RB p.7]; TurnEnd "advance to next player" [types::moves]); oracle answers empty.
7. **Rules registry accessor names** (`OrderRegistry`/`TechRegistry` lookup surface) — declared in
   engine files off the allowlist; authors bind to the published public surface, data-access only.

`Move::MovePresident.path`'s canonical representative contents are deliberately NOT needed — the
label contract (§1.1) exists precisely to sidestep that encoding.

---

## 8. Attestation — every file read for this spec

| File | Why |
|---|---|
| `docs/plans/2026-07-20-phase-2.9-differential-oracle-plan.md` | Phase brief: contract, independence rule, staging |
| `guidingpath.md` (all 203 lines incl. the full decision log) | Normative rulings + engine encoding conventions |
| `analysis-1/00-yokohama-rulebook.pdf` (pages 1–16, all) | Final rules authority; all `[RB p.N]` citations |
| `analysis-1/yokohama-kb/01-rules-reference.md` | Rules reference (secondary to rulebook/DL) |
| `analysis-1/yokohama-kb/07-order-cards.yaml` | Verified order-card data (fixture ids, reward keys) |
| `analysis-1/yokohama-kb/08-tech-cards.yaml` | Verified tech data (costs, copies, effect texts) |
| `analysis-1/yokohama-kb/09-building-sites.yaml` | Verified site data (space rewards for §3.6/§6) |
| `crates/yokohama-core/src/types/mod.rs` | Type-module inventory |
| `crates/yokohama-core/src/types/ids.rs` | PlayerId/AreaId/card ids; RouteVector keying note |
| `crates/yokohama-core/src/types/player.rs` | PlayerState, assistant/shophouse/TH states, take_cost |
| `crates/yokohama-core/src/types/board.rs` | AreaType, BoardLayout, sites, five-power effects |
| `crates/yokohama-core/src/types/cards.rs` | Order/tech/achievement types, Port/Lab boards |
| `crates/yokohama-core/src/types/tracks.rs` | Church/Customs tracks, mercenary supply |
| `crates/yokohama-core/src/types/moves.rs` | TurnPhase, Move, AreaAction — the exact encoding |
| `crates/yokohama-core/src/types/state.rs` | GameState, GameConfig, pending_choices, sub-turn fields |
| `crates/yokohama-core/src/types/errors.rs` | MoveError vocabulary |
| `crates/yokohama-core/src/types/choices.rs` | PendingChoice variants, WarehouseItem, MoveSource |
| `crates/yokohama-core/src/game/rules.rs` | Rules accessor surface ONLY (public fields, content_hash) |
| `CLAUDE.md` | Rules-knowledge section (provided in session context) |

**Not read** (explicitly, per the independence rule): anything under
`crates/yokohama-core/src/game/` other than `rules.rs` (no `legal_moves.rs`, `apply_move.rs`,
`movement.rs`, `scoring.rs`, `choices.rs`, `actions/`, `data.rs`, `board_gen.rs`, `cards.rs`),
`analysis-1/yokohama-kb/01-rules-reference-engine-mapped.md`, KB files 02–06, all test files
(including `types/tests.rs`, which the allowlist technically permits but which was skipped as
unnecessary), all other `docs/` files, and the CLI crate. Directory listings (`ls`, `wc -l`,
PDF page count) were used for navigation only.

---

## 9. Orchestrator amendments (Stage-1b gate, 2026-07-20)

Recorded by the orchestrating session at the spec gate. All three are ENCODING-class
adjudications (phase-plan divergence class 3 — harmonized on the oracle side without touching any
rules question) or interface decisions; none injects an engine rules belief.

- **A. API adoption.** The crate API follows §1.1/§5 verbatim: `RouteLabel { destination, fees }`
  with derives `Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize` and
  `mover_cost()` as a method; entrypoint `pub fn oracle_moves`. The scaffold stub's earlier
  `{to, mover_cost, player_credits, bank}` shape and `legal_moves` name are SUPERSEDED; the
  impl-aware difftest harness adapts to this shape (orchestrator-owned change on the impl-aware
  side).
- **B. Chinatown emission (encoding adjudication).** §3.4's "enumerates ALL feasible ordered
  sequences" is amended: the oracle emits ONE representative `AreaAction::Chinatown` per
  ACHIEVABLE multiset — achievable = at least one ordering of the multiset passes §3.4's
  sequential running-balance validation — with the representative's `exchanges` Vec sorted by
  variant (any fixed total order; the runner normalizes). Rationale: the comparison unit for this
  payload is the multiset (a canonical-representative encoding, the class the phase plan assigns
  to normative-encoding harmonization), and enumerating orderings would only multiply comparison
  keys (~66k sequences vs ~2k multisets at power 5). The §7.1-A3 sequential-vs-budget RULES
  ambiguity is UNAFFECTED and stays live: the two readings produce different achievable-multiset
  sets, which is exactly what the differential comparison tests.
- **C. Registry data-access surface (closes §7.2-7).** Module authors may use, as data access
  only: `rules.orders.get(OrderCardId) -> Option<&OrderCard>`, `rules.orders.iter()`,
  `rules.techs.get(TechCardId) -> Option<&TechCard>`, `rules.techs.iter()`, and the public fields
  `rules.building_sites: Vec<BuildingSite>` / `rules.five_power_tokens: Vec<FivePowerEffect>`.
  Signatures published by the orchestrator (names only, no bodies); `game/cards.rs` itself
  remains forbidden to oracle authors.
- **D. Station interim (round-1 triage, 2026-07-20).** §4.2's Reading-1 emission is SUPERSEDED
  in code: the §7.1-A1 divergence surfaced live (3/60 games, byte-identical repro, preserved as
  `crates/yokohama-difftest/fixtures/ruling-a1/`), was verified, and is with the owner as ruling
  A1. Pending that ruling the oracle mirrors the engine's Reading 2 (own assistant required)
  behind the `PENDING-RULING §7.1-A1` marker so the differential gate binds; the engine is
  deliberately untouched (no unadjudicated reading enters `rules_hash`-stamped records). The
  ruling flips exactly one condition on whichever side loses. (Recorded here at the Stage-1
  review's must-fix: an attested spec must not assert the opposite of the code on the very
  question the harness exists to probe.)
- **D. Chinatown Mining rider (round-2 triage, 2026-07-20 — class-2 ORACLE BUG, fixed).** The
  oracle's Chinatown running-balance model (`chinatown_apply`, YenToCopper) added exactly 1
  copper, omitting the Mining Technology rider — verified card text (08-tech-cards.yaml Mining
  Technology): "Whenever you gain one or more Copper: gain 1 additional Copper", UNIVERSAL per
  DL 2026-07-18 ("Mining Technology applies to EVERY Copper receipt") and KB 01:596; stacking is
  +1 per owned copy per receipt. Observed consequence: with 1x Mining, 3 Yen, 0 copper, power 3,
  the achievable multiset {CopperToYen, CopperToYen, YenToCopper} (buy first ⇒ 2 copper, then
  sell twice) was wrongly absent. THE MISREADING, recorded verbatim: the spec's §3.4 Chinatown
  rate list transcribed RB p.13's printed exchange rates but missed the cross-cutting Mining
  rider that lives on the tech card and in the decision log, not on the Chinatown board — and
  lane D implemented the spec faithfully. This cross-reference miss is itself evidence the rules
  sources make the interaction easy to overlook. Fix: §3.4 rate list amended, `chinatown_moves`
  signature gains `rules: &Rules` (§5), `mining_copies` computed once before the DFS.
