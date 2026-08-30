//! Notification → typed FACT extraction (H2, plan T8's front half).
//!
//! A [`Fact`] is what one BGA notification determines on its own — the
//! translator consumes the fact STREAM against the engine's legal list, so
//! this module owns the wire vocabulary (arg fields and the PINNED log
//! templates) and knows nothing about game state.
//!
//! **Log parsing is template-pinned (plan T7's evidence ladder):** the
//! history feed's `log` field is BGA's raw template string (placeholders
//! unsubstituted except literal amounts), so a template is matched EXACTLY or
//! by a stated prefix/suffix rule; a log that matches no pinned template is a
//! typed [`ImportError`] naming the string — never a regex guess (design §6).
//!
//! The socket form is preferred wherever a move carries both (inventory §5:
//! same uid, richer args, never the reverse).

use super::names::area_type_from_tile_name;
use super::orders::OrderMapper;
use super::{require_u64, require_u8, BgaMapError, ImportError};
use serde_json::Value;
use std::collections::BTreeMap;
use yokohama_core::{AreaType, FivePowerEffect, OrderCardId, SpaceReward};

/// Payment kind for the second Port card, as the log names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondPayment {
    TwoYen,
    OneImport,
}

/// Church payment booleans, parsed from `res_list`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChurchPays {
    pub copper: bool,
    pub silk: bool,
    pub tea: bool,
    pub seafood: bool,
    pub yen: bool,
    pub imported_good: bool,
    /// The yen AMOUNT the log named (census §4.1: BGA charged 2 where the
    /// engine charges 1 — recorded so the divergence is reportable).
    pub yen_amount: u8,
}

/// How BGA's `presidentPlacement` said the President reached the tile — the
/// `action` arg's observed vocabulary, and the ONLY thing the translator needs
/// beyond the destination to price the move against the engine's legal list.
///
/// The full measured register (which values exist, on which moves, and what
/// each one's args carry) lives on the `presidentPlacement` arm in [`extract`];
/// read it there before adding a variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresidentVia {
    /// `action: "places"` — the ordinary verb: a placement from hand, or a
    /// walk along the mover's own assistant chain, priced by its fees.
    Placed,
    /// `action: "trams"` — the Tram technology's Movement-step move ("move
    /// your President to any area", `tech_tram`), which is FREE by card text.
    Tram,
}

/// One notification's determination. `seat` is the engine seat (0-based).
#[derive(Debug, Clone, PartialEq)]
pub enum Fact {
    // ---- drive facts (each demands engine moves) ---------------------------
    Placement { seat: u8, option_b: bool, areas: Vec<AreaType> },
    President { seat: u8, dest: AreaType, power: u8, via: PresidentVia },
    ReturnPresident { seat: u8 },
    PortTake { seat: u8, card: Option<OrderCardId>, second: Option<SecondPayment> },
    PortSecondDeclined { seat: u8 },
    LabTake { seat: u8, tech_name: String, tech_id_key: Option<(u64, u64)> },
    LabDeclined { seat: u8 },
    OrderCompleted { seat: u8, card: OrderCardId },
    AchievementClaimed { seat: u8, category: usize, vp: u8 },
    BuildShop { seat: u8, tile: AreaType, reward: SpaceReward },
    PowerToken { seat: u8, face: FivePowerEffect },
    ChurchPay { seat: u8, faith_added: u8, pays: ChurchPays },
    ChurchSpace { seat: u8, vp: u8, moves: u8 },
    Customs { seat: u8, count: u8, vp: u8 },
    WarehousePull { seat: u8, assistants: u8, shophouses: u8, trading_houses: u8 },
    University { seat: u8 },
    Mercenary { seat: u8, tile: AreaType, power: u8 },
    /// `None` = the player's hand. A `to: None` fact is the census §4.2 site.
    AssistMove { seat: u8, from: Option<AreaType>, to: Option<AreaType> },

    // ---- note facts (corroboration; consumed freely) -----------------------
    Recovery { seat: u8, count: u8 },
    RecoverNone { seat: u8 },
    PassFee { seat: u8 },
    PlaceFee { seat: u8, amount: u8 },
    Gain { seat: u8, text: String },
    /// BGA granted the Port's order-row entitlement. `vp_bonus` is the VP booked
    /// WITH it — 0 below 5 Power, 3 at 5 Power, where BGA books the bonus at the
    /// president-placement move while the engine books it inside the atomic Port
    /// action. A note fact either way; the bonus is typed because the translator
    /// needs the MOVE it landed on to skip that boundary by granularity.
    PortAccess { seat: u8, vp_bonus: u8 },
    PortRefill,
    Discard { seat: u8 },
    EndGameMarker,
    LastRound,
    FinalScoringMarker,
}

impl Fact {
    /// Drive facts demand an engine move; note facts are corroboration.
    pub fn is_drive(&self) -> bool {
        !matches!(
            self,
            Fact::Recovery { .. }
                | Fact::RecoverNone { .. }
                | Fact::PassFee { .. }
                | Fact::PlaceFee { .. }
                | Fact::Gain { .. }
                | Fact::PortAccess { .. }
                | Fact::PortRefill
                | Fact::Discard { .. }
                | Fact::EndGameMarker
                | Fact::LastRound
                | Fact::FinalScoringMarker
        )
    }

    pub fn seat(&self) -> Option<u8> {
        match self {
            Fact::Placement { seat, .. }
            | Fact::President { seat, .. }
            | Fact::ReturnPresident { seat }
            | Fact::PortTake { seat, .. }
            | Fact::PortSecondDeclined { seat }
            | Fact::LabTake { seat, .. }
            | Fact::LabDeclined { seat }
            | Fact::OrderCompleted { seat, .. }
            | Fact::AchievementClaimed { seat, .. }
            | Fact::BuildShop { seat, .. }
            | Fact::PowerToken { seat, .. }
            | Fact::ChurchPay { seat, .. }
            | Fact::ChurchSpace { seat, .. }
            | Fact::Customs { seat, .. }
            | Fact::WarehousePull { seat, .. }
            | Fact::University { seat }
            | Fact::Mercenary { seat, .. }
            | Fact::AssistMove { seat, .. }
            | Fact::Recovery { seat, .. }
            | Fact::RecoverNone { seat }
            | Fact::PassFee { seat }
            | Fact::PlaceFee { seat, .. }
            | Fact::Gain { seat, .. }
            | Fact::PortAccess { seat, .. }
            | Fact::Discard { seat } => Some(*seat),
            _ => None,
        }
    }
}

