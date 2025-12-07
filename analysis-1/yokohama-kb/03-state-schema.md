# Yokohama Game State Schema

> Data structures for game engine implementation and VLM state extraction.

---

## Core State Structure

```rust
/// Complete game state - sufficient to resume game at any point
pub struct GameState {
    // Configuration (immutable after setup)
    pub config: GameConfig,
    pub board_layout: BoardLayout,
    
    // Dynamic state
    pub players: Vec<PlayerState>,
    pub current_player: PlayerId,
    pub turn_phase: TurnPhase,
    pub turn_number: u32,
    
    // Shared resources
    pub church_track: ChurchTrack,
    pub customs_track: CustomsTrack,
    pub available_orders: PortState,
    pub available_techs: LabState,
    pub order_deck_remaining: u8,
    pub tech_deck_remaining: u8,
    
    // Tokens
    pub mercenary_supply: MercenarySupply,
    pub station_location: Option<AreaId>,
    
    // Game progression
    pub end_game_triggered: bool,
    pub trigger_reason: Option<EndGameTrigger>,
    pub final_round: bool,
}

pub struct GameConfig {
    pub player_count: u8,  // 2, 3, or 4
    pub player_colors: Vec<PlayerColor>,
    pub start_player: PlayerId,
}

pub struct BoardLayout {
    /// Area tiles and their positions
    pub areas: HashMap<AreaId, AreaConfig>,
    
    /// Which areas are adjacent (bidirectional)
    pub adjacency: HashMap<AreaId, Vec<AreaId>>,
    
    /// Canal connections (areas that require 1 yen to cross between)
    pub canal_crossings: Vec<(AreaId, AreaId)>,
    
    /// Building site VP values per area
    pub building_sites: HashMap<AreaId, BuildingSite>,
    
    /// 5-Power token assignments (if not yet claimed)
    pub five_power_tokens: HashMap<AreaId, Option<FivePowerEffect>>,
}
```

---

## Player State

```rust
pub struct PlayerState {
    pub id: PlayerId,
    pub color: PlayerColor,
    
    // Victory tracking
    pub victory_points: u32,
    
    // Pawns and tokens
    pub president_location: PresidentLocation,
    pub assistants: AssistantState,
    pub shophouses: ShophouseState,
    pub trading_houses: TradingHouseState,
    
    // Resources
    pub money: u32,
    pub copper: u32,
    pub silk: u32,
    pub tea: u32,
    pub seafood: u32,
    pub imported_goods: u32,
    
    // Cards
    pub order_cards_in_hand: Vec<OrderCardId>,
    pub completed_orders: Vec<OrderCardId>,
    pub technologies: Vec<TechCardId>,
    
    // Tokens collected
    pub five_power_tokens: Vec<FivePowerToken>,  // face-down, just count
    pub mercenaries: MercenaryState,
    
    // Track positions (for tie-breaking)
    pub church_assistants: Vec<u8>,   // positions on track
    pub customs_assistants: Vec<u8>,  // positions on track
}

pub enum PresidentLocation {
    InHand,
    OnBoard(AreaId),
}

pub struct AssistantState {
    pub in_hand: u8,
    pub in_warehouse: u8,
    pub on_board: HashMap<AreaId, u8>,
    pub on_achievements: u8,
    pub on_church: u8,
    pub on_customs: u8,
}

pub struct ShophouseState {
    pub in_hand: u8,
    pub in_warehouse: u8,
    pub built: HashMap<AreaId, ShophouseSlot>,  // which slot (1-4)
}

pub struct TradingHouseState {
    pub in_warehouse: u8,
    pub built: Vec<AreaId>,  // order matters for cost tracking
}

pub struct MercenaryState {
    pub available: HashMap<Country, u8>,  // face-up, usable
    pub used: HashMap<Country, u8>,       // face-down
}
```

---

## Board Elements

```rust
pub enum AreaType {
    // Production
    CopperMine,
    SilkMill,
    TeaPlantation,
    FishingGround,
    
    // Commercial
    Bank,
    PortA,
    PortB,
    LaboratoryA,
    LaboratoryB,
    EmploymentAgency,
    Chinatown,
    Church,
    Customs,
    
    // Special
    Canal,
}

pub struct AreaConfig {
    pub area_type: AreaType,
    pub grid_position: (u8, u8),  // for layout reference
}

pub struct BuildingSite {
    pub shophouse_rewards: [u32; 4],  // VP for slots 1-4
    pub trading_house_reward: u32,
    pub shophouses_built: HashMap<PlayerId, u8>,  // slot occupied
    pub trading_house_owner: Option<PlayerId>,
}

pub enum FivePowerEffect {
    ImportedGoods(u8),
    Copper(u8),
    Silk(u8),
    Tea(u8),
    Seafood(u8),
    Money(u32),
    VictoryPoints(u32),
    Movement(u8),
    // Add others as discovered from physical game
}
```

