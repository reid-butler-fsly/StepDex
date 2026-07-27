//! The StepDex dashboard — a single self-contained HTML page (no external CDNs,
//! no build step) with an EdgeWalk-style neon aesthetic. The computed leaderboard
//! JSON is injected inline so the page renders instantly with zero extra fetches.

/// Build the full HTML page, embedding the leaderboard JSON.
pub fn render(data_json: &str) -> String {
    TEMPLATE.replace("/*__STEPDEX_DATA__*/", data_json)
}

const TEMPLATE: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>StepDex — Walk to Earn</title>
<style>
  :root {
    --bg: #070912;
    --panel: rgba(20, 26, 44, 0.72);
    --panel-border: rgba(120, 150, 220, 0.18);
    --text: #e8edff;
    --muted: #8b9bb4;
    --neon: #4fd8ff;
    --neon2: #ff4fd8;
  }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    color: var(--text);
    background:
      radial-gradient(1200px 700px at 15% -10%, rgba(79,216,255,0.16), transparent 60%),
      radial-gradient(1000px 600px at 110% 10%, rgba(255,79,216,0.14), transparent 55%),
      var(--bg);
    min-height: 100vh;
  }
  .wrap { max-width: 1120px; margin: 0 auto; padding: 32px 20px 80px; }
  header .kicker {
    letter-spacing: .32em; text-transform: uppercase; font-size: 12px;
    color: var(--neon); margin-bottom: 8px;
  }
  header h1 {
    margin: 0; font-size: 40px; font-weight: 800; line-height: 1.05;
    background: linear-gradient(90deg, #fff, var(--neon) 60%, var(--neon2));
    -webkit-background-clip: text; background-clip: text; color: transparent;
  }
  header p { color: var(--muted); margin: 10px 0 0; max-width: 660px; }
  .powered { margin-top: 10px; font-size: 12px; color: var(--muted); }
  .powered b { color: var(--text); }

  .grid { display: grid; gap: 16px; }
  .insights { grid-template-columns: repeat(4, 1fr); margin: 28px 0; }
  @media (max-width: 720px){ .insights { grid-template-columns: repeat(2, 1fr);} }
  .card {
    background: var(--panel); border: 1px solid var(--panel-border);
    border-radius: 16px; padding: 18px; backdrop-filter: blur(8px);
  }
  .stat .label { font-size: 12px; color: var(--muted); text-transform: uppercase; letter-spacing: .1em; }
  .stat .value { font-size: 26px; font-weight: 800; margin-top: 6px; }
  .stat .sub { font-size: 12px; color: var(--muted); margin-top: 4px; }

  h2 { font-size: 15px; letter-spacing: .14em; text-transform: uppercase; color: var(--muted); margin: 34px 0 14px; }

  /* Podium */
  .podium { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; align-items: end; }
  @media (max-width: 720px){ .podium { grid-template-columns: 1fr; } }
  .pod {
    position: relative; border-radius: 18px; padding: 20px 18px;
    background: var(--panel); border: 1px solid var(--panel-border); overflow: hidden;
  }
  .pod::before {
    content: ""; position: absolute; inset: -40% -40% auto -40%; height: 160px;
    background: radial-gradient(closest-side, var(--glow), transparent 70%);
    filter: blur(6px); opacity: .55;
  }
  .pod .rankbadge {
    font-size: 12px; font-weight: 700; color: #0a0d18; padding: 3px 10px; border-radius: 999px;
    display: inline-block; background: var(--glow);
  }
  .pod .name { font-size: 20px; font-weight: 800; margin: 12px 0 2px; position: relative; }
  .pod .team { font-size: 12px; color: var(--muted); }
  .pod .steps { font-size: 22px; font-weight: 800; margin-top: 12px; }
  .pod .steps small { font-size: 12px; color: var(--muted); font-weight: 600; }
  .pod .reward { margin-top: 14px; padding-top: 12px; border-top: 1px dashed var(--panel-border); }
  .pod.first { transform: translateY(-14px); }

  .tierchip {
    display: inline-block; font-size: 11px; font-weight: 700; padding: 2px 9px; border-radius: 999px;
    color: #0a0d18;
  }
  .cardname { font-weight: 700; margin-top: 8px; }
  .cardmeta { font-size: 12px; color: var(--muted); margin-top: 2px; }
  .cardval { font-weight: 800; margin-top: 6px; }

  /* Two-column layout for teams + rarity */
  .cols { display: grid; grid-template-columns: 1.4fr 1fr; gap: 16px; align-items: start; }
  @media (max-width: 820px){ .cols { grid-template-columns: 1fr; } }

  /* Table */
  table { width: 100%; border-collapse: collapse; }
  th, td { text-align: left; padding: 11px 12px; border-bottom: 1px solid var(--panel-border); font-size: 14px; }
  th { color: var(--muted); font-size: 11px; text-transform: uppercase; letter-spacing: .1em; }
  td.num, th.num { text-align: right; font-variant-numeric: tabular-nums; }
  tr:hover td { background: rgba(255,255,255,0.02); }
  .rankcell { font-weight: 800; color: var(--muted); width: 42px; }

  /* Bars (teams + rarity) */
  .bars { display: grid; gap: 10px; }
  .bar { display: grid; grid-template-columns: 150px 1fr 62px; align-items: center; gap: 12px; font-size: 13px; }
  .bar.rar { grid-template-columns: 96px 1fr 34px; }
  .bar .track { height: 10px; border-radius: 999px; background: rgba(255,255,255,0.06); overflow: hidden; }
  .bar .fill { height: 100%; border-radius: 999px; }
  .bar .lab { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .bar .lab small { color: var(--muted); }

  /* Daily equalizer sparkline */
  .eq { display: flex; align-items: flex-end; gap: 2px; height: 34px; }
  .eq span { flex: 1 1 0; border-radius: 2px 2px 0 0; min-height: 2px; opacity: .85; }

  /* Showcase */
  .showcase { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px,1fr)); gap: 12px; }
  .poke {
    border-radius: 14px; padding: 14px; background: var(--panel);
    border: 1px solid var(--panel-border); position: relative; overflow: hidden;
  }
  .poke::after {
    content:""; position:absolute; inset:auto -30% -50% -30%; height:120px;
    background: radial-gradient(closest-side, var(--glow), transparent 70%); opacity:.35;
  }
  .poke .pk-rank { font-size: 11px; color: var(--muted); }
  .poke .pk-name { font-weight: 800; margin: 6px 0 2px; position: relative; }
  .btn {
    display:inline-block; font-size:13px; color: var(--neon);
    text-decoration:none; border:1px solid var(--panel-border); padding:2px 10px; border-radius:10px;
  }
  .btn:hover { border-color: var(--neon); }
  footer { margin-top: 40px; color: var(--muted); font-size: 12px; }
