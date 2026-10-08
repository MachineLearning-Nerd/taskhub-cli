# Writes and recovery

The CLI records every write in a local journal before sending it, so you never need to create or remember request IDs. A committed receipt confirms the write; fetch again only for information the receipt lacks.

```text
taskhub comment WEB-12 --body-file findings.md
taskhub submit WEB-12 --summary-file summary.md --testing-file tests.md --attach shot.png --pr auto
taskhub reject WEB-12 --reason-file reason.md --attach repro.png
taskhub items move WEB-12 in_progress --from todo
taskhub items update WEB-12 --if-version 7 --priority high
```

- `submit` moves the item to Dev Done with an evidence comment in one step, and notifies the Testers. Put real evidence in the testing file. Name tests you did not run under limitations.
- Status changes other than `reject` need `--from`: the status you saw in `show`. `reject` works only from Dev Done. Field edits need `--if-version`: the version you saw.
- Mention people with `@username`. Check `unmatchedMentions` in the receipt for names that notified no one.
- `--attach` accepts .png, .jpg/.jpeg, .gif, .webp, .pdf, .md and .docx files up to 4 MB; the extension must match the content.

## Outcomes

| Result | What to do |
| --- | --- |
| `ok:true`, `outcome:"committed"` | Done. Report the key and what changed. A `replayed:true` receipt describes the original write. |
| Exit 8, `outcome:"unknown"` | Run `taskhub retry OP` with the operation ID from `meta.operation.id`, once. If it is still unknown, stop and report the ID. Never repeat the original command. |
| `VERSION_CONFLICT` or `STATUS_CONFLICT` | Someone changed the item. Run `taskhub show`, and redo the write only if it still matches what the user asked for. |
| `ALREADY_CLAIMED` | Another agent took it. Run `taskhub next --claim`. |
| `SIGN_OFF_REQUIRES_PERSON` | Stop. Tell the user a Tester signs off in TaskHub. |
| `NOT_FOUND` on an item you were working on | It may have been deleted. Stop and tell the user; don't recreate it. |
| `REPLAY_WINDOW_EXPIRED` | Retry protection has ended. Use `taskhub show` to check whether the write happened before doing anything else. |
| `IDEMPOTENCY_CONFLICT` | Investigate; don't work around it. |
| Authentication, permission or validation errors | Fix the cause the hint names; don't retry unchanged. |

`taskhub pending` lists writes whose outcome is unknown. Leave them for the user if you cannot resolve them.
