# Decisions

Rationale log. Newest at the bottom. Each entry: decision, why, what would change it.

## Tauri over Electron

Requirement is an always-running tray utility with near-zero idle cost. Electron holds Chromium+Node permanently; Tauri uses the system WebView2 + a Rust process (measured ~33 MB idle). The user's C drive is nearly full — toolchains were installed on D: (`RUSTUP_HOME`, `CARGO_HOME`, `BUN_INSTALL`).

## Heatmaps are derived, never stored

Every entry is an event row first; heatmap cells are `COUNT(DISTINCT canonical_topic)` queries. Schema changes (streaks, mastery, weekly views) never require data migrations or backfills.

## Intensity = distinct topics per day

Typing "2sum" five times must not produce a darker cell than five different topics. Buckets: 0 / 1-2 / 3-4 / 5-6 / 7+ (GitHub-compatible).

## Classification: ranking over noul; slugs over names

Live experiments against the daemon (see [[Laya-Integration]]): one choice question over slug-keyed criteria beats per-domain noul on both quality (0.53-0.64 vs 0.52 noisy) and latency (2s vs 6.8s). Display names as criteria keys collapse into `none`. Domain `description` fields exist because the small model needs semantic text, not acronyms.

## Two-pass (domains, then subdomains)

Pass 2 only asks subdomain questions for domains that made the cut — usually one question, keeping total latency ~2-5s. A single mega-question over all domain×subdomain pairs would scale poorly as the taxonomy grows.

## Review queue over inline disambiguation

Capture must stay frictionless (priority 1). Low-confidence entries save raw and surface in the dashboard Review tab. User corrections are valuable data for future few-shot calibration.

## Punctuation dropped, not spaced, in canonical topics

"2-sum" ≡ "2sum" dedupe matters more than readable canonical text; aliases share a canonical topic. LLM-assisted alias resolution deferred until real collision volume justifies it.

## Window routing by label, not URL params

`index.html?window=capture` in tauri.conf.json gets re-encoded by the protocol handler (webview 404s / blank). `getCurrentWindow().label` is deterministic. Debug exe caveat: it loads `devUrl` (localhost:1420) — running it without the vite server shows ERR_CONNECTION_REFUSED. Release exe embeds the frontend.

## Daemon dependency is soft

Capture never blocks on Laya. Daemon down → events stay `pending`, retried every 30s by a loop with its own DB connection. The daemon itself is a WinSW service that boots with Windows and restarts on crash (owned by the Laya-Inbox project setup).

## Classification limits accepted; correction made instant

The local `english` checkpoint reliably classifies concrete activity entries but has fixed word associations on abstract topic phrases ("dynamic programming" → Web Development) that survived 10+ prompt variations (noul, score rubrics, 29-option choices, contrast instructions, state corrections). Decision: stop prompt-tuning, raise the auto-save gate to 0.5, add one-click reclassify on every recent entry, and persist manual corrections (`source='review'`). Few-shot corrections stay in the state for future model upgrades. A stronger Laya checkpoint drop would immediately lift quality with zero app changes.

## Release builds require the tauri CLI (custom-protocol)

Plain `cargo build --release` produces an exe that still targets the dev server — the `custom-protocol` feature (which embeds `frontendDist`) is only enabled by `tauri build`. Keep the `[features]` block in Cargo.toml and always build standalone exe via `bun run tauri build --no-bundle`.

## Pitch-black bento UI; heatmap window starts at project launch

Dashboard shows only Sep 2026 (project start) → today — a fixed `PROJECT_START` constant feeds both the heatmap and the `daily_counts` range, because a GitHub-style 12-month rolling window was ~95% empty cells and invited horizontal scrolling. Scrolling is replaced with Gmail-style batch paging (`26` weeks per page, older/newer arrows, newest page by default). Theme is pitch black with zero border radius (blocky buttons/cards); color is reserved for heatmap cells, with domain colors only as taxonomy card left-borders and the daemon status dot.

## Daemon /health is GET; webview startup races app state

Two bugs that made the dashboard lie at startup: `daemon_health` POSTed to `/health`, but the daemon route is GET-only (405 → badge always "offline"), and the config-defined webview can fire its first `invoke`s before `setup()` finishes `app.manage(AppState)` → "state not managed" rejections that React never retried (zeros everywhere). Fix: health uses GET with a 5s timeout, state is managed first thing in setup, and the frontend retries any "state not managed" failure once after 300ms (plus a manual Refresh button that re-runs every load).

Related: [[Architecture]], [[Pipeline]], [[Laya-Integration]]