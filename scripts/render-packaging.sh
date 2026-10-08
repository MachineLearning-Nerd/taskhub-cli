#!/usr/bin/env bash
# Renders the Homebrew formula and AUR PKGBUILD for a release from its checksum files.
# Usage: scripts/render-packaging.sh VERSION CHECKSUM_DIR OUT_DIR
#   CHECKSUM_DIR holds taskhub-cli-<target>.tar.xz.sha256 files (from the GitHub Release, or target/distrib).
# Env: REPOSITORY (default from Cargo.toml), MAINTAINER_EMAIL (default: the git user.email).
set -euo pipefail
version=${1:?version}; sums=${2:?checksum dir}; out=${3:?output dir}
root=$(cd "$(dirname "$0")/.." && pwd)
repository=${REPOSITORY:-$(sed -n 's/^repository = "\(.*\)"$/\1/p' "$root/Cargo.toml")}
email=${MAINTAINER_EMAIL:-$(git config user.email || true)}
mkdir -p "$out"
render() { # render TEMPLATE OUTPUT
  local text
  text=$(sed -e "s|@VERSION@|$version|g" -e "s|@REPOSITORY@|$repository|g" -e "s|@MAINTAINER_EMAIL@|$email|g" "$1")
  for target in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl; do
    local placeholder file
    placeholder="@SHA256_$(tr 'a-z-' 'A-Z_' <<<"$target")@"
    file=$sums/taskhub-cli-$target.tar.xz.sha256
    if [[ $text == *"$placeholder"* ]]; then
      [[ -f $file ]] || { echo "missing $file" >&2; exit 1; }
      text=${text//$placeholder/$(cut -d' ' -f1 "$file")}
    fi
  done
  if grep -qE '@[A-Z0-9_]+@' <<<"$text"; then
    grep -oE '@[A-Z0-9_]+@' <<<"$text" | sort -u >&2
    echo "unfilled placeholders in $2" >&2
    exit 1
  fi
  printf '%s\n' "$text" >"$2"
}
render "$root/packaging/homebrew/taskhub.rb.in" "$out/taskhub.rb"
render "$root/packaging/aur/PKGBUILD.in" "$out/PKGBUILD"
echo "Wrote $out/taskhub.rb and $out/PKGBUILD"
