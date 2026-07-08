//! StepDex domain model: parse the embedded CSVs, rank walkers, and hand out
//! Pokémon cards as rewards — the more you walk, the rarer the card you earn.
//!
//! Everything here runs at the edge, per request, inside the Compute WASM guest.

use serde::Serialize;

/// Steps CSV, embedded into the WASM binary at compile time.
/// Schema: participant_id,display_name,team,steps,last_sync
/// Swap in a real CSV with the same columns and rebuild — no code changes needed.
const STEPS_CSV: &str = include_str!("../data/steps.csv");

/// Reward pool CSV (cleaned from the Pokémon collection), sorted rarest-first.
/// Schema: card_name,set_name,card_number,condition,market
const POKEMON_CSV: &str = include_str!("../data/pokemon.csv");

/// Average stride length in meters — matches EdgeWalk's "Average" preset (0.74 m).
const STRIDE_M: f64 = 0.74;
const METERS_PER_MILE: f64 = 1609.344;

/// A Pokémon card available as a reward. `market` (USD) is our rarity proxy.
#[derive(Clone, Debug, Serialize)]
pub struct Card {
    pub card_name: String,
    pub set_name: String,
    pub card_number: String,
    pub condition: String,
    pub market: f64,
    pub tier: &'static str,
    /// Hex accent color for the tier, used by the UI glow/badges.
    pub tier_color: &'static str,
}

/// A walker after ranking, paired with the card their rank earns them.
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub rank: usize,
    pub participant_id: String,
    pub display_name: String,
    pub team: String,
    pub steps: u64,
    pub miles: f64,
    pub last_sync: String,
    /// `None` when there are more walkers than cards in the pool.
    pub reward: Option<Card>,
}

/// One rarity tier bucket for the distribution insight.
#[derive(Clone, Debug, Serialize)]
pub struct TierCount {
    pub tier: &'static str,
    pub tier_color: &'static str,
    pub count: usize,
}

/// The fully computed leaderboard + insights, serialized to the UI as JSON.
#[derive(Clone, Debug, Serialize)]
pub struct Leaderboard {
    pub entries: Vec<Entry>,
    pub total_participants: usize,
    pub total_steps: u64,
    pub total_miles: f64,
    pub total_reward_value: f64,
    pub tier_distribution: Vec<TierCount>,
    pub top_reward: Option<Card>,
    /// Steps walked per $1 of reward value earned across the group.
    pub steps_per_dollar: f64,
    pub cards_available: usize,
}

/// Map a market price to a rarity tier + accent color.
fn tier_for(market: f64) -> (&'static str, &'static str) {
    match market {
        m if m > 50.0 => ("Chase", "#ff4fd8"),
        m if m >= 10.0 => ("Ultra Rare", "#b06bff"),
        m if m >= 2.0 => ("Rare", "#4f8cff"),
        m if m >= 0.5 => ("Uncommon", "#33d6a6"),
        _ => ("Common", "#8b9bb4"),
    }
}

/// Minimal CSV line splitter. Our data files are controlled and comma-free
/// within fields, so a plain split is safe and dependency-light.
fn fields(line: &str) -> Vec<&str> {
    line.split(',').map(|f| f.trim()).collect()
}

/// Parse the reward pool, already sorted rarest-first in the CSV.
fn parse_cards() -> Vec<Card> {
    POKEMON_CSV
        .lines()
        .skip(1) // header
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let f = fields(line);
            if f.len() < 5 {
                return None;
            }
            let market: f64 = f[4].parse().ok()?;
            let (tier, tier_color) = tier_for(market);
            Some(Card {
                card_name: f[0].to_string(),
                set_name: f[1].to_string(),
                card_number: f[2].to_string(),
                condition: f[3].to_string(),
                market,
                tier,
                tier_color,
            })
        })
        .collect()
}

/// Raw walker record before ranking.
struct Walker {
    participant_id: String,
    display_name: String,
    team: String,
    steps: u64,
    last_sync: String,
}

fn parse_walkers() -> Vec<Walker> {
    STEPS_CSV
        .lines()
        .skip(1) // header
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let f = fields(line);
            if f.len() < 5 {
                return None;
            }
            Some(Walker {
                participant_id: f[0].to_string(),
                display_name: f[1].to_string(),
                team: f[2].to_string(),
                steps: f[3].parse().ok()?,
                last_sync: f[4].to_string(),
            })
        })
        .collect()
}

fn steps_to_miles(steps: u64) -> f64 {
    (steps as f64 * STRIDE_M) / METERS_PER_MILE
}

/// Compute the ranked leaderboard: most steps earns the rarest card.
pub fn compute() -> Leaderboard {
    let mut walkers = parse_walkers();
    let cards = parse_cards();

    // Rank by steps, highest first.
    walkers.sort_by(|a, b| b.steps.cmp(&a.steps));

    let mut entries = Vec::with_capacity(walkers.len());
    let mut total_steps: u64 = 0;
    let mut total_reward_value = 0.0;
    let mut tiers: Vec<(&'static str, &'static str, usize)> = vec![
        ("Chase", "#ff4fd8", 0),
        ("Ultra Rare", "#b06bff", 0),
        ("Rare", "#4f8cff", 0),
        ("Uncommon", "#33d6a6", 0),
        ("Common", "#8b9bb4", 0),
    ];

    for (i, w) in walkers.into_iter().enumerate() {
        total_steps += w.steps;
        // Rank i (0-based) earns card i from the rarest-first pool.
        let reward = cards.get(i).cloned();
        if let Some(ref c) = reward {
            total_reward_value += c.market;
            if let Some(t) = tiers.iter_mut().find(|t| t.0 == c.tier) {
                t.2 += 1;
            }
        }
        entries.push(Entry {
            rank: i + 1,
            participant_id: w.participant_id,
            display_name: w.display_name,
            team: w.team,
            steps: w.steps,
            miles: steps_to_miles(w.steps),
            last_sync: w.last_sync,
            reward,
        });
    }

    let total_participants = entries.len();
    let total_miles = steps_to_miles(total_steps);
    let steps_per_dollar = if total_reward_value > 0.0 {
        total_steps as f64 / total_reward_value
    } else {
        0.0
    };

    let tier_distribution = tiers
        .into_iter()
        .map(|(tier, tier_color, count)| TierCount {
            tier,
            tier_color,
            count,
        })
        .collect();

    Leaderboard {
        entries,
        total_participants,
        total_steps,
        total_miles,
        total_reward_value,
        tier_distribution,
        top_reward: cards.first().cloned(),
        steps_per_dollar,
        cards_available: cards.len(),
    }
}

/// Expose the raw steps CSV (for the "Download CSV" feature).
pub fn steps_csv() -> &'static str {
    STEPS_CSV
}
