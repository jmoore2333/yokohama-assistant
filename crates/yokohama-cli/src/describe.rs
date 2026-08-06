//! Plain-words rendering of moves, areas, phases, and achievements --
//! the CLI's option text (design docs/plans/2026-07-18-phase2-cli-driver-design.md).

use yokohama_core::*;
use yokohama_core::game::actions::{
    bank_reward, calculate_church_faith, copper_mine_reward, eligible_customs_spaces_for_cost,
    fishing_ground_reward, silk_mill_reward, tea_plantation_reward,
};

use crate::render;

pub fn area_type_name(t: AreaType) -> &'static str {
    match t {
        AreaType::CopperMine => "Copper Mine",
        AreaType::SilkMill => "Silk Mill",
        AreaType::TeaPlantation => "Tea Plantation",
        AreaType::FishingGround => "Fishing Ground",
        AreaType::Bank => "Bank",
        AreaType::PortA => "Port A",
        AreaType::PortB => "Port B",
        AreaType::LaboratoryA => "Laboratory A",
        AreaType::LaboratoryB => "Laboratory B",
        AreaType::EmploymentAgency => "Employment Agency",
        AreaType::Chinatown => "Chinatown",
        AreaType::Church => "Church",
        AreaType::Customs => "Customs",
        AreaType::Canal => "Canal",
    }
}

pub fn country_name(c: Country) -> &'static str {
    match c {
        Country::English => "Britain",
        Country::American => "America",
        Country::French => "France",
        Country::German => "Germany",
        Country::Dutch => "Netherlands",
    }
}

/// A good's name as a card index filter states it.
pub fn card_good_name(g: crate::screen_model::CardGood) -> &'static str {
    use crate::screen_model::CardGood;
    match g {
        CardGood::Seafood => "seafood",
        CardGood::Tea => "tea",
        CardGood::Silk => "silk",
        CardGood::Copper => "copper",
    }
}

/// Where a card already sits, in the words BOTH surfaces that name a location
/// use: the dim note on an index card, and the refusal a card prompt prints
/// when the operator types that card again (finding F6 — every duplicate
/// refusal names WHERE the card already is).
///
/// One function rather than two strings kept in step: a player who reads
/// `= Port A slot 1` on a card and then `O01 is Port A slot 1` in a refusal is
/// reading the same fact twice, which is what makes a refusal something to act
/// on rather than argue with.
///
/// **Every one of these fits a card's interior**, because the index prints them
/// ON the card, two columns in behind a `= `. `card_locations_fit_a_card_face`
/// is the pin, and it is why these read `Seat 0's deal` (the approved mock's
/// own wording) rather than the longer phrasings that would clip.
pub fn card_location(at: crate::screen_model::CardLocation) -> String {
    use crate::screen_model::CardLocation;
    match at {
        CardLocation::Deal { seat } => format!("Seat {}'s deal", seat.0),
        CardLocation::PortSlot { board_b, slot } => {
            format!("{} slot {}", port_board_name(board_b), slot + 1)
        }
        CardLocation::OwnHand => "in your hand".to_string(),
        CardLocation::ThisReveal => "in this reveal".to_string(),
        CardLocation::LabSlot { board_b, slot } => {
            format!("{} slot {}", lab_board_name(board_b), slot + 1)
        }
        CardLocation::OwnedBy { seat } => format!("owned by Seat {}", seat.0),
        CardLocation::OnArea { area } => format!("on area A{:02}", area.0),
        CardLocation::AchievementSlot { category } => {
            format!("category {} in play", category_letter(category))
        }
    }
}

/// `A` / `B` / `C` — an achievement category as the printed cards group them.
pub fn category_letter(c: AchievementCategory) -> &'static str {
    match c {
        AchievementCategory::A => "A",
        AchievementCategory::B => "B",
        AchievementCategory::C => "C",
    }
}

/// One of a building site's four Shophouse spaces, in the words the printed
/// card lays them out in (`09-building-sites.yaml`'s TL/TR/ML/MR keys). Named
/// rather than abbreviated because the `sites` view exists to be read at the
/// table beside the physical card: `top-left` finds the space, `TL` does not.
pub fn site_space_name(slot: usize) -> &'static str {
    match slot {
        0 => "top-left",
        1 => "top-right",
        2 => "mid-left",
        3 => "mid-right",
        // `SPACES_PER_SITE` is 4 and the arrays are fixed-size, so this arm is
        // unreachable from any real card; it names the slot rather than
        // panicking, on the same totality rule every other renderer here holds.
        _ => "space",
    }
}


/// What a card prompt is recording, as the index's context header states it.
///
/// **The one place [`crate::screen_model::RecordingTarget`]'s 0-based positions
/// become the numbers a person reads** (T5 review S3), and it converts all of
/// them the same way: `nth`, `slot` and `which` each get [`human_position`], so
/// no variant can be a special case and no caller has to pre-increment. Totals
/// (`of`) are printed as they are — they are counts, not positions.
pub fn recording_target(target: crate::screen_model::RecordingTarget) -> String {
    use crate::screen_model::RecordingTarget;
    match target {
        RecordingTarget::PortRefill { board_b, slot, nth, of } => format!(
            "{} slot {} (reveal {} of {of})",
            port_board_name(board_b),
            human_position(slot),
            human_position(nth),
        ),
        RecordingTarget::TechRefill { board_b } => {
            format!("the card that refilled {}", lab_board_name(board_b))
        }
        RecordingTarget::PortSetup { board_b, slot, of } => format!(
            "{} slot {} of {of}",
            port_board_name(board_b),
            human_position(slot),
        ),
        RecordingTarget::DealSetup { seat, which, of } => format!(
            "Seat {}'s dealt card {} of {of}",
            seat.0,
            human_position(which),
        ),
        RecordingTarget::LabSetup { board_b, slot, of } => format!(
            "{} slot {} of {of}",
            lab_board_name(board_b),
            human_position(slot),
        ),
        // No `human_position` on either of the last two, and that is the point
        // of them: an area id and a category are IDENTITIES the driver already
        // names, so there is nothing to convert and nothing to be off by one
        // about. See `RecordingTarget::SiteSetup`.
        RecordingTarget::SiteSetup { area } => {
            format!("the building-site card on area A{:02}", area.0)
        }
        RecordingTarget::AchievementSetup { category } => {
            format!("the category {} Achievement card", category_letter(category))
        }
        RecordingTarget::StartingOrder { seat } => {
            format!("Seat {}'s hidden starting order", seat.0)
        }
    }
}

/// What a live capture just did, as the prompt above the question states it —
/// the plain-words sentence R1 asks for, in place of the abbreviations the
/// pre-redesign one-liner packed together.
///
/// Worded here rather than in the renderer for the reason every other sentence
/// in this crate is: the renderer draws, `describe` speaks. The two setup
/// targets are unreachable from the REPL's capture (the wizard asks those
/// inline, with its own question text) and are worded anyway, because a
/// `RecordingTarget` that could reach this function with nothing to say would be
/// a hole waiting for the next variant.
pub fn reveal_event(target: crate::screen_model::RecordingTarget) -> String {
    use crate::screen_model::RecordingTarget;
    match target {
        RecordingTarget::PortRefill { board_b, of, .. } => format!(
            "{} refilled {} from the order deck.",
            port_board_name(board_b),
            crate::card_render::plural(of, "slot"),
        ),
        RecordingTarget::TechRefill { board_b } => {
            format!("{} refilled from the technology deck.", lab_board_name(board_b))
        }
        RecordingTarget::StartingOrder { seat } => {
            format!("Seat {}'s starting order is face up at the table.", seat.0)
        }
        RecordingTarget::PortSetup { board_b, .. } => {
            format!("{}'s display is being recorded.", port_board_name(board_b))
        }
        RecordingTarget::LabSetup { board_b, .. } => {
            format!("{}'s display is being recorded.", lab_board_name(board_b))
        }
        RecordingTarget::DealSetup { seat, .. } => {
            format!("Seat {}'s dealt cards are being recorded.", seat.0)
        }
        RecordingTarget::SiteSetup { area } => {
            format!("Area A{:02}'s building-site card is being recorded.", area.0)
        }
        RecordingTarget::AchievementSetup { category } => format!(
            "The category {} Achievement card is being recorded.",
            category_letter(category),
        ),
    }
}

/// A registry as a sentence names it — "order cards", "technologies".
///
/// Worded here rather than at the call site for the reason every other word in
/// this crate is: one registry, one name, wherever it is said. Its first caller
/// is the refusal a cross-registry index pick gets, which has to say which
/// registry the live capture is actually asking about (T6 review M1).
pub fn registry_name(kind: crate::screen_model::IndexKind) -> &'static str {
    use crate::screen_model::IndexKind;
    match kind {
        IndexKind::Orders => "order cards",
        IndexKind::Techs => "technologies",
        IndexKind::Achievements => "achievement cards",
        IndexKind::Sites => "building-site cards",
    }
}

/// The same registry counted as ONE — for the running already-recorded line,
/// which prints a number in front of it.
///
/// Spelled out rather than trimmed from the plural: `technologies` does not
/// lose an `s`, and "1 technologie" is the kind of line that makes a tester
/// wonder what else is approximate.
pub fn registry_name_singular(kind: crate::screen_model::IndexKind) -> &'static str {
    use crate::screen_model::IndexKind;
    match kind {
        IndexKind::Orders => "order card",
        IndexKind::Techs => "technology",
        IndexKind::Achievements => "achievement card",
        IndexKind::Sites => "building-site card",
    }
}

/// A 0-based model position as the screen prints it.
///
/// Named rather than written `+ 1` at each site so the conversion is countable:
/// a variant that renders a position WITHOUT calling this is visible at a
/// glance, which is the whole content of the one-convention rule
/// [`crate::screen_model::RecordingTarget`] states.
fn human_position(index: usize) -> usize {
    index + 1
}

/// Port slot numbers are shown 1-based everywhere (`render::ports_view`'s own
/// labeling and the live reveal prompt's), so the two helpers above and the
/// prompts they feed all read the same board name.
pub fn port_board_name(board_b: bool) -> &'static str {
    if board_b {
        "Port B"
    } else {
        "Port A"
    }
}

pub fn lab_board_name(board_b: bool) -> &'static str {
    if board_b {
        "Laboratory B"
    } else {
        "Laboratory A"
    }
}

pub fn phase_name(p: TurnPhase) -> &'static str {
    match p {
        TurnPhase::AdditionalActionFirst => "Additional actions (before)",
        TurnPhase::Placement => "Placement",
        TurnPhase::Movement => "Movement",
        TurnPhase::AreaAction => "Area action",
        TurnPhase::FivePowerBonus => "5-Power bonus",
        TurnPhase::Building => "Building",
        TurnPhase::Recovery => "Recovery",
        TurnPhase::AdditionalActionSecond => "Additional actions (after)",
        TurnPhase::TurnEnd => "Turn end",
    }
}

/// The SHORT name a phase carries in the decision zone's chain, where all nine
/// sit on one line together (3.5a §0.8; the approved mocks' own list).
///
/// A second table rather than a shortening of [`phase_name`], because the two
/// answer different questions and the difference is not mechanical: the chain
/// distinguishes its two `Additional` steps by POSITION — they are the first
/// and last links of a line the player reads left to right — while
/// [`phase_name`] has to name one on its own, out of context, and so spells
/// "Additional actions (before)". Deriving one from the other would mean
/// trimming a parenthetical and hoping, and would put the chain's width at the
/// mercy of a wording change made for a different surface.
pub fn phase_chain_name(p: TurnPhase) -> &'static str {
    match p {
        TurnPhase::AdditionalActionFirst => "Additional",
        TurnPhase::Placement => "Placement",
        TurnPhase::Movement => "Movement",
        TurnPhase::AreaAction => "Area Action",
        TurnPhase::FivePowerBonus => "5-Power",
        TurnPhase::Building => "Building",
        TurnPhase::Recovery => "Recovery",
        TurnPhase::AdditionalActionSecond => "Additional",
        TurnPhase::TurnEnd => "End",
    }
}

