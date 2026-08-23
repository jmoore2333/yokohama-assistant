//! The catch-up TRANSLATOR (H2, plan T8/T9): the fact stream consumed against
//! the engine's own legal list, every event applied through the same checked
//! `Session::apply` boundary as typed entry.
//!
//! The translator PROPOSES, the engine DISPOSES: at each decision point it
//! looks at the engine's pending-choice head / turn phase, takes the next
//! DRIVE fact when it belongs there, and otherwise plays the absence arm
//! (`Skip*`/`Decline*`/`EndAdditionalPhase`) — each still checked against
//! `get_legal_moves` by `Session::apply` itself. A fact the engine refuses, a
//! fact with no arm, or an ambiguity that survives the evidence is a typed
//! [`ImportError`] carrying the BGA move id and the engine context — the STOP
//! shape plan §4 demands, never a skip.
//!
//! Reveals ride the same choreography as typed live entry (`reveal.rs`): a
//! Port/Lab take is scratch-applied to find the refilled slots, the key model
//! names the cards (ascending deck keys), and the group commits as ONE atomic
//! `Session::apply(move, reveals)`. The opponent's starting order reveals via
//! `apply_standalone` at its completion moment — the live model's normal path.

use super::facts::{Fact, FactStream, PresidentVia, SecondPayment, TimedFact};
use super::resolve::Resolved;
use super::ImportError;
use crate::session::Session;
use std::collections::{BTreeMap, BTreeSet};
use yokohama_core::{
    apply_move_unchecked, AreaAction, AreaId, AreaType, ChurchTrack, CustomsTrack, Move,
    MoveSource, MoveTarget, PendingChoice, PlayerId, PortPayment, PresidentLocation, RecordedEvent,
    TurnPhase, WarehouseItem,
};

/// A counter mismatch found by the T9 cross-check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterDiff {
    pub move_id: u64,
    pub seat: u8,
    pub counter: String,
    pub bga: i64,
    pub engine: i64,
}

/// How far the replay got and what it verified on the way.
#[derive(Debug, Default)]
pub struct ReplayReport {
    /// Highest BGA move id whose facts were fully consumed.
    pub last_move_id: u64,
    pub engine_moves: usize,
    pub reveal_events: usize,
    pub checkpoints_passed: usize,
    /// First checkpoint divergence, if any (a STOP).
    pub diffs: Vec<CounterDiff>,
    /// Counter keys deliberately not cross-checked, with reasons.
    pub unchecked_counters: Vec<(String, String)>,
    pub notes: Vec<String>,
}

fn rerr(move_id: u64, detail: impl Into<String>) -> ImportError {
    ImportError::Sheet { detail: format!("replay STOP at BGA move {move_id}: {}", detail.into()) }
}

/// Counter keys the T9 check deliberately does not model, each with its
/// stated reason (T9: "a key we deliberately do not model is listed
/// explicitly with its reason").
const UNCHECKED_COUNTERS: [(&str, &str); 8] = [
    ("player_id", "identity, not state"),
    ("gears", "BGA tracks cumulative lab gears; the engine prices lab takes by power and keeps no cumulative gear stat"),
    ("prod_count", "BGA aggregation semantics unverified (buildings vs distinct areas)"),
    ("comm_count", "BGA aggregation semantics unverified (buildings vs distinct areas)"),
    ("flags_gb", "country-icon aggregation semantics unverified against engine sets"),
    ("flags_us", "country-icon aggregation semantics unverified against engine sets"),
    ("flags_fr", "country-icon aggregation semantics unverified against engine sets"),
    ("flags_ge", "country-icon aggregation semantics unverified against engine sets"),
];
const UNCHECKED_EXTRA: [(&str, &str); 1] =
    [("flags_ne", "country-icon aggregation semantics unverified against engine sets")];

/// The deck keys a k-slot Port refill reveals, in DRAW order.
///
/// Two measured facts meet here and the direction is their consequence, not a
/// convention: (1) the ENGINE refills rightmost-empty-first from the deck's
/// top (`refill_port`'s `Vec::pop`; pinned by this module's
/// `port_refill_reveal_list_is_draw_order_first_named_card_lands_rightmost`),
/// so `reveals[0]` lands in the RIGHTMOST refilled slot; (2) BGA's display
/// mechanics (census §1: slide-left, then ascending keys refill the vacated
/// right end — the order `port_shadow`'s ascending append mirrors) put the
/// LOWEST new key leftmost. Ascending keys landing in ascending slots
/// therefore requires the draw list DESCENDING. Reversing it is invisible to
/// the corpus checkpoints (the escaped mutation M4, H2 review SHOULD-1) but
/// mis-arranges the engine's display, whose slots carry LOW/MID/HIGH
/// take-power gating — pinned by
/// `refill_draw_keys_descend_so_ascending_keys_land_left_to_right`.
fn refill_draw_keys(next_key: u64, k: usize) -> Vec<u64> {
    (0..k).map(|i| next_key + (k - 1 - i) as u64).collect()
}

/// Every legal `MovePresident` to `dest_id`, each paired with the price the
/// ENGINE will charge for its path.
///
/// The price comes from `calculate_movement_cost_with_tech`, which is the same
/// single pricing source `enumerate_president_routes` labels routes with and
/// `apply_move` credits fees from — so a cost computed here cannot drift from
/// the cost the engine actually applies. It counts BOTH fee kinds: 1 Yen per
/// opponent President passed (paid to that player) and 1 Yen per Canal crossing
/// (paid to the bank). That second kind is why a fee COUNT is not a substitute
/// for a price — a canal-tolled walk passes nobody's President and still is not
/// free.
fn president_candidates_to(
    state: &yokohama_core::GameState,
    rules: &yokohama_core::Rules,
    legal: &[Move],
    seat: u8,
    dest_id: AreaId,
) -> Vec<(Move, u32)> {
    let player = PlayerId::new(seat);
    legal
        .iter()
        .filter_map(|m| match m {
            Move::MovePresident { path, destination } if *destination == dest_id => Some((
                m.clone(),
                yokohama_core::calculate_movement_cost_with_tech(
                    state,
                    player,
                    path,
                    Some(&rules.techs),
                ),
            )),
            _ => None,
        })
        .collect()
}

/// The UNIQUE zero-cost label among priced candidates — the label a FREE move
/// carries, whether that move is a Station teleport, a Tram teleport, or the
/// free WALK that suppressed one. `Err(n)` reports how many were found when it
/// was not exactly one, so the caller can refuse with the right message.
///
/// Uniqueness is the engine's guarantee, not a hope: `enumerate_president_routes`
/// emits one move per distinct `(destination, fee vector)`, so at most one walk
/// to a destination carries the EMPTY vector, and both teleport arms are
/// emitted only when no zero-cost move to that destination is already listed.
fn select_free_president_label(candidates: &[(Move, u32)]) -> Result<Move, usize> {
    let free: Vec<&Move> = candidates.iter().filter(|(_, cost)| *cost == 0).map(|(m, _)| m).collect();
    match free.len() {
        1 => Ok(free[0].clone()),
        n => Err(n),
    }
}

/// The WHOLE tram selection, as one seam a test can drive on a Tram-bearing
/// state (T1 review MUST-1: the naive readings — `path.is_empty()` inside the
/// selection, or routing a tram fact through the fee-count matcher — survived
/// the original suite because nothing exercised the selection end to end).
///
/// Order of gates: (1) a tram fact carrying OBSERVED pass-through fees
/// contradicts the matcher's whole premise (Tram moves for FREE; both
/// surviving register facts are fee-free) and refuses rather than absorbing
/// the evidence into a zero-cost pick (T1 review SHOULD-1); (2) the unique
/// zero-PRICE label among the destination's candidates — the teleport, or the
/// free WALK that suppressed it; (3) no unique free label is a typed STOP
/// naming the move id, the destination and every candidate with its price.
fn select_tram_move(
    state: &yokohama_core::GameState,
    rules: &yokohama_core::Rules,
    legal: &[Move],
    seat: u8,
    dest_id: AreaId,
    fees: usize,
    move_id: u64,
) -> Result<Move, ImportError> {
    // Display-only: the refusal text names the printed area type, derived
    // here so the signature stays inside clippy's argument bound.
    let dest = state.board_layout.areas.get(&dest_id).map(|cfg| cfg.area_type);
    if fees > 0 {
        return Err(rerr(
            move_id,
            format!(
                "a `trams` president move to {dest:?} ({dest_id:?}) arrived with {fees} observed pass-through \
                 fee(s) — the Tram technology moves for FREE and both surviving register facts \
                 are fee-free; refusing rather than assuming (never guessed)"
            ),
        ));
    }
    let candidates = president_candidates_to(state, rules, legal, seat, dest_id);
    match select_free_president_label(&candidates) {
        Ok(mv) => Ok(mv),
        Err(0) => Err(rerr(
            move_id,
            format!(
                "a `trams` president move to {dest:?} ({dest_id:?}) has no free (zero-cost) legal label — \
                 the Tram technology moves for free, so one must exist; candidates considered: {}",
                describe_candidates(&candidates)
            ),
        )),
        Err(n) => Err(rerr(
            move_id,
            format!(
                "{n} distinct zero-cost labels to {dest:?} ({dest_id:?}) — the engine's route enumeration \
                 emits at most one per destination; candidates considered: {}",
                describe_candidates(&candidates)
            ),
        )),
    }
}

/// Name every candidate considered, so a refusal reports what it saw rather
/// than picking the nearest.
fn describe_candidates(candidates: &[(Move, u32)]) -> String {
    if candidates.is_empty() {
        return "none".to_string();
    }
    candidates
        .iter()
        .map(|(m, cost)| match m {
            Move::MovePresident { path, .. } => format!("path {path:?} costing {cost}"),
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The BGA move boundaries that fall strictly INSIDE one engine-atomic Port
/// action — the ones whose counters describe a state the engine steps over.
///
/// Two things can stretch a Port action across BGA moves, and they stack:
///
/// * a **5-Power bonus**, booked by BGA at the president-placement move while
///   the engine books it inside the atomic action (with the take it requires),
///   so every boundary from the bonus up to the take is interior;
/// * a **two-card take split across moves** (first measured at m169/m170),
///   whose intervening boundary is interior for the same reason.
///
/// The last take's OWN move is never interior — that boundary is where the
/// composite's outcome gets pinned. With no bonus this is exactly the
/// `first_take..second_take` span the pair handling has always used, which is
/// what keeps every already-green checkpoint on the healthy captures green.
fn port_composite_interior(
    bonus_move: Option<u64>,
    first_take: u64,
    second_take: Option<u64>,
) -> std::ops::Range<u64> {
    bonus_move.unwrap_or(first_take)..second_take.unwrap_or(first_take)
}

/// One Laboratory payment variant, written as the PRICES it debits.
///
/// A summed scalar cannot describe these: `4 yen + 1 import` and
/// `2 yen + 2 imports` are different debits, and a reader deciding whether the
/// import guessed right needs to see both currencies. The 4th-tech surcharge is
/// labelled as such, because two variants can differ ONLY there and would
/// otherwise render identically.
fn lab_payment_label(action: &AreaAction) -> String {
    let AreaAction::Laboratory { extra_yen, extra_imported, extra_tech_payment, .. } = action else {
        return "not a Laboratory action".to_string();
    };
    let plural = |n: u8| if n == 1 { "import" } else { "imports" };
    let mut parts: Vec<String> = Vec::new();
    if *extra_yen > 0 {
        parts.push(format!("{extra_yen} yen"));
    }
    if *extra_imported > 0 {
        parts.push(format!("{extra_imported} {}", plural(*extra_imported)));
    }
    match extra_tech_payment {
        Some(true) => parts.push("4th-tech surcharge 2 yen".to_string()),
        Some(false) => parts.push("4th-tech surcharge 1 import".to_string()),
        None => {}
    }
    if parts.is_empty() {
        return "nothing beyond power".to_string();
    }
    parts.join(" + ")
}

/// What one Laboratory variant DEBITS, as `(yen, imported goods)` — the
/// 4th-tech surcharge included, since it is spent from the same two pockets.
fn lab_payment_debit(action: &AreaAction) -> (u32, u32) {
    let AreaAction::Laboratory { extra_yen, extra_imported, extra_tech_payment, .. } = action else {
        return (0, 0);
    };
    let (surcharge_yen, surcharge_imports) = match extra_tech_payment {
        Some(true) => (2, 0),
        Some(false) => (0, 1),
        None => (0, 0),
    };
    (*extra_yen as u32 + surcharge_yen, *extra_imported as u32 + surcharge_imports)
}

/// How a Laboratory take's payment was settled.
#[derive(Debug, PartialEq, Eq)]
enum LabPayment {
    /// The legal list left exactly one mix. No evidence needed, nothing
    /// declared — the path that existed before any of this, untouched.
    Determined(Move),
    /// Several mixes were legal and BGA's counters fit exactly one.
    Derived { chosen: Move, chosen_label: String, candidates: Vec<String> },
    /// The observed spend fits NO legal mix. The engine and BGA disagree about
    /// what happened, which is a divergence — never an ambiguity to be resolved
    /// by picking something.
    NoneConsistent { spend: (u32, u32), candidates: Vec<String> },
    /// Several mixes were legal and the counters did not choose between them —
    /// no usable window, or a spend that fits more than one. The honest refusal
    /// stands.
    Underdetermined { candidates: Vec<String> },
}

/// Settle which Laboratory payment was made.
///
/// **Every exact-cover variant pays the same industry value**, by construction:
/// `generate_lab_actions` emits `extra_imported = k` with
/// `extra_yen = 2*(deficit-k)`, so the value is `k + (deficit-k) = deficit` for
/// every `k`, and the 4th-tech surcharge forks 2 yen against 1 import for the
/// same entitlement. The mixes are INCOMPARABLE — no ordering exists to prefer
/// one — so a legal list with more than one entry cannot be narrowed by ranking.
/// It was ranked once, by a key that summed yen and imports as if one unit, and
/// on BGA table 896199544 m118 that silently debited a payment nobody made.
///
/// What the capture's notification never records, its COUNTERS do. `spend` is
/// the mover's `(yen, imports)` decrease across the move, read from the T9
/// checkpoints bracketing it; a mix is consistent when that is exactly what it
/// debits. **Owner ruling, 2026-08-23 (guidingpath):** seeing the funds and
/// types available beforehand and afterwards allows a derivation just as good as
/// seeing the turn, because that is the only way things could have been used in
/// it.
///
/// **The cost, recorded rather than absorbed:** at a derived move the two
/// consumed counters STOP being an independent check, because they chose the
/// move — a payment divergence there can no longer be detected. Every other
/// counter's T9 check still runs at that move, and the chosen mix must still be
/// one the engine offered, so legality authority is untouched. The declared line
/// the caller prints says both halves out loud.
///
/// Absent or underdetermining evidence may resolve NOTHING: the refusal stands.
/// Deriving settles an ambiguity; it never manufactures an answer.
fn resolve_lab_payment(variants: Vec<Move>, spend: Option<(u32, u32)>) -> LabPayment {
    if variants.len() == 1 {
        return LabPayment::Determined(variants.into_iter().next().expect("checked len"));
    }
    let labels: Vec<String> = variants
        .iter()
        .map(|m| match m {
            Move::ExecuteAreaAction { action } => lab_payment_label(action),
            _ => "not an area action".to_string(),
        })
        .collect();
    let Some(spend) = spend else {
        return LabPayment::Underdetermined { candidates: labels };
    };
    let fitting: Vec<usize> = variants
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            matches!(m, Move::ExecuteAreaAction { action } if lab_payment_debit(action) == spend)
        })
        .map(|(i, _)| i)
        .collect();
    match fitting.as_slice() {
        [only] => LabPayment::Derived {
            chosen: variants[*only].clone(),
            chosen_label: labels[*only].clone(),
            candidates: labels,
        },
        [] => LabPayment::NoneConsistent { spend, candidates: labels },
        _ => LabPayment::Underdetermined { candidates: labels },
    }
}


pub struct Translator<'a> {
    session: Session,
    resolved: &'a Resolved,
    facts: Vec<TimedFact>,
    cursor: usize,
    /// BGA player_id per engine seat.
    pid_of_seat: Vec<String>,
    /// Port display shadow: BGA order keys by position (leftmost first).
    port_shadow: Vec<u64>,
    next_order_deck_key: u64,
    /// Lab display shadow: BGA tech keys by position.
    lab_shadow: Vec<u64>,
    next_tech_deck_key: u64,
    /// Display observations `(anchor_move, keys)` for thin-take pruning.
    port_observations: Vec<(u64, BTreeSet<u64>)>,
    /// Kept starting-order card per seat (key model), for reveal timing.
    /// CANDIDATE kept starting orders per seat — every deal-pair key the key
    /// model saw held by that seat. A MID-GAME snapshot (the proving corpus)
    /// leaves exactly one candidate; a TABLE-OPEN snapshot (the H3 procedure,
    /// measured live at gate table 896157897) shows BOTH dealt cards still
    /// held, so the kept identity stays a two-candidate set until the discard
    /// or the completion itself disambiguates. A single-value map here silently
    /// overwrote candidate 1 with candidate 2 — the discarded card — and the
    /// completion's reveal-then-fulfil path never fired.
    kept_card_by_seat: BTreeMap<u8, Vec<yokohama_core::OrderCardId>>,
    /// Stashed customs VP for the space choice that follows the action.
    stash_customs_vp: Option<u8>,
    checkpoints: Vec<super::facts::CounterCheckpoint>,
    next_checkpoint: usize,
    /// Engine counter snapshots after every apply since the last checkpoint
    /// match — BGA's per-move counters describe MID-TURN states (m128 is
    /// pre-recovery, m129 post-recovery), so a checkpoint passes if the
    /// engine PASSED THROUGH a matching state, in order.
    state_snapshots: Vec<Vec<BTreeMap<String, i64>>>,
    snap_cursor: usize,
    /// BGA move ids INSIDE a spanning composite — a BGA-move boundary that
    /// falls between the facts of ONE engine-atomic action (first observed at
    /// m169/m170: a two-card Port take split across two BGA moves, applied as
    /// one engine `AreaAction::Port`). A checkpoint at such a boundary
    /// describes a state the engine never passes through BY GRANULARITY, not
    /// by divergence; `run_checkpoints_below` skips it WITH A NAMED NOTE
    /// instead of stopping — the composite's outcome is still pinned by the
    /// next checkpoint at or past its final move.
    mid_composite_moves: BTreeSet<u64>,
    /// The first `assistantPlacement` move — BGA moves BELOW this are the
    /// SETUP ERA (the two starting-order discards BGA sequences as moves 2..3
    /// while the engine atomizes both inside `finalize_setup`), so no engine
    /// state can match their asymmetric mid-discard counters. Measured live at
    /// the gate game (table 896157897, the first table whose capture ever
    /// carried setup-era checkpoints — the proving corpus's socket joined at
    /// move 107). Checkpoints below it are named skips, same discipline as
    /// `mid_composite_moves`.
    first_placement_move: u64,
    pub report: ReplayReport,
    max_move_seen: u64,
    /// A checkpoint STOP found inside `consume_drive` (surfaced by `step`).
    pending_checkpoint_error: Option<ImportError>,
    /// The fact stream carries BGA's own end-of-game markers (`finalScoring` /
    /// "End of game"). Only when BGA itself declared the game over may the
    /// translator close out the final turn's absence arms after the last
    /// drive fact (see `close_out_final_turn`); a bounded `--through` replay
    /// drops the markers with the facts and stays honestly mid-turn.
    stream_declares_game_end: bool,
}

