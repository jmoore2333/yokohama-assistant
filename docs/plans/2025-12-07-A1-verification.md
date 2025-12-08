# A1 Verification Checklist

**Started:** 2025-12-07
**Status:** In Progress

---

## Summary

| Metric | Count |
|--------|-------|
| Total items | TBD |
| Verified | 48 |
| Gaps fixed | 11 |
| Remaining | TBD |

---

## Legend

- ✅ Verified (test + code review + edge cases pass)
- ❌ Gap found (needs fix)
- 🔧 Fixed (was gap, now complete with TDD)
- ⏳ In progress

---

## Verification Log

### Task 1: Power Calculation

| Item | Status | Tests | Notes |
|------|--------|-------|-------|
| President +1 | ✅ | test_power_president_only | Verified |
| Assistants +1 each | ✅ | test_power_with_assistants | Verified |
| Shophouse +1 | ✅ | test_power_shophouse_bonus | Added edge case test |
| Trading House +1 | ✅ | test_power_trading_house_bonus | Added edge case test |
| Station +1 for ALL | ✅ | test_power_station_benefits_all_players | Added test for multi-player |
| Cap at 5 | ✅ | test_power_capped_at_5 | Verified |
| Zero power | ✅ | test_power_zero_no_presence | Added edge case test |
| All sources combined | ✅ | test_power_all_sources_combined | Added integration test |

### Task 2: Production Area Actions

| Item | Status | Tests | Notes |
|------|--------|-------|-------|
| Copper Mine rewards | ✅ | test_copper_mine_rewards | Power 3→1, 4→2, 5→3 |
| Silk Mill rewards | ✅ | test_silk_mill_rewards | Power 2→1, 3→2, 4→3, 5→4 |
| Tea Plantation rewards | ✅ | test_tea_plantation_rewards | 1:1 with power |
| Fishing Ground rewards | ✅ | test_fishing_ground_rewards | 1:1 with power |
| Power 0 edge case | ✅ | test_power_zero_returns_zero | Added - all return 0 |
| Power >5 capped | ✅ | test_power_capped_at_five | Added - max rewards at 5 |
| Mining Technology | ✅ | test_mining_technology_bonus | In tech_effects.rs |

### Task 3: Commercial Area Actions

| Item | Status | Tests | Notes |
|------|--------|-------|-------|
| Bank reward | ✅ | test_bank_reward | 1 Yen per power, capped at 5 |
| Employment Agency | ✅ | test_employment_agency_tokens | Power 2→1, 3→2, 4→3, 5→4 |
| Employment full range | ✅ | test_employment_agency_full_range | Added edge cases |
| Chinatown validation | ✅ | test_chinatown_exchange_validation | Exchange count vs power |
| Power 0 edge case | ✅ | test_power_zero_returns_zero | Added |
| Chinatown rates | ✅ | apply_chinatown_exchange | Copper/Silk→2Y, Tea/Seafood→1Y |

### Task 4: Port and Laboratory Actions

| Item | Status | Tests | Notes |
|------|--------|-------|-------|
| Port accessible slots | ✅ | test_port_accessible_slots | Power 2→Low, 3→Mid, 4+→All |
| Port Power 5 bonus | ✅ | test_port_power_5_bonus, test_port_execute_power_5_vp_bonus | +3 VP at power 5 |
| Port hand limit | ✅ | test_port_hand_limit_validation | Max 3 cards in hand |
| Second card payment | ✅ | test_port_second_card_payment_required | 2 Yen OR 1 Imported |
| Port execute + slide | ✅ | test_port_execute_takes_cards_to_hand, test_port_slide | Cards slide left |
| Lab industry value | ✅ | test_industry_value_calculation | Power + 2Y=1IV + Imported |
| Lab position surcharge | ✅ | test_lab_position_surcharge | Slots 2,3 cost +1,+2 |
| Fifth tech penalty | ✅ | test_fifth_tech_penalty, test_lab_fifth_tech_penalty | 5th+ costs extra |
| Lab validation | ✅ | test_lab_validate_industry_value | IV check works |
| Lab execute | ✅ | test_lab_execute_acquires_tech | Acquires card properly |
| Refill from deck | ✅ | test_port_refill, test_lab_refill | Deck management |

### Task 5: Church and Customs Actions

| Item | Status | Tests | Notes |
|------|--------|-------|-------|
| Church faith from power | ✅ | test_church_faith_from_power | Power 2→1 Faith (critical rule) |
| Calculate total faith | ✅ | test_calculate_church_faith | Power + goods contributions |
| Customs max imported | ✅ | test_customs_max_imported | Power-based limit |
| Power 0 edge case | ✅ | test_power_zero_edge_cases | Added |
| Power >5 capped | ✅ | test_power_above_five_capped | Added |
| Church/Customs validation | ✅ | validate_church_action, validate_customs_action | Resource + assistant checks |

### Task 6: Movement and Pathfinding

| Item | Status | Tests | Notes |
|------|--------|-------|-------|
| Possible action area | ✅ | test_is_possible_action_area | Assistant required, president blocking |
| Find reachable areas | ✅ | test_find_reachable_areas | BFS through assistant chain |
| Canal toll cost | ✅ | test_movement_cost_canal | 1 Yen per crossing |
| Mercenary ignores blocking | ✅ | test_mercenary_ignores_opponent_president_blocking | Bypasses presidents |
| Mercenary no president cost | ✅ | test_mercenary_no_cost_for_opponent_presidents | Free passage |
| Mercenary pays canal | ✅ | test_mercenary_still_pays_canal_toll | Still pays tolls |
| Mercenary combined | ✅ | test_mercenary_with_opponent_and_canal | Both scenarios |
| Stagecoach free passage | ✅ | Code review: lines 229-233 | Tech integration |
| Postal System stopping | ✅ | Code review: lines 55-70 | Tech integration |
| Station teleport | ✅ | test_station_teleport_included_in_movement | In legal_moves.rs |

---

## Session Notes

### 2025-12-07

- Session start: Verification design complete
- Focus: Sequential verification from Task 10 onwards
