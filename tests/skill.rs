//! C3: the embedded skill, its examples, `guide` and `skill install`.

mod common;

use clap::Parser;
use common::*;
use taskhub_cli::cli::Cli;
use taskhub_cli::skill::{SKILL_MD, WRITES_MD};

/// Every `taskhub ...` command the skill shows: lines in code blocks and inline code spans.
fn examples() -> Vec<String> {
    let mut out = Vec::new();
    for doc in [SKILL_MD, WRITES_MD] {
        let mut in_block = false;
        for line in doc.lines() {
            if line.starts_with("```") {
                in_block = !in_block;
                continue;
            }
            if in_block {
                if let Some(command) = line.trim().strip_prefix("taskhub ") {
                    out.push(command.split("  #").next().unwrap().trim().to_owned());
                }
                continue;
            }
            for (i, span) in line.split('`').enumerate() {
                if i % 2 == 1 {
                    if let Some(command) = span.strip_prefix("taskhub ") {
                        out.push(command.to_owned());
                    }
                }
            }
        }
    }
    out
}

#[test]
fn every_example_in_the_skill_parses() {
    let examples = examples();
    assert!(examples.len() >= 15, "found only {} examples", examples.len());
    for example in examples {
        if example.contains('<') || example.ends_with("--help") {
            continue;
        }
        let argv = std::iter::once("taskhub").chain(example.split_whitespace());
        if let Err(error) = Cli::try_parse_from(argv) {
            panic!("`taskhub {example}` does not parse:\n{error}");
        }
    }
}

#[test]
fn guide_prints_the_skill_without_frontmatter() {
    let sandbox = Sandbox::new();
    let run = sandbox.run(&["guide", "--human"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stdout.starts_with("# TaskHub"), "{}", run.stdout);
    assert!(run.stdout.contains("# Writes and recovery"));
    assert!(!run.stdout.contains("taskhub-cli:"));
    let json = sandbox.run(&["guide", "--json"]).json();
    assert!(json["data"]["guide"].as_str().unwrap().contains("## Outcomes"));
}

#[test]
fn install_writes_both_clients_updates_itself_and_never_overwrites_a_foreign_skill() {
    let sandbox = Sandbox::new();
    let run = sandbox.run(&["skill", "install", "--json"]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(run.json()["data"]["installed"].as_array().unwrap().len(), 2);
    for dir in [".claude/skills/taskhub", ".agents/skills/taskhub"] {
        let base = sandbox.home().join(dir);
        assert_eq!(std::fs::read_to_string(base.join("SKILL.md")).unwrap(), SKILL_MD);
        assert_eq!(std::fs::read_to_string(base.join("references/writes.md")).unwrap(), WRITES_MD);
    }
    // Our own bundle is updated in place.
    let codex = sandbox.home().join(".agents/skills/taskhub/SKILL.md");
    std::fs::write(&codex, SKILL_MD.replace("# TaskHub", "# Old")).unwrap();
    assert_eq!(sandbox.run(&["skill", "install", "--client", "codex", "--json"]).code, 0);
    assert_eq!(std::fs::read_to_string(&codex).unwrap(), SKILL_MD);
    // Someone else's skill with the same name is left alone, and nothing is half-written.
    let theirs = "---\nname: taskhub\ndescription: theirs\n---\n# Theirs\n";
    std::fs::write(&codex, theirs).unwrap();
    let claude = sandbox.home().join(".claude/skills/taskhub/SKILL.md");
    std::fs::remove_file(&claude).unwrap();
    let refused = sandbox.run(&["skill", "install", "--json"]);
    assert_eq!(refused.code, 2, "{}", refused.stdout);
    assert_eq!(refused.error_code(), "INVALID_INPUT");
    assert_eq!(std::fs::read_to_string(&codex).unwrap(), theirs);
    assert!(!claude.exists(), "a refusal must not install for the other client either");
}

#[test]
fn project_install_needs_a_repository() {
    let sandbox = Sandbox::new();
    let outside = sandbox.run(&["skill", "install", "--project", "--json"]);
    assert_eq!(outside.code, 2);
    let git =
        std::process::Command::new("git").args(["init", "-q"]).current_dir(sandbox.work()).status().unwrap();
    assert!(git.success());
    let inside = sandbox.run(&["skill", "install", "--project", "--client", "claude", "--json"]);
    assert_eq!(inside.code, 0, "{}", inside.stdout);
    assert!(sandbox.work().join(".claude/skills/taskhub/SKILL.md").exists());
    assert!(!sandbox.home().join(".claude/skills").exists());
}
