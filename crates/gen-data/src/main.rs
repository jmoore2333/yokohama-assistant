//! Generates `crates/yokohama-core/src/game/data.rs` from the verified card YAML
//! in `analysis-1/yokohama-kb/`. Run with `cargo run -p gen-data` from the workspace
//! root (paths below are relative to that root).
//!
//! This crate intentionally does not depend on `yokohama-core`: it emits Rust
//! source text (struct literals referencing `yokohama-core`'s types by name)
//! rather than constructing real instances of those types.

use std::collections::HashSet;
use std::fmt::Write as _;

use serde::Deserialize;

const ORDERS_PATH: &str = "analysis-1/yokohama-kb/07-order-cards.yaml";
const TECHS_PATH: &str = "analysis-1/yokohama-kb/08-tech-cards.yaml";
const SITES_PATH: &str = "analysis-1/yokohama-kb/09-building-sites.yaml";
const OUTPUT_PATH: &str = "crates/yokohama-core/src/game/data.rs";

const VALID_TECH_EFFECTS: &[&str] = &[
    "LanguageSchool",
    "Telegram",
    "Telephone",
    "Newspaper",
    "Exposition",
    "Ball",
    "Gaslight",
    "Station",
    "University",
    "LetterPressPrinting",
    "PostalSystem",
    "Winery",
    "ElectricalLight",
    "MiningTechnology",
    "SpinningMill",
    "PatentSystem",
    "Stagecoach",
    "Tram",
    "Brickyard",
    "SystemOfStock",
];

