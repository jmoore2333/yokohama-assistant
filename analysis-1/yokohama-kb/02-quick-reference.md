# Yokohama Quick Reference Tables

> Optimized for fast programmatic lookups during game state evaluation.

---

## Area Action Rewards

### Production Areas

```
COPPER_MINE:
  power_1: 0 copper
  power_2: 0 copper
  power_3: 1 copper
  power_4: 2 copper
  power_5: 3 copper

SILK_MILL:
  power_1: 0 silk
  power_2: 1 silk
  power_3: 2 silk
  power_4: 3 silk
  power_5: 4 silk

TEA_PLANTATION:
  power_1: 1 tea
  power_2: 2 tea
  power_3: 3 tea
  power_4: 4 tea
  power_5: 5 tea

FISHING_GROUND:
  power_1: 1 seafood
  power_2: 2 seafood
  power_3: 3 seafood
  power_4: 4 seafood
  power_5: 5 seafood
```

### Commercial Areas

```
BANK:
  power_1: 1 yen
  power_2: 2 yen
  power_3: 3 yen
  power_4: 4 yen
  power_5: 5 yen

PORT:
  power_1: cannot_take
  power_2: [low]
  power_3: [low, mid]
  power_4: [low, mid, high]
  power_5: [low, mid, high] + 3 VP
  card_cost:
    one_card: free
    two_cards: 2 yen OR 1 imported_good

LABORATORY:
  # Power converts to Industry Value 1:1 (max 5)
  # Additional sources: 2 yen = 1 IV, 1 imported_good = 1 IV
  # Card cost = printed_cost + position_modifier (0, +1, or +2)
  # 5th+ tech requires extra: 2 yen OR 1 imported_good

EMPLOYMENT_AGENCY:
  power_1: 0 tokens
  power_2: 1 token
  power_3: 2 tokens
  power_4: 3 tokens
  power_5: 4 tokens
  token_costs:
    assistant: 0 yen
    shophouse: 2 yen
    trading_house_1st: 4 yen
    trading_house_2nd: 5 yen
    trading_house_3rd: 6 yen
    trading_house_4th: 7 yen

CHINATOWN:
  # exchanges_allowed = power
  exchange_rates:
    copper_to_yen: [1, 2]      # pay 1 copper, get 2 yen
    silk_to_yen: [1, 2]
    tea_to_yen: [1, 1]
    seafood_to_yen: [1, 1]
    yen_to_copper: [2, 1]
    yen_to_silk: [2, 1]
    yen_to_tea: [1, 1]
    yen_to_seafood: [1, 1]
    yen_to_imported: [4, 1]
    # NOTE: cannot sell imported_goods

CHURCH:
  faith_from_power:
    power_1: 1
    power_2: 1
    power_3: 3
    power_4: 4
    power_5: 5
  faith_from_goods:  # max 1 each per turn
    copper: 1
    silk: 1
    tea: 1
    seafood: 1
    yen: 1
    imported_good: 1
  track_rewards:
    faith_4: {vp: 3}
    faith_5: {vp: 4}
    faith_6: {vp: 5, movement: 2}
    faith_7: {vp: 6, yen: 1}
    faith_8: {vp: 7, movement: 1}
    faith_9: {vp: 8, imported_good: 1}
    faith_10: {vp: 11}
    faith_11: {vp: 14}

CUSTOMS:
  imported_goods_allowed:
    power_1: 0
    power_2: 1
    power_3: 2
    power_4: 3
    power_5: 4
  track_rewards:
    goods_1: {vp: 5, yen: 2}
    goods_2: {vp: 4, yen: 2}
    goods_3: {vp: 10, imported_good: 1}
    goods_4: {vp: 9, imported_good: 1}
    goods_5: {vp: 16}
    goods_6: {vp: 15}
    goods_7plus: {vp: 22}

CANAL:
  # No action, movement only
  crossing_cost: 1 yen
```