/// One fact with its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedFact {
    pub move_id: u64,
    pub kind: String,
    pub fact: Fact,
}

/// A BGA `counters` checkpoint observed at the end of a move (socket only).
#[derive(Debug, Clone, PartialEq)]
pub struct CounterCheckpoint {
    pub move_id: u64,
    /// player_id string → the raw counters object.
    pub per_player: BTreeMap<String, Value>,
}

/// The extracted stream.
#[derive(Debug, Default)]
pub struct FactStream {
    pub facts: Vec<TimedFact>,
    pub checkpoints: Vec<CounterCheckpoint>,
}

fn err_template(move_id: u64, kind: &str, log: &str) -> ImportError {
    ImportError::Sheet {
        detail: format!(
            "move {move_id}: {kind} log matches no pinned template (never guessed): {log:?}"
        ),
    }
}

fn strip_bold(s: &str) -> String {
    s.replace("<B>", "").replace("</B>", "")
}

fn area(name: &str) -> Result<AreaType, BgaMapError> {
    area_type_from_tile_name(name.trim())
}

/// Parse one reward item like `2 move(s)` / `2 tea` / `3 VP(s)` into a
/// [`SpaceReward`] increment. Pinned item vocabulary; unknown = error.
fn add_reward_item(out: &mut SpaceReward, item: &str, ctx: &str) -> Result<(), ImportError> {
    let item = strip_bold(item);
    let item = item.trim();
    let (num, rest) = item.split_once(' ').ok_or_else(|| ImportError::Sheet {
        detail: format!("{ctx}: reward item {item:?} is not `<n> <what>`"),
    })?;
    let n: u8 = num.parse().map_err(|_| ImportError::Sheet {
        detail: format!("{ctx}: reward count {num:?} is not a number"),
    })?;
    match rest.trim() {
        "move(s)" | "move" | "moves" => out.movement += n,
        "tea" => out.tea += n,
        "fish" => out.seafood += n,
        "silk" => out.silk += n,
        "copper" => out.copper += n,
        "yen" | "yen(s)" | "yens" => out.yen += n,
        "VP" | "VP(s)" => out.vp += n,
        other => {
            return Err(ImportError::Sheet {
                detail: format!("{ctx}: reward item kind {other:?} is outside the pinned vocabulary"),
            })
        }
    }
    Ok(())
}

/// `powerBonus.res_name` → token face, with its count.
fn token_face(res_name: &str, count: u8, ctx: &str) -> Result<FivePowerEffect, ImportError> {
    Ok(match res_name {
        "Fish" => FivePowerEffect::Seafood(count),
        "Tea" => FivePowerEffect::Tea(count),
        "Silk" => FivePowerEffect::Silk(count),
        "Copper" => FivePowerEffect::Copper(count),
        "Import(s)" => FivePowerEffect::ImportedGoods(count),
        "Yen(s)" | "Yens" => FivePowerEffect::Money(count),
        other => {
            return Err(ImportError::Sheet {
                detail: format!("{ctx}: powerBonus res_name {other:?} is outside the pinned vocabulary"),
            })
        }
    })
}

/// `res_list` item → church payment flag. Pinned: `1 tea`, `1 fish`,
/// `1 silk`, `1 copper`, `1 import`, `N yens` (the yen amount is recorded).
fn church_res_item(pays: &mut ChurchPays, item: &str, ctx: &str) -> Result<(), ImportError> {
    let item = item.trim();
    let (num, rest) = item.split_once(' ').ok_or_else(|| ImportError::Sheet {
        detail: format!("{ctx}: res_list item {item:?} is not `<n> <what>`"),
    })?;
    let n: u8 = num.parse().map_err(|_| ImportError::Sheet {
        detail: format!("{ctx}: res_list count {num:?} is not a number"),
    })?;
    match rest {
        "tea" => pays.tea = true,
        "fish" => pays.seafood = true,
        "silk" => pays.silk = true,
        "copper" => pays.copper = true,
        "import" | "import(s)" => pays.imported_good = true,
        "yen" | "yens" => {
            pays.yen = true;
            pays.yen_amount = n;
        }
        other => {
            return Err(ImportError::Sheet {
                detail: format!("{ctx}: res_list item kind {other:?} is outside the pinned vocabulary"),
            })
        }
    }
    Ok(())
}

