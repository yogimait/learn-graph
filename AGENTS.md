# AGENTS.md — LearnGraph (my-tracker) Project Rules

Follow these rules throughout the project. They apply to every coding agent working in this repo, including opencode.

---

## 1. Project in One Line

LearnGraph is a lightweight Windows tray app: **Ctrl+` → type what you learned → Enter → Laya (local daemon) classifies it into your own taxonomy → GitHub-style heatmaps per domain.**

Heatmaps are the primary feature; the dashboard is where you go later to understand yourself. Capture takes seconds.

---

## 2. Golden Engineering Rules (non-negotiable)

1. **Never lose the original user input.** The raw text is written to SQLite (`status = pending`) before any classification starts.
2. **Laya never invents the taxonomy.** Domain/subdomain ids are validated against the `domains`/`subdomains` tables; unknown ids are rejected deterministically.
3. **The DB lock is never held during HTTP calls.** Snapshot a `ClassifyCtx` under lock, call the daemon lock-free, write results under a fresh lock.
4. **SQLite is the source of truth.** `%APPDATA%\com.learngraph.app\learngraph.db`, migrations via `PRAGMA user_version`. Never ship a DB in the repo.
5. **Local-first.** The LayaDaemon (`http://127.0.0.1:8080`, WinSW service from Laya-Inbox) may be down; events then stay `pending` and a background loop retries every 30s. Capture always works.
6. **Laya is an installed runtime, not a dependency.** The daemon + venv live in `D:\Projects\personalAI\venv`. Never reinstall, move, or reconfigure it. This app only POSTs to `/v1/systemone`.
7. **Toolchains live on D:.** `RUSTUP_HOME=D:\rust\rustup`, `CARGO_HOME=D:\rust\cargo`, `BUN_INSTALL=D:\tools\bun`. The C drive has ~4GB free — never install toolchains or caches there.
8. **Secrets live nowhere in code.** No API keys needed: classification is fully local.
9. **KISS / YAGNI / ponytail.** V1 non-goals: mastery levels, streaks, cloud sync, search, AI mentor, context-aware source capture.

---

## 3. Laya Classification Contract (tuned against the live daemon)

Do not casually change the prompt shape — it was calibrated by live experiments (see `docs/Laya-Integration.md`):

- **Pass 1:** ONE choice question, criteria keys = lowercase slugs of domain names, criteria text = the domain's user-editable `description`. State = `{entry}` only. Instruction: "Which single domain does this learning entry primarily belong to? ..."
- **Pass 2:** one call, one choice question per included domain over subdomain slugs + `none`.
- **Multi-domain:** secondary domain included if `p >= multi_floor` AND `p >= winner_p * multi_ratio`.
- **Gates:** winner `p < review_threshold` (default 0.5) → `needs_review`; subdomain `p < subdomain_threshold` → no subdomain.
- Multi-word display names as criteria keys measurably break the small model (collapses into `none`). Keep slugs.
- The `english` checkpoint has hard word associations ("dynamic programming" → webdev, immune to prompt fixes). The mitigation is the high gate + one-click reclassify — do not chase prompt-only fixes without new live evidence.

---

## 4. Living Documentation (Obsidian vault)

- `docs/` is the project vault, opened in Obsidian. `docs/Home.md` is the index; every doc is wiki-linked.
- Document meaningful changes in the same task that makes them. Docs describe what IS implemented; plans live in `docs/Development-Plan.md`.
- Keep `[[Architecture]]`, `[[Data-Model]]`, `[[Laya-Integration]]`, `[[Pipeline]]`, `[[Testing]]`, `[[Decisions]]` in sync with code. Drift between code and docs is a defect.
- DevVault (`C:\Users\Hp\Obsidian\DevVault`) indexes this project as `DevVault/Projects/LearnGraph.md`.

---

## 5. Code Conventions

- Rust (src-tauri): modules by concern — `db.rs`, `laya.rs`, `pipeline.rs`, `commands.rs`, `models.rs`, `lib.rs`. No logic in `lib.rs` beyond wiring.
- TypeScript (src): strict; `lib/` (api, types), `components/`, `capture/`, `main/`. Plain CSS in `styles.css` — no UI framework.
- No comments unless marking a deliberate simplification (`ponytail:`) or a non-obvious constraint.
- Window routing is by Tauri window **label** (`getCurrentWindow().label`), never URL params — the Tauri protocol handler re-encodes query strings.

---

## 6. Commands

| Command | What it does |
|---|---|
| `bun install` | install JS deps |
| `bun run dev` | vite dev server only (frontend) |
| `bun run tauri dev` | full dev loop (vite + debug binary; needs dev server) |
| `bun run build` | tsc + vite production build |
| `bun run tauri build` | release exe + installer |
| `cargo test` (in src-tauri) | 8 unit tests incl. live-daemon pipeline smoke |
| `cargo check` | fast typecheck |

**Run the app standalone:** `src-tauri\target\release\my-tracker.exe`. The DEBUG exe loads `http://localhost:1420` and shows ERR_CONNECTION_REFUSED unless `bun run dev` is active.

Daemon: `curl http://127.0.0.1:8080/health` (service `LayaDaemon`, from the Laya-Inbox project).

---

## 7. Before Finishing Any Task

1. `cargo check` clean, no warnings.
2. `cargo test` green.
3. `bun run build` (tsc strict) clean.
4. Docs updated if the change is meaningful.
5. No secrets, no new dependencies without checking the ponytail ladder.