impl<'a> Translator<'a> {
    pub fn new(
        session: Session,
        resolved: &'a Resolved,
        stream: FactStream,
        feed: &yokohama_bridge::feed::CollapsedFeed,
    ) -> Result<Translator<'a>, ImportError> {
        let answers = &resolved.answers;
        let pid_of_seat: Vec<String> =
            answers.seats.iter().map(|s| s.bga_player_id.clone()).collect();
        // Kept starting order per seat: the deal-pair key (2s+1, 2s+2) that the
        // key model saw HELD (location 4) by that seat's player id.
        let mut kept_card_by_seat = BTreeMap::new();
        for seat in 0..answers.player_count {
            for k in [2 * seat as u64 + 1, 2 * seat as u64 + 2] {
                if resolved.key_model.order_owner.get(&k).map(|p| p.as_str())
                    == Some(pid_of_seat[seat as usize].as_str())
                {
                    kept_card_by_seat
                        .entry(seat)
                        .or_insert_with(Vec::new)
                        .push(resolved.key_model.order_card(k)?);
                }
            }
        }
        // Display observations for thin-take pruning: every location-2 key set
        // with a move anchor (snapshot move_nbr, or the socket move id).
        let mut port_observations: Vec<(u64, BTreeSet<u64>)> = Vec::new();
        let collect_map = |map: &serde_json::Value, out: &mut Vec<(u64, BTreeSet<u64>)>, anchor: u64| {
            if let Some(obj) = map.as_object() {
                let keys: BTreeSet<u64> = obj
                    .values()
                    .filter(|r| r.get("location").and_then(|l| l.as_str()) == Some("2"))
                    .filter_map(|r| r.get("order_key").and_then(yokohama_bridge::feed::flex_u64))
                    .collect();
                if !keys.is_empty() {
                    out.push((anchor, keys));
                }
            }
        };
        for snap in &feed.snapshots {
            let anchor = snap
                .gamedatas
                .get("notifications")
                .and_then(|n| n.get("move_nbr"))
                .and_then(yokohama_bridge::feed::flex_u64)
                .unwrap_or(u64::MAX);
            if let Some(map) = snap.gamedatas.get("orders") {
                collect_map(map, &mut port_observations, anchor);
            }
        }
        for mv in &feed.moves {
            if let Some(pk) = &mv.socket {
                for n in &pk.notifications {
                    if let Some(map) = n.args.get("orders") {
                        collect_map(map, &mut port_observations, mv.move_id);
                    }
                }
            }
        }
        port_observations.sort_by_key(|(a, _)| *a);