---

## Power Calculation

```
power_sources:
  president_pawn: 1
  assistant_pawn: 1  # per pawn
  shophouse: 1
  trading_house: 1
  station_token: 1   # if present in area

max_effective_power: 5
```

---

## Foreign Mercenaries

```
supply:
  english: 3
  american: 2
  french: 2
  german: 2
  dutch: 1

acquisition_trigger:
  # Every 2nd matching country icon (2, 4, 6...)
  # Icons from: completed_orders + technology_cards

usage:
  limit: once_per_turn
  target_requirements:
    - has_your_assistant: true
    - has_your_president: false
    - has_opponent_president: allowed  # no payment needed
  effect: execute_steps_3_through_6
```

---

## Achievement Cards

```
category_A:  # Trade Good holdings (no payment)
  copper_5:    {first: 8, subsequent: 6}
  silk_6:      {first: 8, subsequent: 6}
  tea_7:       {first: 8, subsequent: 6}
  seafood_7:   {first: 8, subsequent: 6}

category_B:  # Collection goals
  yen_10:           {first: 9, subsequent: 7}
  tokens_5:         {first: 9, subsequent: 7}  # 5-power + mercenaries
  orders_5:         {first: 9, subsequent: 7}
  technologies_4:   {first: 9, subsequent: 7}

category_C:  # Building presence
  production_4:     {first: 10, subsequent: 8}
  commercial_4:     {first: 10, subsequent: 8}
  prod3_comm2:      {first: 10, subsequent: 8}
  prod2_comm3:      {first: 10, subsequent: 8}
```

---

## End Game Triggers

```
triggers:
  trading_houses_built: 4      # any single player
  shophouses_built: 8          # any single player
  order_deck_empty: true       # AND empty slot on Port board
  church_assistants:
    2_player: 4
    3_player: 5
    4_player: 6
  customs_assistants:
    2_player: 4
    3_player: 5
    4_player: 6

procedure:
  1: complete_current_round    # until before start_player
  2: play_final_round          # all players one turn
  3: final_scoring
```

---

## Final Scoring

```
1_church_bonus:
  first_place: 6 VP
  second_place: 3 VP
  tiebreaker: rightmost_assistant

2_customs_bonus:
  first_place: 8 VP
  second_place: 4 VP
  tiebreaker: rightmost_assistant

3_technology_bonus:
  # Compare total Industry Value on owned tech cards
  first_place: 10 VP
  second_place: 5 VP
  tiebreaker: closest_to_start_player

4_country_bonus:
  set_5_different: 12 VP
  set_4_different: 8 VP
  set_3_different: 4 VP
  set_2_different: 2 VP
  # Each icon used once, optimize for largest sets

5_remainder_scoring:
  unused_mercenary: 1 VP each
  imported_good: 1 VP each
  per_2_yen: 1 VP
  per_3_trade_goods: 1 VP  # any combination
```

---

## Technology Card Effects