---

## Tracks

```rust
pub struct ChurchTrack {
    /// Slots 4-11 (Faith values)
    /// Each slot: Option<(PlayerId, arrival_order)>
    pub slots: [Option<(PlayerId, u8)>; 8],
    pub next_arrival_order: u8,
}

pub struct CustomsTrack {
    /// Slots 1-7+ (Imported Goods paid)
    /// Each slot: Option<(PlayerId, arrival_order)>
    pub slots: [Option<(PlayerId, u8)>; 8],
    pub next_arrival_order: u8,
}
```

---

## Cards

```rust
pub struct PortState {
    pub port_a: PortBoard,
    pub port_b: Option<PortBoard>,  // None in 2P
}

pub struct PortBoard {
    pub low: Option<OrderCardId>,
    pub mid: Option<OrderCardId>,
    pub high: Option<OrderCardId>,
}

pub struct LabState {
    pub lab_a: LabBoard,
    pub lab_b: Option<LabBoard>,  // None in 2P/3P
}

pub struct LabBoard {
    /// 4 slots, left to right
    /// Position affects cost: slots 3-4 add +1 or +2 industry value
    pub slots: [Option<TechCardId>; 4],
}

pub struct OrderCard {
    pub id: OrderCardId,
    pub country: Country,
    pub requirements: TradeGoodRequirement,
    pub rewards: OrderRewards,
}

pub struct TradeGoodRequirement {
    pub copper: u8,
    pub silk: u8,
    pub tea: u8,
    pub seafood: u8,
}

pub struct OrderRewards {
    pub victory_points: u32,
    pub money: u32,
    pub imported_goods: u8,
    pub copper: u8,
    pub silk: u8,
    pub tea: u8,
    pub seafood: u8,
    pub movement: u8,
    pub any_trade_good: u8,
    pub warehouse: u8,  // items from warehouse
}

pub struct TechCard {
    pub id: TechCardId,
    pub name: String,
    pub country: Country,
    pub industry_cost: u8,
    pub effect: TechEffect,
}

pub enum TechEffect {
    // See Technology Cards Reference for full list
    Tram,
    Stagecoach,
    PostalSystem,
    Station,
    Gaslight,
    ElectricalLight,
    MiningTechnology,
    SpinningMill,
    Ball,
    LanguageSchool,
    PatentSystem,
    LetterPressPrinting,
    Winery,
    Brickyard,
    SystemOfStock,
    University,
    Newspaper,
    Telegram,
    Telephone,
    Exposition,
}

pub enum Country {
    English,
    American,
    French,
    German,
    Dutch,
}
```

---

## Turn Phases

```rust
pub enum TurnPhase {
    /// Phase A - before main action
    AdditionalActionFirst,
    
    /// Phase B - main action sequence
    Placement,
    Movement,
    AreaAction,
    FivePowerBonus,
    Building,
    Recovery,
    
    /// Phase C - after main action
    AdditionalActionSecond,
}

/// Tracks what's available in current Additional Action phase
pub struct AdditionalActionState {
    pub mercenary_used_this_turn: bool,
    // Achievement/Order fulfillment is always available if requirements met
}
```

---

## Move Representation

```rust
pub enum Move {
    // Phase A/C: Additional Actions
    UseMercenary {
        country: Country,
        target_area: AreaId,
    },
    FulfillAchievement {
        achievement: AchievementType,
    },
    FulfillOrder {
        order_id: OrderCardId,
    },
    EndAdditionalPhase,
    
    // Step 1: Placement
    PlaceThreeAreas {
        placements: Vec<AreaId>,  // 1-3 areas, 1 assistant each
    },
    PlaceTwoSameArea {
        area: AreaId,
    },
    
    // Step 2: Movement
    MovePresident {
        path: Vec<AreaId>,  // intermediate areas for payment tracking
        destination: AreaId,
    },
    ReturnPresidentToHand {
        recover_assistants_from: Vec<AreaId>,
    },
    
    // Step 3: Area Action (implicit from president location)
    // Varies by area type
    ExecuteAreaAction {
        area_action: AreaAction,
    },
    
    // Step 4: 5-Power Bonus
    Take5PowerToken,
    Skip5PowerBonus,
    
    // Step 5: Building
    BuildShophouse {
        slot: u8,  // 1-4
    },
    BuildTradingHouse,
    SkipBuilding,
    
    // Step 6: Recovery (automatic, no choice)
}

pub enum AreaAction {
    // Production
    ProduceCopper { power: u8 },
    ProduceSilk { power: u8 },
    ProduceTea { power: u8 },
    ProduceSeafood { power: u8 },
    
    // Commercial
    Bank { power: u8 },
    Port {
        cards_taken: Vec<PortSlot>,
        payment: Option<PortPayment>,
    },
    Laboratory {
        card_id: TechCardId,
        money_paid: u32,
        imported_goods_paid: u8,
    },
    EmploymentAgency {
        tokens: Vec<WarehouseToken>,
    },
    Chinatown {
        exchanges: Vec<ChinatownExchange>,
    },
    Church {
        goods_paid: ChurchPayment,
        target_faith: u8,
    },
    Customs {
        imported_goods_paid: u8,
    },
}
```

