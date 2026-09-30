# LearnGraph — Documentation Index

Personal learning progress tracker for Windows: global hotkey capture, Laya classification, GitHub-style heatmaps per domain.

**One line:** Ctrl+` → type what you learned → Laya classifies into your taxonomy → heatmaps per domain.

## Reading order

- [[Architecture]] — components, windows, process split, tech choices
- [[Data-Model]] — SQLite schema, status flow, heatmap aggregation
- [[Pipeline]] — persist-first capture, background classify, retry loop
- [[Laya-Integration]] — daemon HTTP contract, tuned prompt recipe, thresholds
- [[Testing]] — what is covered and how to run it
- [[Decisions]] — rationale log
- [[Development-Plan]] — what is done, what comes next

## Quick facts

| Thing | Value |
|---|---|
| Stack | Tauri 2 + React 19 + TS strict + rusqlite |
| Toolchains | Rust `D:\rust`, Bun `D:\tools\bun` (user PATH) |
| DB | `%APPDATA%\com.learngraph.app\learngraph.db` |
| Laya | `http://127.0.0.1:8080` (LayaDaemon WinSW service, CPU) |
| Hotkey | `Ctrl+`` (global) |
| Idle RAM | ~33 MB |
| Release exe | `src-tauri\target\release\my-tracker.exe` (7.6 MB) |

## Dev loop

```powershell
bun run tauri dev        # full dev loop
cargo test               # in src-tauri/
bun run build            # frontend production build
bun run tauri build      # release exe + installer
```

Related: [[Working-Style]] in DevVault.