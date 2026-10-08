// Records which TaskHub commit the contract snapshot came from, for `taskhub version`.
use std::{env, fs, path::Path};

fn main() {
    println!("cargo::rerun-if-changed=api/v1/contract-source.json");
    let source = fs::read_to_string("api/v1/contract-source.json").unwrap_or_default();
    let commit = source
        .split("\"taskhubCommit\": \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .unwrap_or("unknown");
    let out = Path::new(&env::var("OUT_DIR").expect("OUT_DIR is set by cargo")).join("contract_commit.txt");
    fs::write(out, commit).expect("OUT_DIR is writable");
}
