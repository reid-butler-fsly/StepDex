//! StepDex domain model: parse the embedded step-challenge CSV, rank walkers
//! and teams, and hand out Pokémon cards as rewards — the more you walk, the
//! rarer the card you earn.
//!
//! Everything here runs at the edge, per request, inside the Compute WASM guest.

use serde::Serialize;

/// Step-challenge export, embedded into the WASM binary at compile time.
/// Real Gemini/Fitbit-style export: 9 meta columns then one column per day.
/// Header: Team Source,Name,Total Steps,Avg Daily Steps,Daily Step Goal,
///         Total Distance (mi),Total Distance (km),Avg Daily Distance (mi),
///         Avg Daily Distance (km),<date>,<date>,...
const STEPS_CSV: &str = include_str!("../data/steps.csv");

/// Reward pool CSV (cleaned from the Pokémon collection), sorted rarest-first.
/// Schema: card_name,set_name,card_number,condition,market
const POKEMON_CSV: &str = include_str!("../data/pokemon.csv");

/// Number of leading (non-daily) columns in the steps CSV.
const META_COLS: usize = 9;

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
    /// Card art URL (Pokémon TCG CDN) for the hover preview; "" if unknown.
    pub image_url: String,
}

/// A walker after ranking, paired with the card their rank earns them.
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub rank: usize,
    pub name: String,
    pub team: String,
    pub total_steps: u64,
    pub avg_daily: u64,
    pub goal: u64,
    pub distance_mi: f64,
    /// Best single-day step count.
    pub best_day: u64,
    /// Days with a recorded (non-N.A) step count.
    pub active_days: usize,
    /// Days the walker hit or beat their daily goal.
    pub days_goal_met: usize,
    /// Per-day steps (0 fills a missing/N.A day) for the equalizer sparkline.
    pub daily: Vec<u64>,
    /// `None` when there are more walkers than cards in the pool.
    pub reward: Option<Card>,
}

/// A team's aggregate standing.
#[derive(Clone, Debug, Serialize)]
pub struct Team {
    pub rank: usize,
    pub name: String,
    pub total_steps: u64,
    pub distance_mi: f64,
    pub members: usize,
    pub avg_per_member: u64,
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
    /// Daily column labels (dates), for the equalizer axis.
    pub dates: Vec<String>,
    pub entries: Vec<Entry>,
    pub teams: Vec<Team>,
    pub total_participants: usize,
    pub total_teams: usize,
    pub total_steps: u64,
    pub total_distance_mi: f64,
    pub total_reward_value: f64,
    pub tier_distribution: Vec<TierCount>,
    pub top_reward: Option<Card>,
    pub cards_available: usize,
}

/// Map a market price to a rarity tier + accent color.
fn tier_for(market: f64) -> (&'static str, &'static str) {
    match market {
        m if m >= 350.0 => ("Grail", "#ffd24f"),
        m if m >= 250.0 => ("Chase", "#ff4fd8"),
        m if m >= 150.0 => ("Ultra Rare", "#b06bff"),
        m if m >= 60.0 => ("Rare", "#4f8cff"),
        _ => ("Premium", "#33d6a6"),
    }
}

/// Quote-aware CSV line splitter. Handles double-quoted fields that contain
/// commas (e.g. `"234,139"`) and escaped `""` quotes. Returns trimmed fields.
fn split_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

/// Parse a possibly-quoted, comma-grouped integer; `N.A`/blank -> `None`.
fn parse_num(s: &str) -> Option<u64> {
    let t = s.trim().replace(',', "");
    if t.is_empty() || t.eq_ignore_ascii_case("n.a") || t.eq_ignore_ascii_case("na") {
        return None;
    }
    t.parse::<f64>().ok().map(|v| v.round() as u64)
}

/// Parse a float (distance); `N.A`/blank -> `None`.
fn parse_f(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', "");
    if t.is_empty() || t.eq_ignore_ascii_case("n.a") {
        return None;
    }
    t.parse::<f64>().ok()
}

/// Parse the reward pool, already sorted rarest-first in the CSV.
fn parse_cards() -> Vec<Card> {
    POKEMON_CSV
        .lines()
        .skip(1) // header
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let f = split_csv_line(line);
            if f.len() < 5 {
                return None;
            }
            let market: f64 = f[4].parse().ok()?;
            let (tier, tier_color) = tier_for(market);
            Some(Card {
                card_name: f[0].clone(),
                set_name: f[1].clone(),
                card_number: f[2].clone(),
                condition: f[3].clone(),
                market,
                tier,
                tier_color,
                image_url: f.get(5).cloned().unwrap_or_default(),
            })
        })
        .collect()
}

/// Raw walker record before ranking.
struct Walker {
    team: String,
    name: String,
    total_steps: u64,
    avg_daily: u64,
    goal: u64,
    distance_mi: f64,
    daily: Vec<Option<u64>>,
}

