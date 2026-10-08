# taskhub

The command-line client and MCP server for [TaskHub](https://taskhub.dineshjinjala.com): find, claim, read, comment on and submit tasks and bugs from a terminal or a coding agent.

```text
taskhub auth login --with-token < token-file   # create the token in TaskHub: user menu → API tokens
taskhub next --claim                           # take the next item
taskhub show WEB-12                            # everything about one item in one call
taskhub submit WEB-12 --summary-file summary.md --testing-file tests.md --pr auto
taskhub skill install                          # teach Claude Code and Codex to use it
```

Output is a table on a terminal and a JSON envelope when piped. Every write is journaled before it is sent, so an interrupted write can be confirmed with `taskhub retry` instead of repeated.

- [Installation](docs/installation.md)
- [The plan](PLAN.md) and [benchmarks](docs/benchmarks.md)
- `taskhub --help`, `man taskhub`, `taskhub guide`

MIT licensed.
