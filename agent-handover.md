# Agent Handover: Yokohama AI Assistant

**Last Updated:** 2025-12-07
**Session Summary:** Plans COMPLETE - integrated v3 feedback, rulebook corrections, and technical audit. Ready for A1 implementation.

**Next Action:** Execute A1-Foundation (Tasks 1-10), then A1-Implementation (Tasks 1-16)

---

## Project Overview

Building an AR assistant for the board game **Yokohama** that:
- Observes physical tabletop via overhead camera
- Extracts game state using cloud VLM (Gemini/Claude)
- Provides real-time strategic recommendations via MCTS AI
- Runs on local hardware (Xeon + RTX 5000 + 64GB RAM)

**Key insight discovered:** Yokohama is nearly perfect information - the AI knows ALL players' Order cards (except each player's starting card). This dramatically simplifies the AI architecture.

---

## Current Status: Ready to Execute A1 Plans

The A1 (Rust Game Engine) implementation plans are **complete and reviewed with v3 feedback integrated**. Next step is to execute them.

**v3 feedback integrated (2025-12-07):**
- Church/Customs: assistant-from-hand requirement, power=2→1 Faith special case, Winery/Brickyard bonuses
- Movement: Stagecoach free passage, Postal System stopping, Station teleport ignoring opponents
- Placement: Newspaper 4-area, Telegram adjacent bonus, combinatorial blowup concerns
- Port/Lab: hand limit 3, second card payment, position surcharges, fifth-tech penalty
- Building: power thresholds (Shophouse 4+, Trading House 5), Gaslight/Electrical Light bonuses
- Scoring: country set distinctness constraint, majority tie-breakers (rightmost/closest-to-start)
- Setup: player-count variants (2p/3p/4p area removals, Port B/Lab B presence)

**Rulebook corrections integrated (additional review):**
1. **5-Power Movement Token (CRITICAL FIX)**: Moves **ASSISTANTS** not President - tactical repositioning tool, ignores adjacency/fees (Page 10, Source 532)
2. **Mercenary Rules (RESOLVED)**: Opponent Presidents do **NOT** block mercenaries; **NO** 1 Yen payment required; only OWN President blocks (Page 8, Source 392-394)
3. **Employment Agency (ADDED)**: Full implementation with power-based item limits and specific costs (Assistants: free, Shophouses: 2¥, Trading Houses: 4/5/6/7¥) (Page 12, Source 706-718)

**Architectural edge cases addressed:**
- Zero-assistant soft-lock prevention: `ReturnPresidentToHand` always available as emergency exit (Page 6, Step 2b)
- Canal traversal: pathfinding allows crossing Canal (1¥ cost) without stopping there

**Technical audit findings integrated (Yokohama Assistant Rule Alignment Review):**
1. **Trading House Passive Income (CRITICAL)**: When any player acts in an area, OTHER players with Trading Houses there receive 1 Yen each from bank - this is the primary economic engine for long-term strategy
2. **Start Area Return Prohibition**: President cannot return to the area it started in during the same turn - prevents "passing" by moving back and forth
3. **Placement Cost Validation**: legal_moves generator must filter by affordability (1 Yen per area with opponent President)

**Already verified as implemented:**
- Two-Assistant placement option (`PlaceTwoSameArea`)
- Mercenary once-per-turn tracking (`mercenary_used` flag)
- Achievement claim tracking (`claimed_by: Vec<PlayerId>`)
- Variable Trading House costs (4/5/6/7 Yen)
- Telephone technology (leave 1 assistant during recovery)

### GitHub Repository
```
https://github.com/jmoore2333/yokohama-assistant (private)
```

### Repository Structure
```
/home/jmoore/Documents/yokohama/
├── agent-handover.md                 # This file
├── analysis-1/
│   ├── 00-yokohama-rulebook.pdf      # Official game rules (16 pages)
│   └── yokohama-kb/                  # Knowledge base for game rules
│       ├── 00-README.md              # KB overview and usage
│       ├── 01-rules-reference.md     # Complete rules in structured format
│       ├── 02-quick-reference.md     # Lookup tables (power, rewards, etc.)
│       ├── 03-state-schema.md        # Draft Rust data structures
│       ├── 04-strategy-guide.md      # Strategic insights for evaluator
│       ├── 05-vlm-prompts.md         # VLM extraction prompts
│       └── 06-visual-reference.md    # Visual guide for CV
├── docs/plans/
│   ├── 2025-12-07-yokohama-assistant-design.md  # High-level design doc
│   ├── 2025-12-07-A1-foundation.md   # A1 Part 1: Core types (10 tasks)
│   ├── 2025-12-07-A1-implementation.md  # A1 Part 2: Game logic (16 tasks)
│   ├── A1-notes.md                   # Review feedback v1
│   └── a1-notes-v2.md                # Review feedback v2
```