pub fn area_label(state: &GameState, area: AreaId) -> String {
    let name = state
        .board_layout
        .areas
        .get(&area)
        .map(|a| area_type_name(a.area_type))
        .unwrap_or("Unknown area");
    format!("{name} (A{:02})", area.0)
}

/// An area's name in the redesigned frame's vocabulary: the NAME in caps, as
/// the board tiles and the dashboard already spell it.
///
/// The frame has TWO name resolvers by design, not by accident: the renderers
/// resolve an [`AreaId`] against the board MODEL (they may not open a
/// `GameState` — the R9 seam), and this resolves it against the state, for the
/// call sites that already hold one. They must agree, and they do because both
/// spell the name with [`area_type_name`]; what they cannot share is the
/// lookup.
pub fn area_name_upper(state: &GameState, area: AreaId) -> String {
    state
        .board_layout
        .areas
        .get(&area)
        .map(|a| area_type_name(a.area_type).to_uppercase())
        .unwrap_or_else(|| format!("A{:02}", area.0))
}

/// [`area_name_upper`] plus the printed id, for a row naming an area the
/// player may have to TYPE (the `a`/`b`/`m`/`r` grammars address areas by id).
pub fn area_label_upper(state: &GameState, area: AreaId) -> String {
    format!("{} (A{:02})", area_name_upper(state, area), area.0)
}

/// Plain-words statement of an achievement's fulfillment condition.
pub fn achievement_desc(a: AchievementType) -> String {
    match a {
        AchievementType::Copper5 => "Own 5 Copper".to_string(),
        AchievementType::Silk6 => "Own 6 Silk".to_string(),
        AchievementType::Tea7 => "Own 7 Tea".to_string(),
        AchievementType::Seafood7 => "Own 7 Seafood".to_string(),
        AchievementType::Money10 => "Own 10 Yen".to_string(),
        AchievementType::Tokens5 => "Own 5 tokens (5-Power + mercenaries)".to_string(),
        AchievementType::Orders5 => "Complete 5 orders".to_string(),
        AchievementType::Technologies4 => "Own 4 technologies".to_string(),
        AchievementType::Production4 => "Buildings in 4 production areas".to_string(),
        AchievementType::Commercial4 => "Buildings in 4 commercial areas".to_string(),
        AchievementType::Prod3Comm2 => "Buildings in 3 production + 2 commercial areas".to_string(),
        AchievementType::Prod2Comm3 => "Buildings in 2 production + 3 commercial areas".to_string(),
    }
}

/// One-line plain-words statement of a Technology card's effect, for the
/// `labs` view (item 5). A TOTAL match over `TechEffect` -- no wildcard arm,
/// so adding a variant is a compile error here rather than a silently-missing
/// line. The CLI guide's §8.2 lists card NAMES only, not per-effect wording,
/// so these are the SINGLE display source for what a tech does -- the `labs`
/// view exists precisely so a player can choose without an external
/// reference. ASCII only, to stay terminal-safe, and short enough that the
/// composed lab line clears 140 columns
/// (`labs_view_lines_fit_the_documented_terminal_width`).
///
/// AUTHORING RULE (F2, Codex adversarial review of PR #4): each line is
/// authored from the RULES sources -- rulebook pp. 15-16, the KB reference
/// `analysis-1/yokohama-kb/01-rules-reference.md`, and guidingpath's
/// decision log for the rulings (A4: Patent System is self-exclusive) --
/// and must carry every qualifier that would change a purchase decision:
/// the cost it still charges, the restriction it does not lift, the timing
/// it starts from. Five lines shipped without one (Patent System and
/// Letterpress Printing read as self-paying, Postal System hid its stop
/// fee, Stagecoach hid that it waives that fee, Telegram hid its bonus
/// assistant's placement fee) because the only coverage was "non-empty".
/// `tech_effect_text_carries_the_qualifiers_that_change_a_purchase` now
/// pins them card by card. A sixth followed (third review, I4): Station
/// dropped p.15's "and you do not need to pay even if there are other
/// President pawns in the Area", which is the whole reason the card is
/// worth 4 industry -- a standing Postal System PLUS Stagecoach for one
/// area, rather than the conditional shortcut the old line described.
pub fn tech_effect_text(effect: TechEffect) -> &'static str {
    match effect {
        TechEffect::Tram => "Teleport the President to any area without an opponent President",
        TechEffect::Stagecoach => "Move through opponent Presidents fee-free; with Postal System, stopping on one is free too",
        TechEffect::PostalSystem => "May stop and act where opponent Presidents stand; pay each owner 1 Yen unless you own Stagecoach",
        TechEffect::Station => "Place the Station token: +1 Power for all there; free teleport, no President fee; your assistant needed",
        TechEffect::Gaslight => "+2 VP when you build a Shophouse",
        TechEffect::ElectricalLight => "+3 VP when you build a Trading House",
        TechEffect::MiningTechnology => "+1 Copper each time you receive 1 or more Copper (once per gain, not per token)",
        TechEffect::SpinningMill => "Once per turn: discard 1 Silk to gain 2 Yen",
        TechEffect::Ball => "+2 VP when you complete an Order",
        TechEffect::LanguageSchool => "+1 Yen when you complete an Order",
        TechEffect::PatentSystem => "+2 VP for each Technology you acquire after this one (never for this card itself)",
        TechEffect::LetterPressPrinting => "+1 Yen for each Technology you acquire after this one (never for this card itself)",
        TechEffect::Winery => "+3 VP when you carry out the Church action and place an assistant",
        TechEffect::Brickyard => "+4 VP when you carry out the Customs action and place an assistant",
        TechEffect::SystemOfStock => "At turn start, if you have 1 Yen or less, refill to 2 Yen",
        TechEffect::University => "Once per turn: pay 1 Yen to take an assistant from the Warehouse",
        TechEffect::Newspaper => "Placement Option A may use 4 different areas instead of 3",
        TechEffect::Telegram => "After placing 2 assistants in one area, may place 1 in an adjacent area (normal placement fee applies)",
        TechEffect::Telephone => "During Recovery, may leave 1 assistant in the action area",
        TechEffect::Exposition => "Immediately gain 10 VP when acquired",
    }
}

/// Plain-words name of a trade good, for `Move::ChooseAnyGood`.
fn trade_good_name(g: TradeGood) -> &'static str {
    match g {
        TradeGood::Copper => "copper",
        TradeGood::Silk => "silk",
        TradeGood::Tea => "tea",
        TradeGood::Seafood => "fish",
    }
}

/// Plain-words article + name of a warehouse item, for
/// `Move::ChooseWarehouseTake` (e.g. "an assistant", "a shophouse").
fn warehouse_item_name(item: WarehouseItem) -> &'static str {
    match item {
        WarehouseItem::Assistant => "an assistant",
        WarehouseItem::Shophouse => "a shophouse",
        WarehouseItem::TradingHouse => "a trading house",
    }
}

/// Yen cost of taking `item` from the warehouse right now, for describing
/// `Move::ChooseWarehouseTake` -- assistants free, shophouses 2 Yen (both via
/// `WarehouseItem::flat_cost`, T2-m3 single source), trading houses at the
/// printed take progression indexed by tokens already out of the warehouse
/// (`TradingHouseState::already_taken`, T2-m2 single source; mirrors
/// `warehouse_take_options`'s own pricing).
fn warehouse_item_cost(p: &PlayerState, item: WarehouseItem) -> u32 {
    match item.flat_cost() {
        Some(cost) => cost,
        None => TradingHouseState::take_cost(p.trading_houses.already_taken()),
    }
}

/// Plain-words trade text for a single Chinatown exchange.
fn chinatown_exchange_text(exchange: ChinatownExchange) -> &'static str {
    match exchange {
        ChinatownExchange::CopperToYen => "1 copper -> 2 yen",
        ChinatownExchange::SilkToYen => "1 silk -> 2 yen",
        ChinatownExchange::TeaToYen => "1 tea -> 1 yen",
        ChinatownExchange::SeafoodToYen => "1 fish -> 1 yen",
        ChinatownExchange::YenToCopper => "2 yen -> 1 copper",
        ChinatownExchange::YenToSilk => "2 yen -> 1 silk",
        ChinatownExchange::YenToTea => "1 yen -> 1 tea",
        ChinatownExchange::YenToSeafood => "1 yen -> 1 fish",
        ChinatownExchange::YenToImported => "4 yen -> 1 imported good",
    }
}

/// The area this AreaAction targets: the mercenary sub-turn's target area
/// when `state.is_mercenary_action`, else the current player's President
/// location on board -- mirrors the engine's own acting-area resolution in
/// `game::legal_moves::generate_area_action_moves`/`generate_building_
/// moves` so a preview never disagrees with what the engine will actually
/// act on. `None` when the President is still in hand: unreached by any
/// legal `ExecuteAreaAction`/`BuildShophouse`/`BuildTradingHouse` move
/// (both phases require the President on board first) but kept as a
/// defensive fallback for direct state construction (fixtures/tests).
fn acting_area(state: &GameState) -> Option<AreaId> {
    if state.is_mercenary_action {
        state.mercenary_target_area
    } else {
        match state.current_player().president_location {
            PresidentLocation::OnBoard(a) => Some(a),
            PresidentLocation::InHand => None,
        }
    }
}

/// Production-area preview text: `"{verb} (power {p} -> +{r} {good}, next
/// +{r2} at power {q})"` where `r` is the REAL reward `reward_fn` computes
/// for power `p` and the `next` clause names the closest higher power that
/// increases the yield (omitted at the table cap). Production is NOT always
/// power-for-power -- Copper Mine and Silk Mill under-produce at low power
/// (e.g. `copper_mine_reward(3) == 1`); only Tea Plantation/Fishing Ground
/// are a flat 1:1 (`game::actions::production`) -- so this always calls the
/// engine's own reward function rather than echoing the power number, which
/// would misstate the yield for the two non-linear areas. The threshold
/// clause probes the SAME function upward (every table caps by power 6, so
/// the probe is bounded), keeping `+0 copper` legible as "not yet" rather
/// than reading as a bug (Task 6; grounded UX finding, forge baseline).
/// Falls back to the bare verb when the acting area can't be determined
/// (see `acting_area`).
fn production_preview(state: &GameState, verb: &str, good: TradeGood, reward_fn: fn(u8) -> u8) -> String {
    match acting_area(state) {
        Some(area) => {
            // Recomputed rather than read from state.current_action_power,
            // but the two are always equal here -- including the mercenary
            // formula, which omits the President's own +1 exactly where the
            // target area is guaranteed not to hold it.
            let power = calculate_power(state, state.current_player, area);
            let reward = reward_fn(power);
            let next = (power + 1..=6).find(|&q| reward_fn(q) > reward);
            let threshold = match next {
                Some(q) => format!(", next +{} at power {q}", reward_fn(q)),
                None => String::new(),
            };
            format!("{verb} (power {power} -> +{reward} {}{threshold})", trade_good_name(good))
        }
        None => verb.to_string(),
    }
}

