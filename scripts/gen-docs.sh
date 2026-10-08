#!/usr/bin/env bash
# Regenerates the man page and shell completions that ship in the release archives.
# tests/dist_extra.rs fails when these files are out of date with the command tree.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
cargo build --quiet --locked
bin=target/debug/taskhub
mkdir -p dist-extra/completions
"$bin" man >dist-extra/taskhub.1
"$bin" completions bash >dist-extra/completions/taskhub.bash
"$bin" completions zsh >dist-extra/completions/_taskhub
"$bin" completions fish >dist-extra/completions/taskhub.fish
echo "Wrote dist-extra/taskhub.1 and dist-extra/completions/"