</style>
</head>
<body>
<div class="wrap">
  <header>
    <div class="kicker">StepDex · Walk to Earn</div>
    <h1>Every step earns a Pokémon.</h1>
    <p>Rank the step-challenge field by total steps, then hand out cards from the collection —
       the more you walk, the rarer the card you earn. Ranking, team standings, rewards and
       insights are all computed live at the edge.</p>
    <div class="powered" id="range"></div>
    <div class="powered">Computed on <b>Fastly Compute</b> · rarity from live market value ·
       <a class="btn" href="/api/steps.csv">Download CSV</a></div>
  </header>

  <div class="grid insights" id="insights"></div>

  <h2>Podium</h2>
  <div class="podium" id="podium"></div>

  <div class="cols">
    <div>
      <h2>Team standings</h2>
      <div class="card"><div class="bars" id="teams"></div></div>
    </div>
    <div>
      <h2>Rarity handed out</h2>
      <div class="card"><div class="bars" id="rarity"></div></div>
    </div>
  </div>

  <h2>Full leaderboard</h2>
  <div class="card" style="padding:6px 6px; overflow-x:auto">
    <table>
      <thead><tr>
        <th class="num">#</th><th>Walker</th><th>Team</th>
        <th class="num">Steps</th><th class="num">Miles</th>
        <th class="num">Best day</th><th class="num">Goal days</th>
        <th>Daily activity</th>
        <th>Reward</th><th>Rarity</th><th class="num">Value</th>
      </tr></thead>
      <tbody id="rows"></tbody>
    </table>
  </div>

  <h2>Cards earned</h2>
  <div class="showcase" id="showcase"></div>

  <footer>StepDex POC · in the spirit of EdgeWalk · rewards assigned by total-step rank.</footer>
</div>

<script>
const DATA = /*__STEPDEX_DATA__*/;

const fmt = (n) => n.toLocaleString("en-US");
const usd = (n) => "$" + n.toFixed(2);
const miles = (n) => n.toFixed(1) + " mi";

function stat(label, value, sub) {
  return `<div class="card stat"><div class="label">${label}</div>
    <div class="value">${value}</div><div class="sub">${sub||""}</div></div>`;
}

// Neon equalizer: one bar per day, height scaled to the walker's best day,
// colored by their reward tier (falls back to neon cyan).
function equalizer(daily, color) {
  const max = Math.max(1, ...daily);
  return `<div class="eq">` + daily.map(v => {
    const h = v > 0 ? Math.max(6, Math.round(v / max * 100)) : 4;
    const op = v > 0 ? 0.9 : 0.18;
    return `<span title="${fmt(v)}" style="height:${h}%;background:${color};opacity:${op}"></span>`;
  }).join("") + `</div>`;
}

function renderRange() {
  const d = DATA.dates;
  if (d && d.length) {
    document.getElementById("range").textContent =
      `Challenge window: ${d[0]} → ${d[d.length-1]} · ${d.length} days`;
  }
}