/// Fee preview suffix for a placement option (Task 6): ` — fee N yen (1 to
/// Blue, ...)`, or empty when free.
///
/// **The arithmetic is [`crate::screen_model::placement_fee`]; this function is
/// only its wording** (T0-review MF1, discharged at T4). It used to hold the
/// per-owner tally itself, which left the decision zone's typed `FeeModel` with
/// no placement half and would have meant two copies of the same rule the
/// moment the zone printed one. The engine's `calculate_placement_cost` /
/// `credit_placement_fees` pair is `pub(crate)` to yokohama-core and this crate
/// may not widen it, so the sourcing is unchanged in kind: the mirror plus the
/// scratch-apply pin (`placement_fee_preview_matches_the_engine_charge`), which
/// now covers both the model and this string because one is computed from the
/// other.
///
/// `placements` lists one entry PER ASSISTANT (Option B passes its area twice,
/// plus the Telegram bonus area). `pub(crate)`: the Option-B picker reuses it
/// for per-area fee marks so the picker and the option text can never disagree.
pub(crate) fn placement_fee_note(state: &GameState, placements: &[AreaId]) -> String {
    let Some(fee) = crate::screen_model::placement_fee(state, placements) else {
        return String::new();
    };
    let parts: Vec<String> = fee
        .to_players
        .iter()
        .map(|&(owner, n)| format!("{n} to {}", render::color_name(state.player(owner).color)))
        .collect();
    format!(" — fee {} yen ({})", fee.total, parts.join(", "))
}

/// One plain-words line describing an `AreaAction` (called from
/// `describe_move`'s `ExecuteAreaAction` arm). `state` locates the acting
/// area (`acting_area`) so the production arms (Copper Mine/Silk Mill/Tea
/// Plantation/Fishing Ground) can preview the real yield at the current
/// power (Task 12, T2.2a) -- every other arm's text is still independent
/// of board state.
fn describe_area_action(action: &AreaAction, state: &GameState, rules: &Rules) -> String {
    match action {
        AreaAction::ProduceCopper => production_preview(state, "Produce copper", TradeGood::Copper, copper_mine_reward),
        AreaAction::ProduceSilk => production_preview(state, "Produce silk", TradeGood::Silk, silk_mill_reward),
        AreaAction::ProduceTea => production_preview(state, "Produce tea", TradeGood::Tea, tea_plantation_reward),
        AreaAction::ProduceSeafood => {
            production_preview(state, "Produce seafood", TradeGood::Seafood, fishing_ground_reward)
        }
        AreaAction::Bank => {
            // Yield preview (item 3, third review): 1 yen per power, capped at
            // 5, at the current power -- the same `bank_reward` the engine pays,
            // in the shape of the production/Church previews. Falls back to the
            // bare verb when the acting area can't be determined.
            match acting_area(state) {
                Some(area) => {
                    let power = calculate_power(state, state.current_player, area);
                    let reward = bank_reward(power);
                    let next = (power + 1..=6).find(|&q| bank_reward(q) > reward);
                    let threshold = match next {
                        Some(q) => format!(", next +{} at power {q}", bank_reward(q)),
                        None => String::new(),
                    };
                    format!("Take money at the Bank (power {power} -> +{reward} yen{threshold})")
                }
                None => "Take money at the Bank".to_string(),
            }
        }
        AreaAction::Port { cards_taken, second_card_payment } => {
            let cards = cards_taken.iter().map(|id| format!("O{:02}", id.0)).collect::<Vec<_>>().join(", ");
            let mut s = format!("Port: take {cards}");
            match second_card_payment {
                Some(PortPayment::TwoYen) => s.push_str(", paying 2 yen"),
                Some(PortPayment::OneImportedGood) => s.push_str(", paying 1 imported good"),
                None => {}
            }
            s
        }
        AreaAction::Laboratory { card_id, extra_yen, extra_imported, fifth_tech_payment } => {
            let name = crate::input::tech_name(*card_id, &rules.techs);
            let mut s = format!("Laboratory: acquire {name}");
            if *extra_yen > 0 {
                s.push_str(&format!(", +{extra_yen} yen"));
            }
            if *extra_imported > 0 {
                s.push_str(&format!(", +{extra_imported} imported"));
            }
            // Rulebook p.12: "the 5th Technology card and later each cost
            // either an extra 2 Yen or 1 Imported Good token" -- the penalty
            // fires for the 5th AND every later acquisition (engine:
            // `fifth_tech_penalty_required`, owns 4+), so the label reads
            // "5th+" rather than a fixed "5th" that misnames a 6th/7th buy.
            match fifth_tech_payment {
                Some(true) => s.push_str(", 5th+ tech fee: 2 yen"),
                Some(false) => s.push_str(", 5th+ tech fee: 1 imported good"),
                None => {}
            }
            s
        }
        AreaAction::EmploymentAgency { assistants, shophouses, trading_houses } => {
            let mut parts = Vec::new();
            if *assistants > 0 {
                parts.push(format!("{assistants} assistant{}", if *assistants == 1 { "" } else { "s" }));
            }
            if *shophouses > 0 {
                parts.push(format!("{shophouses} shophouse{}", if *shophouses == 1 { "" } else { "s" }));
            }
            if *trading_houses > 0 {
                parts.push(format!(
                    "{trading_houses} trading house{}",
                    if *trading_houses == 1 { "" } else { "s" }
                ));
            }
            // Cost preview (Task 6): assistants free, shophouses at their
            // flat price, trading houses at the printed 4/5/6/7 progression
            // -- the same WarehouseItem::flat_cost / TradingHouseState::
            // take_cost tables the engine charges from (test-pinned against
            // an actual apply's money delta).
            let already = state.current_player().trading_houses.already_taken();
            let sh_cost = WarehouseItem::Shophouse.flat_cost().unwrap_or(0) * (*shophouses as u32);
            let th_cost: u32 =
                (0..*trading_houses).map(|i| TradingHouseState::take_cost(already + i as usize)).sum();
            let cost = sh_cost + th_cost;
            let cost_text =
                if cost == 0 { " (free)".to_string() } else { format!(" — cost {cost} yen") };
            format!("Employment Agency: take {}{cost_text}", parts.join(", "))
        }
        AreaAction::Chinatown { exchanges } => {
            let parts = exchanges.iter().map(|e| chinatown_exchange_text(*e)).collect::<Vec<_>>().join(", ");
            format!("Chinatown: {parts}")
        }
        AreaAction::Church { copper, silk, tea, seafood, yen, imported_good } => {
            let mut parts = Vec::new();
            if *copper {
                parts.push("copper");
            }
            if *silk {
                parts.push("silk");
            }
            if *tea {
                parts.push("tea");
            }
            if *seafood {
                parts.push("fish");
            }
            if *yen {
                parts.push("yen");
            }
            if *imported_good {
                parts.push("imported good");
            }
            // Faith preview (Task 6): the engine's own power + faith
            // arithmetic (`calculate_power` + `calculate_church_faith`), so
            // the number shown is exactly the `ChurchSpace { total_faith }`
            // the engine will queue (test-pinned). Skipped when the acting
            // area can't be determined (defensive fixture case).
            let kinds = parts.len();
            let faith_text = match acting_area(state) {
                Some(area) => {
                    let power = calculate_power(state, state.current_player, area);
                    let faith = calculate_church_faith(
                        power, *copper, *silk, *tea, *seafood, *yen, *imported_good,
                    );
                    let kinds_text = if kinds == 1 { "kind" } else { "kinds" };
                    format!(" -> faith {faith} (power {power} + {kinds} {kinds_text})")
                }
                None => String::new(),
            };
            if parts.is_empty() {
                format!("Church: pay nothing{faith_text}")
            } else {
                format!("Church: pay {}{faith_text}", parts.join(", "))
            }
        }
        AreaAction::Customs { imported_goods_paid } => {
            let pay = if *imported_goods_paid == 1 {
                "Customs: pay 1 imported good".to_string()
            } else {
                format!("Customs: pay {imported_goods_paid} imported goods")
            };
            // Outcome preview (item 3, third review): the space(s) this payment
            // lands on and their printed VP, read from the same
            // `eligible_customs_spaces_for_cost` the engine places from --
            // mirroring Church's faith preview, so the option shows what it
            // gains, not only what it costs. Gated on an acting context (like
            // the power-based previews above): outside one the option never
            // appears in real play, and the base plain-words form stays intact.
            let spaces = eligible_customs_spaces_for_cost(&state.customs_track, *imported_goods_paid);
            match (acting_area(state), spaces.as_slice()) {
                (Some(_), [s]) => format!("{pay} -> space {s} (+{} VP)", CustomsTrack::vp_reward(*s)),
                (Some(_), many @ [_, _, ..]) => {
                    let list = many
                        .iter()
                        .map(|s| format!("space {s} (+{} VP)", CustomsTrack::vp_reward(*s)))
                        .collect::<Vec<_>>()
                        .join(" or ");
                    format!("{pay} -> choose {list}")
                }
                // No acting context, or (defensively) no eligible space.
                _ => pay,
            }
        }
        AreaAction::Skip => "Take the area action, declining all optional parts".to_string(),
    }
}

