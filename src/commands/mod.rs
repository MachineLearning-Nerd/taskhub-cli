//! Command implementations. Each returns a `Success` or a `CliError`; none prints to stdout or exits.

mod auth;
mod notices;
mod read;
pub(crate) mod render;
mod write;

pub use auth::fetch_me;

use crate::api::{self, Client, HintContext};
use crate::cli::{
    AttachmentsCommand, Cli, Command, CommentsCommand, InboxAction, ItemsCommand, PendingAction,
    ProjectsCommand,
};
use crate::credentials::{self, Credential};
use crate::error::{CliError, Result};
use crate::output::{Mode, Success};
use clap::CommandFactory;
use serde_json::{Value, json};
use std::time::Duration;

/// A logged-in client for one command.
pub struct Session {
    pub credential: Credential,
    pub client: Client,
}

impl Session {
    pub fn open(deadline: Duration) -> Result<Session> {
        let credential = credentials::load()?;
        let client = Client::new(&credential, deadline)?;
        Ok(Session { credential, client })
    }

    /// GET an envelope; returns `(data, meta)`.
    pub fn get(&self, path: &str, query: &[(String, String)], key: Option<&str>) -> Result<(Value, Value)> {
        let hint = HintContext { key: key.map(str::to_owned) };
        let mut envelope = self.client.get(path, query, &hint)?;
        let data = envelope.get_mut("data").map(Value::take).unwrap_or(Value::Null);
        let meta = envelope.get_mut("meta").map(Value::take).unwrap_or(Value::Null);
        Ok((data, meta))
    }
}

pub fn run(cli: Cli, mode: Mode) -> Result<Success> {
    // `auth` already reports expiry; offline commands must stay offline.
    let wants_notices = !matches!(
        cli.command,
        Command::Auth(_) | Command::Version | Command::Completions(_) | Command::Man | Command::Guide
    );
    let result = dispatch(cli.command);
    if mode == Mode::Human && wants_notices && result.is_ok() {
        notices::daily();
    }
    result
}

fn dispatch(command: Command) -> Result<Success> {
    match command {
        Command::Auth(command) => auth::run(command),
        Command::Show(args) => read::show(args),
        Command::Mine(args) => read::mine(args),
        Command::Next(args) => write::next(args),
        Command::Claim(args) => write::claim(args),
        Command::Comment(args) => write::comment(args),
        Command::Submit(args) => write::submit(args),
        Command::Reject(args) => write::reject(args),
        Command::Link(args) => write::link(args),
        Command::Branch(args) => write::branch(args),
        Command::Open(args) => write::open(args),
        Command::Inbox(args) => match args.action {
            Some(InboxAction::Done(done)) => write::inbox_done(done),
            None => read::inbox(args),
        },
        Command::Items(ItemsCommand::Create(args)) => write::items_create(args),
        Command::Items(ItemsCommand::Update(args)) => write::items_update(args),
        Command::Items(ItemsCommand::Move(args)) => write::items_move(args),
        Command::Comments(CommentsCommand::Edit(args)) => write::comments_edit(args),
        Command::Attachments(AttachmentsCommand::Add(args)) => write::attachments_add(args),
        Command::Pending(args) => match args.action {
            Some(PendingAction::Discard(op)) => write::pending_discard(op),
            None => write::pending(),
        },
        Command::Retry(args) => write::retry(args),
        Command::Items(ItemsCommand::List(args)) => read::items_list(args),
        Command::Items(ItemsCommand::Get(args)) => read::items_get(args),
        Command::Items(ItemsCommand::Activity(args)) => read::activity(args),
        Command::Comments(CommentsCommand::List(args)) => read::comments_list(args),
        Command::Attachments(AttachmentsCommand::List(args)) => read::attachments_list(args),
        Command::Attachments(AttachmentsCommand::Download(args)) => read::download(args),
        Command::Projects(ProjectsCommand::List) => read::projects_list(),
        Command::Projects(ProjectsCommand::Show(args)) => read::project_show(args),
        Command::Version => Ok(version()),
        Command::Completions(args) => {
            let mut command = Cli::command();
            let mut buffer = Vec::new();
            clap_complete::generate(args.shell, &mut command, "taskhub", &mut buffer);
            Ok(raw(buffer))
        }
        Command::Man => {
            let mut buffer = Vec::new();
            clap_mangen::Man::new(Cli::command())
                .render(&mut buffer)
                .map_err(|e| CliError::internal(format!("Could not render the man page: {e}")))?;
            Ok(raw(buffer))
        }
        _ => Err(CliError::internal("This command is not implemented yet.")),
    }
}

fn version() -> Success {
    let cli = env!("CARGO_PKG_VERSION");
    let commit = crate::CONTRACT_COMMIT;
    Success::new(
        json!({ "cli": cli, "apiVersion": crate::API_VERSION, "contract": commit }),
        format!("taskhub {cli}\nAPI version {}\nContract from TaskHub {commit}", crate::API_VERSION),
    )
}

/// Output that is not an envelope (completions, the man page): printed as-is in both modes.
fn raw(bytes: Vec<u8>) -> Success {
    let mut success = Success::new(Value::Null, String::from_utf8_lossy(&bytes).into_owned());
    success.meta.insert("raw".into(), json!(true));
    success
}

/// Query pairs with repeated keys for lists, as TaskHub expects (`status=todo&status=in_progress`).
#[derive(Default)]
pub struct Query(Vec<(String, String)>);

impl Query {
    pub fn one(mut self, key: &str, value: Option<impl ToString>) -> Self {
        if let Some(value) = value {
            self.0.push((key.to_owned(), value.to_string()));
        }
        self
    }

    pub fn many<T: ToString>(mut self, key: &str, values: impl IntoIterator<Item = T>) -> Self {
        self.0.extend(values.into_iter().map(|value| (key.to_owned(), value.to_string())));
        self
    }

    pub fn flag(self, key: &str, on: bool) -> Self {
        self.one(key, on.then_some("true"))
    }

    pub fn pairs(&self) -> &[(String, String)] {
        &self.0
    }
}

pub fn default_deadline() -> Duration {
    api::COMMAND_DEADLINE
}
