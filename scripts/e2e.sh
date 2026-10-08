#!/usr/bin/env bash
# The plan's end-to-end workflow (milestone 4), run with the real binary against a TaskHub server.
#
# Never point this at production: it creates, claims, comments on and submits items.
#
# Needs bash, jq, git and python3, plus token files for a project that holds no To Do items:
#   E2E_DEV_TOKEN      a Developer's write token
#   E2E_DEV2_TOKEN     a second Developer's write token (the claim race)
#   E2E_TESTER_TOKEN   a Tester's write token
#   E2E_REVOKE_TOKEN   another write token, and E2E_REVOKE_CMD, a command that revokes it
# Optional: TASKHUB_BIN (default target/release/taskhub), E2E_ORIGIN (default http://127.0.0.1:3100),
# E2E_PROJECT (default E2E), E2E_TESTER (the Tester's username, default neha).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
bin=${TASKHUB_BIN:-$here/../target/release/taskhub}
bin=$(cd "$(dirname "$bin")" && pwd)/$(basename "$bin")
origin=${E2E_ORIGIN:-http://127.0.0.1:3100}
project=${E2E_PROJECT:-E2E}
tester_name=${E2E_TESTER:-neha}
for var in E2E_DEV_TOKEN E2E_DEV2_TOKEN E2E_TESTER_TOKEN E2E_REVOKE_TOKEN E2E_REVOKE_CMD; do
  [[ -n ${!var:-} ]] || { echo "e2e: $var is not set" >&2; exit 2; }
done
case $origin in
  *taskhub.dineshjinjala.com*) echo "e2e: refusing to run against production" >&2; exit 2 ;;
esac

work=$(mktemp -d "${TMPDIR:-/tmp}/taskhub-e2e.XXXXXX")
proxy_pid=
cleanup() {
  [[ -n $proxy_pid ]] && kill "$proxy_pid" 2>/dev/null
  rm -rf "$work"
}
trap cleanup EXIT

passed=0
step() { printf '\n== %s\n' "$*"; }
ok() { passed=$((passed + 1)); printf '   ok  %s\n' "$*"; }
fail() { printf '   FAIL %s\n' "$*" >&2; exit 1; }
expect() { # expect DESCRIPTION ACTUAL EXPECTED
  [[ $2 == "$3" ]] && ok "$1" || fail "$1: got '$2', expected '$3'"
}

# as HOME_NAME ARGS...: run the CLI as one of the sandboxed users, always with JSON output.
as() {
  local home=$work/$1
  shift
  HOME=$home XDG_CONFIG_HOME=$home/.config XDG_STATE_HOME=$home/.local/state TASKHUB_TOKEN= TASKHUB_TOKEN_FILE= \
    "$bin" --json "$@"
}
# run HOME_NAME ARGS...: like `as`, but records the exit code in $code and the output in $out.
run() {
  set +e
  out=$(as "$@")
  code=$?
  set -e
}
login() { # login HOME_NAME TOKEN_FILE [ORIGIN]
  mkdir -p "$work/$1"
  chmod 700 "$work/$1"
  as "$1" auth login --with-token --origin "${3:-$origin}" <"$2" >/dev/null
}

step "Setup"
login dev "$E2E_DEV_TOKEN"
login dev2 "$E2E_DEV2_TOKEN"
login tester "$E2E_TESTER_TOKEN"
login doomed "$E2E_REVOKE_TOKEN"
dev_name=$(as dev auth status | jq -r .data.me.owner.username)
ok "logged in four sandboxed users (Developer: $dev_name)"
todo=$(as dev items list --project "$project" --status todo | jq '.data | length')
[[ $todo == 0 ]] || fail "$project already has $todo To Do item(s); move or delete them in TaskHub first"
printf '\x89PNG\r\n\x1a\n%s' "end-to-end screenshot" >"$work/shot.png"
repo=$work/repo
git init -q "$repo" && git -C "$repo" -c user.email=e2e@example.com -c user.name=e2e commit -q --allow-empty -m init
ok "seeded a screenshot and a git repository"

step "Two Developers race next --claim"
r1=$(as tester items create --project "$project" --type task --title "E2E race item one" --priority low | jq -r .data.key)
r2=$(as tester items create --project "$project" --type task --title "E2E race item two" --priority low | jq -r .data.key)
as dev next --claim --project "$project" >"$work/race-dev.json" &
as dev2 next --claim --project "$project" >"$work/race-dev2.json" &
wait
a=$(jq -r .data.key "$work/race-dev.json")
b=$(jq -r .data.key "$work/race-dev2.json")
[[ $a != "$b" && $a != null && $b != null ]] && ok "each claimed a different item ($a, $b)" || fail "race: $a and $b"
lost=$(jq -s -c 'map(.meta.lostRaces // []) | add' "$work/race-dev.json" "$work/race-dev2.json")
ok "lost races reported along the way: $lost"
for key in "$r1" "$r2"; do
  expect "$key is in progress" "$(as tester items get "$key" | jq -r .data.status)" in_progress
done

step "Developer: next --claim → branch --create → show → comment"
key=$(as tester items create --project "$project" --type bug --title "E2E workflow item" --priority high \
  --description "Created by scripts/e2e.sh." | jq -r .data.key)
