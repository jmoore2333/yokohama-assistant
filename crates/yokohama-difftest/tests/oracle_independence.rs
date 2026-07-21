//! Oracle-independence tripwire (Codex PR-3 review follow-up; guide_sync
//! precedent: mechanically enforce what the docs promise).
//!
//! The phase plan's visible-coupling guarantee says `yokohama-oracle`
//! depends on `yokohama-core` for TYPES ONLY — never logic. Reviews audit
//! that by reading imports; this test makes the audit mechanical:
//!
//! 1. every name imported from `yokohama_core` anywhere in
//!    `crates/yokohama-oracle/src/*.rs` must be on the hardcoded allowlist
//!    below (the audited types surface + `Rules`, built from the actual
//!    imports at adoption time — each name reviewed to be a type, not a
//!    function);
//! 2. the oracle's `Cargo.toml` `[dependencies]` must be exactly
//!    `yokohama-core` + `serde`, with `serde_json` under
//!    `[dev-dependencies]` only.
//!
//! When this test fails on a NEW name: if the name is a serde-serializable
//! TYPE (struct/enum) the oracle legitimately needs, add it to the
//! allowlist AT REVIEW with a note; if it is a function or anything from
//! `yokohama_core::game`'s logic surface, the import is forbidden — author
//! the computation independently from the rules sources instead.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The audited types surface + `Rules` (the static-data bundle). Built from
/// the oracle's actual imports on 2026-07-20; every entry verified to be a
/// TYPE (struct or enum — `BoardLayoutBuilder` is the test-fixture board
/// builder type, data construction not rules logic). Keep sorted.
const ALLOWED_CORE_IMPORTS: &[&str] = &[
    "AchievementCard",
    "AchievementType",
    "AreaAction",
    "AreaConfig",
    "AreaId",
    "AreaType",
    "BoardLayout",
    "BoardLayoutBuilder",
    "BuildingSite",
    "ChinatownExchange",
    "ChurchSlot",
    "ChurchTrack",
    "Country",
    "CustomsSlot",
    "CustomsTrack",
    "FivePowerEffect",
    "GameConfig",
    "GameState",
    "LabBoard",
    "LabState",
    "MercenarySupply",
    "Move",
    "MoveSource",
    "OrderCardId",
    "PendingChoice",
    "PlayerColor",
    "PlayerId",
    "PlayerState",
    "PortBoard",
    "PortPayment",
    "PortState",
    "PresidentLocation",
    "Rules",
    "TechCard",
    "TechCardId",
    "TechEffect",
    "TradeGood",
    "TradingHouseState",
    "TurnPhase",
    "WarehouseItem",
];

fn oracle_crate_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../yokohama-oracle")
}

/// Every `use yokohama_core::...;` statement in `source`, flattened into the
/// individual imported names (brace groups split on commas; whitespace and
/// line breaks stripped). Globs and renames are returned verbatim so the
/// allowlist rejects them with a readable message.
fn core_imported_names(source: &str, file: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = source;
    while let Some(pos) = rest.find("use yokohama_core::") {
        let after = &rest[pos + "use yokohama_core::".len()..];
        let end = after
            .find(';')
            .unwrap_or_else(|| panic!("{file}: unterminated `use yokohama_core::` statement"));
        let body = after[..end]
            .replace(['{', '}'], "")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        for item in body.split(',') {
            let item = item.trim();
            if !item.is_empty() {
                names.push(item.to_string());
            }
        }
        rest = &after[end..];
    }
    names
}

#[test]
fn oracle_imports_from_core_are_types_on_the_allowlist() {
    let allowed: BTreeSet<&str> = ALLOWED_CORE_IMPORTS.iter().copied().collect();
    assert_eq!(
        allowed.len(),
        ALLOWED_CORE_IMPORTS.len(),
        "allowlist has a duplicate entry"
    );

    let src_dir = oracle_crate_dir().join("src");
    let mut files_checked = 0usize;
    let mut violations: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&src_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", src_dir.display()))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    entries.sort();
    assert!(
        entries.iter().any(|p| p.file_name().is_some_and(|n| n == "lib.rs")),
        "oracle src dir must contain lib.rs — wrong path? {}",
        src_dir.display()
    );

    for path in entries {
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        files_checked += 1;
        for name in core_imported_names(&source, &file) {
            seen.insert(name.clone());
            // Reject anything that is not a bare allowlisted name: globs,
            // `as` renames, and nested paths all need eyes at review.
            if !allowed.contains(name.as_str()) {
                violations.push(format!("{file}: `{name}`"));
            }
        }
    }

    assert!(files_checked >= 10, "expected the oracle's module files, found {files_checked}");
    assert!(
        !seen.is_empty(),
        "no yokohama_core imports found at all — the parser or path is broken"
    );
    assert!(
        violations.is_empty(),
        "yokohama-oracle imports names from yokohama_core that are NOT on the audited \
         types allowlist:\n  {}\n\
         The oracle's independence rule (phase plan, visible-coupling guarantee): it may \
         import TYPES ONLY from yokohama-core — never functions or anything from the \
         engine's logic surface (get_legal_moves / apply_move / movement / scoring / \
         choices / data accessors beyond Rules). If a new name is a genuine TYPE the \
         oracle needs, add it to ALLOWED_CORE_IMPORTS in this test AT REVIEW with a \
         note; if it is logic, the import is forbidden — author the computation \
         independently from the rules sources.",
        violations.join("\n  ")
    );
}

/// Parse the dependency names under one `[section]` of a Cargo.toml.
fn toml_section_deps(toml: &str, section: &str) -> BTreeSet<String> {
    let mut deps = BTreeSet::new();
    let mut in_section = false;
    for line in toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == format!("[{section}]");
            continue;
        }
        if !in_section || line.is_empty() || line.starts_with('#') {
            continue;
        }
        // `name = ...` or `name.workspace = true`
        let name: String = line
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '=' && *c != '.')
            .collect();
        if !name.is_empty() {
            deps.insert(name);
        }
    }
    deps
}

#[test]
fn oracle_cargo_dependencies_are_exactly_core_plus_serde() {
    let path = oracle_crate_dir().join("Cargo.toml");
    let toml = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    let deps = toml_section_deps(&toml, "dependencies");
    let expected: BTreeSet<String> =
        ["yokohama-core", "serde"].into_iter().map(String::from).collect();
    assert_eq!(
        deps, expected,
        "yokohama-oracle [dependencies] must be EXACTLY yokohama-core + serde (the \
         visible-coupling guarantee); any new dependency is an independence-review \
         decision, not a routine edit"
    );

    let dev_deps = toml_section_deps(&toml, "dev-dependencies");
    let expected_dev: BTreeSet<String> = ["serde_json"].into_iter().map(String::from).collect();
    assert_eq!(
        dev_deps, expected_dev,
        "yokohama-oracle [dev-dependencies] must be exactly serde_json (test-only JSON \
         plumbing) — serde_json must never move into [dependencies]"
    );
}