/// Extract the fact stream from a collapsed feed. `seat_of` maps a BGA
/// player_id string to an engine seat.
pub fn extract(
    feed: &yokohama_bridge::feed::CollapsedFeed,
    seat_of: &dyn Fn(&str) -> Option<u8>,
    _mapper: &OrderMapper,
) -> Result<FactStream, ImportError> {
    let mut out = FactStream::default();
    let mapper = _mapper;
    for mv in &feed.moves {
        // Prefer the socket form (arg-rich); the history spine otherwise.
        let pk = mv.socket.as_ref().or(mv.history.as_ref());
        let Some(pk) = pk else { continue };
        let mid = mv.move_id;
        let mut last_counters: Option<BTreeMap<String, Value>> = None;
        for n in &pk.notifications {
            // Every socket notification may carry a counters object; keep the last.
            if let Some(c) = n.args.get("counters").and_then(|c| c.as_object()) {
                let mut per_player = BTreeMap::new();
                for (k, v) in c {
                    if k != "eog_triggers" {
                        per_player.insert(k.clone(), v.clone());
                    }
                }
                last_counters = Some(per_player);
            }
            let log = n.log.as_deref().unwrap_or("");
            let args = &n.args;
            let seat = |ctx: &str| -> Result<u8, ImportError> {
                let pid = args.get("player_id").and_then(|p| match p {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(x) => Some(x.to_string()),
                    _ => None,
                });
                pid.as_deref().and_then(seat_of).ok_or_else(|| ImportError::Sheet {
                    detail: format!("move {mid}: {ctx} names no known player_id"),
                })
            };
            let mut push = |fact: Fact| {
                out.facts.push(TimedFact { move_id: mid, kind: n.kind.clone(), fact });
            };
            match n.kind.as_str() {
                // ---- chrome (inventory §7.3) -------------------------------
                // gameStateMultipleActiveUpdate: the framework's multi-active
                // state update from the SIMULTANEOUS opening placement —
                // measured live at the gate game (table 896153951, move 1);
                // the proving corpus's socket joined mid-game and never saw it.
                "gameStateChange" | "yourturnack" | "updateReflexionTime" | "wakeupPlayers"
                | "tableWindow" | "resultsAvailable" | "tableInfosChanged"
                | "updatePlayerNotifCount" | "updatePlayerTableStatus" | "chat"
                | "playerstatus" | "undoRestorePoint" | "simpleNode"
                // simpleNote: BGA's standard framework notice, measured live at
                // the gate game (table 896157897, move 1) — the proving corpus
                // never carried one. (The pre-existing "simpleNode" entry above
                // is retained but suspect: never observed on any wire; possibly
                // an inventory transcription of this very kind.)
                | "simpleNote"
                | "gameStateMultipleActiveUpdate" => {}

                "orderDiscarded" => push(Fact::Discard { seat: seat("orderDiscarded")? }),

                "assistantPlacement" => {
                    let s = seat("assistantPlacement")?;
                    if log.contains("places 2 assistants") {
                        let name = args.get("tile_name").and_then(|t| t.as_str()).ok_or_else(|| {
                            ImportError::Sheet { detail: format!("move {mid}: 2-on-1 placement without tile_name") }
                        })?;
                        push(Fact::Placement {
                            seat: s,
                            option_b: true,
                            areas: vec![area(&strip_bold(name)).map_err(ImportError::Map)?],
                        });
                    } else if log.contains("places 1 assistant") {
                        let names = args.get("tile_names").and_then(|t| t.as_str()).ok_or_else(|| {
                            ImportError::Sheet { detail: format!("move {mid}: option-A placement without tile_names") }
                        })?;
                        let mut areas = Vec::new();
                        for part in strip_bold(names).split(',') {
                            areas.push(area(part).map_err(ImportError::Map)?);
                        }
                        push(Fact::Placement { seat: s, option_b: false, areas });
                    } else {
                        return Err(err_template(mid, "assistantPlacement", log));
                    }
                }

                // ---- presidentPlacement: THE REGISTER-U5 OBSERVED SET -------
                //
                // The `action` arg is a VERB substituted into one shared log
                // template (`${player_name} ${action} the president on the
                // <B>${tile_name}</B> …`), which is why this arm dispatches on
                // the arg and never on the log string.
                //
                // MEASURED VOCABULARY (2026-08-12, W2 Step 0): exactly
                // `"places"` and `"trams"`, over every banked capture. Async
                // table 896199544 — the ONLY one that carries a Tram — holds 84
                // `places` and 8 `trams` notification instances; the other two
                // captures hold `places` alone.
                //
                // `"trams"` is the Tram technology's Movement-step move. Its
                // provenance in the capture is unambiguous: the Tram card is
                // taken at m118 by ONE seat, and every `trams` notification in
                // the game is that seat's, after that move.
                //
                // THE EIGHT INSTANCES, all of them (move × transport):
                //
                //   m127  socket ×2   tile 3 → 1 (Church)     route_keys []
                //   m128  socket ×2   tile 3 → 2 (Silk Mill)  route_keys ["2"]
                //         history ×1  arg-thin: action/player_id/tile_name/power
                //   m137  socket ×2   tile 2 → 1 (Church)     route_keys ["1"]
                //         history ×1  arg-thin, as above
                //
                // **m127 never reaches this extractor.** It is a SUPERSEDED
                // takeback — the player trammed to the Church, saw the faith,
                // undid it and re-moved — so BGA's history spine keeps only the
                // surviving line and `feed`'s structural supersession rule drops
                // the rewound socket packets. TWO tram facts survive, m128 and
                // m137, and on both of them `route_keys == [trg_tile_key]`.
                //
                // **`route_keys` is recorded here as evidence and deliberately
                // NOT read** — the posture the `places` arm has always had. It
                // is not a trustworthy path: at m95 a `places` move walked
                // Copper Mine → Employment Agency, two steps apart on BGA's own
                // tile links, with `route_keys: []`. What determines the engine
                // move is `tile_name` (the destination) plus the `via`
                // discriminator; `translate.rs` prices the rest against the
                // engine's own legal list.
                //
                // Anything outside `places`/`trams` stays a hard error naming
                // what was seen: this widen covers exactly the measured set.
                "presidentPlacement" => {
                    let s = seat("presidentPlacement")?;
                    let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("<absent>");
                    let via = match action {
                        "places" => PresidentVia::Placed,
                        "trams" => PresidentVia::Tram,
                        other => {
                            return Err(ImportError::Sheet {
                                detail: format!(
                                    "move {mid}: presidentPlacement action {other:?} is outside the \
                                     observed set (register U5: \"places\" | \"trams\") — hard \
                                     error, never guessed"
                                ),
                            })
                        }
                    };
                    let name = args.get("tile_name").and_then(|t| t.as_str()).ok_or_else(|| {
                        ImportError::Sheet { detail: format!("move {mid}: presidentPlacement without tile_name") }
                    })?;
                    let power = require_u8(args, "power", "presidentPlacement").map_err(ImportError::Map)?;
                    push(Fact::President {
                        seat: s,
                        dest: area(&strip_bold(name)).map_err(ImportError::Map)?,
                        power,
                        via,
                    });
                }

                "returnPresident" => push(Fact::ReturnPresident { seat: seat("returnPresident")? }),
                "removeNoAssistants" => push(Fact::RecoverNone { seat: seat("removeNoAssistants")? }),

                "resourceChange" => {
                    let s = seat("resourceChange")?;
                    if log.contains("uses a foreign agent to activate tile") {
                        let name = args.get("tile_name").and_then(|t| t.as_str()).ok_or_else(|| {
                            ImportError::Sheet { detail: format!("move {mid}: mercenary use without tile_name") }
                        })?;
                        let power = require_u8(args, "tile_power", "mercenary resourceChange")
                            .map_err(ImportError::Map)?;
                        push(Fact::Mercenary {
                            seat: s,
                            tile: area(&strip_bold(name)).map_err(ImportError::Map)?,
                            power,
                        });
                    } else if log.contains("for passing through their President") {
                        push(Fact::PassFee { seat: s });
                    } else if log.contains("gains: access to") {
                        push(Fact::PortAccess { seat: s, vp_bonus: 0 });
                    } else if log.contains("VPs,access to") {
                        // The 5-Power Port bonus, measured live at async table
                        // 896199544 m105 — the first banked capture to run a
                        // Port action at 5 Power. BGA books the +3 HERE, at the
                        // president placement; the engine books it inside the
                        // atomic Port action, which requires an actual take. The
                        // totals agree, the instants do not, and `translate`
                        // skips the intervening boundary by granularity.
                        //
                        // The "3" is literal wire text, on the Stock Market
                        // precedent below: a different amount is a different
                        // template and stays a hard error until measured. That
                        // promise is why the SHAPE is matched here and the
                        // AMOUNT checked inside it. Matching `gains: 3 VPs,
                        // access to` directly would let `gains: 2 VPs,access to`
                        // fall past every Port arm into the generic `gains:`
                        // splitter as an untyped note — surfacing much later as
                        // a truthful but far-removed T9 divergence instead of
                        // the template error promised right here.
                        if !log.contains("gains: 3 VPs,access to") {
                            return Err(err_template(mid, "resourceChange", log));
                        }
                        push(Fact::PortAccess { seat: s, vp_bonus: 3 });
                    } else if log == "${player_name}'s Stock Market sets yens to 2" {
                        // The SystemOfStock turn-start top-up (measured live at
                        // table 896157897, m95/m109). The engine applies the
                        // same effect at turn start (`apply_turn_start_effects`)
                        // so this is corroboration; the T9 `yens` checkpoints
                        // arbitrate agreement. The "2" is literal wire text —
                        // a different amount is a different template and stays
                        // a hard error until measured.
                        push(Fact::Gain { seat: s, text: "Stock Market sets yens to 2".to_string() });
                    } else if let Some(rest) = log.split("gains: ").nth(1) {
                        push(Fact::Gain { seat: s, text: strip_bold(rest) });
                    } else {
                        return Err(err_template(mid, "resourceChange", log));
                    }
                }

                "payForPres" => {
                    let s = seat("payForPres")?;
                    let amount = require_u8(args, "amount", "payForPres").map_err(ImportError::Map)?;
                    push(Fact::PlaceFee { seat: s, amount });
                }

                "orderAcquisition" => {
                    let s = seat("orderAcquisition")?;
                    let text = args.get("order_text").and_then(|t| t.as_str()).unwrap_or("");
                    let second = if text == "an order" {
                        None
                    } else if text.contains("second order") {
                        if text.contains("2 <B>Yens</B>") || text.contains("2 Yens") {
                            Some(SecondPayment::TwoYen)
                        } else if text.contains("Import") {
                            Some(SecondPayment::OneImport)
                        } else {
                            return Err(err_template(mid, "orderAcquisition order_text", text));
                        }
                    } else {
                        return Err(err_template(mid, "orderAcquisition order_text", text));
                    };
                    let card = match args.get("order_card") {
                        Some(row) if row.is_object() => {
                            Some(mapper.map_row(row, &format!("move {mid} orderAcquisition"))?.card)
                        }
                        _ => None,
                    };
                    push(Fact::PortTake { seat: s, card, second });
                }

                "orderRefresh" => push(Fact::PortRefill),
                "orderRefused" => push(Fact::PortSecondDeclined { seat: seat("orderRefused")? }),
                "techRefusal" => push(Fact::LabDeclined { seat: seat("techRefusal")? }),

                "techAcquisition" => {
                    let s = seat("techAcquisition")?;
                    let name = args
                        .get("tech_name")
                        .and_then(|t| t.as_str())
                        .ok_or_else(|| ImportError::Sheet {
                            detail: format!("move {mid}: techAcquisition without tech_name"),
                        })?
                        .to_string();
                    let tech_id_key = match args.get("tech_card") {
                        Some(row) if row.is_object() => Some((
                            require_u64(row, "tech_id", "techAcquisition").map_err(ImportError::Map)?,
                            require_u64(row, "tech_key", "techAcquisition").map_err(ImportError::Map)?,
                        )),
                        _ => None,
                    };
                    push(Fact::LabTake { seat: s, tech_name: name, tech_id_key });
                }

                "orderCompletion" => {
                    let s = seat("orderCompletion")?;
                    let row = args.get("order").ok_or_else(|| ImportError::Sheet {
                        detail: format!("move {mid}: orderCompletion without its order object"),
                    })?;
                    let card = mapper.map_row(row, &format!("move {mid} orderCompletion"))?.card;
                    push(Fact::OrderCompleted { seat: s, card });
                }

                "achievementClaimed" => {
                    let s = seat("achievementClaimed")?;
                    let class = args.get("achievement_class").and_then(|c| c.as_str()).unwrap_or("");
                    let category = match class {
                        "A" => 0,
                        "B" => 1,
                        "C" => 2,
                        other => {
                            return Err(ImportError::Sheet {
                                detail: format!("move {mid}: achievement_class {other:?} is not A|B|C"),
                            })
                        }
                    };
                    let vp = require_u8(args, "vp_count", "achievementClaimed").map_err(ImportError::Map)?;
                    push(Fact::AchievementClaimed { seat: s, category, vp });
                }

                "buildShop" => {
                    let s = seat("buildShop")?;
                    let name = args.get("tile_name").and_then(|t| t.as_str()).ok_or_else(|| {
                        ImportError::Sheet { detail: format!("move {mid}: buildShop without tile_name") }
                    })?;
                    let mut reward = SpaceReward::default();
                    let ctx = format!("move {mid} buildShop");
                    if let Some(gain_text) = args
                        .get("result_info")
                        .and_then(|r| r.get("gain_text"))
                        .and_then(|g| g.as_array())
                    {
                        for item in gain_text {
                            if let Some(itxt) = item.as_str() {
                                add_reward_item(&mut reward, itxt, &ctx)?;
                            }
                        }
                    } else if let Some(rest) = log.split("and gains: ").nth(1) {
                        for item in rest.split(',') {
                            add_reward_item(&mut reward, item, &ctx)?;
                        }
                    } else {
                        return Err(err_template(mid, "buildShop", log));
                    }
                    push(Fact::BuildShop {
                        seat: s,
                        tile: area(&strip_bold(name)).map_err(ImportError::Map)?,
                        reward,
                    });
                }

                "buildHouse" | "stationPlacement" | "removeAssistants" => {
                    return Err(ImportError::Sheet {
                        detail: format!(
                            "move {mid}: `{}` fired — its arg shape is UNKNOWN (inventory §7.4) and \
                             the bridge hard-errors on first sight rather than guessing",
                            n.kind
                        ),
                    });
                }

                "powerBonus" => {
                    let s = seat("powerBonus")?;
                    let count = require_u8(args, "res_count", "powerBonus").map_err(ImportError::Map)?;
                    let res = args.get("res_name").and_then(|r| r.as_str()).unwrap_or("<absent>");
                    push(Fact::PowerToken {
                        seat: s,
                        face: token_face(res, count, &format!("move {mid}"))?,
                    });
                }

                "churchPlacement" => {
                    let s = seat("churchPlacement")?;
                    if let Some(res_list) = args.get("res_list").and_then(|r| r.as_str()) {
                        let faith = require_u8(args, "added_faith", "churchPlacement").map_err(ImportError::Map)?;
                        let mut pays = ChurchPays::default();
                        for item in res_list.split(',') {
                            church_res_item(&mut pays, item, &format!("move {mid}"))?;
                        }
                        push(Fact::ChurchPay { seat: s, faith_added: faith, pays });
                    } else if args.get("vp_count").is_some() {
                        let vp = require_u8(args, "vp_count", "churchPlacement").map_err(ImportError::Map)?;
                        let moves = require_u8(args, "move_count", "churchPlacement").map_err(ImportError::Map)?;
                        push(Fact::ChurchSpace { seat: s, vp, moves });
                    } else {
                        return Err(err_template(mid, "churchPlacement", log));
                    }
                }

                "customsPlacement" => {
                    let s = seat("customsPlacement")?;
                    let count = require_u8(args, "import_count", "customsPlacement").map_err(ImportError::Map)?;
                    // gain_list: "22 VP(s)" or "10 VP(s),1 warehouse pull(s)" —
                    // the VP names the space; riders (spaces with printed
                    // warehouse/yen bonuses) are the engine's own cascade.
                    let gain = args.get("gain_list").and_then(|g| g.as_str()).unwrap_or("");
                    let mut vp: Option<u8> = None;
                    for item in gain.split(',') {
                        let item = item.trim();
                        if let Some(v) = item.strip_suffix(" VP(s)") {
                            vp = Some(v.parse::<u8>().map_err(|_| {
                                err_template(mid, "customsPlacement gain_list", gain)
                            })?);
                        } else if item.ends_with(" warehouse pull(s)") || item.ends_with(" yen(s)") {
                            // pinned rider vocabulary — the engine queues these
                        } else {
                            return Err(err_template(mid, "customsPlacement gain_list", gain));
                        }
                    }
                    let vp = vp.ok_or_else(|| err_template(mid, "customsPlacement gain_list", gain))?;
                    push(Fact::Customs { seat: s, count, vp });
                }

                "warehousePull" => {
                    let s = seat("warehousePull")?;
                    let (mut a, mut sh, mut th) = (0u8, 0u8, 0u8);
                    if args.get("assist_count").is_some() {
                        a = require_u8(args, "assist_count", "warehousePull").map_err(ImportError::Map)?;
                        sh = require_u8(args, "shop_count", "warehousePull").map_err(ImportError::Map)?;
                        th = require_u8(args, "house_count", "warehousePull").map_err(ImportError::Map)?;
                    } else if let Some(rest) = log.split("pulls from the <B>Warehouse</B>: ").nth(1) {
                        for item in rest.split(',') {
                            let item = strip_bold(item);
                            let item = item.trim();
                            let (num, what) = item.split_once(' ').ok_or_else(|| {
                                err_template(mid, "warehousePull", log)
                            })?;
                            let n: u8 = num.parse().map_err(|_| err_template(mid, "warehousePull", log))?;
                            match what {
                                "assistant(s)" => a = n,
                                "shop(s)" | "shophouse(s)" => sh = n,
                                "house(s)" | "trading house(s)" => th = n,
                                _ => return Err(err_template(mid, "warehousePull", log)),
                            }
                        }
                    } else {
                        return Err(err_template(mid, "warehousePull", log));
                    }
                    push(Fact::WarehousePull { seat: s, assistants: a, shophouses: sh, trading_houses: th });
                }

                "pullAssistant" => {
                    if !log.starts_with("<B>University</B>:") {
                        return Err(err_template(mid, "pullAssistant", log));
                    }
                    push(Fact::University { seat: seat("pullAssistant")? });
                }

                "moveAssistant" => {
                    let s = seat("moveAssistant")?;
                    // Socket form: from_id/to_id tile keys, "0" = hand — but the
                    // key→area join needs board context, so BOTH forms parse the
                    // pinned log templates (direction-bearing; inventory §7.2's
                    // log-text trap is exactly why templates are matched on the
                    // RAW template string, not the rendered words).
                    let from_loc = args.get("from_loc").and_then(|f| f.as_str());
                    let to_loc = args.get("to_loc").and_then(|t| t.as_str());
                    const T_AREA_AREA: &str =
                        "${player_name} moves an assistant from <B>${from_loc}</B> to <B>${to_loc}</B>";
                    const T_HAND_AREA: &str =
                        "${player_name} moves an assistant from their hand to <B>${to_loc}</B>";
                    const T_AREA_HAND: &str =
                        "${player_name} moves an assistant from <B>${to_loc}</B> to their hand";
                    let fact = match log {
                        T_AREA_AREA => Fact::AssistMove {
                            seat: s,
                            from: Some(
                                area(from_loc.ok_or_else(|| err_template(mid, "moveAssistant", log))?)
                                    .map_err(ImportError::Map)?,
                            ),
                            to: Some(
                                area(to_loc.ok_or_else(|| err_template(mid, "moveAssistant", log))?)
                                    .map_err(ImportError::Map)?,
                            ),
                        },
                        T_HAND_AREA => Fact::AssistMove {
                            seat: s,
                            from: None,
                            to: Some(
                                area(to_loc.ok_or_else(|| err_template(mid, "moveAssistant", log))?)
                                    .map_err(ImportError::Map)?,
                            ),
                        },
                        // The trap template: the SOURCE is rendered in ${to_loc}.
                        T_AREA_HAND => Fact::AssistMove {
                            seat: s,
                            from: Some(
                                area(to_loc.ok_or_else(|| err_template(mid, "moveAssistant", log))?)
                                    .map_err(ImportError::Map)?,
                            ),
                            to: None,
                        },
                        _ => return Err(err_template(mid, "moveAssistant", log)),
                    };
                    push(fact);
                }

                "assistantRecovery" => {
                    let s = seat("assistantRecovery")?;
                    let count = require_u8(args, "assist_count", "assistantRecovery").map_err(ImportError::Map)?;
                    push(Fact::Recovery { seat: s, count });
                }

                "endGame" => push(Fact::EndGameMarker),
                "lastRound" => push(Fact::LastRound),
                "finalScoring" => push(Fact::FinalScoringMarker),

                other => {
                    return Err(ImportError::Sheet {
                        detail: format!(
                            "move {mid}: notification type `{other}` has no translator arm — \
                             a named Unsupported, never a silent fall-through (plan §5)"
                        ),
                    });
                }
            }
        }
        if let Some(per_player) = last_counters {
            out.checkpoints.push(CounterCheckpoint { move_id: mid, per_player });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One history-form notification as a capture line. Player NAMES are never
    /// carried on a tracked file (the 2026-08-08 redaction ruling) and the
    /// player id is a placeholder; every other arg is verbatim from the wire.
    fn wire_line(move_id: u64, kind: &str, log: &str, args: Value) -> String {
        serde_json::json!({
            "received_at_ms": 1, "kind": "packets",
            "payload": {"source": "xhr", "body_text": serde_json::json!({
                "status": 1, "data": {"valid": 1, "data": [{
                    "packet_type": "history", "table_id": 42, "packet_id": "1",
                    "move_id": move_id.to_string(),
                    "data": [{"uid": "u1", "type": kind, "log": log, "args": args}]
                }]}
            }).to_string()}
        })
        .to_string()
    }

    fn extract_one(line: &str) -> Result<FactStream, ImportError> {
        let feed = yokohama_bridge::feed::read_and_collapse(line, None).unwrap();
        let mapper = OrderMapper::from_rules(&yokohama_core::Rules::standard()).unwrap();
        extract(&feed, &|pid| (pid == "11111111").then_some(0), &mapper)
    }

    /// The `presidentPlacement` log template is one string for BOTH observed
    /// actions — the verb is substituted from `${action}`, which is exactly why
    /// the arm dispatches on the ARG and never on the log.
    const T_PRESIDENT: &str =
        "${player_name} ${action} the president on the <B>${tile_name}</B> with a power of <B>${power}</B>";

    /// Register U5, shape 1 — a Tram move with an EMPTY `route_keys`.
    ///
    /// Verbatim from async table 896199544 m127 (redacted per above): the
    /// President tramming from tile 3 to tile 1 (Church), two steps apart on
    /// BGA's own tile links, so nothing was walked and `route_keys` is `[]`.
    /// In the real capture this instance is a SUPERSEDED takeback and the feed
    /// collapse drops it before extraction — it is pinned here because it is
    /// the shape BGA emits for a genuine teleport, and the arm must read it as
    /// a President fact rather than refuse it.
    #[test]
    fn the_trams_action_extracts_as_a_president_fact() {
        let line = wire_line(
            127,
            "presidentPlacement",
            T_PRESIDENT,
            serde_json::json!({
                "action": "trams", "player_id": "11111111", "power": "3",
                "pres_tile_key": "3", "route_keys": [], "tile_name": "Church",
                "trg_tile_key": "1"
            }),
        );
        let stream = extract_one(&line).expect("the trams verb is inside the observed set");
        assert_eq!(
            stream.facts[0].fact,
            Fact::President {
                seat: 0,
                dest: AreaType::Church,
                power: 3,
                via: PresidentVia::Tram
            }
        );
    }

    /// The Port entitlement line comes in TWO wordings, and the difference is
    /// three victory points.
    ///
    /// Below 5 Power BGA logs "gains: access to …" and nothing else moves. AT 5
    /// Power it logs the bonus FIRST — "gains: 3 VPs,access to …" — and books
    /// the VP at the president-placement move, before any order is taken. The
    /// engine grants the same +3 inside its atomic Port action, which requires
    /// an actual take (`apply_move.rs`'s `skip_at_power_5_port_does_not_grant_
    /// vp_bonus`), so the two systems agree on the total and disagree on the
    /// instant — and the translator can only skip that boundary by granularity
    /// if the bonus is TYPED rather than swallowed by the generic `gains:` arm.
    ///
    /// Verbatim from async table 896199544 m105, the first banked capture ever
    /// to run a Port action at 5 Power. The "3" is literal wire text, on the
    /// Stock Market precedent above: a different amount is a different template
    /// and stays a hard error until measured.
    #[test]
    fn the_five_power_port_line_types_its_vp_bonus_and_the_plain_one_reads_zero() {
        const T_RESOURCE: &str = "${player_name} gains: 3 VPs,access to <B>Low-High</B> Orders on the <B>Port Management Board</B>";
        let line = wire_line(
            105,
            "resourceChange",
            T_RESOURCE,
            serde_json::json!({ "player_id": "11111111" }),
        );
        let stream = extract_one(&line).expect("the 5-power port line is inside the observed set");
        assert_eq!(
            stream.facts[0].fact,
            Fact::PortAccess { seat: 0, vp_bonus: 3 },
            "the 5-power wording must carry its bonus, not fall to the generic `gains:` arm"
        );

        // The sub-5-power wording is the same fact with no bonus — so a reader
        // never has to ask which wording produced it.
        let plain = wire_line(
            30,
            "resourceChange",
            "${player_name} gains: access to <B>Low</B> Orders on the <B>Port Management Board</B>",
            serde_json::json!({ "player_id": "11111111" }),
        );
        let stream = extract_one(&plain).expect("the plain port line still extracts");
        assert_eq!(stream.facts[0].fact, Fact::PortAccess { seat: 0, vp_bonus: 0 });
    }

    /// **The claim above has to be TRUE, not merely intended.**
    ///
    /// "A different amount is a different template and stays a hard error until
    /// measured" is the Stock Market precedent, and there it holds: that arm
    /// matches the whole log EXACTLY, so any other amount falls through to
    /// `err_template`. Borrowed onto a `contains` arm it did NOT hold — a
    /// hypothetical `gains: 2 VPs,access to …` matched neither Port arm and fell
    /// to the generic `gains:` splitter as an untyped note, which would surface
    /// much later as a truthful but far-removed T9 divergence rather than as the
    /// template error the comment promises.
    ///
    /// The shape is now matched on its own and the AMOUNT checked inside it, so
    /// the promise is kept. Unmeasured and unreachable in all three banked
    /// captures — which is exactly why it needs a test rather than a reader's
    /// trust.
    #[test]
    fn a_port_bonus_amount_other_than_the_measured_three_is_a_hard_error() {
        let line = wire_line(
            105,
            "resourceChange",
            "${player_name} gains: 2 VPs,access to <B>Low-High</B> Orders on the <B>Port Management Board</B>",
            serde_json::json!({ "player_id": "11111111" }),
        );
        let err = extract_one(&line).expect_err("an unmeasured VP amount must STOP, not be noted");
        let text = err.to_string();
        assert!(text.contains("resourceChange"), "the refusal names the notification: {text}");
        assert!(text.contains("2 VPs,access to"), "the refusal quotes what it saw: {text}");
    }

    /// Register U5, shape 2 — a Tram move whose `route_keys` is NON-empty.
    ///
    /// Verbatim from async table 896199544 m128 (redacted per above), one of
    /// the two SURVIVING tram facts: tile 3 → tile 2 (Silk Mill), which BGA's
    /// tile links make directly adjacent, so `route_keys` carries the single
    /// destination key `["2"]` — on both surviving tram facts it is exactly
    /// `[trg_tile_key]`. The extracted fact is the SAME shape as the empty-route
    /// case: `route_keys` is recorded as evidence in the arm's register and
    /// deliberately not read, so it cannot change what the fact says.
    #[test]
    fn a_trams_action_with_route_keys_extracts_per_its_observed_shape() {
        let line = wire_line(
            128,
            "presidentPlacement",
            T_PRESIDENT,
            serde_json::json!({
                "action": "trams", "player_id": "11111111", "power": "2",
                "pres_tile_key": "3", "route_keys": ["2"], "tile_name": "Silk Mill",
                "trg_tile_key": "2"
            }),
        );
        let stream = extract_one(&line).expect("a non-empty route_keys is inside the observed set");
        assert_eq!(
            stream.facts[0].fact,
            Fact::President {
                seat: 0,
                dest: AreaType::SilkMill,
                power: 2,
                via: PresidentVia::Tram
            }
        );
    }

    /// The register-U5 discipline SURVIVES the widen: the observed set grew by
    /// exactly one measured verb, and a third one still stops the import naming
    /// itself. Without this the widen could be read as "presidentPlacement is
    /// now permissive", which is the opposite of what it is.
    #[test]
    fn an_unobserved_president_action_still_hard_errors() {
        let line = wire_line(
            9,
            "presidentPlacement",
            T_PRESIDENT,
            serde_json::json!({
                "action": "jetpack", "player_id": "11111111", "power": "3",
                "pres_tile_key": "3", "route_keys": [], "tile_name": "Church",
                "trg_tile_key": "1"
            }),
        );
        match extract_one(&line).unwrap_err() {
            ImportError::Sheet { detail } => {
                assert!(detail.contains("jetpack"), "the refusal names what was seen: {detail}");
                assert!(detail.contains("register U5"), "{detail}");
                assert!(detail.contains("move 9"), "the refusal names the move id: {detail}");
            }
            other => panic!("expected the U5 hard error, got {other:?}"),
        }
    }

    #[test]
    fn reward_items_parse_the_pinned_vocabulary_and_refuse_outside_it() {
        let mut r = SpaceReward::default();
        add_reward_item(&mut r, "2 move(s)", "t").unwrap();
        add_reward_item(&mut r, "2 tea", "t").unwrap();
        assert_eq!(r.movement, 2);
        assert_eq!(r.tea, 2);
        assert!(add_reward_item(&mut r, "2 blessings", "t").is_err());
    }

    #[test]
    fn church_res_items_parse_and_record_the_yen_amount() {
        let mut p = ChurchPays::default();
        for item in "1 tea,1 fish,1 silk,1 copper,2 yens".split(',') {
            church_res_item(&mut p, item, "t").unwrap();
        }
        assert!(p.tea && p.seafood && p.silk && p.copper && p.yen);
        assert!(!p.imported_good);
        assert_eq!(p.yen_amount, 2, "the census §4.1 divergence evidence rides the fact");
        assert!(church_res_item(&mut ChurchPays::default(), "1 souls", "t").is_err());
    }

    #[test]
    fn the_area_to_hand_trap_template_reads_the_direction_correctly() {
        // Inventory §7.2: in the to-hand template the SOURCE is rendered in
        // ${to_loc}; a translator written off to_loc alone reads it backwards.
        let line = serde_json::json!({
            "received_at_ms": 1, "kind": "packets",
            "payload": {"source": "xhr", "body_text": serde_json::json!({
                "status": 1, "data": {"valid": 1, "data": [{
                    "packet_type": "history", "table_id": 42, "packet_id": "1", "move_id": "9",
                    "data": [{"uid": "u1", "type": "moveAssistant",
                        "log": "${player_name} moves an assistant from <B>${to_loc}</B> to their hand",
                        "args": {"player_id": "11", "player_name": "P", "to_loc": "Bank"}}]
                }]}
            }).to_string()}
        })
        .to_string();
        let feed = yokohama_bridge::feed::read_and_collapse(&line, None).unwrap();
        let mapper = OrderMapper::from_rules(&yokohama_core::Rules::standard()).unwrap();
        let stream = extract(&feed, &|pid| (pid == "11").then_some(0), &mapper).unwrap();
        assert_eq!(
            stream.facts[0].fact,
            Fact::AssistMove { seat: 0, from: Some(AreaType::Bank), to: None }
        );
    }

    /// The Stock Market turn-start template (measured live at gate table
    /// 896157897, m95 and m109): BGA renders the SystemOfStock tech's
    /// turn-start top-up as a `resourceChange` whose log matches none of the
    /// pinned prefixes, so extraction hard-errored on the whole capture. The
    /// engine models the same effect (`apply_turn_start_effects`: money ≤ 1
    /// → 2 at turn start), so the notification is pure corroboration — a
    /// NOTE fact, with the T9 `yens` checkpoints arbitrating whether the two
    /// implementations actually agree. The "2" in the template is literal
    /// wire text; a different amount is a different template and stays a
    /// hard error until measured.
    #[test]
    fn the_stock_market_turn_start_template_is_a_pinned_note_fact() {
        let line = serde_json::json!({
            "received_at_ms": 1, "kind": "packets",
            "payload": {"source": "xhr", "body_text": serde_json::json!({
                "status": 1, "data": {"valid": 1, "data": [{
                    "packet_type": "history", "table_id": 42, "packet_id": "1", "move_id": "95",
                    "data": [{"uid": "u1", "type": "resourceChange",
                        "log": "${player_name}'s Stock Market sets yens to 2",
                        "args": {"player_id": "11", "player_name": "P"}}]
                }]}
            }).to_string()}
        })
        .to_string();
        let feed = yokohama_bridge::feed::read_and_collapse(&line, None).unwrap();
        let mapper = OrderMapper::from_rules(&yokohama_core::Rules::standard()).unwrap();
        let stream = extract(&feed, &|pid| (pid == "11").then_some(1), &mapper).unwrap();
        assert_eq!(
            stream.facts[0].fact,
            Fact::Gain { seat: 1, text: "Stock Market sets yens to 2".to_string() }
        );
        assert!(!stream.facts[0].fact.is_drive(), "corroboration, not a demanded engine move");
    }

    #[test]
    fn an_unknown_notification_kind_is_a_named_unsupported() {
        let line = serde_json::json!({
            "received_at_ms": 1, "kind": "packets",
            "payload": {"source": "xhr", "body_text": serde_json::json!({
                "status": 1, "data": {"valid": 1, "data": [{
                    "packet_type": "history", "table_id": 42, "packet_id": "1", "move_id": "9",
                    "data": [{"uid": "u1", "type": "mysteryEvent", "log": "", "args": {}}]
                }]}
            }).to_string()}
        })
        .to_string();
        let feed = yokohama_bridge::feed::read_and_collapse(&line, None).unwrap();
        let mapper = OrderMapper::from_rules(&yokohama_core::Rules::standard()).unwrap();
        match extract(&feed, &|_| Some(0), &mapper).unwrap_err() {
            ImportError::Sheet { detail } => assert!(detail.contains("mysteryEvent"), "{detail}"),
            other => panic!("expected named Unsupported, got {other:?}"),
        }
    }

    #[test]
    fn an_unobserved_shape_hard_errors_on_first_sight() {
        let line = serde_json::json!({
            "received_at_ms": 1, "kind": "packets",
            "payload": {"source": "xhr", "body_text": serde_json::json!({
                "status": 1, "data": {"valid": 1, "data": [{
                    "packet_type": "history", "table_id": 42, "packet_id": "1", "move_id": "9",
                    "data": [{"uid": "u1", "type": "stationPlacement", "log": "", "args": {}}]
                }]}
            }).to_string()}
        })
        .to_string();
        let feed = yokohama_bridge::feed::read_and_collapse(&line, None).unwrap();
        let mapper = OrderMapper::from_rules(&yokohama_core::Rules::standard()).unwrap();
        match extract(&feed, &|_| Some(0), &mapper).unwrap_err() {
            ImportError::Sheet { detail } => {
                assert!(detail.contains("stationPlacement") && detail.contains("UNKNOWN"), "{detail}")
            }
            other => panic!("expected UNKNOWN-shape hard error, got {other:?}"),
        }
    }
}
