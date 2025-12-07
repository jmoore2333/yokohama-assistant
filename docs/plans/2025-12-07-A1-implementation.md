# A1-Implementation: Yokohama Game Engine Logic

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement complete game rules: legal move generation, state transitions, area actions, and scoring. The primary consumers of `legal_moves` and `apply_move` are the Rust MCTS engine and external tools (e.g., AR assistant), which rely on these functions for high-throughput game tree exploration.

**Architecture:** Build on A1-Foundation types. Add `game` module with rules engine, `actions` module for area-specific logic, `validation` module for legality checks. All functions are pure (take state, return new state or results). Designed for fast simulation with minimal allocations.

**Tech Stack:** Rust, no additional dependencies beyond serde.

**Prerequisites:** Complete A1-Foundation plan first.

**Reference Docs:**
- `analysis-1/yokohama-kb/01-rules-reference.md` - Turn structure and rules
- `analysis-1/yokohama-kb/02-quick-reference.md` - Power/reward tables

---

## Architecture Overview

### Primary Consumers
The primary consumers of `legal_moves` and `apply_move` are:
- **Rust MCTS engine**: High-throughput game tree exploration (thousands of calls/second)
- **AR assistant**: State validation and move recommendations
- **External tools**: Replay analysis, game logging

### Information Model
- The engine is **deterministic**: given an initial `GameState` (with all decks fully specified), the entire game tree contains no further randomness
- Hidden information is only represented for UI/UX purposes; MCTS operates on the fully known state
- All state transitions are pure functions (`state + move → new_state`)

### Module to Rules Mapping

| Module | Rules Coverage (Areas / Sections) |
|--------|-----------------------------------|
| `game/power.rs` | Power calculation (President, Assistants, Buildings, Station) |
| `game/movement.rs` | Movement step, pathfinding, Tram/Stagecoach effects |
| `game/legal_moves.rs` | Legal move generation for all phases |
| `game/apply_move.rs` | State transitions for all move types |
| `game/scoring.rs` | Final scoring (tracks, techs, countries, remainder) |
| `game/tech_effects.rs` | Technology card effect application |
| `actions/production.rs` | Copper Mine, Silk Mill, Tea Plantation, Fishing Ground |
| `actions/commercial.rs` | Bank, Employment Agency, Chinatown |
| `actions/cards.rs` | Port (Orders), Laboratory (Tech) |
| `actions/tracks.rs` | Church, Customs |

### Technology Integration
**All** the following phases consult `tech_effects`:
- **Movement**: Tram teleport, Stagecoach free passage, Postal System stopping
- **Power calculation**: Station (+1 to all players in area)
- **Placement**: Newspaper (4 areas), Telegram (bonus adjacent)
- **Building**: Gaslight (+2 VP), Electrical Light (+3 VP)
- **Order fulfillment**: Ball (+2 VP), Language School (+1 yen)
- **Track placement**: Winery (+3 VP), Brickyard (+4 VP)
- **Tech acquisition**: Patent System (+2 VP), Letterpress Printing (+1 yen)
- **Costs/discounts**: Various techs modify costs

Pattern: `apply_move` builds an `ActionContext` that passes through `tech_effects` helpers to derive modified options or effects.

### Turn State Machine

```
┌─────────────────────────────────────────────────────────────────┐
│                         PLAYER TURN                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  AdditionalActionFirst ─────────────────────────────────────┐   │
│        │ (Mercenary, Achievement, Order)                    │   │
│        ▼                                                    │   │
│  ┌─ Placement ──▶ Movement ──▶ AreaAction ──▶ 5-Power ─┐    │   │
│  │                                                     │    │   │
│  │           MAIN ACTION (Steps 1-6)                   │    │   │
│  │                                                     ▼    │   │
│  └─────────────────────────────────────── Building ────┘    │   │
│                                               │             │   │
│                                               ▼             │   │
│                                           Recovery          │   │
│                                               │             │   │
│                                               ▼             │   │
│  AdditionalActionSecond ◀───────────────────┘             │   │
│        │ (Mercenary, Achievement, Order)                    │   │
│        ▼                                                    │   │
│     [Next Player]                                           │   │
└─────────────────────────────────────────────────────────────────┘

MERCENARY SUB-TURN (when UseMercenary is played):
  AreaAction ──▶ 5-Power ──▶ Building ──▶ Recovery ──▶ [Return to AdditionalAction]
```

### Move Atomicity
A single `Move` represents **one decision point**:
- `Move::PlaceThreeAreas { areas }` - Complete placement decision
- `Move::MovePresident { path, destination }` - Complete movement decision
- `Move::ExecuteAreaAction { action }` - Complete area action with all parameters
- `Move::BuildShophouse { slot }` / `Move::BuildTradingHouse` - Building decision
- `Move::UseMercenary { country, target }` - Initiates mercenary sub-turn

This granularity matches what the assistant UI needs for "step-by-step" recommendations while keeping the state machine simple.

### Performance Considerations
For MCTS throughput:
- `GameState` should be **cheap to clone** (prefer fixed-size arrays, minimize `Vec` in hot paths)
- `apply_move` should **allocate minimally** (ideally zero allocations in main loop)
- `legal_moves` returns small `Vec`s; consider stack-allocated `SmallVec` for common cases
- Target: **10,000+ apply_move calls per second** on modern hardware

### Validation vs Admin Changes
The `validation` module is **rules-level**:
- Checks `Move` objects that represent in-game actions
- Assumes `GameState` is already internally consistent
- Returns `Result<(), ValidationError>` for illegal moves

**Higher-level layers** (outside `yokohama-core`) may apply "admin edits":
- Adjusting resources to match physical game state
- Repositioning buildings after CV correction
- These bypass validation to sync engine state with camera/player agreement

### End-Game Triggers
The game ends when ANY of these occur:

| Trigger | Condition |
|---------|-----------|
| Trading Houses | Any player builds their 4th Trading House |
| Shophouses | Any player builds their 8th Shophouse |
| Order Deck | Order deck empties AND an Order slot on Port is empty |
| Church Track | Required assistants placed (2P: 4, 3P: 5, 4P: 6) |
| Customs Track | Required assistants placed (2P: 4, 3P: 5, 4P: 6) |

