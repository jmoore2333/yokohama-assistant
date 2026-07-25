# Yokohama AI Assistant Knowledge Base

> Reference documentation for the Yokohama board game AI assistant project.
> Designed for use with Claude Code and the Rust game engine.

---

## Document Index

| File | Purpose | Primary Users |
|------|---------|---------------|
| `01-rules-reference.md` | Complete game rules in structured format | Game Engine, VLM, Human reference |
| `02-quick-reference.md` | Lookup tables for fast programmatic access | Game Engine, Evaluator |
| `03-state-schema.md` | Data structures for state representation | Game Engine, VLM extraction |
| `04-strategy-guide.md` | Strategic insights for heuristic evaluation | MCTS Evaluator |
| `05-vlm-prompts.md` | Prompt templates for vision model extraction | VLM Pipeline |
| `06-visual-reference.md` | Guide for rulebook image extraction | VLM Pipeline, UI |

---

## Quick Links by Task

### Implementing the Game Engine
1. Start with `03-state-schema.md` for data structures
2. Reference `01-rules-reference.md` for rule implementation
3. Use `02-quick-reference.md` for reward tables

### Building the VLM Pipeline
1. Use `05-vlm-prompts.md` for extraction prompts
2. Reference `06-visual-reference.md` for image preparation
3. Validate against `03-state-schema.md` output format

### Tuning the Evaluator
1. Study `04-strategy-guide.md` for strategic factors
2. Reference `02-quick-reference.md` for scoring values
3. Use `01-rules-reference.md` for end-game calculation

### Debugging State Issues
1. Check `01-rules-reference.md` for rule clarification
2. Validate against `03-state-schema.md` structures
3. Cross-reference `02-quick-reference.md` tables

---

## Key Rule Clarifications

### Most Common Edge Cases

**Movement through opponent Presidents:**
- Must pay 1 Yen to each opponent President in areas you pass through
- Cannot stop in area with opponent President (unless have Postal System)
- Stagecoach allows free passage but not stopping

**Power calculation:**
- President (1) + Assistants (1 each) + Shophouse (1) + Trading House (1) + Station (1 if present)
- Maximum effective power is 5 (higher values treated as 5)
- Station bonus applies to ALL players in that area

**Church faith contribution limits:**
- Maximum 1 of each resource type per Church action
- Can contribute: 1 Copper + 1 Silk + 1 Tea + 1 Seafood + 1 Yen + 1 Imported Good
- Power contributes separately (not limited)

**Technology 5th+ card penalty:**
- 5th Technology card and beyond each require additional 2 Yen OR 1 Imported Good
- This is in addition to the Industry Value cost
- Applies per card (5th costs extra, 6th costs extra, etc.)

**Order card hand limit:**
- Maximum 3 Order cards in hand at any time
- Cannot discard Order cards
- Cannot take from Port if would exceed limit

**Foreign Mercenary usage:**
- Once per turn (across both Additional Action phases)
- Target area must have your assistant but NOT your President
- Can target area with opponent President (no payment needed)

---

## Information Visibility

### Public Information
- All board state (areas, pawns, buildings)
- All resource holdings
- All track positions (Church, Customs)
- All Technologies owned
- All completed Order cards
- Order cards taken from Port (everyone sees which)
- Technology cards taken from Lab
- Available cards on Port and Lab boards
- Achievement card progress

### Hidden Information
- An opponent's starting Order card only -- every OTHER card in their hand was taken from the Port in public view (see Tracking Insight below)
- Starting Order card (draw 2, keep 1 at game start)

### Tracking Insight
Since everyone sees which Order cards are taken from Port:
- You know 2 of 3 Order cards each opponent holds
- Only uncertainty: their starting card
- Opponent behavior may reveal starting card over time

---

## Usage with Claude Code

### Querying Rules
```
User: "What happens if I have 6 power in an area?"
Claude Code: Check 01-rules-reference.md, section "Power Calculation"
Answer: Power values 6+ are treated as maximum 5
```

### Validating State Transitions
```
User: "Is this move legal: place 2 assistants in Port, then 1 in Bank"
Claude Code: Check 01-rules-reference.md, section "B1. Placement Step"
Answer: No - Option A requires 3 DIFFERENT areas; this places 2 in Port
```

### Looking Up Rewards
```
User: "How much silk do I get with 4 power at Silk Mill?"
Claude Code: Check 02-quick-reference.md, SILK_MILL section
Answer: 3 Raw Silk tokens
```

### Strategic Advice
```
User: "Should I prioritize Ball or Gaslight technology?"
Claude Code: Check 04-strategy-guide.md, "Technology Synergies" section
Answer: Depends on strategy - Ball for Order focus (+2 VP per order), 
        Gaslight for Shophouse rush (+2 VP per shophouse, max +16 VP)
```

---

## Version Notes

- Based on OKAZU Brand English rulebook
- Covers base game (no expansions)
- Designed for 2-4 player games
- Compatible with standard and randomized setups

---

## File Checksums

Generate after finalizing documents:
```bash
md5sum *.md > checksums.md5
```

Use to verify document integrity in production.
