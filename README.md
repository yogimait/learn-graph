# LearnGraph

A lightweight Windows tray app for tracking what you learn: press **Ctrl+`** anywhere, type one line about what you just learned or practiced, and a local AI classifies it into your personal skill taxonomy. Your learning history builds up as GitHub-style heatmaps.

No cloud. No accounts. Everything stays on your machine.

## How it works

1. **Capture** — press **Ctrl+`** anywhere, type what you learned, hit Enter. The overlay closes instantly; classification happens in the background.
2. **Classify** — a local daemon ([Laya](docs/Laya-Integration.md)) scores the entry against your own domains (DSA, System Design, Web Development, ...) and subdomains (Trees, Graphs, DP, ...). High-confidence entries are auto-saved; low-confidence ones land in a review queue for one-click confirmation.
3. **Visualize** — every entry feeds GitHub-style contribution heatmaps: one overall view plus one per domain, with intensity based on distinct topics practiced per day.

## Features

- Global hotkey capture from anywhere (**Ctrl+`**; configurable value, fixed binding in this version)
- Local, two-pass AI classification: domain first, then subdomain — with multi-domain tagging
- Review queue for uncertain entries; manual corrections are persisted and reused as classification few-shots
- GitHub-style heatmaps (overall + per domain), paged in batches instead of endless scrolling
- Fully editable taxonomy: domains, subdomains, and the descriptions the classifier reads
- Dashboard with stats tiles (total entries, this week, needs review, daemon status)
- Resilient by design: the daemon being offline never blocks capture — entries queue as `pending` and are retried every 30 seconds
- SQLite storage (WAL) with versioned migrations; the raw text of every entry is written before anything else touches it

## Tech stack

| Layer | Choice |
|---|---|
| Shell | [Tauri 2](https://v2.tauri.app) (Rust, system WebView2) |
| Frontend | React 19 + TypeScript + Vite, plain CSS |
| Storage | SQLite via `rusqlite` |
| Classification | Local HTTP daemon (`http://127.0.0.1:8080`), no API keys |
| Idle footprint | ~30 MB RAM |

## Getting started

Prerequisites: [Rust](https://rustup.rs) (stable, MSVC), [Bun](https://bun.sh) 1.4+, and the Tauri 2 [Windows prerequisites](https://v2.tauri.app/start/prerequisites/) (WebView2 + VS Build Tools).

```bash
bun install          # install JS deps

bun run tauri dev    # full dev loop (vite + debug binary)
bun run build        # typecheck + production frontend build
cargo test           # run in src-tauri — 8 unit tests incl. a live-daemon smoke test

# standalone release exe
cd src-tauri
cargo build --release --features custom-protocol
# -> src-tauri/target/release/my-tracker.exe
```

The classifier daemon is expected at `http://127.0.0.1:8080` (`/health`, `/v1/systemone`). Without it, the app still captures and stores everything — entries wait as `pending` until the daemon is reachable. See [docs/Laya-Integration.md](docs/Laya-Integration.md) for the prompt contract and thresholds.

Data lives in `%APPDATA%\com.learngraph.app\learngraph.db`. It is never bundled with the repo.

## Project structure

```
src/                    React frontend
  capture/              Ctrl+` overlay window
  main/                 Dashboard: heatmaps, review, taxonomy, settings
  components/           SVG heatmap (GitHub-style calendar)
  lib/                  Tauri API bindings + types
src-tauri/              Rust backend
  src/db.rs             SQLite layer, migrations, seeds
  src/laya.rs           Daemon client: prompts, gates, validators
  src/pipeline.rs       Capture/classify/review flow, retry loop
  src/commands.rs       Tauri command surface
docs/                   Project docs (Obsidian vault): architecture,
                        data model, pipeline, testing, decisions
```

## Status

V1 is complete and in daily use. Planned next: few-shot calibration from user corrections, subdomain breakdown bars, hotkey re-registration from Settings. See [docs/Development-Plan.md](docs/Development-Plan.md).