**Procedure**:
1. Complete current round (all players finish until start player's turn would begin)
2. Play one final round (all players take one more turn)
3. Proceed to final scoring

### Final Scoring Checklist
VP sources to implement in `scoring.rs`:

- [ ] **Orders completed**: Base VP from each completed order
- [ ] **Tech cards**: VP from tech effects (e.g., Patent System ongoing)
- [ ] **Buildings on board**: VP printed on building site cards
- [ ] **Church majority**: 1st place: 6 VP, 2nd place: 3 VP
- [ ] **Customs majority**: 1st place: 8 VP, 2nd place: 4 VP
- [ ] **Technology majority**: 1st place: 10 VP, 2nd place: 5 VP (by total Industry Value)
- [ ] **Country set bonuses**: 5 different: 12 VP, 4: 8 VP, 3: 4 VP, 2: 2 VP
- [ ] **5-Power tokens**: Any that grant VP
- [ ] **Unused mercenaries**: 1 VP each
- [ ] **Imported goods**: 1 VP each
- [ ] **Remaining yen**: 1 VP per 2 yen
- [ ] **Remaining trade goods**: 1 VP per 3 goods (any combination)

Tie-breakers:
- Church/Customs: Rightmost assistant on track
- Technology: Closest to start player (clockwise)

### Card Registries as CV Integration Points
`OrderRegistry` and `TechRegistry` provide:
- **Rules metadata**: Costs, countries, icons, VP, effects
- **Stable identifiers**: Short string codes (e.g., `"order_eng_01"`, `"tech_tram"`) used by CV and UI layers
- **Single source of truth**: All card definitions live here; no duplication in other services

Example:
```rust
impl OrderRegistry {
    /// Get card by stable identifier (for CV integration)
    pub fn by_code(&self, code: &str) -> Option<&OrderCard>;

    /// Get card by numeric ID (for engine)
    pub fn by_id(&self, id: OrderCardId) -> Option<&OrderCard>;
}
```

---

## Task 1: Create Game Module Structure

**Files:**
- Create: `crates/yokohama-core/src/game/mod.rs`
- Create: `crates/yokohama-core/src/game/power.rs`
- Modify: `crates/yokohama-core/src/lib.rs`

**Step 1: Create game module directory**

Run:
```bash
mkdir -p crates/yokohama-core/src/game
```

**Step 2: Create module file**

```rust
//! Game rules engine for Yokohama.

mod power;

pub use power::*;
```

**Step 3: Implement power calculation**

Reference: `02-quick-reference.md` "Power Calculation" section

```rust
//! Power calculation for area actions.

use crate::types::{AreaId, BoardLayout, GameState, PlayerId};

/// Calculate power for a player in an area.
///
/// Power calculation order (per rulebook):
/// 1. President pawn (+1)
/// 2. Assistants (+1 each)
/// 3. Shophouse (+1)
/// 4. Trading House (+1)
/// 5. Station token (+1 for ALL players if present)
/// 6. Tech modifiers (future: if any tech adds power)
/// 7. Cap at 5 (ALWAYS applied last)
///
/// Note: All modifiers are applied BEFORE the cap.
pub fn calculate_power(state: &GameState, player: PlayerId, area: AreaId) -> u8 {
    let player_state = state.player(player);
    let mut power: u8 = 0;

    // 1. President pawn (+1)
    if player_state.president_location == crate::types::PresidentLocation::OnBoard(area) {
        power += 1;
    }

    // 2. Assistants (+1 each)
    power += player_state.assistants.in_area(area);

    // 3. Shophouse (+1)
    if player_state.shophouses.built.contains_key(&area) {
        power += 1;
    }

    // 4. Trading House (+1)
    if player_state.trading_houses.built.contains(&area) {
        power += 1;
    }

    // 5. Station token (+1 for ALL players if present)
    if state.station_location == Some(area) {
        power += 1;
    }

    // 6. Tech modifiers would go here (none in base game add power)

    // 7. Cap at 5 (ALWAYS applied last, after all modifiers)
    power.min(5)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::collections::HashMap;

    fn create_test_state() -> GameState {
        let config = GameConfig::new(
            2,
            vec![PlayerColor::Red, PlayerColor::Blue],
            PlayerId::new(0),
        );

        let mut areas = HashMap::new();
        areas.insert(AreaId::new(0), AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });

        let board_layout = BoardLayout {
            areas,
            adjacency: HashMap::new(),
            canal_crossings: vec![],
            building_sites: HashMap::new(),
            five_power_tokens: HashMap::new(),
        };

        let players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];

        GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::Placement,
            turn_number: 1,
            additional_action_state: AdditionalActionState::new(),
            current_action_power: None,
            port_state: PortState::new(false),
            lab_state: LabState::new(false),
            order_deck_remaining: 20,
            tech_deck_remaining: 20,
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
        }
    }

    #[test]
    fn test_power_president_only() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);

        state.players[0].president_location = PresidentLocation::OnBoard(area);
        assert_eq!(calculate_power(&state, player, area), 1);
    }

    #[test]
    fn test_power_with_assistants() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);

        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, 3);
        assert_eq!(calculate_power(&state, player, area), 4);
    }

    #[test]
    fn test_power_capped_at_5() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);

        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, 5);
        state.players[0].shophouses.built.insert(area, 0);
        // President (1) + 5 assistants + 1 shophouse = 7, but capped at 5
        assert_eq!(calculate_power(&state, player, area), 5);
    }

    #[test]
    fn test_power_with_station() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);

        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.station_location = Some(area);
        // President (1) + Station (1) = 2
        assert_eq!(calculate_power(&state, player, area), 2);
    }
}
```

**Step 4: Update lib.rs to include game module**

Add to `crates/yokohama-core/src/lib.rs`:

```rust
pub mod game;

pub use game::*;
```

**Step 5: Run tests**

Run:
```bash
cargo test -p yokohama-core
```

Expected: All tests pass

**Step 6: Commit**

```bash
git add crates/yokohama-core/src/game/
git add crates/yokohama-core/src/lib.rs
git commit -m "feat: add power calculation"
```

---

## Task 2: Implement Production Area Actions

**Files:**
- Create: `crates/yokohama-core/src/game/actions/mod.rs`
- Create: `crates/yokohama-core/src/game/actions/production.rs`

**Step 1: Create actions module**

```rust
//! Area action implementations.

mod production;

pub use production::*;
```

**Step 2: Implement production area actions**

Reference: `02-quick-reference.md` "Production Areas" section

```rust
//! Production area actions (Copper Mine, Silk Mill, Tea Plantation, Fishing Ground).

use crate::types::{GameState, PlayerId};
use crate::game::calculate_power;

/// Result of a production action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionResult {
    pub copper: u8,
    pub silk: u8,
    pub tea: u8,
    pub seafood: u8,
}

impl ProductionResult {
    pub fn empty() -> Self {
        Self { copper: 0, silk: 0, tea: 0, seafood: 0 }
    }
}

/// Calculate Copper Mine rewards based on power.
pub fn copper_mine_reward(power: u8) -> u8 {
    match power {
        0..=2 => 0,
        3 => 1,
        4 => 2,
        _ => 3, // 5+
    }
}

/// Calculate Silk Mill rewards based on power.
pub fn silk_mill_reward(power: u8) -> u8 {
    match power {
        0..=1 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        _ => 4, // 5+
    }
}

/// Calculate Tea Plantation rewards based on power.
pub fn tea_plantation_reward(power: u8) -> u8 {
    power.min(5) // 1:1, capped at 5
}

/// Calculate Fishing Ground rewards based on power.
pub fn fishing_ground_reward(power: u8) -> u8 {
    power.min(5) // 1:1, capped at 5
}

/// Apply production rewards to player state, accounting for Mining Technology.
pub fn apply_production(
    state: &mut GameState,
    player: PlayerId,
    result: &ProductionResult,
) {
    let player_state = &mut state.players[player.0 as usize];

    // Check for Mining Technology bonus
    let has_mining_tech = player_state.technologies.iter().any(|&tech_id| {
        // Would need to look up tech card by ID - simplified check
        // In full implementation, maintain a lookup or store TechEffect in player state
        false // TODO: implement proper tech lookup
    });

    let copper_bonus = if has_mining_tech && result.copper > 0 { 1 } else { 0 };

    player_state.copper += result.copper as u32 + copper_bonus as u32;
    player_state.silk += result.silk as u32;
    player_state.tea += result.tea as u32;
    player_state.seafood += result.seafood as u32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copper_mine_rewards() {
        assert_eq!(copper_mine_reward(1), 0);
        assert_eq!(copper_mine_reward(2), 0);
        assert_eq!(copper_mine_reward(3), 1);
        assert_eq!(copper_mine_reward(4), 2);
        assert_eq!(copper_mine_reward(5), 3);
    }

    #[test]
    fn test_silk_mill_rewards() {
        assert_eq!(silk_mill_reward(1), 0);
        assert_eq!(silk_mill_reward(2), 1);
        assert_eq!(silk_mill_reward(3), 2);
        assert_eq!(silk_mill_reward(4), 3);
        assert_eq!(silk_mill_reward(5), 4);
    }

    #[test]
    fn test_tea_plantation_rewards() {
        for power in 1..=5 {
            assert_eq!(tea_plantation_reward(power), power);
        }
    }

    #[test]
    fn test_fishing_ground_rewards() {
        for power in 1..=5 {
            assert_eq!(fishing_ground_reward(power), power);
        }
    }
}
```

**Step 3: Update game/mod.rs**

```rust
//! Game rules engine for Yokohama.

mod power;
pub mod actions;

pub use power::*;
```

**Step 4: Run tests**

Run:
```bash
cargo test -p yokohama-core
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add crates/yokohama-core/src/game/actions/
git add crates/yokohama-core/src/game/mod.rs
git commit -m "feat: add production area action calculations"
```

---

## Task 3: Implement Commercial Area Actions

**Files:**
- Create: `crates/yokohama-core/src/game/actions/commercial.rs`

**Step 1: Implement Bank and Employment Agency**

```rust
//! Commercial area actions (Bank, Employment Agency, Chinatown).

use crate::types::{ChinatownExchange, GameState, PlayerId};

/// Bank reward: 1 Yen per power.
pub fn bank_reward(power: u8) -> u8 {
    power.min(5)
}

/// Employment Agency: tokens allowed based on power.
pub fn employment_agency_tokens_allowed(power: u8) -> u8 {
    match power {
        0..=1 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        _ => 4, // 5+
    }
}

/// Apply a Chinatown exchange.
pub fn apply_chinatown_exchange(
    state: &mut GameState,
    player: PlayerId,
    exchange: ChinatownExchange,
) -> Result<(), &'static str> {
    let p = &mut state.players[player.0 as usize];

    match exchange {
        ChinatownExchange::CopperToYen => {
            if p.copper < 1 {
                return Err("Not enough copper");
            }
            p.copper -= 1;
            p.money += 2;
        }
        ChinatownExchange::SilkToYen => {
            if p.silk < 1 {
                return Err("Not enough silk");
            }
            p.silk -= 1;
            p.money += 2;
        }
        ChinatownExchange::TeaToYen => {
            if p.tea < 1 {
                return Err("Not enough tea");
            }
            p.tea -= 1;
            p.money += 1;
        }
        ChinatownExchange::SeafoodToYen => {
            if p.seafood < 1 {
                return Err("Not enough seafood");
            }
            p.seafood -= 1;
            p.money += 1;
        }
        ChinatownExchange::YenToCopper => {
            if p.money < 2 {
                return Err("Not enough money");
            }
            p.money -= 2;
            p.copper += 1;
        }
        ChinatownExchange::YenToSilk => {
            if p.money < 2 {
                return Err("Not enough money");
            }
            p.money -= 2;
            p.silk += 1;
        }
        ChinatownExchange::YenToTea => {
            if p.money < 1 {
                return Err("Not enough money");
            }
            p.money -= 1;
            p.tea += 1;
        }
        ChinatownExchange::YenToSeafood => {
            if p.money < 1 {
                return Err("Not enough money");
            }
            p.money -= 1;
            p.seafood += 1;
        }
        ChinatownExchange::YenToImported => {
            if p.money < 4 {
                return Err("Not enough money");
            }
            p.money -= 4;
            p.imported_goods += 1;
        }
    }

    Ok(())
}

/// Validate Chinatown exchanges are within power limit.
pub fn validate_chinatown_exchanges(
    exchanges: &[ChinatownExchange],
    power: u8,
) -> Result<(), &'static str> {
    if exchanges.len() > power as usize {
        return Err("Too many exchanges for power level");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bank_reward() {
        assert_eq!(bank_reward(1), 1);
        assert_eq!(bank_reward(3), 3);
        assert_eq!(bank_reward(5), 5);
        assert_eq!(bank_reward(6), 5); // Capped
    }

    #[test]
    fn test_employment_agency_tokens() {
        assert_eq!(employment_agency_tokens_allowed(1), 0);
        assert_eq!(employment_agency_tokens_allowed(2), 1);
        assert_eq!(employment_agency_tokens_allowed(5), 4);
    }

    #[test]
    fn test_chinatown_exchange_validation() {
        let exchanges = vec![
            ChinatownExchange::CopperToYen,
            ChinatownExchange::SilkToYen,
            ChinatownExchange::TeaToYen,
        ];
        assert!(validate_chinatown_exchanges(&exchanges, 3).is_ok());
        assert!(validate_chinatown_exchanges(&exchanges, 2).is_err());
    }
}
```

**Step 2: Add to actions/mod.rs**

```rust
//! Area action implementations.

mod production;
mod commercial;

pub use production::*;
pub use commercial::*;
```

**Step 3: Run tests**

Run:
```bash
cargo test -p yokohama-core
```

**Step 4: Commit**

```bash
git add crates/yokohama-core/src/game/actions/
git commit -m "feat: add commercial area actions (Bank, Employment, Chinatown)"
```

---

## Task 4: Implement Port and Laboratory Actions

**Files:**
- Create: `crates/yokohama-core/src/game/actions/cards.rs`

**Step 1: Implement Port action**

Reference: `01-rules-reference.md` "Port" section

**Critical rulebook requirements (v3 feedback):**

**Port:**
- **Hand limit**: Cannot exceed **3 Order cards** in hand; must enforce before taking
- **Slot access by power**: Power 2 = Low only; Power 3 = Low+Mid; Power 4-5 = All slots
- **Max cards per action**: **2 cards maximum**
- **Second card cost**: **2 Yen OR 1 Imported Good** (player's choice)
- **Power 5 bonus**: +3 VP immediately when taking card(s) at Power 5
- **Slide/refill**: After taking, cards slide left to fill gaps; new cards refill from deck to rightmost empty slots

**Laboratory:**
- **Industry Value calculation**: Power contributes **1:1 IV (max 5)**; **2 Yen = 1 IV**; **1 Imported Good = 1 IV**; no fractional IV
- **Position surcharges**: Slot 3 = +1 cost; Slot 4 = +2 cost (added to card's printed Industry cost)
- **Fifth-tech penalty**: 5th+ tech costs additional **2 Yen OR 1 Imported Good** (per card, not once per game)
- **Card sliding**: After taking, cards slide left; new card fills rightmost slot from deck

```rust
//! Port and Laboratory actions.

use crate::types::{GameState, PlayerId, PortSlot, PortPayment};

/// Which Port slots are accessible at a given power level.
pub fn port_accessible_slots(power: u8) -> Vec<PortSlot> {
    match power {
        0..=1 => vec![],
        2 => vec![PortSlot::Low],
        3 => vec![PortSlot::Low, PortSlot::Mid],
        _ => vec![PortSlot::Low, PortSlot::Mid, PortSlot::High], // 4-5
    }
}

/// Does power 5 at Port grant bonus VP?
pub fn port_power_5_bonus_vp(power: u8) -> u8 {
    if power >= 5 { 3 } else { 0 }
}

/// Validate Port action.
pub fn validate_port_action(
    state: &GameState,
    player: PlayerId,
    cards_taken: &[PortSlot],
    payment: Option<PortPayment>,
    power: u8,
) -> Result<(), &'static str> {
    let p = state.player(player);
    let accessible = port_accessible_slots(power);

    // Check all slots are accessible
    for slot in cards_taken {
        if !accessible.contains(slot) {
            return Err("Slot not accessible at this power level");
        }
    }

    // Check hand limit
    if p.order_cards_in_hand.len() + cards_taken.len() > 3 {
        return Err("Would exceed Order card hand limit of 3");
    }

    // Check payment for second card
    if cards_taken.len() == 2 {
        match payment {
            None => return Err("Must pay for second card"),
            Some(PortPayment::TwoYen) => {
                if p.money < 2 {
                    return Err("Not enough money for second card");
                }
            }
            Some(PortPayment::OneImportedGood) => {
                if p.imported_goods < 1 {
                    return Err("Not enough imported goods for second card");
                }
            }
        }
    }

    // Can't take more than 2 cards
    if cards_taken.len() > 2 {
        return Err("Cannot take more than 2 cards from Port");
    }

    Ok(())
}

/// Calculate industry value available for Laboratory.
pub fn calculate_industry_value(
    power: u8,
    extra_yen: u8,
    extra_imported: u8,
) -> u8 {
    // Power gives 1:1 industry value, max 5
    let from_power = power.min(5);
    // 2 Yen = 1 Industry Value
    let from_yen = extra_yen / 2;
    // 1 Imported Good = 1 Industry Value
    let from_imported = extra_imported;

    from_power + from_yen + from_imported
}

/// Cost for 5th+ technology card (penalty).
pub fn fifth_tech_penalty_required(tech_count: usize) -> bool {
    tech_count >= 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_accessible_slots() {
        assert!(port_accessible_slots(1).is_empty());
        assert_eq!(port_accessible_slots(2), vec![PortSlot::Low]);
        assert_eq!(port_accessible_slots(3), vec![PortSlot::Low, PortSlot::Mid]);
        assert_eq!(
            port_accessible_slots(4),
            vec![PortSlot::Low, PortSlot::Mid, PortSlot::High]
        );
    }

    #[test]
    fn test_port_power_5_bonus() {
        assert_eq!(port_power_5_bonus_vp(4), 0);
        assert_eq!(port_power_5_bonus_vp(5), 3);
    }

    #[test]
    fn test_industry_value_calculation() {
        // Power only
        assert_eq!(calculate_industry_value(3, 0, 0), 3);

        // Power + Yen (2 yen = 1 IV)
        assert_eq!(calculate_industry_value(3, 2, 0), 4);
        assert_eq!(calculate_industry_value(3, 4, 0), 5);
        assert_eq!(calculate_industry_value(3, 1, 0), 3); // 1 yen not enough

        // Power + Imported Goods
        assert_eq!(calculate_industry_value(3, 0, 2), 5);

        // Combined
        assert_eq!(calculate_industry_value(3, 2, 1), 5);
    }

    #[test]
    fn test_fifth_tech_penalty() {
        assert!(!fifth_tech_penalty_required(0));
        assert!(!fifth_tech_penalty_required(3));
        assert!(fifth_tech_penalty_required(4)); // 5th card
        assert!(fifth_tech_penalty_required(5)); // 6th card
    }
}
```

**Step 2: Add to actions/mod.rs**

```rust
mod cards;
pub use cards::*;
```

**Step 3: Run tests and commit**

```bash
cargo test -p yokohama-core
git add crates/yokohama-core/src/game/actions/
git commit -m "feat: add Port and Laboratory action logic"
```

---

## Task 5: Implement Church and Customs Actions

**Files:**
- Create: `crates/yokohama-core/src/game/actions/tracks.rs`

**Step 1: Implement Church faith calculation**

Reference: `02-quick-reference.md` "CHURCH" and "CUSTOMS" sections

**Critical rulebook requirements (v3 feedback):**
- Church Faith-from-power mapping: 1→1, **2→1** (special case!), 3→3, 4→4, 5→5
- Both Church and Customs require spending an **assistant FROM HAND** to place on track
- At most one of each good type (Copper, Silk, Tea, Seafood, Yen, Imported Good) may be contributed per Church action
- Customs payment is **LIMITED BY POWER** (power 2→max 1 IG, power 3→max 2 IG, etc.)
- Apply Winery (+3 VP) on Church placement, Brickyard (+4 VP) on Customs placement

```rust
//! Church and Customs track actions.

use crate::types::{ChurchTrack, CustomsTrack, GameState, PlayerId};

/// Calculate Faith value from power.
/// CRITICAL: Power 2 gives only 1 Faith (not 2). This is per rulebook p.14.
/// Mapping: 1→1, 2→1, 3→3, 4→4, 5→5
pub fn church_faith_from_power(power: u8) -> u8 {
    match power {
        0 => 0,
        1 => 1,
        2 => 1, // CRITICAL: power 2 gives 1 Faith, not 2 (rulebook p.14)
        3 => 3,
        4 => 4,
        _ => 5,
    }
}

/// Calculate total Faith for Church action.
pub fn calculate_church_faith(
    power: u8,
    copper: bool,
    silk: bool,
    tea: bool,
    seafood: bool,
    yen: bool,
    imported_good: bool,
) -> u8 {
    let mut faith = church_faith_from_power(power);

    // Each good type contributes max 1 Faith
    if copper { faith += 1; }
    if silk { faith += 1; }
    if tea { faith += 1; }
    if seafood { faith += 1; }
    if yen { faith += 1; }
    if imported_good { faith += 1; }

    faith
}

/// Validate Church action.
pub fn validate_church_action(
    state: &GameState,
    player: PlayerId,
    copper: bool,
    silk: bool,
    tea: bool,
    seafood: bool,
    yen: bool,
    imported_good: bool,
) -> Result<(), &'static str> {
    let p = state.player(player);

    if copper && p.copper < 1 {
        return Err("Not enough copper");
    }
    if silk && p.silk < 1 {
        return Err("Not enough silk");
    }
    if tea && p.tea < 1 {
        return Err("Not enough tea");
    }
    if seafood && p.seafood < 1 {
        return Err("Not enough seafood");
    }
    if yen && p.money < 1 {
        return Err("Not enough money");
    }
    if imported_good && p.imported_goods < 1 {
        return Err("Not enough imported goods");
    }

    // Check assistant available
    if p.assistants.in_hand < 1 {
        return Err("No assistant in hand for Church");
    }

    Ok(())
}

/// Maximum imported goods payable at Customs based on power.
pub fn customs_max_imported_goods(power: u8) -> u8 {
    match power {
        0..=1 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        _ => 4,
    }
}

/// Validate Customs action.
pub fn validate_customs_action(
    state: &GameState,
    player: PlayerId,
    imported_goods_to_pay: u8,
    power: u8,
) -> Result<(), &'static str> {
    let p = state.player(player);

    let max_allowed = customs_max_imported_goods(power);
    if imported_goods_to_pay > max_allowed {
        return Err("Exceeds maximum imported goods for power level");
    }

    if imported_goods_to_pay > p.imported_goods as u8 {
        return Err("Not enough imported goods");
    }

    if imported_goods_to_pay < 1 {
        return Err("Must pay at least 1 imported good");
    }

    // Check assistant available
    if p.assistants.in_hand < 1 {
        return Err("No assistant in hand for Customs");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_church_faith_from_power() {
        assert_eq!(church_faith_from_power(1), 1);
        assert_eq!(church_faith_from_power(2), 1); // Special case
        assert_eq!(church_faith_from_power(3), 3);
        assert_eq!(church_faith_from_power(4), 4);
        assert_eq!(church_faith_from_power(5), 5);
    }

    #[test]
    fn test_calculate_church_faith() {
        // Power 3 only
        assert_eq!(calculate_church_faith(3, false, false, false, false, false, false), 3);

        // Power 3 + all goods (max 6 extra)
        assert_eq!(calculate_church_faith(3, true, true, true, true, true, true), 9);

        // Power 5 + 2 goods
        assert_eq!(calculate_church_faith(5, true, true, false, false, false, false), 7);
    }

    #[test]
    fn test_customs_max_imported() {
        assert_eq!(customs_max_imported_goods(1), 0);
        assert_eq!(customs_max_imported_goods(2), 1);
        assert_eq!(customs_max_imported_goods(3), 2);
        assert_eq!(customs_max_imported_goods(4), 3);
        assert_eq!(customs_max_imported_goods(5), 4);
    }
}
```

**Step 2: Add to actions/mod.rs**

```rust
mod tracks;
pub use tracks::*;
```

**Step 3: Commit**

```bash
cargo test -p yokohama-core
git add crates/yokohama-core/src/game/actions/
git commit -m "feat: add Church and Customs action logic"
```

---

## Task 6: Implement Movement and Pathfinding

**Files:**
- Create: `crates/yokohama-core/src/game/movement.rs`

**Step 1: Implement movement validation**

Reference: `01-rules-reference.md` "B2. Movement Step" section

**Critical rulebook requirements (v3 feedback):**
- **Assistant-chain validation**: Must verify assistants exist along ENTIRE path, not just at destination
- **Canal tolls**: 1 Yen per canal crossing
- **President entry fees**: 1 Yen to enter (not pass through) area with opponent President
- **Tech effects that modify movement:**
  - **Stagecoach**: Free passage through opponent Presidents (no fees to pass; fees may still apply to enter destination without Stagecoach)
  - **Postal System**: May stop AND act in an area with opponent President (normally cannot stop)
  - **Station**: Owner may teleport to Station area regardless of opponent Presidents; +1 power for ALL players in that area
- **Tram**: Move President to any area without opponent President (free teleport, separate from Station)

```rust
//! President movement and pathfinding.

use std::collections::{HashSet, VecDeque};
use crate::types::{AreaId, GameState, PlayerId, PresidentLocation, TechEffect};

/// Check if an area is a valid destination (Possible Action Area).
pub fn is_possible_action_area(state: &GameState, player: PlayerId, area: AreaId) -> bool {
    let p = state.player(player);

    // Must have at least 1 assistant
    if p.assistants.in_area(area) < 1 {
        return false;
    }

    // Check for other Presidents (unless has Postal System)
    let has_postal = false; // TODO: check player technologies

    if !has_postal {
        for (i, other_player) in state.players.iter().enumerate() {
            if i != player.0 as usize {
                if other_player.president_location == PresidentLocation::OnBoard(area) {
                    return false;
                }
            }
        }
    }

    true
}

/// Find all areas reachable from current position via assistant chain.
pub fn find_reachable_areas(
    state: &GameState,
    player: PlayerId,
    from: AreaId,
) -> Vec<(AreaId, Vec<AreaId>)> {
    let p = state.player(player);
    let mut reachable = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // Start from adjacent areas
    visited.insert(from);

    for &adj in state.board_layout.adjacent_areas(from) {
        // Can only move through areas with our assistants
        if p.assistants.in_area(adj) > 0 {
            queue.push_back((adj, vec![adj]));
        }
    }

    while let Some((current, path)) = queue.pop_front() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        // Is this a valid destination?
        if is_possible_action_area(state, player, current) && current != from {
            reachable.push((current, path.clone()));
        }

        // Continue exploring if we have assistants here
        if p.assistants.in_area(current) > 0 {
            for &adj in state.board_layout.adjacent_areas(current) {
                if !visited.contains(&adj) && p.assistants.in_area(adj) > 0 {
                    let mut new_path = path.clone();
                    new_path.push(adj);
                    queue.push_back((adj, new_path));
                }
            }
        }
    }

    reachable
}

/// Check if player can teleport to Station area (Station technology).
/// Rulebook p.15: "You may move your President pawn to this area during
/// the Movement Step for free... regardless of other President pawns."
pub fn can_teleport_to_station(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> Option<AreaId> {
    // Must have Station technology
    let p = state.player(player);
    let has_station = p.technologies.iter().any(|&tech_id| {
        tech_registry.get(tech_id).map(|t| t.effect == TechEffect::Station).unwrap_or(false)
    });

    if !has_station {
        return None;
    }

    // Must have a station placed
    let station_area = state.station_location?;

    // Station area must be a valid action area (have our assistant)
    // Note: Station allows ignoring opponent Presidents for stopping
    if p.assistants.in_area(station_area) > 0 {
        Some(station_area)
    } else {
        None
    }
}

/// Generate station teleport move if available.
pub fn generate_station_teleport_move(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> Option<Move> {
    let station_area = can_teleport_to_station(state, player, tech_registry)?;

    // Can't teleport to where we already are
    let p = state.player(player);
    if p.president_location == PresidentLocation::OnBoard(station_area) {
        return None;
    }

    // Teleport is free (empty path, 0 cost)
    Some(Move::MovePresident {
        path: vec![], // No intermediate areas
        destination: station_area,
    })
}

/// Calculate movement cost (opponent Presidents and canal tolls).
pub fn calculate_movement_cost(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
) -> u32 {
    let mut cost: u32 = 0;
    let has_stagecoach = false; // TODO: check player technologies

    // Check each area in path (except destination)
    let intermediate = if path.len() > 1 { &path[..path.len() - 1] } else { &[] };

    for &area in intermediate {
        // Pay for opponent Presidents
        if !has_stagecoach {
            for other_player in &state.players {
                if other_player.id != player {
                    if other_player.president_location == PresidentLocation::OnBoard(area) {
                        cost += 1;
                    }
                }
            }
        }
    }

    // Check for canal crossings
    if path.len() >= 2 {
        for window in path.windows(2) {
            if state.board_layout.is_canal_crossing(window[0], window[1]) {
                cost += 1;
            }
        }
    }

    cost
}

/// Validate a movement action.
pub fn validate_movement(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
    destination: AreaId,
) -> Result<(), &'static str> {
    let p = state.player(player);

    // Check we can afford the movement
    let cost = calculate_movement_cost(state, player, path);
    if p.money < cost {
        return Err("Not enough money to pay movement costs");
    }

    // Check destination is reachable and valid
    if !is_possible_action_area(state, player, destination) {
        return Err("Destination is not a valid action area");
    }

    // Verify path connectivity
    let current_pos = match p.president_location {
        PresidentLocation::OnBoard(area) => area,
        PresidentLocation::InHand => {
            return Err("President must be on board to move (use place instead)");
        }
    };

    // Check path starts adjacent to current position
    if let Some(&first) = path.first() {
        if !state.board_layout.adjacent_areas(current_pos).contains(&first) {
            return Err("Path must start adjacent to current position");
        }
    }

    // Check destination matches end of path
    if path.last() != Some(&destination) && !path.is_empty() {
        return Err("Path must end at destination");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require full state setup - see integration tests
}
```

**Step 2: Add to game/mod.rs**

```rust
mod movement;
pub use movement::*;
```

**Step 3: Commit**

```bash
cargo test -p yokohama-core
git add crates/yokohama-core/src/game/
git commit -m "feat: add movement and pathfinding logic"
```

---

## Task 7: Implement Legal Move Generation

**Files:**
- Create: `crates/yokohama-core/src/game/legal_moves.rs`

**Step 1: Implement legal move generator**

**Critical rulebook requirements (v3 feedback):**
- **Default placement**: Either 3 different areas (1 each) OR 2 in same area
- **Newspaper (tech)**: Option A allows up to **4 different areas** (not 3)
- **Telegram (tech)**: After placing 2 in same area, may place **1 additional assistant in an adjacent area** (bonus placement respecting board adjacency)
- **President entry fees**: 1 Yen to place assistant in area with opponent President
- **Combinatorial concern**: With Newspaper (4 areas) or Telegram (adjacency bonus), move generation can explode. Consider pruning strategies or lazy evaluation for MCTS performance.

**Architectural Notes (critical edge cases):**

**1. Zero-Assistant Soft-Lock Prevention (Page 6, Step 2b):**
If a player has 0 assistants in hand AND their President is in an area with no assistants (and no reachable areas via assistant chain), they are physically stuck.
- **Solution:** `generate_movement_moves` MUST always include `Move::ReturnPresidentToHand` as an "emergency exit"
- This move returns President to hand and recovers all assistants from ALL areas (not just current area)
- Verify this option is available even when no other legal movement exists

**2. Canal Traversal Logic:**
The Canal area type cannot be a valid action area (no assistants can be placed there).
- `is_possible_action_area` correctly returns `false` for Canal (assistants check fails)
- **VERIFY:** `find_reachable_areas` must allow **traversing** the Canal node even though `is_possible_action_area` returns false
- Pathfinding should treat Canal as a passable waypoint (costs 1 Yen) but not a destination
- Test: Player can move from Area A → Canal → Area B if they have assistants in A and B

```rust
//! Legal move generation for each turn phase.

use crate::types::*;
use crate::game::*;

/// Generate all legal moves for the current game state.
pub fn get_legal_moves(state: &GameState) -> Vec<Move> {
    match state.turn_phase {
        TurnPhase::AdditionalActionFirst | TurnPhase::AdditionalActionSecond => {
            generate_additional_action_moves(state)
        }
        TurnPhase::Placement => generate_placement_moves(state),
        TurnPhase::Movement => generate_movement_moves(state),
        TurnPhase::AreaAction => generate_area_action_moves(state),
        TurnPhase::FivePowerBonus => generate_five_power_moves(state),
        TurnPhase::Building => generate_building_moves(state),
        TurnPhase::Recovery => vec![], // Automatic, no moves
        TurnPhase::TurnEnd => vec![],
    }
}

fn generate_additional_action_moves(state: &GameState) -> Vec<Move> {
    let mut moves = Vec::new();
    let player = state.current_player;
    let p = state.player(player);

    // Always can end the phase
    moves.push(Move::EndAdditionalPhase);

    // Foreign Mercenaries (once per turn)
    if !state.additional_action_state.mercenary_used {
        for (&country, &count) in &p.mercenaries.available {
            if count > 0 {
                // Find valid target areas
                for (&area, &assistant_count) in &p.assistants.on_board {
                    if assistant_count > 0 {
                        // Must not have our president
                        let has_our_president =
                            p.president_location == PresidentLocation::OnBoard(area);
                        if !has_our_president {
                            moves.push(Move::UseMercenary {
                                country,
                                target_area: area,
                            });
                        }
                    }
                }
            }
        }
    }

    // Fulfill Orders
    for &order_id in &p.order_cards_in_hand {
        // Would need to check if requirements met
        // Simplified: always offer the option, validation happens on execution
        moves.push(Move::FulfillOrder { order_id });
    }

    // Fulfill Achievements
    for achievement in &state.achievements {
        if !achievement.claimed_by.contains(&player) {
            moves.push(Move::FulfillAchievement {
                achievement: achievement.achievement_type,
            });
        }
    }

    moves
}

fn generate_placement_moves(state: &GameState) -> Vec<Move> {
    let mut moves = Vec::new();
    let player = state.current_player;
    let p = state.player(player);

    if p.assistants.in_hand == 0 {
        // Must place 0 (degenerate case)
        moves.push(Move::PlaceThreeAreas { placements: vec![] });
        return moves;
    }

    // Get all placeable areas (not Canal)
    let placeable_areas: Vec<AreaId> = state
        .board_layout
        .areas
        .iter()
        .filter(|(_, config)| config.area_type.allows_placement())
        .map(|(&id, _)| id)
        .collect();

    // Option A: 1 assistant in each of 1-3 different areas
    // We generate combinations
    for i in 0..=placeable_areas.len().min(3).min(p.assistants.in_hand as usize) {
        if i == 0 {
            moves.push(Move::PlaceThreeAreas { placements: vec![] });
        } else if i == 1 {
            for &area in &placeable_areas {
                moves.push(Move::PlaceThreeAreas {
                    placements: vec![area],
                });
            }
        } else if i == 2 && p.assistants.in_hand >= 2 {
            for (j, &area1) in placeable_areas.iter().enumerate() {
                for &area2 in &placeable_areas[j + 1..] {
                    moves.push(Move::PlaceThreeAreas {
                        placements: vec![area1, area2],
                    });
                }
            }
        } else if i == 3 && p.assistants.in_hand >= 3 {
            for (j, &area1) in placeable_areas.iter().enumerate() {
                for (k, &area2) in placeable_areas[j + 1..].iter().enumerate() {
                    for &area3 in &placeable_areas[j + k + 2..] {
                        moves.push(Move::PlaceThreeAreas {
                            placements: vec![area1, area2, area3],
                        });
                    }
                }
            }
        }
    }

    // Option B: 2 assistants in 1 area (if have at least 2)
    if p.assistants.in_hand >= 2 {
        for &area in &placeable_areas {
            moves.push(Move::PlaceTwoSameArea { area });
        }
    }

    moves
}

fn generate_movement_moves(state: &GameState) -> Vec<Move> {
    let mut moves = Vec::new();
    let player = state.current_player;
    let p = state.player(player);

    match p.president_location {
        PresidentLocation::InHand => {
            // Place president on any Possible Action Area
            for &area in state.board_layout.areas.keys() {
                if is_possible_action_area(state, player, area) {
                    moves.push(Move::PlacePresident { destination: area });
                }
            }
        }
        PresidentLocation::OnBoard(current) => {
            // Move to reachable areas
            for (dest, path) in find_reachable_areas(state, player, current) {
                moves.push(Move::MovePresident {
                    path,
                    destination: dest,
                });
            }

            // Return to hand
            let areas_with_assistants: Vec<AreaId> = p
                .assistants
                .on_board
                .iter()
                .filter(|(_, &count)| count > 0)
                .map(|(&area, _)| area)
                .collect();

            // Generate subset of areas to recover from (simplified: just offer current area)
            moves.push(Move::ReturnPresidentToHand {
                recover_assistants_from: areas_with_assistants,
            });
        }
    }

    moves
}

fn generate_area_action_moves(state: &GameState) -> Vec<Move> {
    // This is complex - depends on area type and player resources
    // Simplified: return single action based on area type
    let player = state.current_player;
    let p = state.player(player);

    // For mercenary sub-turns, use the mercenary target area instead of president location
    let area = if state.is_mercenary_action {
        match state.mercenary_target_area {
            Some(a) => a,
            None => return vec![], // Should never happen if state is consistent
        }
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            PresidentLocation::InHand => return vec![],
        }
    };

    let area_config = match state.board_layout.areas.get(&area) {
        Some(c) => c,
        None => return vec![],
    };

    vec![generate_area_action_for_type(state, area_config.area_type)]
}

fn generate_area_action_for_type(state: &GameState, area_type: AreaType) -> Move {
    // Simplified: return default action for area type
    let action = match area_type {
        AreaType::CopperMine => AreaAction::ProduceCopper,
        AreaType::SilkMill => AreaAction::ProduceSilk,
        AreaType::TeaPlantation => AreaAction::ProduceTea,
        AreaType::FishingGround => AreaAction::ProduceSeafood,
        AreaType::Bank => AreaAction::Bank,
        AreaType::PortA | AreaType::PortB => AreaAction::Port {
            cards_taken: vec![PortSlot::Low],
            second_card_payment: None,
        },
        AreaType::LaboratoryA | AreaType::LaboratoryB => AreaAction::Laboratory {
            card_id: TechCardId(0), // Placeholder
            extra_yen: 0,
            extra_imported: 0,
            fifth_tech_payment: None,
        },
        AreaType::EmploymentAgency => AreaAction::EmploymentAgency {
            assistants: 0,
            shophouses: 0,
            trading_houses: 0,
        },
        AreaType::Chinatown => AreaAction::Chinatown { exchanges: vec![] },
        AreaType::Church => AreaAction::Church {
            copper: false,
            silk: false,
            tea: false,
            seafood: false,
            yen: false,
            imported_good: false,
        },
        AreaType::Customs => AreaAction::Customs { imported_goods_paid: 1 },
        AreaType::Canal => AreaAction::Bank, // Should never happen
    };

    Move::ExecuteAreaAction { action }
}

fn generate_five_power_moves(state: &GameState) -> Vec<Move> {
    let player = state.current_player;
    let p = state.player(player);

    // Check if power was 5 and token is still there
    if state.current_action_power == Some(5) {
        // For mercenary sub-turns, use the mercenary target area
        let area = if state.is_mercenary_action {
            state.mercenary_target_area
        } else {
            match p.president_location {
                PresidentLocation::OnBoard(a) => Some(a),
                PresidentLocation::InHand => None,
            }
        };

        if let Some(area) = area {
            if let Some(Some(_)) = state.board_layout.five_power_tokens.get(&area) {
                return vec![Move::Take5PowerToken, Move::Skip5PowerBonus];
            }
        }
    }

    vec![Move::Skip5PowerBonus]
}

fn generate_building_moves(state: &GameState) -> Vec<Move> {
    let mut moves = vec![Move::SkipBuilding];
    let player = state.current_player;
    let p = state.player(player);

    // Need 4+ power action
    if state.current_action_power.unwrap_or(0) < 4 {
        return moves;
    }

    // For mercenary sub-turns, use the mercenary target area instead of president location
    let area = if state.is_mercenary_action {
        match state.mercenary_target_area {
            Some(a) => a,
            None => return moves,
        }
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            PresidentLocation::InHand => return moves,
        }
    };

    if let Some(site) = state.board_layout.building_sites.get(&area) {
        // Shophouse
        if p.shophouses.in_hand > 0 {
            if let Some(slot) = site.next_shophouse_slot(player) {
                moves.push(Move::BuildShophouse { slot: slot as u8 });
            }
        }

        // Trading House
        if p.trading_houses.in_warehouse > 0 && site.can_build_trading_house() {
            moves.push(Move::BuildTradingHouse);
        }
    }

    moves
}
```

**Step 2: Add to game/mod.rs**

```rust
mod legal_moves;
pub use legal_moves::*;
```

**Step 3: Commit**

```bash
cargo test -p yokohama-core
git add crates/yokohama-core/src/game/
git commit -m "feat: add legal move generation"
```

---

## Task 8: Implement State Transitions (Apply Move)

**Files:**
- Create: `crates/yokohama-core/src/game/apply_move.rs`

**Step 1: Implement apply_move function**

```rust
//! Apply moves to game state.

use crate::types::*;
use crate::game::*;

/// Apply a move to the game state, returning the new state.
pub fn apply_move(state: &GameState, mv: &Move) -> Result<GameState, &'static str> {
    let mut new_state = state.clone();

    match mv {
        Move::EndAdditionalPhase => {
            advance_phase(&mut new_state);
        }

        Move::PlaceThreeAreas { placements } => {
            apply_placement(&mut new_state, placements)?;
            new_state.turn_phase = TurnPhase::Movement;
        }

        Move::PlaceTwoSameArea { area } => {
            apply_placement_same_area(&mut new_state, *area)?;
            new_state.turn_phase = TurnPhase::Movement;
        }

        Move::MovePresident { path, destination } => {
            apply_movement(&mut new_state, path, *destination)?;
            new_state.turn_phase = TurnPhase::AreaAction;
        }

        Move::PlacePresident { destination } => {
            apply_place_president(&mut new_state, *destination)?;
            new_state.turn_phase = TurnPhase::AreaAction;
        }

        Move::ReturnPresidentToHand { recover_assistants_from } => {
            apply_return_to_hand(&mut new_state, recover_assistants_from)?;
            // Skip to Additional Action Second
            new_state.turn_phase = TurnPhase::AdditionalActionSecond;
        }

        Move::ExecuteAreaAction { action } => {
            apply_area_action(&mut new_state, action)?;
            new_state.turn_phase = TurnPhase::FivePowerBonus;
        }

        Move::Take5PowerToken => {
            apply_take_5_power(&mut new_state)?;
            new_state.turn_phase = TurnPhase::Building;
        }

        Move::Skip5PowerBonus => {
            new_state.turn_phase = TurnPhase::Building;
        }

        Move::BuildShophouse { slot } => {
            apply_build_shophouse(&mut new_state, *slot)?;
            new_state.turn_phase = TurnPhase::Recovery;
        }

        Move::BuildTradingHouse => {
            apply_build_trading_house(&mut new_state)?;
            new_state.turn_phase = TurnPhase::Recovery;
        }

        Move::SkipBuilding => {
            new_state.turn_phase = TurnPhase::Recovery;
        }

        Move::UseMercenary { country, target_area } => {
            apply_mercenary(&mut new_state, *country, *target_area)?;
        }

        Move::FulfillOrder { order_id } => {
            apply_fulfill_order(&mut new_state, *order_id)?;
        }

        Move::FulfillAchievement { achievement } => {
            apply_fulfill_achievement(&mut new_state, *achievement)?;
        }
    }

    // Handle Recovery phase automatically
    if new_state.turn_phase == TurnPhase::Recovery {
        apply_recovery(&mut new_state);

        // Check if this was a mercenary sub-turn
        if new_state.is_mercenary_action {
            // Return to the Additional Action phase we came from
            if let Some(return_phase) = new_state.mercenary_return_phase {
                new_state.turn_phase = return_phase;
            } else {
                // Fallback: if return phase not set, go to AdditionalActionSecond
                new_state.turn_phase = TurnPhase::AdditionalActionSecond;
            }
            // Reset mercenary sub-turn state
            new_state.is_mercenary_action = false;
            new_state.mercenary_target_area = None;
            new_state.mercenary_return_phase = None;
            new_state.current_action_power = None;
        } else {
            // Normal turn flow: Recovery -> AdditionalActionSecond
            new_state.turn_phase = TurnPhase::AdditionalActionSecond;
        }
    }

    // Check end game triggers
    check_end_game_triggers(&mut new_state);

    Ok(new_state)
}

fn advance_phase(state: &mut GameState) {
    state.turn_phase = match state.turn_phase {
        TurnPhase::AdditionalActionFirst => TurnPhase::Placement,
        TurnPhase::AdditionalActionSecond => {
            // End of turn - handle end game logic
            let next_player = state.next_player();

            // Check if game should end after this turn
            if state.final_round && next_player == state.config.start_player {
                // Final round completed - game over
                return; // Stay in AdditionalActionSecond, game is over
            }

            // Check if we should start the final round
            if state.end_game_triggered && !state.final_round {
                // Current round completes when we return to start player
                if next_player == state.config.start_player {
                    state.final_round = true;
                }
            }

            // Advance to next player
            state.current_player = next_player;
            state.turn_number += 1;
            state.additional_action_state = AdditionalActionState::new();
            state.current_action_power = None;
            TurnPhase::AdditionalActionFirst
        }
        other => other, // Should not happen
    };
}

/// Check if the game is over (final round completed)
pub fn is_game_over(state: &GameState) -> bool {
    if !state.final_round {
        return false;
    }
    // Game is over when final round is complete and we'd return to start player
    // This is checked at the end of AdditionalActionSecond phase
    state.turn_phase == TurnPhase::AdditionalActionSecond
        && state.next_player() == state.config.start_player
}

fn apply_placement(state: &mut GameState, placements: &[AreaId]) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player_mut(player);

    for &area in placements {
        if p.assistants.in_hand < 1 {
            return Err("Not enough assistants in hand");
        }
        p.assistants.in_hand -= 1;
        *p.assistants.on_board.entry(area).or_insert(0) += 1;

        // TODO: Check and pay President fees
    }

    Ok(())
}

fn apply_placement_same_area(state: &mut GameState, area: AreaId) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player_mut(player);

    if p.assistants.in_hand < 2 {
        return Err("Not enough assistants in hand");
    }

    p.assistants.in_hand -= 2;
    *p.assistants.on_board.entry(area).or_insert(0) += 2;

    // TODO: Check and pay President fees

    Ok(())
}

fn apply_movement(
    state: &mut GameState,
    path: &[AreaId],
    destination: AreaId,
) -> Result<(), &'static str> {
    let player = state.current_player;

    // Pay movement costs
    let cost = calculate_movement_cost(state, player, path);
    let p = state.player_mut(player);
    if p.money < cost {
        return Err("Not enough money for movement");
    }
    p.money -= cost;
    p.president_location = PresidentLocation::OnBoard(destination);

    // Calculate and store power for this action
    let power = calculate_power(state, player, destination);
    state.current_action_power = Some(power);

    Ok(())
}

fn apply_place_president(state: &mut GameState, destination: AreaId) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player_mut(player);
    p.president_location = PresidentLocation::OnBoard(destination);

    let power = calculate_power(state, player, destination);
    state.current_action_power = Some(power);

    Ok(())
}

fn apply_return_to_hand(
    state: &mut GameState,
    areas: &[AreaId],
) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player_mut(player);

    p.president_location = PresidentLocation::InHand;

    for &area in areas {
        if let Some(&count) = p.assistants.on_board.get(&area) {
            p.assistants.in_hand += count;
            p.assistants.on_board.remove(&area);
        }
    }

    Ok(())
}

fn apply_area_action(state: &mut GameState, action: &AreaAction) -> Result<(), &'static str> {
    let player = state.current_player;
    let power = state.current_action_power.unwrap_or(0);

    match action {
        AreaAction::ProduceCopper => {
            let reward = copper_mine_reward(power);
            state.player_mut(player).copper += reward as u32;
        }
        AreaAction::ProduceSilk => {
            let reward = silk_mill_reward(power);
            state.player_mut(player).silk += reward as u32;
        }
        AreaAction::ProduceTea => {
            let reward = tea_plantation_reward(power);
            state.player_mut(player).tea += reward as u32;
        }
        AreaAction::ProduceSeafood => {
            let reward = fishing_ground_reward(power);
            state.player_mut(player).seafood += reward as u32;
        }
        AreaAction::Bank => {
            let reward = bank_reward(power);
            state.player_mut(player).money += reward as u32;
        }

        // Employment Agency - retrieve items from warehouse
        // Rulebook Page 12, Source 706-718
        AreaAction::EmploymentAgency { assistants, shophouses, trading_houses } => {
            apply_employment_agency(state, player, power, *assistants, *shophouses, *trading_houses)?;
        }

        // Other actions would be implemented similarly
        _ => {} // TODO: implement Port, Lab, Church, Customs, Chinatown
    }

    Ok(())
}

/// Employment Agency: retrieve items from warehouse to hand/board.
/// Rulebook Page 12 (Source 706-718).
///
/// Power determines total items allowed:
/// - Power 1: 0 items
/// - Power 2: 1 item
/// - Power 3: 2 items
/// - Power 4: 3 items
/// - Power 5: 4 items
///
/// Costs:
/// - Assistants: FREE
/// - Shophouses: 2 Yen each
/// - Trading Houses: 4, 5, 6, 7 Yen (based on how many already built)
fn apply_employment_agency(
    state: &mut GameState,
    player: PlayerId,
    power: u8,
    assistants: u8,
    shophouses: u8,
    trading_houses: u8,
) -> Result<(), &'static str> {
    // Calculate max items allowed based on power
    let max_items = match power {
        0..=1 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        _ => 4, // Power 5
    };

    let total_items = assistants + shophouses + trading_houses;
    if total_items > max_items {
        return Err("Exceeds maximum items for power level");
    }

    let p = state.player(player);

    // Validate availability in warehouse
    if assistants > p.assistants.in_warehouse {
        return Err("Not enough assistants in warehouse");
    }
    if shophouses > p.shophouses.in_warehouse {
        return Err("Not enough shophouses in warehouse");
    }
    if trading_houses > p.trading_houses.in_warehouse {
        return Err("Not enough trading houses in warehouse");
    }

    // Calculate total cost
    let shophouse_cost = shophouses as u32 * 2; // 2 Yen each

    // Trading house cost: 4, 5, 6, 7 Yen based on how many already built
    let already_built = p.trading_houses.total_built();
    let mut th_cost = 0u32;
    for i in 0..trading_houses {
        let th_index = already_built + i as usize;
        th_cost += match th_index {
            0 => 4,
            1 => 5,
            2 => 6,
            _ => 7, // 3rd and beyond
        };
    }

    let total_cost = shophouse_cost + th_cost; // Assistants are FREE

    if p.money < total_cost {
        return Err("Not enough Yen to pay for items");
    }

    // Apply the action
    let p = state.player_mut(player);
    p.money -= total_cost;

    // Move assistants: warehouse -> hand
    p.assistants.in_warehouse -= assistants;
    p.assistants.in_hand += assistants;

    // Move shophouses: warehouse -> hand
    p.shophouses.in_warehouse -= shophouses;
    p.shophouses.in_hand += shophouses;

    // Trading houses stay in warehouse until built (they go warehouse -> board directly)
    // Actually per rulebook, they just become "available" - but we track in_warehouse vs built
    // For simplicity, trading houses retrieved here are just marked as available
    // (no change needed - they're already in warehouse, just "unlocked" for building)

    Ok(())
}

fn apply_take_5_power(state: &mut GameState) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player(player);

    if let PresidentLocation::OnBoard(area) = p.president_location {
        if let Some(effect_opt) = state.board_layout.five_power_tokens.get_mut(&area) {
            if let Some(effect) = effect_opt.take() {
                let p = state.player_mut(player);
                p.five_power_tokens += 1;

                // Apply effect
                match effect {
                    FivePowerEffect::ImportedGoods(n) => p.imported_goods += n as u32,
                    FivePowerEffect::Copper(n) => p.copper += n as u32,
                    FivePowerEffect::Silk(n) => p.silk += n as u32,
                    FivePowerEffect::Tea(n) => p.tea += n as u32,
                    FivePowerEffect::Seafood(n) => p.seafood += n as u32,
                    FivePowerEffect::Money(n) => p.money += n as u32,
                    FivePowerEffect::VictoryPoints(n) => p.victory_points += n as u32,
                    FivePowerEffect::Movement(_) => {} // TODO: handle movement bonus
                }
            }
        }
    }

    Ok(())
}

fn apply_build_shophouse(state: &mut GameState, slot: u8) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player(player);

    let area = match p.president_location {
        PresidentLocation::OnBoard(a) => a,
        _ => return Err("President not on board"),
    };

    // Get VP reward
    let vp = state
        .board_layout
        .building_sites
        .get(&area)
        .map(|site| site.shophouse_rewards[slot as usize])
        .unwrap_or(0);

    let p = state.player_mut(player);
    if p.shophouses.in_hand < 1 {
        return Err("No shophouse in hand");
    }
    p.shophouses.in_hand -= 1;
    p.shophouses.built.insert(area, slot);
    p.victory_points += vp as u32;

    // Check for Gaslight bonus
    // TODO: implement tech bonus

    // Update building site
    if let Some(site) = state.board_layout.building_sites.get_mut(&area) {
        site.shophouses_built[slot as usize] = Some(player);
    }

    Ok(())
}

fn apply_build_trading_house(state: &mut GameState) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player(player);

    let area = match p.president_location {
        PresidentLocation::OnBoard(a) => a,
        _ => return Err("President not on board"),
    };

    let vp = state
        .board_layout
        .building_sites
        .get(&area)
        .map(|site| site.trading_house_reward)
        .unwrap_or(0);

    let p = state.player_mut(player);
    if p.trading_houses.in_warehouse < 1 {
        return Err("No trading house in warehouse");
    }

    let cost = p.trading_houses.next_cost();
    if p.money < cost as u32 {
        return Err("Not enough money for trading house");
    }

    p.money -= cost as u32;
    p.trading_houses.in_warehouse -= 1;
    p.trading_houses.built.push(area);
    p.victory_points += vp as u32;

    if let Some(site) = state.board_layout.building_sites.get_mut(&area) {
        site.trading_house_owner = Some(player);
    }

    Ok(())
}

fn apply_recovery(state: &mut GameState) {
    let player = state.current_player;
    let p = state.player(player);

    // During mercenary sub-turn, recover from mercenary target area
    // During normal turn, recover from president's location
    let recovery_area = if state.is_mercenary_action {
        state.mercenary_target_area
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => Some(a),
            PresidentLocation::InHand => None,
        }
    };

    if let Some(area) = recovery_area {
        // TODO: Check for Telephone tech - allows leaving 1 assistant
        let p = state.player_mut(player);
        if let Some(&count) = p.assistants.on_board.get(&area) {
            p.assistants.in_hand += count;
            p.assistants.on_board.remove(&area);
        }
    }
}

fn apply_mercenary(
    state: &mut GameState,
    country: Country,
    target_area: AreaId,
) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player(player);

    // Validate: mercenary not already used this turn
    if state.additional_action_state.mercenary_used {
        return Err("Mercenary already used this turn");
    }

    // Validate: player has this mercenary available
    let available = p.mercenaries.available.get(&country).copied().unwrap_or(0);
    if available == 0 {
        return Err("No available mercenary of this country");
    }

    // Validate: target area has player's assistant but NOT president
    let assistant_count = p.assistants.on_board.get(&target_area).copied().unwrap_or(0);
    if assistant_count == 0 {
        return Err("Target area must have your assistant");
    }
    if let PresidentLocation::OnBoard(loc) = p.president_location {
        if loc == target_area {
            return Err("Target area cannot have your president");
        }
    }

    // Calculate power at target area for the mercenary action
    // Power = Assistants + Shophouse + TradingHouse + Station (NO president)
    let mut power: u8 = assistant_count;
    if p.shophouses.built.contains_key(&target_area) {
        power += 1;
    }
    if p.trading_houses.built.contains(&target_area) {
        power += 1;
    }
    if state.station_location == Some(target_area) {
        power += 1;
    }
    power = power.min(5);

    // Consume the mercenary (flip face-down)
    let p_mut = state.player_mut(player);
    *p_mut.mercenaries.available.entry(country).or_insert(0) -= 1;
    *p_mut.mercenaries.used.entry(country).or_insert(0) += 1;

    // Mark mercenary as used for this turn
    state.additional_action_state.mercenary_used = true;

    // Store return phase (where to go after sub-turn completes)
    state.mercenary_return_phase = Some(state.turn_phase);

    // Set up mercenary sub-turn state
    state.is_mercenary_action = true;
    state.mercenary_target_area = Some(target_area);
    state.current_action_power = Some(power);

    // Transition to AreaAction phase (start of nested sub-turn: AreaAction -> Building -> Recovery)
    state.turn_phase = TurnPhase::AreaAction;

    Ok(())
}

fn apply_fulfill_order(state: &mut GameState, order_id: OrderCardId) -> Result<(), &'static str> {
    // TODO: Implement order fulfillment
    Ok(())
}

fn apply_fulfill_achievement(
    state: &mut GameState,
    achievement: AchievementType,
) -> Result<(), &'static str> {
    // TODO: Implement achievement fulfillment
    Ok(())
}

fn check_end_game_triggers(state: &mut GameState) {
    if state.end_game_triggered {
        return;
    }

    let player = state.current_player;

    // Check Trading Houses (4 built)
    for p in &state.players {
        if p.trading_houses.total_built() >= 4 {
            state.end_game_triggered = true;
            state.trigger_reason = Some(EndGameTrigger::TradingHousesComplete(p.id));
            state.trigger_player = Some(p.id);
            return;
        }
    }

    // Check Shophouses (8 built)
    for p in &state.players {
        if p.shophouses.total_built() >= 8 {
            state.end_game_triggered = true;
            state.trigger_reason = Some(EndGameTrigger::ShophousesComplete(p.id));
            state.trigger_player = Some(p.id);
            return;
        }
    }

    // Check Church track
    if state.church_track.total_assistants() >= state.config.track_trigger() {
        state.end_game_triggered = true;
        state.trigger_reason = Some(EndGameTrigger::ChurchTrackFull);
        state.trigger_player = Some(player);
    }

    // Check Customs track
    if state.customs_track.total_assistants() >= state.config.track_trigger() {
        state.end_game_triggered = true;
        state.trigger_reason = Some(EndGameTrigger::CustomsTrackFull);
        state.trigger_player = Some(player);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_game_state() -> GameState {
        // Create minimal test state
        let mut state = GameState::new(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        state
    }

    #[test]
    fn test_mercenary_initiates_sub_turn() {
        let mut state = test_game_state();
        let player = state.current_player;

        // Give player a mercenary and assistant at target area
        let target = AreaId::new(5);
        {
            let p = state.player_mut(player);
            p.mercenaries.available.insert(Country::English, 1);
            p.assistants.on_board.insert(target, 2);
            p.president_location = PresidentLocation::OnBoard(AreaId::new(1)); // Different area
        }

        // Apply mercenary action
        let result = apply_move(
            &state,
            &Move::UseMercenary {
                country: Country::English,
                target_area: target,
            },
        );

        assert!(result.is_ok());
        let new_state = result.unwrap();

        // Verify sub-turn state
        assert!(new_state.is_mercenary_action);
        assert_eq!(new_state.mercenary_target_area, Some(target));
        assert_eq!(new_state.mercenary_return_phase, Some(TurnPhase::AdditionalActionFirst));
        assert_eq!(new_state.turn_phase, TurnPhase::AreaAction);
        assert!(new_state.additional_action_state.mercenary_used);

        // Verify mercenary was consumed
        let p = new_state.player(player);
        assert_eq!(p.mercenaries.available.get(&Country::English).copied().unwrap_or(0), 0);
        assert_eq!(p.mercenaries.used.get(&Country::English).copied().unwrap_or(0), 1);
    }

    #[test]
    fn test_mercenary_sub_turn_returns_to_additional_action() {
        let mut state = test_game_state();

        // Set up as if we're in the middle of a mercenary sub-turn, at Recovery
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(AreaId::new(5));
        state.mercenary_return_phase = Some(TurnPhase::AdditionalActionFirst);
        state.turn_phase = TurnPhase::Building;

        // Skip building to trigger Recovery
        let result = apply_move(&state, &Move::SkipBuilding);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        // After Recovery during mercenary sub-turn, should return to Additional Action
        assert!(!new_state.is_mercenary_action);
        assert_eq!(new_state.mercenary_target_area, None);
        assert_eq!(new_state.mercenary_return_phase, None);
        assert_eq!(new_state.turn_phase, TurnPhase::AdditionalActionFirst);
    }

    #[test]
    fn test_mercenary_recovery_uses_target_area() {
        let mut state = test_game_state();
        let player = state.current_player;
        let target = AreaId::new(5);
        let president_area = AreaId::new(1);

        // Set up mercenary sub-turn state with assistants at both areas
        {
            let p = state.player_mut(player);
            p.president_location = PresidentLocation::OnBoard(president_area);
            p.assistants.on_board.insert(target, 3);
            p.assistants.on_board.insert(president_area, 2);
        }

        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(target);
        state.mercenary_return_phase = Some(TurnPhase::AdditionalActionFirst);
        state.turn_phase = TurnPhase::Recovery;

        // Manually call apply_recovery
        apply_recovery(&mut state);

        // Should have recovered from mercenary target, not president area
        let p = state.player(player);
        assert_eq!(p.assistants.on_board.get(&target).copied().unwrap_or(0), 0);
        assert_eq!(p.assistants.on_board.get(&president_area).copied().unwrap_or(0), 2);
        assert_eq!(p.assistants.in_hand, 8 + 3); // Starting 8 + recovered 3
    }

    #[test]
    fn test_mercenary_power_excludes_president() {
        let mut state = test_game_state();
        let player = state.current_player;
        let target = AreaId::new(5);

        // Set up: 2 assistants + shophouse at target = 3 power (no president)
        {
            let p = state.player_mut(player);
            p.mercenaries.available.insert(Country::English, 1);
            p.assistants.on_board.insert(target, 2);
            p.shophouses.built.insert(target, 1);
            p.president_location = PresidentLocation::OnBoard(AreaId::new(1));
        }

        let result = apply_move(
            &state,
            &Move::UseMercenary {
                country: Country::English,
                target_area: target,
            },
        );

        assert!(result.is_ok());
        let new_state = result.unwrap();

        // Power should be 2 assistants + 1 shophouse = 3
        assert_eq!(new_state.current_action_power, Some(3));
    }

    #[test]
    fn test_end_game_triggers_final_round_at_start_player() {
        // 4-player game: Player 0 is start player
        let mut state = GameState::new(4);
        state.current_player = PlayerId::new(2); // Player 2's turn
        state.turn_phase = TurnPhase::AdditionalActionSecond;

        // Player 2 triggers end game
        state.end_game_triggered = true;
        state.trigger_player = Some(PlayerId::new(2));

        // Advance from Player 2 -> Player 3
        advance_phase(&mut state);
        assert_eq!(state.current_player, PlayerId::new(3));
        assert!(!state.final_round); // Not yet final round

        // Advance from Player 3 -> Player 0 (start player)
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        advance_phase(&mut state);
        assert_eq!(state.current_player, PlayerId::new(0));
        assert!(state.final_round); // Now in final round
    }

    #[test]
    fn test_game_ends_after_final_round() {
        // 3-player game: Player 0 is start player
        let mut state = GameState::new(3);
        state.end_game_triggered = true;
        state.final_round = true;
        state.current_player = PlayerId::new(2); // Last player in final round
        state.turn_phase = TurnPhase::AdditionalActionSecond;

        // Player 2 finishes their final turn
        assert!(is_game_over(&state));

        // Advance should NOT change current player (game is over)
        advance_phase(&mut state);
        assert_eq!(state.current_player, PlayerId::new(2));
        assert_eq!(state.turn_phase, TurnPhase::AdditionalActionSecond);
    }

    #[test]
    fn test_all_players_get_final_turn() {
        // 2-player game: Player 0 is start, Player 0 triggers end game
        let mut state = GameState::new(2);
        state.current_player = PlayerId::new(0);
        state.turn_phase = TurnPhase::AdditionalActionSecond;

        // Player 0 triggers end game
        state.end_game_triggered = true;

        // Player 0's turn ends -> Player 1's turn (still current round)
        advance_phase(&mut state);
        assert_eq!(state.current_player, PlayerId::new(1));
        assert!(!state.final_round);

        // Player 1's turn ends -> back to Player 0, start final round
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        advance_phase(&mut state);
        assert_eq!(state.current_player, PlayerId::new(0));
        assert!(state.final_round);

        // Player 0 takes final turn
        assert!(!is_game_over(&state)); // Not over yet

        // Player 0's final turn ends -> Player 1's final turn
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        advance_phase(&mut state);
        assert_eq!(state.current_player, PlayerId::new(1));
        assert!(!is_game_over(&state)); // Still Player 1's turn

        // Player 1 finishes final turn
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        assert!(is_game_over(&state)); // NOW game is over
    }
}
```

**Step 2: Add to game/mod.rs**

```rust
mod apply_move;
pub use apply_move::*;
```

**Step 3: Commit**

```bash
cargo test -p yokohama-core
git add crates/yokohama-core/src/game/
git commit -m "feat: add apply_move state transitions"
```

---

## Task 9: Implement Final Scoring

**Files:**
- Create: `crates/yokohama-core/src/game/scoring.rs`

**Step 1: Implement end-game scoring**

Reference: `02-quick-reference.md` "Final Scoring" section

```rust
//! Final scoring calculations.

use std::collections::HashMap;
use crate::types::*;

/// Registry for looking up Order cards by ID.
/// Single source of truth for card definitions; used by engine and CV integration.
pub struct OrderRegistry {
    cards: HashMap<OrderCardId, OrderCard>,
    /// Stable string codes for CV integration (e.g., "order_eng_01")
    codes: HashMap<String, OrderCardId>,
}

impl OrderRegistry {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
            codes: HashMap::new(),
        }
    }

    /// Insert a card with its stable code for CV integration.
    pub fn insert(&mut self, card: OrderCard, code: &str) {
        self.codes.insert(code.to_string(), card.id);
        self.cards.insert(card.id, card);
    }

    /// Get card by numeric ID (for engine).
    pub fn get(&self, id: OrderCardId) -> Option<&OrderCard> {
        self.cards.get(&id)
    }

    /// Get card by stable identifier (for CV integration).
    pub fn by_code(&self, code: &str) -> Option<&OrderCard> {
        self.codes.get(code).and_then(|id| self.cards.get(id))
    }
}

/// Registry for looking up Tech cards by ID.
/// Single source of truth for card definitions; used by engine and CV integration.
pub struct TechRegistry {
    cards: HashMap<TechCardId, TechCard>,
    /// Stable string codes for CV integration (e.g., "tech_tram")
    codes: HashMap<String, TechCardId>,
}

impl TechRegistry {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
            codes: HashMap::new(),
        }
    }

    /// Insert a card with its stable code for CV integration.
    pub fn insert(&mut self, card: TechCard, code: &str) {
        self.codes.insert(code.to_string(), card.id);
        self.cards.insert(card.id, card);
    }

    /// Get card by numeric ID (for engine).
    pub fn get(&self, id: TechCardId) -> Option<&TechCard> {
        self.cards.get(&id)
    }

    /// Get card by stable identifier (for CV integration).
    pub fn by_code(&self, code: &str) -> Option<&TechCard> {
        self.codes.get(code).and_then(|id| self.cards.get(id))
    }
}

/// Calculate final scores for all players.
pub fn calculate_final_scores(
    state: &GameState,
    order_registry: &OrderRegistry,
    tech_registry: &TechRegistry,
) -> Vec<(PlayerId, u32)> {
    let mut scores: Vec<(PlayerId, u32)> = state
        .players
        .iter()
        .map(|p| (p.id, p.victory_points))
        .collect();

    // 1. Church Bonus
    let church_bonus = calculate_church_bonus(state);
    for (player, bonus) in church_bonus {
        if let Some((_, score)) = scores.iter_mut().find(|(id, _)| *id == player) {
            *score += bonus;
        }
    }

    // 2. Customs Bonus
    let customs_bonus = calculate_customs_bonus(state);
    for (player, bonus) in customs_bonus {
        if let Some((_, score)) = scores.iter_mut().find(|(id, _)| *id == player) {
            *score += bonus;
        }
    }

    // 3. Technology Bonus
    let tech_bonus = calculate_technology_bonus(state, tech_registry);
    for (player, bonus) in tech_bonus {
        if let Some((_, score)) = scores.iter_mut().find(|(id, _)| *id == player) {
            *score += bonus;
        }
    }

    // 4. Country Bonus
    for player in &state.players {
        let country_vp = calculate_country_bonus(state, player.id, order_registry, tech_registry);
        if let Some((_, score)) = scores.iter_mut().find(|(id, _)| *id == player.id) {
            *score += country_vp;
        }
    }

    // 5. Remainder Scoring
    for player in &state.players {
        let remainder_vp = calculate_remainder_bonus(player);
        if let Some((_, score)) = scores.iter_mut().find(|(id, _)| *id == player.id) {
            *score += remainder_vp;
        }
    }

    scores
}

/// Church track majority bonus: 1st = 6 VP, 2nd = 3 VP.
fn calculate_church_bonus(state: &GameState) -> Vec<(PlayerId, u32)> {
    let mut counts: HashMap<PlayerId, (u8, u8)> = HashMap::new(); // (count, rightmost_position)

    for (index, slot) in state.church_track.slots.iter().enumerate() {
        if let Some(slot_data) = slot {
            let entry = counts.entry(slot_data.player).or_insert((0, 0));
            entry.0 += 1;
            entry.1 = entry.1.max(index as u8);
        }
    }

    let mut rankings: Vec<(PlayerId, u8, u8)> = counts
        .into_iter()
        .map(|(player, (count, position))| (player, count, position))
        .collect();

    // Sort by count (desc), then by rightmost position (desc) for tiebreaker
    rankings.sort_by(|a, b| {
        b.1.cmp(&a.1).then(b.2.cmp(&a.2))
    });

    let mut bonuses = Vec::new();
    if let Some((player, _, _)) = rankings.get(0) {
        bonuses.push((*player, 6));
    }
    if let Some((player, _, _)) = rankings.get(1) {
        bonuses.push((*player, 3));
    }

    bonuses
}

/// Customs track majority bonus: 1st = 8 VP, 2nd = 4 VP.
fn calculate_customs_bonus(state: &GameState) -> Vec<(PlayerId, u32)> {
    let mut counts: HashMap<PlayerId, (u8, u8)> = HashMap::new();

    for (index, slot) in state.customs_track.slots.iter().enumerate() {
        if let Some(slot_data) = slot {
            let entry = counts.entry(slot_data.player).or_insert((0, 0));
            entry.0 += 1;
            entry.1 = entry.1.max(index as u8);
        }
    }

    let mut rankings: Vec<(PlayerId, u8, u8)> = counts
        .into_iter()
        .map(|(player, (count, position))| (player, count, position))
        .collect();

    rankings.sort_by(|a, b| {
        b.1.cmp(&a.1).then(b.2.cmp(&a.2))
    });

    let mut bonuses = Vec::new();
    if let Some((player, _, _)) = rankings.get(0) {
        bonuses.push((*player, 8));
    }
    if let Some((player, _, _)) = rankings.get(1) {
        bonuses.push((*player, 4));
    }

    bonuses
}

/// Technology total industry value bonus: 1st = 10 VP, 2nd = 5 VP.
/// Tiebreaker: closest to start player (clockwise).
fn calculate_technology_bonus(state: &GameState, tech_registry: &TechRegistry) -> Vec<(PlayerId, u32)> {
    // Calculate total industry value for each player
    let mut totals: Vec<(PlayerId, u32, u8)> = state
        .players
        .iter()
        .map(|p| {
            let total_iv: u32 = p.technologies
                .iter()
                .filter_map(|&tech_id| tech_registry.get(tech_id))
                .map(|card| card.industry_cost as u32)
                .sum();
            // Distance from start player (for tiebreaker - lower is better)
            let distance = (p.id.0 + state.config.player_count - state.config.start_player.0)
                % state.config.player_count;
            (p.id, total_iv, distance)
        })
        .collect();

    // Sort by industry value (desc), then by distance from start player (asc)
    totals.sort_by(|a, b| {
        b.1.cmp(&a.1).then(a.2.cmp(&b.2))
    });

    let mut bonuses = Vec::new();
    if let Some((player, iv, _)) = totals.get(0) {
        if *iv > 0 {
            bonuses.push((*player, 10));
        }
    }
    if let Some((player, iv, _)) = totals.get(1) {
        if *iv > 0 {
            bonuses.push((*player, 5));
        }
    }

    bonuses
}

/// Country set bonus: 5 diff = 12, 4 diff = 8, 3 diff = 4, 2 diff = 2.
fn calculate_country_bonus(
    state: &GameState,
    player: PlayerId,
    order_registry: &OrderRegistry,
    tech_registry: &TechRegistry,
) -> u32 {
    let p = state.player(player);

    // Count country icons from completed orders and technologies
    let mut country_counts: HashMap<Country, u8> = HashMap::new();

    for &order_id in &p.completed_orders {
        if let Some(card) = order_registry.get(order_id) {
            *country_counts.entry(card.country).or_insert(0) += 1;
        }
    }

    for &tech_id in &p.technologies {
        if let Some(card) = tech_registry.get(tech_id) {
            *country_counts.entry(card.country).or_insert(0) += 1;
        }
    }

    // Greedy set matching: form largest sets first
    let mut total_vp = 0u32;

    loop {
        // Count how many different countries we have at least 1 icon for
        let available: Vec<Country> = country_counts
            .iter()
            .filter(|(_, &count)| count > 0)
            .map(|(&country, _)| country)
            .collect();

        let set_size = available.len();
        if set_size < 2 {
            break; // Can't form any more sets
        }

        // Award VP for this set
        let set_vp = match set_size {
            5 => 12,
            4 => 8,
            3 => 4,
            2 => 2,
            _ => 0,
        };
        total_vp += set_vp;

        // Consume one icon from each country in the set
        for country in available.iter().take(set_size.min(5)) {
            if let Some(count) = country_counts.get_mut(country) {
                *count -= 1;
            }
        }
    }

    total_vp
}

/// Remainder scoring: mercenaries, imported goods, money, trade goods.
fn calculate_remainder_bonus(player: &PlayerState) -> u32 {
    let mut vp = 0u32;

    // Unused mercenaries: 1 VP each
    vp += player.mercenaries.unused_count() as u32;

    // Imported goods: 1 VP each
    vp += player.imported_goods;

    // Money: 1 VP per 2 Yen
    vp += player.money / 2;

    // Trade goods: 1 VP per 3 goods
    vp += player.total_trade_goods() / 3;

    vp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remainder_bonus() {
        let mut player = PlayerState::new(PlayerId::new(0), PlayerColor::Red, true);
        player.imported_goods = 2;
        player.money = 5;
        player.copper = 3;
        player.silk = 3;
        player.tea = 3;
        // 2 imported + 2 (5/2) + 3 (9/3) = 7 VP
        assert_eq!(calculate_remainder_bonus(&player), 7);
    }
}
```

**Step 2: Add to game/mod.rs**

```rust
mod scoring;
pub use scoring::*;
```

**Step 3: Commit**

```bash
cargo test -p yokohama-core
git add crates/yokohama-core/src/game/
git commit -m "feat: add final scoring calculations"
```

---

## Task 10: Final Cleanup and Integration Test

**Step 1: Run all tests**

Run:
```bash
cargo test -p yokohama-core --all-features
```

Expected: All tests pass

**Step 2: Run clippy**

Run:
```bash
cargo clippy -p yokohama-core -- -D warnings
```

Fix any warnings.

**Step 3: Generate documentation**

Run:
```bash
cargo doc -p yokohama-core --no-deps --open
```

**Step 4: Final commit**

```bash
git add .
git commit -m "feat(A1-implementation): complete game rules engine

Core game logic:
- Power calculation (President, assistants, buildings, Station)
- Production area rewards (Copper Mine, Silk Mill, Tea, Fishing)
- Commercial actions (Bank, Employment Agency, Chinatown)
- Port and Laboratory actions with validation
- Church and Customs track logic
- Movement and pathfinding through assistant chains
- Legal move generation for all phases
- State transitions (apply_move)
- End game trigger detection
- Final scoring (majority bonuses, country sets, remainder)"
```

---

## Task 11: Card Registries and Order Fulfillment

**Files:**
- Create: `crates/yokohama-core/src/game/cards.rs`
- Modify: `crates/yokohama-core/src/game/mod.rs`

**Step 1: Move registries to dedicated module and add Order fulfillment**

```rust
//! Card registries and Order card fulfillment.

use std::collections::HashMap;
use crate::types::*;

/// Registry for looking up Order cards by ID.
#[derive(Debug, Clone, Default)]
pub struct OrderRegistry {
    cards: HashMap<OrderCardId, OrderCard>,
}

impl OrderRegistry {
    pub fn new() -> Self {
        Self { cards: HashMap::new() }
    }

    pub fn insert(&mut self, card: OrderCard) {
        self.cards.insert(card.id, card);
    }

    pub fn get(&self, id: OrderCardId) -> Option<&OrderCard> {
        self.cards.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &OrderCard> {
        self.cards.values()
    }
}

/// Registry for looking up Tech cards by ID.
#[derive(Debug, Clone, Default)]
pub struct TechRegistry {
    cards: HashMap<TechCardId, TechCard>,
}

impl TechRegistry {
    pub fn new() -> Self {
        Self { cards: HashMap::new() }
    }

    pub fn insert(&mut self, card: TechCard) {
        self.cards.insert(card.id, card);
    }

    pub fn get(&self, id: TechCardId) -> Option<&TechCard> {
        self.cards.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &TechCard> {
        self.cards.values()
    }
}

/// Check if player can fulfill an Order card.
pub fn can_fulfill_order(
    state: &GameState,
    player: PlayerId,
    order_id: OrderCardId,
    order_registry: &OrderRegistry,
) -> bool {
    let p = state.player(player);

    // Must have the order in hand
    if !p.order_cards_in_hand.contains(&order_id) {
        return false;
    }

    // Must have required resources
    if let Some(card) = order_registry.get(order_id) {
        let req = &card.requirements;
        p.copper >= req.copper as u32
            && p.silk >= req.silk as u32
            && p.tea >= req.tea as u32
            && p.seafood >= req.seafood as u32
    } else {
        false
    }
}

/// Fulfill an Order card, paying costs and receiving rewards.
pub fn fulfill_order(
    state: &mut GameState,
    player: PlayerId,
    order_id: OrderCardId,
    order_registry: &OrderRegistry,
    tech_registry: &TechRegistry,
) -> Result<(), &'static str> {
    if !can_fulfill_order(state, player, order_id, order_registry) {
        return Err("Cannot fulfill order: missing requirements");
    }

    let card = order_registry.get(order_id).unwrap();
    let req = card.requirements;
    let rewards = card.rewards;
    let country = card.country;

    let p = state.player_mut(player);

    // Pay costs
    p.copper -= req.copper as u32;
    p.silk -= req.silk as u32;
    p.tea -= req.tea as u32;
    p.seafood -= req.seafood as u32;

    // Remove from hand, add to completed
    p.order_cards_in_hand.retain(|&id| id != order_id);
    p.completed_orders.push(order_id);

    // Receive rewards
    p.victory_points += rewards.victory_points as u32;
    p.money += rewards.money as u32;
    p.imported_goods += rewards.imported_goods as u32;
    p.copper += rewards.copper as u32;
    p.silk += rewards.silk as u32;
    p.tea += rewards.tea as u32;
    p.seafood += rewards.seafood as u32;

    // Check for Ball technology bonus (+2 VP)
    let has_ball = p.technologies.iter().any(|&tech_id| {
        tech_registry.get(tech_id).map(|t| t.effect == TechEffect::Ball).unwrap_or(false)
    });
    if has_ball {
        p.victory_points += 2;
    }

    // Check for Language School technology bonus (+1 Yen)
    let has_language_school = p.technologies.iter().any(|&tech_id| {
        tech_registry.get(tech_id).map(|t| t.effect == TechEffect::LanguageSchool).unwrap_or(false)
    });
    if has_language_school {
        p.money += 1;
    }

    // Check for Foreign Mercenary eligibility
    check_mercenary_eligibility(state, player, country, order_registry, tech_registry);

    Ok(())
}

/// Check and award Foreign Mercenary if player has reached threshold.
fn check_mercenary_eligibility(
    state: &mut GameState,
    player: PlayerId,
    _country: Country,
    order_registry: &OrderRegistry,
    tech_registry: &TechRegistry,
) {
    let p = state.player(player);

    // Count country icons
    for country in Country::ALL {
        let mut count = 0u8;

        for &order_id in &p.completed_orders {
            if let Some(card) = order_registry.get(order_id) {
                if card.country == country {
                    count += 1;
                }
            }
        }

        for &tech_id in &p.technologies {
            if let Some(card) = tech_registry.get(tech_id) {
                if card.country == country {
                    count += 1;
                }
            }
        }

        // Award mercenary at 2, 4, 6...
        let threshold = (count / 2) * 2;
        let already_has = p.mercenaries.available.get(&country).copied().unwrap_or(0)
            + p.mercenaries.used.get(&country).copied().unwrap_or(0);

        if threshold > already_has * 2 {
            // Player earned a new mercenary
            let supply = match country {
                Country::English => &mut state.mercenary_supply.english,
                Country::American => &mut state.mercenary_supply.american,
                Country::French => &mut state.mercenary_supply.french,
                Country::German => &mut state.mercenary_supply.german,
                Country::Dutch => &mut state.mercenary_supply.dutch,
            };

            if *supply > 0 {
                *supply -= 1;
                let p = state.player_mut(player);
                *p.mercenaries.available.entry(country).or_insert(0) += 1;
            }
        }
    }
}

/// Remove a card from Port and slide remaining cards left.
/// Rulebook p.12: Cards slide left to fill gaps, new cards fill rightmost slots.
pub fn remove_and_slide_port(port: &mut PortBoard, slot: PortSlot) -> Option<OrderCardId> {
    let taken = match slot {
        PortSlot::Low => port.low.take(),
        PortSlot::Mid => port.mid.take(),
        PortSlot::High => port.high.take(),
    };

    // Slide cards left to fill gaps
    // Low <- Mid <- High <- (new card from deck)
    if port.low.is_none() && port.mid.is_some() {
        port.low = port.mid.take();
    }
    if port.mid.is_none() && port.high.is_some() {
        port.mid = port.high.take();
    }
    // Note: port.high is now None, will be filled by refill_port

    taken
}

/// Refill Port board from deck, placing new cards in rightmost empty slots.
pub fn refill_port(
    port: &mut PortBoard,
    deck: &mut Vec<OrderCardId>,
) {
    // Fill slots from right to left (high first, then mid, then low)
    if port.high.is_none() && !deck.is_empty() {
        port.high = deck.pop();
    }
    if port.mid.is_none() && !deck.is_empty() {
        port.mid = deck.pop();
    }
    if port.low.is_none() && !deck.is_empty() {
        port.low = deck.pop();
    }
}

/// Remove a card from Lab and slide remaining cards left.
/// Rulebook p.13: Cards slide left, position affects cost (+0, +0, +1, +2).
pub fn remove_and_slide_lab(lab: &mut LabBoard, slot_index: usize) -> Option<TechCardId> {
    if slot_index >= 4 {
        return None;
    }

    let taken = lab.slots[slot_index].take();

    // Slide cards left to fill the gap
    for i in slot_index..3 {
        if lab.slots[i].is_none() && lab.slots[i + 1].is_some() {
            lab.slots[i] = lab.slots[i + 1].take();
        }
    }
    // Note: rightmost slot is now None, will be filled by refill_lab

    taken
}

/// Refill Lab board from deck, placing new card in rightmost slot.
pub fn refill_lab(
    lab: &mut LabBoard,
    deck: &mut Vec<TechCardId>,
) {
    // Find rightmost empty slot and fill from deck
    for i in (0..4).rev() {
        if lab.slots[i].is_none() && !deck.is_empty() {
            lab.slots[i] = deck.pop();
            break; // Only fill one slot per refill
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_slide() {
        let mut port = PortBoard {
            low: Some(OrderCardId(1)),
            mid: Some(OrderCardId(2)),
            high: Some(OrderCardId(3)),
        };

        // Take mid card
        let taken = remove_and_slide_port(&mut port, PortSlot::Mid);
        assert_eq!(taken, Some(OrderCardId(2)));

        // High should slide to mid
        assert_eq!(port.low, Some(OrderCardId(1)));
        assert_eq!(port.mid, Some(OrderCardId(3)));
        assert_eq!(port.high, None);
    }

    #[test]
    fn test_lab_slide() {
        let mut lab = LabBoard {
            slots: [
                Some(TechCardId(1)),
                Some(TechCardId(2)),
                Some(TechCardId(3)),
                Some(TechCardId(4)),
            ],
        };

        // Take slot 1 (second from left)
        let taken = remove_and_slide_lab(&mut lab, 1);
        assert_eq!(taken, Some(TechCardId(2)));

        // Cards should slide left
        assert_eq!(lab.slots[0], Some(TechCardId(1)));
        assert_eq!(lab.slots[1], Some(TechCardId(3)));
        assert_eq!(lab.slots[2], Some(TechCardId(4)));
        assert_eq!(lab.slots[3], None);
    }
}
```

**Step 2: Update mod.rs**

```rust
mod cards;
pub use cards::*;
```

**Step 3: Commit**

```bash
cargo test -p yokohama-core
git add crates/yokohama-core/src/game/
git commit -m "feat: add card registries, Order fulfillment, and card sliding"
```

---

## Task 12: Achievement Validation and Fulfillment

**Files:**
- Create: `crates/yokohama-core/src/game/achievements.rs`

**Step 1: Implement Achievement validation and fulfillment**

```rust
//! Achievement card validation and fulfillment.

use crate::types::*;

/// Check if player can claim an Achievement.
pub fn can_claim_achievement(
    state: &GameState,
    player: PlayerId,
    achievement: AchievementType,
) -> bool {
    let p = state.player(player);

    // Check if already claimed by this player
    for card in &state.achievements {
        if card.achievement_type == achievement && card.claimed_by.contains(&player) {
            return false;
        }
    }

    // Check if player has assistant in hand (required to place)
    if p.assistants.in_hand < 1 {
        return false;
    }

    // Check specific achievement requirements
    match achievement {
        // Category A: Trade good holdings (no payment, just demonstrate ownership)
        AchievementType::Copper5 => p.copper >= 5,
        AchievementType::Silk6 => p.silk >= 6,
        AchievementType::Tea7 => p.tea >= 7,
        AchievementType::Seafood7 => p.seafood >= 7,

        // Category B: Collection goals
        AchievementType::Money10 => p.money >= 10,
        AchievementType::Tokens5 => {
            p.five_power_tokens + p.mercenaries.total() >= 5
        }
        AchievementType::Orders5 => p.completed_orders.len() >= 5,
        AchievementType::Technologies4 => p.technologies.len() >= 4,

        // Category C: Building presence
        AchievementType::Production4 => {
            count_production_buildings(state, player) >= 4
        }
        AchievementType::Commercial4 => {
            count_commercial_buildings(state, player) >= 4
        }
        AchievementType::Prod3Comm2 => {
            count_production_buildings(state, player) >= 3
                && count_commercial_buildings(state, player) >= 2
        }
        AchievementType::Prod2Comm3 => {
            count_production_buildings(state, player) >= 2
                && count_commercial_buildings(state, player) >= 3
        }
    }
}

/// Count unique production areas where player has a building.
fn count_production_buildings(state: &GameState, player: PlayerId) -> usize {
    let p = state.player(player);
    let mut count = 0;

    for (&area_id, _) in &p.shophouses.built {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_production() {
                count += 1;
            }
        }
    }

    for &area_id in &p.trading_houses.built {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_production() {
                // Don't double-count if also have shophouse
                if !p.shophouses.built.contains_key(&area_id) {
                    count += 1;
                }
            }
        }
    }

    count
}

/// Count unique commercial areas where player has a building.
fn count_commercial_buildings(state: &GameState, player: PlayerId) -> usize {
    let p = state.player(player);
    let mut count = 0;

    for (&area_id, _) in &p.shophouses.built {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_commercial() {
                count += 1;
            }
        }
    }

    for &area_id in &p.trading_houses.built {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_commercial() {
                if !p.shophouses.built.contains_key(&area_id) {
                    count += 1;
                }
            }
        }
    }

    count
}

/// Claim an Achievement.
pub fn claim_achievement(
    state: &mut GameState,
    player: PlayerId,
    achievement: AchievementType,
) -> Result<u32, &'static str> {
    if !can_claim_achievement(state, player, achievement) {
        return Err("Cannot claim achievement: requirements not met");
    }

    // Find the achievement card and add player to claimed list
    let mut vp_awarded = 0u32;
    for card in &mut state.achievements {
        if card.achievement_type == achievement {
            let is_first = card.claimed_by.is_empty();
            card.claimed_by.push(player);
            vp_awarded = if is_first {
                achievement.first_place_vp() as u32
            } else {
                achievement.subsequent_vp() as u32
            };
            break;
        }
    }

    // Lock an assistant on the achievement
    let p = state.player_mut(player);
    p.assistants.in_hand -= 1;
    p.assistants.on_achievements += 1;
    p.victory_points += vp_awarded;

    Ok(vp_awarded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_achievement_first_vs_subsequent() {
        assert_eq!(AchievementType::Copper5.first_place_vp(), 8);
        assert_eq!(AchievementType::Copper5.subsequent_vp(), 6);
    }
}
```

**Step 2: Update mod.rs**

```rust
mod achievements;
pub use achievements::*;
```

**Step 3: Commit**

```bash
cargo test -p yokohama-core
git add crates/yokohama-core/src/game/
git commit -m "feat: add Achievement validation and fulfillment"
```

---

## Task 13: Technology Effects

**Files:**
- Create: `crates/yokohama-core/src/game/tech_effects.rs`

**Step 1: Implement technology effect checks**

```rust
//! Technology card effect helpers.

use crate::types::*;
use crate::game::TechRegistry;

/// Check if player has a specific technology effect.
pub fn has_tech_effect(
    state: &GameState,
    player: PlayerId,
    effect: TechEffect,
    tech_registry: &TechRegistry,
) -> bool {
    let p = state.player(player);
    p.technologies.iter().any(|&tech_id| {
        tech_registry.get(tech_id).map(|t| t.effect == effect).unwrap_or(false)
    })
}

/// Apply turn-start effects (System of Stock).
pub fn apply_turn_start_effects(
    state: &mut GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) {
    // System of Stock: if Yen <= 1, refill to 2
    if has_tech_effect(state, player, TechEffect::SystemOfStock, tech_registry) {
        let p = state.player_mut(player);
        if p.money <= 1 {
            p.money = 2;
        }
    }
}

/// Calculate building VP bonus from Gaslight/Electrical Light.
pub fn building_vp_bonus(
    state: &GameState,
    player: PlayerId,
    is_shophouse: bool,
    tech_registry: &TechRegistry,
) -> u32 {
    if is_shophouse {
        if has_tech_effect(state, player, TechEffect::Gaslight, tech_registry) {
            2
        } else {
            0
        }
    } else {
        // Trading House
        if has_tech_effect(state, player, TechEffect::ElectricalLight, tech_registry) {
            3
        } else {
            0
        }
    }
}

/// Calculate copper bonus from Mining Technology.
pub fn copper_bonus(
    state: &GameState,
    player: PlayerId,
    base_copper: u8,
    tech_registry: &TechRegistry,
) -> u8 {
    if base_copper > 0 && has_tech_effect(state, player, TechEffect::MiningTechnology, tech_registry) {
        1
    } else {
        0
    }
}

/// Calculate placement limit (3 or 4 with Newspaper).
pub fn placement_limit(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> u8 {
    if has_tech_effect(state, player, TechEffect::Newspaper, tech_registry) {
        4
    } else {
        3
    }
}

/// Check if player can stop in area with opponent President (Postal System).
pub fn can_stop_with_opponent(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> bool {
    has_tech_effect(state, player, TechEffect::PostalSystem, tech_registry)
}

/// Check if player can pass through opponents without paying (Stagecoach).
pub fn free_passage(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> bool {
    has_tech_effect(state, player, TechEffect::Stagecoach, tech_registry)
}

/// Check if player can leave 1 assistant during recovery (Telephone).
pub fn can_leave_assistant(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> bool {
    has_tech_effect(state, player, TechEffect::Telephone, tech_registry)
}

/// Technology acquisition bonuses.
pub fn tech_acquisition_bonuses(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> (u32, u32) {
    let vp_bonus = if has_tech_effect(state, player, TechEffect::PatentSystem, tech_registry) {
        2
    } else {
        0
    };

    let yen_bonus = if has_tech_effect(state, player, TechEffect::LetterPressPrinting, tech_registry) {
        1
    } else {
        0
    };

    (vp_bonus, yen_bonus)
}

/// Church placement bonus (Winery).
pub fn church_vp_bonus(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> u32 {
    if has_tech_effect(state, player, TechEffect::Winery, tech_registry) {
        3
    } else {
        0
    }
}

/// Customs placement bonus (Brickyard).
pub fn customs_vp_bonus(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> u32 {
    if has_tech_effect(state, player, TechEffect::Brickyard, tech_registry) {
        4
    } else {
        0
    }
}
```

**Step 2: Update mod.rs**

```rust
mod tech_effects;
pub use tech_effects::*;
```

**Step 3: Commit**

```bash
cargo test -p yokohama-core
git add crates/yokohama-core/src/game/
git commit -m "feat: add technology effect helpers"
```

---

## Task 14: End Game Trigger Detection

**Files:**
- Create: `crates/yokohama-core/src/game/end_game.rs`

**Step 1: Implement comprehensive end game detection**

```rust
//! End game trigger detection and final round handling.

use crate::types::*;

/// Check all end game triggers and update state if triggered.
pub fn check_end_game_triggers(state: &mut GameState) {
    if state.end_game_triggered {
        return; // Already triggered
    }

    // 1. Trading Houses: any player built all 4
    for p in &state.players {
        if p.trading_houses.total_built() >= 4 {
            trigger_end_game(state, EndGameTrigger::TradingHousesComplete(p.id), p.id);
            return;
        }
    }

    // 2. Shophouses: any player built all 8
    for p in &state.players {
        if p.shophouses.total_built() >= 8 {
            trigger_end_game(state, EndGameTrigger::ShophousesComplete(p.id), p.id);
            return;
        }
    }

    // 3. Order deck empty AND a Port slot is empty
    if state.order_deck_remaining == 0 {
        let port_has_empty_slot = check_port_empty_slot(state);
        if port_has_empty_slot {
            trigger_end_game(state, EndGameTrigger::OrderDeckEmpty, state.current_player);
            return;
        }
    }

    // 4. Church track: reached threshold
    let church_threshold = state.config.track_trigger();
    if state.church_track.total_assistants() >= church_threshold {
        trigger_end_game(state, EndGameTrigger::ChurchTrackFull, state.current_player);
        return;
    }

    // 5. Customs track: reached threshold
    let customs_threshold = state.config.track_trigger();
    if state.customs_track.total_assistants() >= customs_threshold {
        trigger_end_game(state, EndGameTrigger::CustomsTrackFull, state.current_player);
        return;
    }
}

fn trigger_end_game(state: &mut GameState, reason: EndGameTrigger, trigger_player: PlayerId) {
    state.end_game_triggered = true;
    state.trigger_reason = Some(reason);
    state.trigger_player = Some(trigger_player);
}

fn check_port_empty_slot(state: &GameState) -> bool {
    // Check Port A
    if state.port_state.port_a.low.is_none()
        || state.port_state.port_a.mid.is_none()
        || state.port_state.port_a.high.is_none()
    {
        return true;
    }

    // Check Port B if exists
    if let Some(ref port_b) = state.port_state.port_b {
        if port_b.low.is_none() || port_b.mid.is_none() || port_b.high.is_none() {
            return true;
        }
    }

    false
}

/// Determine if the game is completely over (final round finished).
pub fn is_game_over(state: &GameState) -> bool {
    if !state.end_game_triggered {
        return false;
    }

    if !state.final_round {
        // Need to complete current round first, then play final round
        return false;
    }

    // Final round complete when we've looped back after all players took a turn
    // The player before start player should be the last to play
    let player_before_start = PlayerId::new(
        (state.config.start_player.0 + state.config.player_count - 1) % state.config.player_count,
    );

    // Game is over if current player is at TurnEnd and is the player before start player
    state.current_player == player_before_start && state.turn_phase == TurnPhase::TurnEnd
}

/// Advance to final round after completing current round.
pub fn check_round_completion(state: &mut GameState) {
    if !state.end_game_triggered || state.final_round {
        return;
    }

    // Current round ends when we reach the player before start player at TurnEnd
    let player_before_start = PlayerId::new(
        (state.config.start_player.0 + state.config.player_count - 1) % state.config.player_count,
    );

    if state.current_player == player_before_start && state.turn_phase == TurnPhase::TurnEnd {
        state.final_round = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_trigger_by_player_count() {
        let config_2p = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        assert_eq!(config_2p.track_trigger(), 4);

        let config_3p = GameConfig::new(
            3,
            vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            PlayerId::new(0),
        );
        assert_eq!(config_3p.track_trigger(), 5);
    }
}
```

**Step 2: Update mod.rs**

```rust
mod end_game;
pub use end_game::*;
```

**Step 3: Update apply_move to use end game checks**

In `apply_move.rs`, replace the inline `check_end_game_triggers` with:

```rust
use crate::game::end_game::{check_end_game_triggers, check_round_completion};

// At end of apply_move:
check_end_game_triggers(&mut new_state);
check_round_completion(&mut new_state);
```

**Step 4: Commit**

```bash
cargo test -p yokohama-core
git add crates/yokohama-core/src/game/
git commit -m "feat: add comprehensive end game detection"
```

---

## Task 15: Integration Test Scenarios

**Files:**
- Create: `crates/yokohama-core/tests/integration.rs`

**Step 1: Write integration tests covering key scenarios**

```rust
//! Integration tests for complete game scenarios.

use yokohama_core::*;
use std::collections::HashMap;

fn setup_2p_game() -> (GameState, OrderRegistry, TechRegistry) {
    // Create minimal 2-player game state for testing
    let config = GameConfig::new(
        2,
        vec![PlayerColor::Red, PlayerColor::Blue],
        PlayerId::new(0),
    );

    let mut areas = HashMap::new();
    areas.insert(AreaId::new(0), AreaConfig {
        area_type: AreaType::Bank,
        grid_position: (0, 0),
    });
    areas.insert(AreaId::new(1), AreaConfig {
        area_type: AreaType::CopperMine,
        grid_position: (0, 1),
    });

    let mut adjacency = HashMap::new();
    adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
    adjacency.insert(AreaId::new(1), vec![AreaId::new(0)]);

    let board_layout = BoardLayout {
        areas,
        adjacency,
        canal_crossings: vec![],
        building_sites: HashMap::new(),
        five_power_tokens: HashMap::new(),
    };

    let players = vec![
        PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
        PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
    ];

    let state = GameState {
        config,
        board_layout,
        players,
        current_player: PlayerId::new(0),
        turn_phase: TurnPhase::AdditionalActionFirst,
        turn_number: 1,
        additional_action_state: AdditionalActionState::new(),
        current_action_power: None,
        port_state: PortState::new(false),
        lab_state: LabState::new(false),
        order_deck_remaining: 20,
        tech_deck_remaining: 20,
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
    };

    (state, OrderRegistry::new(), TechRegistry::new())
}

#[test]
fn test_basic_turn_flow() {
    let (mut state, _, _) = setup_2p_game();

    // Should start in Additional Action First
    assert_eq!(state.turn_phase, TurnPhase::AdditionalActionFirst);

    // Get legal moves
    let moves = get_legal_moves(&state);
    assert!(!moves.is_empty());

    // Should include EndAdditionalPhase
    assert!(moves.iter().any(|m| matches!(m, Move::EndAdditionalPhase)));
}

#[test]
fn test_power_calculation_comprehensive() {
    let (mut state, _, _) = setup_2p_game();
    let player = PlayerId::new(0);
    let area = AreaId::new(0);

    // No power initially
    assert_eq!(calculate_power(&state, player, area), 0);

    // Add president
    state.players[0].president_location = PresidentLocation::OnBoard(area);
    assert_eq!(calculate_power(&state, player, area), 1);

    // Add assistants
    state.players[0].assistants.on_board.insert(area, 2);
    assert_eq!(calculate_power(&state, player, area), 3);

    // Add shophouse
    state.players[0].shophouses.built.insert(area, 0);
    assert_eq!(calculate_power(&state, player, area), 4);

    // Add trading house
    state.players[0].trading_houses.built.push(area);
    assert_eq!(calculate_power(&state, player, area), 5); // Capped at 5
}

#[test]
fn test_end_game_trigger_trading_houses() {
    let (mut state, _, _) = setup_2p_game();

    // Build 4 trading houses for player 0
    state.players[0].trading_houses.built = vec![
        AreaId::new(0),
        AreaId::new(1),
        AreaId::new(2),
        AreaId::new(3),
    ];
    state.players[0].trading_houses.in_warehouse = 0;

    check_end_game_triggers(&mut state);

    assert!(state.end_game_triggered);
    assert!(matches!(
        state.trigger_reason,
        Some(EndGameTrigger::TradingHousesComplete(p)) if p == PlayerId::new(0)
    ));
}

#[test]
fn test_remainder_scoring() {
    let mut player = PlayerState::new(PlayerId::new(0), PlayerColor::Red, true);

    // 3 imported goods = 3 VP
    player.imported_goods = 3;
    // 6 yen = 3 VP
    player.money = 6;
    // 9 trade goods = 3 VP
    player.copper = 3;
    player.silk = 3;
    player.tea = 3;

    assert_eq!(calculate_remainder_bonus(&player), 9);
}
```

**Step 2: Run integration tests**

Run:
```bash
cargo test -p yokohama-core --test integration
```

**Step 3: Commit**

```bash
git add crates/yokohama-core/tests/
git commit -m "test: add integration test scenarios"
```

---

## Task 16: Complete Missing Features and Edge Cases

**Goal:** Address all remaining TODOs and edge cases identified in code review.

### 16.1 Movement: Tech-Aware Traversal

**Files:** `crates/yokohama-core/src/game/movement.rs`

**Requirements:**
- [ ] Stagecoach: Pass through opponent Presidents without paying
- [ ] Postal System: Allow stopping in opponent President areas
- [ ] Station teleport: Free movement to Station area (owner only, when taking that action)
- [ ] Path validation: Verify assistants exist along ENTIRE path, not just destination
- [ ] Movement costs: Include opponent President fees (1 Yen each)

**Tests to add:**
```rust
#[test]
fn test_stagecoach_free_passage() {
    // Setup: opponent president in path
    // Verify: player with Stagecoach doesn't pay
}

#[test]
fn test_postal_system_allows_stopping() {
    // Setup: opponent president in destination
    // Verify: player with Postal System can stop there
}

#[test]
fn test_station_teleport() {
    // Setup: player owns Station, station at area X
    // Verify: can move directly to X regardless of path
}

#[test]
fn test_path_requires_assistants_throughout() {
    // Setup: assistants at start and end, but gap in middle
    // Verify: move is illegal
}
```

### 16.2 Placement: Tech-Aware Limits

**Files:** `crates/yokohama-core/src/game/legal_moves.rs`

**Requirements:**
- [ ] Newspaper: Option A allows 4 areas (not 3)
- [ ] Telegram: After placing 2 in same area, may place 1 in adjacent
- [ ] President fees: Charge 1 Yen to enter area with opponent President

**Tests to add:**
```rust
#[test]
fn test_newspaper_four_areas() {
    // Verify: player with Newspaper can use Option A with 4 different areas
}

#[test]
fn test_telegram_adjacent_bonus() {
    // Verify: after placing 2 in area X, can place 1 in adjacent area
}
```

### 16.3 Port Action: Complete Implementation

**Files:** `crates/yokohama-core/src/game/actions/cards.rs`

**Requirements:**
- [ ] Hand limit: Cannot take cards if would exceed 3 in hand
- [ ] Second card cost: 2 Yen OR 1 Imported Good
- [ ] Power 5 bonus: +3 VP when taking cards at Power 5
- [ ] Slot validation: Can only take from occupied slots
- [ ] Position-based access: Power 2 = low only, Power 3 = low+mid, Power 4+ = all

**Tests to add:**
```rust
#[test]
fn test_port_hand_limit() {
    // Setup: player has 2 orders in hand
    // Verify: can take 1 card, cannot take 2
}

#[test]
fn test_port_power_5_vp() {
    // Setup: player has power 5 at Port
    // Verify: gains +3 VP when taking order card(s)
}

#[test]
fn test_port_second_card_payment() {
    // Verify: taking 2 cards requires payment
}
```

### 16.4 Laboratory Action: Industry Value Calculation

**Files:** `crates/yokohama-core/src/game/actions/cards.rs`

**Requirements:**
- [ ] Base industry: Power (max 5) converts 1:1 to Industry Value
- [ ] Bonus industry: 2 Yen = 1 IV, 1 Imported Good = 1 IV (no fractional IV)
- [ ] Position surcharges: Slots 3-4 add +1/+2 to card's printed Industry cost
- [ ] Fifth tech penalty: 5th+ tech costs extra 2 Yen OR 1 Imported Good (per card)
- [ ] Card sliding: After taking, cards slide left, refill from right

**Tests to add:**
```rust
#[test]
fn test_lab_industry_from_power() {
    // Verify: base industry = min(power, 5)
}

#[test]
fn test_lab_fifth_tech_penalty() {
    // Setup: player has 4 techs
    // Verify: 5th tech requires extra payment
}

#[test]
fn test_lab_position_surcharge() {
    // Verify: rightmost slots cost extra
}
```

### 16.5 Building: Tech Bonuses

**Files:** `crates/yokohama-core/src/game/apply_move.rs`

**Requirements:**
- [ ] Gaslight: +2 VP when building Shophouse
- [ ] Electrical Light: +3 VP when building Trading House
- [ ] Trading House costs: 4, 5, 6, 7 Yen for 1st, 2nd, 3rd, 4th

**Tests to add:**
```rust
#[test]
fn test_gaslight_shophouse_bonus() {
    // Setup: player has Gaslight tech
    // Verify: building shophouse grants +2 VP
}

#[test]
fn test_electrical_light_trading_house_bonus() {
    // Setup: player has Electrical Light tech
    // Verify: building trading house grants +3 VP
}
```

### 16.6 Recovery: Telephone Effect

**Files:** `crates/yokohama-core/src/game/apply_move.rs`

**Requirements:**
- [ ] Telephone: May leave 1 assistant in area after recovery

**Tests to add:**
```rust
#[test]
fn test_telephone_leaves_one() {
    // Setup: player has Telephone, 3 assistants in area
    // Verify: after recovery, 1 remains (not 0)
}
```

### 16.7 Scoring: Country Set Distinctness Constraint

**Files:** `crates/yokohama-core/src/game/scoring.rs`

**Requirements (v3 feedback - critical fix):**
- [ ] **Sets MUST consist of DISTINCT countries** - no duplicate countries within a set
- [ ] Each country icon may be used **at most once** across ALL sets
- [ ] Scoring: 2 different = 2 VP, 3 different = 4 VP, 4 different = 8 VP, 5 different = 12 VP
- [ ] **Greedy matching**: Form largest sets first (5-set worth 12 VP is better than two 3-sets worth 8 VP total)

**Current bug:** Greedy algorithm may incorrectly form sets using multiple icons from same country.

**Correct algorithm:**
```rust
// Greedy largest-to-smallest packing
loop {
    // Count distinct countries with available icons
    let available_countries: Vec<Country> = country_counts
        .iter()
        .filter(|(_, &count)| count > 0)
        .map(|(&country, _)| country)
        .collect();

    let set_size = available_countries.len();
    if set_size < 2 { break; }

    // Award VP based on DISTINCT country count (not total icons)
    let vp = match set_size {
        5 => 12,
        4 => 8,
        3 => 4,
        2 => 2,
        _ => 0,
    };
    total_vp += vp;

    // Consume ONE icon from each DISTINCT country
    for country in &available_countries {
        *country_counts.get_mut(country).unwrap() -= 1;
    }
}
```

**Tests to add:**
```rust
#[test]
fn test_country_set_distinct_countries_only() {
    // Setup: player has 3 English, 1 French, 1 German (5 total icons, 3 distinct countries)
    // Verify: scores for one 3-set (3 distinct) = 4 VP, not 5-set
}

#[test]
fn test_country_set_multiple_sets() {
    // Setup: player has 2 of each country (10 total icons, 5 distinct)
    // Verify: scores two 5-sets = 24 VP (not one 10-icon mess)
}

#[test]
fn test_country_set_greedy_largest_first() {
    // Setup: 5 distinct countries with 1 icon each
    // Verify: forms one 5-set (12 VP), not multiple smaller sets
}
```

### 16.8 Additional Tech Effects

**Files:** `crates/yokohama-core/src/game/tech_effects.rs`

**Requirements:**
- [ ] Mining Technology: +1 copper when receiving copper
- [ ] Spinning Mill: Once per turn, discard 1 silk → gain 2 Yen
- [ ] Ball: +2 VP when completing order
- [ ] Language School: +1 Yen when completing order
- [ ] Patent System: +2 VP when acquiring technology
- [ ] Letterpress Printing: +1 Yen when acquiring technology
- [ ] Winery: +3 VP when placing assistant on Church
- [ ] Brickyard: +4 VP when placing assistant on Customs
- [ ] System of Stock: At turn start, if Yen ≤ 1, refill to 2
- [ ] University: Once per turn, pay 1 Yen → take assistant from warehouse

**Tests to add:**
```rust
#[test]
fn test_mining_technology_bonus() {
    // Verify: receiving copper gives +1 extra
}

#[test]
fn test_ball_order_bonus() {
    // Verify: completing order gives +2 VP
}

#[test]
fn test_winery_church_bonus() {
    // Verify: placing on Church gives +3 VP
}
```

### 16.9 End Game Trigger: Order Deck Empty

**Files:** `crates/yokohama-core/src/game/end_game.rs`

**Requirements:**
- [ ] Trigger when order deck is empty AND a Port slot is empty (post-replenish condition)
- [ ] Check AFTER card removals and refills (not before)

**Tests to add:**
```rust
#[test]
fn test_order_deck_empty_trigger() {
    // Setup: deck has 1 card, all port slots full
    // Action: take last card
    // Verify: triggers end game (deck empty + slot empty after refill attempt)
}
```

### 16.10 Setup Configuration (2p/3p Variants)

**Files:**
- `crates/yokohama-core/src/types/state.rs` (enhance `GameConfig`)
- Create: `crates/yokohama-core/src/game/setup.rs`

**Requirements (v3 feedback):**
Connect `GameConfig` to player-count-specific setup:

| Setting | 2 Players | 3 Players | 4 Players |
|---------|-----------|-----------|-----------|
| Areas removed | Lab B, Port B, Chinatown, Canal, 1 of each production | Lab B, 1 of each production | None |
| Order cards removed | 16 | 5 | 0 |
| Track trigger | 4 assistants | 5 assistants | 6 assistants |
| Starting yen | 3 (start player) / 4 (others) | Same | Same |
| Port B present | No | Yes | Yes |
| Lab B present | No | No | Yes |

- [ ] Add `removed_areas: Vec<AreaId>` to GameConfig or compute from player_count
- [ ] Add `has_port_b: bool` and `has_lab_b: bool` fields to GameConfig
- [ ] Implement `GameConfig::for_player_count(count: u8)` constructor
- [ ] Add validation: BoardLayoutBuilder should reject areas that should be removed for player count
- [ ] Add setup validators that ensure configuration matches player count

**Tests to add:**
```rust
#[test]
fn test_2p_setup_removes_correct_areas() {
    let config = GameConfig::for_player_count(2);
    assert!(!config.has_port_b);
    assert!(!config.has_lab_b);
    assert_eq!(config.track_trigger(), 4);
}

#[test]
fn test_4p_setup_has_all_areas() {
    let config = GameConfig::for_player_count(4);
    assert!(config.has_port_b);
    assert!(config.has_lab_b);
    assert_eq!(config.track_trigger(), 6);
}
```

### 16.11 Mercenary: Opponent President Rules (RESOLVED)

**Files:** `crates/yokohama-core/src/game/apply_move.rs`, `crates/yokohama-core/src/game/legal_moves.rs`

**RULEBOOK CLARIFICATION (Page 8, Source 392-394):**
> "It is possible to choose an Area that contains another player's President pawn. If you choose an Area that contains another player's President pawn, **you do not need to pay them any money.**"

The rules are now explicit:

| Rule | Answer |
|------|--------|
| Opponent President blocks Mercenary? | **NO** - explicitly allowed |
| Pay 1 Yen to opponent when using Mercenary in their area? | **NO** - explicitly exempt |
| Own President blocks Mercenary? | **YES** - cannot use Mercenary where your own President is |

**Requirements:**
- [ ] **Allow** target areas with opponent Presidents (no blocking check)
- [ ] **Skip** the 1 Yen payment when using Mercenary (unlike normal placement/movement)
- [ ] **Enforce** that target area does NOT contain player's OWN President (Source 392)
- [ ] Ensure mercenary action returns to correct additional-action phase (already implemented)
- [ ] Add test for mercenary once-per-turn across BOTH additional-action halves

**Implementation update in `apply_mercenary`:**
```rust
// Validate: target area has player's assistant but NOT OWN president
// NOTE: Opponent Presidents do NOT block mercenaries (rulebook p.8)
if let PresidentLocation::OnBoard(loc) = p.president_location {
    if loc == target_area {
        return Err("Target area cannot have YOUR OWN president");
    }
}
// NOTE: No opponent President check needed - mercenaries bypass this
// NOTE: No 1 Yen payment to opponents - mercenaries are exempt
```

**Tests to add:**
```rust
#[test]
fn test_mercenary_allowed_with_opponent_president() {
    // Setup: opponent president in target area, player has assistant there
    // Verify: mercenary action IS valid (not blocked)
}

#[test]
fn test_mercenary_no_payment_to_opponent() {
    // Setup: opponent president in target area
    // Action: use mercenary
    // Verify: player's Yen unchanged (no 1 Yen fee)
}

#[test]
fn test_mercenary_blocked_by_own_president() {
    // Setup: player's OWN president in target area
    // Verify: mercenary action is INVALID
}

#[test]
fn test_mercenary_once_per_turn_both_phases() {
    // Use mercenary in AdditionalActionFirst
    // Verify: cannot use again in AdditionalActionSecond
}
```

### 16.12 Five-Power Movement Token Effect (CORRECTED)

**Files:** `crates/yokohama-core/src/game/apply_move.rs`

**RULEBOOK CORRECTION (Page 10, Source 532):**
> "Movement icon: **Move an Assistant pawn** from your hand or from an Area to another Area."

The Movement token does **NOT** move the President. It is a tactical **Assistant repositioning** tool.

**Requirements:**
- [ ] `FivePowerEffect::Movement(n)`: Move up to `n` assistants (typically 2-3) between areas or from hand to board
- [ ] Movement ignores adjacency - can teleport assistants anywhere
- [ ] Movement ignores President fees - no cost to place in opponent's President area
- [ ] Can move from Hand → Board, or Board → Board (different area)
- [ ] Cannot move assistants FROM an area with your President (they'd be recovered anyway)
- [ ] Ensure taken-once semantics (token removed from board, count incremented)

**Implementation:**
```rust
/// Apply 5-Power Movement token effect.
/// Moves assistants (NOT President) - teleport repositioning.
fn apply_movement_token_effect(
    state: &mut GameState,
    player: PlayerId,
    moves: Vec<AssistantMove>, // Vec of (from: Option<AreaId>, to: AreaId)
    max_moves: u8,
) -> Result<(), &'static str> {
    if moves.len() > max_moves as usize {
        return Err("Exceeds movement token limit");
    }

    let p = state.player_mut(player);
    for mv in moves {
        match mv.from {
            None => {
                // Hand -> Board
                if p.assistants.in_hand < 1 {
                    return Err("No assistant in hand");
                }
                p.assistants.in_hand -= 1;
                *p.assistants.on_board.entry(mv.to).or_insert(0) += 1;
            }
            Some(from_area) => {
                // Board -> Board (different area)
                let count = p.assistants.on_board.get(&from_area).copied().unwrap_or(0);
                if count < 1 {
                    return Err("No assistant in source area");
                }
                *p.assistants.on_board.get_mut(&from_area).unwrap() -= 1;
                *p.assistants.on_board.entry(mv.to).or_insert(0) += 1;
            }
        }
    }
    Ok(())
}
```

**Tests to add:**
```rust
#[test]
fn test_movement_token_moves_assistants_not_president() {
    // Claim 5-Power Movement(2) token
    // Verify: can move 2 assistants from hand/board to any area
    // Verify: President location unchanged
}

#[test]
fn test_movement_token_ignores_adjacency() {
    // Setup: assistant in area A, want to move to non-adjacent area Z
    // Verify: movement succeeds (teleport allowed)
}

#[test]
fn test_movement_token_no_president_fee() {
    // Setup: opponent President in target area
    // Verify: no Yen charged when placing assistant via movement token
}

#[test]
fn test_five_power_token_claimed_only_once() {
    // Setup: player at 5 power, token present
    // Action: claim token
    // Verify: token removed from board, count incremented
    // Action: another player achieves 5 power in same area
    // Verify: no token to claim
}
```

### 16.13 Building Power Thresholds

**Files:** `crates/yokohama-core/src/game/legal_moves.rs`, `crates/yokohama-core/src/game/apply_move.rs`

**Requirements (v3 feedback):**
Enforce power thresholds for building:

- [ ] **Shophouse**: Requires Power **4+** at action area
- [ ] **Trading House**: Requires Power **5** at action area (only at max power)
- [ ] Already tracking `current_action_power` in state - use for validation

**Tests to add:**
```rust
#[test]
fn test_shophouse_requires_power_4() {
    // Setup: player has power 3 at area
    // Verify: BuildShophouse not in legal moves
}

#[test]
fn test_trading_house_requires_power_5() {
    // Setup: player has power 4 at area
    // Verify: BuildTradingHouse not in legal moves
}
```

### 16.14 Trading House Passive Income (CRITICAL)

**Files:** `crates/yokohama-core/src/game/apply_move.rs`

**AUDIT FINDING (Section 3.5.1):**
> "If there are any Trading Houses owned by other players in the Area... the owner receives 1 Yen from the bank."

This is the **primary economic engine** for long-term strategy - incentivizes building on high-traffic areas (Bank, Port, Lab).

**Requirements:**
- [ ] When a player executes an area action, check for Trading Houses owned by OTHER players in that area
- [ ] Each other player with a Trading House in that area receives **1 Yen from bank**
- [ ] This happens regardless of whether the acting player has a Trading House there
- [ ] Applies to ALL area actions (production, commercial, Church, Customs, etc.)

**Implementation:**
```rust
/// Apply passive income to Trading House owners when area is used.
/// Returns list of (PlayerId, amount) for UI notification.
fn apply_trading_house_income(
    state: &mut GameState,
    active_player: PlayerId,
    area: AreaId,
) -> Vec<(PlayerId, u32)> {
    let mut payouts = Vec::new();

    // Find all Trading Houses in this area owned by OTHER players
    for player in &mut state.players {
        if player.id != active_player {
            if player.trading_houses.built.contains(&area) {
                player.money += 1;
                payouts.push((player.id, 1));
            }
        }
    }

    payouts
}
```

**Integration point in `apply_area_action`:**
```rust
fn apply_area_action(state: &mut GameState, action: &AreaAction) -> Result<(), &'static str> {
    let player = state.current_player;
    let area = get_action_area(state)?;

    // FIRST: Pay passive income to other Trading House owners
    let _payouts = apply_trading_house_income(state, player, area);

    // THEN: Execute the area action
    match action {
        // ... existing action handling
    }
}
```

**AI Implications:**
- Without this, MCTS will **systematically undervalue** Trading House construction
- AI sees high cost (4-7 Yen) without recurring revenue
- This creates a "positive interaction loop" - building on high-traffic areas is an investment

**Tests to add:**
```rust
#[test]
fn test_trading_house_passive_income() {
    // Setup: Player B has Trading House in Bank area
    // Action: Player A uses Bank action
    // Verify: Player B gains 1 Yen, Player A gains their Bank reward
}

#[test]
fn test_trading_house_income_multiple_owners() {
    // Setup: Players B and C each have Trading House in Port area
    // Action: Player A uses Port action
    // Verify: Both B and C gain 1 Yen each
}

#[test]
fn test_trading_house_no_self_income() {
    // Setup: Player A has Trading House in Bank area
    // Action: Player A uses Bank action
    // Verify: Player A does NOT gain passive income from own Trading House
}
```

### 16.15 Movement: Start Area Return Prohibition

**Files:** `crates/yokohama-core/src/game/movement.rs`, `crates/yokohama-core/src/game/legal_moves.rs`

**AUDIT FINDING (Section 3.4.1):**
> "The President pawn may not return to the Area that it started in."

Without this constraint, a player could "pass" by moving to adjacent tile and back.

**Requirements:**
- [ ] Track `president_start_area: Option<AreaId>` in turn state (set at start of Movement phase)
- [ ] Path validator must reject paths where `destination == president_start_area`
- [ ] Clear `president_start_area` at end of Movement phase

**Implementation:**
```rust
/// Check if movement destination is valid (not start area).
pub fn validate_movement_destination(
    state: &GameState,
    destination: AreaId,
) -> Result<(), &'static str> {
    if let Some(start) = state.president_start_area {
        if destination == start {
            return Err("Cannot return to starting area in same turn");
        }
    }
    Ok(())
}
```

**Tests to add:**
```rust
#[test]
fn test_cannot_return_to_start_area() {
    // Setup: President starts in Bank area
    // Action: Try to move Bank -> Port -> Bank
    // Verify: Move is rejected
}

#[test]
fn test_can_return_to_start_on_different_turn() {
    // Setup: President ended previous turn in Bank
    // Action: New turn, move to Port, then next turn move back to Bank
    // Verify: Move is allowed (different turn)
}
```

### 16.16 Placement Cost Validation in Legal Moves

**Files:** `crates/yokohama-core/src/game/legal_moves.rs`

**AUDIT FINDING (Section 3.3.2):**
> "Placement is not always free. If there is a President pawn in the area... you have to pay 1 Yen for each Assistant pawn you place."

The legal_moves generator must check affordability.

**Requirements:**
- [ ] For each placement combination, calculate total cost (1 Yen per area with opponent President)
- [ ] Only include placement moves the player can **afford**
- [ ] This affects both `PlaceThreeAreas` and `PlaceTwoSameArea` variants

**Implementation:**
```rust
/// Calculate placement cost for a set of areas.
fn calculate_placement_cost(
    state: &GameState,
    player: PlayerId,
    placements: &[AreaId],
) -> u32 {
    let mut cost = 0;
    for &area in placements {
        // Check if any opponent has President in this area
        for other in &state.players {
            if other.id != player {
                if other.president_location == PresidentLocation::OnBoard(area) {
                    cost += 1;
                    break; // Only pay once per area, not per opponent
                }
            }
        }
    }
    cost
}

// In generate_placement_moves:
fn generate_placement_moves(state: &GameState) -> Vec<Move> {
    let player = state.current_player;
    let p = state.player(player);
    let mut moves = Vec::new();

    // ... generate candidate placements ...

    // Filter by affordability
    for placement in candidate_placements {
        let cost = calculate_placement_cost(state, player, &placement);
        if p.money >= cost {
            moves.push(Move::PlaceThreeAreas { placements: placement });
        }
    }

    moves
}
```

**Tests to add:**
```rust
#[test]
fn test_placement_cost_opponent_president() {
    // Setup: Opponent President in Bank, player has 0 Yen
    // Verify: Cannot place in Bank (filtered from legal moves)
}

#[test]
fn test_placement_cost_multiple_areas() {
    // Setup: Opponent Presidents in Bank and Port, player has 1 Yen
    // Verify: Can place in Bank OR Port (1 Yen), but not both (2 Yen)
}
```

### Step: Commit All Fixes

```bash
cargo test -p yokohama-core
git add crates/yokohama-core/src/game/
git commit -m "feat: complete remaining features and edge cases"
```

---

## Summary

This plan implements the complete game rules for Yokohama:

| Module | Purpose |
|--------|---------|
| `game/power.rs` | Power calculation |
| `game/actions/production.rs` | Production area rewards |
| `game/actions/commercial.rs` | Bank, Employment, Chinatown |
| `game/actions/cards.rs` | Port and Laboratory logic |
| `game/actions/tracks.rs` | Church and Customs |
| `game/movement.rs` | Pathfinding and movement |
| `game/legal_moves.rs` | Legal move generation |
| `game/apply_move.rs` | State transitions |
| `game/scoring.rs` | End game scoring (with registries) |
| `game/cards.rs` | Card registries & Order fulfillment |
| `game/achievements.rs` | Achievement validation |
| `game/tech_effects.rs` | Technology effect helpers |
| `game/end_game.rs` | End game trigger detection |
| `game/setup.rs` | Setup configuration by player count |
| `tests/integration.rs` | Integration test scenarios |

**Fully covered (including v3 feedback + rulebook corrections):**
- All 5 end game triggers with proper final round handling (post-replenish Order deck condition)
- Order card fulfillment with tech bonuses (Ball +2 VP, Language School +1 Yen)
- Achievement validation for all 12 types
- Technology effects (20 cards worth of effects) including movement/placement modifiers
- Country set scoring with **distinct-country constraint** and greedy largest-first matching
- Technology majority bonus with industry value sum and **closest-to-start-player tiebreaker**
- Church/Customs majority bonus with **rightmost-assistant tiebreaker**
- Church Faith calculation including **power=2 → 1 Faith special case**
- Assistant-from-hand requirement for Church/Customs placement
- Winery (+3 VP) and Brickyard (+4 VP) bonuses on track placement
- Port hand limit (3), second card payment, Power 5 +3 VP bonus
- Laboratory Industry Value calculation (power cap, 2Y=1IV, 1IG=1IV, position surcharges, fifth-tech penalty)
- Movement tech hooks (Stagecoach free passage, Postal System stopping, Station teleport)
- Placement tech hooks (Newspaper 4-area, Telegram adjacent bonus)
- Building power thresholds (Shophouse 4+, Trading House 5)
- Setup configuration by player count (area removals, Port B/Lab B presence)
- Mercenary once-per-turn across both additional-action phases

**Rulebook corrections (critical fixes):**
- **5-Power Movement token**: Moves **ASSISTANTS** (not President) - tactical repositioning, ignores adjacency/fees
- **Mercenary rules**: Opponent Presidents do NOT block; NO 1 Yen payment; only OWN President blocks
- **Employment Agency**: Full implementation with power-based limits and specific costs (Assistants free, Shophouses 2¥, TH 4/5/6/7¥)

**Architectural edge cases:**
- Zero-assistant soft-lock prevention: `ReturnPresidentToHand` always available as emergency exit
- Canal traversal: pathfinding allows crossing (1¥) without stopping

**Task 16 Subtasks (v3 additions + audit findings):**
- 16.1 Movement: Tech-aware traversal (Stagecoach, Postal, Station)
- 16.2 Placement: Tech-aware limits (Newspaper, Telegram)
- 16.3 Port: Complete validation (hand limit, payments, slots)
- 16.4 Laboratory: Industry Value calculation
- 16.5 Building: Tech bonuses (Gaslight, Electrical Light)
- 16.6 Recovery: Telephone effect
- 16.7 Scoring: Country set distinctness constraint
- 16.8 Additional tech effects (20 technologies)
- 16.9 End game trigger: Order deck empty (post-replenish)
- 16.10 Setup configuration (2p/3p/4p variants)
- 16.11 Mercenary: Opponent president rules (RESOLVED - not blocked, no payment)
- 16.12 Five-Power Movement token (CORRECTED - moves Assistants, not President)
- 16.13 Building power thresholds (Shophouse 4+, Trading House 5)
- **16.14 Trading House Passive Income (CRITICAL - audit finding)**
- **16.15 Movement: Start Area Return Prohibition (audit finding)**
- **16.16 Placement Cost Validation (audit finding)**

**Audit-Derived Additions:**
- Trading House passive income: Other players' TH owners get 1 Yen when you act in their area
- Start area prohibition: President cannot return to starting area in same turn
- Placement cost filtering: legal_moves must check affordability (1 Yen per opponent President area)

**Next modules:** A2 (MCTS), A3 (Heuristic Evaluator), A4 (PyO3 Bindings)