/// One plain-words line describing a `Move` -- the CLI's option text.
pub fn describe_move(mv: &Move, state: &GameState, rules: &Rules) -> String {
    match mv {
        Move::UseMercenary { country, target_area } => {
            format!("Use a {} mercenary at {}", country_name(*country), area_label(state, *target_area))
        }
        Move::FulfillAchievement { achievement } => {
            format!("Claim achievement: {}", achievement_desc(*achievement))
        }
        Move::FulfillOrder { order_id } => match rules.orders.get(*order_id) {
            Some(card) => format!("Fulfill {}", order_prompt_line(card)),
            None => format!("Fulfill order O{:02}", order_id.0),
        },
        Move::UseUniversity => "Use University: pay 1 yen, take an assistant from the warehouse".to_string(),
        Move::UseSpinningMill => "Use Textile Mill: discard 1 silk, gain 2 yen".to_string(),
        Move::EndAdditionalPhase => "End additional-action phase".to_string(),
        Move::PlaceThreeAreas { placements } if placements.is_empty() => "Place no assistants".to_string(),
        Move::PlaceThreeAreas { placements } => format!(
            "Place 1 assistant in each of: {}{}",
            placements.iter().map(|a| area_label(state, *a)).collect::<Vec<_>>().join(", "),
            placement_fee_note(state, placements),
        ),
        Move::PlaceTwoSameArea { area, adjacent_bonus } => {
            let mut s = format!("Place 2 assistants in {}", area_label(state, *area));
            // One fee entry PER ASSISTANT: the area twice, plus Telegram's
            // bonus assistant (which pays under the same rule -- see
            // apply_place_two_same_area's reuse of apply_placement).
            let mut placements = vec![*area, *area];
            if let Some(adj) = adjacent_bonus {
                s.push_str(&format!(" and 1 in {} (Telegram)", area_label(state, *adj)));
                placements.push(*adj);
            }
            s.push_str(&placement_fee_note(state, &placements));
            s
        }
        Move::MovePresident { path, destination } if path.is_empty() => {
            format!("Move President directly to {} (free teleport)", area_label(state, *destination))
        }
        Move::MovePresident { path, destination } => {
            // `path` already carries the destination as its last element --
            // the same slice `apply_movement` prices (Task 6) -- so it feeds
            // `route_fee_vector` unchanged, never `path` + `destination`
            // appended (that would double-count the last hop).
            let vector = route_fee_vector(state, state.current_player, path, Some(&rules.techs), false);
            let cost_text = if vector.is_empty() {
                "free".to_string()
            } else {
                let total: u32 = vector.values().sum();
                // Player entries first (in PlayerId order), canal toll (the
                // `None` bank entry) last -- matches the design's example
                // wording, not `RouteVector`'s natural key order (`None`
                // sorts first in a BTreeMap since `None < Some(_)`).
                let parts: Vec<String> = vector
                    .iter()
                    .filter_map(|(recipient, amount)| {
                        recipient.map(|player_id| {
                            format!("{amount} to {}", render::color_name(state.player(player_id).color))
                        })
                    })
                    .chain(vector.get(&None).map(|amount| {
                        format!("{amount} canal toll{}", if *amount == 1 { "" } else { "s" })
                    }))
                    .collect();
                format!("pay {total} yen ({})", parts.join(", "))
            };
            format!("Move President to {} — {}", area_label(state, *destination), cost_text)
        }
        // Counts encoding (I5 ruling): per-area counts, not just which areas
        // -- two retreats differing only by count (e.g. recover 1 vs 2 from
        // the same area) must render distinguishably (Task 7's report;
        // Task 8 gives this its real treatment).
        Move::ReturnPresidentToHand { recover } => {
            if recover.is_empty() {
                "Return President to hand (recover no assistants)".to_string()
            } else {
                let total: u32 = recover.values().map(|&c| c as u32).sum();
                let parts = recover
                    .iter()
                    .map(|(a, c)| format!("A{:02} x{}", a.0, c))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Return President to hand, recovering {total} assistant(s) ({parts})")
            }
        }
        Move::PlacePresident { destination } => {
            format!("Place President from hand in {}", area_label(state, *destination))
        }
        Move::ExecuteAreaAction { action } => describe_area_action(action, state, rules),
        Move::Take5PowerToken => "Take the 5-Power token".to_string(),
        Move::Skip5PowerBonus => "Skip the 5-Power token".to_string(),
        Move::BuildShophouse { slot } => {
            // The space number is ALWAYS printed (Phase-2.9 per-slot
            // enumeration: one option per empty slot, and two blank slots
            // share the reward text "-", so the reward alone can't
            // distinguish them at the table).
            let reward = acting_area(state)
                .and_then(|a| state.board_layout.building_sites.get(&a))
                // and_then, not map: `slot_reward_text` yields None for an
                // out-of-range slot (a tampered record's can be), which falls
                // through to the bare "(space N)" wording below instead of
                // panicking. See C1, external review 2026-07-20.
                .and_then(|site| slot_reward_text(site, *slot as usize));
            // usize arithmetic: `slot + 1` on the raw u8 debug-overflows at
            // slot = 255 (tampered record) — the None arm must stay total.
            let space = (*slot as usize) + 1;
            match reward {
                Some(r) => format!("Build a Shophouse (space {space}: {r})"),
                None => format!("Build a Shophouse (space {space})"),
            }
        }
        Move::BuildTradingHouse => {
            let reward = acting_area(state).and_then(|a| state.board_layout.building_sites.get(&a));
            match reward {
                Some(site) => format!("Build a Trading House ({}vp)", site.trading_house_reward),
                None => "Build a Trading House".to_string(),
            }
        }
        Move::SkipBuilding => "Skip building".to_string(),
        Move::ChooseAnyGood { good } => format!("Take 1 {} (any-good reward)", trade_good_name(*good)),
        Move::ChooseWarehouseTake { item } => {
            let cost = warehouse_item_cost(state.current_player(), *item);
            let cost_text = if cost == 0 { "free".to_string() } else { format!("{cost} yen") };
            format!("Take {} from the warehouse ({cost_text})", warehouse_item_name(*item))
        }
        Move::DeclineWarehouseTake => "Decline the warehouse take".to_string(),
        Move::ChooseAssistantMove { from, to } => {
            let from_label = match from {
                MoveSource::Hand => "hand".to_string(),
                MoveSource::Area(a) => area_label(state, *a),
            };
            format!("Move an assistant from {from_label} to {} (reward move)", area_label(state, *to))
        }
        Move::DeclineAssistantMove => "Decline the assistant move".to_string(),
        // Show the space's FULL printed reward -- VP plus its movement rider
        // (Church) or warehouse/yen rider (Customs) -- in the SAME vocabulary
        // the `tracks` view prints, single-sourced from `render`'s reward-text
        // helpers so the choice can never disagree with `tracks`. The rider is
        // decision-relevant when the choice offers more than one space (Church
        // faith 4/5 = 1move vs 6 = 2move; Customs cost-1 space 1 = 2wh vs
        // space 2 = 2yen), and each space's placement queues that rider next
        // (rulebook p.13/p.14).
        Move::ChooseChurchSpace { space } => {
            format!("Place on Church faith-{space} space ({})", render::church_reward_text(*space))
        }
        Move::ChooseCustomsSpace { space } => {
            format!("Place on Customs space {space} ({})", render::customs_reward_text(*space))
        }
        Move::ChooseRecoveryLeave { leave_one: false } => {
            "Recover all assistants from the action area".to_string()
        }
        Move::ChooseRecoveryLeave { leave_one: true } => {
            "Leave 1 assistant behind (Telephone)".to_string()
        }
        Move::ChooseStationPlace { area } => {
            format!("Place the Station token in {}", area_label(state, *area))
        }
    }
}

/// One plain-words line describing a `RecordedEvent` as it happened: a
/// `Move` delegates to `describe_move` against `state`, while each
/// `Reveal*` variant's wording depends only on the event's own fields, not
/// `state` -- `state`/`rules` are threaded through regardless so every
/// caller can call this ONE function uniformly regardless of variant,
/// passing the state as of when the event actually happened. Moved here
/// (T2.4) from the CLI's old `Repl::describe_event`, which always described
/// against whatever state was CURRENT when `log` was invoked -- harmless
/// for the Reveal arms (state-independent), but wrong for `Move` once
/// state-dependent wording (movement fees/recipients, a trading-house
/// take's printed price) went stale after the state it described moved on.
/// `Session` (`apply`/`apply_standalone`/`load`) now calls this once per
/// event, at the moment the event is folded, and caches the result.
/// Plain-words label for a 5-Power token face (`2 copper`, `3 yen`, ...) --
/// what the reveal echo shows the operator to eyeball against the physical
/// token, and what the `log` view records, in place of the raw `{:?}` debug.
pub fn five_power_effect_label(effect: FivePowerEffect) -> String {
    match effect {
        FivePowerEffect::ImportedGoods(n) => format!("{n} imported goods"),
        FivePowerEffect::Copper(n) => format!("{n} copper"),
        FivePowerEffect::Silk(n) => format!("{n} silk"),
        FivePowerEffect::Tea(n) => format!("{n} tea"),
        FivePowerEffect::Seafood(n) => format!("{n} seafood"),
        FivePowerEffect::Money(n) => format!("{n} yen"),
    }
}

/// Comma-joined `O<id>` codes for a reveal group (`(none)` when empty, a
/// defensive case that shouldn't occur in real play).
fn join_order_ids(cards: &[OrderCardId]) -> String {
    let ids: Vec<String> = cards.iter().map(|c| format!("O{:02}", c.0)).collect();
    if ids.is_empty() { "(none)".to_string() } else { ids.join(", ") }
}

pub fn describe_recorded_event(ev: &RecordedEvent, state: &GameState, rules: &Rules) -> String {
    match ev {
        RecordedEvent::Move(mv) => describe_move(mv, state, rules),
        // Carry the card ids, not just a count, so the `log` view is an audit
        // trail: an operator can read back exactly which physical cards were
        // recorded and catch a mis-entry the desync guard structurally cannot
        // (review §3 -- the guard proves the engine placed what was typed, not
        // that what was typed matches the table).
        RecordedEvent::RevealOrders(cards) => {
            format!("reveal orders: {}", join_order_ids(cards))
        }
        RecordedEvent::RevealTechs(cards) => {
            let ids: Vec<String> = cards.iter().map(|c| format!("T{:02}", c.0)).collect();
            format!("reveal techs: {}", if ids.is_empty() { "(none)".to_string() } else { ids.join(", ") })
        }
        RecordedEvent::RevealStartingOrder { player, card } => {
            format!("reveal starting order: P{} = O{:02}", player.0, card.0)
        }
    }
}

