//! Phase 3.1a Task 6 benchmark example — RECORDS (does not assert) the
//! latency / peak-outstanding-bytes / allocation-count CURVES for the factored
//! action source vs the exhaustive `get_legal_moves`, on the 4P retreat worst
//! case (product `3³ · 2¹⁴ = 442,368`) and a Newspaper placement family (3231).
//!
//! Every figure here is machine-variable, so nothing is asserted — the standing
//! HARD gate is `tests/action_source_worstcase.rs` (the allocation-COUNT bound).
//! Run once and capture the numbers for the PR / docs:
//!
//!   cargo run --release -p yokohama-core --example action_cost
//!
//! Meter (spec §4, PINNED): `peak_bytes` is peak OUTSTANDING requested bytes
//! (net-live high-water mark), NOT RSS; `allocs` is the deterministic event
//! count. Both come from the shared `action_cost_meter` (the same meter the
//! bound test uses). Latencies are `Instant`-timed, averaged over a loop with
//! `std::hint::black_box` to defeat elision — no criterion, no new dependency.

#[path = "../tests/action_cost_meter/mod.rs"]
mod meter;

use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::{Duration, Instant};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use yokohama_core::{
    families, get_legal_moves, placement_iter, placement_upper_len, retreat_iter,
    retreat_len, retreat_sample, AchievementCard, AchievementType, AreaConfig, AreaId, AreaType,
    BoardLayout, ChurchTrack, CustomsTrack, GameConfig, GameState, LabState, MercenarySupply,
    MoveFamily, PlayerColor, PlayerId, PlayerState, PortState, PresidentLocation, Rules, TechEffect,
    TurnPhase,
};

/// Average wall-clock of `f` over `iters` runs, `black_box`ing the result so the
/// optimizer cannot elide the work.
fn time_avg<R>(iters: u32, mut f: impl FnMut() -> R) -> Duration {
    let start = Instant::now();
    for _ in 0..iters {
        black_box(f());
    }
    start.elapsed() / iters
}

/// The 4P retreat worst case `on_board` (product 442,368): ids 0,1,2 at count 2,
/// ids 3..17 at count 1.
fn worst_case_on_board() -> BTreeMap<AreaId, u8> {
    let mut m = BTreeMap::new();
    for a in 0..3u8 {
        m.insert(AreaId::new(a), 2);
    }
    for a in 3..17u8 {
        m.insert(AreaId::new(a), 1);
    }
    m
}

/// The full 4P OnBoard worst-case `GameState` (see the bound test's twin): 17
/// placement-allowing areas, president on board at area 0, worst-case assistants,
/// minimal adjacency (0~1) so the recorded ceiling is dominated by the retreat
/// product, not a route DFS.
fn worst_case_state() -> GameState {
    let colors = [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green];
    let mut areas = BTreeMap::new();
    for i in 0..17u8 {
        areas.insert(
            AreaId::new(i),
            AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, i) },
        );
    }
    let mut adjacency = BTreeMap::new();
    adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
    adjacency.insert(AreaId::new(1), vec![AreaId::new(0)]);

    let mut s = GameState {
        config: GameConfig::for_player_count(4),
        board_layout: BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        },
        players: (0..4)
            .map(|i| PlayerState::new(PlayerId::new(i), colors[i as usize], i == 0))
            .collect(),
        current_player: PlayerId::new(0),
        turn_phase: TurnPhase::Movement,
        turn_number: 1,
        additional_action_state: Default::default(),
        current_action_power: None,
        is_mercenary_action: false,
        mercenary_target_area: None,
        mercenary_return_phase: None,
        president_start_area: None,
        port_state: PortState::new(false),
        lab_state: LabState::new(false),
        order_deck: Vec::new(),
        tech_deck: Vec::new(),
        orders_removed: Vec::new(),
        church_track: ChurchTrack::new(),
        customs_track: CustomsTrack::new(),
        achievements: [
            AchievementCard::new(AchievementType::Copper5),
            AchievementCard::new(AchievementType::Money10),
            AchievementCard::new(AchievementType::Production4),
        ],
        mercenary_supply: MercenarySupply::new(),
        station_location: None,
        end_game_triggered: false,
        trigger_reason: None,
        final_round: false,
        trigger_player: None,
        pending_choices: Vec::new(),
    };
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
    s.players[0].assistants.on_board = worst_case_on_board();
    s
}