---

## VLM Extraction Schema

```json
{
  "extraction_type": "turn_delta" | "full_state" | "setup",
  
  "changes": {
    "player_actions": [
      {
        "player": "red" | "blue" | "yellow" | "green",
        "action_type": "placement" | "movement" | "area_action" | "building" | "order" | "tech" | "achievement",
        "details": { ... }
      }
    ],
    
    "board_changes": {
      "assistants_placed": [{"area": "string", "player": "string", "count": 1}],
      "assistants_removed": [{"area": "string", "player": "string", "count": 1}],
      "president_moved": {"player": "string", "from": "string", "to": "string"},
      "building_built": {"area": "string", "player": "string", "type": "shophouse" | "trading_house"},
      "cards_taken": {
        "orders": [{"port": "A" | "B", "slot": "low" | "mid" | "high"}],
        "techs": [{"lab": "A" | "B", "slot": 1-4}]
      },
      "track_changes": {
        "church": {"player": "string", "position": 4-11},
        "customs": {"player": "string", "position": 1-7}
      }
    },
    
    "resource_changes": {
      "player": "string",
      "money": "+/- amount",
      "copper": "+/- amount",
      "silk": "+/- amount",
      "tea": "+/- amount",
      "seafood": "+/- amount",
      "imported_goods": "+/- amount"
    }
  },
  
  "confidence": 0.0-1.0,
  "uncertainties": [
    {
      "type": "string",
      "region": {"x": 0, "y": 0, "w": 0, "h": 0},
      "question": "string",
      "options": ["option1", "option2"]
    }
  ]
}
```

---

## Setup Extraction Schema

```json
{
  "board_layout": {
    "grid": [
      ["area_type", "area_type", "area_type"],
      ["area_type", "area_type", "area_type"],
      ...
    ],
    "adjacencies": [
      {"from": "position", "to": "position"}
    ],
    "canal_crossings": [
      {"area1": "position", "area2": "position"}
    ]
  },
  
  "building_sites": {
    "position": {
      "shophouse_vp": [3, 1, 8, 4],
      "trading_house_vp": 9
    }
  },
  
  "five_power_tokens": {
    "position": "effect_description"
  },
  
  "initial_cards": {
    "lab_a": ["tech_name", "tech_name", "tech_name", "tech_name"],
    "lab_b": ["tech_name", "tech_name", "tech_name", "tech_name"],
    "port_a": {
      "low": "order_description",
      "mid": "order_description", 
      "high": "order_description"
    },
    "port_b": {
      "low": "order_description",
      "mid": "order_description",
      "high": "order_description"
    }
  },
  
  "player_count": 2-4,
  "achievement_cards": {
    "A": "type",
    "B": "type",
    "C": "type"
  }
}
```

---

## Validation Rules

```rust
/// Key validation checks for state transitions

pub fn validate_placement(state: &GameState, placement: &Placement) -> Result<(), ValidationError> {
    // 1. Player has enough assistants in hand
    // 2. Target areas don't include Canal
    // 3. President fee can be paid for each area
    // 4. Option A: all different areas, Option B: same area
}

pub fn validate_movement(state: &GameState, movement: &Movement) -> Result<(), ValidationError> {
    // 1. Each step has player's assistant in destination
    // 2. Path doesn't return to start
    // 3. Destination has no opponent president (unless Postal System)
    // 4. Player can afford all payments (opponents, canal)
}

pub fn validate_area_action(state: &GameState, action: &AreaAction) -> Result<(), ValidationError> {
    // Per-area validation:
    // - Port: hand limit, card availability, payment
    // - Lab: industry value calculation, tech limit cost
    // - Employment: warehouse availability, payment
    // - Church: max 1 of each good type, slot availability
    // - Customs: imported goods available, slot availability
}

pub fn validate_building(state: &GameState, building: &Building) -> Result<(), ValidationError> {
    // 1. Power was 4+
    // 2. Token available in hand
    // 3. Slot not already occupied (by self for shophouse, by anyone for trading house)
}
```
