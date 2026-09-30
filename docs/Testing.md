# Testing

Run from `src-tauri/` with the Rust env on PATH (`D:\rust\cargo\bin`):

```powershell
cargo test          # 8 tests, ~7s (includes one live-daemon classification)
cargo check         # fast typecheck
bun run build       # frontend tsc strict + vite build
```

## Coverage

| Test | What it guards |
|---|---|
| `laya::tests::canonical_topic_normalizes` | normalization rules; punctuation dropped not spaced ("2-sum" ≡ "2sum") |
| `db::tests::seeds_and_lists_taxonomy` | first-run seed: 6 domains with subdomains |
| `db::tests::daily_counts_dedupes_topics_and_splits_domains` | heatmap aggregation core: distinct-topic dedupe per day, per-domain splits, overall view |
| `db::tests::pending_and_status_flow` | status transitions + pending query |
| `db::tests::taxonomy_crud_and_validation` | CRUD + `valid_classification` gate (unknown ids rejected) |
| `pipeline::tests::capture_persists_before_classification` | live end-to-end: daemon up → auto_saved/needs_review with classifications; daemon down → raw event stays `pending` |
| `pipeline::tests::capture_rejects_empty` | empty/whitespace/punctuation-only captures rejected |
| `laya::tests::classification_outcome_low_confidence_goes_to_review` | gate shape |

The pipeline test is a **smoke test against the real LayaDaemon** — it asserts behavior, not model quality. It passes in both daemon states (up: classified; down: pending), so it never flakes on daemon outages.

## Frontend

No test framework (YAGNI at this scale). `bun run build` runs `tsc --noEmit`-equivalent strict typecheck. The only non-trivial TS logic — heatmap intensity buckets and calendar geometry — is exercised by the Rust-side aggregation tests plus manual smoke:

```powershell
# Manual smoke (verified working):
# 1. start release exe  2. Ctrl+`  3. type + Enter
# 4. row appears in events + classifications (see Pipeline)
```

## Verified end-to-end (live daemon)

- "learned binary search trees" → auto_saved, DSA·Trees + WebDev (multi-domain)
- "solved CAP theorem question" → needs_review (0.30 < 0.35 gate) → Review tab
- Idle RAM ~33 MB; hotkey works from any foreground app

Related: [[Pipeline]], [[Laya-Integration]]