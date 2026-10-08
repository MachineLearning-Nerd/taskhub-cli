//! The command-line surface, as specified in PLAN.md "Commands". Parsing only: behavior lives in `commands/`.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "taskhub",
    version,
    about = "Work with TaskHub tasks and bugs from the terminal and from coding agents.",
    long_about = "Work with TaskHub tasks and bugs from the terminal and from coding agents.\n\n\
        Output is JSON when stdout is not a terminal. Start with `taskhub auth login`, then `taskhub next --claim`.",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Print the JSON envelope even on a terminal.
    #[arg(long, global = true, conflicts_with = "human")]
    pub json: bool,
    /// Print human-readable output even when piped.
    #[arg(long, global = true)]
    pub human: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Find the next item to work on: sent-back work first, then assigned, then unassigned.
    Next(NextArgs),
    /// Assign an item to yourself and move it from To Do to In Progress.
    Claim(RefArg),
    /// Show an item with its project, allowed moves, latest comments, files and links.
    Show(ShowArgs),
    /// Your To Do and In Progress items.
    Mine(MineArgs),
    /// Add a Markdown comment; @username notifies.
    Comment(CommentArgs),
    /// Submit work for review: evidence comment, links and the move to Dev Done in one step.
    Submit(SubmitArgs),
    /// Send an item in Dev Done back to In Progress with a reason (Testers).
    Reject(RejectArgs),
    /// Inbox entries for your projects. Reading never marks them read.
    Inbox(InboxArgs),
    /// Add a link to an item; adding the same URL again is harmless.
    Link(LinkArgs),
    /// Print a branch name for an item, or switch to it with --create.
    Branch(BranchArgs),
    /// Open the item in the browser (prints the URL when not on a terminal).
    Open(RefArg),
    /// List, read, create, edit and move items.
    #[command(subcommand)]
    Items(ItemsCommand),
    /// List and edit comments.
    #[command(subcommand)]
    Comments(CommentsCommand),
    /// List, upload and download files.
    #[command(subcommand)]
    Attachments(AttachmentsCommand),
    /// List projects and show their labels, members and limits.
    #[command(subcommand)]
    Projects(ProjectsCommand),
    /// Writes whose outcome is unknown.
    Pending(PendingArgs),
    /// Resend a pending write exactly, with its original request ID.
    Retry(RetryArgs),
    /// Log in with an API token, check the login, or log out.
    #[command(subcommand)]
    Auth(AuthCommand),
    /// A one-page guide for agents.
    Guide,
    /// Install the TaskHub agent skill for Claude Code or Codex.
    #[command(subcommand)]
    Skill(SkillCommand),
    /// Serve TaskHub tools over MCP on stdin and stdout.
    Mcp,
    /// Print shell completions.
    Completions(CompletionsArgs),
    /// Print the CLI, contract and API versions. Works offline.
    Version,
    /// Write the man page to stdout (for packaging).
    #[command(hide = true)]
    Man,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ItemTypeArg {
    Task,
    Bug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum StatusArg {
    Todo,
    InProgress,
    DevDone,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PriorityArg {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum KindArg {
    Mention,
    Assigned,
    Rejected,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LinkKindArg {
    Pr,
    Link,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortArg {
    Number,
    Priority,
}

/// Options shared by every write.
#[derive(Debug, Clone, Default, Args)]
pub struct WriteOpts {
    /// Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records).
    #[arg(long, value_name = "UUID")]
    pub request_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct RefArg {
    /// Item key such as WEB-12. Taken from the git branch when left out.
    #[arg(value_name = "REF")]
    pub reference: Option<String>,
    #[command(flatten)]
    pub write: WriteOpts,
}

#[derive(Debug, Args)]
pub struct NextArgs {
    /// Only these projects.
    #[arg(long = "project", value_name = "KEY")]
    pub projects: Vec<String>,
    #[arg(long = "type", value_enum)]
    pub item_type: Option<ItemTypeArg>,
    /// Claim the item (assigned or unassigned work), retrying the next item if another agent wins.
    #[arg(long)]
    pub claim: bool,
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    #[arg(value_name = "REF")]
    pub reference: Option<String>,
    /// How many of the latest comments to include (1–100).
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub comments: u32,
    /// How many files to include (1–100).
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub attachments: u32,
}

#[derive(Debug, Args)]
pub struct MineArgs {
    #[arg(long = "status", value_enum)]
    pub statuses: Vec<StatusArg>,
}

#[derive(Debug, Args)]
#[group(id = "body_source", required = true, multiple = false, args = ["body", "body_file"])]
pub struct CommentArgs {
    #[arg(value_name = "REF")]
    pub reference: Option<String>,
    /// Comment text (Markdown).
    #[arg(long)]
    pub body: Option<String>,
    /// Read the comment from a file, or `-` for stdin.
    #[arg(long, value_name = "PATH")]
    pub body_file: Option<PathBuf>,
    /// Attach files (png, jpg, gif, webp, pdf, md, docx; up to 4 MB each).
    #[arg(long = "attach", value_name = "FILE")]
    pub attach: Vec<PathBuf>,
    #[command(flatten)]
    pub write: WriteOpts,
}

#[derive(Debug, Args)]
pub struct SubmitArgs {
    #[arg(value_name = "REF")]
    pub reference: Option<String>,
    /// The status you saw; the submission fails if the item has moved since.
    #[arg(long, value_enum, default_value = "in_progress")]
    pub from: StatusArg,
    /// What changed.
    #[arg(long, conflicts_with = "summary_file")]
    pub summary: Option<String>,
    #[arg(long, value_name = "PATH")]
    pub summary_file: Option<PathBuf>,
    /// How it was tested. Name tests you did not run under limitations.
    #[arg(long, conflicts_with = "testing_file")]
    pub testing: Option<String>,
    #[arg(long, value_name = "PATH")]
    pub testing_file: Option<PathBuf>,
    #[arg(long, conflicts_with = "limitations_file")]
    pub limitations: Option<String>,
    #[arg(long, value_name = "PATH")]
    pub limitations_file: Option<PathBuf>,
    #[arg(long = "attach", value_name = "FILE")]
    pub attach: Vec<PathBuf>,
    /// Pull request URL, or `auto` to ask `gh` for the current branch's PR.
    #[arg(long, value_name = "URL|auto")]
    pub pr: Option<String>,
    /// Related links.
    #[arg(long = "link", value_name = "URL")]
    pub links: Vec<String>,
    /// Read summary, testing, limitations, links and files from a JSON file (or `-`).
    #[arg(long, value_name = "PATH")]
    pub input_file: Option<PathBuf>,
    #[command(flatten)]
    pub write: WriteOpts,
}

#[derive(Debug, Args)]
#[group(id = "reason_source", required = true, multiple = false, args = ["reason", "reason_file"])]
pub struct RejectArgs {
    #[arg(value_name = "REF")]
    pub reference: String,
    #[arg(long)]
    pub reason: Option<String>,
    #[arg(long, value_name = "PATH")]
    pub reason_file: Option<PathBuf>,
    #[arg(long = "attach", value_name = "FILE")]
    pub attach: Vec<PathBuf>,
    #[command(flatten)]
    pub write: WriteOpts,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct InboxArgs {
    #[command(subcommand)]
    pub action: Option<InboxAction>,
    #[arg(long)]
    pub unread: bool,
    #[arg(long = "kind", value_enum)]
    pub kinds: Vec<KindArg>,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub limit: Option<u32>,
    #[arg(long)]
    pub cursor: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum InboxAction {
    /// Mark entries read.
    Done(InboxDoneArgs),
}

#[derive(Debug, Args)]
#[group(id = "which", required = true, multiple = false, args = ["ids", "all"])]
pub struct InboxDoneArgs {
    #[arg(value_name = "ID")]
    pub ids: Vec<String>,
    #[arg(long)]
    pub all: bool,
    #[command(flatten)]
    pub write: WriteOpts,
}

#[derive(Debug, Args)]
pub struct LinkArgs {
    /// `[REF] URL`: REF is used only if it looks like KEY-N.
    #[arg(value_name = "[REF] URL", num_args = 1..=2, required = true)]
    pub positionals: Vec<String>,
    #[arg(long, value_enum, default_value = "link")]
    pub kind: LinkKindArg,
    #[command(flatten)]
    pub write: WriteOpts,
}

#[derive(Debug, Args)]
pub struct BranchArgs {
    #[arg(value_name = "REF")]
    pub reference: Option<String>,
    /// Switch to the branch, creating it if needed.
    #[arg(long)]
    pub create: bool,
}

#[derive(Debug, Subcommand)]
pub enum ItemsCommand {
    /// Item summaries.
    List(ItemsListArgs),
    /// One item.
    Get(ItemsGetArgs),
    /// The activity timeline.
    Activity(PagedRefArgs),
    /// Create an item in To Do.
    Create(ItemsCreateArgs),
    /// Change some fields; the rest stay as they are.
    Update(ItemsUpdateArgs),
    /// Move between To Do, In Progress and Dev Done.
    Move(ItemsMoveArgs),
}

#[derive(Debug, Args)]
pub struct ItemsListArgs {
    #[arg(long = "project", value_name = "KEY")]
    pub projects: Vec<String>,
    #[arg(long = "type", value_enum)]
    pub item_type: Option<ItemTypeArg>,
    #[arg(long = "status", value_enum)]
    pub statuses: Vec<StatusArg>,
    #[arg(long, value_enum)]
    pub priority: Option<PriorityArg>,
    /// `me`, `none` or a username.
    #[arg(long)]
    pub assignee: Option<String>,
    /// Items with any of these labels.
    #[arg(long = "label", value_name = "NAME")]
    pub labels: Vec<String>,
    #[arg(long)]
    pub search: Option<String>,
    /// Only items agents have worked on.
    #[arg(long)]
    pub agent: bool,
    #[arg(long, value_enum)]
    pub sort: Option<SortArg>,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub limit: Option<u32>,
    #[arg(long)]
    pub cursor: Option<String>,
}

#[derive(Debug, Args)]
pub struct ItemsGetArgs {
    #[arg(value_name = "REF")]
    pub reference: Option<String>,
}

#[derive(Debug, Args)]
pub struct PagedRefArgs {
    #[arg(value_name = "REF")]
    pub reference: Option<String>,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub limit: Option<u32>,
    #[arg(long)]
    pub cursor: Option<String>,
}

#[derive(Debug, Args)]
pub struct ItemsCreateArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long = "type", value_enum)]
    pub item_type: Option<ItemTypeArg>,
    #[arg(long)]
    pub title: Option<String>,
    /// The description (Markdown).
    #[arg(long, conflicts_with = "body_file")]
    pub description: Option<String>,
    /// Read the description from a file, or `-` for stdin.
    #[arg(long, value_name = "PATH")]
    pub body_file: Option<PathBuf>,
    #[arg(long, value_enum)]
    pub priority: Option<PriorityArg>,
    #[arg(long)]
    pub assignee: Option<String>,
    #[arg(long = "label", value_name = "NAME")]
    pub labels: Vec<String>,
    /// Read the fields from a JSON file (or `-`): project, type, title, description, priority, assignee, labels.
    #[arg(long, value_name = "PATH")]
    pub input_file: Option<PathBuf>,
    #[command(flatten)]
    pub write: WriteOpts,
}

#[derive(Debug, Args)]
pub struct ItemsUpdateArgs {
    #[arg(value_name = "REF")]
    pub reference: Option<String>,
    /// The version you saw; the edit fails if the item changed since.
    #[arg(long, value_name = "N")]
    pub if_version: u64,
    #[arg(long = "type", value_enum)]
    pub item_type: Option<ItemTypeArg>,
    #[arg(long)]
    pub title: Option<String>,
    /// The new description (Markdown).
    #[arg(long, conflicts_with = "body_file")]
    pub description: Option<String>,
    /// Read the new description from a file, or `-` for stdin.
    #[arg(long, value_name = "PATH")]
    pub body_file: Option<PathBuf>,
    #[arg(long, value_enum)]
    pub priority: Option<PriorityArg>,
    /// A username, or `none` to unassign.
    #[arg(long)]
    pub assignee: Option<String>,
    /// Replace the labels with these.
    #[arg(long = "label", value_name = "NAME", conflicts_with = "no_labels")]
    pub labels: Vec<String>,
    /// Remove every label.
    #[arg(long)]
    pub no_labels: bool,
    #[command(flatten)]
    pub write: WriteOpts,
}

#[derive(Debug, Args)]
pub struct ItemsMoveArgs {
    /// `[REF] STATUS`: REF is used only if it looks like KEY-N.
    #[arg(value_name = "[REF] STATUS", num_args = 1..=2, required = true)]
    pub positionals: Vec<String>,
    /// The status you saw.
    #[arg(long, value_enum)]
    pub from: StatusArg,
    #[command(flatten)]
    pub write: WriteOpts,
}

#[derive(Debug, Subcommand)]
pub enum CommentsCommand {
    /// Comments, newest first. Continue with --cursor.
    List(PagedRefArgs),
    /// Edit a comment you wrote through a token.
    Edit(CommentsEditArgs),
}

#[derive(Debug, Args)]
#[group(id = "body_source", required = true, multiple = false, args = ["body", "body_file"])]
pub struct CommentsEditArgs {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long, value_name = "PATH")]
    pub body_file: Option<PathBuf>,
    #[command(flatten)]
    pub write: WriteOpts,
}

#[derive(Debug, Subcommand)]
pub enum AttachmentsCommand {
    /// File metadata.
    List(PagedRefArgs),
    /// Upload files to an item.
    Add(AttachmentsAddArgs),
    /// Download a file.
    Download(AttachmentsDownloadArgs),
}

#[derive(Debug, Args)]
pub struct AttachmentsAddArgs {
    /// `[REF] FILE...`: REF is used only if it looks like KEY-N.
    #[arg(value_name = "[REF] FILE", num_args = 1.., required = true)]
    pub positionals: Vec<String>,
    #[command(flatten)]
    pub write: WriteOpts,
}

#[derive(Debug, Args)]
pub struct AttachmentsDownloadArgs {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(long, short, value_name = "PATH")]
    pub output: PathBuf,
    /// Replace an existing file.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Subcommand)]
pub enum ProjectsCommand {
    /// Projects you can reach with this token.
    List,
    /// One project: labels, members, enums and limits.
    Show(ProjectShowArgs),
}

#[derive(Debug, Args)]
pub struct ProjectShowArgs {
    #[arg(value_name = "KEY")]
    pub key: String,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct PendingArgs {
    #[command(subcommand)]
    pub action: Option<PendingAction>,
}

#[derive(Debug, Subcommand)]
pub enum PendingAction {
    /// Forget a pending write after checking it yourself.
    Discard(OperationArg),
}

#[derive(Debug, Args)]
pub struct OperationArg {
    #[arg(value_name = "OP")]
    pub id: String,
}

#[derive(Debug, Args)]
pub struct RetryArgs {
    #[arg(value_name = "OP")]
    pub id: String,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Save an API token. Reads it from stdin with --with-token, or prompts on a terminal.
    Login(LoginArgs),
    /// Who you are logged in as, and where the credential came from.
    Status,
    /// Delete the saved credentials.
    Logout,
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// Read the token from stdin.
    #[arg(long)]
    pub with_token: bool,
    /// The TaskHub server. HTTPS, or http://127.0.0.1 for local development.
    #[arg(long, value_name = "URL")]
    pub origin: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum SkillCommand {
    /// Write the embedded skill into Claude Code's and/or Codex's skill directory.
    Install(SkillInstallArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ClientArg {
    Claude,
    Codex,
    All,
}

#[derive(Debug, Args)]
pub struct SkillInstallArgs {
    #[arg(long, value_enum, default_value = "all")]
    pub client: ClientArg,
    /// Install into this repository instead of your home directory.
    #[arg(long)]
    pub project: bool,
}

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}

impl StatusArg {
    pub fn as_status(self) -> crate::types::Status {
        use crate::types::Status;
        match self {
            StatusArg::Todo => Status::Todo,
            StatusArg::InProgress => Status::InProgress,
            StatusArg::DevDone => Status::DevDone,
            StatusArg::Done => Status::Done,
        }
    }
}

impl ItemTypeArg {
    pub fn as_type(self) -> crate::types::ItemType {
        match self {
            ItemTypeArg::Task => crate::types::ItemType::Task,
            ItemTypeArg::Bug => crate::types::ItemType::Bug,
        }
    }
}

impl PriorityArg {
    pub fn as_priority(self) -> crate::types::Priority {
        match self {
            PriorityArg::High => crate::types::Priority::High,
            PriorityArg::Medium => crate::types::Priority::Medium,
            PriorityArg::Low => crate::types::Priority::Low,
        }
    }
}

impl LinkKindArg {
    pub fn as_kind(self) -> crate::types::LinkKind {
        match self {
            LinkKindArg::Pr => crate::types::LinkKind::Pr,
            LinkKindArg::Link => crate::types::LinkKind::Link,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn command_tree_is_consistent() {
        Cli::command().debug_assert();
    }
}