        let pc = answers.player_count;
        let stream_declares_game_end = stream
            .facts
            .iter()
            .any(|tf| matches!(tf.fact, Fact::EndGameMarker | Fact::FinalScoringMarker));
        Ok(Translator {
            session,
            resolved,
            first_placement_move: stream
                .facts
                .iter()
                .filter(|f| f.kind == "assistantPlacement")
                .map(|f| f.move_id)
                .min()
                .unwrap_or(0),
            facts: stream.facts,
            cursor: 0,
            pid_of_seat,
            port_shadow: super::keys::KeyModel::initial_port_keys(pc).to_vec(),
            next_order_deck_key: super::keys::KeyModel::first_order_deck_key(pc),
            lab_shadow: (1..=6).collect(),
            next_tech_deck_key: 7,
            port_observations,
            kept_card_by_seat,
            stash_customs_vp: None,
            checkpoints: stream.checkpoints,
            next_checkpoint: 0,
            state_snapshots: Vec::new(),
            snap_cursor: 0,
            mid_composite_moves: BTreeSet::new(),
            // The deliberately-unchecked list rides the report from
            // CONSTRUCTION so every return path — the STOPPED runs included —
            // carries it (H2 review SHOULD-2: T9's "listed explicitly with
            // its reason" has to hold on exactly the run whose report
            // matters most, and the headline acceptance run stops at m154).
            report: ReplayReport {
                unchecked_counters: UNCHECKED_COUNTERS
                    .iter()
                    .chain(UNCHECKED_EXTRA.iter())
                    .map(|(k, r)| (k.to_string(), r.to_string()))
                    .collect(),
                ..ReplayReport::default()
            },
            max_move_seen: 0,
            pending_checkpoint_error: None,
            stream_declares_game_end,
        })
    }

    // ---- fact cursor -------------------------------------------------------

    fn next_drive_index(&self, from: usize) -> Option<usize> {
        (from..self.facts.len()).find(|i| self.facts[*i].fact.is_drive())
    }

    fn peek_drive(&self) -> Option<&TimedFact> {
        self.next_drive_index(self.cursor).map(|i| &self.facts[i])
    }

    /// Consume through the next drive fact (notes before it consumed freely).
    /// T9 checkpoints for every EARLIER move run first — at this point the
    /// engine has played all the absence arms (skips/declines and the
    /// automatic recovery tail) belonging to those moves, so its state is the
    /// move-end state BGA's counters describe (measured: checking at step()
    /// entry compared a MID-TURN engine state against move-end counters).
    fn consume_drive(&mut self) -> Option<TimedFact> {
        let idx = self.next_drive_index(self.cursor)?;
        let tf = self.facts[idx].clone();
        if let Err(e) = self.run_checkpoints_below(tf.move_id) {
            self.pending_checkpoint_error = Some(e);
            return None;
        }
        self.cursor = idx + 1;
        self.max_move_seen = self.max_move_seen.max(tf.move_id);
        Some(tf)
    }

    /// What a `consume_drive()` that returned `None` ACTUALLY means.
    ///
    /// **`None` is two unrelated facts wearing one shape**, and telling them
    /// apart is the whole point of this helper. Either the drive queue is
    /// genuinely empty, or [`Self::consume_drive`] ran the checkpoints below the
    /// next fact, one of them FAILED, and the real error was stashed in
    /// `pending_checkpoint_error` (which is only read at the next [`Self::step`]
    /// entry — too late for a caller that is about to return an error of its
    /// own from inside the current one).
    ///
    /// So the stashed error WINS whenever there is one, and `when_empty` — the
    /// caller's description of an exhausted queue — is reachable only when
    /// nothing was stashed. Reporting a checkpoint divergence as a missing fact
    /// cost a whole diagnosis on BGA table 896199544, where "port take fact
    /// vanished" printed at m106 with the `orderAcquisition` notification
    /// sitting in the feed and the true stop a T9 divergence one move earlier.
    ///
    /// Taking the error CLEARS it, so one failure is never reported twice.
    fn drive_absence(&mut self, move_id: u64, when_empty: &str) -> ImportError {
        match self.pending_checkpoint_error.take() {
            Some(e) => e,
            None => rerr(move_id, when_empty),
        }
    }

    /// Consume WITHOUT advancing the checkpoint boundary — for the 2nd+
    /// fact of one gathering decide (a port pair, a composed assistant move),
    /// whose engine apply has not happened yet. Their checkpoints run at the
    /// next first-consume, after the turn's absence arms settle (measured:
    /// checking at the second consume compared a PRE-APPLY engine state
    /// against the take's post-state counters).
    fn consume_drive_uncheckpointed(&mut self) -> Option<TimedFact> {
        let idx = self.next_drive_index(self.cursor)?;
        let tf = self.facts[idx].clone();
        self.cursor = idx + 1;
        self.max_move_seen = self.max_move_seen.max(tf.move_id);
        Some(tf)
    }

    /// Pass-through fee notes accompanying the NEXT drive fact: BGA emits the
    /// `pays 1 yen … passing through` resourceChange in the SAME packet as its
    /// presidentPlacement, sometimes BEFORE it (measured at m78) — so the fee
    /// window is everything from the cursor to the drive fact after next.
    fn pass_fee_count_window(&self) -> usize {
        let Some(i) = self.next_drive_index(self.cursor) else { return 0 };
        let end = self.next_drive_index(i + 1).unwrap_or(self.facts.len());
        self.facts[self.cursor..end]
            .iter()
            .filter(|tf| matches!(tf.fact, Fact::PassFee { .. }))
            .count()
    }

    // ---- engine helpers ----------------------------------------------------

    fn state(&self) -> &yokohama_core::GameState {
        self.session.state()
    }

    fn area_id(&self, ty: AreaType, move_id: u64) -> Result<AreaId, ImportError> {
        self.state()
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type == ty)
            .map(|(id, _)| *id)
            .ok_or_else(|| rerr(move_id, format!("area {ty:?} is not on this board")))
    }

    fn current_seat(&self) -> u8 {
        self.state().current_player.0
    }

    /// `action: "places"` — disambiguate the route label by the OBSERVED
    /// pass-through fee count (unchanged since H2; BGA emits one `payForPres`
    /// per president passed, and the engine's own path pricing counts the same
    /// areas).
    fn walked_president_move(
        &self,
        seat: u8,
        dest: AreaType,
        dest_id: AreaId,
        fees: usize,
        move_id: u64,
    ) -> Result<Move, ImportError> {
        let state = self.state();
        let opponents_at = |area: &AreaId| {
            state.players.iter().enumerate().any(|(i, p)| {
                i != seat as usize && p.president_location == PresidentLocation::OnBoard(*area)
            })
        };
        let candidates: Vec<Move> = self
            .session
            .legal()
            .into_iter()
            .filter(|m| match m {
                Move::MovePresident { path, destination } => {
                    *destination == dest_id
                        && path.iter().filter(|a| opponents_at(a)).count() == fees
                }
                _ => false,
            })
            .collect();
        match candidates.len() {
            1 => Ok(candidates.into_iter().next().expect("len checked")),
            0 => Err(rerr(
                move_id,
                format!("no legal president route to {dest:?} with {fees} pass-through fee(s)"),
            )),
            n => Err(rerr(
                move_id,
                format!("{n} distinct route labels to {dest:?} with {fees} fee(s) — underdetermined"),
            )),
        }
    }

    /// `action: "trams"` — the Tram technology's move, matched on the
    /// TELEPORT'S LABEL rather than on a fee count.
    ///
    /// Tram's card text is "move your President to any area … for free", and
    /// the engine emits it as `MovePresident { path: [] }` with an empty fee
    /// vector. But `movement_routes_family` SUPPRESSES a teleport to any
    /// destination already covered by a ZERO-COST move, so the surviving
    /// representative of a tram to an adjacent tile may be the free WALK —
    /// which is exactly what both surviving tram facts in async table 896199544
    /// are (m128 tile 3→2, m137 tile 2→1, both single steps on BGA's tile
    /// links, both by a seat holding 0 yen, so neither could have cost
    /// anything). Matching on `path.is_empty()` would therefore find nothing;
    /// matching on the pass-through fee count alone is not equivalent either,
    /// because a CANAL toll is charged to the bank and adds no pass-through fee
    /// while still making a walk a different label from the teleport.
    ///
    /// So the rule is the LABEL: the unique legal move to this destination
    /// whose total cost is zero. The engine guarantees uniqueness —
    /// `enumerate_president_routes` emits one move per distinct
    /// `(destination, fee vector)` so at most one walk carries the empty
    /// vector, and the teleport is emitted only when no zero-cost move to that
    /// destination exists.
    ///
    /// A tram fact that matches no such label is a typed STOP naming the move
    /// id, the destination and every candidate considered — never a nearest
    /// pick. The body lives in the free [`select_tram_move`] so the selection
    /// is testable on Tram-bearing states without a `Translator` (T1 review
    /// MUST-1); this method is a one-line delegate.
    fn tram_president_move(
        &self,
        seat: u8,
        dest_id: AreaId,
        fees: usize,
        move_id: u64,
    ) -> Result<Move, ImportError> {
        select_tram_move(
            self.state(),
            self.session.rules(),
            &self.session.legal(),
            seat,
            dest_id,
            fees,
            move_id,
        )
    }

    /// Resolve the pending customs-space choice from the stashed
    /// `customsPlacement` VP evidence — the one choice whose evidence is
    /// consumed WITH its action fact (the church space arrives as its own
    /// fact; the customs space rides the action's `gain_list`), so it must
    /// resolve wherever the choice surfaces: mid-stream (`step_choice`) or at
    /// the endgame close-out, where the stream ends with the choice still
    /// queued (measured live at table 896157897: the game's final move is a
    /// customs sale, and `close_out_final_turn` refused on the non-declinable
    /// pending choice it could in fact resolve from evidence).
    fn customs_space_move(&mut self, move_id: u64) -> Result<Move, ImportError> {
        let vp = self.stash_customs_vp.take().ok_or_else(|| {
            rerr(move_id, "customs-space choice with no stashed VP evidence")
        })?;
        let space = (1..=7u8)
            .find(|s| CustomsTrack::vp_reward(*s) == vp)
            .ok_or_else(|| rerr(move_id, format!("no customs space grants {vp} VP")))?;
        Ok(Move::ChooseCustomsSpace { space })
    }

    /// Engine counter snapshot for every checked key, per seat.
    fn counter_snapshot(&self) -> Vec<BTreeMap<String, i64>> {
        let keys = [
            "tea", "fish", "silk", "copper", "yens", "imports", "vps", "orders_done",
            "orders_hand", "power_tokens", "tech_count", "fa_tokens", "fa_used",
            "board_president", "hand_president", "board_assists", "hand_assists", "wh_assists",
            "board_shops", "hand_shops", "wh_shops", "board_houses", "hand_houses", "wh_houses",
        ];
        (0..self.state().players.len() as u8)
            .map(|seat| {
                keys.iter()
                    .map(|k| (k.to_string(), self.engine_counter(seat, k).expect("checked key")))
                    .collect()
            })
            .collect()
    }

    fn push_snapshot(&mut self) {
        let snap = self.counter_snapshot();
        if self.state_snapshots.last() != Some(&snap) {
            self.state_snapshots.push(snap);
        }
    }

    fn apply(&mut self, mv: Move, reveals: Vec<RecordedEvent>, move_id: u64) -> Result<(), ImportError> {
        self.report.engine_moves += 1;
        self.report.reveal_events += reveals.len();
        self.session.apply(mv.clone(), reveals).map_err(|e| {
            rerr(
                move_id,
                format!(
                    "engine refused {mv:?}: {e}\n  phase {:?}, current seat {}, pending {:?}\n  \
                     legal: {:?}",
                    self.state().turn_phase,
                    self.current_seat(),
                    self.state().pending_choices,
                    self.session.legal().into_iter().take(12).collect::<Vec<_>>()
                ),
            )
        })?;
        self.push_snapshot();
        Ok(())
    }

    // ---- reveals -----------------------------------------------------------

    /// Scratch-apply `mv` and, if Port/Lab slots refilled from the deck, bind
    /// the reveal events the key model predicts.
    fn reveals_for(&mut self, mv: &Move, move_id: u64) -> Result<Vec<RecordedEvent>, ImportError> {
        let before = self.state().clone();
        let after = match apply_move_unchecked(&before, mv, self.session.rules()) {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()), // the real apply will report it properly
        };
        let mut reveals = Vec::new();
        let (slots, board_b) = crate::reveal::refilled_port_slots(&before, &after);
        if !slots.is_empty() {
            if board_b {
                return Err(rerr(move_id, "port B refill in a 2P game"));
            }
            // Ascending deck keys land in ascending slots; draw order is
            // rightmost-first, so the reveal list is keys DESCENDING (the
            // derivation and its pins live on `refill_draw_keys`).
            let k = slots.len();
            let keys: Vec<u64> = refill_draw_keys(self.next_order_deck_key, k);
            let mut cards = Vec::with_capacity(k);
            for key in &keys {
                cards.push(self.resolved.key_model.order_card(*key)?);
            }
            // Shadow: the take already removed its keys; append ascending.
            for i in 0..k {
                self.port_shadow.push(self.next_order_deck_key + i as u64);
            }
            self.next_order_deck_key += k as u64;
            reveals.push(RecordedEvent::RevealOrders(cards));
        }
        if crate::reveal::lab_b_changed(&before, &after) {
            return Err(rerr(move_id, "lab B refill in a 2P game"));
        }
        let lab_before: Vec<_> = before.lab_state.lab_a.slots.iter().flatten().copied().collect();
        let lab_after: Vec<_> = after.lab_state.lab_a.slots.iter().flatten().copied().collect();
        if lab_after.len() == lab_before.len() && lab_before != lab_after {
            // A tech left and the deck refilled the vacated slot.
            let key = self.next_tech_deck_key;
            let tech_id = self.resolved.key_model.tech_id(key)?;
            let card = *self.resolved.tech_cards.get(&key).ok_or_else(|| {
                rerr(move_id, format!("lab refill tech_key {key} (tech_id {tech_id}) has no resolved identity"))
            })?;
            self.lab_shadow.push(key);
            self.next_tech_deck_key += 1;
            reveals.push(RecordedEvent::RevealTechs(vec![card]));
        }
        Ok(reveals)
    }

    // ---- thin-take resolution ---------------------------------------------

    /// Candidate display keys for a thin Port take by `seat`: on the shadow
    /// display, eventually owned by that seat, and absent from the next
    /// display observation at-or-after this move.
    fn thin_take_candidates(&self, seat: u8, move_id: u64) -> Vec<u64> {
        let pid = &self.pid_of_seat[seat as usize];
        let next_obs = self
            .port_observations
            .iter()
            .find(|(anchor, _)| *anchor >= move_id)
            .map(|(_, keys)| keys);
        self.port_shadow
            .iter()
            .copied()
            .filter(|k| self.resolved.key_model.order_owner.get(k) == Some(pid))
            .filter(|k| next_obs.map(|obs| !obs.contains(k)).unwrap_or(true))
            .collect()
    }

    // ---- checkpoints (T9) --------------------------------------------------

    fn engine_counter(&self, seat: u8, key: &str) -> Option<i64> {
        let p = &self.state().players[seat as usize];
        let v: i64 = match key {
            "tea" => p.tea as i64,
            "fish" => p.seafood as i64,
            "silk" => p.silk as i64,
            "copper" => p.copper as i64,
            "yens" => p.money as i64,
            "imports" => p.imported_goods as i64,
            // BGA's endgame checkpoint (m194 here) carries the FINAL total —
            // its finalScoring folds every bonus into `vps` — while the
            // engine's `victory_points` stays incremental and the bonuses live
            // in `final_result`. Mirror BGA's convention exactly when (and
            // only when) the game is over, so the terminal checkpoint compares
            // final against final; every earlier snapshot is incremental on
            // both sides.
            "vps" => {
                if self.session.is_over() {
                    self.session
                        .final_scores()
                        .iter()
                        .find(|(p, _)| p.0 == seat)
                        .map(|(_, s)| *s as i64)?
                } else {
                    p.victory_points as i64
                }
            }
            "orders_done" => p.completed_orders.len() as i64,
            "orders_hand" => {
                p.order_cards_in_hand.len() as i64 + if p.starting_order_hidden { 1 } else { 0 }
            }
            "power_tokens" => p.five_power_tokens as i64,
            "tech_count" => p.technologies.len() as i64,
            "fa_tokens" => {
                (p.mercenaries.available.values().sum::<u8>()
                    + p.mercenaries.used.values().sum::<u8>()) as i64
            }
            "fa_used" => p.mercenaries.used.values().sum::<u8>() as i64,
            "board_president" => matches!(p.president_location, PresidentLocation::OnBoard(_)) as i64,
            "hand_president" => matches!(p.president_location, PresidentLocation::InHand) as i64,
            "board_assists" => {
                (p.assistants.on_board.values().sum::<u8>()
                    + p.assistants.on_church
                    + p.assistants.on_customs
                    + p.assistants.on_achievements) as i64
            }
            "hand_assists" => p.assistants.in_hand as i64,
            "wh_assists" => p.assistants.in_warehouse as i64,
            // `built` maps area → SLOT INDEX (0-3), so the count is len().
            "board_shops" => p.shophouses.built.len() as i64,
            "hand_shops" => p.shophouses.in_hand as i64,
            "wh_shops" => p.shophouses.in_warehouse as i64,
            "board_houses" => p.trading_houses.built.len() as i64,
            "hand_houses" => p.trading_houses.in_hand as i64,
            "wh_houses" => p.trading_houses.in_warehouse as i64,
            _ => return None,
        };
        Some(v)
    }

    /// The mover's `(yen, imports)` SPEND across `move_id`, read from the T9
    /// checkpoints BRACKETING it — the evidence a derived lab payment rests on.
    ///
    /// `None` means no usable window, and every arm that returns it is a reason
    /// to refuse rather than to derive: no checkpoint at this move or below it,
    /// no counters for this seat, non-numeric values — or **either pocket GREW**.
    /// A payment only ever spends, so a rise means something else moved money in
    /// this window and the delta no longer isolates the payment. Reading it as
    /// one would be exactly the guess this machinery exists to avoid.
    fn observed_spend(&self, seat: u8, move_id: u64) -> Option<(u32, u32)> {
        let pid = self.pid_of_seat.get(seat as usize)?;
        let read = |cp: &super::facts::CounterCheckpoint| -> Option<(u64, u64)> {
            let obj = cp.per_player.get(pid)?.as_object()?;
            let yens = yokohama_bridge::feed::flex_u64(obj.get("yens")?)?;
            let imports = yokohama_bridge::feed::flex_u64(obj.get("imports")?)?;
            Some((yens, imports))
        };
        let after = read(self.checkpoints.iter().find(|cp| cp.move_id == move_id)?)?;
        let before = read(self.checkpoints.iter().rfind(|cp| cp.move_id < move_id)?)?;
        Some((
            before.0.checked_sub(after.0)? as u32,
            before.1.checked_sub(after.1)? as u32,
        ))
    }

    /// Does snapshot `snap` satisfy checkpoint `cp`? Returns the diffs when it
    /// does not. Counters compare RAW — since rev 6 (D1 church yen price 2,
    /// D2 surcharge from the 4th tech) the engine agrees with BGA and there is
    /// no offset machinery left to acknowledge a divergence: any diff is a STOP.
    fn checkpoint_diffs(
        &self,
        cp: &super::facts::CounterCheckpoint,
        snap: &[BTreeMap<String, i64>],
    ) -> Result<Vec<CounterDiff>, ImportError> {
        let mut diffs = Vec::new();
        for (pid, counters) in &cp.per_player {
            let Some(seat) = self.pid_of_seat.iter().position(|p| p == pid) else {
                return Err(rerr(cp.move_id, format!("checkpoint counters for unknown player {pid}")));
            };
            let obj = counters.as_object().ok_or_else(|| {
                rerr(cp.move_id, format!("counters for {pid} is not an object"))
            })?;
            for (key, value) in obj {
                if UNCHECKED_COUNTERS.iter().chain(UNCHECKED_EXTRA.iter()).any(|(k, _)| k == key) {
                    continue;
                }
                let bga = yokohama_bridge::feed::flex_u64(value).ok_or_else(|| {
                    rerr(cp.move_id, format!("counter {key} for {pid} is not numeric: {value}"))
                })? as i64;
                let engine = snap[seat].get(key).copied().ok_or_else(|| {
                    rerr(
                        cp.move_id,
                        format!("counter key `{key}` is neither checked nor deliberately listed — T9 refuses silent ignores"),
                    )
                })?;
                if bga != engine {
                    diffs.push(CounterDiff {
                        move_id: cp.move_id,
                        seat: seat as u8,
                        counter: key.clone(),
                        bga,
                        engine,
                    });
                }
            }
        }
        Ok(diffs)
    }

    /// Run every checkpoint below `boundary` against the engine's state
    /// SEQUENCE: each must match some state the engine passed through, at or
    /// after the previous checkpoint's match. A checkpoint no state satisfies
    /// is a STOP carrying the diff against the latest state.
    fn run_checkpoints_below(&mut self, boundary: u64) -> Result<(), ImportError> {
        while self.next_checkpoint < self.checkpoints.len()
            && self.checkpoints[self.next_checkpoint].move_id < boundary
        {
            let cp = self.checkpoints[self.next_checkpoint].clone();
            self.next_checkpoint += 1;
            let mut matched = None;
            for i in self.snap_cursor..self.state_snapshots.len() {
                match self.checkpoint_diffs(&cp, &self.state_snapshots[i].clone())? {
                    d if d.is_empty() => {
                        matched = Some(i);
                        break;
                    }
                    _ => {}
                }
            }
            match matched {
                Some(i) => {
                    self.snap_cursor = i;
                    self.report.checkpoints_passed += 1;
                }
                None if cp.move_id < self.first_placement_move => {
                    // SETUP ERA (see `first_placement_move`): BGA sequences the
                    // starting-order discards as moves; the engine's
                    // `finalize_setup` atomizes them, so the asymmetric
                    // mid-discard state exists in BGA's granularity only. The
                    // post-setup state is pinned by the first real-turn
                    // checkpoint at or past the first Placement.
                    self.report.notes.push(format!(
                        "checkpoint m{}: setup-era (before the first Placement at m{}) — BGA \
                         sequences the starting-order discards the engine atomizes in \
                         finalize_setup; skipped by granularity, the post-setup state is \
                         checked from the first Placement on",
                        cp.move_id, self.first_placement_move
                    ));
                }
                None if self.mid_composite_moves.contains(&cp.move_id) => {
                    // A BGA-move boundary INSIDE one engine-atomic action
                    // (see `mid_composite_moves`): the state its counters
                    // describe exists in BGA's finer move granularity only.
                    // Skipped WITH ITS NAME — never counted as green, and the
                    // composite's end state is still pinned by the next
                    // checkpoint at or past its final move.
                    self.report.notes.push(format!(
                        "checkpoint m{}: describes the state inside an engine-atomic action \
                         (a composite spanning BGA moves); skipped by granularity, the \
                         composite's outcome is checked at the next checkpoint",
                        cp.move_id
                    ));
                }
                None => {
                    let last = self
                        .state_snapshots
                        .last()
                        .cloned()
                        .unwrap_or_else(|| self.counter_snapshot());
                    let diffs = self.checkpoint_diffs(&cp, &last)?;
                    self.report.diffs = diffs;
                    let d = self.report.diffs.first().cloned();
                    if let Some(d) = d {
                        return Err(rerr(
                            cp.move_id,
                            format!(
                                "T9 counter divergence — no engine state in this turn window \
                                 matches BGA's move-{} counters; nearest: seat {} `{}` BGA {} vs \
                                 engine {} ({} diff(s) total)",
                                cp.move_id,
                                d.seat,
                                d.counter,
                                d.bga,
                                d.engine,
                                self.report.diffs.len()
                            ),
                        ));
                    }
                }
            }
        }
        // Bound memory: states before the last match can never match again.
        if self.snap_cursor > 0 {
            self.state_snapshots.drain(..self.snap_cursor);
            self.snap_cursor = 0;
        }
        Ok(())
    }

    // ---- the decision function --------------------------------------------

    /// Translate one engine decision point. Returns false when the stream is
    /// exhausted and the engine has nothing further to do.
    fn step(&mut self) -> Result<bool, ImportError> {
        if let Some(e) = self.pending_checkpoint_error.take() {
            return Err(e);
        }
        if self.session.is_over() {
            // Post-game BGA bookkeeping (the physical president return) is
            // consumed with a note; any other remaining drive fact is a STOP.
            while let Some(tf) = self.peek_drive().cloned() {
                match tf.fact {
                    Fact::ReturnPresident { .. } | Fact::RecoverNone { .. } => {
                        self.report.notes.push(format!(
                            "move {}: post-game {} consumed as BGA cleanup (engine game already over)",
                            tf.move_id, tf.kind
                        ));
                        self.consume_drive();
                    }
                    _ => {
                        return Err(rerr(
                            tf.move_id,
                            format!("engine game is over but BGA continues with {:?}", tf.fact),
                        ))
                    }
                }
            }
            return Ok(false);
        }
        let Some(next) = self.peek_drive().cloned() else {
            // Stream exhausted. When BGA itself declared the game over (the
            // m194-class endgame markers rode the stream), the final turn's
            // absence arms close out — stream end past the markers is the same
            // absence evidence a later-move fact provides mid-stream. Without
            // the markers this is a bounded replay and stays honestly mid-turn.
            if self.stream_declares_game_end && !self.session.is_over() {
                self.close_out_final_turn()?;
            }
            return Ok(false);
        };
        let move_id = next.move_id;
        self.report.last_move_id = self.max_move_seen;

        // Pending choice head first (queue-first legality).
        if let Some(choice) = self.state().pending_choices.first().cloned() {
            return self.step_choice(choice, &next).map(|_| true);
        }

        let seat = self.current_seat();
        match self.state().turn_phase {
            TurnPhase::AdditionalActionFirst | TurnPhase::AdditionalActionSecond => {
                match &next.fact {
                    Fact::University { seat: s } if *s == seat => {
                        self.consume_drive();
                        self.apply(Move::UseUniversity, vec![], move_id)?;
                    }
                    Fact::OrderCompleted { seat: s, card } if *s == seat => {
                        let card = *card;
                        self.consume_drive();
                        // A hidden starting order reveals at its completion —
                        // the live model's normal path.
                        if self.state().players[seat as usize].starting_order_hidden
                            && self.kept_card_by_seat.get(&seat).is_some_and(|c| c.contains(&card))
                        {
                            self.session
                                .apply_standalone(RecordedEvent::RevealStartingOrder {
                                    player: PlayerId::new(seat),
                                    card,
                                })
                                .map_err(|e| rerr(move_id, format!("starting-order reveal refused: {e}")))?;
                            self.report.reveal_events += 1;
                        }
                        self.apply(Move::FulfillOrder { order_id: card }, vec![], move_id)?;
                    }
                    Fact::AchievementClaimed { seat: s, category, .. } if *s == seat => {
                        let category = *category;
                        self.consume_drive();
                        let ach = self.state().achievements[category].achievement_type;
                        self.apply(Move::FulfillAchievement { achievement: ach }, vec![], move_id)
                            .map_err(|e| {
                                rerr(
                                    move_id,
                                    format!(
                                        "claimed category {category} could not be fulfilled as \
                                         {ach:?} — if this category was a HYPOTHESIS slot the \
                                         other candidate is the answer: {e}"
                                    ),
                                )
                            })?;
                    }
                    Fact::Mercenary { seat: s, tile, .. } if *s == seat => {
                        let tile = *tile;
                        self.consume_drive();
                        let target = self.area_id(tile, move_id)?;
                        let countries: Vec<_> = self
                            .session
                            .legal()
                            .into_iter()
                            .filter_map(|m| match m {
                                Move::UseMercenary { country, target_area } if target_area == target => {
                                    Some(country)
                                }
                                _ => None,
                            })
                            .collect();
                        match countries.as_slice() {
                            [one] => {
                                let mv = Move::UseMercenary { country: *one, target_area: target };
                                self.apply(mv, vec![], move_id)?;
                            }
                            [] => return Err(rerr(move_id, format!("no legal mercenary use at {tile:?}"))),
                            several => {
                                // BGA does not model WHICH country's agent is
                                // spent — its `fa_tokens`/`fa_used` counters
                                // and the "uses a foreign agent" notification
                                // are country-less, while the engine tracks
                                // per-country tokens (the reveal-fed faces).
                                // The choice is provably SCORE-INERT: the
                                // remainder bonus and Tokens5 count agents,
                                // and the country set bonus reads completed
                                // orders + technologies only (scoring.rs
                                // `calculate_country_bonus`) — never agent
                                // tokens. Pick DETERMINISTICALLY (the smallest
                                // by `Country` order, so batch and follow stay
                                // byte-identical) and DECLARE it; the one way
                                // the pick could matter — a later use needing
                                // the exact country this spent — refuses at
                                // that move and STOPs the replay, the designed
                                // backstop.
                                let mut sorted = several.to_vec();
                                sorted.sort();
                                let pick = sorted[0];
                                self.report.notes.push(format!(
                                    "move {move_id}: ASSUMPTION (declared inert) — BGA's foreign \
                                     agents are country-less counters, so which of the engine's \
                                     {sorted:?} tokens was spent at {tile:?} is unknowable from \
                                     the capture; spent {pick:?} (smallest by Country order, \
                                     deterministic). Score-inert: agents score by COUNT and the \
                                     country set bonus reads orders+techs only; a later \
                                     country-specific use this pick broke would STOP the replay."
                                ));
                                let mv = Move::UseMercenary { country: pick, target_area: target };
                                self.apply(mv, vec![], move_id)?;
                            }
                        }
                    }
                    _ => {
                        // Nothing of ours here — close the additional phase.
                        self.apply(Move::EndAdditionalPhase, vec![], move_id)?;
                    }
                }
            }
            TurnPhase::Placement => match &next.fact {
                Fact::Placement { seat: s, option_b, areas } if *s == seat => {
                    let (option_b, areas) = (*option_b, areas.clone());
                    self.consume_drive();
                    let mv = if option_b {
                        Move::PlaceTwoSameArea {
                            area: self.area_id(areas[0], move_id)?,
                            adjacent_bonus: None,
                        }
                    } else {
                        let mut ids = Vec::new();
                        for a in &areas {
                            ids.push(self.area_id(*a, move_id)?);
                        }
                        ids.sort_by_key(|a| a.0);
                        Move::PlaceThreeAreas { placements: ids }
                    };
                    self.apply(mv, vec![], move_id)?;
                }
                _ => {
                    // The seat placed nothing this turn.
                    self.apply(Move::PlaceThreeAreas { placements: vec![] }, vec![], move_id)?;
                }
            },
            TurnPhase::Movement => match &next.fact {
                Fact::President { seat: s, dest, via, .. } if *s == seat => {
                    let dest = *dest;
                    let via = *via;
                    let fees = self.pass_fee_count_window();
                    self.consume_drive();
                    let dest_id = self.area_id(dest, move_id)?;
                    let in_hand = matches!(
                        self.state().players[seat as usize].president_location,
                        PresidentLocation::InHand
                    );
                    // The `via` dispatch. A residual the fix wave records
                    // rather than closes (T1 review MUST-1's mutation C): a
                    // dispatch that routed `Tram` through the fee-count
                    // matcher is behaviorally invisible at 2P, where price
                    // and fee count coincide — the seam pins below catch the
                    // naive SELECTIONS, and the end-to-end catch for the
                    // ROUTING is Task 2's replay of m128/m137 once the
                    // resolve-stage hold on this capture clears.
                    let mv = if in_hand {
                        match via {
                            PresidentVia::Placed => {
                                Move::PlacePresident { destination: dest_id }
                            }
                            // Unobserved shape, refused rather than re-read
                            // as a placement (T1 review SHOULD-2): both
                            // surviving tram facts move an ON-BOARD
                            // President, and a silent PlacePresident here
                            // could charge a placement fee the evidence
                            // never showed. Untested-unreachable arm, the
                            // T3-NOTE-4 posture: spelled out, never `_`.
                            PresidentVia::Tram => {
                                return Err(rerr(
                                    move_id,
                                    format!(
                                        "a `trams` president move to {dest:?} with the President \
                                         IN HAND is outside the observed set — both surviving \
                                         tram facts (m128, m137) move an on-board President; \
                                         hard error, never guessed"
                                    ),
                                ));
                            }
                        }
                    } else {
                        match via {
                            PresidentVia::Placed => self.walked_president_move(
                                seat, dest, dest_id, fees, move_id,
                            )?,
                            PresidentVia::Tram => {
                                self.tram_president_move(seat, dest_id, fees, move_id)?
                            }
                        }
                    };
                    self.apply(mv, vec![], move_id)?;
                }
                Fact::ReturnPresident { seat: s } if *s == seat => {
                    self.consume_drive();
                    self.apply(
                        Move::ReturnPresidentToHand { recover: BTreeMap::new() },
                        vec![],
                        move_id,
                    )?;
                }
                other => {
                    return Err(rerr(
                        move_id,
                        format!(
                            "engine expects seat {seat}'s Movement but the next fact is {other:?} — \
                             a no-move turn shape the corpus never showed"
                        ),
                    ))
                }
            },
            TurnPhase::AreaAction => self.step_area_action(&next, move_id)?,
            TurnPhase::FivePowerBonus => match &next.fact {
                Fact::PowerToken { seat: s, face } if *s == seat => {
                    let face = *face;
                    self.consume_drive();
                    // Verify the resolved face is what the engine will grant.
                    if let PresidentLocation::OnBoard(area) =
                        self.state().players[seat as usize].president_location
                    {
                        let on_board =
                            self.state().board_layout.five_power_tokens.get(&area).copied().flatten();
                        if on_board != Some(face) {
                            return Err(rerr(
                                move_id,
                                format!(
                                    "powerBonus face {face:?} but the engine board holds {on_board:?} \
                                     at {area:?} — token-face resolution diverges"
                                ),
                            ));
                        }
                    }
                    self.apply(Move::Take5PowerToken, vec![], move_id)?;
                }
                _ => self.apply(Move::Skip5PowerBonus, vec![], move_id)?,
            },
            TurnPhase::Building => match &next.fact {
                Fact::BuildShop { seat: s, tile, reward } if *s == seat => {
                    let (tile, reward) = (*tile, *reward);
                    self.consume_drive();
                    let area = self.area_id(tile, move_id)?;
                    let site = self
                        .state()
                        .board_layout
                        .building_sites
                        .get(&area)
                        .cloned()
                        .ok_or_else(|| rerr(move_id, format!("no building site at {tile:?}")))?;
                    let slot = site
                        .spaces
                        .iter()
                        .position(|sp| *sp == reward)
                        .ok_or_else(|| {
                            rerr(
                                move_id,
                                format!(
                                    "assigned site at {tile:?} has no space rewarding {reward:?} — \
                                     site resolution diverges"
                                ),
                            )
                        })?;
                    self.apply(Move::BuildShophouse { slot: slot as u8 }, vec![], move_id)?;
                }
                _ => self.apply(Move::SkipBuilding, vec![], move_id)?,
            },
            phase => {
                return Err(rerr(
                    move_id,
                    format!("engine surfaced decision point in unexpected phase {phase:?}"),
                ))
            }
        }
        Ok(true)
    }

    fn step_choice(&mut self, choice: PendingChoice, next: &TimedFact) -> Result<(), ImportError> {
        let move_id = next.move_id;
        let seat = self.current_seat();
        match choice {
            PendingChoice::AssistantMove => match &next.fact {
                Fact::AssistMove { seat: s, from, to: Some(to) } if *s == seat => {
                    let (from, to) = (*from, *to);
                    self.consume_drive();
                    let from = match from {
                        None => MoveSource::Hand,
                        Some(a) => MoveSource::Area(self.area_id(a, move_id)?),
                    };
                    let to = MoveTarget::Area(self.area_id(to, move_id)?);
                    self.apply(Move::ChooseAssistantMove { from, to }, vec![], move_id)?;
                }
                Fact::AssistMove { seat: s, from: Some(from), to: None } if *s == seat => {
                    // BGA's board→hand icon leg, translated DIRECTLY since
                    // rev 6 (owner ruling D3, 2026-08-08): the engine's
                    // movement icon offers `MoveTarget::Hand` for an on-board
                    // source. This retires BOTH rev-5 shapes — the two-leg
                    // composition (m106+107, area → hand → area collapsed into
                    // one icon + decline; the second leg now replays as its own
                    // icon with `MoveSource::Hand`) and the m154 one-leg STOP
                    // (census §4.2), which was exactly a to-hand move with no
                    // continuation.
                    let from = *from;
                    self.consume_drive();
                    let from = MoveSource::Area(self.area_id(from, move_id)?);
                    self.apply(
                        Move::ChooseAssistantMove { from, to: MoveTarget::Hand },
                        vec![],
                        move_id,
                    )?;
                }
                _ => self.apply(Move::DeclineAssistantMove, vec![], move_id)?,
            },
            PendingChoice::WarehouseTake => match &next.fact {
                Fact::WarehousePull { seat: s, assistants, shophouses, trading_houses } if *s == seat => {
                    let (a, sh, th) = (*assistants, *shophouses, *trading_houses);
                    self.consume_drive();
                    let item = match (a, sh, th) {
                        (1, 0, 0) => WarehouseItem::Assistant,
                        (0, 1, 0) => WarehouseItem::Shophouse,
                        (0, 0, 1) => WarehouseItem::TradingHouse,
                        other => {
                            return Err(rerr(
                                move_id,
                                format!("warehouse take of {other:?} does not name exactly one item"),
                            ))
                        }
                    };
                    self.apply(Move::ChooseWarehouseTake { item }, vec![], move_id)?;
                }
                _ => self.apply(Move::DeclineWarehouseTake, vec![], move_id)?,
            },
            PendingChoice::ChurchSpace { .. } => match &next.fact {
                Fact::ChurchSpace { seat: s, vp, moves } if *s == seat => {
                    let (vp, moves) = (*vp, *moves);
                    self.consume_drive();
                    let space = (4..=11u8)
                        .find(|f| ChurchTrack::vp_reward(*f) == vp && ChurchTrack::movement_rider(*f) == moves)
                        .ok_or_else(|| {
                            rerr(move_id, format!("no church space grants {vp} VP with {moves} move(s)"))
                        })?;
                    self.apply(Move::ChooseChurchSpace { space }, vec![], move_id)?;
                }
                other => {
                    return Err(rerr(
                        move_id,
                        format!("engine expects a church-space choice; next fact is {other:?}"),
                    ))
                }
            },
            PendingChoice::CustomsSpace { .. } => {
                let mv = self.customs_space_move(move_id)?;
                self.apply(mv, vec![], move_id)?;
            }
            other => {
                return Err(rerr(
                    move_id,
                    format!(
                        "pending choice {other:?} has no translator arm (unobserved in the corpus — \
                         hard error, never guessed)"
                    ),
                ))
            }
        }
        Ok(())
    }

    fn step_area_action(&mut self, next: &TimedFact, move_id: u64) -> Result<(), ImportError> {
        let seat = self.current_seat();
        // Classify the pending action area from the LEGAL LIST, not from the
        // president's location — a mercenary sub-turn acts on the mercenary's
        // target area while the president stands elsewhere (measured at m97).
        let legal = self.session.legal();
        let mut has_port = false;
        let mut has_lab = false;
        let mut has_church = false;
        let mut has_customs = false;
        let mut has_ea = false;
        let mut has_chinatown = false;
        let mut imperative: Vec<Move> = Vec::new();
        for m in &legal {
            if let Move::ExecuteAreaAction { action } = m {
                match action {
                    AreaAction::Port { .. } => has_port = true,
                    AreaAction::Laboratory { .. } => has_lab = true,
                    AreaAction::Church { .. } => has_church = true,
                    AreaAction::Customs { .. } => has_customs = true,
                    AreaAction::EmploymentAgency { .. } => has_ea = true,
                    AreaAction::Chinatown { .. } => has_chinatown = true,
                    AreaAction::Skip => {}
                    _ => imperative.push(m.clone()),
                }
            }
        }
        if has_chinatown {
            return Err(rerr(move_id, "Chinatown action surfaced — unobserved arm (2P board has none)"));
        }
        if has_port {
            return match &next.fact {
                Fact::PortTake { seat: s, .. } if *s == seat => self.step_port_take(move_id),
                _ => self.apply(Move::ExecuteAreaAction { action: AreaAction::Skip }, vec![], move_id),
            };
        }
        if has_lab {
            return match &next.fact {
                Fact::LabTake { seat: s, .. } if *s == seat => self.step_lab_take(move_id),
                Fact::LabDeclined { seat: s } if *s == seat => {
                    self.consume_drive();
                    self.apply(Move::ExecuteAreaAction { action: AreaAction::Skip }, vec![], move_id)
                }
                _ => self.apply(Move::ExecuteAreaAction { action: AreaAction::Skip }, vec![], move_id),
            };
        }
        if has_church {
            return match &next.fact {
                Fact::ChurchPay { seat: s, pays, .. } if *s == seat => {
                    let pays = *pays;
                    self.consume_drive();
                    // The rev-5 D1 offset arm lived here (BGA charged 2 yen,
                    // the engine 1). Since rev 6 the engine's church yen price
                    // IS 2 (owner ruling D1, 2026-08-08) — nothing to
                    // acknowledge, and a wrong charge surfaces at the next
                    // checkpoint as a hard STOP.
                    let mv = Move::ExecuteAreaAction {
                        action: AreaAction::Church {
                            copper: pays.copper,
                            silk: pays.silk,
                            tea: pays.tea,
                            seafood: pays.seafood,
                            yen: pays.yen,
                            imported_good: pays.imported_good,
                        },
                    };
                    self.apply(mv, vec![], move_id)?;
                    // A single eligible space auto-resolves INLINE (the
                    // engine's singleton rule), so BGA's ChurchSpace fact has
                    // no pending choice to meet — consume it here and VERIFY
                    // the engine landed on the space it names (measured at
                    // m153: faith 4, one empty space ≤ 4).
                    if !matches!(
                        self.state().pending_choices.first(),
                        Some(PendingChoice::ChurchSpace { .. })
                    ) {
                        if let Some(TimedFact { fact: Fact::ChurchSpace { seat: s2, vp, moves }, .. }) =
                            self.peek_drive().cloned()
                        {
                            if s2 == seat {
                                self.consume_drive_uncheckpointed();
                                let space = (4..=11u8).find(|f| {
                                    ChurchTrack::vp_reward(*f) == vp
                                        && ChurchTrack::movement_rider(*f) == moves
                                });
                                let landed = self.state().players[seat as usize]
                                    .church_positions
                                    .last()
                                    .copied();
                                if space != landed {
                                    return Err(rerr(
                                        move_id,
                                        format!(
                                            "inline church auto-resolve landed on {landed:?} but \
                                             BGA names space {space:?} — divergence"
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                    Ok(())
                }
                _ => self.apply(Move::ExecuteAreaAction { action: AreaAction::Skip }, vec![], move_id),
            };
        }
        if has_customs {
            return match &next.fact {
                Fact::Customs { seat: s, count, vp } if *s == seat => {
                    let (count, vp) = (*count, *vp);
                    self.consume_drive();
                    self.stash_customs_vp = Some(vp);
                    let mv = Move::ExecuteAreaAction {
                        action: AreaAction::Customs { imported_goods_paid: count },
                    };
                    self.apply(mv, vec![], move_id)
                }
                _ => self.apply(Move::ExecuteAreaAction { action: AreaAction::Skip }, vec![], move_id),
            };
        }
        if has_ea {
            return match &next.fact {
                Fact::WarehousePull { seat: s, assistants, shophouses, trading_houses } if *s == seat => {
                    let (a, sh, th) = (*assistants, *shophouses, *trading_houses);
                    self.consume_drive();
                    let mv = Move::ExecuteAreaAction {
                        action: AreaAction::EmploymentAgency {
                            assistants: a,
                            shophouses: sh,
                            trading_houses: th,
                        },
                    };
                    self.apply(mv, vec![], move_id)
                }
                _ => self.apply(Move::ExecuteAreaAction { action: AreaAction::Skip }, vec![], move_id),
            };
        }
        // A tech refusal at a Laboratory where the engine offers NO take (none
        // affordable) reaches here with `has_lab` false — the legal list is
        // [Skip] alone (first observed at m176: power 3, no affordable card).
        // BGA prints the same refusal whether the take was declined or
        // impossible; Skip is the translation either way, and the checkpoints
        // keep pinning the resulting state.
        if let Fact::LabDeclined { seat: s } = &next.fact {
            if *s == seat {
                self.consume_drive();
                return self.apply(Move::ExecuteAreaAction { action: AreaAction::Skip }, vec![], move_id);
            }
        }
        // Imperative production/Bank action: exactly one legal move.
        match imperative.len() {
            1 => self.apply(imperative.remove(0), vec![], move_id),
            n => Err(rerr(
                move_id,
                format!("expected one imperative area action, found {n}: {imperative:?}"),
            )),
        }
    }

    /// The move of the 5-Power Port bonus BGA is about to book ahead of this
    /// take, if it booked one.
    ///
    /// The bonus arrives as a NOTE fact (`PortAccess` with a nonzero
    /// `vp_bonus`), so it is skipped rather than consumed — and it has to be
    /// read BEFORE the take's `consume_drive`, because that call is what runs
    /// the checkpoint on the bonus's own move. Scanning forward from the cursor
    /// to the next drive fact is exactly the window of notes that call will
    /// skip.
    /// Returns `(bonus_move, take_move)`.
    fn pending_port_bonus(&self, seat: u8) -> Option<(u64, u64)> {
        let next = self.next_drive_index(self.cursor)?;
        let take_move = self.facts[next].move_id;
        self.facts[self.cursor..next]
            .iter()
            .find(|tf| {
                matches!(tf.fact, Fact::PortAccess { seat: s, vp_bonus } if s == seat && vp_bonus > 0)
            })
            .map(|tf| (tf.move_id, take_move))
    }

    fn step_port_take(&mut self, move_id: u64) -> Result<(), ImportError> {
        let seat = self.current_seat();
        // Mark the bonus boundary BEFORE consuming: `consume_drive` is the call
        // that runs the checkpoint standing on it, so marking afterwards would
        // be marking a boundary that has already stopped the replay.
        if let Some((bonus, take)) = self.pending_port_bonus(seat) {
            self.mid_composite_moves.extend(port_composite_interior(Some(bonus), take, None));
        }
        // First take fact.
        let Some(first) = self.consume_drive() else {
            return Err(self.drive_absence(move_id, "port take fact vanished"));
        };
        let Fact::PortTake { card: card1, .. } = first.fact else {
            return Err(rerr(move_id, "port take arm consumed a non-take fact"));
        };
        // Second take / refusal, if adjacent in the stream.
        let mut card2: Option<Option<yokohama_core::OrderCardId>> = None;
        let mut payment: Option<SecondPayment> = None;
        match self.peek_drive().cloned() {
            Some(TimedFact { fact: Fact::PortTake { seat: s, card, second: Some(pay) }, move_id: m2, .. })
                if s == seat =>
            {
                self.consume_drive_uncheckpointed();
                card2 = Some(card);
                payment = Some(pay);
                // The pair can SPAN BGA moves (measured at m169/m170) — every
                // boundary strictly inside the span carries counters for a
                // state the atomic engine Port move steps over.
                self.mid_composite_moves
                    .extend(port_composite_interior(None, first.move_id, Some(m2)));
            }
            Some(TimedFact { fact: Fact::PortSecondDeclined { seat: s }, move_id: m2, .. }) if s == seat => {
                self.consume_drive_uncheckpointed();
                self.mid_composite_moves
                    .extend(port_composite_interior(None, first.move_id, Some(m2)));
            }
            _ => {}
        }
        // Resolve identities: socket takes name their card; thin takes go
        // through ownership + next-observation pruning (census §3).
        let need = 1 + card2.is_some() as usize;
        let mut cards: Vec<yokohama_core::OrderCardId> = Vec::new();
        let named: Vec<Option<yokohama_core::OrderCardId>> = match card2 {
            Some(c2) => vec![card1, c2],
            None => vec![card1],
        };
        if named.iter().all(|c| c.is_some()) {
            cards = named.into_iter().map(|c| c.expect("checked")).collect();
        } else if named.iter().all(|c| c.is_none()) {
            let candidates = self.thin_take_candidates(seat, move_id);
            if candidates.len() != need {
                return Err(rerr(
                    move_id,
                    format!(
                        "thin port take needs exactly {need} candidate key(s); evidence leaves \
                         {candidates:?} — underdetermined, never guessed"
                    ),
                ));
            }
            for key in &candidates {
                cards.push(self.resolved.key_model.order_card(*key)?);
            }
        } else {
            return Err(rerr(move_id, "port take pair mixes named and unnamed cards — unmeasured shape"));
        }
        // Shadow: remove the taken keys.
        let taken_keys: Vec<u64> = cards
            .iter()
            .filter_map(|c| {
                self.port_shadow
                    .iter()
                    .copied()
                    .find(|k| self.resolved.key_model.order_cards.get(k) == Some(c))
            })
            .collect();
        if taken_keys.len() != cards.len() {
            return Err(rerr(move_id, "taken card not on the shadow display — key model diverges"));
        }
        self.port_shadow.retain(|k| !taken_keys.contains(k));
        let second_card_payment = payment.map(|p| match p {
            SecondPayment::TwoYen => PortPayment::TwoYen,
            SecondPayment::OneImport => PortPayment::OneImportedGood,
        });
        // The semantic identity of a Port take is the card SET plus the
        // second-card payment; the engine enumerates ONE canonical order per
        // set, and BGA's notification order need not match it (first observed
        // at m169: BGA named [O02, O08], the engine's move carries [O08, O02]).
        // Match the legal list by set, falling back to the constructed order
        // (whose refusal carries the diagnostic) when no set-match exists.
        let mut cards_taken = cards.clone();
        let set = |v: &[yokohama_core::OrderCardId]| {
            let mut s = v.to_vec();
            s.sort_by_key(|c| c.0);
            s
        };
        let want = set(&cards);
        if let Some(canonical) = self.session.legal().into_iter().find_map(|m| match m {
            Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken: ct, second_card_payment: p } }
                if p == second_card_payment && set(&ct) == want =>
            {
                Some(ct)
            }
            _ => None,
        }) {
            cards_taken = canonical;
        }
        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Port { cards_taken, second_card_payment },
        };
        let reveals = self.reveals_for(&mv, move_id)?;
        self.apply(mv, reveals, move_id)
    }

    fn step_lab_take(&mut self, move_id: u64) -> Result<(), ImportError> {
        let Some(tf) = self.consume_drive() else {
            return Err(self.drive_absence(move_id, "lab take fact vanished"));
        };
        let Fact::LabTake { tech_name, tech_id_key, .. } = tf.fact else {
            return Err(rerr(move_id, "lab take arm consumed a non-take fact"));
        };
        // Which display key? Socket names it; thin joins by ledger title and
        // class over the shadow display.
        let key = match tech_id_key {
            Some((_, key)) => key,
            None => {
                let ledger = super::names::TechLedger::load(self.session.rules())
                    .map_err(ImportError::Map)?;
                let mut candidates = Vec::new();
                for k in &self.lab_shadow {
                    let tech_id = self.resolved.key_model.tech_id(*k)?;
                    match ledger.lookup(tech_id) {
                        super::names::TechLookup::Known(r) if r.title == tech_name => {
                            candidates.push(*k)
                        }
                        super::names::TechLookup::Known(_) => {}
                        super::names::TechLookup::Unknown { .. } => {
                            // An unknown-titled display card could be the named
                            // one only if its class matches the named title's.
                            let class = self.resolved.key_model.tech_classes.get(&tech_id);
                            let engine_class = self
                                .session
                                .rules()
                                .techs
                                .iter()
                                .find(|c| c.name == tech_name)
                                .map(|c| {
                                    (super::orders::engine_country_code(c.country), c.industry_cost)
                                });
                            if class.copied() == engine_class {
                                candidates.push(*k);
                            }
                        }
                    }
                }
                match candidates.as_slice() {
                    [one] => *one,
                    other => {
                        return Err(rerr(
                            move_id,
                            format!(
                                "thin lab take of `{tech_name}` matches display keys {other:?} — \
                                 not exactly one; underdetermined"
                            ),
                        ))
                    }
                }
            }
        };
        let card = *self
            .resolved
            .tech_cards
            .get(&key)
            .ok_or_else(|| rerr(move_id, format!("tech_key {key} has no resolved engine card")))?;
        self.lab_shadow.retain(|k| *k != key);
        // Payment variant: one legal mix needs nothing; several are settled by
        // BGA's counters, or refused (see `resolve_lab_payment`).
        let variants: Vec<Move> = self
            .session
            .legal()
            .into_iter()
            .filter(|m| {
                matches!(m, Move::ExecuteAreaAction { action: AreaAction::Laboratory { card_id, .. } } if *card_id == card)
            })
            .collect();
        if variants.is_empty() {
            return Err(rerr(move_id, format!("no legal Laboratory take of {card:?}")));
        }
        let seat = self.current_seat();
        let spend = self.observed_spend(seat, move_id);
        let mv = match resolve_lab_payment(variants, spend) {
            LabPayment::Determined(one) => one,
            LabPayment::Derived { chosen, chosen_label, candidates } => {
                let (yen, imports) = spend.expect("a derivation implies a counter window");
                self.report.notes.push(format!(
                    "move {move_id}: DERIVATION (declared, counter-evidenced) — the capture names \
                     no lab payment, and {} legal mixes debit different resources for the same \
                     industry value [{}]. BGA's own counters bracketing this move show seat \
                     {seat} spending {yen} yen and {imports} {}, which fits exactly one: took \
                     {chosen_label}. `yens` and `imports` for seat {seat} are therefore NOT \
                     independently cross-checked at this move — they selected it; every other \
                     counter's T9 check still runs here, and the mix taken is one the engine \
                     itself offered.",
                    candidates.len(),
                    candidates.join(" | "),
                    if imports == 1 { "import" } else { "imports" },
                ));
                chosen
            }
            LabPayment::NoneConsistent { spend, candidates } => {
                return Err(rerr(
                    move_id,
                    format!(
                        "lab payment divergence for {card:?}: BGA's counters show seat {seat} \
                         spending {} yen and {} import(s) across this move, which NO legal mix \
                         debits — the engine and the capture disagree about what was paid, not \
                         merely about which of several payments it was; the legal mixes were {}",
                        spend.0,
                        spend.1,
                        candidates.join(" | ")
                    ),
                ))
            }
            LabPayment::Underdetermined { candidates } => {
                return Err(rerr(
                    move_id,
                    format!(
                        "ambiguous lab payment variants for {card:?}: the capture names no \
                         payment and {} legal mixes debit different resources for the same \
                         industry value — {}",
                        candidates.len(),
                        candidates.join(" | ")
                    ),
                ))
            }
        };
        let reveals = self.reveals_for(&mv, move_id)?;
        self.apply(mv, reveals, move_id)?;
        // The rev-5 D2 offset arm lived here (BGA charged the extra-tech
        // penalty from the 4TH tech, the engine from the 5th). Since rev 6 the
        // engine's surcharge starts at the 4th card too (owner ruling D2,
        // 2026-08-08 — `extra_tech_payment` from the 4th), so the counters
        // agree raw and any 4th-tech divergence is an ordinary checkpoint STOP.
        Ok(())
    }

    /// Close out the final turn after the last drive fact, gated on BGA's own
    /// end-of-game markers (`stream_declares_game_end`): apply exactly the
    /// absence arms the translator applies mid-stream when the next fact
    /// belongs to a later decision — declines for declinable choices,
    /// Skip/EndAdditionalPhase for the phase closers — until the engine
    /// reaches game end. Anything that is NOT an absence arm (a real
    /// Placement/Movement decision, a choice with no decline) is a STOP: BGA
    /// said the game ended, the engine disagrees, and that divergence is owner
    /// territory, never a guess.
    fn close_out_final_turn(&mut self) -> Result<(), ImportError> {
        let move_id = self.max_move_seen;
        for _ in 0..64 {
            if self.session.is_over() {
                return Ok(());
            }
            if let Some(choice) = self.state().pending_choices.first().cloned() {
                let closer = match choice {
                    PendingChoice::AssistantMove => Move::DeclineAssistantMove,
                    PendingChoice::WarehouseTake => Move::DeclineWarehouseTake,
                    // The customs space is non-declinable but EVIDENCE-BACKED:
                    // its VP rode the already-consumed customsPlacement fact
                    // (see `customs_space_move`). A final-move customs sale
                    // ends the stream with this choice queued — measured live
                    // at table 896157897 m144.
                    PendingChoice::CustomsSpace { .. } if self.stash_customs_vp.is_some() => {
                        self.customs_space_move(move_id)?
                    }
                    other => {
                        return Err(rerr(
                            move_id,
                            format!(
                                "BGA declared the game over but the engine still holds a \
                                 non-declinable pending choice {other:?} — cannot close out"
                            ),
                        ))
                    }
                };
                self.apply(closer, vec![], move_id)?;
                continue;
            }
            let closer = match self.state().turn_phase {
                TurnPhase::AdditionalActionFirst | TurnPhase::AdditionalActionSecond => {
                    Move::EndAdditionalPhase
                }
                TurnPhase::FivePowerBonus => Move::Skip5PowerBonus,
                TurnPhase::Building => Move::SkipBuilding,
                TurnPhase::AreaAction => Move::ExecuteAreaAction { action: AreaAction::Skip },
                phase => {
                    return Err(rerr(
                        move_id,
                        format!(
                            "BGA declared the game over but the engine still expects a real \
                             {phase:?} decision — cannot close out"
                        ),
                    ))
                }
            };
            self.apply(closer, vec![], move_id)?;
        }
        Err(rerr(move_id, "final-turn close-out did not reach game end within 64 moves"))
    }

    /// Drive the whole stream. Returns the report; the session is available
    /// via [`Translator::into_session`].
    pub fn run(&mut self) -> Result<(), ImportError> {
        loop {
            if !self.step()? {
                if let Some(e) = self.pending_checkpoint_error.take() {
                    return Err(e);
                }
                break;
            }
        }
        // The tail: checkpoints after the last consumed fact.
        // (`unchecked_counters` is populated at construction, not here — a
        // STOPPED run's report lists them too.)
        self.run_checkpoints_below(u64::MAX)?;
        // With the game over, the remaining facts are all NOTE facts (the
        // m194-class recover-none corroboration and the endgame markers —
        // every remaining DRIVE fact was either consumed by the is_over
        // cleanup arm or errored there). The tail checkpoint above verified
        // the state they describe, so their move ids join the progress figure:
        // "progressed through BGA move 194" means m194's content was checked,
        // not merely that a drive fact carried that id.
        if self.session.is_over() {
            while self.cursor < self.facts.len() {
                self.max_move_seen = self.max_move_seen.max(self.facts[self.cursor].move_id);
                self.cursor += 1;
            }
        }
        self.report.last_move_id = self.max_move_seen;
        Ok(())
    }

    pub fn into_session(self) -> (Session, ReplayReport) {
        (self.session, self.report)
    }

    pub fn session(&self) -> &Session {
        &self.session
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yokohama_core::{GameConfig, Rules, StartingOrderChoice};

    fn sim_2p(seed: u64) -> Session {
        Session::new_sim(
            GameConfig::for_player_count(2),
            seed,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        )
        .unwrap()
    }

    /// The ENGINE half of the refill-order pin (H2 review SHOULD-1, escaped
    /// mutation M4): the reveal list `Session::apply` consumes is in DRAW
    /// order (`arrange_orders_on_top` puts `reveals[0]` on the deck top), and
    /// `refill_port` fills rightmost-empty-first via `Vec::pop` — so the
    /// FIRST named card must land in the RIGHTMOST refilled slot. This is the
    /// measured fact `refill_draw_keys` derives its direction from; if this
    /// test ever fails, that function's doc comment is wrong before its code
    /// is.
    #[test]
    fn port_refill_reveal_list_is_draw_order_first_named_card_lands_rightmost() {
        let mut s = sim_2p(900);
        let port = *s
            .state()
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == AreaType::PortA)
            .unwrap()
            .0;
        s.apply(Move::EndAdditionalPhase, vec![]).unwrap();
        s.apply(Move::PlaceThreeAreas { placements: vec![port] }, vec![]).unwrap();
        s.apply(Move::PlacePresident { destination: port }, vec![]).unwrap();
        let take = s
            .legal()
            .into_iter()
            .find(|m| {
                matches!(
                    m,
                    Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } }
                        if cards_taken.len() == 2
                )
            })
            .expect("a 2-card port take is available at power 2");
        let before = s.state().clone();
        let n = before.order_deck.len();
        // Draw order: the deck's LAST element is the next `Vec::pop` draw
        // (an identity arrangement, so only the LANDING is under test).
        let (first_draw, second_draw) = (before.order_deck[n - 1], before.order_deck[n - 2]);
        s.apply(take, vec![RecordedEvent::RevealOrders(vec![first_draw, second_draw])]).unwrap();
        let (slots, board_b) = crate::reveal::refilled_port_slots(&before, s.state());
        assert!(!board_b, "2P game refills Port A");
        assert_eq!(slots.len(), 2, "the 2-card take vacates two slots");
        assert!(slots[0] > slots[1], "refilled_port_slots reports draw order: rightmost first");
        assert_eq!(
            s.state().port_state.port_a.slots[slots[0]],
            Some(first_draw),
            "the FIRST revealed card lands in the RIGHTMOST refilled slot"
        );
        assert_eq!(
            s.state().port_state.port_a.slots[slots[1]],
            Some(second_draw),
            "the SECOND revealed card lands in the slot left of it"
        );
    }

    /// Drive a 2P sim by always taking the first legal move until a Movement
    /// decision point offers real `MovePresident` labels — i.e. the President
    /// is already ON the board, which is the only shape a `trams` fact can
    /// arrive in (turn 1 places from hand).
    fn sim_at_a_president_move() -> Session {
        let mut s = sim_2p(900);
        for _ in 0..400 {
            let legal = s.legal();
            if legal.iter().any(|m| matches!(m, Move::MovePresident { .. })) {
                return s;
            }
            // Placements go WIDE (the empty placement is legal and first, and a
            // President with no assistant chain can never walk), and the
            // President never voluntarily goes back to hand.
            let widest = legal
                .iter()
                .filter_map(|m| match m {
                    Move::PlaceThreeAreas { placements } => Some((placements.len(), m)),
                    _ => None,
                })
                .max_by_key(|(n, _)| *n)
                .filter(|(n, _)| *n > 0)
                .map(|(_, m)| m.clone());
            let mv = widest
                .or_else(|| {
                    legal
                        .iter()
                        .find(|m| !matches!(m, Move::ReturnPresidentToHand { .. }))
                        .cloned()
                })
                .expect("a sim always has a legal move");
            s.apply(mv, vec![]).expect("the drive stays inside the legal list");
        }
        panic!("no Movement decision point with the President on board within 400 moves");
    }

    /// The ENGINE guarantee the `Ok` arm depends on, pinned where the matcher
    /// can see it: per destination there is AT MOST ONE zero-cost label, and
    /// at a real Movement point some free label is a WALK with a non-empty
    /// path (the suppression shape a `path.is_empty()` reading would miss).
    ///
    /// Renamed at the T1 fix wave (review SHOULD-3): this test involves no
    /// tram fact and no Tram technology — the tests that DRIVE the tram
    /// selection are `select_tram_move_picks_the_free_walk_…` and its
    /// siblings below.
    #[test]
    fn every_destination_has_at_most_one_zero_cost_label_and_some_free_one_walks() {
        let s = sim_at_a_president_move();
        let seat = s.state().current_player.0;
        let legal = s.legal();
        let destinations: BTreeSet<AreaId> = legal
            .iter()
            .filter_map(|m| match m {
                Move::MovePresident { destination, .. } => Some(*destination),
                _ => None,
            })
            .collect();
        assert!(!destinations.is_empty(), "the drive reached a real Movement decision point");

        let mut free_walks = 0usize;
        for dest_id in &destinations {
            let candidates =
                president_candidates_to(s.state(), s.rules(), &legal, seat, *dest_id);
            assert!(!candidates.is_empty(), "a listed destination has at least one label");
            let free: Vec<&(Move, u32)> =
                candidates.iter().filter(|(_, cost)| *cost == 0).collect();
            assert!(
                free.len() <= 1,
                "the engine emits at most one zero-cost label per destination, saw {}: {:?}",
                free.len(),
                free
            );
            if let Ok(picked) = select_free_president_label(&candidates) {
                // The selected label IS the free one, and it is the move the
                // engine would apply — not a reconstruction.
                assert!(legal.contains(&picked), "the pick comes from the legal list");
                match &picked {
                    Move::MovePresident { path, destination } => {
                        assert_eq!(destination, dest_id);
                        if !path.is_empty() {
                            free_walks += 1;
                        }
                    }
                    other => panic!("the pick must be a president move, got {other:?}"),
                }
            }
        }
        assert!(
            free_walks > 0,
            "at least one free label is a WALK with a non-empty path — the suppression case a \
             `path.is_empty()` matcher would miss"
        );
    }

    /// The refusal shape: the selection REFUSES rather than picking a nearest
    /// label, and the refusal's evidence names every candidate with its price.
    /// `Err(n)` carries the count so the caller can say which of the two things
    /// went wrong — no free label at all, or more than one (which would mean
    /// the engine's per-destination uniqueness broke).
    #[test]
    fn a_tram_fact_with_no_unique_free_label_refuses_and_names_what_it_saw() {
        let a = AreaId::new(1);
        let b = AreaId::new(2);
        let tolled = |path: Vec<AreaId>, cost: u32| {
            (Move::MovePresident { path, destination: b }, cost)
        };

        // Nothing free: every route to the destination costs something.
        let none_free = vec![tolled(vec![a, b], 1), tolled(vec![b], 2)];
        assert_eq!(select_free_president_label(&none_free), Err(0));
        let described = describe_candidates(&none_free);
        assert!(described.contains("costing 1") && described.contains("costing 2"), "{described}");

        // Two free labels: refuse rather than take the first.
        let two_free = vec![tolled(vec![a, b], 0), tolled(vec![b], 0)];
        assert_eq!(select_free_president_label(&two_free), Err(2));

        // Exactly one free label among priced rivals: that one, not the cheapest.
        let one_free = vec![tolled(vec![a, b], 3), tolled(vec![b], 0)];
        assert_eq!(
            select_free_president_label(&one_free),
            Ok(Move::MovePresident { path: vec![b], destination: b })
        );

        assert_eq!(describe_candidates(&[]), "none");
    }

    /// **THE SEAM PIN, suppressed side** (T1 review MUST-1): drive
    /// [`select_tram_move`] itself — not its helpers — at every destination
    /// whose free label is the WALK the engine kept after suppressing the
    /// teleport, and require it to return exactly that walk. A selection
    /// re-written as `path.is_empty()` (the naive "Tram is a teleport"
    /// reading, the review's escaped mutation A) refuses every one of these
    /// and fails this test's anti-vacuity floor.
    #[test]
    fn select_tram_move_picks_the_free_walk_the_engine_kept_after_suppression() {
        let s = sim_at_a_president_move();
        let seat = s.state().current_player.0;
        let legal = s.legal();
        let mut suppressed_walks = 0usize;
        for dest_id in president_destinations(&legal) {
            let candidates =
                president_candidates_to(s.state(), s.rules(), &legal, seat, dest_id);
            let free: Vec<&(Move, u32)> =
                candidates.iter().filter(|(_, cost)| *cost == 0).collect();
            if free.len() != 1 {
                continue;
            }
            let Move::MovePresident { path, .. } = &free[0].0 else { continue };
            if path.is_empty() {
                continue; // a genuine teleport — the sibling test's subject
            }
            let picked = select_tram_move(
                s.state(),
                s.rules(),
                &legal,
                seat,
                dest_id,
                0,
                4242,
            )
            .expect("a destination with a free walk must select");
            assert_eq!(
                picked,
                free[0].0.clone(),
                "the selection is the engine's surviving free WALK, path and all",
            );
            suppressed_walks += 1;
        }
        assert!(
            suppressed_walks > 0,
            "anti-vacuity: at least one destination exercises the suppressed-into-walk shape",
        );
    }

    /// **THE SEAM PIN, teleport side**: grant the acting seat the Tram
    /// technology on a cloned state, re-enumerate, and every destination
    /// whose free label is the TELEPORT (empty path — nothing free walks
    /// there) must come back from [`select_tram_move`] as that teleport.
    /// Together with the suppressed-side pin this kills the naive selections
    /// in both directions.
    #[test]
    fn select_tram_move_picks_the_teleport_where_no_free_walk_exists() {
        let s = sim_at_a_president_move();
        let seat = s.state().current_player.0;
        let rules = s.rules();
        let tram_id = rules
            .techs
            .iter()
            .find(|t| t.effect == yokohama_core::TechEffect::Tram)
            .expect("the standard registry carries the Tram technology")
            .id;
        let mut state = s.state().clone();
        if !state.players[seat as usize].technologies.contains(&tram_id) {
            state.players[seat as usize].technologies.push(tram_id);
        }
        let legal = yokohama_core::get_legal_moves(&state, rules);
        let mut teleports = 0usize;
        for dest_id in president_destinations(&legal) {
            let candidates = president_candidates_to(&state, rules, &legal, seat, dest_id);
            let free: Vec<&(Move, u32)> =
                candidates.iter().filter(|(_, cost)| *cost == 0).collect();
            if free.len() != 1 {
                continue;
            }
            let Move::MovePresident { path, .. } = &free[0].0 else { continue };
            if !path.is_empty() {
                continue; // the suppressed shape — the sibling test's subject
            }
            let picked = select_tram_move(
                &state,
                rules,
                &legal,
                seat,
                dest_id,
                0,
                4242,
            )
            .expect("a destination whose free label is the teleport must select");
            assert_eq!(picked, free[0].0.clone(), "the selection is the teleport, empty path");
            teleports += 1;
        }
        assert!(
            teleports > 0,
            "anti-vacuity: granting Tram must surface at least one unsuppressed teleport",
        );
    }

    /// **THE SEAM PIN, refusal side**: driven through [`select_tram_move`]
    /// itself. (a) A tram fact carrying observed pass-through fees refuses
    /// BEFORE any selection — Tram moves for free, and absorbing fee evidence
    /// into a zero-cost pick would discard a contradiction (T1 review
    /// SHOULD-1). (b) A destination with no free label refuses naming the
    /// move id, the destination and every candidate with its price.
    #[test]
    fn select_tram_move_refuses_fees_and_no_free_label_naming_what_it_saw() {
        let s = sim_at_a_president_move();
        let seat = s.state().current_player.0;
        let legal = s.legal();
        let dests: Vec<AreaId> = president_destinations(&legal);
        assert!(!dests.is_empty());

        // (a) the fee gate, on any real destination.
        let err = select_tram_move(
            s.state(),
            s.rules(),
            &legal,
            seat,
            dests[0],
            2,
            777,
        )
        .expect_err("observed fees contradict a free move");
        let text = err.to_string();
        assert!(text.contains("2 observed pass-through fee(s)"), "{text}");
        assert!(text.contains("777"), "the refusal names its move id: {text}");

        // (b) no free label: an area that is NOT a legal destination at all
        // (here: unreachable at this point) has an empty candidate set, which
        // is the same Err(0) arm a fully-priced destination reaches — refused
        // naming the move id and what was considered, never a nearest pick.
        // (The priced-rivals wording is pinned by value in the
        // `describe_candidates` rows of the refusal-shape test above.)
        let listed: BTreeSet<AreaId> = dests.iter().copied().collect();
        let unreachable = s
            .state()
            .board_layout
            .areas
            .keys()
            .copied()
            .find(|id| !listed.contains(id))
            .expect("some board area is not a legal destination at this point");
        let err = select_tram_move(
            s.state(),
            s.rules(),
            &legal,
            seat,
            unreachable,
            0,
            888,
        )
        .expect_err("no free label must refuse, never nearest-pick");
        let text = err.to_string();
        assert!(text.contains("no free (zero-cost) legal label"), "{text}");
        assert!(text.contains("none"), "an empty candidate set is named as such: {text}");
        assert!(text.contains("888"), "{text}");
    }

    /// Every `MovePresident` destination in a legal list, deduplicated.
    fn president_destinations(legal: &[Move]) -> Vec<AreaId> {
        let set: BTreeSet<AreaId> = legal
            .iter()
            .filter_map(|m| match m {
                Move::MovePresident { destination, .. } => Some(*destination),
                _ => None,
            })
            .collect();
        set.into_iter().collect()
    }

    /// The TRANSLATOR half of the same pin: given the engine fact above
    /// (`reveals[0]` lands rightmost) and BGA's display mechanics (census §1:
    /// slide-left, then ascending keys refill the vacated right end — the
    /// order `port_shadow`'s ascending append mirrors), only a DESCENDING
    /// draw list puts ascending keys in ascending slots. Reversing this list
    /// changes nothing the corpus checkpoints can see — the escaped mutation
    /// M4 — but silently mis-arranges the engine's display, and slot
    /// arrangement carries LOW/MID/HIGH zone take-power gating, so a wrong
    /// order could flip a take's legality in a future corpus.
    #[test]
    fn refill_draw_keys_descend_so_ascending_keys_land_left_to_right() {
        assert_eq!(refill_draw_keys(38, 2), vec![39, 38]);
        assert_eq!(refill_draw_keys(7, 3), vec![9, 8, 7]);
        assert_eq!(refill_draw_keys(13, 1), vec![13]);
        assert!(refill_draw_keys(1, 0).is_empty());
    }

    /// H2 review SHOULD-2: `unchecked_counters` must ride the report from
    /// CONSTRUCTION, not only `run()`'s Ok tail — the headline acceptance run
    /// STOPS at m154, and T9's rule ("a key we deliberately do not model is
    /// listed explicitly with its reason") has to hold on exactly the run
    /// whose report matters most.
    #[test]
    fn unchecked_counters_ride_the_report_from_construction_not_only_the_ok_path() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
        ))
        .unwrap();
        let rules = Rules::standard();
        let feed = yokohama_bridge::feed::read_and_collapse(&text, None).unwrap();
        let answers = super::super::setup::synthesize(&feed, &rules).unwrap();
        let mapper = super::super::orders::OrderMapper::from_rules(&rules).unwrap();
        let seats = answers.seats.clone();
        let stream = super::super::facts::extract(
            &feed,
            &|pid| seats.iter().find(|s| s.bga_player_id == pid).map(|s| s.seat),
            &mapper,
        )
        .unwrap();
        let resolved = super::super::resolve::resolve(answers, &feed, &stream, &rules).unwrap();
        let session = super::super::setup::build_session(&resolved.answers, rules).unwrap();
        let t = Translator::new(session, &resolved, stream, &feed).unwrap();
        // F2/V7 (lens-2): the pin is the EXACT key SET, not `.len()` against the
        // very constants that build it. Comparing the report's length to
        // `UNCHECKED_COUNTERS.len() + UNCHECKED_EXTRA.len()` is self-referential
        // — it can never disagree — so silently GROWING the unchecked list (a
        // counter dropped from the T9 cross-check) passed the whole suite. A
        // literal set makes adding or removing a key a failing test that forces
        // a review of WHY that counter is no longer cross-checked.
        let keys: Vec<&str> = t.report.unchecked_counters.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            [
                "player_id", "gears", "prod_count", "comm_count",
                "flags_gb", "flags_us", "flags_fr", "flags_ge", "flags_ne",
            ],
            "the deliberately-unchecked key set is EXACTLY these, in this order"
        );
    }

    // The F3 classifier test (`the_extra_tech_delta_classifier_admits_only_the_
    // penalty_vocabulary`) died with the D2 offset machinery at rev 6: the
    // engine now charges the surcharge from the 4th card itself, so there is
    // no absorber left whose vocabulary needed bounding. The corpus-level
    // negative (`a_perturbed_checkpoint_counter_stops_the_replay_nothing_
    // absorbs_it` in tests/bga_import.rs) pins that a counter divergence at
    // the 4th-tech move — or anywhere else — is now a plain T9 STOP.

    /// D6 AT THE TRANSLATOR BOUNDARY, pinned in BOTH directions (live gate
    /// table 896157897 m49). Rulebook p.13/p.14 lets the player source a
    /// Church/Customs track pawn "from the [Church/Customs] Area or your
    /// hand"; the owner RULED area-first for BGA parity and rev 7 landed the
    /// flip in `take_church_or_customs_assistant` (tracks.rs). Mid-turn that
    /// is observable — an area-sourced take leaves the board total unchanged
    /// (a pawn moves area → track, both counted as board) and the hand
    /// untouched, where hand-first would run one board HIGH and one hand LOW
    /// until Recovery reconverges them — so a BGA checkpoint inside the take
    /// window discriminates the two rules, and this test drives one of each:
    /// the AREA-sourced counters BGA actually reports must replay CLEAN, and
    /// the HAND-sourced counters must STOP. Both arms fail against a
    /// hand-first engine (the first stops, the second passes), which is what
    /// makes this a D6 pin rather than a plumbing smoke.
    ///
    /// This REPLACES the rev-6-era source-rule STOP recorder, whose own doc
    /// named this moment: it annotated exactly this divergence while the
    /// engine was hand-first and could not arise once rev 7 landed. Keeping
    /// it would have been worse than dead — its numeric signature would
    /// eventually have attached D6's story to some unrelated counter diff.
    ///
    /// The live capture is gitignored, so the trigger is scripted
    /// synthetically: Option-B two on Church, president onto Church (power
    /// 3), church pay with the yen type (start money 3 covers the 2-yen
    /// price, faith 3+1=4 lands the one eligible space inline), then the
    /// checkpoint. What only the live run proves — that BGA's counters match
    /// an area-first engine — is the m49 observation itself, and the whole
    /// 896157897 replay at rev 7 is its standing evidence.
    #[test]
    fn the_track_take_sources_from_the_area_and_a_hand_sourced_checkpoint_stops() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
        ))
        .unwrap();
        let rules = Rules::standard();
        let feed = yokohama_bridge::feed::read_and_collapse(&text, None).unwrap();
        let answers = super::super::setup::synthesize(&feed, &rules).unwrap();
        // A fully synthetic drive script (seat 0 is the excerpt's start
        // player): the facts are constructed directly rather than parsed, so
        // the committed fixture needs no capture bytes. `FactStream` is not
        // Clone, and each arm consumes one, so the script is a builder.
        let script = || FactStream {
            facts: vec![
                TimedFact {
                    move_id: 4,
                    kind: "assistantPlacement".to_string(),
                    fact: Fact::Placement {
                        seat: 0,
                        option_b: true,
                        areas: vec![AreaType::Church],
                    },
                },
                TimedFact {
                    move_id: 5,
                    kind: "presidentPlacement".to_string(),
                    fact: Fact::President {
                        seat: 0,
                        dest: AreaType::Church,
                        power: 3,
                        via: PresidentVia::Placed,
                    },
                },
                TimedFact {
                    move_id: 6,
                    kind: "churchPlacement".to_string(),
                    fact: Fact::ChurchPay {
                        seat: 0,
                        faith_added: 1,
                        pays: super::super::facts::ChurchPays {
                            yen: true,
                            yen_amount: 2,
                            ..Default::default()
                        },
                    },
                },
            ],
            checkpoints: Vec::new(),
        };
        let resolved = super::super::resolve::resolve(answers, &feed, &script(), &rules).unwrap();
        let fresh_session =
            || super::super::setup::build_session(&resolved.answers, rules.clone()).unwrap();
        let s0_hand = fresh_session().state().players[0].assistants.in_hand;
        let pid = resolved.answers.seats[0].bga_player_id.clone();
        // `yens` down by the 2-yen church price is the payment anchor that
        // keeps either checkpoint from matching the PRE-take state, whose
        // board/hand counts read the same as the area-sourced post-take ones.
        let checkpoint = |board: i64, hand: i64| {
            vec![super::super::facts::CounterCheckpoint {
                move_id: 7,
                per_player: BTreeMap::from([(
                    pid.clone(),
                    serde_json::json!({
                        "board_assists": board,
                        "hand_assists": hand,
                        "yens": 1,
                    }),
                )]),
            }]
        };

        // AREA-sourced — what BGA reports, and what rev 7 replays: the two
        // placed pawns become one on the area plus one on the track, board
        // total unchanged at 2, hand untouched.
        let mut area_stream = script();
        area_stream.checkpoints = checkpoint(2, s0_hand as i64 - 2);
        let mut t = Translator::new(fresh_session(), &resolved, area_stream, &feed).unwrap();
        t.run().expect("rev 7 sources the track pawn from the activated area, matching BGA");
        assert_eq!(t.report.checkpoints_passed, 1, "the checkpoint was actually compared");

        // HAND-sourced — the rev-6 shape: the track pawn comes out of the
        // hand on top of an untouched area, so board runs one HIGH and hand
        // one LOW. Against rev 7 that is a plain counter divergence.
        let mut hand_stream = script();
        hand_stream.checkpoints = checkpoint(3, s0_hand as i64 - 3);
        let mut t = Translator::new(fresh_session(), &resolved, hand_stream, &feed).unwrap();
        let detail = t
            .run()
            .expect_err("hand-sourced counters cannot match an area-first engine")
            .to_string();
        assert!(detail.contains("T9 counter divergence"), "{detail}");
        assert!(detail.contains("board_assists") || detail.contains("hand_assists"), "{detail}");
    }

    /// The owned prerequisites of a translator over the committed excerpt, with
    /// `checkpoints` as its ENTIRE checkpoint set — the shape
    /// [`a_checkpoint_no_state_satisfies_stops_with_its_field_level_diff`]
    /// established, factored out so several cells can place a checkpoint
    /// wherever the mechanism under test needs one. Returns the parts rather
    /// than a `Translator` because a translator BORROWS its feed and resolution,
    /// so both have to outlive it in the caller's frame. `seat0_pid` is handed
    /// back too: `resolve` consumes the answers a caller would read it from.
    ///
    /// `checkpoints` is built from seat 0's player id rather than passed in,
    /// since every cell that places one needs that id to key it.
    fn excerpt_parts(
        checkpoints: impl FnOnce(&str) -> Vec<super::super::facts::CounterCheckpoint>,
    ) -> (
        yokohama_bridge::feed::CollapsedFeed,
        super::super::resolve::Resolved,
        super::super::facts::FactStream,
        Rules,
    ) {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
        ))
        .unwrap();
        let rules = Rules::standard();
        let feed = yokohama_bridge::feed::read_and_collapse(&text, None).unwrap();
        let answers = super::super::setup::synthesize(&feed, &rules).unwrap();
        let mapper = super::super::orders::OrderMapper::from_rules(&rules).unwrap();
        let seats = answers.seats.clone();
        let seat0_pid = answers.seats[0].bga_player_id.clone();
        let mut stream = super::super::facts::extract(
            &feed,
            &|pid| seats.iter().find(|s| s.bga_player_id == pid).map(|s| s.seat),
            &mapper,
        )
        .unwrap();
        stream.checkpoints = checkpoints(&seat0_pid);
        let resolved = super::super::resolve::resolve(answers, &feed, &stream, &rules).unwrap();
        (feed, resolved, stream, rules)
    }

    /// **The message-masking regression (BGA table 896199544, the game-3 m106
    /// stop).** `consume_drive` returns `None` for TWO unrelated reasons — the
    /// drive queue is empty, or a checkpoint FAILED and its real error was
    /// stashed in `pending_checkpoint_error`. The take arms turned the second
    /// into a claim about the first ("lab/port take fact vanished"), and because
    /// that fabricated error throws from INSIDE the current `step()`, the stashed
    /// diagnosis is never reached at `step()`'s entry and is discarded outright.
    ///
    /// On the live capture this printed "port take fact vanished" while the
    /// m106 `orderAcquisition` notification was sitting right there in the feed —
    /// the real stop being a T9 divergence at m105. The shape is reproduced here
    /// on the LAB arm, which carries the identical defect: the excerpt's lab take
    /// at m6 consumes the checkpoint placed at m5, so the failure lands inside
    /// `step_lab_take`'s `consume_drive`.
    #[test]
    fn a_checkpoint_failure_inside_a_take_arm_surfaces_the_real_divergence() {
        // m5 (the president placement onto the Laboratory) is real-turn era and
        // inside the excerpt's clean window; the m6 lab take is the first fact
        // consumed past it, so ITS `consume_drive` runs this checkpoint.
        let (feed, resolved, stream, rules) = excerpt_parts(|pid| {
            vec![super::super::facts::CounterCheckpoint {
                move_id: 5,
                per_player: BTreeMap::from([(pid.to_string(), serde_json::json!({ "yens": 9999 }))]),
            }]
        });
        let session = super::super::setup::build_session(&resolved.answers, rules).unwrap();
        let mut t = Translator::new(session, &resolved, stream, &feed).unwrap();
        let detail = t.run().expect_err("a checkpoint no engine state satisfies must STOP").to_string();
        assert!(
            detail.contains("T9 counter divergence"),
            "the REAL error must survive the take arm: {detail}"
        );
        assert!(detail.contains("`yens`"), "the diff names the counter: {detail}");
        assert!(detail.contains("BGA 9999"), "the diff carries BGA's value: {detail}");
        assert!(
            !detail.contains("vanished"),
            "a checkpoint failure must never be reported as a missing fact: {detail}"
        );
    }

    /// The general mechanism, both arms, at the helper every paraphrasing site
    /// now goes through: a stashed checkpoint error WINS, and the caller's own
    /// "vanished" wording is reachable ONLY when nothing was stashed — i.e. only
    /// when the queue is genuinely empty.
    #[test]
    fn drive_absence_prefers_the_stashed_error_and_only_then_the_callers_wording() {
        let (feed, resolved, stream, rules) = excerpt_parts(|_| vec![]);
        let session = super::super::setup::build_session(&resolved.answers, rules).unwrap();
        let mut t = Translator::new(session, &resolved, stream, &feed).unwrap();

        // Nothing stashed: the caller's wording stands, and the caller is the
        // only one who can describe an empty queue.
        let empty = t.drive_absence(42, "port take fact vanished").to_string();
        assert!(empty.contains("port take fact vanished"), "{empty}");
        assert!(empty.contains("42"), "the caller's move id rides the error: {empty}");

        // A stashed checkpoint error: it wins, and the wording is never applied.
        t.pending_checkpoint_error = Some(rerr(7, "T9 counter divergence — seat 0 `yens`"));
        let stashed = t.drive_absence(42, "port take fact vanished").to_string();
        assert!(stashed.contains("T9 counter divergence"), "the stashed error wins: {stashed}");
        assert!(!stashed.contains("vanished"), "the fabricated claim is gone: {stashed}");

        // And taking it CLEARS it, so one failure is never reported twice.
        assert!(
            t.pending_checkpoint_error.is_none(),
            "the stashed error must be taken, not left to surface again"
        );
    }

    fn lab(extra_yen: u8, extra_imported: u8, extra_tech_payment: Option<bool>) -> Move {
        Move::ExecuteAreaAction {
            action: AreaAction::Laboratory {
                card_id: yokohama_core::TechCardId(4),
                extra_yen,
                extra_imported,
                extra_tech_payment,
            },
        }
    }

    /// **The m118 shape (BGA table 896199544), and why the old key was wrong.**
    ///
    /// Seat 1 buys Tram: industry cost 5, slot surcharge +1, president power 3,
    /// so the deficit is 3 — and holding 4 yen and 2 imports, exactly TWO mixes
    /// are affordable. `k=1` debits 4 yen + 1 import (what the human paid, and
    /// what BGA's counters show); `k=2` debits 2 yen + 2 imports.
    ///
    /// Both pay the SAME industry value — every exact-cover variant does, since
    /// `extra_yen = 2*(deficit-k)` makes the value `k + (deficit-k) = deficit`
    /// for every `k`. So there is no "cheaper" mix to prefer: they are
    /// incomparable choices a player makes, and the capture's notification does
    /// not say which. The old key summed yen and imports as if they were one
    /// unit, ranked `k=2` (sum 4) below `k=1` (sum 5), found its "minimum"
    /// unique, and silently paid the wrong one — diverging the economy for the
    /// rest of the game. FALSE uniqueness, manufactured by the key.
    ///
    /// The declared policy is that the payment must be UNIQUE or the import
    /// refuses; with a key that cannot invent an ordering, this shape refuses,
    /// and it names each candidate by the prices it debits.
    /// **The m118 derivation (BGA table 896199544), owner-ruled option (a).**
    ///
    /// Two legal mixes, and BGA's own counters say which was paid: seat 1's yen
    /// fell by 4 and imports by 1 across the move. That fits `k=1` exactly and
    /// `k=2` not at all, so the capture DID determine the payment — just in the
    /// counters rather than in the notification. The owner's grounds, recorded
    /// in the 2026-08-23 ruling row: seeing the funds and types available before
    /// and after allows a derivation as good as seeing the turn, because that is
    /// the only way things could have been used in it.
    #[test]
    fn counters_that_fit_exactly_one_mix_derive_it_and_name_what_they_saw() {
        let variants = vec![lab(4, 1, None), lab(2, 2, None)];
        let LabPayment::Derived { chosen, chosen_label, candidates } =
            resolve_lab_payment(variants, Some((4, 1)))
        else {
            panic!("a spend fitting exactly one mix is evidence, not ambiguity");
        };
        assert_eq!(chosen, lab(4, 1, None), "the human's mix, not the cheapest-looking one");
        assert_eq!(chosen_label, "4 yen + 1 import");
        assert_eq!(candidates, vec!["4 yen + 1 import", "2 yen + 2 imports"]);
    }

    /// No usable counter evidence → dad87f6's honest refusal STANDS, unchanged.
    /// Deriving is allowed to resolve an ambiguity; its absence may not create
    /// a guess.
    #[test]
    fn without_counter_evidence_the_ambiguity_refusal_still_stands() {
        let variants = vec![lab(4, 1, None), lab(2, 2, None)];
        let LabPayment::Underdetermined { candidates } = resolve_lab_payment(variants, None) else {
            panic!("no evidence must leave the refusal in place");
        };
        assert_eq!(candidates, vec!["4 yen + 1 import", "2 yen + 2 imports"]);
    }

    /// Counters CAN underdetermine even when present: two different variants can
    /// debit the same resources. `2 yen + (surcharge 1 import)` and
    /// `1 import + (surcharge 2 yen)` both spend 2 yen and 1 import, so a spend
    /// of (2,1) does not choose between them — and the refusal must stand rather
    /// than take the first.
    #[test]
    fn counters_that_fit_two_mixes_equally_do_not_choose_between_them() {
        let variants = vec![lab(2, 0, Some(false)), lab(0, 1, Some(true))];
        let LabPayment::Underdetermined { candidates } =
            resolve_lab_payment(variants, Some((2, 1)))
        else {
            panic!("an equal fit is not a derivation");
        };
        assert_eq!(
            candidates,
            vec!["2 yen + 4th-tech surcharge 1 import", "1 import + 4th-tech surcharge 2 yen"]
        );
    }

    /// A spend matching NO legal mix is a real divergence — the engine and BGA
    /// disagree about what happened, which is a drift STOP and emphatically not
    /// an ambiguity to be resolved by picking something.
    #[test]
    fn a_spend_no_legal_mix_explains_is_a_divergence_not_an_ambiguity() {
        let variants = vec![lab(4, 1, None), lab(2, 2, None)];
        let LabPayment::NoneConsistent { spend, candidates } =
            resolve_lab_payment(variants, Some((3, 3)))
        else {
            panic!("an unexplainable spend must not resolve to any mix");
        };
        assert_eq!(spend, (3, 3));
        assert_eq!(candidates, vec!["4 yen + 1 import", "2 yen + 2 imports"]);
    }

    /// The single-legal-mix path is UNTOUCHED: no derivation, nothing declared,
    /// with or without counters. Nothing changed where nothing was ambiguous.
    #[test]
    fn a_single_legal_lab_payment_resolves_without_deriving_anything() {
        for spend in [None, Some((0, 0)), Some((9, 9))] {
            let LabPayment::Determined(mv) = resolve_lab_payment(vec![lab(2, 1, None)], spend)
            else {
                panic!("one legal mix needs no evidence and declares nothing");
            };
            assert_eq!(mv, lab(2, 1, None));
        }
    }

    /// The 4th-tech surcharge is labelled as such — two variants can differ ONLY
    /// there and would otherwise render identically.
    #[test]
    fn the_fourth_tech_surcharge_fork_is_named_as_a_surcharge() {
        let variants = vec![lab(0, 0, Some(true)), lab(0, 0, Some(false))];
        let LabPayment::Underdetermined { candidates } = resolve_lab_payment(variants, None) else {
            panic!("the surcharge fork is a real choice, not a tie to break");
        };
        assert_eq!(
            candidates,
            vec!["4th-tech surcharge 2 yen", "4th-tech surcharge 1 import"]
        );
    }

    /// **The m105-boundary shape (BGA table 896199544), as arithmetic.**
    ///
    /// A Port action at 5 Power books its +3 VP at the PRESIDENT-PLACEMENT move,
    /// one move before the take; the engine books it inside the atomic Port
    /// action, with the take. So the boundary carrying "bonus granted, nothing
    /// taken" is a state the engine steps over — mid-composite by granularity,
    /// exactly like the m169/m170 two-move take pair already handled — and the
    /// composite's interior has to START at the bonus rather than at the first
    /// take. Before this, m105 fell outside the span, was CHECKED, and stopped
    /// the whole import on a divergence that was pure bookkeeping order.
    ///
    /// The no-bonus rows are the ones that must not move: with `None` the span
    /// is byte-identical to the old `first.move_id..m2`, which is what keeps
    /// every already-green checkpoint on the healthy captures green.
    #[test]
    fn the_port_composite_interior_starts_at_the_bonus_and_is_unchanged_without_one() {
        // Game 3's shape: bonus at m105, takes at m106 and m107. Both interior
        // boundaries are skipped; the LAST take's own move stays checked.
        assert_eq!(port_composite_interior(Some(105), 106, Some(107)), 105..107);
        // A single take after a bonus: the bonus boundary alone.
        assert_eq!(port_composite_interior(Some(105), 106, None), 105..106);

        // NO bonus — the pre-existing behaviour, unchanged. The m169/m170 pair:
        assert_eq!(port_composite_interior(None, 169, Some(170)), 169..170);
        // and a lone take spans nothing at all.
        assert!(port_composite_interior(None, 169, None).is_empty());
    }

    /// F2 (lens-2 MUST): the T9 counter cross-check gets its NEGATIVE pin. A
    /// checkpoint whose counters no engine state can satisfy must produce the
    /// typed `T9 counter divergence` STOP naming the field-level diff. Before
    /// this test the whole comparison could be disabled — `if bga != engine` →
    /// `if false && bga != engine`, every checkpoint then trivially "green" —
    /// with the ENTIRE suite still passing and the shipped report byte-identical
    /// (lens-2's escaped mutation). The excerpt carries no socket counters, so
    /// the cross-check ran on nothing across the always-run tier; this injects
    /// the one checkpoint it lacked, at an early move with a `yens` value the
    /// engine never holds. Re-run under that mutation, THIS test goes red.
    #[test]
    fn a_checkpoint_no_state_satisfies_stops_with_its_field_level_diff() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
        ))
        .unwrap();
        let rules = Rules::standard();
        let feed = yokohama_bridge::feed::read_and_collapse(&text, None).unwrap();
        let answers = super::super::setup::synthesize(&feed, &rules).unwrap();
        let mapper = super::super::orders::OrderMapper::from_rules(&rules).unwrap();
        let seats = answers.seats.clone();
        let mut stream = super::super::facts::extract(
            &feed,
            &|pid| seats.iter().find(|s| s.bga_player_id == pid).map(|s| s.seat),
            &mapper,
        )
        .unwrap();
        // Read seat 0's real player id before `resolve` consumes `answers`.
        // The excerpt's own checkpoints sit at moves 116-119 (`counters` nested
        // in the socket frames' body_text); the replay never reaches them across
        // the redacted gap. Replace the set with ONE checkpoint at an early
        // REAL-TURN move whose `yens` no engine state holds — `consume_drive`
        // checks it when the next drive fact past it lands, so the STOP is
        // this checkpoint's, not the gap's. The move id sits AT the first
        // Placement (m4) — the live gate game taught the translator that
        // checkpoints BELOW it are setup-era named skips (BGA sequences the
        // starting-order discards finalize_setup atomizes), and the excerpt's
        // redaction gap begins at m6, so m4 is the one move that is both
        // real-turn-era and inside the excerpt's clean window.
        let pid = answers.seats[0].bga_player_id.clone();
        stream.checkpoints = vec![super::super::facts::CounterCheckpoint {
            move_id: 4,
            per_player: BTreeMap::from([(pid, serde_json::json!({ "yens": 9999 }))]),
        }];
        let resolved = super::super::resolve::resolve(answers, &feed, &stream, &rules).unwrap();
        let session = super::super::setup::build_session(&resolved.answers, rules).unwrap();
        let mut t = Translator::new(session, &resolved, stream, &feed).unwrap();
        let detail = t.run().expect_err("a checkpoint no engine state satisfies must STOP").to_string();
        assert!(detail.contains("T9 counter divergence"), "{detail}");
        assert!(detail.contains("`yens`"), "the diff names the counter: {detail}");
        assert!(detail.contains("BGA 9999"), "the diff carries BGA's value: {detail}");
    }
}
