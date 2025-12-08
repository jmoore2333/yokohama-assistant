# A1 Verification Design

**Created:** 2025-12-07
**Purpose:** Systematic verification of A1 implementation plan with TDD remediation

---

## Goal

Verify every item in the A1 implementation plan is correctly implemented with:
- Test coverage
- Code that matches the official Yokohama rulebook
- Edge cases handled

When gaps are found, fix immediately with TDD (failing test first, then fix).

---

## Verification Criteria

For each item to be marked "verified":

1. **Test exists and passes** - At least one test covers the feature
2. **Code review** - Implementation matches official rulebook rules
3. **Edge cases** - Boundary conditions, empty states, error conditions handled

---

## Reference Materials

| Priority | Document | Purpose |
|----------|----------|---------|
| Primary | `docs/plans/2025-12-07-A1-implementation.md` | What to verify |
| Primary | `docs/plans/2025-12-07-A1-foundation.md` | Type definitions |
| Authoritative | `analysis-1/00-yokohama-rulebook.pdf` | Official rules |
| Secondary | `analysis-1/yokohama-kb/*.md` | Convenient markdown (unofficial) |

---

## Workflow

### Per-Item Verification

1. Read the plan item
2. Locate implementation in `crates/yokohama-core/src/`
3. Find tests covering this feature
4. Code review against rulebook
5. Check edge cases
6. If gap found:
   - Write failing test (TDD red)
   - Fix implementation (TDD green)
   - Refactor if needed
7. Update implementation plan: `[ ]` → `[x]` with note
8. Update verification checklist

### Per-Section Flow

1. Read section of implementation plan
2. Create TodoWrite items for features
3. Verify each feature
4. Commit section (code + checklist)
5. Move to next section

---

## Section Order

Skip Tasks 1-9 (foundation/types - covered by foundation plan).

Start at Task 10, verify in order:

1. Power calculation
2. Movement
3. Legal moves generation
4. Apply move / state transitions
5. Area actions (production, port, lab, etc.)
6. Building
7. Scoring
8. End game triggers
9. Tech effects integration

---

## Deliverables

1. **Updated implementation plan** - All verified items marked `[x]`
2. **Verification checklist** - `docs/plans/2025-12-07-A1-verification.md`
3. **TDD fixes** - Any gaps fixed with proper tests
4. **Updated handover** - Session summary in `agent-handover.md`

---

## Remediation Process

When a gap is found:

```
1. Write failing test that exposes the gap
2. Run test - confirm it fails
3. Fix the implementation
4. Run test - confirm it passes
5. Check for regressions (run full test suite)
6. Update checklist with "Fixed" status
```

---

## Progress Tracking

- TodoWrite for current section items
- Verification checklist for cumulative progress
- Commits after each major section
- Handover doc updated at session end
