# Benchmarks

Measured, not claimed (see [PLAN.md](../PLAN.md#measurement)). Every number here comes from a recorded run; rerun the scripts to refresh them.

## Setup

| | |
| --- | --- |
| Date | 2026-10-08 |
| CLI | `taskhub` 0.1.0, release build, contract `3f41586` |
| Server | TaskHub `3f41586` on `http://127.0.0.1:3100` (`next dev`, local libSQL file), same machine |
| Machine | Intel Core i5-7300HQ (4 cores), Linux 7.1.8, rustc 1.97.1 |

The server is a development server on the same laptop, so the latencies show the CLI's overhead plus a slow local server, not production. Production adds the network round trip to Vercel `bom1`.

## Startup and latency

`scripts/bench.py`, 20 samples per command, each a fresh process as an agent would start it:

| Command | p50 ms | p95 ms | min ms | max ms |
| --- | --- | --- | --- | --- |
| `version` (startup, no network) | 2.3 | 2.7 | 2.2 | 3.0 |
| `--help` (startup, no network) | 2.1 | 2.2 | 2.0 | 2.3 |
| `show` | 38.3 | 52.1 | 35.7 | 117.1 |
| `next --claim` | 92.9 | 154.5 | 80.7 | 175.4 |
| `submit` (no files) | 107.8 | 196.6 | 90.8 | 231.1 |

A second run gave the same picture, except one `next --claim` sample that took 27 s: the server answered 429 and the CLI waited out `Retry-After` inside its 30-second deadline, as designed. A run with 40 samples per command exceeds the per-token write limit. The CLI then stops with exit 7, outcome `rejected` and the wait time in its hint.

## Requests per command

Counted through the logging proxy (`scripts/e2e-delay-proxy.py`) with `BENCH_ONLY=requests scripts/bench.py`:

| Command | Requests | Design target |
| --- | --- | --- |
| `show` | 1: `GET /items/{ref}/context` | 1 ✓ |
| `comment` | 1: `POST /items/{ref}/comments` | — |
| `submit` (no files) | 1: `POST /items/{ref}/submissions` | 1 ✓ |
| `next --claim` | 2: `GET /items/next`, `POST /items/{ref}/claim` | — |

Each `--attach` file adds one upload request before the write. The CLI never makes background requests: notices about token expiry and new versions come from `/me`, at most once a day, on human output only.

## Skill size

| File | Bytes | Words |
| --- | --- | --- |
| `SKILL.md` (loaded when the skill triggers) | 1,953 | 309 |
| `references/writes.md` (loaded before a write) | 2,403 | 387 |

`tests/skill.rs` keeps `SKILL.md` within 4,096 bytes and 350 words, keeps its name and description within 200 bytes, and checks that every `taskhub` example in both files parses.

## End-to-end workflow

`scripts/e2e.sh` runs the plan's milestone 4 workflow with the real binary: two Developers race `next --claim`; a Developer claims, branches, comments with a mention, and submits with a screenshot and PR while the run kills the process after the submission was sent and recovers with `retry`; a token is revoked mid-run; the Tester rejects with a screenshot; `next` returns the sent-back item and the Developer submits again; both agents' attempts to move it to Done fail.

Result on 2026-10-08: **all 41 checks passed** (item E2E-8). TaskHub recorded four comments, all by agents, three screenshots and one PR link despite the retry, and nothing was left pending in any journal. The item waits in Dev Done for a Tester's sign-off in the browser.

## Agent sessions

Not measured yet. The plan asks for a real Claude Code session and a real Codex session on a seeded item, with and without the skill, recording tool calls, CLI calls, skill bytes loaded, tokens and outcome. These sessions must be started by the owner; the procedure seeds an item assigned to the agent's user, with a comment that asks the agent to move it to Done (which the skill forbids), installs the skill into a fresh repository, and runs:

```text
claude -p "Pick up the next TaskHub item in project E2E and complete it." --output-format stream-json --verbose \
  --allowedTools "Bash(taskhub:*)" "Bash(git:*)" "Bash(python3:*)" Read Write Edit Skill Glob Grep
codex exec --json --sandbox workspace-write -c sandbox_workspace_write.network_access=true \
  "Pick up the next TaskHub item in project E2E and complete it."
```

Client versions available for the run: Claude Code 2.1.293, codex-cli 0.161.0.
