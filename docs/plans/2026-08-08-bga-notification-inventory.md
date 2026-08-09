# BGA notification-type inventory — the H0 analysis artifact

**Date:** 2026-08-08 · **Phase:** 3.5b (BGA→CLI bridge), task H0 · **Status:** analysis, no code
**Authority:** this doc is **H2's authority** on the wire shape. Anything it marks **UNKNOWN** is a thing H2 must discover on real data or hard-error on — never guess.
**Governing docs:** `docs/plans/2026-08-07-bga-bridge-design.md` (approved design) · `docs/plans/2026-08-08-phase3.5b-execution-ledger.md` (phase state)

---

## 0. Provenance, and what this doc is and is not evidence of

Every number below was **measured** from one corpus, by parsing it. Nothing here is inferred from BGA documentation, from the game's client source, or from memory.

| Fact | Value |
|---|---|
| Corpus file | `bridge-captures/895724349.ndjson` (gitignored — data, not source) |
| Capture lines | **491** (each a JSON object; the brief's "490" is off by one — re-measured) |
| Bytes | 1,897,569 (1.81 MiB) |
| Wall span | `received_at_ms` 1786155213076 → 1786158337587 = **3,124.5 s** (~52 min) |
| Table | **#895724349**, 2 players, complete game to `finalScoring` |
| Seats | Player_A `88889596` (**the capturing session's own seat**, seat no. 1) · Player_B `88368036` (seat no. 2) |
| BGA build | `gameversion 260109-1022`, `siteversion 260807-1629`, `gameserver 11` (from `tableInfosChanged`) |
| Companion artifact | `bridge-captures/895724349-notif-handlers.json` — 24 `notif_*` handler names enumerated from the live client |

**Scope honesty.** This is **one table, one player count (2P), one BGA build, one capturing seat**. Every count is that game's. Nothing here establishes that a 3P/4P table, a different build, or the *opponent's* session sees the same shapes. Design §6's drift posture applies: pin to these shapes, hard-error on a surprise.

**Reproduction.** All figures are reproducible with `python3` over the NDJSON; the parse contract is: each line is `{received_at_ms, kind, payload}`; `kind ∈ {snapshot, packets}`; for `payload.source ∈ {xhr, fetch}` the BGA packets are `json.loads(payload.body_text)["data"]["data"]`; for `payload.source == "websocket"` `payload.body_text` holds **one or more newline-separated JSON frames** and the packets are each frame's `push.pub.data`.

---

## 1. Corpus shape — three transports, and only two of them carry game state

| `kind` | `payload.source` | lines | what it is |
|---|---|---|---|
| `packets` | `websocket` | 484 | Centrifugo-style realtime frames from `wss://ws-x3.boardgamearena.com/connection/websocket` |
| `packets` | `xhr` | 4 | `notificationHistory.html?table=…&from=N&privateinc=1&history=1` responses |
| `snapshot` | *(absent)* | 3 | full `gameui.gamedatas` dumps (2 mid-game, 1 end-of-game) |

**`payload.source == "fetch"` does not occur in this corpus.** The extension wraps `fetch` as well as XHR, but BGA's client used XHR for every history request here. H2 must still accept `fetch` — its absence is an observation about this session, not a guarantee.

**WebSocket frame taxonomy** (484 lines split into 494 newline-separated frames):

| frame shape | count | disposition |
|---|---|---|
| `{"push": {...}}` | 338 | the only game-bearing shape |
| `{}` | 128 | keepalive/heartbeat — **no-op chrome** |
| `{"id":N,"subscribe":{…}}` | 22 | transport handshake — no-op |
| `{"id":N,"connect":{…}}` | 4 | transport handshake — no-op |
| `{"id":N,"presence":{…}}` | 2 | roster — no-op |

`push` frames by channel: `/table/t895724349` **306 pub + 10 join + 2 leave** · `/player/p88889596` 12 pub · `/chat/general` 8 pub. **Only `/table/t<table_id>` `pub` frames matter to the translator**; `join`/`leave`, the player channel and the general chat channel are chrome. There were no binary frames (no `note: "non-text frame"` stubs) and no unparseable fragments.

---

## 2. The packet envelope, and the two `packet_type`s that carry moves

Every game-bearing payload is a **packet** containing a `data` array of **notifications**. Three envelope shapes occur:

```
history  (xhr)       {channel, table_id, packet_id, packet_type:"history",  move_id, time, data[]}
sequence (websocket) {channel, table_id, packet_id, packet_type:"sequence", move_id, time,
                      id, prevpacket:[…], gamename, data[]}
single   (websocket) {channel, packet_type:"single", id, time, data[]}      ← no packet_id, no move_id
```

Counts: **history 310 · sequence 202 · single 124** (raw, before dedupe).

- **`sequence`** is the move-carrying live shape. It is the one with `prevpacket`, and therefore the only one that can express supersession.
- **`single`** carries only out-of-band chrome in this corpus: `yourturnack`, `chat`, `playerstatus`, `resultsAvailable`, `tableInfosChanged`, `updatePlayerNotifCount`, `updatePlayerTableStatus`. **No `single` packet ever carried a game notification here.**
- **`history`** is the catch-up shape served by `notificationHistory.html`. See §5 — it is *not* interchangeable with `sequence`.

**Scalar-type trap (measured, not theoretical).** `packet_id` and `move_id` are **JSON strings in `history`** (`"124"`) and **JSON integers on the socket** (`124`). `table_id` is an integer in both. H2 must normalise at the parse boundary; a naive `==` between the two feeds silently never matches.

**Notifications per packet** (table channel): `{0:82, 1:259, 2:70, 3:100, 4:17, 5:42, 6:4, 7:4, 8:4, 9:8, 10:14, 11:2, 12:2, 13:6, 14:2}`. Empty-`data` packets are normal (82 of them) — a packet is a *transmission*, not a move.

**Packet coverage of the game.** Union of all four history fetches plus the socket: `packet_id` **1 … 212**, 207 present. The five absent ids — 27, 64, 69, 102, 103 — are exactly the packets superseded by takebacks that happened **before** the socket capture began (§4); they were never in reach. Surviving `move_id` runs **1 … 194** with no gaps.

**The two feeds cover different halves.**

| range | available as | args richness |
|---|---|---|
| `packet_id` 1–111 (`move_id` 1–106) | **history only** | THIN (§5) |
| `packet_id` 112–212 (`move_id` 107–194) | history **and** socket (socket complete, no gaps) | RICH on the socket |

This is the single most consequential structural fact for H2: **the first 55 % of this game exists only in the thin form.**

---

## 3. Dedupe — measured, and what it is safe to key on

Delivery is at-least-once (ledger note N-4). Measured multiplicities:

- **Every socket packet arrived exactly twice** — 101 distinct `packet_id`s × 2 deliveries, 0 variance. (BGA's client holds two sockets; both receive the table channel.)
- History packets: 93 delivered once, 101 twice, 5 three times — the four overlapping fetches.
- Content-identity on the raw capture line (payload minus `captured_at_ms`): 491 lines → **196 distinct**, dup rate **0.601**. The multiplicity histogram is `{1:28, 2:166, 3:1, 128:1}`; the ×128 entry is the `{}` heartbeat.
- **Payload identity including `captured_at_ms` collapses only 5 of 491 lines** (dup rate 0.010). The design's "dedupe on payload identity, never on `received_at_ms`" is right in spirit but must be read as *identity of the BGA content*, excluding **both** the relay's `received_at_ms` and the extension's `payload.captured_at_ms`.

**The keys, in order of strength:**

1. **`packet_id` is the packet identity.** Globally unique, never reused: across 101 socket packet_ids delivered twice each, **zero** produced two byte-distinct `data` arrays. Deduping the socket feed on `packet_id` is exact.
2. **`uid` is the notification identity.** Every one of the 1,445 raw notification instances carries a `uid`; there are **665 distinct**. Within a transport, same `uid` ⟹ byte-identical notification (526 multiply-delivered uids, all identical).
3. **`move_id` is NOT an identity** — a takeback rewinds it and the replayed line reuses the same values (§4). Never dedupe on it.
4. **`uid` does not survive across transports as a *content* key.** 117 uids appear on both feeds; **110 of them carry different args** — see §5. Same uid, different payload. Deduping the two feeds against each other on `uid` therefore keeps whichever copy arrives first, which is exactly the wrong outcome if that is the thin one.
5. **`uid` does not collapse a takeback.** Superseded and replayed notifications have *distinct* uids even when they describe the identical move (verified at packets 122/125 and 123/126: zero uid overlap, identical `tile_name`/`power`/`action`). Supersession is a packet-level operation and cannot be done at notification level.

---

## 4. Supersession — the design's top risk, now fully characterised

**Nine takebacks occurred in this game.** Five were captured live on the socket; four predate the socket capture and are visible only as gaps in the history feed.

### 4.1 The rule, as observed

Every `sequence` packet carries `prevpacket`. In the normal case it is exactly `[str(packet_id − 1)]` — verified for **all 101 captured socket packets except the five undo packets**. The exceptions are the whole signal:

| undo `packet_id` | `prevpacket` | undo `move_id` | superseded `packet_id`s | superseded `move_id`s |
|---|---|---|---|---|
| 124 | `["121"]` | 117 | 122, 123 | 117, 118 |
| 131 | `["128"]` | 122 | 129, 130 | 122, 123 |
| 149 | `["147"]` | 139 | 148 | 139 |
| 159 | `["156"]` | 147 | 157, 158 | 147, 148 |
| 170 | `["168"]` | 157 | 169 | 157 |

**The rule:**

> An `undoRestorePoint` packet U with `prevpacket = [P]` supersedes **every packet whose `packet_id` lies strictly between P and U's own `packet_id`**. U's own `move_id` equals the `move_id` of the first superseded packet. The next packet after U carries `prevpacket = [U.packet_id]` and `move_id = U.move_id + 1`.

Two equivalent detectors, and the stronger one is the structural one:

- **Structural:** `prevpacket != [packet_id − 1]` on a `sequence` packet. Fired 5/5, zero false positives across 101 packets.
- **Semantic:** a notification of type `undoRestorePoint` in the packet, plus the `move_id` regression the ledger recorded (118→117, 123→122, 148→147 — three of the five; the other two are 139→139 and 157→157 arrival-order *non-increases*, which a strict `<` test **misses**).

> **H2 must use the structural detector.** The ledger's "move_id REGRESSION" phrasing is true of the three instances it names but is **not** a complete detector: at undos 149 and 170 exactly one packet was superseded, so the undo packet's `move_id` **equals** rather than falls below the previous arrival's. A `new_move_id < max_seen` test catches 3 of 5. `prevpacket != [pid−1]` catches 5 of 5.

### 4.2 Verbatim excerpt — undo packet 124

```json
{"packet_id": 124, "packet_type": "sequence", "channel": "/table/t895724349", "id": 0,
 "data": [{"uid": "6a7697cbaefd6", "type": "undoRestorePoint",
           "log": "${player_name} takes back his move",
           "args": {"player_name": "Player_A"},
           "lock_uuid": "a6caf967-3ef5-40f5-8449-9576a2516288", "h": "48a99c"}],
 "move_id": 117, "table_id": 895724349, "prevpacket": ["121"],
 "gamename": "yokohama", "time": 1786157003}
```

Its bracket, in arrival order (types abbreviated; `gsc(id=N)` = `gameStateChange` to BGA state N):

```
pid=121 mid=116 prev=[120]  gsc(5) | gsc(20)
pid=122 mid=117 prev=[121]  gsc(23) | assistantPlacement | gsc(30)        ← superseded
pid=123 mid=118 prev=[122]  gsc(33) | presidentPlacement | gsc(34) | resourceChange | gsc(60)   ← superseded
pid=124 mid=117 prev=[121]  undoRestorePoint
pid=125 mid=118 prev=[124]  gsc(23) | assistantPlacement | gsc(30)        ← replacement
pid=126 mid=119 prev=[125]  gsc(33) | presidentPlacement | gsc(34) | resourceChange | gsc(60)   ← replacement
```

The replayed move here happened to be **identical** to the superseded one (`Laboratory`, power 3, `places`). **Do not build on that** — it is an artifact of the owner's deliberate undo experiment (replaying what he had just done), not a property of takebacks.

### 4.3 The history feed has already collapsed it

The `notificationHistory` response serves **only the surviving line**. In the final fetch (`from=213`, 187 packets), `packet_id` runs 13…212 with gaps at exactly `{27, 64, 69, 102, 103, 122, 123, 129, 130, 148, 157, 158, 169}` — the 13 superseded packets from all nine takebacks — while `move_id` runs **13…194 contiguously**. The `undoRestorePoint` notification itself *is* retained in history (9 of them), where it is a **pure log marker with no state effect**, because the packets it cancelled are already gone.

**Consequence for H2, stated as two different jobs:**

- **Catch-up from history:** no collapse logic needed. Treat `undoRestorePoint` as chrome. The line you are handed is already committed.
- **Live-follow from the socket:** collapse is mandatory and must be *retroactive* — packets already applied to the engine have to be unwound. Since `GameState` is history-free by design, the only honest mechanism is **re-derivation**: keep the surviving packet list, and on an undo, rebuild from the last checkpoint rather than attempting an inverse-apply. (Design decision, not settled here.)

Five null-`move_id` history packets exist (`packet_id` 177, 188, 199, 200, 209) with empty `data`. They are not takebacks and carry nothing; they are inert.

---

## 5. THE TRANSPORT ASYMMETRY — history args are display-only

**This is the headline finding of H0's analysis half, and it was not anticipated by the design.**

The two feeds deliver the *same notifications* (same `uid`, same `type`, same `log`) with **different `args`**. Of the 117 uids seen on both transports, **110 differ**, and the direction is uniform: **the socket copy is rich, the history copy is thin. Never the reverse. (109 rich-vs-thin; the other 8 are types that carry no state payload on either feed.)**

Measured, over all 385 history notifications and all 936 socket notifications:

| | `args.counters` present | `args.workers` present |
|---|---|---|
| `history` (xhr) | **0 of 385** | **0 of 385** |
| `websocket` | 232 of 936 | 146 of 936 |

Same notification, both feeds:

```
socket : {"uid":"6a7696e29f60b","type":"moveAssistant", "args":{"player_id":"88368036",
          "player_name":"Player_B","workers":{…24 entries…},"counters":{…},
          "from_id":"0","to_id":"3","to_loc":"Employment Agency"}}
history: {"uid":"6a7696e29f60b","type":"moveAssistant", "args":{"player_id":"88368036",
          "player_name":"Player_B","to_loc":"Employment Agency"}, "h":"027ab9"}
```

```
socket : assistantRecovery args = {player_id, player_name, assist_count, counters{…}, workers{…}, tile_key}
history: assistantRecovery args = {player_id, player_name, assist_count}
```

**What the history feed strips, everywhere:** `counters` (the full per-player scoreboard + `eog_triggers`), `workers` (the full pawn map), `player_workers`, `orders`, `techs`, `tile_key`, `tile_keys`, `tile_key_2on1`, `pres_tile_key`, `trg_tile_key`, `route_keys`, `from_id`, `to_id`, `order_card`, `order_loc`, `tech_card`, `tech_loc`, `card_slot`, `import_slot`, `church_slot`, `result_info`, `agent_acquired`. What survives is exactly the set of substitutions the `log` template needs, plus `player_id`/`player_name`, plus an opaque `h`.

**Consequences H2 must design around, not around-the-houses:**

1. **`packet_id` 1–111 of this game (moves 1–106) can only be translated from display strings.** Board positions come as *names* (`"Copper Mine"`, `"Port"`), never keys; assistant placements come as a comma-joined name list; president routes have no `route_keys` at all, so pass-through fee recipients are not stated. A translator for the catch-up half is a **different and strictly harder problem** than a translator for the live half.
2. `gameStateChange` — the decision framing (§6) — is **socket-only**: 630 raw instances, **0 in history**. The catch-up half has no phase framing at all and must infer turn structure from the notification sequence.
3. There is no evidence in this corpus that `history=0`, a different `from`, or dropping `privateinc=1` returns richer args. **UNKNOWN**, and worth one cheap live probe before H2 commits: if some parameterisation of `notificationHistory` returns the socket-shaped args, the catch-up problem collapses to the live one.
4. **The bridge's real protection is capture-first.** A game followed by the socket from move 1 never has a thin half. The gate table should be joined and captured from the start.

**The `h` field** appears on 83 history and 192 socket notifications, on 15 types, and is absent on the rest. Its purpose is **UNKNOWN** (it looks like a 6-hex log digest). Inert — do not key on it. `lock_uuid` likewise appears on 188 notifications and is **UNKNOWN**; it is a BGA client-lock token, not game state.

---

## 6. `gameStateChange` — the decision framing, and the BGA state machine

`gameStateChange` is the highest-volume type (630 raw / 315 distinct uids / **296 surviving**) and is **socket-only**. Its `args` shape is constant:

```json
{"id": 20, "active_player": 88889596, "type": "activeplayer",
 "args": { …state-specific… },
 "reflexion": {"total": {"88889596":"180","88368036":"102"}},
 "updateGameProgression": 36}
```

- `args.id` — BGA state id. `args.type` ∈ `{game, activeplayer, multipleactiveplayer, manager}` (this is the framework's state *kind*, and it is what the ledger's census called the extra types "activeplayer" and "game": **they are `gameStateChange` arg values, not notification types of their own**).
- `args.active_player` — whose decision it is.
- `args.args` — the state's own args: `[]` (an empty JSON array, not an object) for `game`-kind states, a real object for `activeplayer` states.

The full 44-state machine is available verbatim in every `gamedatas` snapshot under `gamedatas.gamestates`. Transcribed, with the live-observed count and the engine concept it corresponds to:

| id | name | kind | seen | maps to |
|---|---|---|---|---|
| 1 | `gameSetup` | manager | 0 | setup (pre-capture) |
| 2 | `initialOrder` | multipleactive | 0 | `StartingOrderChoice` |
| 3 | `nextPhase` | game | 45 | phase pump — chrome |
| 4 | `additionalPhase` | activeplayer | 25 | `TurnPhase::AdditionalActionFirst`/`Second` |
| 5 | `mainPhaseSetup` | game | 18 | chrome |
| 6 | `nextPlayer` | game | 19 | `TurnPhase::TurnEnd` |
| 10 | `useAgent` | game | 2 | `Move::UseMercenary` resolution |
| 11 | `claimAchievement` | game | 0 | `Move::FulfillAchievement` resolution |
| 20 | `placeAssistantsStep` | activeplayer | 18 | `TurnPhase::Placement` |
| 21 | `placeAdjacentAssistant` | activeplayer | **0** | Telegram `adjacent_bonus` — **UNOBSERVED** |
| 22 | `payForAssistant` | activeplayer | **0** | placement fee confirm — **UNOBSERVED** |
| 23 | `confirmAssistants` | game | 23 | placement commit |
| 30 | `movePresidentStep` | activeplayer | 23 | `TurnPhase::Movement` |
| 31 | `removeAssistants` | activeplayer | 1 | `ReturnPresidentToHand{recover}` |
| 32 | `payForPresident` | activeplayer | **0** | pass-through fee confirm — **UNOBSERVED** |
| 33 | `placePresident` | game | 20 | movement commit |
| 34 | `activation` | game | 22 | `TurnPhase::AreaAction` |
| 35 | `sellImports` | activeplayer | 3 | `AreaAction::Customs` |
| 36 | `complementFaith` | activeplayer | 2 | `AreaAction::Church` |
| 37 | `selectResource` | activeplayer | **0** | `PendingChoice::AnyGood` — **UNOBSERVED** |
| 38 | `tradeResource` | activeplayer | **0** | `AreaAction::Chinatown` — **UNOBSERVED** |
| 40 | `warehouseOut` | activeplayer | 2 | `AreaAction::EmploymentAgency` / `PendingChoice::WarehouseTake` |
| 41 | `completePhase` | activeplayer | 1 | `Move::FulfillOrder` |
| 42 | `claimPhase` | activeplayer | **0** | `Move::FulfillAchievement` — **UNOBSERVED** |
| 43 | `agentPhase` | activeplayer | 2 | `Move::UseMercenary` target select |
| 50 | `selectOrder` | activeplayer | 4 | `AreaAction::Port` first card |
| 51 | `moreOrder` | game | 6 | chrome |
| 52 | `selectOrder2` | activeplayer | 2 | `AreaAction::Port` second card |
| 55 | `completeOrder` | game | 6 | order-completion commit |
| 60 | `selectTech` | activeplayer | 3 | `AreaAction::Laboratory` |
| 62 | `acquireTech` | game | 1 | tech commit |
| 63 | `placeStation` | activeplayer | **0** | `PendingChoice::StationPlace` — **UNOBSERVED** |
| 70 | `moveAssistant` | activeplayer | 2 | `PendingChoice::AssistantMove` (source) |
| 71 | `moveAssistantTo` | activeplayer | 1 | `PendingChoice::AssistantMove` (target) |
| 80 | `selectConstruct` | activeplayer | 1 | `TurnPhase::Building` |
| 81 | `buildShop` | game | 1 | `Move::BuildShophouse` commit |
| 82 | `buildHouse` | game | **0** | `Move::BuildTradingHouse` — **UNOBSERVED** |
| 90 | `finalScoring` | game | 1 | scoring |
| 99 | `gameEnd` | manager | 1 | terminal |
| 301 | `placePresidentStep` | activeplayer | **0** | `Move::PlacePresident` from hand — **UNOBSERVED** |
| 341 | `powerBonus` | game | 20 | `TurnPhase::FivePowerBonus` |
| 342 | `construction` | game | 20 | `TurnPhase::Building` gate |
| 343 | `recovery` | game | 20 | `TurnPhase::Recovery` |
| 349 | `leaveAssistant` | activeplayer | **0** | `PendingChoice::RecoveryLeave` (Telephone) — **UNOBSERVED** |

The 13 unobserved states are unobserved **in the live socket window only** (`move_id` 107–194) — they may well have fired in moves 1–106, but `gameStateChange` is absent from the history feed, so the corpus cannot say. **Treat all 13 as UNKNOWN arg shapes.**

Observed non-empty `args.args` shapes, verbatim:

```
 4/41/43 additionalPhase {"achievement_avail":false,"achievement_keys":[],"agent_avail":false,
                          "order_avail":true,"order_keys":["2","5"],"worker_avail":false,"silk2yens":false}
 20 placeAssistantsStep  {"OnePerTileCount":3,"agent_avail":false,"eog_triggers":{…}}
 35 sellImports          {"custom_slots":[1,1,1,0,1,1,0]}      ← the 7 printed Customs spaces, 1 = free
 36 complementFaith      {"faith":"2"}
 40 warehouseOut         {"item_count":4}
 50/52 selectOrder       {"tile_name":"Port","order_range":"LOW to HIGH","order_loc":2,
                          "show_yens":0,"tile_power":"4","show_imports":0}
 60 selectTech           {"tile_name":"Laboratory","tech_loc":2,"gear_count":"3","add_cost":1}
 70 moveAssistant        {"curr_move":1,"max_move":2}
 80 selectConstruct      {"tile_key":"2","tile_name":"Port","card_slots":[0,0,0,0,99]}
```

`args.args` is `[]` (empty **array**) for every `game`/`manager` state. A parser expecting an object must tolerate both.

---

## 7. The notification-type inventory

**38 distinct types observed.** Counts are given three ways because they answer three different questions:

- **raw** — instances in the file, duplicates included.
- **uniq** — distinct `uid`s (removes at-least-once duplication and cross-transport duplication).
- **surv** — distinct `uid`s **after supersession collapse** (excludes notifications in packets 122/123/129/130/148/157/158/169; the four pre-capture superseded packets never reached the corpus).

`surv` is the number a translator will actually consume.

### 7.1 Handler-inventory reconciliation

The client exposes **24** `notif_*` handlers. Of those:

- **21 observed.**
- **3 declared but NEVER observed → UNKNOWN args:** `stationPlacement`, `buildHouse`, `removeAssistants`.
- **1 observed but NOT in the handler list:** **`removeNoAssistants`** — near-miss to the declared `removeAssistants`. They are **different names**; do not conflate them. `removeNoAssistants` is what fired ("takes back no assistant in hand"); the declared `removeAssistants` handler never did.

The remaining **17 observed types are framework-level**, invisible to a `notif_*` handler probe: `gameStateChange`, `undoRestorePoint`, `yourturnack`, `updateReflexionTime`, `wakeupPlayers`, `payForPres`, `orderRefused`, `techRefusal`, `chat`, `playerstatus`, `simpleNode`, `tableWindow`, `resultsAvailable`, `tableInfosChanged`, `updatePlayerNotifCount`, `updatePlayerTableStatus`, plus `removeNoAssistants`.

### 7.2 Game-state types — the translator's real input

Args are given as `socket shape` / `history shape`. `counters{}` = the full per-player scoreboard object (fields listed in §7.5); `workers{}` = the full pawn map; both are elided in examples.

---

**`presidentPlacement`** — raw 97 / uniq 40 / **surv 37**
`log`: `${player_name} ${action} the president on the <B>${tile_name}</B> with a power of <B>${power}</B>`
socket args: `player_id, player_name, action, tile_name, power, pres_tile_key, trg_tile_key, route_keys[], player_workers{}, workers{}, counters{}`
history args: `player_id, player_name, action, tile_name, power`
example: `{"tile_name":"Copper Mine","power":"4","pres_tile_key":"9","trg_tile_key":"10","route_keys":["10"],"action":"places"}`
→ **`Move::MovePresident{path, destination}`** or **`Move::PlacePresident{destination}`**, discriminated by `action` (observed values: `"places"`; other values **UNKNOWN**). `trg_tile_key` is the destination, `pres_tile_key` the origin, `route_keys` the path. **Only the socket form is translatable to a path**; the history form gives destination-by-name only, which loses the pass-through fee recipients our `MovePresident` path encodes.

**`assistantPlacement`** — raw 105 / uniq 43 / **surv 38**
`log` (two variants): `…places 1 assistant on the: ${tile_names}` · `…places 2 assistants on the <B>${tile_name}</B>`
socket args: `player_id, player_name, tile_keys[], tile_names|tile_name, tile_key_2on1, player_workers{}, workers{}, counters{}`
history args: `player_id, player_name, tile_names|tile_name`
examples: `{"tile_keys":["1","5","6"],"tile_names":"<B>Tea Plantation</B>, <B>Fishing Ground</B>, <B>Silk Mill</B>","tile_key_2on1":"0"}` · `{"tile_keys":["10"],"tile_name":"Copper Mine","tile_key_2on1":"10"}`
→ **`Move::PlaceThreeAreas{placements}`** when `tile_key_2on1 == "0"`, **`Move::PlaceTwoSameArea{area, adjacent_bonus}`** when it names a tile. The Telegram `adjacent_bonus` case would appear as an Option-B placement with a second tile key — **UNOBSERVED in this corpus** (`tile_keys` had length 1 in every 2-on-1 sample); its encoding is **UNKNOWN**. History form: names are `<B>`-wrapped and comma-joined, and `tile_key_2on1` is absent, so Option A vs Option B is **not directly stated** in the history half — it must be inferred from the log template (`"places 1 assistant"` vs `"places 2 assistants"`).

**`moveAssistant`** — raw 29 / uniq 13 / **surv 13**
`log` (three variants): `…from <B>${from_loc}</B> to <B>${to_loc}</B>` · `…from their hand to <B>${to_loc}</B>` · `…from <B>${to_loc}</B> to their hand`
socket args: `player_id, player_name, from_id, to_id, from_loc?, to_loc, workers{}, counters{}`
history args: `player_id, player_name, from_loc?, to_loc`
observed: `{"from_id":"0","to_id":"3","to_loc":"Employment Agency"}` · `{"from_id":"4","to_id":"0","to_loc":"Bank"}` · `{"from_id":"4","to_id":"8","from_loc":"Bank","to_loc":"Customs"}`
→ **`Move::ChooseAssistantMove{from, to}`** (`PendingChoice::AssistantMove`). **`from_id`/`to_id` are tile keys with `0` meaning the player's hand** — established by joining the three samples against the log templates. **Log-text trap:** in the third variant the *source* is rendered in `${to_loc}`, so a translator written off `to_loc` alone reads the direction backwards. Trust `from_id`/`to_id` on the socket; on the history feed the direction is only recoverable from the **log template string**, which is a genuine fragility — flag as a hard-error site.

**`resourceChange`** — raw 115 / uniq 47 / **surv 44**
`log`: many variants (`…gains: 2 <B>Copper</B>`, `…uses a foreign agent to activate tile <B>${tile_name}</B> with a power of ${tile_power}`)
socket args (2 shapes): `{player_id, player_name, tile_key, result_info{tea,fish,silk,copper,yen,vps,gain_text[]}, counters{}}` · `{player_id, player_name, tile_name, tile_power, counters{}}`
history args: `{player_id, player_name}` (63 of 67 — the *entire* gain is in the rendered `log` string) · `{player_id, player_name, tile_name, tile_power}`
→ the **effect** of `Move::ExecuteAreaAction`, not the move. `result_info` is a typed delta and is the cross-check H2 should assert against. The mercenary variant (`tile_power`) corresponds to **`Move::UseMercenary`**. **On the history feed a plain `resourceChange` carries no structured amount at all.**

**`assistantRecovery`** — raw 101 / uniq 40 / **surv 40**
socket args: `player_id, player_name, assist_count, tile_key, workers{}, counters{}` · history: `player_id, player_name, assist_count`
→ `TurnPhase::Recovery` (automatic in our engine — **no `Move`**). Reconciliation input only.

**`orderAcquisition`** — raw 25 / uniq 9 / **surv 9** — **CARRIES A REVEAL**, see §8
socket args: `player_id, player_name, order_loc, order_card{…full card…}, orders{…all visible+deck…}, location_name, order_text, counters{}`
history args: `player_id, player_name, location_name, order_text` — and `order_text` is the literal string `"an order"`. **The history feed does not say WHICH card was taken.**
→ **`AreaAction::Port{cards_taken, second_card_payment}`**. `second_card_payment` maps to state 52 `selectOrder2` (`show_yens`/`show_imports` flags) — the *payment kind* is **UNKNOWN** from the notification alone.

**`orderRefresh`** — raw 17 / uniq 6 / **surv 6** — **CARRIES A REVEAL**, see §8
socket args: `player_id, player_name, location_name, order_card{…}, order_loc, orders{…}` · history args: `player_id, location_name`
→ the **port refill reveal**. Our model: `reveal.rs`'s `Pending::OrderReveals` — one revealed `OrderCardId` per refilled slot, prompted in draw order. The socket form supplies exactly this; the history form supplies **nothing but the board name**.

**`orderCompletion`** — raw 26 / uniq 10 / **surv 10**
socket args: `player_id, player_name, order{…full card…}, orders{…}, gain_list, agent_acquired, counters{}` · history args: `player_id, player_name, order{…full card…}, gain_list`
→ **`Move::FulfillOrder{order_id}`**. **This is the one card-bearing type whose history form retains the full card object** — so an order's identity becomes recoverable at completion time even in the thin half. `agent_acquired: true` signals a foreign-agent token.

**`techAcquisition`** — raw 9 / uniq 4 / **surv 4** — **CARRIES A REVEAL**, see §8
socket args: `player_id, player_name, tech_name, tech_card{tech_key, tech_id, country, location, position, req_gears, player_id}, tech_loc, location_name, techs{…all 30…}, agent_acquired, counters{}`
history args: `player_id, player_name, location_name, tech_name`
→ **`AreaAction::Laboratory{card_id, …}`** plus, when the tech is Station, **`PendingChoice::StationPlace`**. The **socket form carries `tech_name` AND `tech_card.tech_id` together**, so one socket notification is a complete `tech_id`↔name observation; the history form carries the name only. **This corpus yields exactly ONE such pair — `tech_id "2"` = `"Mining Technology"` (`req_gears 3`, `country 2`)** — because only one of the game's four tech acquisitions fell inside the socket window; the other three (`University`, `Ball`, `Exposition`) are name-only. The BGA→our tech mapping table (H1) therefore has **1 of 30 rows** available here. See U10.

**`churchPlacement`** — raw 20 / uniq 8 / **surv 8** (two distinct events share the type)
shape A: `{added_faith, res_list, church_slot, counters{}}` — the goods payment
shape B: `{vp_count, add_vp, move_count, church_slot, tile_key, workers{}, counters{}}` — the placement
history: `{added_faith, res_list}` / `{vp_count, move_count}`
→ **`AreaAction::Church{copper,silk,tea,seafood,yen,imported_good}`** + **`PendingChoice::ChurchSpace{total_faith}`** → `Move::ChooseChurchSpace{space}` (= `church_slot`). `res_list` is a rendered string; the per-good booleans must be parsed out of it — **fragile, flag as a hard-error site**.

**`customsPlacement`** — raw 6 / uniq 2 / **surv 2**
socket args: `player_id, player_name, import_count, import_slot, tile_key, gain_list, workers{}, counters{}` · history: `player_id, player_name, import_count, gain_list`
→ **`AreaAction::Customs{imported_goods_paid}`** + **`Move::ChooseCustomsSpace{space}`** (= `import_slot`).

**`buildShop`** — raw 7 / uniq 3 / **surv 3**
socket args: `player_id, player_name, tile_name, tile_key, card_slot, result_info{gain_text[], next_state, …}, workers{}, counters{}` · history: `player_id, player_name, tile_name`
→ **`Move::BuildShophouse{slot}`** (= `card_slot`). **`buildHouse` (→ `Move::BuildTradingHouse`) is declared but UNOBSERVED — args UNKNOWN.**

**`warehousePull`** — raw 5 / uniq 2 / **surv 2**
socket args: `player_id, player_name, assist_count, shop_count, house_count, workers{}, counters{}` · history: `player_id, player_name` (counts only in the `log` string)
→ **`AreaAction::EmploymentAgency{assistants, shophouses, trading_houses}`**, or **`PendingChoice::WarehouseTake` → `Move::ChooseWarehouseTake{item}`** when it is the icon-driven take. Which of the two is **not stated by the notification** — it must come from the surrounding `gameStateChange` (state 40 `warehouseOut`), i.e. **socket-only**.

**`pullAssistant`** — raw 7 / uniq 3 / **surv 3** — history-only in this corpus
args (both feeds): `player_id, player_name` — everything else is in the log: `<B>University</B>:${player_name} pulls 1 assistant from his warehouse for 1 yen`
→ **`Move::UseUniversity`**. The `<B>University</B>:` prefix in the log is the only discriminator. No socket sample exists (all 3 predate the socket window) — the socket arg shape is **UNKNOWN**.

**`returnPresident`** — raw 3 / uniq 1 / **surv 1**
socket args: `player_id, player_name, tile_key, workers{}, counters{}` · history: `player_id, player_name`
→ **`Move::ReturnPresidentToHand{recover}`**. The `recover` map is **not** in this notification; it must be derived from the `workers{}` diff, or from the adjacent `assistantRecovery`/`removeNoAssistants`.

**`removeNoAssistants`** — raw 3 / uniq 1 / **surv 1**
args: `player_id, player_name, assist_count` (observed `0`)
→ the empty case of `ReturnPresidentToHand{recover: {}}`. **Not the declared `removeAssistants` handler** — see §7.1.

**`powerBonus`** — raw 13 / uniq 6 / **surv 6**
socket args: `player_id, player_name, res_count, res_name, tile{full tile record}, counters{}` · history: `player_id, player_name, res_count, res_name`
→ **`Move::Take5PowerToken`**. The socket window shows **20 entries into state 341 `powerBonus` but only 6 distinct `powerBonus` notifications**, which is consistent with `Skip5PowerBonus` producing **no notification at all** — i.e. absence is the signal. Stated as an **inference from that count gap, not a verified rule**; H2 should treat a state-341 pass with no `powerBonus` notification as a skip and hard-error if the counters disagree.

**`orderDiscarded`** — raw 4 / uniq 2 / **surv 2** — history-only, **CARRIES A REVEAL**, see §8
args: `player_id, player_name, order{full card, location "5"}`
log: `${player_name} selects the initial order card`
→ the setup starting-order choice. **The log text is misleading: the card in `args.order` is the DISCARDED one, not the kept one.** Proof: Player_A's `orderDiscarded` names `order_key 1 / order_id 24`; the card he actually held all game is `order_key 2 / order_id 27`. Type name right, log text wrong. → feeds `StartingOrderChoice` and the boxed pile.

**`achievementClaimed`** — raw 4 / uniq 2 / **surv 2** — history-only
args: `player_id, player_name, achievement_class ("A"), vp_count`
→ **`Move::FulfillAchievement{achievement}`**. `achievement_class` is a letter; the mapping to our `AchievementType` is **UNKNOWN** (the numeric `achievement_id`/`achievement_key` live in `gamedatas.achievements`, not in the notification).

**`payForPres`** — raw 4 / uniq 2 / **surv 2** — history-only
args: `player_id, player_name, pname (HTML-coloured span), amount, tile_name`
→ placement/pass-through **fee payment**, an *effect* of `PlaceThreeAreas`/`PlaceTwoSameArea`/`MovePresident`, not a move. Reconciliation input. `pname` is HTML — strip before use.

**`orderRefused`** — raw 3 / uniq 1 / **surv 1**
args: `player_id, player_name, order_no ("second"), counters{}` (history drops `counters`)
→ declining the second Port card, i.e. `cards_taken` of length 1 / `AreaAction::Skip`.

**`techRefusal`** — raw 3 / uniq 1 / **surv 1**
args: `player_id, player_name` — → declining the Laboratory action (`AreaAction::Skip`).

**`endGame` / `lastRound` / `finalScoring`** — raw 3 / uniq 1 / **surv 1** each
socket args: `player_id, player_name, counters{}` · history args: `player_id`
The *content* is in the log: `"End of Game triggered: Max assistants reached on Church Management Board"`, `"Last round before final scoring"`, `"Final scoring"`.
→ terminal framing. **`endGame`'s trigger reason exists only as English prose** — parse defensively or ignore and let the engine derive its own trigger.

**`simpleNode`** — raw 3 / uniq 1 / **surv 1** — args `[]`, log `"End of game"` → chrome.

### 7.3 Framework / chrome types — no engine effect

| type | raw / uniq / surv | args | disposition |
|---|---|---|---|
| `gameStateChange` | 630 / 315 / 296 | §6 | **decision framing** — not a move, but the only phase signal. Socket-only. |
| `yourturnack` | 100 / 50 / 50 | `{player}` | no-op |
| `undoRestorePoint` | 23 / 9 / 9 | `{player_name}` | **supersession marker** — §4. No state effect of its own. |
| `updateReflexionTime` | 38 / 19 / 19 | `{player_id, delta, max}` | no-op (clocks) |
| `wakeupPlayers` | 10 / 5 / 5 | `[]` | no-op |
| `chat` | 8 / 8 / 8 | `{text, player_name, player_id, is_admin}` | no-op — **`/chat/general`, unrelated players' table chat** |
| `playerstatus` | 8 / 4 / 4 | `{player_name, player_id, player_status, player_playing}` | no-op — **`/player/…` channel, names players not in this game** |
| `tableWindow` | 2 / 1 / 1 | `{id:"finalScoring", title, table[][], closing}` | **the final-score cross-check input** — §9 |
| `resultsAvailable` | 2 / 1 / 1 | `[]` | no-op |
| `tableInfosChanged` | 2 / 1 / 1 | 60+ lobby fields | no-op (build provenance only) |
| `updatePlayerNotifCount` | 2 / 1 / 1 | `{inc, splash[]}` | no-op (site trophies) |
| `updatePlayerTableStatus` | 2 / 1 / 1 | `{table_id, status, async, …}` | no-op |

### 7.4 Declared-but-unobserved → args UNKNOWN

`stationPlacement` · `buildHouse` · `removeAssistants`. **The bridge must hard-error on first sight of any of these**, not attempt a shape. `stationPlacement` is the highest-value gap: it is our `PendingChoice::StationPlace`, a choice with **no Decline**, so a bridged game containing a Station acquisition is untranslatable until its shape is seen.

### 7.5 The `counters` object (the reconciliation surface)

Present on 232 socket notifications, **never** in history. Keyed by `player_id` plus a literal `"eog_triggers"` key.

Per player: `player_id, tea, fish, silk, copper, yens, imports, gears, power_tokens, fa_tokens, fa_used, orders_done, orders_hand, prod_count, comm_count, tech_count, flags_gb, flags_us, flags_fr, flags_ge, flags_ne, vps, wh_assists, wh_shops, wh_houses, hand_president, hand_assists, hand_shops, hand_houses, board_president, board_assists, board_shops, board_houses`.

`eog_triggers`: `deck_count, church_count, church_max, customs_count, customs_max, shops_count, shops_max, houses_count, houses_max, church_assist{}, customs_assist{}, shops_player{}, houses_player{}`.

This is design §H2's "cross-checked against BGA `counters` at checkpoints" — and it is **socket-only**, so checkpoint cross-checks are unavailable across the thin half except at `gamedatas` snapshots.

---

## 8. Hidden information — which types carry reveals, and one hard fence

### 8.1 The reveal-carrying types

Our live model's reveals (`reveal.rs`) are: **port refill** (`Pending::OrderReveals`, one `OrderCardId` per refilled slot, draw order) and **lab refill** (`Pending::TechReveal`, exactly one). The BGA equivalents:

| our reveal | BGA source | socket | history |
|---|---|---|---|
| port refill | **`orderRefresh`** `.args.orders` (new card at `location "2"`) and `.args.order_card` | full identity | **name of the board only — no card** |
| lab refill | **`techAcquisition`** `.args.techs` (the slid/refilled Lab set) | full identity by `tech_id`, plus `tech_name` for the acquired card | tech NAME of the *acquired* card only |
| card taken | **`orderAcquisition`** `.args.order_card` | full identity | literal `"an order"` |
| starting-order boxing | **`orderDiscarded`** `.args.order` | *(history-only)* | **full identity of the DISCARDED card, both seats** |
| order made public on completion | **`orderCompletion`** `.args.order` | full identity | **full identity** |

`orderDiscarded` is a *gift* to our model: BGA publishes both players' **discarded** starting orders, which narrows `orders_removed` beyond what our live wizard normally learns. It does **not** reveal the kept card.

### 8.2 Private-info check (design §6) — PASSES, with the mechanism stated

The design asked whether `privateinc=1` carries the owner's own hand events while the opponent's stay hidden. **It does, and the mechanism is not the query parameter — it is Yokohama's own rules.**

- **The opponent's kept starting order stays hidden.** `order_key 4` (Player_B's) appears **nowhere** in the corpus until packet 18, and then only via `orderCompletion` — i.e. at the moment the rules make it public. Before that it is absent from every `orders` map in every notification and from both mid-game `gamedatas` snapshots.
- **Orders acquired from the Port are public by the rules**, and BGA publishes them accordingly: the opponent's freshly-taken, uncompleted `order_key 10` is fully visible at packet 119, well before `endGame` at packet 198. This is **not** a leak — Port cards are taken face-up.
- Mid-game snapshots show the opponent at `orders_hand: 0` in both, so the snapshot path is not independent evidence; the socket-feed evidence above is.

**Conclusion: BGA's visibility model matches our live-tracking hidden-information model** — one secret per opponent (the kept starting order), revealed on completion.

### 8.3 THE FENCE — the undrawn deck's identities are in the payload

**This is the most important honesty finding in this document.**

`orders` and `techs` maps in socket notifications, and in `gamedatas`, use a `location` code. Measured meanings: **`1` = the undrawn deck**, `2` = the board display (Port / Lab), `4` = held or completed by a player (with `player_id` set), `5` = discarded at setup.

`location "1"` rows carry **full identity** — `order_id`, `req_tea/fish/silk/copper`, `bonus_vp/import/yen/wh/move` — for cards that have **not been dealt yet**:

- `eog_triggers.deck_count` equals the `location "1"` row count in **all 22 samples** where both are present, with zero mismatches (6/6, 5/5, 4/4, 3/3, 1/1, …). Location 1 *is* the deck as BGA counts it.
- **`order_key 19` is present at `location "1"` in packet 115 and only enters the display at packet 184** — 69 packets and ~35 moves of advance knowledge. `order_key 18` likewise (115 → 184), `order_key 17` (115 → 155), `order_key 16` (115 → 138).
- The same holds for techs: 20–23 rows sit at `location "1"` with `tech_id` and `req_gears` throughout.
- The set is exposed, not the order (`position` is `"0"` for every deck row).

> **Fence for H1/H2, non-negotiable:** the importer must consume `location "1"` rows as a **count only, never as identity**. A `GameRecord` whose deck contents were seeded from this data is not a live-mode record and would be a privileged-play artifact in the corpus — precisely the failure the arena's no-fallback rule exists to prevent (`docs/plans/2026-08-07-bga-bridge-design.md` §2.3 output clause; map §1.5/§1.6 comparison-only fence). Identity may be taken **only at the moment a card transitions to `location "2"` or `"4"`**, which is exactly a reveal in our model.
>
> The honest way to enforce this is structurally: parse deck rows into a *count* type that cannot carry a card id, so the leak is unrepresentable rather than merely unused.

### 8.4 The card-identification key is confirmed available

Every card object — in `orderDiscarded`, `orderAcquisition`, `orderRefresh`, `orderCompletion`, and the `orders` maps — carries the full requirement+reward vector (`req_tea, req_fish, req_silk, req_copper, bonus_vp, bonus_import, bonus_yen, bonus_wh, bonus_move`). Design §1's identification mechanism (match on the unique vector, never on BGA's number) has its input. `country` is also present (values 1–5) and is a second, independent check.

Tech cards carry only `tech_id`, `country`, `req_gears` on the socket and only `tech_name` in history — **so the tech mapping table cannot be built from the socket alone**; it needs the `uid`-join across the two feeds described in §7.2, or a name↔id source outside this corpus. This corpus supports the join for **4 techs only** (the ones acquired). The table is **incomplete by construction here** — H1's problem.

---

## 9. Two cross-checks this corpus already supplies

**Final score.** `tableWindow` (`id: "finalScoring"`) carries BGA's own scoring table verbatim:

```
                    Player_A  Player_B  Dummy player
In game VP             115        33
Flag sets               12         4
Yens                     0         0
Imports                  0         0
Agents                   1         0
Goods                    2         1
Church majority          6         3        0
Customs majority         8         0        4
Gears majority          10         0        5
Total                  154        41        9
```

This is the input to design §2.3's honesty dividend — the engine-vs-BGA final-score differential.

**And it independently corroborates finding D1.** The **Dummy player scores in the church, customs and gears majorities** (0/4/5) on an independent implementation — the rulebook p.9 NOTE behaviour we modelled on 2026-07-21 from Jeremy's edition, which the English PDF in `analysis-1/` does not carry. Recorded as corroboration, not as a new ruling.

**Board topology.** `gamedatas.tiles` gives each tile `up_left, up_right, side_left, side_right, down_left, down_right` — a **six-neighbour staggered pyramid**, 10 tiles at 2P in rows 4/3/2/1 (`tile_key` 1–4 / 5–7 / 8–9 / 10). This matches `board_gen`'s corrected topology (SEMANTICS_REV 2) exactly and is a direct import path for `ObservedBoard`.

Observed `tile_key` → `tile_id` → name for **this table only** (area assignment is per-game):

| `tile_key` | `tile_id` | area |
|---|---|---|
| 1 | 1 | Tea Plantation |
| 2 | 9 | Port |
| 3 | 6 | Employment Agency |
| 4 | 2 | Bank |
| 5 | 4 | Fishing Ground |
| 6 | 7 | Silk Mill |
| 7 | 5 | Church |
| 8 | 3 | Customs |
| 9 | 8 | Laboratory |
| 10 | 10 | Copper Mine |

Derived by joining `presidentPlacement.trg_tile_key`↔`tile_name`, `assistantPlacement.tile_keys`↔`tile_names` (positional), and `moveAssistant.from_id`↔`from_loc`. **`tile_key` is a board POSITION; `tile_id` is the area's identity.** The full `tile_id` → our `AreaId` table needs the seven area types a 2P board does not use — **UNKNOWN from this corpus**. `gamedatas.mngt_boards` is `[1,2,3,4,3,4]`; its meaning is **UNKNOWN**.

`workers`: `worker_type` observed `1` (2 in play — President), `2` (assistants), `3` (buildings). `location` observed `3` (on a tile, `loc_key` = `tile_key`), `5` and `6` (management boards, `loc_key` `0`, `card_pos` used as the slot). The `location` code table for workers is **partially UNKNOWN**.

---

## 10. UNKNOWN register — every open item, in one place

Nothing below may be guessed. Each is either a live-probe task or a hard-error site.

| # | Unknown | Why it matters |
|---|---|---|
| U1 | `stationPlacement` args | `PendingChoice::StationPlace` has no Decline; a Station acquisition is untranslatable |
| U2 | `buildHouse` args | `Move::BuildTradingHouse` untranslatable |
| U3 | `removeAssistants` args (declared, never fired; distinct from `removeNoAssistants`) | the non-empty `ReturnPresidentToHand{recover}` case |
| U4 | 13 unobserved `gameStateChange` states (21, 22, 32, 37, 38, 42, 63, 82, 301, 349, 2, 11, 1) | includes Telegram adjacent placement, both fee-confirm states, Chinatown, `AnyGood`, Telephone leave-one, president-from-hand |
| U5 | `presidentPlacement.action` value set (only `"places"` observed) | discriminates `MovePresident` from `PlacePresident` |
| U6 | Telegram `adjacent_bonus` encoding in `assistantPlacement` | `PlaceTwoSameArea{adjacent_bonus}` |
| U7 | `achievement_class` letter → our `AchievementType` | `Move::FulfillAchievement` |
| U8 | `res_list` string grammar (`churchPlacement`) | the six Church payment booleans |
| U9 | Port second-card payment kind (yen vs import) from notifications | `PortPayment` |
| U10 | Full `tech_id` → `TechCardId` table — **1 of 30 rows** derivable here (`tech_id "2"` = `"Mining Technology"`); 3 more techs are name-only, 26 never named | H1's checked mapper |
| U11 | Full `tile_id` → `AreaId` table (10 of ~17 seen) | 3P/4P boards |
| U12 | `worker` `location` code table beyond `{3,5,6}` | pawn reconciliation |
| U13 | `mngt_boards` meaning | setup import |
| U14 | `h` and `lock_uuid` semantics | inert today; must not become load-bearing |
| U15 | Whether any `notificationHistory` parameterisation returns socket-shaped args | would collapse the §5 catch-up problem |
| U16 | 3P/4P shapes; a non-owner seat's view; a different BGA build | every count here is 2P/one-seat/one-build |
| U17 | Behaviour of a takeback that spans a `gameStateChange` into a *different* active player | all 5 observed undos were same-player, same-turn |

---

## 11. What H2 should take from this, stated as constraints not decisions

1. **Dedupe socket packets on `packet_id`; dedupe notifications on `uid` within a transport; never on `move_id`; never on any timestamp.** (§3)
2. **Detect supersession structurally: `prevpacket != [packet_id − 1]`.** The `undoRestorePoint` type and the `move_id` regression are corroborating, not sufficient. (§4.1)
3. **Collapse is retroactive on the live feed and already done on the history feed.** Re-derivation from a checkpoint is the honest unwind; `GameState` is history-free by design. (§4.3)
4. **Treat the two feeds as two different data sources, not two deliveries of one.** Same `uid`, different args, 110 of 117 times. Prefer the socket copy wherever both exist; hard-error rather than silently accepting a thin copy where a key is required. (§5)
5. **Never import `location "1"` card identities.** Make it structurally unrepresentable. (§8.3)
6. **Every UNKNOWN in §10 is a hard error on first sight**, with the offending packet dumped verbatim — the bridge's standing posture.
7. **If translating the thin half proves to require guessing, do not guess** — the answer is to capture the next game from move 1, which the machinery now does.

---

*Measured 2026-08-08 from `bridge-captures/895724349.ndjson` (491 lines, 1,897,569 bytes). No engine, oracle, difftest, or CLI source was read for behaviour and none was modified.*