```
# Movement
tram:
  industry_cost: 5
  effect: "move_president_anywhere_no_opponent_president"
  restriction: "no_additional_movement_this_turn"

stagecoach:
  industry_cost: 3
  effect: "pass_through_opponent_presidents_free"
  restriction: "still_cannot_stop_there"

postal_system:
  industry_cost: ?
  effect: "may_stop_and_act_in_opponent_president_area"
  restriction: "still_pay_passing_unless_have_stagecoach"

station:
  industry_cost: 4
  on_acquire: "place_station_token_any_area"
  passive: "+1_power_for_all_players_in_area"
  owner_bonus: "may_move_directly_to_station_free"
  restriction: "only_if_taking_that_action"

# Building Bonuses
gaslight:
  industry_cost: 3
  trigger: "build_shophouse"
  effect: "+2_VP"

electrical_light:
  industry_cost: ?
  trigger: "build_trading_house"
  effect: "+3_VP"

# Production Bonuses
mining_technology:
  industry_cost: 3
  trigger: "receive_copper"
  effect: "+1_copper"

spinning_mill:
  industry_cost: 4
  trigger: "once_per_turn"
  effect: "discard_1_silk_gain_2_yen"

# Order/Tech Bonuses
ball:
  industry_cost: 5
  trigger: "complete_order"
  effect: "+2_VP"

language_school:
  industry_cost: ?
  trigger: "complete_order"
  effect: "+1_yen"

patent_system:
  industry_cost: 3
  trigger: "acquire_technology"
  effect: "+2_VP"

letterpress_printing:
  industry_cost: ?
  trigger: "acquire_technology"
  effect: "+1_yen"

# Track Bonuses
winery:
  industry_cost: ?
  trigger: "place_assistant_church"
  effect: "+3_VP"

brickyard:
  industry_cost: 4
  trigger: "place_assistant_customs"
  effect: "+4_VP"

# Economy
system_of_stock:
  industry_cost: ?
  trigger: "turn_start"
  condition: "yen <= 1"
  effect: "refill_to_2_yen"

university:
  industry_cost: ?
  trigger: "once_per_turn"
  effect: "pay_1_yen_take_assistant_from_warehouse"

# Placement
newspaper:
  industry_cost: ?
  effect: "placement_option_a_allows_4_areas"

telegram:
  industry_cost: ?
  effect: "after_placing_2_same_area_may_place_1_adjacent"

telephone:
  industry_cost: 6
  effect: "recovery_step_may_leave_1_assistant"

# Instant
exposition:
  industry_cost: 5
  effect: "immediately_gain_10_VP"
```

---

## Setup Reference

```
player_count_config:
  2_player:
    areas_removed: [laboratory_b, port_b, chinatown, canal, 
                    copper_mine_x1, silk_mill_x1, tea_plantation_x1, fishing_ground_x1]
    order_cards_removed: 16
    track_trigger: 4
    
  3_player:
    areas_removed: [laboratory_b, copper_mine_x1, silk_mill_x1, tea_plantation_x1]
    order_cards_removed: 5
    track_trigger: 5
    
  4_player:
    areas_removed: []
    order_cards_removed: 0
    track_trigger: 6

starting_resources:
  start_player:
    yen: 3
  other_players:
    yen: 4
  all_players:
    president_in_hand: 1
    assistants_in_hand: 8
    assistants_in_warehouse: 12
    shophouses_in_hand: 2
    shophouses_in_warehouse: 6
    trading_houses_in_warehouse: 4
    copper: 1
    silk: 1
    tea: 1
    seafood: 1
    order_cards: 1  # draw 2, keep 1

board_setup:
  - shuffle_area_boards_facedown
  - place_in_grid_configuration
  - place_building_site_card_each_area
  - place_5power_token_each_area
  - place_tech_cards_on_laboratories
  - place_order_cards_on_ports
```

---

## Area Categories

```
production_areas:
  - copper_mine
  - silk_mill
  - tea_plantation
  - fishing_ground

commercial_areas:
  - bank
  - port_a
  - port_b
  - laboratory_a
  - laboratory_b
  - employment_agency
  - chinatown
  - church
  - customs

special_areas:
  - canal  # no action, crossing only
```

---

## Resource Value Reference (for Evaluator)

```
# Approximate relative values for heuristic evaluation
trade_good_base_values:
  copper: 2.0    # hardest to get (3 power minimum)
  silk: 1.5      # medium difficulty
  tea: 1.0       # easiest
  seafood: 1.0   # easiest

imported_good_value: 3.0  # versatile, multiple uses

# Context multipliers
resource_toward_order: 1.5x   # if needed for held order
resource_excess: 0.5x         # beyond immediate needs

# Yen purchasing power
yen_to_copper: 2      # via Chinatown
yen_to_silk: 2
yen_to_tea: 1
yen_to_seafood: 1
yen_to_imported: 4
```