// ---------------------------------------------------------------------------
// YAML shapes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderCardsFile {
    order_cards: Vec<OrderCardYaml>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderCardYaml {
    id: u8,
    #[allow(dead_code)] // traceability only (sprite-sheet cell)
    position: String,
    country: String,
    #[serde(default)]
    requirements: RequirementsYaml,
    #[serde(default)]
    rewards: OrderRewardYaml,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RequirementsYaml {
    #[serde(default)]
    seafood: u8,
    #[serde(default)]
    tea: u8,
    #[serde(default)]
    silk: u8,
    #[serde(default)]
    copper: u8,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct OrderRewardYaml {
    #[serde(default)]
    vp: u8,
    #[serde(default)]
    yen: u8,
    #[serde(default)]
    imports: u8,
    #[serde(default)]
    assistants: u8,
    #[serde(default)]
    shophouse_build: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TechCardsFile {
    tech_cards: Vec<TechCardYaml>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TechCardYaml {
    id: String,
    #[allow(dead_code)] // traceability only (sprite-sheet cells)
    #[serde(default)]
    positions: Vec<String>,
    copies: u8,
    name: String,
    effect: String,
    industry_cost: u8,
    country: String,
    #[allow(dead_code)] // flavor text, not modeled
    #[serde(default)]
    description: String,
    #[allow(dead_code)] // editorial note, not modeled
    #[serde(default)]
    note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildingSitesFile {
    building_sites: Vec<BuildingSiteYaml>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildingSiteYaml {
    #[allow(dead_code)] // traceability only (site_NNN label)
    id: String,
    #[allow(dead_code)] // traceability only (sprite-sheet cell)
    position: String,
    spaces: SpacesYaml,
    trading_house_vp: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpacesYaml {
    #[serde(rename = "TL")]
    tl: SpaceRewardYaml,
    #[serde(rename = "TR")]
    tr: SpaceRewardYaml,
    #[serde(rename = "ML")]
    ml: SpaceRewardYaml,
    #[serde(rename = "MR")]
    mr: SpaceRewardYaml,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct SpaceRewardYaml {
    #[serde(default)]
    vp: u8,
    #[serde(default)]
    yen: u8,
    #[serde(default)]
    seafood: u8,
    #[serde(default)]
    tea: u8,
    #[serde(default)]
    silk: u8,
    #[serde(default)]
    copper: u8,
    #[serde(default)]
    any_good: u8,
    #[serde(default)]
    shophouse_build: u8,
    #[serde(default)]
    assistants: u8,
}

// ---------------------------------------------------------------------------
// Mapping helpers
// ---------------------------------------------------------------------------

/// Maps a YAML country code to the `Country` enum variant name (as source text).
fn country_variant(code: &str) -> Result<&'static str, String> {
    Ok(match code {
        "US" => "American",
        "DE" => "German",
        "NL" => "Dutch",
        "FR" => "French",
        "GB" => "English",
        other => return Err(format!("unknown country code {other:?}")),
    })
}

fn validate_tech_effect(effect: &str) -> Result<(), String> {
    if VALID_TECH_EFFECTS.contains(&effect) {
        Ok(())
    } else {
        Err(format!("unknown TechEffect variant {effect:?}"))
    }
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn fmt_trade_good_requirement(r: &RequirementsYaml) -> String {
    format!(
        "TradeGoodRequirement {{ copper: {}, silk: {}, tea: {}, seafood: {} }}",
        r.copper, r.silk, r.tea, r.seafood
    )
}

// Rulebook p.10 "Description of the Icons": the YAML's `assistants`/`shophouse_build`
// keys describe the printed art (a meeple-with-arrow, a house-with-red-arrow), not the
// rule. The meeple-with-arrow is the *Movement* icon (move an assistant from hand or an
// area to another area; no President fees) and the house-with-red-arrow is the
// *Warehouse* icon (take a token — assistant/shophouse/trading house — from the
// Warehouse tile to hand, at Employment Agency cost). Both reward structs (OrderRewards,
// SpaceReward) therefore use `movement`/`warehouse_tokens`, not the YAML's visual names.

fn fmt_order_rewards(r: &OrderRewardYaml) -> String {
    format!(
        "OrderRewards {{ victory_points: {}, money: {}, imported_goods: {}, copper: 0, silk: 0, tea: 0, seafood: 0, any_trade_good: 0, movement: {}, warehouse_tokens: {} }}",
        r.vp, r.yen, r.imports, r.assistants, r.shophouse_build
    )
}

fn fmt_space_reward(r: &SpaceRewardYaml) -> String {
    format!(
        "SpaceReward {{ vp: {}, yen: {}, seafood: {}, tea: {}, silk: {}, copper: {}, any_good: {}, movement: {}, warehouse_tokens: {} }}",
        r.vp, r.yen, r.seafood, r.tea, r.silk, r.copper, r.any_good, r.assistants, r.shophouse_build
    )
}

/// Builds the `(OrderCard, code)` literal text for one order card.
fn build_order_entry(o: &OrderCardYaml) -> Result<(String, String), String> {
    let country = country_variant(&o.country).map_err(|e| format!("order id {}: {e}", o.id))?;
    let code = format!("O{:02}", o.id);
    let literal = format!(
        "OrderCard {{ id: OrderCardId({}), country: Country::{}, requirements: {}, rewards: {} }}",
        o.id,
        country,
        fmt_trade_good_requirement(&o.requirements),
        fmt_order_rewards(&o.rewards)
    );
    Ok((literal, code))
}

/// One physical Technology card after expanding `copies`.
struct TechEntry {
    id: u32,
    code: String,
    name: String,
    country: &'static str,
    industry_cost: u8,
    effect: String,
}

/// Expands the 20 unique tech titles (with `copies: 1|2`) into 30 physical
/// cards, minting consecutive ids in YAML file order.
fn expand_techs(entries: &[TechCardYaml]) -> Result<Vec<TechEntry>, String> {
    let mut out = Vec::new();
    let mut next_id: u32 = 1;
    let mut seen_codes = HashSet::new();

    for t in entries {
        validate_tech_effect(&t.effect).map_err(|e| format!("tech {:?}: {e}", t.id))?;
        let country = country_variant(&t.country).map_err(|e| format!("tech {:?}: {e}", t.id))?;
        if t.copies != 1 && t.copies != 2 {
            return Err(format!("tech {:?}: unsupported copies value {} (expected 1 or 2)", t.id, t.copies));
        }

        let mut push_copy = |id: u32, code: String| -> Result<(), String> {
            if !seen_codes.insert(code.clone()) {
                return Err(format!("duplicate generated tech code {code:?}"));
            }
            out.push(TechEntry {
                id,
                code,
                name: t.name.clone(),
                country,
                industry_cost: t.industry_cost,
                effect: t.effect.clone(),
            });
            Ok(())
        };

        push_copy(next_id, t.id.clone())?;
        next_id += 1;
        if t.copies == 2 {
            push_copy(next_id, format!("{}_b", t.id))?;
            next_id += 1;
        }
    }

    let total = next_id - 1;
    if total != 30 {
        return Err(format!("expected 30 physical tech cards after expanding copies, got {total}"));
    }
    Ok(out)
}

fn fmt_tech_card(t: &TechEntry) -> String {
    format!(
        "TechCard {{ id: TechCardId({}), name: \"{}\".to_string(), country: Country::{}, industry_cost: {}, effect: TechEffect::{} }}",
        t.id,
        escape_str(&t.name),
        t.country,
        t.industry_cost,
        t.effect
    )
}

fn fmt_building_site(s: &BuildingSiteYaml) -> String {
    format!(
        "BuildingSite {{ spaces: [{}, {}, {}, {}], trading_house_reward: {}, shophouses_built: [None; 4], trading_house_owner: None }}",
        fmt_space_reward(&s.spaces.tl),
        fmt_space_reward(&s.spaces.tr),
        fmt_space_reward(&s.spaces.ml),
        fmt_space_reward(&s.spaces.mr),
        s.trading_house_vp
    )
}

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

fn read_yaml<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    serde_yaml::from_str(&text).map_err(|e| format!("parsing {path}: {e}"))
}

fn run() -> Result<(), String> {
    let orders_file: OrderCardsFile = read_yaml(ORDERS_PATH)?;
    let techs_file: TechCardsFile = read_yaml(TECHS_PATH)?;
    let sites_file: BuildingSitesFile = read_yaml(SITES_PATH)?;

    if orders_file.order_cards.len() != 36 {
        return Err(format!(
            "expected 36 order cards in {ORDERS_PATH}, found {}",
            orders_file.order_cards.len()
        ));
    }
    if sites_file.building_sites.len() != 24 {
        return Err(format!(
            "expected 24 building sites in {SITES_PATH}, found {}",
            sites_file.building_sites.len()
        ));
    }

    // Orders: validate id range/uniqueness, then build literals.
    let mut seen_order_ids = HashSet::new();
    let mut order_lines = Vec::new();
    let mut seen_order_codes = HashSet::new();
    for o in &orders_file.order_cards {
        if !(1..=36).contains(&o.id) {
            return Err(format!("order id {} out of range 1..=36", o.id));
        }
        if !seen_order_ids.insert(o.id) {
            return Err(format!("duplicate order id {}", o.id));
        }
        let (literal, code) = build_order_entry(o)?;
        if !seen_order_codes.insert(code.clone()) {
            return Err(format!("duplicate generated order code {code:?}"));
        }
        order_lines.push(format!("        ({literal}, \"{code}\"),"));
    }

    // Techs: expand copies, then build literals.
    let techs = expand_techs(&techs_file.tech_cards)?;
    let tech_lines: Vec<String> = techs
        .iter()
        .map(|t| format!("        ({}, \"{}\"),", fmt_tech_card(t), t.code))
        .collect();

    // Building sites: build literals in file order.
    let site_lines: Vec<String> = sites_file
        .building_sites
        .iter()
        .map(|s| format!("        {},", fmt_building_site(s)))
        .collect();

    let mut out = String::new();
    writeln!(out, "// GENERATED by gen-data — DO NOT EDIT").unwrap();
    writeln!(out, "// Source: {ORDERS_PATH}, {TECHS_PATH}, {SITES_PATH}").unwrap();
    writeln!(out, "// Regenerate with: cargo run -p gen-data (from the workspace root)").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "use crate::types::{{BuildingSite, Country, OrderCard, OrderCardId, OrderRewards, SpaceReward, TechCard, TechCardId, TechEffect, TradeGoodRequirement}};"
    )
    .unwrap();
    writeln!(out).unwrap();

    writeln!(out, "pub(crate) fn standard_orders() -> Vec<(OrderCard, &'static str)> {{").unwrap();
    writeln!(out, "    vec![").unwrap();
    for line in &order_lines {
        writeln!(out, "{line}").unwrap();
    }
    writeln!(out, "    ]").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "pub(crate) fn standard_techs() -> Vec<(TechCard, &'static str)> {{").unwrap();
    writeln!(out, "    vec![").unwrap();
    for line in &tech_lines {
        writeln!(out, "{line}").unwrap();
    }
    writeln!(out, "    ]").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "pub(crate) fn standard_sites() -> Vec<BuildingSite> {{").unwrap();
    writeln!(out, "    vec![").unwrap();
    for line in &site_lines {
        writeln!(out, "{line}").unwrap();
    }
    writeln!(out, "    ]").unwrap();
    writeln!(out, "}}").unwrap();

    std::fs::write(OUTPUT_PATH, out).map_err(|e| format!("writing {OUTPUT_PATH}: {e}"))?;

    println!(
        "gen-data: wrote {} orders, {} techs, {} building sites to {OUTPUT_PATH}",
        order_lines.len(),
        tech_lines.len(),
        site_lines.len()
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("gen-data: {e}");
        std::process::exit(1);
    }
}
