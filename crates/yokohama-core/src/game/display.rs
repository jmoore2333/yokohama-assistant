//! Display layer for card and site rendering.

use crate::types::{BuildingSite, Country, OrderCard, OrderRewards, SpaceReward, TradeGoodRequirement};

/// Display a country with its full name (prompt).
fn country_prompt(country: Country) -> &'static str {
    match country {
        Country::English => "Britain",
        Country::American => "America",
        Country::French => "France",
        Country::German => "Germany",
        Country::Dutch => "Netherlands",
    }
}

/// Display a country with its 2-letter code (compact).
fn country_compact(country: Country) -> &'static str {
    match country {
        Country::English => "GB",
        Country::American => "US",
        Country::French => "FR",
        Country::German => "DE",
        Country::Dutch => "NL",
    }
}

/// Render order requirements in prompt format (plain words).
fn render_requirements_prompt(req: &TradeGoodRequirement) -> String {
    let mut parts = Vec::new();
    if req.copper > 0 {
        parts.push(format!("{} copper", req.copper));
    }
    if req.silk > 0 {
        parts.push(format!("{} silk", req.silk));
    }
    if req.tea > 0 {
        parts.push(format!("{} tea", req.tea));
    }
    if req.seafood > 0 {
        parts.push(format!("{} fish", req.seafood));
    }
    parts.join(" + ")
}

/// Render order requirements in compact format.
fn render_requirements_compact(req: &TradeGoodRequirement) -> String {
    let mut parts = Vec::new();
    if req.copper > 0 {
        parts.push(format!("{}cop", req.copper));
    }
    if req.silk > 0 {
        parts.push(format!("{}silk", req.silk));
    }
    if req.tea > 0 {
        parts.push(format!("{}tea", req.tea));
    }
    if req.seafood > 0 {
        parts.push(format!("{}fish", req.seafood));
    }
    parts.join(" ")
}

/// Render order rewards in prompt format (plain words).
fn render_rewards_prompt(rewards: &OrderRewards) -> String {
    let mut parts = Vec::new();
    if rewards.victory_points > 0 {
        parts.push(format!("{} VP", rewards.victory_points));
    }
    if rewards.money > 0 {
        parts.push(format!("{} yen", rewards.money));
    }
    if rewards.imported_goods > 0 {
        parts.push(format!("{} import", rewards.imported_goods));
    }
    if rewards.copper > 0 {
        parts.push(format!("{} copper", rewards.copper));
    }
    if rewards.silk > 0 {
        parts.push(format!("{} silk", rewards.silk));
    }
    if rewards.tea > 0 {
        parts.push(format!("{} tea", rewards.tea));
    }
    if rewards.seafood > 0 {
        parts.push(format!("{} fish", rewards.seafood));
    }
    if rewards.movement > 0 {
        parts.push(format!("{} move", rewards.movement));
    }
    if rewards.any_trade_good > 0 {
        parts.push(format!("{} any good", rewards.any_trade_good));
    }
    if rewards.warehouse_tokens > 0 {
        parts.push(format!("{} warehouse", rewards.warehouse_tokens));
    }
    parts.join(" + ")
}

/// Render order rewards in compact format (space-separated).
fn render_rewards_compact(rewards: &OrderRewards) -> String {
    let mut parts = Vec::new();
    if rewards.victory_points > 0 {
        parts.push(format!("{}vp", rewards.victory_points));
    }
    if rewards.money > 0 {
        parts.push(format!("{}yen", rewards.money));
    }
    if rewards.imported_goods > 0 {
        parts.push(format!("{}imp", rewards.imported_goods));
    }
    if rewards.copper > 0 {
        parts.push(format!("{}cop", rewards.copper));
    }
    if rewards.silk > 0 {
        parts.push(format!("{}silk", rewards.silk));
    }
    if rewards.tea > 0 {
        parts.push(format!("{}tea", rewards.tea));
    }
    if rewards.seafood > 0 {
        parts.push(format!("{}fish", rewards.seafood));
    }
    if rewards.movement > 0 {
        parts.push(format!("{}move", rewards.movement));
    }
    if rewards.any_trade_good > 0 {
        parts.push(format!("{}any", rewards.any_trade_good));
    }
    if rewards.warehouse_tokens > 0 {
        parts.push(format!("{}wh", rewards.warehouse_tokens));
    }
    parts.join(" ")
}

/// Render an order card in full, readable format for prompts.
///
/// Format: `O17 [Britain]  needs 2 copper + 1 fish  ->  8 VP + 1 import`
pub fn order_prompt_line(card: &OrderCard) -> String {
    let id = card.id.0;
    let country = country_prompt(card.country);
    let requirements = render_requirements_prompt(&card.requirements);
    let rewards = render_rewards_prompt(&card.rewards);
    format!("O{:02} [{}]  needs {}  ->  {}", id, country, requirements, rewards)
}

/// Render an order card in compact format for display.
///
/// Format: `O17 GB  2cop 1fish -> 8vp 1imp`
pub fn order_compact_line(card: &OrderCard) -> String {
    let id = card.id.0;
    let country = country_compact(card.country);
    let requirements = render_requirements_compact(&card.requirements);
    let rewards = render_rewards_compact(&card.rewards);
    format!("O{:02} {}  {} -> {}", id, country, requirements, rewards)
}