---

## A1 Implementation Plans Summary

### A1-Foundation (10 Tasks)
Core type definitions with zero game logic:
1. Initialize Cargo workspace
2. Define ID types (PlayerId, AreaId, OrderCardId, TechCardId) with TryFrom for CV safety
3. Define board types (AreaType, BuildingSite, BoardLayout, BoardLayoutBuilder)
4. Define player state types (AssistantState, ShophouseState, TradingHouseState, MercenaryState)
5. Define card types (OrderCard, TechCard, TechEffect enum)
6. Define track types (ChurchTrack, CustomsTrack)
7. Define move types (Move enum, AreaAction enum)
8. Define game state (GameState, GameConfig, TurnPhase)
9. Add helper methods
10. Final cleanup

**Key features:**
- `InvalidIdError` + `try_new()` for safe ID construction from CV data
- `BoardLayoutBuilder` with validation (adjacency, sites, 5-Power tokens)
- `starting_order: Option<OrderCardId>` for hidden Order card tracking
- Comprehensive glossary mapping rulebook terms to Rust types

### A1-Implementation (16 Tasks)
Complete game rules engine:
1. Power calculation
2. Production area actions
3. Commercial area actions (Bank, Employment, Chinatown)
4. Port and Laboratory actions
5. Church and Customs actions
6. Movement and pathfinding
7. Legal move generation
8. State transitions (apply_move)
9. Final scoring (with registries)
10. Card registries and Order fulfillment
11. Achievement validation
12. Technology effects
13. End game triggers
14. Integration tests
15. Integration test scenarios
16. **Complete Missing Features** (comprehensive subtasks for all TODOs)

