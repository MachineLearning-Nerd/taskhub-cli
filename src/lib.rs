//! taskhub-cli: the TaskHub command-line client and MCP server.
#![forbid(unsafe_code)]

pub mod api;
pub mod cli;
pub mod clock;
pub mod commands;
pub mod config;
pub mod credentials;
pub mod digest;
pub mod error;
pub mod git;
pub mod output;
pub mod refs;
pub mod token;
pub mod types;

/// The API version this CLI speaks.
pub const API_VERSION: u32 = 1;
/// The TaskHub commit the bundled contract snapshot came from.
pub const CONTRACT_COMMIT: &str = include_str!(concat!(env!("OUT_DIR"), "/contract_commit.txt"));