claimed=$(as dev next --claim --project "$project")
expect "next --claim picked the new item" "$(jq -r .data.key <<<"$claimed")" "$key"
expect "it is in progress" "$(jq -r .data.status <<<"$claimed")" in_progress
branch=$(cd "$repo" && as dev branch "$key" --create | jq -r .data.branch)
expect "branch is checked out" "$(git -C "$repo" branch --show-current)" "$branch"
shown=$(cd "$repo" && as dev show)
expect "show takes the key from the branch" "$(jq -r .data.item.key <<<"$shown")" "$key"
expect "show reports where the key came from" "$(jq -r .meta.ref.source <<<"$shown")" branch
comment=$(cd "$repo" && as dev comment --body "Starting on this. @$tester_name will you review? @nobody-e2e")
expect "the mention notified the Tester" "$(jq -c .data.mentioned <<<"$comment")" "[\"$tester_name\"]"
expect "the mistyped mention is reported" "$(jq -c .data.unmatchedMentions <<<"$comment")" '["nobody-e2e"]'

step "Developer: submit, killed after it was sent, then retry"
port_file=$work/proxy-port
python3 "$here/e2e-delay-proxy.py" "$origin" POST '/submissions$' 4 >"$port_file" &
proxy_pid=$!
for _ in $(seq 50); do [[ -s $port_file ]] && break; sleep 0.1; done
proxy=http://127.0.0.1:$(cat "$port_file")
login slow "$E2E_DEV_TOKEN" "$proxy"
as slow submit "$key" --summary "Fixed the workflow item." --testing "Ran scripts/e2e.sh: the scripted steps pass." \
  --attach "$work/shot.png" --pr https://github.com/example/taskhub/pull/1 >/dev/null 2>&1 &
submit_pid=$!
sleep 2.5
kill -9 "$submit_pid" 2>/dev/null || fail "submit finished before it could be killed"
wait "$submit_pid" 2>/dev/null || true
pending=$(as slow pending)
expect "pending lists the interrupted submission" "$(jq '.data | length' <<<"$pending")" 1
op=$(jq -r '.data[0].id' <<<"$pending")
retried=$(as slow retry "$op")
expect "retry confirms the original write" "$(jq -r .meta.operation.outcome <<<"$retried")" committed
expect "the receipt is a replay" "$(jq -r .meta.operation.replayed <<<"$retried")" true
expect "the item is in Dev Done" "$(jq -r .data.status <<<"$retried")" dev_done
expect "nothing is pending any more" "$(as slow pending | jq '.data | length')" 0
again=$(as slow retry "$op")
expect "a second retry returns the same receipt" "$(jq -r .data.version <<<"$again")" "$(jq -r .data.version <<<"$retried")"
kill "$proxy_pid" && proxy_pid=

step "A token revoked mid-run"
expect "the token works" "$(as doomed show "$key" | jq -r .ok)" true
eval "$E2E_REVOKE_CMD" >/dev/null
run doomed show "$key"
expect "after revoking, exit code" "$code" 3
expect "error code" "$(jq -r .error.code <<<"$out")" UNAUTHENTICATED

step "Tester: inbox --kind review → show → reject with a screenshot"
review=$(as tester inbox --kind review --unread)
[[ $(jq --arg k "$key" '[.data[] | select(.item.key == $k)] | length' <<<"$review") -ge 1 ]] &&
  ok "the review request is in the Tester's inbox" || fail "no review entry for $key"
expect "show sees Dev Done" "$(as tester show "$key" | jq -r .data.item.status)" dev_done
rejected=$(as tester reject "$key" --reason "The screenshot shows the old layout." --attach "$work/shot.png")
expect "reject sends it back" "$(jq -r .data.status <<<"$rejected")" in_progress

step "Developer: inbox --kind rejected → next returns it → submit again"
back=$(as dev inbox --kind rejected --unread)
[[ $(jq --arg k "$key" '[.data[] | select(.item.key == $k)] | length' <<<"$back") -ge 1 ]] &&
  ok "the rejection is in the Developer's inbox" || fail "no rejected entry for $key"
next=$(as dev next --project "$project")
expect "next returns the sent-back item" "$(jq -r .data.key <<<"$next")" "$key"
expect "and says why" "$(jq -r .data.reason <<<"$next")" sent_back
resubmit=$(as dev submit "$key" --summary "Updated the layout." --testing "Checked the new screenshot." \
  --attach "$work/shot.png")
expect "the second submission lands in Dev Done" "$(jq -r .data.status <<<"$resubmit")" dev_done

step "Sign-off stays human"
for who in dev tester; do
  run "$who" items move "$key" done --from dev_done
  expect "$who: moving to Done, exit code" "$code" 4
  expect "$who: error code" "$(jq -r .error.code <<<"$out")" SIGN_OFF_REQUIRES_PERSON
done

step "What TaskHub recorded"
comments=$(as tester comments list "$key" --limit 100)
expect "comments: start, submission, rejection, resubmission" "$(jq '.data | length' <<<"$comments")" 4
expect "every comment was written by an agent" "$(jq '[.data[] | select(.author.agent != true)] | length' <<<"$comments")" 0
expect "screenshots: one per submission and one for the rejection" \
  "$(as tester attachments list "$key" --limit 100 | jq '[.data[] | select(.filename == "shot.png")] | length')" 3
item=$(as tester show "$key")
expect "one PR link, despite the retry" "$(jq '[.data.links.data[] | select(.kind == "pr")] | length' <<<"$item")" 1
expect "the item waits for a Tester's sign-off" "$(jq -r .data.item.status <<<"$item")" dev_done
for who in dev dev2 tester; do
  expect "$who has nothing pending" "$(as "$who" pending | jq '.data | length')" 0
done

printf '\nAll %d checks passed. %s is in Dev Done; a Tester signs it off in the browser.\n' "$passed" "$key"
