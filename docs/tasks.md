# CLI implementation tasks

The work breakdown for [PLAN.md](../PLAN.md), following the [coding guidelines](coding-guidelines.md). Tasks run in order. Each one ends with one focused commit on `implementation` and a report. It counts as done only after the tester has checked it.

## Roles and places

| Who | Works in | Talks to |
| --- | --- | --- |
| TaskhubImplementor | `~/Projects/taskhub-cli` (branch `implementation`) | mock servers in tests; the TaskHub test server for manual checks |
| TaskhubTestComputerAgent | a detached checkout of the commit under test | the TaskHub test server `http://127.0.0.1:3100` (`~/Projects/TaskHub-test`, `~/.taskhub/test.db`), plus the browser to confirm results |

- **TaskHub's API is live** in production since 2026-10-08 (commit `3f41586`). The contract source is `~/Projects/TaskHub-agent-api/docs/api/v1/`.
- **Reports** go to `~/.taskhub/reports/`: `C<n>-impl.md` and `C<n>-test.md`, in the same formats as the TaskHub tasks.
- **Production is off limits** for every task. Use only local servers and the test server.

## Tasks

### C0 — Crate and contract

The plan's milestone 0, plus the crate skeleton:

- `Cargo.toml` with the chosen dependencies and their versions, `Cargo.lock`, `rustfmt.toml`, `#![forbid(unsafe_code)]`, and an empty command tree that builds.
- `scripts/sync-contract.sh <path-to-TaskHub>`, `api/v1/` with `contract-source.json`, and a drift test.
- Hand-written request and response types.
- Fixture round-trip tests, including the Unicode (UTF-16) and three-state patch cases.

**Test:** run the sync against the TaskHub checkout and confirm there is no drift. Hand-edit one snapshot file and confirm the drift test fails. Run the full definition of done.

### C1 — Core and reads

The plan's milestone 1:

- config, credentials and `auth login | status | logout` (stdin, env, token file, credentials file; origin rules; `0600` checks)
- transport and its limits, the envelope, hints and exit codes
- branch inference
- `show`, `mine`, `items list | get | activity`, `comments list`, `attachments list | download`, `projects`, `inbox`, completions

**Test:**
- Create a token at `http://127.0.0.1:3100/settings/tokens` in the browser, and run `auth login --with-token` from a `0600` file.
- Run every read command against the test server, at a terminal and piped. Compare with what the browser shows.
- The inbox stays unread.
- A revoked token gives exit 3 with a hint.
- An ungranted project gives exit 5.
- `http://localhost` is refused.
- The token never appears in any output or file other than the credentials file.

### C2 — Writes and the work loop

The plan's milestone 2:

- the journal, `pending` and `retry`
- `next`, `claim`, `comment`, `comments edit`, `attachments add`, `link`, `submit`, `reject`, `inbox done`
- `items create | update | move`
- `branch`, `--pr auto`

**Test:**
- Drive the loop against the test server, and check each result in the browser: claim, a comment with an @mention, an upload, submit with a PR link, reject with a Tester token.
- An attempt to move to Done gives exit 4.
- Kill the CLI after a submission is sent; `pending` lists it, and `retry` completes it once with no duplicate.
- Two `next --claim` calls racing each other.
- A stale `--from` gives exit 6.
- An item deleted to Trash mid-work gives exit 5.

### C3 — Agent surface

The plan's milestone 3: `guide`, the embedded skill and `skill install` (Claude Code and Codex, personal and project scopes), `mcp` (every tool in the plan, over stdio), and `open`.

**Test:**
- `skill install` into temporary `HOME` directories, and its refusal to overwrite a different skill.
- The skill stays within its size budget.
- An MCP stdio client lists the tools, and runs `show` and `comment` with the same envelope and journal as the CLI.

### C4 — End to end with real agents

The plan's milestone 4, against the TaskHub test server, not staging:

1. Run the plan's scripted workflow with a Developer token and a Tester token. Include the kill and `retry`, a token revoked mid-run, and racing claims.
2. With the skill installed, run a real Claude Code session and a real Codex session on a seeded item. Each should claim it, comment, and submit with evidence. Then a person signs off in the browser.
3. Record the measurements in `docs/benchmarks.md`: startup time, p50 and p95 for `show`, `next --claim` and `submit` (local server), tool and CLI call counts, and skill bytes loaded.

**Test:** every write is correct, with no duplicates. Everything the agents did shows as "username (agent)" in the browser. Sign-off stayed human.

### C5 — Distribution, prepared

The plan's milestone 5, up to the point where publishing needs the owner:

- `dist` configuration for the four targets, plus a release workflow with actions pinned to commit hashes.
- The man page and completions in the release archive.
- A Homebrew formula template, and an AUR `PKGBUILD` for `taskhub-cli-bin`.
- `docs/installation.md`.
- A local release build for `x86_64-unknown-linux-gnu`, with a smoke test of the installed binary.

Don't create the GitHub repository, tags, tap or AUR package, and don't publish anything. The owner decides the names and the account first.

**Test:** install the local build into a clean `HOME` and run the C1 read checks with it. The archive contains the binary, the man page, completions and the license.
