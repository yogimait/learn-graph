# Laya Integration

Laya is the classification layer — a small local decision model (choice / score / noul, no free-text generation) served by the always-on **LayaDaemon** (WinSW service from the Laya-Inbox project): `python -m laya.serve` on CPU, `http://127.0.0.1:8080`.

The app **never loads a model, never spawns Python, never touches the venv** (`D:\Projects\personalAI\venv`). It only POSTs JSON.

## HTTP contract

- `GET /health` → `{"status":"ok","loaded":["english"],"device":"cpu"}`
- `POST /v1/systemone` with `{state, questions, model:"auto"}` → `{answers: {name: {...}}, routing}`

Answer shapes:
- `choice` → `{choice: "<criteria-key>", probabilities: {key: p}, confidence}`
- `noul` → `{noul: p}`

## Tuned recipe (calibrated by live experiments — do not casually change)

| Variable | Use | Measured impact ("solved Two Sum with a hash map") |
|---|---|---|
| Criteria keys | **lowercase slugs** of domain names (`dsa`, `webdev`, `systemdesign`) | display names (`"DSA"`, `"System Design"`, `"AI/ML"`) collapsed into `none` (0.45+) |
| Criteria text | user's semantic **description** | bare name + subdomain list ranked WebDev (0.38) over DSA (0.08) |
| State | `{entry}` **only** | an extra `note` field degraded results |
| Question shape | ONE choice over all domains | per-domain noul was noisy (Web 0.79 for Two Sum) and slower (6.8s vs ~2s) |

Latency: ~1.6-3s per pass warm (scales with question count). Two passes ≈ 2-5s total. Cold start ~12s.

## Decision gates (deterministic, in laya.rs)

```
winner = pass1.choice                    // slug
winner_p = probabilities[winner]
secondary d: p(d) >= multi_floor AND p(d) >= winner_p * multi_ratio
status   = winner_p >= review_threshold ? auto_saved : needs_review
subdomain picked only if its p >= subdomain_threshold
```

Defaults (settings table, editable in the Settings tab): `review_threshold 0.5` (raised in migration v2 after quality issues), `multi_ratio 0.5`, `multi_floor 0.2`, `subdomain_threshold 0.3`.

The validator clamps everything to real taxonomy ids — Laya can never invent a domain.

## Known quality limits (measured, not assumed)

The loaded `english` checkpoint has hard word associations that no prompt shape overcame (10+ live experiments):

| Entry | Model verdict | Truth |
|---|---|---|
| "solved Two Sum with a hash map" | dsa 0.53-0.64 | correct |
| "learned binary search trees" | webdev 0.39 / dsa 0.24 | wrong primary |
| "learned dynamic programming" | webdev 0.47-0.62 / dsa 0.08-0.25 | wrong |
| "solved CAP theorem question" | webdev 0.30 / sysdesign 0.25 | ambiguous |

Dead ends verified: per-domain noul (noisy), score-rubric relevance (no discrimination, all ~0.96), 29-option subdomain choice (degenerate), explicit contrast instructions (ignored), corrections field in state (ignored).

## Product mitigation (the design answer)

- **High gate:** winner must clear `review_threshold 0.5` to auto-save; everything else lands in the review queue.
- **Instant correction:** every Recent-entries row has a `reclassify` action (moves the event to the review queue); manual classifications are persisted with `source = 'review'` and feed future few-shots.
- **Prompt recipe stays tuned** for what the model does well (concrete activity entries) and the taxonomy stays user-owned.

## Race safety

The sync capture path and the 30s retry loop both classify via `claim_event` (atomic `UPDATE ... WHERE status='pending'`) — exactly one worker wins; the loser reverts or skips. Migration v2 deduped the duplicate rows the pre-claim race produced.

## Future: learning from corrections

Manual review classifications (probability 1.0) are ideal few-shot examples. Plan: include the N most recent user-corrected `entry → domain` pairs in the pass-1 state so the model calibrates to the user's taxonomy. Not implemented yet — see [[Development-Plan]].

Related: [[Pipeline]], [[Decisions]]