/// A Newspaper placement family (3231): the worst-case board reused at
/// `Placement`, president in hand, in_hand 4 + Newspaper (k up to 4), money 1000
/// (fully affordable).
fn placement_state(rules: &Rules) -> GameState {
    let mut s = worst_case_state();
    s.turn_phase = TurnPhase::Placement;
    s.players[0].president_location = PresidentLocation::InHand;
    s.players[0].assistants.on_board.clear();
    s.players[0].assistants.in_hand = 4;
    s.players[0].money = 1_000;
    let newspaper =
        rules.techs.iter().find(|t| t.effect == TechEffect::Newspaper).expect("Newspaper").id;
    s.players[0].technologies = vec![newspaper];
    s
}

fn main() {
    let rules = Rules::standard();
    let state = worst_case_state();
    let on_board = worst_case_on_board();
    let mut rng = ChaCha8Rng::seed_from_u64(20_260_725);

    println!("== Phase 3.1a action-cost curves (release; peak = peak OUTSTANDING requested bytes) ==\n");

    // -- Retreat: the six-figure handover family -------------------------------
    let retreat_family_len = retreat_len(&on_board);
    println!("RETREAT family (4P OnBoard worst case): {retreat_family_len} moves\n");

    // Serve one candidate lazily: deep index + independent sample.
    let nth_lat = time_avg(200_000, || retreat_iter(&on_board).nth(400_000));
    let (_n, nth_m) = meter::measure(|| retreat_iter(&on_board).nth(400_000));
    let sample_lat = time_avg(200_000, || retreat_sample(&on_board, &mut rng));
    let (_s, sample_m) = meter::measure(|| retreat_sample(&on_board, &mut rng));

    // Materialize the whole retreat family (what the lazy serve avoids).
    let mat_lat = time_avg(10, || retreat_iter(&on_board).collect::<Vec<_>>());
    let (mat_vec, mat_m) = meter::measure(|| retreat_iter(&on_board).collect::<Vec<_>>());

    // The exhaustive ceiling: the whole legal list (retreat + routes).
    let legal_lat = time_avg(10, || get_legal_moves(&state, &rules));
    let (legal, legal_m) = meter::measure(|| get_legal_moves(&state, &rules));

    println!("  {:<34} {:>10} {:>12} {:>16}", "operation", "allocs", "peak bytes", "latency");
    println!("  {:-<34} {:->10} {:->12} {:->16}", "", "", "", "");
    row("retreat_iter().nth(400_000)", nth_m, nth_lat);
    row("retreat_sample()", sample_m, sample_lat);
    row(&format!("retreat_iter().collect()  [{}]", mat_vec.len()), mat_m, mat_lat);
    row(&format!("get_legal_moves()  [{}]", legal.len()), legal_m, legal_lat);
    println!(
        "\n  => serve-one avoids the exhaustive ceiling by ~{}x allocations \
         ({} vs {}).\n",
        legal_m.allocs / nth_m.allocs.max(1),
        legal_m.allocs,
        nth_m.allocs,
    );

    // -- Placement: combinations (secondary, smaller family) -------------------
    let pstate = placement_state(&rules);
    assert!(
        families(&pstate, &rules).into_iter().any(|f| matches!(f, MoveFamily::Placement)),
        "Placement dispatches a Placement family",
    );
    let upper = placement_upper_len(&pstate, &rules);
    // Realized comes from the iterator — the module's authority on realized
    // placement moves; `family_len` for Placement IS `placement_upper_len`, so
    // printing it here would show the bound twice and never the gap.
    let realized = placement_iter(&pstate, &rules).count();
    println!("PLACEMENT family (4P + Newspaper, in_hand 4): upper_len {upper} (realized {realized})\n");

    let pfirst_lat = time_avg(50_000, || placement_iter(&pstate, &rules).next());
    let (_pf, pfirst_m) = meter::measure(|| placement_iter(&pstate, &rules).next());
    let pmat_lat = time_avg(2_000, || placement_iter(&pstate, &rules).collect::<Vec<_>>());
    let (pmat_vec, pmat_m) = meter::measure(|| placement_iter(&pstate, &rules).collect::<Vec<_>>());

    println!("  {:<34} {:>10} {:>12} {:>16}", "operation", "allocs", "peak bytes", "latency");
    println!("  {:-<34} {:->10} {:->12} {:->16}", "", "", "", "");
    row("placement_iter().next()", pfirst_m, pfirst_lat);
    row(&format!("placement_iter().collect()  [{}]", pmat_vec.len()), pmat_m, pmat_lat);
    println!(
        "\n  (placement paging is O(k) generate-and-filter, not O(1) unrank — a 3.3 \
         widening concern; the family is small, so the gap is modest vs retreat.)"
    );
}

/// Print one `operation | allocs | peak bytes | latency` row.
fn row(label: &str, m: meter::Meter, latency: Duration) {
    println!("  {label:<34} {:>10} {:>12} {:>16?}", m.allocs, m.peak_bytes, latency);
}