/// Render a single space reward in compact form (e.g., "1vp", "2vp+1yen", "-").
fn render_space_compact(space: &SpaceReward) -> String {
    let mut parts = Vec::new();
    if space.vp > 0 {
        parts.push(format!("{}vp", space.vp));
    }
    if space.yen > 0 {
        parts.push(format!("{}yen", space.yen));
    }
    if space.copper > 0 {
        parts.push(format!("{}cop", space.copper));
    }
    if space.silk > 0 {
        parts.push(format!("{}silk", space.silk));
    }
    if space.tea > 0 {
        parts.push(format!("{}tea", space.tea));
    }
    if space.seafood > 0 {
        parts.push(format!("{}fish", space.seafood));
    }
    if space.any_good > 0 {
        parts.push(format!("{}any", space.any_good));
    }
    if space.movement > 0 {
        parts.push(format!("{}move", space.movement));
    }
    if space.warehouse_tokens > 0 {
        parts.push(format!("{}wh", space.warehouse_tokens));
    }

    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("+")
    }
}

/// Printed reward text for one Shophouse space of `site` (e.g. `"1vp"`,
/// `"2vp+1yen"`, `"-"` when blank) -- the same compact formatter
/// `site_compact_line` uses per space, exposed standalone so callers (the
/// CLI's `BuildShophouse` option text) can print a single slot's reward
/// without re-deriving reward-text semantics themselves (Task 12, T2.2a).
/// `None` when `slot` is out of range for the site's `SPACES_PER_SITE`
/// spaces, so callers describing an UNVALIDATED slot fall through to their
/// own bare-slot wording instead of aborting the process. `Session::load`
/// describes each recorded event before `replay_from` validates it, so a
/// tampered record used to reach a raw `site.spaces[slot]` index here
/// (C1, external review 2026-07-20).
pub fn slot_reward_text(site: &BuildingSite, slot: usize) -> Option<String> {
    site.spaces.get(slot).map(render_space_compact)
}

/// Render a building site in compact format.
///
/// Format: `S03  [1vp 2vp+1yen - 2tea]  TH 8vp`
pub fn site_compact_line(index: usize, site: &BuildingSite) -> String {
    // Iterating the array beats four literal `slot_reward_text` calls: every
    // space is in range by construction (no Option to unwrap), and the line
    // cannot drift if `SPACES_PER_SITE` ever changes.
    let spaces = site.spaces.iter().map(render_space_compact).collect::<Vec<_>>().join(" ");
    format!("S{:02}  [{}]  TH {}vp", index, spaces, site.trading_house_reward)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OrderCardId, SPACES_PER_SITE};

    #[test]
    fn order_prompt_line_is_plain_words() {
        let card = OrderCard {
            id: OrderCardId(17),
            country: Country::English,
            requirements: TradeGoodRequirement::new(2, 0, 0, 1),
            rewards: OrderRewards { victory_points: 8, imported_goods: 1, ..Default::default() },
        };
        assert_eq!(
            order_prompt_line(&card),
            "O17 [Britain]  needs 2 copper + 1 fish  ->  8 VP + 1 import"
        );
    }

    #[test]
    fn order_compact_line_uses_short_words() {
        let card = OrderCard {
            id: OrderCardId(17),
            country: Country::English,
            requirements: TradeGoodRequirement::new(2, 0, 0, 1),
            rewards: OrderRewards { victory_points: 8, imported_goods: 1, ..Default::default() },
        };
        assert_eq!(order_compact_line(&card), "O17 GB  2cop 1fish -> 8vp 1imp");
    }

    #[test]
    fn slot_reward_text_renders_a_single_space() {
        let site = BuildingSite {
            spaces: [
                SpaceReward { vp: 1, ..Default::default() },
                SpaceReward { vp: 2, yen: 1, ..Default::default() },
                SpaceReward::default(),
                SpaceReward { tea: 2, ..Default::default() },
            ],
            trading_house_reward: 8,
            shophouses_built: [None; 4],
            trading_house_owner: None,
        };
        assert_eq!(slot_reward_text(&site, 0).as_deref(), Some("1vp"));
        assert_eq!(slot_reward_text(&site, 1).as_deref(), Some("2vp+1yen"));
        assert_eq!(slot_reward_text(&site, 2).as_deref(), Some("-"));
        assert_eq!(slot_reward_text(&site, 3).as_deref(), Some("2tea"));
        // C1 (external review 2026-07-20): an out-of-range slot must yield
        // None so an unvalidated caller falls through to its own wording,
        // rather than indexing the fixed array and aborting the process.
        assert_eq!(slot_reward_text(&site, SPACES_PER_SITE), None);
        assert_eq!(slot_reward_text(&site, 250), None);
    }

    #[test]
    fn site_compact_line_renders_spaces() {
        let site = BuildingSite {
            spaces: [
                SpaceReward { vp: 1, ..Default::default() },
                SpaceReward { vp: 2, yen: 1, ..Default::default() },
                SpaceReward::default(),
                SpaceReward { tea: 2, ..Default::default() },
            ],
            trading_house_reward: 8,
            shophouses_built: [None; 4],
            trading_house_owner: None,
        };
        assert_eq!(site_compact_line(3, &site), "S03  [1vp 2vp+1yen - 2tea]  TH 8vp");
    }
}
