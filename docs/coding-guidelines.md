# Coding guidelines

How to write taskhub-cli so it stays easy to change and hard to get wrong. [PLAN.md](../PLAN.md) says what to build; this says how.

## Shape

- One binary crate, `taskhub-cli`, executable `taskhub`, edition 2024, laid out as in the plan's "Rust project layout".
- `#![forbid(unsafe_code)]` at the crate root.
  - The only exception is the no-replace rename for downloads (`renameat2` / `renamex_np`). Keep it in one small module with `#[allow(unsafe_code)]` and a safety comment, or use a crate that wraps it.
- Commands are thin. Each command module turns parsed arguments into calls on `api`, `journal`, `git` and `credentials`, then returns a typed result. It never prints, exits or builds HTTP requests itself.
- **One road for each concern:**
  - All HTTP goes through `api.rs`.
  - All output goes through `output.rs`: the envelope, human tables and hints.
  - All exits go through `main.rs`.
  - All writes go through `journal.rs`.
  - A second way to do any of these is a bug.

## Errors and exit codes

- One `CliError` enum with `thiserror`. Every variant carries:
  - its stable `code` (the API's codes plus the CLI's own: `REF_REQUIRED`, `PR_NOT_FOUND`, `OPERATION_OWNER_MISMATCH`, `UPLOAD_CHANGED`, `REPLAY_WINDOW_EXPIRED`, `OUTCOME_UNKNOWN`, `PROTOCOL_ERROR`, …)
  - its exit code
  - its hint
- The exit-code mapping is a single `match` in one place, tested against the plan's table.
- No `unwrap`, `expect` or `panic!` outside tests. In `main` alone, a panic hook turns a bug into exit 1 with a JSON error when stdout is not a terminal.
- `anyhow` is not used. Errors stay typed all the way to the envelope.

## Secrets

- Tokens live only inside a `Secret` newtype that has no `Display`, and whose `Debug` prints `Secret(…)`. The only way to read the value is a method named `expose_for_header`, used once in `api.rs`.
- Mark the `Authorization` header as sensitive.
- Never put a token in arguments, URLs, environment output, logs, the journal, error messages or test snapshots. A test greps all captured output and journal files for the synthetic test token.
- Write files containing credentials or journal entries with mode `0600`, inside directories created with `0700`. Check permissions on read, and refuse when they are wrong.

## Types and the contract

- Request types use `#[serde(deny_unknown_fields)]`. Response types tolerate added fields.
- Enums from the server have an `Unknown(String)` fallback that is displayed but never sent back.
- Three-state patch fields (missing / null / value) use one explicit `Patch<T>` type, never `Option<Option<T>>` by convention alone.
- Field limits count UTF-16 code units, shared with TaskHub through the Unicode fixtures.
- **The contract snapshot is generated, not edited:**
  - `api/v1/` comes only from `scripts/sync-contract.sh`.
  - Every fixture round-trips in a test.

## Journal and retries

- Write the journal entry and `fsync` it (file and directory) before the request leaves the process.
- Only `journal.rs` decides whether a write may be resent. The rule is identical bytes, an identical `Idempotency-Key`, and at most one automatic resend.
- reqwest's own retries and redirects are off.
- Multi-step writes (uploads, then the submission) are a parent entry with child steps. A retry resumes from the first unfinished step.

## Output

- When stdout is not a terminal: the JSON envelope only, with no color, no prompts and no progress.
- On a terminal: human tables, color only when `NO_COLOR` is unset.
- Hints are data in the envelope (`error.hint`). Never parse or match on message text, in code or in tests.
- Write the envelope, including errors, to stdout. Send stderr only diagnostics a person reads.

## Style

- Format with `rustfmt` and `max_width = 110` in `rustfmt.toml`.
- Lint with `cargo clippy --all-targets -- -D warnings`. Allow a lint only locally, with a reason.
- Name things after the domain: `ItemRef`, `OperationId`, `Origin`, `Version`, `Status`. Use newtypes where a mix-up is possible (an item key vs a comment ID, a version vs a count).
- Comments explain why, not what. Keep functions short, and keep modules to one concern.
- Choose dependencies deliberately. Pin them through `Cargo.lock` (committed), prefer `rustls`, and add nothing for convenience alone.

## Tests

- Unit tests sit next to the code. Process-level tests in `tests/` run the real binary (`assert_cmd`) against a local mock HTTP server that the test controls.
- **Required coverage:** every row of the plan's "Required validation" table, plus the journal crash cases (kill after send, before the receipt) through a real child process.
- **Test environment:**
  - Tests never touch the network beyond `127.0.0.1`.
  - They use temporary `HOME` and `XDG_*` directories and synthetic tokens.
  - Temporary files live under `$TMPDIR`; set it to a directory in `$HOME` on this machine, because `/tmp` has a tight per-user quota.

## Definition of done

Each task ends with all of these passing:

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
```

Then make one focused commit on `implementation`, and write a report to `~/.taskhub/reports/C<n>-impl.md` in the same format as the TaskHub reports.

## Where to work

| | Path |
| --- | --- |
| Repository | the Linux worktree `~/Projects/taskhub-cli`, branch `implementation`; the shared-drive checkout is for the Mac |
| Build output | the default `target/` (btrfs) |
| TaskHub | `~/Projects/TaskHub-agent-api` (branch `agent-api`): source of the contract |
| TaskHub test server | `~/Projects/TaskHub-test` on `http://127.0.0.1:3100`: end-to-end runs |

Never touch ports 3000 or 3200, `~/Projects/TaskHub-preview`, or any production service.
