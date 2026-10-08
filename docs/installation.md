# Installing taskhub

`taskhub` is one self-contained binary. Releases cover Linux (static musl builds, x86_64 and aarch64) and macOS (Intel and Apple silicon). Every archive holds the binary, the man page, shell completions, the README and the license, with a SHA-256 checksum and a GitHub build attestation.

> **Not published yet.** The repository, release tags, Homebrew tap and AUR package don't exist until the owner creates them. The commands below show the intended layout; the names in them (`MachineLearning-Nerd/taskhub-cli`, the tap, `taskhub-cli-bin`) are proposals. Until then, build from source.

## Shell installer (Linux and macOS)

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/MachineLearning-Nerd/taskhub-cli/releases/latest/download/taskhub-cli-installer.sh | sh
```

It installs into `$XDG_BIN_HOME` or `~/.local/bin`. It doesn't install the man page or completions; take those from the archive, or use a package below.

## Homebrew (macOS and Linux)

```sh
brew install MachineLearning-Nerd/tap/taskhub
```

The formula installs the binary, `man taskhub`, and bash, zsh and fish completions.

## Arch Linux (AUR)

```sh
yay -S taskhub-cli-bin     # or any AUR helper; the package installs /usr/bin/taskhub
```

## From a release archive

```sh
target=x86_64-unknown-linux-musl   # or aarch64-unknown-linux-musl, x86_64-apple-darwin, aarch64-apple-darwin
base=https://github.com/MachineLearning-Nerd/taskhub-cli/releases/latest/download
curl -LO "$base/taskhub-cli-$target.tar.xz" -LO "$base/taskhub-cli-$target.tar.xz.sha256"
sha256sum -c "taskhub-cli-$target.tar.xz.sha256"        # macOS: shasum -a 256 -c
gh attestation verify "taskhub-cli-$target.tar.xz" -R MachineLearning-Nerd/taskhub-cli   # optional
tar -xJf "taskhub-cli-$target.tar.xz"
install -m 755 "taskhub-cli-$target/taskhub" ~/.local/bin/
```

Then, optionally:

```sh
install -Dm644 "taskhub-cli-$target/taskhub.1" ~/.local/share/man/man1/taskhub.1
install -Dm644 "taskhub-cli-$target/completions/taskhub.bash" ~/.local/share/bash-completion/completions/taskhub
install -Dm644 "taskhub-cli-$target/completions/_taskhub" ~/.local/share/zsh/site-functions/_taskhub   # on fpath
install -Dm644 "taskhub-cli-$target/completions/taskhub.fish" ~/.config/fish/completions/taskhub.fish
```

## From source

Needs Rust 1.85 or newer.

```sh
cargo install --locked --git https://github.com/MachineLearning-Nerd/taskhub-cli
```

Or `taskhub completions <shell>` and `taskhub man` print the same completions and man page as the archives.

## After installing

```sh
taskhub --version
taskhub auth login --with-token < token-file   # create the token in TaskHub: user menu → API tokens
taskhub auth status
taskhub skill install                          # optional: the skill for Claude Code and Codex
```

For agents that speak MCP rather than shell, register `taskhub mcp` as a stdio server; it uses the same login.

`taskhub` tells you about a newer version at most once a day, on human output only. Upgrade with the same method you installed with.

## Uninstalling

Run `taskhub auth logout` first if you want the saved token deleted, then remove the binary or the package. Configuration lives in `~/.config/taskhub/` and the write journal in `~/.local/state/taskhub/`.

## Releasing (owner)

1. Decide the repository, tap and AUR names, and create them. Update `repository` in `Cargo.toml` if it changes, then run `dist generate`.
2. Bump the version in `Cargo.toml`, run `scripts/gen-docs.sh` (the man page carries the version), and commit.
3. Push a tag `vX.Y.Z`. `.github/workflows/release.yml` builds the four targets, checksums, the shell installer and attestations, and publishes the GitHub Release.
4. Download the `.sha256` files and run `scripts/render-packaging.sh X.Y.Z <dir> <out>`. Commit `taskhub.rb` to the tap. Copy `PKGBUILD` into the AUR repository, run `makepkg --printsrcinfo > .SRCINFO`, and push.
