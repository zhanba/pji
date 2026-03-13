use clap::{Args, Parser, Subcommand};
use pji::app::PjiApp;
use pji::util::is_interactive_mode;

/// A CLI for managing, finding, and opening Git repositories.
#[derive(Debug, Parser)]
#[command(name = "pji")]
#[command(version, about = "A CLI for managing, finding, and opening Git repositories.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Optional query for fuzzy search (shorthand for 'pji find <query>')
    query: Option<String>,

    /// Disable interactive TUI prompts (auto-detected when not a TTY)
    #[arg(short = 'n', long = "no-interactive", global = true)]
    no_interactive: bool,

    /// Auto-confirm destructive prompts (useful in scripts)
    #[arg(short = 'y', long = "yes", global = true)]
    yes: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Configure the root directory for your repositories
    Config {
        /// Root path to add non-interactively (skips the interactive prompt)
        #[arg(long)]
        root: Option<String>,
    },
    /// Add a git repository
    Add {
        /// git repository url
        git: String,
    },
    /// Remove a git repository
    Remove {
        /// git repository url
        git: String,
    },
    /// List all git repositories
    List {
        #[arg(short, long)]
        long: bool,
    },
    /// Fuzzy search for git repositories
    Find { query: Option<String> },
    /// Scan all git repositories in the root directory and save their information
    Scan,
    /// Clean pji metadata and configuration
    Clean,
    /// Open a git repository page (e.g., home, PR, issue) in the browser
    Open(OpenArgs),
    /// Manage git worktrees
    #[command(alias = "wt")]
    Worktree(WorktreeArgs),
}

#[derive(Debug, Args)]
#[command(flatten_help = true)]
struct WorktreeArgs {
    #[command(subcommand)]
    command: Option<WorktreeCommands>,
}

#[derive(Debug, Subcommand)]
enum WorktreeCommands {
    /// List worktrees for current or selected repository
    List {
        /// Optional query to filter/select repository
        query: Option<String>,
    },
    /// Fuzzy select and switch to a worktree
    #[command(alias = "sw")]
    Switch {
        /// Optional query to filter worktrees
        query: Option<String>,
    },
    /// Create a new worktree
    Add {
        /// Branch name to create or checkout (required in non-interactive mode)
        #[arg(short, long)]
        branch: Option<String>,
        /// Create a new branch (use with --branch)
        #[arg(long)]
        new_branch: bool,
        /// Base branch for the new branch (use with --new-branch)
        #[arg(long)]
        base: Option<String>,
        /// Path for the new worktree (defaults to <repo>/../<repo>.worktrees/<branch>)
        #[arg(long)]
        path: Option<String>,
    },
    /// Remove a worktree
    Remove {
        /// Path or name of the worktree to remove
        worktree: Option<String>,
        /// Force removal even if worktree is dirty
        #[arg(short, long)]
        force: bool,
    },
    /// Clean up stale worktree information
    Prune,
}

#[derive(Debug, Args)]
#[command(flatten_help = true)]
struct OpenArgs {
    #[command(subcommand)]
    command: Option<OpenCommands>,

    #[command(flatten)]
    home: OpenHomeArgs,
}

#[derive(Debug, Subcommand)]
enum OpenCommands {
    /// open a git repository home page in browser
    Home(OpenHomeArgs),
    /// open a git repository pull request page in browser
    PR {
        /// pull request number
        number: Option<u32>,
    },
    /// open a git repository issue page in browser
    Issue {
        /// issue number
        number: Option<u32>,
    },
}

#[derive(Debug, Args)]
struct OpenHomeArgs {
    /// git repository name. If it's empty pji will open repository based on current directory
    url: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let interactive = is_interactive_mode(cli.no_interactive);
    let auto_yes = cli.yes;

    match cli.command {
        Some(command) => match command {
            Commands::Config { root } => {
                PjiApp::new(interactive, auto_yes).start_config(root.as_deref());
            }
            Commands::Add { git } => {
                PjiApp::new(interactive, auto_yes).add(git.as_str());
            }
            Commands::Remove { git } => {
                PjiApp::new(interactive, auto_yes).remove(git.as_str());
            }
            Commands::List { long } => {
                PjiApp::new(interactive, auto_yes).list(long);
            }
            Commands::Find { query } => {
                PjiApp::new(interactive, auto_yes).find(query.as_deref().unwrap_or(""));
            }
            Commands::Scan => {
                PjiApp::new(interactive, auto_yes).scan();
            }
            Commands::Clean => PjiApp::clean(),
            Commands::Open(args) => {
                let open_cmd = args.command.unwrap_or(OpenCommands::Home(args.home));
                match open_cmd {
                    OpenCommands::Home(home) => {
                        PjiApp::new(interactive, auto_yes).open_home(home.url);
                    }
                    OpenCommands::PR { number } => {
                        PjiApp::new(interactive, auto_yes).open_pr(number);
                    }
                    OpenCommands::Issue { number } => {
                        PjiApp::new(interactive, auto_yes).open_issue(number);
                    }
                }
            }
            Commands::Worktree(args) => {
                let wt_cmd = args
                    .command
                    .unwrap_or(WorktreeCommands::Switch { query: None });
                match wt_cmd {
                    WorktreeCommands::List { query } => {
                        PjiApp::new(interactive, auto_yes).worktree_list(query);
                    }
                    WorktreeCommands::Switch { query } => {
                        PjiApp::new(interactive, auto_yes).worktree_switch(query);
                    }
                    WorktreeCommands::Add {
                        branch,
                        new_branch,
                        base,
                        path,
                    } => {
                        PjiApp::new(interactive, auto_yes)
                            .worktree_add(branch, new_branch, base, path);
                    }
                    WorktreeCommands::Remove { worktree, force } => {
                        PjiApp::new(interactive, auto_yes).worktree_remove(worktree, force);
                    }
                    WorktreeCommands::Prune => {
                        PjiApp::new(interactive, auto_yes).worktree_prune();
                    }
                }
            }
        },
        None => {
            // Default to find command when no subcommand is provided
            PjiApp::new(interactive, auto_yes).find(cli.query.as_deref().unwrap_or(""));
        }
    }
}
