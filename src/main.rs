use clap::{Args, Parser, Subcommand};
use dialoguer::console::{user_attended, user_attended_stderr};
use std::io::{self, IsTerminal};
use std::path::PathBuf;

mod app;

use app::{AppOptions, PjiApp};

/// A CLI for managing, finding, and opening Git repositories.
#[derive(Debug, Parser)]
#[command(name = "pji")]
#[command(version, about = "A CLI for managing, finding, and opening Git repositories.", long_about = None)]
struct Cli {
    /// Force non-interactive mode; also auto-enabled when not attached to a terminal
    #[arg(short = 'n', long, global = true)]
    non_interactive: bool,

    /// Select a root directory without prompting
    #[arg(long, global = true, value_name = "DIR")]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Optional query for fuzzy search (shorthand for 'pji find <query>')
    query: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Configure the root directory for your repositories
    Config {
        /// Root directory to add
        root: Option<PathBuf>,
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
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
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
        /// Branch to check out or create
        branch: Option<String>,
        /// Worktree path. Defaults to the standard pji worktree path.
        #[arg(short, long, value_name = "PATH")]
        path: Option<PathBuf>,
        /// Base branch when creating a new branch
        #[arg(short = 'b', long, value_name = "BRANCH")]
        base: Option<String>,
        /// Create a new branch
        #[arg(long)]
        new_branch: bool,
    },
    /// Remove a worktree
    Remove {
        /// Path or name of the worktree to remove
        worktree: Option<String>,
        /// Force removal even if worktree is dirty
        #[arg(short, long)]
        force: bool,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let app_options = AppOptions {
        interactive: !cli.non_interactive && terminal_is_interactive(),
        root: cli.root,
    };
    let mut app = PjiApp::new(app_options)?;

    match cli.command {
        Some(command) => match command {
            Commands::Config { root } => {
                app.start_config(root)?;
            }
            Commands::Add { git } => {
                app.add(git.as_str())?;
            }
            Commands::Remove { git, yes } => {
                app.remove(git.as_str(), yes)?;
            }
            Commands::List { long } => {
                app.list(long)?;
            }
            Commands::Find { query } => {
                app.find(query.as_deref().unwrap_or(""))?;
            }
            Commands::Scan => {
                app.scan()?;
            }
            Commands::Clean => PjiApp::clean()?,
            Commands::Open(args) => {
                let open_cmd = args.command.unwrap_or(OpenCommands::Home(args.home));
                match open_cmd {
                    OpenCommands::Home(home) => {
                        app.open_home(home.url)?;
                    }
                    OpenCommands::PR { number } => {
                        app.open_pr(number)?;
                    }
                    OpenCommands::Issue { number } => {
                        app.open_issue(number)?;
                    }
                }
            }
            Commands::Worktree(args) => {
                let wt_cmd = args
                    .command
                    .unwrap_or(WorktreeCommands::Switch { query: None });
                match wt_cmd {
                    WorktreeCommands::List { query } => {
                        app.worktree_list(query)?;
                    }
                    WorktreeCommands::Switch { query } => {
                        app.worktree_switch(query)?;
                    }
                    WorktreeCommands::Add {
                        branch,
                        path,
                        base,
                        new_branch,
                    } => {
                        app.worktree_add(branch, path, base, new_branch)?;
                    }
                    WorktreeCommands::Remove {
                        worktree,
                        force,
                        yes,
                    } => {
                        app.worktree_remove(worktree, force, yes)?;
                    }
                    WorktreeCommands::Prune => {
                        app.worktree_prune()?;
                    }
                }
            }
        },
        None => {
            // Default to find command when no subcommand is provided
            app.find(cli.query.as_deref().unwrap_or(""))?;
        }
    }

    Ok(())
}

fn terminal_is_interactive() -> bool {
    // A prompt-driven CLI needs all three streams attached: stdin for input,
    // stdout for shell handoff/output, and stderr for dialoguer prompts.
    io::stdin().is_terminal() && user_attended() && user_attended_stderr()
}