**Task 16 covers (16 subtasks total):**
- 16.1-16.2: Movement/Placement tech hooks (Stagecoach, Postal, Station, Newspaper, Telegram)
- 16.3-16.4: Port/Lab validation (hand limit 3, IV calculation, position surcharges)
- 16.5-16.6: Building/Recovery (Gaslight/Electrical Light bonuses, Telephone)
- 16.7-16.9: Scoring/End-game (country sets, tech majority, Order deck trigger)
- 16.10: Setup configuration (2p/3p/4p variants)
- 16.11: Mercenary rules (RESOLVED - opponents don't block, no payment)
- 16.12: 5-Power Movement (CORRECTED - moves Assistants, not President)
- 16.13: Building power thresholds (Shophouse 4+, Trading House 5)
- **16.14: Trading House Passive Income (CRITICAL audit finding)**
- **16.15: Start Area Return Prohibition (audit finding)**
- **16.16: Placement Cost Validation (audit finding)**

---

## Key Design Decisions

### Architecture
- **Rust core** (`yokohama-core`) for game engine + MCTS (Python GIL is bottleneck)
- **Python API layer** (FastAPI) for HTTP/WebSocket and VLM integration
- **Cloud VLM** (Gemini 3 Pro primary, Claude 4.5 fallback)
- **Web UI** (React + TypeScript + Tailwind)

### Information Model
- Nearly perfect information game
- `PlayerState.starting_order` tracks hidden starting Order card
- AI knows 2 of 3 opponent Order cards (only starting card uncertain)
- Board layout randomized at setup, persists for session

### Performance Targets
- `GameState` cheap to clone (minimize allocations)
- `apply_move` zero allocations in hot path
- Target: 10,000+ apply_move calls/second for MCTS

---

## Next Steps: Execute A1 Implementation

### Status: Plans Complete, Ready for Coding

All planning and review is **DONE**. The implementation plans have been through:
- Initial design review
- v2 feedback (IV math, validations)
- v3 feedback (rulebook edge cases)
- Technical audit (Trading House income, movement rules)
- Rulebook corrections (5-Power tokens, mercenaries, Employment Agency)

**No more planning needed** - proceed directly to implementation.

### Recommended Execution Order

**Phase 1: Foundation (A1-Foundation, Tasks 1-10)**
```bash
cd /home/jmoore/Documents/yokohama

# Read the foundation plan
cat docs/plans/2025-12-07-A1-foundation.md

# Start with Task 1: Initialize Cargo workspace
# Each task has exact file paths, code samples, and commit commands
```

**Phase 2: Game Logic (A1-Implementation, Tasks 1-16)**
```bash
# After foundation is complete
cat docs/plans/2025-12-07-A1-implementation.md

# Task 16 is the comprehensive "catch-all" with 16 subtasks
# Execute 16.1-16.16 to complete all edge cases
```

### For Claude Code Users
The plans are structured for task-by-task execution:
```
> Use superpowers:executing-plans to implement this plan task-by-task.
```

Or use `/superpowers:execute-plan` slash command.

### Reference Documents (use during implementation)
- `analysis-1/yokohama-kb/02-quick-reference.md` - Power tables, reward values
- `analysis-1/yokohama-kb/01-rules-reference.md` - Rule clarifications
- `analysis-1/00-yokohama-rulebook.pdf` - Original source (PDF)

---

## Git Commands

```bash
cd /home/jmoore/Documents/yokohama

# Check current state
git status
git log --oneline -5

# Pull latest (if needed)
git pull

# View plans
cat docs/plans/2025-12-07-A1-foundation.md
cat docs/plans/2025-12-07-A1-implementation.md
```

---

## After A1: Next Modules

Once A1 is complete, parallel work can begin on:

| Module | Description | Dependencies |
|--------|-------------|--------------|
| A2 | MCTS search algorithm | A1 complete |
| A3 | Heuristic evaluator | A1 complete |
| B1 | VLM prompt engineering | None (can start now) |
| C1 | React UI shell | None (can start now) |

---

## Notes for Next Agent

**Implementation guidance:**
- **Plans are COMPLETE** - No more review needed, just execute task-by-task
- **Use the yokohama-kb** - It has accurate rule lookups (Church Power 2 = 1 Faith, etc.)
- **Task 16 has 16 subtasks** - All edge cases, tech effects, and audit findings are there
- **CV safety matters** - Use `try_new()` for IDs from external data
- **The user knows Yokohama well** - Leverage his expertise for evaluator weights

**Critical rules to remember (commonly missed):**
- Trading House Passive Income: other TH owners get 1 Yen when you act in their area
- 5-Power Movement token moves ASSISTANTS, not President
- Mercenaries bypass opponent Presidents (no blocking, no payment)
- Church Power 2 → only 1 Faith (not 2)
- President cannot return to starting area in same turn

**Performance targets:**
- 10,000+ apply_move calls/second for MCTS
- `GameState` cheap to clone
- Zero allocations in hot path

---

## Recent Commits

```
bde80d4 Integrate v3 feedback and technical audit into A1 plans
ac6a005 Address A1 plan v2 feedback: IV math, validations, hidden order
e45f30f Address A1 plan feedback: safe constructors, builders, task 16
51bb6d3 Add knowledge base and A1 implementation plans
7a6af2f Expand design document with critical details
```

---

## Quick Start Checklist

**Foundation (do first):**
- [ ] Read `docs/plans/2025-12-07-A1-foundation.md`
- [ ] Task 1: Initialize Cargo workspace
- [ ] Task 2: Define ID types with TryFrom
- [ ] Task 3: Define board types with BoardLayoutBuilder
- [ ] Task 4-10: Complete remaining foundation types
- [ ] Verify: `cargo build -p yokohama-core`

**Implementation (do second):**
- [ ] Read `docs/plans/2025-12-07-A1-implementation.md`
- [ ] Tasks 1-15: Core game logic
- [ ] Task 16: All 16 subtasks (edge cases, tech effects, audit fixes)
- [ ] Verify: `cargo test -p yokohama-core`

**Done when:**
- [ ] All tests pass
- [ ] `get_legal_moves()` returns valid moves for any state
- [ ] `apply_move()` correctly transitions state
- [ ] End game detection works for all 5 triggers
