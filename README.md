# StepDex — Walk to Earn 🃏👟

A Fastly Compute (Rust) POC in the spirit of [EdgeWalk](https://edgewalk.demo-fastly.com/):
it turns a **steps CSV** into a neon leaderboard and rewards the biggest walkers with
**Pokémon cards** — the more you walk, the rarer the card you earn. Ranking, rewards, and
insights are all computed **at the edge**, per request.

## How it works

- **Rank-based podium** — walkers are sorted by total steps; #1 earns the rarest available card,
  #2 the next-rarest, and so on.
- **Rarity = market value** — the reward pool (`data/pokemon.csv`) is sorted by each card's
  MARKET price, bucketed into tiers: `Chase >$50 · Ultra Rare $10–50 · Rare $2–10 ·
  Uncommon $0.50–2 · Common <$0.50`.
- **Team standings** — walkers are aggregated by their team, ranked by combined steps.
- **Insights** — combined distance, total reward value, rarity distribution, plus per-walker
  best day, goal-days met, and a neon daily-activity equalizer (in the spirit of EdgeWalk).

## Endpoints

| Route | Description |
|-------|-------------|
| `GET /` | The neon dashboard (leaderboard JSON injected inline — zero extra fetches) |
| `GET /api/leaderboard` | The computed leaderboard as JSON |
| `GET /api/steps.csv` | Download the raw steps CSV |

## Run locally

```sh
fastly compute build
fastly compute serve      # http://127.0.0.1:7676
```

## Swap in real data

Both CSVs are embedded at compile time (`include_str!`), so just replace the files and rebuild.

- **`data/steps.csv`** — a step-challenge export. Columns: `Team Source, Name, Total Steps,
  Avg Daily Steps, Daily Step Goal, Total Distance (mi), Total Distance (km),
  Avg Daily Distance (mi), Avg Daily Distance (km)`, followed by one column per day
  (`YYYY-MM-DD`). The parser is quote-aware (`"12,345"`) and tolerates `N.A` for missing days.
- **`data/pokemon.csv`** — schema: `card_name,set_name,card_number,condition,market`
  (sorted rarest-first; `market` is the rarity proxy)

```sh
# edit data/steps.csv ...
fastly compute build && fastly compute serve
```

## Source layout

- `src/model.rs` — CSV parsing, rarity tiers, rank-based reward assignment, insights
- `src/ui.rs` — the self-contained HTML/CSS/JS dashboard (no external CDNs)
- `src/main.rs` — request router

## Toolchain note

Built with Fastly CLI 15.4.0, `fastly` crate 0.13, stable Rust, target `wasm32-wasip1`.

## Not yet done (easy follow-ups)

- Deploy to a real Fastly account for a public edge URL (`fastly compute publish`)
- Live real-time updates via Fanout (EdgeWalk's live feed)
- Load the CSV from a Fastly KV Store instead of compile-time embedding
