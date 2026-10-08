//! C5: the man page and completions in dist-extra/ match the command tree they were generated from.

use std::path::Path;
use std::process::Command;

fn generated(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_taskhub"))
        .args(args)
        .env("TASKHUB_OUTPUT", "human")
        .output()
        .unwrap();
    assert!(output.status.success(), "taskhub {args:?} failed");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn shipped_man_page_and_completions_are_current() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist-extra");
    for (file, args) in [
        ("taskhub.1", &["man"][..]),
        ("completions/taskhub.bash", &["completions", "bash"][..]),
        ("completions/_taskhub", &["completions", "zsh"][..]),
        ("completions/taskhub.fish", &["completions", "fish"][..]),
    ] {
        let shipped = std::fs::read_to_string(root.join(file)).unwrap_or_default();
        assert!(shipped == generated(args), "dist-extra/{file} is out of date; run scripts/gen-docs.sh");
    }
}