/// One plain-words summary line for the pending choice heading
/// `GameState::pending_choices` -- feeds the `Choose: {...} -- {n}
/// choice(s) pending` banner `Repl::prompt` renders in place of the normal
/// phase name whenever the queue is non-empty (Task 11, design §5.4).
/// Wildcard-free match: every current and future `PendingChoice` variant
/// must be named here explicitly. `rules` is accepted for interface
/// symmetry with `describe_move`'s call site (same precedent as
/// `describe_area_action`'s `_state`) but no variant currently needs it --
/// every choice type's summary is derived from `state` and the choice's own
/// fields alone.
pub fn describe_pending_choice(head: &PendingChoice, state: &GameState, _rules: &Rules) -> String {
    match head {
        PendingChoice::AnyGood => "any-good reward: choose 1 of copper/silk/tea/fish".to_string(),
        PendingChoice::WarehouseTake => {
            let p = state.current_player();
            let parts: Vec<String> = warehouse_take_options(p)
                .into_iter()
                .map(|item| {
                    let cost = warehouse_item_cost(p, item);
                    let label = match item {
                        WarehouseItem::Assistant => "assistant",
                        WarehouseItem::Shophouse => "shophouse",
                        WarehouseItem::TradingHouse => "TH",
                    };
                    if cost == 0 { format!("{label} free") } else { format!("{label} {cost}¥") }
                })
                .collect();
            format!("warehouse take ({})", parts.join(" / "))
        }
        PendingChoice::AssistantMove => "assistant move (reward): choose a source and destination, or decline".to_string(),
        PendingChoice::ChurchSpace { total_faith } => format!("Church space at faith {total_faith}"),
        PendingChoice::CustomsSpace { cost } => format!("Customs space at cost {cost}"),
        PendingChoice::RecoveryLeave => "Telephone: leave 1 assistant behind, or recover all".to_string(),
        PendingChoice::StationPlace => "place the Station token (all players gain +1 Power in its area)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn state_2p() -> GameState {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 1).unwrap();
        finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]).unwrap()
    }

    #[test]
    fn area_label_names_type_and_id() {
        let state = state_2p();
        let some_area = *state.board_layout.areas.keys().next().unwrap();
        let label = area_label(&state, some_area);
        assert!(label.contains(&format!("(A{:02})", some_area.0)), "{label}");
    }

    /// **Every `RecordingTarget` variant's rendered shape, pinned** (T5 review
    /// S3) — the header a person reads while entering a card, at the LAST
    /// position of each run, which is where an off-by-one shows up as a board
    /// that does not exist.
    #[test]
    fn recording_targets_render_their_positions() {
        use crate::screen_model::RecordingTarget as T;
        assert_eq!(
            recording_target(T::PortSetup { board_b: false, slot: 5, of: 6 }),
            "Port A slot 6 of 6",
        );
        assert_eq!(
            recording_target(T::PortSetup { board_b: true, slot: 0, of: 6 }),
            "Port B slot 1 of 6",
        );
        assert_eq!(
            recording_target(T::PortRefill { board_b: false, slot: 5, nth: 2, of: 3 }),
            "Port A slot 6 (reveal 3 of 3)",
        );
        assert_eq!(recording_target(T::TechRefill { board_b: true }), "the card that refilled Laboratory B");
        assert_eq!(
            recording_target(T::DealSetup { seat: PlayerId::new(0), which: 1, of: 2 }),
            "Seat 0's dealt card 2 of 2",
        );
        assert_eq!(
            recording_target(T::LabSetup { board_b: false, slot: 5, of: 6 }),
            "Laboratory A slot 6 of 6",
        );
        // The last two carry IDENTITIES rather than positions, so nothing on
        // their lines is converted — an area id reads exactly as `A03` does
        // everywhere else in the driver, and a category is a letter.
        assert_eq!(
            recording_target(T::SiteSetup { area: AreaId::new(3) }),
            "the building-site card on area A03",
        );
        assert_eq!(
            recording_target(T::AchievementSetup { category: AchievementCategory::B }),
            "the category B Achievement card",
        );
        // T8's, on the same identity side: a seat, and nothing to convert.
        assert_eq!(
            recording_target(T::StartingOrder { seat: PlayerId::new(1) }),
            "Seat 1's hidden starting order",
        );
    }

    /// **Every target says what just happened in plain words** (T8, requirement
    /// R1) — the sentence above the question, which the pre-redesign prompt
    /// packed into the question itself.
    ///
    /// Swept over every variant rather than spot-checked, and asserting a SHAPE
    /// rather than a wording, because the failure this catches is a variant
    /// added with nothing to say: an empty banner line reads as a rendering bug
    /// on the one screen an operator meets while looking at the table.
    #[test]
    fn every_recording_target_states_its_event_as_a_sentence() {
        use crate::screen_model::RecordingTarget as T;
        for target in [
            T::PortRefill { board_b: false, slot: 0, nth: 0, of: 2 },
            T::TechRefill { board_b: true },
            T::PortSetup { board_b: true, slot: 0, of: 6 },
            T::DealSetup { seat: PlayerId::new(1), which: 0, of: 2 },
            T::LabSetup { board_b: false, slot: 0, of: 6 },
            T::SiteSetup { area: AreaId::new(3) },
            T::AchievementSetup { category: AchievementCategory::A },
            T::StartingOrder { seat: PlayerId::new(1) },
        ] {
            let text = reveal_event(target);
            assert!(text.ends_with('.'), "{target:?} must read as a sentence: {text:?}");
            assert!(text.len() > 20, "{target:?} said almost nothing: {text:?}");
        }
        // The two the live capture actually reaches, pinned verbatim — the
        // count is pluralized rather than printed as "1 slot(s)", which is the
        // abbreviation-shaped wording R1 exists to retire.
        assert_eq!(
            reveal_event(T::PortRefill { board_b: false, slot: 0, nth: 0, of: 1 }),
            "Port A refilled 1 slot from the order deck.",
        );
        assert_eq!(
            reveal_event(T::PortRefill { board_b: true, slot: 0, nth: 0, of: 2 }),
            "Port B refilled 2 slots from the order deck.",
        );
        assert_eq!(
            reveal_event(T::TechRefill { board_b: false }),
            "Laboratory A refilled from the technology deck.",
        );
    }

    /// **Every target names the registry its prompt collects** (T7 moved this
    /// off the REPL and onto the model, so two callers cannot file a variant
    /// differently). Swept rather than spot-checked: a new variant defaulting
    /// to the wrong registry draws a screen whose numbers cannot answer the
    /// question, which is the T6-review defect this routing exists to prevent.
    #[test]
    fn every_recording_target_names_its_own_registry() {
        use crate::screen_model::{IndexKind, RecordingTarget as T};
        for (target, want) in [
            (T::PortRefill { board_b: false, slot: 0, nth: 0, of: 1 }, IndexKind::Orders),
            (T::PortSetup { board_b: true, slot: 0, of: 6 }, IndexKind::Orders),
            (T::DealSetup { seat: PlayerId::new(1), which: 0, of: 2 }, IndexKind::Orders),
            (T::TechRefill { board_b: false }, IndexKind::Techs),
            (T::LabSetup { board_b: true, slot: 0, of: 6 }, IndexKind::Techs),
            (T::SiteSetup { area: AreaId::new(0) }, IndexKind::Sites),
            (T::AchievementSetup { category: AchievementCategory::A }, IndexKind::Achievements),
            (T::StartingOrder { seat: PlayerId::new(1) }, IndexKind::Orders),
        ] {
            assert_eq!(target.kind(), want, "{target:?}");
        }
    }

    /// **`slot 7 of 6` is unreachable from a valid target.** The pin above
    /// states four shapes; this one states the PROPERTY behind them, swept over
    /// every variant and every position in range: a rendered position is
    /// between 1 and its own total, never past it. A T6/T7 prompt that passes a
    /// count where a position goes — or pre-increments the way `PortRefill`'s
    /// `nth` used to be passed — fails here rather than on the owner's screen.
    #[test]
    fn recording_targets_render_positions_in_range() {
        use crate::screen_model::RecordingTarget as T;
        for of in 1..=6usize {
            for i in 0..of {
                // Seat 0, so the only numbers on the line are positions and
                // totals — a seat index is neither, and would muddy the sweep.
                // The count beside each case is how many POSITIONS it renders,
                // total included: 2 for a "slot N of M", 3 once a reveal number
                // joins them.
                let cases = [
                    (T::PortSetup { board_b: false, slot: i, of }, 2),
                    (T::PortRefill { board_b: true, slot: i, nth: i, of }, 3),
                    (T::DealSetup { seat: PlayerId::new(0), which: i, of }, 2),
                    (T::LabSetup { board_b: false, slot: i, of }, 2),
                ];
                for (target, positions) in cases {
                    let text = recording_target(target);
                    // Every number the header prints, in order.
                    let numbers: Vec<usize> = text
                        .split(|c: char| !c.is_ascii_digit())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.parse::<usize>().expect("a number"))
                        .collect();
                    assert!(!numbers.is_empty(), "{target:?} printed no position: {text}");
                    // Anti-vacuity: the run's total must actually be stated, so
                    // a header that dropped its "of N" cannot pass by silence.
                    assert!(numbers.contains(&of), "{target:?} must state its total: {text}");
                    // Nothing on the line may exceed the run's own total —
                    // `slot 7 of 6` is a sentence about a board that does not
                    // exist, and it is what a count passed as a position prints.
                    for &n in &numbers {
                        assert!(n <= of, "{target:?} printed {n}, past its total of {of}: {text}");
                    }
                    // ...and at the LAST position of a run, every position on
                    // the line reads as the total. This is the clause that makes
                    // the 1-based conversion load-bearing: a position rendered
                    // straight from the 0-based field prints `of - 1` here.
                    if i + 1 == of {
                        assert_eq!(
                            numbers.iter().filter(|&&n| n == of).count(),
                            positions,
                            "{target:?} must read every position as {of}: {text}",
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn five_power_effect_label_reads_as_plain_goods() {
        // One label per token face; the amounts match parse_token_letter's map.
        assert_eq!(five_power_effect_label(FivePowerEffect::ImportedGoods(1)), "1 imported goods");
        assert_eq!(five_power_effect_label(FivePowerEffect::Copper(2)), "2 copper");
        assert_eq!(five_power_effect_label(FivePowerEffect::Silk(2)), "2 silk");
        assert_eq!(five_power_effect_label(FivePowerEffect::Tea(3)), "3 tea");
        assert_eq!(five_power_effect_label(FivePowerEffect::Seafood(3)), "3 seafood");
        assert_eq!(five_power_effect_label(FivePowerEffect::Money(3)), "3 yen");
    }

    #[test]
    fn reveal_event_descriptions_carry_the_recorded_card_ids_for_audit() {
        let state = state_2p();
        let rules = Rules::standard();
        // Orders and techs list every recorded id (review §3: the `log` view is
        // the operator's audit trail against a mis-entry the guard can't catch).
        let orders = describe_recorded_event(
            &RecordedEvent::RevealOrders(vec![OrderCardId(5), OrderCardId(17)]),
            &state,
            &rules,
        );
        assert_eq!(orders, "reveal orders: O05, O17", "{orders}");
        let techs = describe_recorded_event(
            &RecordedEvent::RevealTechs(vec![TechCardId(3)]),
            &state,
            &rules,
        );
        assert_eq!(techs, "reveal techs: T03", "{techs}");
    }

    #[test]
    fn placement_moves_read_as_plain_words() {
        let state = state_2p();
        let a3 = AreaId::new(3);
        let a5 = AreaId::new(5);
        let rules = Rules::standard();
        let none = describe_move(&Move::PlaceThreeAreas { placements: vec![] }, &state, &rules);
        assert_eq!(none, "Place no assistants");
        let two = describe_move(&Move::PlaceTwoSameArea { area: a3, adjacent_bonus: Some(a5) }, &state, &rules);
        assert!(two.starts_with("Place 2 assistants in "), "{two}");
        assert!(two.contains("Telegram"), "{two}");
    }

    #[test]
    fn area_actions_read_as_plain_words() {
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::ExecuteAreaAction { action: AreaAction::ProduceCopper }, &state, &rules),
            "Produce copper"
        );
        assert_eq!(
            describe_move(&Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: 2 } }, &state, &rules),
            "Customs: pay 2 imported goods"
        );
        let skip = describe_move(&Move::ExecuteAreaAction { action: AreaAction::Skip }, &state, &rules);
        assert!(skip.contains("declin"), "{skip}");
        let ch = describe_move(
            &Move::ExecuteAreaAction {
                action: AreaAction::Chinatown { exchanges: vec![ChinatownExchange::CopperToYen] },
            },
            &state,
            &rules,
        );
        assert!(ch.contains("1 copper -> 2 yen"), "{ch}");
    }

    /// The Laboratory acquisition's `fifth_tech_payment` surcharge is the
    /// rulebook p.12 penalty on "the 5th Technology card AND LATER" (engine:
    /// `fifth_tech_penalty_required` fires once the player owns 4+ techs), so
    /// it applies unchanged to the 6th, 7th, ... acquisition too. The label
    /// must not hard-code the ordinal "5th" (which is false for a 6th/7th
    /// buy) -- "5th+" names the surcharge accurately. NOTE: this is a
    /// different surcharge from the board-position surcharge (`extra_yen`/
    /// `extra_imported`, slots 3-6 = +1,+1,+2,+2); this test pins only the
    /// 5th+ tech-CARD penalty label.
    #[test]
    fn laboratory_fifth_plus_tech_fee_is_not_hard_coded_to_the_fifth() {
        let state = state_2p();
        let rules = Rules::standard();
        let yen = describe_move(
            &Move::ExecuteAreaAction {
                action: AreaAction::Laboratory {
                    card_id: TechCardId(20), extra_yen: 0, extra_imported: 0, fifth_tech_payment: Some(true),
                },
            },
            &state,
            &rules,
        );
        assert!(yen.contains("5th+ tech fee: 2 yen"), "{yen}");
        let imported = describe_move(
            &Move::ExecuteAreaAction {
                action: AreaAction::Laboratory {
                    card_id: TechCardId(20), extra_yen: 0, extra_imported: 0, fifth_tech_payment: Some(false),
                },
            },
            &state,
            &rules,
        );
        assert!(imported.contains("5th+ tech fee: 1 imported good"), "{imported}");
    }

    #[test]
    fn warehouse_take_moves_read_as_plain_words() {
        // Fresh game: no trading houses out yet, so the first take is 4 yen.
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::ChooseWarehouseTake { item: WarehouseItem::Assistant }, &state, &rules),
            "Take an assistant from the warehouse (free)"
        );
        assert_eq!(
            describe_move(&Move::ChooseWarehouseTake { item: WarehouseItem::Shophouse }, &state, &rules),
            "Take a shophouse from the warehouse (2 yen)"
        );
        assert_eq!(
            describe_move(&Move::ChooseWarehouseTake { item: WarehouseItem::TradingHouse }, &state, &rules),
            "Take a trading house from the warehouse (4 yen)"
        );
        assert_eq!(describe_move(&Move::DeclineWarehouseTake, &state, &rules), "Decline the warehouse take");
    }

    #[test]
    fn assistant_move_moves_read_as_plain_words() {
        let state = state_2p();
        let rules = Rules::standard();
        let some_area = *state.board_layout.areas.keys().next().unwrap();

        let from_hand = describe_move(&Move::ChooseAssistantMove { from: MoveSource::Hand, to: some_area }, &state, &rules);
        assert!(from_hand.starts_with("Move an assistant from hand to "), "{from_hand}");
        assert!(from_hand.ends_with("(reward move)"), "{from_hand}");

        let from_area = describe_move(&Move::ChooseAssistantMove { from: MoveSource::Area(some_area), to: some_area }, &state, &rules);
        assert!(from_area.contains(&format!("(A{:02})", some_area.0)), "{from_area}");
        assert!(!from_area.contains("hand"), "{from_area}");

        assert_eq!(describe_move(&Move::DeclineAssistantMove, &state, &rules), "Decline the assistant move");
    }

    /// The Church space choice shows the space's FULL printed reward -- VP
    /// plus its movement rider -- in the SAME vocabulary the `tracks` view
    /// prints, single-sourced from `render::church_reward_text`. A player
    /// choosing among open faith spaces (faith-4 "3vp 1move" vs faith-6 "5vp
    /// 2move") then compares on the whole reward, not VP alone. Rulebook p.13:
    /// a Church placement grants the space's VP AND its movement-icon rider(s)
    /// ("the reward of 5 Victory points and the ability to move 2 pawns").
    #[test]
    fn church_space_choice_shows_vp_and_movement_rider() {
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::ChooseChurchSpace { space: 6 }, &state, &rules),
            format!("Place on Church faith-6 space ({})", render::church_reward_text(6))
        );
        assert_eq!(
            describe_move(&Move::ChooseChurchSpace { space: 6 }, &state, &rules),
            "Place on Church faith-6 space (5vp 2move)"
        );
        // The rider varies across the offered spaces, so it must appear.
        assert!(describe_move(&Move::ChooseChurchSpace { space: 4 }, &state, &rules).contains("1move"));
    }

    /// The Customs space choice likewise shows VP + the space's warehouse/yen
    /// rider, single-sourced from `render::customs_reward_text` (matches
    /// `tracks`). Decision-relevant: within one printed-cost pair the two
    /// spaces differ on rider -- cost-1's space 1 ("5vp 2wh", warehouse takes)
    /// vs space 2 ("4vp 2yen", yen) -- rulebook p.14. Spaces 5-7 carry no
    /// rider, so those show VP alone.
    #[test]
    fn customs_space_choice_shows_vp_and_warehouse_or_yen_rider() {
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::ChooseCustomsSpace { space: 3 }, &state, &rules),
            format!("Place on Customs space 3 ({})", render::customs_reward_text(3))
        );
        assert_eq!(
            describe_move(&Move::ChooseCustomsSpace { space: 1 }, &state, &rules),
            "Place on Customs space 1 (5vp 2wh)"
        );
        assert_eq!(
            describe_move(&Move::ChooseCustomsSpace { space: 2 }, &state, &rules),
            "Place on Customs space 2 (4vp 2yen)"
        );
        // Spaces 5-7 carry no rider: VP only.
        let s5 = describe_move(&Move::ChooseCustomsSpace { space: 5 }, &state, &rules);
        assert_eq!(s5, "Place on Customs space 5 (16vp)", "{s5}");
    }

    #[test]
    fn retreat_descriptions_show_counts_or_none() {
        let state = state_2p();
        let rules = Rules::standard();
        let none = describe_move(
            &Move::ReturnPresidentToHand { recover: std::collections::BTreeMap::new() },
            &state,
            &rules,
        );
        assert_eq!(none, "Return President to hand (recover no assistants)");

        let mut recover = std::collections::BTreeMap::new();
        recover.insert(AreaId::new(0), 2);
        recover.insert(AreaId::new(3), 1);
        let some = describe_move(&Move::ReturnPresidentToHand { recover }, &state, &rules);
        assert_eq!(some, "Return President to hand, recovering 3 assistant(s) (A00 x2, A03 x1)");
    }

    #[test]
    fn recovery_leave_choice_reads_as_plain_words() {
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::ChooseRecoveryLeave { leave_one: false }, &state, &rules),
            "Recover all assistants from the action area"
        );
        assert_eq!(
            describe_move(&Move::ChooseRecoveryLeave { leave_one: true }, &state, &rules),
            "Leave 1 assistant behind (Telephone)"
        );
    }

    #[test]
    fn station_place_choice_reads_as_plain_words() {
        let state = state_2p();
        let rules = Rules::standard();
        let some_area = *state.board_layout.areas.keys().next().unwrap();
        let text = describe_move(&Move::ChooseStationPlace { area: some_area }, &state, &rules);
        assert!(text.starts_with("Place the Station token in "), "{text}");
        assert!(text.contains(&format!("(A{:02})", some_area.0)), "{text}");
    }

    #[test]
    fn describe_pending_choice_covers_all_seven_variants() {
        let mut state = state_2p();
        let rules = Rules::standard();
        // Fresh game already has >=1 of each warehouse item type (12/6/4 in
        // the warehouse respectively); bump money so all three (including
        // the 4-yen first Trading House take) are affordable and listed,
        // regardless of which seat happens to be "current" (the start seat
        // begins with only 3 yen -- see `GameConfig::starting_money`).
        state.players[state.current_player.0 as usize].money = 10;

        let any_good = describe_pending_choice(&PendingChoice::AnyGood, &state, &rules);
        assert!(any_good.contains("any-good"), "{any_good}");

        let warehouse = describe_pending_choice(&PendingChoice::WarehouseTake, &state, &rules);
        assert!(warehouse.contains("assistant free"), "{warehouse}");
        assert!(warehouse.contains("shophouse 2¥"), "{warehouse}");
        assert!(warehouse.contains("TH 4¥"), "{warehouse}");

        let assistant_move = describe_pending_choice(&PendingChoice::AssistantMove, &state, &rules);
        assert!(assistant_move.contains("assistant move"), "{assistant_move}");

        let church = describe_pending_choice(&PendingChoice::ChurchSpace { total_faith: 6 }, &state, &rules);
        assert!(church.contains("Church") && church.contains('6'), "{church}");

        let customs = describe_pending_choice(&PendingChoice::CustomsSpace { cost: 2 }, &state, &rules);
        assert!(customs.contains("Customs") && customs.contains('2'), "{customs}");

        let recovery = describe_pending_choice(&PendingChoice::RecoveryLeave, &state, &rules);
        assert!(recovery.contains("Telephone"), "{recovery}");

        let station = describe_pending_choice(&PendingChoice::StationPlace, &state, &rules);
        assert!(station.contains("Station") && station.contains("Power"), "{station}");
    }

    #[test]
    fn university_and_spinning_mill_read_as_plain_words() {
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::UseUniversity, &state, &rules),
            "Use University: pay 1 yen, take an assistant from the warehouse"
        );
        assert_eq!(
            describe_move(&Move::UseSpinningMill, &state, &rules),
            "Use Textile Mill: discard 1 silk, gain 2 yen"
        );
    }

    /// Task 12 (T2.2a): a BuildShophouse option must show the space's
    /// PRINTED reward, not a bare "(space N)" placeholder -- the CLI must
    /// never require the player to cross-reference the physical site card
    /// themselves to know what a build actually pays out.
    #[test]
    fn build_shophouse_options_print_the_spaces_printed_reward() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let (area, site) = state
            .board_layout
            .building_sites
            .iter()
            .next()
            .map(|(&a, s)| (a, s.clone()))
            .expect("a building site exists");
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);

        let text = describe_move(&Move::BuildShophouse { slot: 0 }, &state, &rules);
        assert!(!text.ends_with("(space 1)"), "bare space-number text must be gone: {text}");
        let expected = slot_reward_text(&site, 0).expect("slot 0 is always in range");
        assert!(text.contains(&expected), "{text}");
    }

    /// Phase-2.9 per-slot enumeration fix: the Building phase now offers one
    /// `BuildShophouse` per empty slot, so each option must name WHICH space
    /// it builds on alongside that space's printed reward -- two blank
    /// spaces would otherwise render identical `(-)` options a tester at the
    /// table cannot tell apart without opening the `sites` view.
    #[test]
    fn tampered_out_of_range_shophouse_slot_describes_without_panicking() {
        // slot = 255 can only come from a tampered record; the description
        // must stay total (usize arithmetic, no u8 overflow) — Stage-1
        // review finding 5.
        let state = state_2p();
        let rules = Rules::standard();
        let text = describe_move(&Move::BuildShophouse { slot: 255 }, &state, &rules);
        assert!(text.contains("space 256"), "total fallback wording: {text}");
    }

    #[test]
    fn build_shophouse_options_identify_the_slot_and_its_reward() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let (area, site) = state
            .board_layout
            .building_sites
            .iter()
            .next()
            .map(|(&a, s)| (a, s.clone()))
            .expect("a building site exists");
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);

        let mut texts = Vec::new();
        for slot in 0u8..4 {
            let text = describe_move(&Move::BuildShophouse { slot }, &state, &rules);
            assert!(
                text.contains(&format!("space {}", slot + 1)),
                "option must name its space (1-based): {text}"
            );
            let reward = slot_reward_text(&site, slot as usize).expect("slot in range");
            assert!(text.contains(&reward), "option must keep the printed reward: {text}");
            texts.push(text);
        }
        for (i, a) in texts.iter().enumerate() {
            for b in texts.iter().skip(i + 1) {
                assert_ne!(a, b, "per-slot options must be pairwise distinguishable");
            }
        }
    }

    /// Task 12 riding item: BuildTradingHouse should likewise show the
    /// site's printed Trading-House VP rather than a bare label.
    #[test]
    fn build_trading_house_option_prints_the_sites_th_reward() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let (area, site) = state
            .board_layout
            .building_sites
            .iter()
            .next()
            .map(|(&a, s)| (a, s.clone()))
            .expect("a building site exists");
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);

        let text = describe_move(&Move::BuildTradingHouse, &state, &rules);
        assert!(text.contains(&format!("{}vp", site.trading_house_reward)), "{text}");
    }

    /// Task 12 (T2.2a): production-area action options must preview the
    /// yield at the CURRENT power, computed the same way the engine
    /// computes it (`calculate_power`) -- not just named.
    #[test]
    fn production_area_actions_preview_the_yield() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let area = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type == AreaType::TeaPlantation)
            .expect("a Tea Plantation area exists")
            .0;
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);
        state.players[cp.0 as usize].assistants.on_board.insert(area, 2);
        state.players[cp.0 as usize].assistants.in_hand -= 2;
        // President (1) + 2 assistants = power 3; Tea Plantation is a flat
        // 1:1 reward, so power 3 -> +3 tea.

        let text =
            describe_move(&Move::ExecuteAreaAction { action: AreaAction::ProduceTea }, &state, &rules);
        assert!(text.contains("power 3"), "{text}");
        assert!(text.contains("+3 tea"), "{text}");
    }

    /// Item 3 (third review): the Bank action option previews its yield (1 yen
    /// per power, capped at 5) at the current power, mirroring the production
    /// and Church previews -- it used to read only "Take money at the Bank",
    /// showing no amount.
    #[test]
    fn bank_action_previews_the_yen_yield() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let area = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type == AreaType::Bank)
            .expect("a Bank area exists")
            .0;
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);
        state.players[cp.0 as usize].assistants.on_board.insert(area, 2);
        state.players[cp.0 as usize].assistants.in_hand -= 2;
        // President (1) + 2 assistants = power 3 -> +3 yen (bank_reward is linear to 5).
        let text = describe_move(&Move::ExecuteAreaAction { action: AreaAction::Bank }, &state, &rules);
        assert!(text.contains("power 3"), "{text}");
        assert!(text.contains("+3 yen"), "{text}");
    }

    /// Item 3 (third review): the Customs action option previews the space(s)
    /// it lands on and their printed VP, mirroring Church's faith preview -- it
    /// used to name only the imported goods paid, previewing nothing gained.
    /// (The applied delta already shows the imports spent and the placement;
    /// this closes the OPTION-preview gap the other area actions don't have.)
    #[test]
    fn customs_action_previews_the_space_and_vp() {
        let mut state = state_2p();
        let rules = Rules::standard();
        // The preview shows the outcome only in an acting context (like the
        // power previews): put the President on the Customs area.
        let area = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type == AreaType::Customs)
            .expect("a Customs area exists")
            .0;
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);
        // On a fresh 2P track, paying 1 imported good is eligible for the two
        // cost-1 spaces (1 and 2); the dummy sits on space 4 (cost 2) and does
        // not block them. vp_reward(1)=5, vp_reward(2)=4.
        let text = describe_move(
            &Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: 1 } },
            &state,
            &rules,
        );
        assert!(text.contains("pay 1 imported good"), "still names the cost: {text}");
        assert!(
            text.contains(&format!("+{} VP", CustomsTrack::vp_reward(1))),
            "previews space 1's VP: {text}"
        );
        assert!(
            text.contains(&format!("+{} VP", CustomsTrack::vp_reward(2))),
            "previews space 2's VP: {text}"
        );
    }

    /// Task 12 correctness guard: Copper Mine's reward table is NOT 1:1
    /// with power (power 3 -> only 1 copper, per `copper_mine_reward`) --
    /// the preview must show the REAL yield, not echo the power number, or
    /// it would actively mislead a player about what the build pays out.
    #[test]
    fn production_preview_uses_the_real_non_linear_reward_not_the_power_number() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let area = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type == AreaType::CopperMine)
            .expect("a Copper Mine area exists")
            .0;
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);
        state.players[cp.0 as usize].assistants.on_board.insert(area, 2);
        state.players[cp.0 as usize].assistants.in_hand -= 2;
        // President (1) + 2 assistants = power 3; Copper Mine at power 3 -> 1 copper.

        let text =
            describe_move(&Move::ExecuteAreaAction { action: AreaAction::ProduceCopper }, &state, &rules);
        assert!(text.contains("power 3"), "{text}");
        assert!(text.contains("+1 copper"), "{text}");
        assert!(!text.contains("+3 copper"), "{text}");
    }

    /// Labs country + effect (pre-gate view batch item 5): `tech_effect_text`
    /// is a TOTAL match over `TechEffect` (no wildcard arm -- a future variant
    /// is a compile error). Every real tech card must therefore map to a
    /// non-empty effect line. Driven off the standard registry, which carries
    /// all 20 effects.
    #[test]
    fn every_tech_effect_has_nonempty_text() {
        let rules = Rules::standard();
        for card in rules.techs.iter() {
            assert!(!tech_effect_text(card.effect).is_empty(), "{:?} has empty effect text", card.effect);
        }
    }

    /// Spot-check a few effect lines against the engine's `TechEffect` docs so
    /// a regression in the mapping is caught, not just emptiness.
    #[test]
    fn tech_effect_text_spot_checks() {
        assert_eq!(tech_effect_text(TechEffect::Exposition), "Immediately gain 10 VP when acquired");
        assert!(tech_effect_text(TechEffect::Gaslight).contains("Shophouse"), "{}", tech_effect_text(TechEffect::Gaslight));
        assert!(tech_effect_text(TechEffect::University).contains("Warehouse"), "{}", tech_effect_text(TechEffect::University));

        // Winery/Brickyard fire on the Church/Customs *area action* (which
        // seats a pawn on the faith/space track), NOT the Placement step --
        // rulebook p.15/p.16 "Whenever you carry out a Church/Customs action
        // and place an Assistant pawn". "place an assistant on the Church"
        // collided with the Placement step's own assistant placement, so the
        // disambiguating word is "action": pin that both name it.
        let winery = tech_effect_text(TechEffect::Winery);
        assert!(winery.contains("+3 VP"), "{winery}");
        assert!(winery.contains("Church action"), "Winery fires on the Church ACTION, not the Placement step (rulebook p.15): {winery}");
        let brickyard = tech_effect_text(TechEffect::Brickyard);
        assert!(brickyard.contains("+4 VP"), "{brickyard}");
        assert!(brickyard.contains("Customs action"), "Brickyard fires on the Customs ACTION, not the Placement step (rulebook p.16): {brickyard}");
    }

    /// F2 (Codex adversarial review of PR #4): `tech_effect_text` is the
    /// ONLY player-facing statement of what a technology does -- the `labs`
    /// view exists so a player can choose WITHOUT an external reference --
    /// so an entry that drops a cost, a restriction or a timing qualifier
    /// changes a purchase decision on wrong information. The coverage
    /// before this was "every effect is non-empty", which is exactly why
    /// five materially incomplete entries shipped; these assertions are
    /// card-SPECIFIC so a future edit that drops a qualifier fails here
    /// instead of reaching a player.
    ///
    /// Rules sources for the wording (never the engine's own logic):
    /// rulebook pp. 15-16 (Technology Cards), the KB reference
    /// `analysis-1/yokohama-kb/01-rules-reference.md`, and guidingpath's
    /// decision log for ruling A4 (Patent System is SELF-EXCLUSIVE,
    /// 2026-07-20, rulebook p.16 "after this one").
    #[test]
    fn tech_effect_text_carries_the_qualifiers_that_change_a_purchase() {
        // A4: "Whenever you gain a Technology card AFTER THIS ONE..." --
        // buying Patent System never pays its own bonus. Same class for
        // Letterpress Printing (rulebook p.15, identical clause).
        let patent = tech_effect_text(TechEffect::PatentSystem);
        assert!(patent.contains("+2 VP"), "{patent}");
        assert!(
            patent.contains("after this one"),
            "Patent System is self-exclusive (ruling A4, rulebook p.16): {patent}"
        );
        let letterpress = tech_effect_text(TechEffect::LetterPressPrinting);
        assert!(letterpress.contains("+1 Yen"), "{letterpress}");
        assert!(
            letterpress.contains("after this one"),
            "Letterpress Printing is self-exclusive (rulebook p.15): {letterpress}"
        );

        // Postal System buys the RIGHT to stop, not a free stop: rulebook
        // p.15 "You must still pay THESE PLAYERS (unless you also own the
        // Stagecoach)" -- the fee goes to the owner(s) of the President
        // pawn(s) you stop on (the standing movement-fee recipient, 1 Yen
        // each), not the bank. The engine prices that destination fee in
        // `route_fee_vector`. The line must NAME that recipient, not just
        // say a fee is owed.
        let postal = tech_effect_text(TechEffect::PostalSystem);
        assert!(
            postal.contains("fee") || postal.contains("pay"),
            "Postal System still pays the players it stops on (rulebook p.15): {postal}"
        );
        assert!(
            postal.contains("owner"),
            "Postal System's stop fee goes to the President's OWNER, and the line must name that (rulebook p.15 'pay these players'): {postal}"
        );
        assert!(
            postal.contains("Stagecoach"),
            "Postal System's fee is waived only by Stagecoach (rulebook p.15): {postal}"
        );

        // Stagecoach waives the pass-through fee AND (rulebook p.15's
        // "unless you also own the Postal System" pairing, engine:
        // "Stagecoach waives president fees entirely") Postal System's
        // destination fee -- and it never lets you stop on its own.
        let stagecoach = tech_effect_text(TechEffect::Stagecoach);
        assert!(
            stagecoach.contains("Postal System"),
            "Stagecoach's value depends on the Postal System interaction (rulebook p.15): {stagecoach}"
        );

        // Station is a PERMANENT free stop at one area, not a convenience:
        // rulebook p.15 "You may move your President pawn to this area
        // during the Movement Step for free, AND YOU DO NOT NEED TO PAY EVEN
        // IF THERE ARE OTHER PRESIDENT PAWNS IN THE AREA" -- the clause the
        // line dropped (third review, I4). Without it Station reads as a
        // conditional shortcut; with it, it is a standing Postal System PLUS
        // Stagecoach for that one area, which is what justifies a 4-industry
        // buy. The engine agrees three ways: `movement.rs`'s station arm
        // never checks for an opponent President, the empty-path teleport is
        // exempted from the Postal destination fee, and `legal_moves.rs`
        // places no block on acting there. Ruling A1 (2026-07-20) retracted
        // the broader "no assistants needed" reading but PRESERVED this one
        // ("The card's explicit clauses stand (free; no payment for other
        // President pawns there)"), so the own-assistant precondition it
        // confirmed must survive the rewrite too.
        let station = tech_effect_text(TechEffect::Station);
        assert!(
            station.contains("President fee") || station.contains("no fee"),
            "Station charges NOTHING for other Presidents standing at the Station area \
             (rulebook p.15; ruling A1 preserved the clause) -- without it the card reads as \
             a conditional convenience: {station}"
        );
        assert!(
            station.contains("free"),
            "Station's move is free (rulebook p.15): {station}"
        );
        assert!(
            station.contains("assistant"),
            "Station still requires one of YOUR assistants at the Station area \
             (ruling A1 Reading 2, 2026-07-20): {station}"
        );

        // Telegram's bonus assistant is not free: rulebook p.16 "Pay the
        // usual cost if another President pawn occupies this adjacent Area."
        let telegram = tech_effect_text(TechEffect::Telegram);
        assert!(
            telegram.contains("fee") || telegram.contains("cost"),
            "Telegram's bonus placement pays the normal President fee (rulebook p.16): {telegram}"
        );

        // Mining Technology pays ONCE per gain, not once per token
        // (rulebook p.15 "Whenever you receive 1 or more Copper token, you
        // receive 1 additional Copper token"; engine `copper_bonus` returns
        // the owned-copy count, independent of `base_copper`).
        let mining = tech_effect_text(TechEffect::MiningTechnology);
        assert!(
            mining.contains("1 or more") && mining.contains("not per token"),
            "Mining Technology pays per GAIN, not per Copper token (rulebook p.15): {mining}"
        );
    }

    /// Task B review minor: the Station effect line must not promise an
    /// UNCONDITIONAL teleport -- the engine gates it on the owner holding an
    /// assistant in the Station's area (`can_teleport_to_station`,
    /// movement.rs:257, in `can_teleport_to_station_with_tech`: `in_area(station_area) > 0`). The one-liner must
    /// therefore carry that assistant-presence gate, not just say "teleport
    /// to it".
    #[test]
    fn station_effect_line_carries_the_teleport_gate() {
        let line = tech_effect_text(TechEffect::Station);
        assert!(line.contains("teleport"), "{line}");
        assert!(line.contains("assistant"), "teleport must read as gated on an assistant: {line}");
    }

    #[test]
    fn every_phase_has_a_plain_name() {
        for p in [
            TurnPhase::AdditionalActionFirst, TurnPhase::Placement, TurnPhase::Movement,
            TurnPhase::AreaAction, TurnPhase::FivePowerBonus, TurnPhase::Building,
            TurnPhase::Recovery, TurnPhase::AdditionalActionSecond, TurnPhase::TurnEnd,
        ] {
            assert!(!phase_name(p).is_empty());
        }
    }

    /// Task 6 (decision-surface previews): placement options carry a fee
    /// preview naming amount and recipient(s) whenever an opponent President
    /// occupies a chosen area -- computed from the same public-state read
    /// the engine's `calculate_placement_cost`/`credit_placement_fees` pair
    /// makes (1 yen per assistant placed there, per owner).
    #[test]
    fn placement_options_preview_the_fee_with_recipients() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let area = *state.board_layout.areas.keys().next().unwrap();
        let other = *state.board_layout.areas.keys().nth(1).unwrap();
        // Blue's President sits in `area`; Red is current.
        state.players[1].president_location = PresidentLocation::OnBoard(area);

        let a = describe_move(&Move::PlaceThreeAreas { placements: vec![area, other] }, &state, &rules);
        assert!(a.contains("fee 1 yen (1 to Blue)"), "{a}");

        let b = describe_move(&Move::PlaceTwoSameArea { area, adjacent_bonus: None }, &state, &rules);
        assert!(b.contains("fee 2 yen (2 to Blue)"), "Option B places 2 assistants: {b}");

        // Telegram's bonus assistant pays the same per-assistant fee
        // (apply_place_two_same_area routes it through apply_placement).
        let t = describe_move(&Move::PlaceTwoSameArea { area: other, adjacent_bonus: Some(area) }, &state, &rules);
        assert!(t.contains("fee 1 yen (1 to Blue)"), "{t}");

        // No opponent President anywhere on the path: no fee text at all.
        let free = describe_move(&Move::PlaceThreeAreas { placements: vec![other] }, &state, &rules);
        assert!(!free.contains("fee"), "{free}");
    }

    /// The previewed fee must equal what the engine actually charges: apply
    /// the placement and compare the payer's money delta.
    #[test]
    fn placement_fee_preview_matches_the_engine_charge() {
        let rules = Rules::standard();
        let mut state = state_2p();
        state.turn_phase = TurnPhase::Placement;
        let area = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type != AreaType::Canal)
            .unwrap()
            .0;
        state.players[1].president_location = PresidentLocation::OnBoard(area);

        let mv = Move::PlaceTwoSameArea { area, adjacent_bonus: None };
        let text = describe_move(&mv, &state, &rules);
        let money_before = state.current_player().money;
        let after = apply_move_unchecked(&state, &mv, &rules).expect("placement applies");
        let charged = money_before - after.players[state.current_player.0 as usize].money;
        assert_eq!(charged, 2, "fixture: two assistants into Blue's area");
        assert!(text.contains(&format!("fee {charged} yen")), "preview must equal the charge: {text}");
    }

    /// Task 6: Employment Agency options preview their total cost --
    /// assistants free, shophouses 2 yen each, trading houses at the printed
    /// 4/5/6/7 progression -- and the previewed number equals the engine's
    /// actual charge.
    #[test]
    fn employment_agency_option_previews_cost_matching_the_engine() {
        let rules = Rules::standard();
        let mut state = state_2p();
        let ea = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == AreaType::EmploymentAgency)
            .expect("2P board has an Employment Agency")
            .0;
        let me = state.current_player;
        state.players[me.0 as usize].president_location = PresidentLocation::OnBoard(ea);
        state.players[me.0 as usize].money = 10;
        // Power 5 (president + 4 assistants): the EA item cap is a power
        // table (`employment_agency_tokens_allowed`, 5+ -> 4 items), and
        // this composite takes 3.
        state.players[me.0 as usize].assistants.on_board.insert(ea, 4);
        state.players[me.0 as usize].assistants.in_hand -= 4;
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(calculate_power(&state, me, ea));

        let action = AreaAction::EmploymentAgency { assistants: 1, shophouses: 1, trading_houses: 1 };
        let mv = Move::ExecuteAreaAction { action };
        let text = describe_move(&mv, &state, &rules);
        assert!(text.contains("cost 6 yen"), "0 + 2 + first-TH 4: {text}");

        let after = apply_move_unchecked(&state, &mv, &rules).expect("EA composite applies");
        let charged = 10 - after.players[me.0 as usize].money;
        assert_eq!(charged, 6, "preview must equal the engine's charge");

        // A free take (assistants only) shows no cost suffix.
        let free = describe_move(
            &Move::ExecuteAreaAction {
                action: AreaAction::EmploymentAgency { assistants: 2, shophouses: 0, trading_houses: 0 },
            },
            &state,
            &rules,
        );
        assert!(free.contains("(free)"), "{free}");
    }

    /// Task 6: Church options preview the resulting faith (engine's own
    /// `calculate_power` + `calculate_church_faith`), and the number matches
    /// the `ChurchSpace { total_faith }` choice the engine actually queues.
    #[test]
    fn church_option_previews_faith_matching_the_engine_queue() {
        let rules = Rules::standard();
        let mut state = state_2p();
        let church = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == AreaType::Church)
            .expect("2P board has a Church")
            .0;
        let me = state.current_player;
        state.players[me.0 as usize].president_location = PresidentLocation::OnBoard(church);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(calculate_power(&state, me, church));

        // President alone = power 1 -> faith 1, +3 payment kinds = faith 4.
        let action = AreaAction::Church { copper: true, silk: true, tea: false, seafood: false, yen: true, imported_good: false };
        let mv = Move::ExecuteAreaAction { action };
        let text = describe_move(&mv, &state, &rules);
        assert!(text.contains("-> faith 4"), "{text}");
        assert!(text.contains("power 1 + 3 kinds"), "{text}");

        let after = apply_move_unchecked(&state, &mv, &rules).expect("church action applies");
        match after.pending_choices.first() {
            Some(PendingChoice::ChurchSpace { total_faith }) => {
                assert_eq!(*total_faith, 4, "preview must equal the queued faith")
            }
            other => {
                // Faith 4 with only space 4 eligible can resolve inline; the
                // placement then lands on faith-4 directly.
                assert!(
                    after.players[me.0 as usize].church_positions.contains(&4) || other.is_some(),
                    "faith-4 outcome expected, got {other:?}"
                );
            }
        }
    }

    /// Task 6: production previews carry threshold context -- the next power
    /// step that increases the yield -- so `+0 copper` reads as "not yet",
    /// not as a bug.
    #[test]
    fn production_preview_names_the_next_threshold() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let mine = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type == AreaType::CopperMine)
            .expect("a Copper Mine area exists")
            .0;
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(mine);
        state.players[cp.0 as usize].assistants.on_board.insert(mine, 1);
        state.players[cp.0 as usize].assistants.in_hand -= 1;
        // President (1) + 1 assistant = power 2; copper_mine_reward(2) = 0,
        // and the next step is +1 at power 3.
        let text = describe_move(&Move::ExecuteAreaAction { action: AreaAction::ProduceCopper }, &state, &rules);
        assert!(text.contains("power 2 -> +0 copper"), "{text}");
        assert!(text.contains("next +1 at power 3"), "{text}");
    }

    /// At the table cap (power 5+ on every production table) there is no
    /// next step, so no threshold clause prints.
    #[test]
    fn production_preview_omits_the_threshold_at_cap() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let mine = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type == AreaType::CopperMine)
            .expect("a Copper Mine area exists")
            .0;
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(mine);
        state.players[cp.0 as usize].assistants.on_board.insert(mine, 5);
        state.players[cp.0 as usize].assistants.in_hand -= 5;
        // Power 6 -> copper capped at 3; no further step exists.
        let text = describe_move(&Move::ExecuteAreaAction { action: AreaAction::ProduceCopper }, &state, &rules);
        assert!(text.contains("+3 copper"), "{text}");
        assert!(!text.contains("next +"), "{text}");
    }

    /// Task 13 (T2.2b): a small hand-built board for MovePresident cost
    /// tests. `state_2p()` can't reach a canal at all -- the standard
    /// 2-player board never places a Canal tile (board_gen.rs's
    /// `two_player_boards_unchanged` pins `canal_crossings.is_empty()` for
    /// every seeded 2p layout) -- so this mirrors movement.rs's Task 6
    /// fixture pattern (`canal_crossing_fixture` / `l2_diamond_fixture`)
    /// locally: those are `#[cfg(test)]`-private to yokohama-core and not
    /// reachable across the crate boundary. `route_fee_vector` only reads
    /// `canal_crossings` and `president_location`s (never `adjacency`), so
    /// the board is deliberately minimal: a `start` hub with three spokes --
    /// one tolled, one fee-bearing (Blue's President sits there), one free.
    fn movement_cost_fixture() -> (GameState, Rules) {
        let start = AreaId::new(0);
        let toll_dest = AreaId::new(1);
        let fee_dest = AreaId::new(2);
        let free_dest = AreaId::new(3);

        let mut areas = BTreeMap::new();
        areas.insert(start, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(toll_dest, AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        areas.insert(fee_dest, AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 2) });
        areas.insert(free_dest, AreaConfig { area_type: AreaType::TeaPlantation, grid_position: (0, 3) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(start, vec![toll_dest, fee_dest, free_dest]);
        adjacency.insert(toll_dest, vec![start, fee_dest]);
        adjacency.insert(fee_dest, vec![start, toll_dest, free_dest]);
        adjacency.insert(free_dest, vec![start, fee_dest]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![(start, toll_dest)],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        let mut players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];
        players[0].president_location = PresidentLocation::OnBoard(start);
        players[0].money = 10;
        players[1].president_location = PresidentLocation::OnBoard(fee_dest); // Blue blocks fee_dest
        players[1].money = 10;

        let state = GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::Movement,
            turn_number: 1,
            additional_action_state: AdditionalActionState::new(),
            current_action_power: None,
            is_mercenary_action: false,
            mercenary_target_area: None,
            mercenary_return_phase: None,
            president_start_area: None,
            port_state: PortState::new(false),
            lab_state: LabState::new(false),
            order_deck: vec![],
            tech_deck: vec![],
            orders_removed: vec![],
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

        (state, Rules::empty_for_tests())
    }

    /// Task 13 (T2.2b): a MovePresident option must show WHO gets paid, not
    /// just how much -- a player choosing between two equal-cost routes
    /// needs to see that one pays an opponent and the other pays the bank
    /// (canal toll) before picking. Also pins that a non-empty zero-cost
    /// path reads as "free", NOT the teleport wording (that stays reserved
    /// for the empty-path Station/Tram case).
    #[test]
    fn movement_descriptions_name_cost_and_recipients() {
        let (state, rules) = movement_cost_fixture();
        let toll_dest = AreaId::new(1);
        let fee_dest = AreaId::new(2);
        let free_dest = AreaId::new(3);

        // Canal fixture -- the toll route:
        let toll_move = Move::MovePresident { path: vec![toll_dest], destination: toll_dest };
        let text = describe_move(&toll_move, &state, &rules);
        assert!(text.contains("canal toll"), "{text}");

        // the fee route (Blue's President sits at fee_dest):
        let fee_move = Move::MovePresident { path: vec![fee_dest], destination: fee_dest };
        let text2 = describe_move(&fee_move, &state, &rules);
        assert!(text2.contains("1 to "), "{text2}");
        assert!(text2.contains("1 to Blue"), "{text2}");

        // a free move: no canal, no opponent -- and it must NOT read as a
        // teleport (non-empty path):
        let free_move = Move::MovePresident { path: vec![free_dest], destination: free_dest };
        let text3 = describe_move(&free_move, &state, &rules);
        assert!(text3.contains("free"), "{text3}");
        assert!(!text3.contains("teleport"), "{text3}");

        // both a toll AND a fee on the same route (path through toll_dest,
        // stopping where Blue's President sits) -- pins the exact wording
        // and the players-before-canal-toll part ordering.
        let combo_move = Move::MovePresident { path: vec![toll_dest, fee_dest], destination: fee_dest };
        let text4 = describe_move(&combo_move, &state, &rules);
        assert_eq!(
            text4,
            format!("Move President to {} — pay 2 yen (1 to Blue, 1 canal toll)", area_label(&state, fee_dest))
        );
    }
}
