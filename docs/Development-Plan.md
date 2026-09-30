# Development Plan

Status of LearnGraph build. Done items reflect what IS implemented; this file is the only place planned work lives.

## Done (V1 core)

- Rust toolchain on D: (rustup stable-msvc 1.98.1), Bun 1.4.2 on D:, PATH persisted
- Tauri 2 + React 19 + TS scaffold, release exe via `tauri build` (custom-protocol embedded), idle ~33 MB
- SQLite layer: migrations v1/v2 (v2: classification dedupe + gate raise), seed taxonomy (6 domains), taxonomy CRUD, events, classifications, settings, `daily_counts` aggregation
- Global hotkey `Ctrl+``, capture overlay (frameless, always-on-top), tray, dashboard window
- Laya two-pass classification with claim-based race safety, deterministic validator, multi-domain split, review_threshold 0.5
- Persist-first + 30s retry loop with its own DB connection; review queue + one-click reclassify from Recent entries; corrections persisted as few-shot source
- Heatmaps: overall + per-domain SVG calendar, 5 buckets, GitHub-style dark empty cells, window Sep 2026 (project start) → today with Gmail-style batch paging (26 weeks/page, no scroll), domain chips, recent entries with reclassify
- UI theme: pitch-black bento layout (stat tiles + heatmap + recent grid), blocky zero-radius controls, color reserved for heatmap cells
- Settings UI: daemon URL, thresholds, start-at-login toggle
- Startup correctness: daemon health via GET (daemon route is GET-only), state managed before webview invokes race it, "state not managed" auto-retry + header Refresh button
- 8 Rust tests incl. live-daemon pipeline smoke; tsc strict clean

## Next (in order)

1. **Few-shot calibration** — feed recent user-corrected classifications into pass-1 state (see [[Laya-Integration]])
2. **LLM-assisted alias dedupe** — "2sum" vs "Two Sum" beyond deterministic normalization, when collision volume justifies it
3. **Subdomain bars** per domain page (concept doc section 3)
4. **Hotkey re-registration from Settings** — currently fixed at `Ctrl+``
5. **Windows installer** — `bun run tauri build` NSIS bundle + autostart verified end-to-end
6. **Custom app icon** (currently scaffold default)

## Non-goals (from the concept doc)

Mastery levels, streaks/gamification, cloud sync, mobile, search, AI mentor dashboard, context-aware source capture.