/// Read the date labels from the CSV header (columns after the meta block).
fn parse_dates() -> Vec<String> {
    match STEPS_CSV.lines().next() {
        Some(h) => split_csv_line(h).into_iter().skip(META_COLS).collect(),
        None => Vec::new(),
    }
}

fn parse_walkers() -> Vec<Walker> {
    STEPS_CSV
        .lines()
        .skip(1) // header
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let f = split_csv_line(line);
            if f.len() < META_COLS {
                return None;
            }
            Some(Walker {
                team: f[0].clone(),
                name: f[1].clone(),
                total_steps: parse_num(&f[2]).unwrap_or(0),
                avg_daily: parse_num(&f[3]).unwrap_or(0),
                goal: parse_num(&f[4]).unwrap_or(0),
                distance_mi: parse_f(&f[5]).unwrap_or(0.0),
                daily: f[META_COLS..].iter().map(|c| parse_num(c)).collect(),
            })
        })
        .collect()
}

/// Aggregate walkers into ranked team standings.
fn build_teams(walkers: &[(usize, &Walker)]) -> Vec<Team> {
    // Preserve first-seen order while summing.
    let mut order: Vec<String> = Vec::new();
    let mut steps: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut dist: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let mut count: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for (_, w) in walkers {
        if !steps.contains_key(&w.team) {
            order.push(w.team.clone());
        }
        *steps.entry(w.team.clone()).or_insert(0) += w.total_steps;
        *dist.entry(w.team.clone()).or_insert(0.0) += w.distance_mi;
        *count.entry(w.team.clone()).or_insert(0) += 1;
    }

    let mut teams: Vec<Team> = order
        .into_iter()
        .map(|name| {
            let total_steps = steps[&name];
            let members = count[&name];
            Team {
                rank: 0,
                total_steps,
                distance_mi: dist[&name],
                members,
                avg_per_member: if members > 0 {
                    total_steps / members as u64
                } else {
                    0
                },
                name,
            }
        })
        .collect();

    teams.sort_by(|a, b| b.total_steps.cmp(&a.total_steps));
    for (i, t) in teams.iter_mut().enumerate() {
        t.rank = i + 1;
    }
    teams
}

/// Compute the ranked leaderboard: most steps earns the rarest card.
pub fn compute() -> Leaderboard {
    let dates = parse_dates();
    let mut walkers = parse_walkers();
    let cards = parse_cards();

    // Rank individuals by total steps, highest first.
    walkers.sort_by(|a, b| b.total_steps.cmp(&a.total_steps));

    // Team standings use the same underlying walker set.
    let indexed: Vec<(usize, &Walker)> = walkers.iter().enumerate().collect();
    let teams = build_teams(&indexed);

    let mut entries = Vec::with_capacity(walkers.len());
    let mut total_steps: u64 = 0;
    let mut total_distance_mi = 0.0;
    let mut total_reward_value = 0.0;
    let mut tiers: Vec<(&'static str, &'static str, usize)> = vec![
        ("Grail", "#ffd24f", 0),
        ("Chase", "#ff4fd8", 0),
        ("Ultra Rare", "#b06bff", 0),
        ("Rare", "#4f8cff", 0),
        ("Premium", "#33d6a6", 0),
    ];

    for (i, w) in walkers.iter().enumerate() {
        total_steps += w.total_steps;
        total_distance_mi += w.distance_mi;

        let best_day = w.daily.iter().filter_map(|d| *d).max().unwrap_or(0);
        let active_days = w.daily.iter().filter(|d| d.is_some()).count();
        let days_goal_met = w
            .daily
            .iter()
            .filter_map(|d| *d)
            .filter(|&v| w.goal > 0 && v >= w.goal)
            .count();
        let daily: Vec<u64> = w.daily.iter().map(|d| d.unwrap_or(0)).collect();

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
            name: w.name.clone(),
            team: w.team.clone(),
            total_steps: w.total_steps,
            avg_daily: w.avg_daily,
            goal: w.goal,
            distance_mi: w.distance_mi,
            best_day,
            active_days,
            days_goal_met,
            daily,
            reward,
        });
    }

    let tier_distribution = tiers
        .into_iter()
        .map(|(tier, tier_color, count)| TierCount {
            tier,
            tier_color,
            count,
        })
        .collect();

    Leaderboard {
        dates,
        total_participants: entries.len(),
        total_teams: teams.len(),
        entries,
        teams,
        total_steps,
        total_distance_mi,
        total_reward_value,
        tier_distribution,
        top_reward: cards.first().cloned(),
        cards_available: cards.len(),
    }
}

/// Expose the raw steps CSV (for the "Download CSV" feature).
pub fn steps_csv() -> &'static str {
    STEPS_CSV
}
