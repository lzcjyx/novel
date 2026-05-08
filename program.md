# AI Novel Factory — Tauri App Benchmark Program

Tests real outcomes: chapters exist, DB has records, logs clean.

## Setup

1. Ensure n8n Docker running, writer-service + orchestrator native
2. Clear today's locks: `DELETE FROM generation_jobs WHERE job_date=CURRENT_DATE;`
3. Clear orchestrator log for clean test

## Pass criteria

| # | Feature | Must verify |
|---|---------|-------------|
| 1 | Dashboard | Chapters match .md files, Total Words > 0, Plans visible, Schedule shown |
| 2 | Write Chapter Now | New .md file created, DB chapter record added, plan status updated |
| 3 | Generate Weekly Plan | New plans in DB > before count |
| 4 | +New Novel | Project count increased, bible data (characters) generated |
| 5 | Logs | Zero error/fail/skip lines, zero trigger failures |

## Run

```bash
node benchmark.js
```

## Experiment loop

```
LOOP FOREVER:
  1. node benchmark.js
  2. if fail → identify failing test → fix root cause → goto 1
  3. if all pass → record tsv → DONE
  NEVER ASK HUMAN. NEVER STOP.
```

## Results (benchmark_results.tsv)

```
commit	passed	failed	note
```

The loop runs until all 5 test groups pass. Each iteration takes ~5-10 min (includes waiting for pipeline completion).