function renderInsights() {
  const d = DATA;
  document.getElementById("insights").innerHTML = [
    stat("Steps walked", fmt(d.total_steps), `${d.total_participants} walkers · ${d.total_teams} teams`),
    stat("Distance", miles(d.total_distance_mi), "combined, actual"),
    stat("Rewards value", usd(d.total_reward_value), `${d.cards_available} cards in pool`),
    stat("Top prize", d.top_reward ? d.top_reward.card_name : "—",
         d.top_reward ? `${d.top_reward.tier} · ${usd(d.top_reward.market)}` : ""),
  ].join("");
}

function rewardBlock(r) {
  if (!r) return `<div class="cardmeta">Keep walking — no card yet</div>`;
  return `<span class="tierchip" style="background:${r.tier_color}">${r.tier}</span>
    <div class="cardname">${r.card_name}</div>
    <div class="cardmeta">${r.set_name} · #${r.card_number} · ${r.condition}</div>
    <div class="cardval" style="color:${r.tier_color}">${usd(r.market)}</div>`;
}

function renderPodium() {
  const top = DATA.entries.slice(0, 3);
  const order = [1, 0, 2]; // silver, gold (raised), bronze
  const medals = ["🥇","🥈","🥉"];
  document.getElementById("podium").innerHTML = order.filter(i => top[i]).map(i => {
    const e = top[i];
    const glow = e.reward ? e.reward.tier_color : "#4fd8ff";
    return `<div class="pod ${i===0?'first':''}" style="--glow:${glow}">
      <span class="rankbadge">${medals[i]} #${e.rank}</span>
      <div class="name">${e.name}</div>
      <div class="team">${e.team}</div>
      <div class="steps">${fmt(e.total_steps)} <small>steps · ${miles(e.distance_mi)}</small></div>
      ${equalizer(e.daily, glow)}
      <div class="reward">${rewardBlock(e.reward)}</div>
    </div>`;
  }).join("");
}

function renderTeams() {
  const teams = DATA.teams;
  const max = Math.max(1, ...teams.map(t => t.total_steps));
  document.getElementById("teams").innerHTML = teams.map(t => `
    <div class="bar">
      <span class="lab">${t.rank}. ${t.name}<br><small>${t.members} members · ${fmt(t.avg_per_member)}/ea</small></span>
      <span class="track"><span class="fill" style="width:${(t.total_steps/max*100)}%;
        background:linear-gradient(90deg,#4fd8ff,#ff4fd8)"></span></span>
      <span class="num">${fmt(t.total_steps)}</span>
    </div>`).join("");
}

function renderRarity() {
  const dist = DATA.tier_distribution;
  const max = Math.max(1, ...dist.map(t => t.count));
  document.getElementById("rarity").innerHTML = dist.map(t => `
    <div class="bar rar">
      <span style="color:${t.tier_color};font-weight:700">${t.tier}</span>
      <span class="track"><span class="fill" style="width:${(t.count/max*100)}%;background:${t.tier_color}"></span></span>
      <span class="num">${t.count}</span>
    </div>`).join("");
}

function renderRows() {
  document.getElementById("rows").innerHTML = DATA.entries.map(e => {
    const r = e.reward;
    const glow = r ? r.tier_color : "#4fd8ff";
    return `<tr>
      <td class="num rankcell">${e.rank}</td>
      <td>${e.name}</td>
      <td style="color:var(--muted)">${e.team}</td>
      <td class="num">${fmt(e.total_steps)}</td>
      <td class="num">${e.distance_mi.toFixed(1)}</td>
      <td class="num">${fmt(e.best_day)}</td>
      <td class="num">${e.days_goal_met}/${e.active_days}</td>
      <td style="min-width:150px">${equalizer(e.daily, glow)}</td>
      <td>${r ? r.card_name : "—"}<div class="cardmeta">${r ? r.set_name : ""}</div></td>
      <td>${r ? `<span class="tierchip" style="background:${r.tier_color}">${r.tier}</span>` : ""}</td>
      <td class="num">${r ? usd(r.market) : "—"}</td>
    </tr>`;
  }).join("");
}

function renderShowcase() {
  document.getElementById("showcase").innerHTML = DATA.entries
    .filter(e => e.reward).map(e => {
      const r = e.reward;
      return `<div class="poke" style="--glow:${r.tier_color}">
        <div class="pk-rank">#${e.rank} · ${e.name}</div>
        <div class="pk-name">${r.card_name}</div>
        <div class="cardmeta">${r.set_name}</div>
        <span class="tierchip" style="background:${r.tier_color};margin-top:8px">${r.tier} · ${usd(r.market)}</span>
      </div>`;
    }).join("");
}

renderRange(); renderInsights(); renderPodium();
renderTeams(); renderRarity(); renderRows(); renderShowcase();
</script>
</body>
</html>"##;
