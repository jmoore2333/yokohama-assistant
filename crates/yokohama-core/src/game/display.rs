//! Display layer for card and site rendering.

use crate::types::{BuildingSite, Country, OrderCard, OrderCardId, OrderRewards, SpaceReward, TradeGoodRequirement};

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

/// Render a building site in compact format.
///
/// Format: `S03  [1vp 2vp+1yen - 2tea]  TH 8vp`
pub fn site_compact_line(index: usize, site: &BuildingSite) -> String {
    let spaces = &site.spaces;
    let space1 = render_space_compact(&spaces[0]);
    let space2 = render_space_compact(&spaces[1]);
    let space3 = render_space_compact(&spaces[2]);
    let space4 = render_space_compact(&spaces[3]);
    let th_reward = site.trading_house_reward;
    format!("S{:02}  [{} {} {} {}]  TH {}vp", index, space1, space2, space3, space4, th_reward)
}

#[cfg(test)]
mod tests {
    use super::*;

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
