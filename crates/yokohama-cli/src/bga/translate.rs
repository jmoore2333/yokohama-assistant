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

use super::facts::{Fact, FactStream, SecondPayment, TimedFact};
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

/// One Church/Customs take that consumed a track pawn while BOTH sources
/// (hand and acting area) held one — the state where core's hand-first pick
/// and BGA's measured area-first pick produce different mid-turn counters.
/// See `record_ambiguous_track_take` for the full story.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AmbiguousTrackTake {
    move_id: u64,
    seat: u8,
    action: &'static str,
    hand: u8,
    area: u8,
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

/// `Some((in_hand, in_acting_area))` when the CURRENT seat could source a
/// Church/Customs track pawn from either place — exactly the states where
/// core's hand-first pick (`take_church_or_customs_assistant`, tracks.rs) and
/// BGA's measured area-first pick produce DIFFERENT mid-turn states. The
/// acting area comes from the engine's own `resolve_acting_area`, so
/// mercenary sub-turns read the mercenary target area, not the president's.
/// One-source states return `None`: there the two rules coincide and a take
/// translates faithfully.
fn both_track_pawn_sources(state: &yokohama_core::GameState) -> Option<(u8, u8)> {
    let acting = yokohama_core::game::actions::resolve_acting_area(state, state.current_player).ok()?;
    let p = &state.players[state.current_player.0 as usize];
    let hand = p.assistants.in_hand;
    let area = p.assistants.in_area(acting);
    (hand >= 1 && area >= 1).then_some((hand, area))
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
    /// Both-sources Church/Customs takes, recorded for the T9 STOP's
    /// source-rule annotation (see `record_ambiguous_track_take`).
    ambiguous_track_takes: Vec<AmbiguousTrackTake>,
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
            ambiguous_track_takes: Vec::new(),
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

    /// The m49 divergence RECORDER (live gate table 896157897; D3-class owner
    /// question). A Church/Customs take consumes one track pawn, and the
    /// rulebook (p.13/p.14) lets the player source it "from the
    /// [Church/Customs] Area or your hand". Core collapsed that choice to
    /// HAND-FIRST (`take_church_or_customs_assistant`, tracks.rs — reasoned
    /// net-inert because the same-turn Recovery reconverges the totals),
    /// while every BGA observation to date sources AREA-first: 896157897
    /// m49's workers list moves a Church-AREA pawn onto the track with the
    /// hand untouched (hand was 1), and the proving corpus's checkpointed
    /// takes (m153/m180) do the same at hand 0, where the two rules
    /// coincide. A both-sources take is therefore POTENTIALLY divergent —
    /// but not provably: the corpus's m82 both-sources take replays green
    /// because no checkpoint observes its window, and stopping there would
    /// kill a legitimately-green replay. So the take is RECORDED, not
    /// stopped, and when a later T9 checkpoint fails with exactly the
    /// source-rule signature (engine board_assists one HIGH and hand_assists
    /// one LOW — hand-sourcing counts the track pawn as board on top of an
    /// untouched area) the STOP carries the named story instead of a bare
    /// counter mystery (the shape that mis-aimed the first m49 diagnosis
    /// toward recovery semantics). Never an offset, never silence — the
    /// engine-side rule is an owner question (align the source or queue a
    /// real choice; the D1/D2 pattern).
    fn record_ambiguous_track_take(&mut self, action: &'static str, move_id: u64) {
        if let Some((hand, area)) = both_track_pawn_sources(self.state()) {
            self.ambiguous_track_takes.push(AmbiguousTrackTake {
                move_id,
                seat: self.current_seat(),
                action,
                hand,
                area,
            });
        }
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

    /// The source-rule signature over a failed checkpoint's diffs: every diff
    /// on one seat, exactly the two assistant counters, engine board one HIGH
    /// and hand one LOW — with a recorded both-sources take by that seat at
    /// or before the checkpoint's move. Returns the matching take.
    fn source_rule_signature(&self, diffs: &[CounterDiff]) -> Option<&AmbiguousTrackTake> {
        let [a, b] = diffs else { return None };
        if a.seat != b.seat {
            return None;
        }
        let (board, hand) = match (a.counter.as_str(), b.counter.as_str()) {
            ("board_assists", "hand_assists") => (a, b),
            ("hand_assists", "board_assists") => (b, a),
            _ => return None,
        };
        if board.engine != board.bga + 1 || hand.engine != hand.bga - 1 {
            return None;
        }
        self.ambiguous_track_takes
            .iter()
            .rev()
            .find(|t| t.seat == board.seat && t.move_id <= board.move_id)
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
                        // A bare counter diff mis-aims diagnosis when the
                        // cause is the known source-rule divergence — name it
                        // whenever the diff carries its exact signature (see
                        // `record_ambiguous_track_take`).
                        let source_story = match self.source_rule_signature(&self.report.diffs) {
                            Some(t) => format!(
                                " — this is the {} track pawn SOURCE-RULE signature: the m{} take \
                                 held {} hand pawn(s) AND {} on the acting area, the engine \
                                 consumed hand-first (core take_church_or_customs_assistant, \
                                 rulebook p.13/p.14 allows either source) where every BGA \
                                 observation to date consumes from the AREA (896157897 m49 \
                                 workers; corpus m153/m180 at hand 0), so the engine runs one \
                                 board-assistant HIGH and one hand-assistant LOW until Recovery \
                                 reconverges them; the engine-side source rule needs an owner \
                                 ruling, not a translator guess",
                                t.action, t.move_id, t.hand, t.area
                            ),
                            None => String::new(),
                        };
                        return Err(rerr(
                            cp.move_id,
                            format!(
                                "T9 counter divergence — no engine state in this turn window \
                                 matches BGA's move-{} counters; nearest: seat {} `{}` BGA {} vs \
                                 engine {} ({} diff(s) total){source_story}",
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
                Fact::President { seat: s, dest, .. } if *s == seat => {
                    let dest = *dest;
                    let fees = self.pass_fee_count_window();
                    self.consume_drive();
                    let dest_id = self.area_id(dest, move_id)?;
                    let in_hand = matches!(
                        self.state().players[seat as usize].president_location,
                        PresidentLocation::InHand
                    );
                    let mv = if in_hand {
                        Move::PlacePresident { destination: dest_id }
                    } else {
                        // Disambiguate the route label by the observed fee count.
                        let state = self.state();
                        let opponents_at = |area: &AreaId| {
                            state.players.iter().enumerate().any(|(i, p)| {
                                i != seat as usize
                                    && p.president_location == PresidentLocation::OnBoard(*area)
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
                            1 => candidates.into_iter().next().expect("len checked"),
                            0 => {
                                return Err(rerr(
                                    move_id,
                                    format!(
                                        "no legal president route to {dest:?} with {fees} pass-through fee(s)"
                                    ),
                                ))
                            }
                            n => {
                                return Err(rerr(
                                    move_id,
                                    format!(
                                        "{n} distinct route labels to {dest:?} with {fees} fee(s) — \
                                         underdetermined"
                                    ),
                                ))
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
                    self.record_ambiguous_track_take("Church", move_id);
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
                    self.record_ambiguous_track_take("Customs", move_id);
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

    fn step_port_take(&mut self, move_id: u64) -> Result<(), ImportError> {
        let seat = self.current_seat();
        // First take fact.
        let first = self.consume_drive().ok_or_else(|| rerr(move_id, "port take fact vanished"))?;
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
                for interior in first.move_id..m2 {
                    self.mid_composite_moves.insert(interior);
                }
            }
            Some(TimedFact { fact: Fact::PortSecondDeclined { seat: s }, move_id: m2, .. }) if s == seat => {
                self.consume_drive_uncheckpointed();
                for interior in first.move_id..m2 {
                    self.mid_composite_moves.insert(interior);
                }
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
        let tf = self.consume_drive().ok_or_else(|| rerr(move_id, "lab take fact vanished"))?;
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
        // Payment variant: the minimal-extra legal variant must be unique.
        let mut variants: Vec<Move> = self
            .session
            .legal()
            .into_iter()
            .filter(|m| {
                matches!(m, Move::ExecuteAreaAction { action: AreaAction::Laboratory { card_id, .. } } if *card_id == card)
            })
            .collect();
        variants.sort_by_key(|m| match m {
            Move::ExecuteAreaAction { action: AreaAction::Laboratory { extra_yen, extra_imported, .. } } => {
                (*extra_yen as u32) + (*extra_imported as u32)
            }
            _ => u32::MAX,
        });
        let minimal_cost = variants.first().map(|m| match m {
            Move::ExecuteAreaAction { action: AreaAction::Laboratory { extra_yen, extra_imported, .. } } => {
                (*extra_yen as u32) + (*extra_imported as u32)
            }
            _ => u32::MAX,
        });
        let minimal: Vec<&Move> = variants
            .iter()
            .filter(|m| match m {
                Move::ExecuteAreaAction { action: AreaAction::Laboratory { extra_yen, extra_imported, .. } } => {
                    Some((*extra_yen as u32) + (*extra_imported as u32)) == minimal_cost
                }
                _ => false,
            })
            .collect();
        let mv = match minimal.as_slice() {
            [one] => (*one).clone(),
            [] => return Err(rerr(move_id, format!("no legal Laboratory take of {card:?}"))),
            several => {
                return Err(rerr(
                    move_id,
                    format!("ambiguous lab payment variants for {card:?}: {several:?}"),
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

    /// The m49 divergence pin (live gate table 896157897): when a T9
    /// checkpoint fails with exactly the track-pawn SOURCE-RULE signature —
    /// engine `board_assists` one HIGH, `hand_assists` one LOW, after a
    /// Church/Customs take that held BOTH sources — the STOP must carry the
    /// NAMED story, not the bare counter diff that mis-aimed the first m49
    /// diagnosis toward recovery semantics. Rulebook p.13/p.14 lets the
    /// player source the track pawn "from the [Church/Customs] Area or your
    /// hand"; core's `take_church_or_customs_assistant` (tracks.rs) collapsed
    /// that to HAND-FIRST (net-inert at end of turn, NOT over the mid-turn
    /// states BGA checkpoints), and every BGA observation sources AREA-first
    /// (m49: a Church-area pawn moves onto the track, counters untouched at
    /// 8/1 where the engine lands 9/0). The take itself must NOT stop — the
    /// proving corpus's m82 both-sources take replays green because no
    /// checkpoint observes its window, and that 194-move acceptance run is a
    /// standing pin. The live capture is gitignored, so the trigger is
    /// scripted synthetically: Option-B two on Church, president onto Church
    /// (power 3), church pay with the yen type (start money 3 covers the
    /// 2-yen price, faith 3+1=4 lands the one eligible space inline), then a
    /// checkpoint carrying the AREA-sourced counters BGA would report. What
    /// only the live run proves — that BGA's counters MATCH an area-first
    /// engine — was measured by a never-committed area-first diagnostic
    /// build replaying 896157897 clean through m49 (33 checkpoints green).
    #[test]
    fn the_source_rule_signature_names_the_story_on_the_t9_stop() {
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
        // the committed fixture needs no capture bytes.
        let mut stream = FactStream {
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
                    fact: Fact::President { seat: 0, dest: AreaType::Church, power: 3 },
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
        let resolved = super::super::resolve::resolve(answers, &feed, &stream, &rules).unwrap();
        let session = super::super::setup::build_session(&resolved.answers, rules.clone()).unwrap();
        // The checkpoint BGA would emit after an AREA-sourced take: the two
        // placed pawns become one on the area plus one on the track (board
        // total unchanged at 2), the hand untouched, and `yens` down by the
        // 2-yen church price — the payment anchor that keeps this checkpoint
        // from matching the PRE-take state, whose board/hand read the same.
        let s0_hand = session.state().players[0].assistants.in_hand;
        let pid = resolved.answers.seats[0].bga_player_id.clone();
        stream.checkpoints = vec![super::super::facts::CounterCheckpoint {
            move_id: 7,
            per_player: BTreeMap::from([(
                pid,
                serde_json::json!({
                    "board_assists": 2,
                    "hand_assists": s0_hand - 2,
                    "yens": 1,
                }),
            )]),
        }];
        let mut t = Translator::new(session, &resolved, stream, &feed).unwrap();
        let detail = t
            .run()
            .expect_err("the area-sourced checkpoint must STOP against the hand-first engine")
            .to_string();
        assert!(detail.contains("T9 counter divergence"), "{detail}");
        assert!(detail.contains("SOURCE-RULE signature"), "the story is named: {detail}");
        assert!(detail.contains("m6 take"), "the take is located: {detail}");
        assert!(detail.contains("hand-first"), "the engine's rule is named: {detail}");
        assert!(detail.contains("owner ruling"), "the routing is named: {detail}");
    }

    /// The guard's CONDITION, pinned exactly: it reads the acting seat's hand
    /// and the ACTING AREA (via the engine's own `resolve_acting_area`), and
    /// fires only when both hold a pawn — one-source takes are exactly the
    /// states where hand-first and area-first coincide, so they must not
    /// stop. The one-source variants are pinned on MUTATED clones because a
    /// legal walk cannot cheaply reach them: the possible-action-area rule
    /// means a president stands only where an own assistant does, so at a
    /// real president-led take the area side is ≥ 1 by construction and the
    /// discriminating side is the hand (empty-hand takes area-source on both
    /// models; a zero-area acting state needs a mercenary activation).
    #[test]
    fn the_both_sources_condition_reads_hand_and_acting_area_exactly() {
        let mut s = sim_2p(900);
        let church = *s
            .state()
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == AreaType::Church)
            .unwrap()
            .0;
        s.apply(Move::EndAdditionalPhase, vec![]).unwrap();
        s.apply(Move::PlaceTwoSameArea { area: church, adjacent_bonus: None }, vec![]).unwrap();
        s.apply(Move::PlacePresident { destination: church }, vec![]).unwrap();
        let state = s.state().clone();
        let hand = state.players[0].assistants.in_hand;
        assert!(hand >= 1, "sim start leaves hand pawns after an Option-B placement");
        assert_eq!(both_track_pawn_sources(&state), Some((hand, 2)));
        // Hand empty: the area is the only source — both models agree.
        let mut hand_empty = state.clone();
        hand_empty.players[0].assistants.in_hand = 0;
        assert_eq!(both_track_pawn_sources(&hand_empty), None);
        // Acting area drained (the mercenary-reachable shape): hand-only.
        let mut area_empty = state.clone();
        area_empty.players[0].assistants.on_board.remove(&church);
        assert_eq!(both_track_pawn_sources(&area_empty), None);
        // No acting area resolves at all (president in hand): no take exists.
        let mut off_board = state;
        off_board.players[0].president_location = PresidentLocation::InHand;
        assert_eq!(both_track_pawn_sources(&off_board), None);
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
