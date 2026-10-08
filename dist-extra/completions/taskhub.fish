# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_taskhub_global_optspecs
    string join \n json human h/help V/version
end

function __fish_taskhub_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_taskhub_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_taskhub_using_subcommand
    set -l cmd (__fish_taskhub_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c taskhub -n "__fish_taskhub_needs_command" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_needs_command" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c taskhub -n "__fish_taskhub_needs_command" -s V -l version -d 'Print version'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "next" -d 'Find the next item to work on: sent-back work first, then assigned, then unassigned'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "claim" -d 'Assign an item to yourself and move it from To Do to In Progress'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "show" -d 'Show an item with its project, allowed moves, latest comments, files and links'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "mine" -d 'Your To Do and In Progress items'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "comment" -d 'Add a Markdown comment; @username notifies'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "submit" -d 'Submit work for review: evidence comment, links and the move to Dev Done in one step'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "reject" -d 'Send an item in Dev Done back to In Progress with a reason (Testers)'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "inbox" -d 'Inbox entries for your projects. Reading never marks them read'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "link" -d 'Add a link to an item; adding the same URL again is harmless'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "branch" -d 'Print a branch name for an item, or switch to it with --create'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "open" -d 'Open the item in the browser (prints the URL when not on a terminal)'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "items" -d 'List, read, create, edit and move items'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "comments" -d 'List and edit comments'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "attachments" -d 'List, upload and download files'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "projects" -d 'List projects and show their labels, members and limits'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "pending" -d 'Writes whose outcome is unknown'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "retry" -d 'Resend a pending write exactly, with its original request ID'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "auth" -d 'Log in with an API token, check the login, or log out'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "guide" -d 'A one-page guide for agents'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "skill" -d 'Install the TaskHub agent skill for Claude Code or Codex'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "mcp" -d 'Serve TaskHub tools over MCP on stdin and stdout'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "completions" -d 'Print shell completions'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "version" -d 'Print the CLI, contract and API versions. Works offline'
complete -c taskhub -n "__fish_taskhub_needs_command" -f -a "man" -d 'Write the man page to stdout (for packaging)'
complete -c taskhub -n "__fish_taskhub_using_subcommand next" -l project -d 'Only these projects' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand next" -l type -r -f -a "task\t''
bug\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand next" -l claim -d 'Claim the item (assigned or unassigned work), retrying the next item if another agent wins'
complete -c taskhub -n "__fish_taskhub_using_subcommand next" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand next" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand next" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand claim" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand claim" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand claim" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand claim" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand show" -l comments -d 'How many of the latest comments to include (1–100)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand show" -l attachments -d 'How many files to include (1–100)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand show" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand show" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand show" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand mine" -l status -r -f -a "todo\t''
in_progress\t''
dev_done\t''
done\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand mine" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand mine" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand mine" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand comment" -l body -d 'Comment text (Markdown)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand comment" -l body-file -d 'Read the comment from a file, or `-` for stdin' -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand comment" -l attach -d 'Attach files (png, jpg, gif, webp, pdf, md, docx; up to 4 MB each)' -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand comment" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand comment" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand comment" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand comment" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l from -d 'The status you saw; the submission fails if the item has moved since' -r -f -a "todo\t''
in_progress\t''
dev_done\t''
done\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l summary -d 'What changed' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l summary-file -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l testing -d 'How it was tested. Name tests you did not run under limitations' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l testing-file -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l limitations -r
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l limitations-file -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l attach -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l pr -d 'Pull request URL, or `auto` to ask `gh` for the current branch\'s PR' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l link -d 'Related links' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l input-file -d 'Read summary, testing, limitations, links and files from a JSON file (or `-`)' -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand submit" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand reject" -l reason -r
complete -c taskhub -n "__fish_taskhub_using_subcommand reject" -l reason-file -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand reject" -l attach -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand reject" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand reject" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand reject" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand reject" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and not __fish_seen_subcommand_from done" -l kind -r -f -a "mention\t''
assigned\t''
rejected\t''
review\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and not __fish_seen_subcommand_from done" -l limit -r
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and not __fish_seen_subcommand_from done" -l cursor -r
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and not __fish_seen_subcommand_from done" -l unread
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and not __fish_seen_subcommand_from done" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and not __fish_seen_subcommand_from done" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and not __fish_seen_subcommand_from done" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and not __fish_seen_subcommand_from done" -f -a "done" -d 'Mark entries read'
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and __fish_seen_subcommand_from done" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and __fish_seen_subcommand_from done" -l all
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and __fish_seen_subcommand_from done" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and __fish_seen_subcommand_from done" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand inbox; and __fish_seen_subcommand_from done" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand link" -l kind -r -f -a "pr\t''
link\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand link" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand link" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand link" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand link" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand branch" -l create -d 'Switch to the branch, creating it if needed'
complete -c taskhub -n "__fish_taskhub_using_subcommand branch" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand branch" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand branch" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand open" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand open" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand open" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand open" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and not __fish_seen_subcommand_from list get activity create update move" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and not __fish_seen_subcommand_from list get activity create update move" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and not __fish_seen_subcommand_from list get activity create update move" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and not __fish_seen_subcommand_from list get activity create update move" -f -a "list" -d 'Item summaries'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and not __fish_seen_subcommand_from list get activity create update move" -f -a "get" -d 'One item'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and not __fish_seen_subcommand_from list get activity create update move" -f -a "activity" -d 'The activity timeline'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and not __fish_seen_subcommand_from list get activity create update move" -f -a "create" -d 'Create an item in To Do'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and not __fish_seen_subcommand_from list get activity create update move" -f -a "update" -d 'Change some fields; the rest stay as they are'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and not __fish_seen_subcommand_from list get activity create update move" -f -a "move" -d 'Move between To Do, In Progress and Dev Done'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l project -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l type -r -f -a "task\t''
bug\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l status -r -f -a "todo\t''
in_progress\t''
dev_done\t''
done\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l priority -r -f -a "high\t''
medium\t''
low\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l assignee -d '`me`, `none` or a username' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l label -d 'Items with any of these labels' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l search -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l sort -r -f -a "number\t''
priority\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l limit -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l cursor -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l agent -d 'Only items agents have worked on'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from get" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from get" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from get" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from activity" -l limit -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from activity" -l cursor -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from activity" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from activity" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from activity" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l project -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l type -r -f -a "task\t''
bug\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l title -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l description -d 'The description (Markdown)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l body-file -d 'Read the description from a file, or `-` for stdin' -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l priority -r -f -a "high\t''
medium\t''
low\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l assignee -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l label -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l input-file -d 'Read the fields from a JSON file (or `-`): project, type, title, description, priority, assignee, labels' -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from create" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l if-version -d 'The version you saw; the edit fails if the item changed since' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l type -r -f -a "task\t''
bug\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l title -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l description -d 'The new description (Markdown)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l body-file -d 'Read the new description from a file, or `-` for stdin' -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l priority -r -f -a "high\t''
medium\t''
low\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l assignee -d 'A username, or `none` to unassign' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l label -d 'Replace the labels with these' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l no-labels -d 'Remove every label'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from update" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from move" -l from -d 'The status you saw' -r -f -a "todo\t''
in_progress\t''
dev_done\t''
done\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from move" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from move" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from move" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand items; and __fish_seen_subcommand_from move" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and not __fish_seen_subcommand_from list edit" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and not __fish_seen_subcommand_from list edit" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and not __fish_seen_subcommand_from list edit" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and not __fish_seen_subcommand_from list edit" -f -a "list" -d 'Comments, newest first. Continue with --cursor'
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and not __fish_seen_subcommand_from list edit" -f -a "edit" -d 'Edit a comment you wrote through a token'
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and __fish_seen_subcommand_from list" -l limit -r
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and __fish_seen_subcommand_from list" -l cursor -r
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and __fish_seen_subcommand_from list" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and __fish_seen_subcommand_from list" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and __fish_seen_subcommand_from edit" -l body -r
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and __fish_seen_subcommand_from edit" -l body-file -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and __fish_seen_subcommand_from edit" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and __fish_seen_subcommand_from edit" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and __fish_seen_subcommand_from edit" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand comments; and __fish_seen_subcommand_from edit" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and not __fish_seen_subcommand_from list add download" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and not __fish_seen_subcommand_from list add download" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and not __fish_seen_subcommand_from list add download" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and not __fish_seen_subcommand_from list add download" -f -a "list" -d 'File metadata'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and not __fish_seen_subcommand_from list add download" -f -a "add" -d 'Upload files to an item'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and not __fish_seen_subcommand_from list add download" -f -a "download" -d 'Download a file'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from list" -l limit -r
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from list" -l cursor -r
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from list" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from list" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from add" -l request-id -d 'Use this UUID as the operation ID and Idempotency-Key (for callers that keep their own records)' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from add" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from add" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from download" -s o -l output -r -F
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from download" -l force -d 'Replace an existing file'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from download" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from download" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand attachments; and __fish_seen_subcommand_from download" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand projects; and not __fish_seen_subcommand_from list show" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand projects; and not __fish_seen_subcommand_from list show" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand projects; and not __fish_seen_subcommand_from list show" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand projects; and not __fish_seen_subcommand_from list show" -f -a "list" -d 'Projects you can reach with this token'
complete -c taskhub -n "__fish_taskhub_using_subcommand projects; and not __fish_seen_subcommand_from list show" -f -a "show" -d 'One project: labels, members, enums and limits'
complete -c taskhub -n "__fish_taskhub_using_subcommand projects; and __fish_seen_subcommand_from list" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand projects; and __fish_seen_subcommand_from list" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand projects; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand projects; and __fish_seen_subcommand_from show" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand projects; and __fish_seen_subcommand_from show" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand projects; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand pending; and not __fish_seen_subcommand_from discard" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand pending; and not __fish_seen_subcommand_from discard" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand pending; and not __fish_seen_subcommand_from discard" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand pending; and not __fish_seen_subcommand_from discard" -f -a "discard" -d 'Forget a pending write after checking it yourself'
complete -c taskhub -n "__fish_taskhub_using_subcommand pending; and __fish_seen_subcommand_from discard" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand pending; and __fish_seen_subcommand_from discard" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand pending; and __fish_seen_subcommand_from discard" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand retry" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand retry" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand retry" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and not __fish_seen_subcommand_from login status logout" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and not __fish_seen_subcommand_from login status logout" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and not __fish_seen_subcommand_from login status logout" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and not __fish_seen_subcommand_from login status logout" -f -a "login" -d 'Save an API token. Reads it from stdin with --with-token, or prompts on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and not __fish_seen_subcommand_from login status logout" -f -a "status" -d 'Who you are logged in as, and where the credential came from'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and not __fish_seen_subcommand_from login status logout" -f -a "logout" -d 'Delete the saved credentials'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and __fish_seen_subcommand_from login" -l origin -d 'The TaskHub server. HTTPS, or http://127.0.0.1 for local development' -r
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and __fish_seen_subcommand_from login" -l with-token -d 'Read the token from stdin'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and __fish_seen_subcommand_from login" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and __fish_seen_subcommand_from login" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and __fish_seen_subcommand_from login" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and __fish_seen_subcommand_from status" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and __fish_seen_subcommand_from status" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and __fish_seen_subcommand_from status" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and __fish_seen_subcommand_from logout" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and __fish_seen_subcommand_from logout" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand auth; and __fish_seen_subcommand_from logout" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand guide" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand guide" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand guide" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand skill; and not __fish_seen_subcommand_from install" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand skill; and not __fish_seen_subcommand_from install" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand skill; and not __fish_seen_subcommand_from install" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand skill; and not __fish_seen_subcommand_from install" -f -a "install" -d 'Write the embedded skill into Claude Code\'s and/or Codex\'s skill directory'
complete -c taskhub -n "__fish_taskhub_using_subcommand skill; and __fish_seen_subcommand_from install" -l client -r -f -a "claude\t''
codex\t''
all\t''"
complete -c taskhub -n "__fish_taskhub_using_subcommand skill; and __fish_seen_subcommand_from install" -l project -d 'Install into this repository instead of your home directory'
complete -c taskhub -n "__fish_taskhub_using_subcommand skill; and __fish_seen_subcommand_from install" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand skill; and __fish_seen_subcommand_from install" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand skill; and __fish_seen_subcommand_from install" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand mcp" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand mcp" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand mcp" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand completions" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand completions" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand completions" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand version" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand version" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand version" -s h -l help -d 'Print help'
complete -c taskhub -n "__fish_taskhub_using_subcommand man" -l json -d 'Print the JSON envelope even on a terminal'
complete -c taskhub -n "__fish_taskhub_using_subcommand man" -l human -d 'Print human-readable output even when piped'
complete -c taskhub -n "__fish_taskhub_using_subcommand man" -s h -l help -d 'Print help'
