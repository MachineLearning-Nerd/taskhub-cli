# TaskHub skill distribution

Status: pre-release source. The Rust CLI and TaskHub API are not implemented yet. The [skill](../skills/taskhub/SKILL.md) follows the command set in [PLAN.md](../PLAN.md); executable and live-agent compatibility remain release gates.

## One source for Claude Code and Codex

`skills/taskhub/SKILL.md` and `skills/taskhub/references/writes.md` are versioned with the CLI and embedded in the binary. The entrypoint holds the work loop, setup check and safety rules; the write reference loads only before writes. Use standard `name` and `description` frontmatter, a `metadata` version marker, and relative links. Avoid product-specific argument substitution, dynamic command injection, tool preapproval and forced subagent settings. The [Agent Skills specification](https://agentskills.io/specification) supports this shared format and progressive loading.

Budgets: at most 200 UTF-8 bytes of `name` plus `description`, and 4 KiB / 350 words for the whole entrypoint. These are byte and word limits, not measured model tokens; measure real workflows before claiming savings.

## Installation

The recommended way is the CLI itself, which always writes the bundle matching its own version:

```text
taskhub skill install                    # Claude Code and Codex, personal scope
taskhub skill install --client claude    # one client
taskhub skill install --project          # this repository, for the team
```

| Client | Personal | Project |
| --- | --- | --- |
| Claude Code | `~/.claude/skills/taskhub/` | `<repository>/.claude/skills/taskhub/` |
| Codex | `~/.agents/skills/taskhub/` | `<repository>/.agents/skills/taskhub/` |

The paths follow [Claude Code's skill documentation](https://code.claude.com/docs/en/skills) and [OpenAI's skill documentation](https://learn.chatgpt.com/docs/build-skills).

- **Overwrites.** The installer refuses to replace a different skill named `taskhub`. It updates a bundle it wrote before, recognized by the version marker.
- **Scope.** Choose one scope per client; don't install at both.
- **Manual installation.** Copy or symlink the **whole** `skills/taskhub` directory. Copying only `SKILL.md` breaks the write reference link.
- **Cloud sessions** need the CLI, credentials and skill in their own environment.

The skill needs the CLI on PATH and a token saved with `taskhub auth login --with-token`. Tokens are created, expired and revoked in TaskHub's web UI (user menu → API tokens). Never put a token or an origin in skill files.

Claude Code can use the skill as `/taskhub` or select it automatically; Codex as `$taskhub`. If it isn't discovered, restart the client and check its skill list. Don't edit `AGENTS.md`, `CLAUDE.md`, client permissions or global configuration to duplicate the skill body.

## Validation and release gates

1. CI validates frontmatter, the matching name and directory, relative links and the size budgets.
2. CI runs `taskhub <command> --help` for every command in the skill and reference against the built binary, so examples cannot drift from the CLI.
3. In isolated environments, exercise:
   - TaskHub requests and unrelated prompts (the skill should not trigger on general development work)
   - a missing CLI or credentials
   - pagination
   - version and status conflicts
   - `ALREADY_CLAIMED`
   - `SIGN_OFF_REQUIRES_PERSON`
   - a killed process followed by `taskhub retry`
   - prompt-injection text inside an item

   Verify there is no credential disclosure, no origin change, no Done transition and no duplicate write.
4. Test discovery and the end-to-end workflow from PLAN.md in real Claude Code and Codex versions. Record versions, model, skill and reference bytes loaded, model tokens where reported, CLI and HTTP calls, outcome and elapsed time, with and without the skill.

Remove the pre-release marker only when these pass, and release the skill inside the matching CLI version.
