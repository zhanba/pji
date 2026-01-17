use clap::{Args, Parser, Subcommand};
use pji::app::PjiApp;

/// A CLI for managing, finding, and opening Git repositories.
#[derive(Debug, Parser)]
#[command(name = "pji")]
#[command(version, about = "A CLI for managing, finding, and opening Git repositories.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Optional query for fuzzy search (shorthand for 'pji find <query>')
    query: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Configure the root directory for your repositories
    Config,
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
        /// Branch name for the worktree
        branch: String,
        /// Create a new branch
        #[arg(short = 'b', long)]
        create_branch: bool,
        /// Custom path for the worktree (defaults to {repo}.worktrees/{branch})
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

    match cli.command {
        Some(command) => match command {
            Commands::Config => {
                PjiApp::new().start_config();
            }
            Commands::Add { git } => {
                PjiApp::new().add(git.as_str());
            }
            Commands::Remove { git } => {
                PjiApp::new().remove(git.as_str());
            }
            Commands::List { long } => {
                PjiApp::new().list(long);
            }
            Commands::Find { query } => {
                PjiApp::new().find(query.as_deref().unwrap_or(""));
            }
            Commands::Scan => {
                PjiApp::new().scan();
            }
            Commands::Clean => PjiApp::clean(),
            Commands::Open(args) => {
                let open_cmd = args.command.unwrap_or(OpenCommands::Home(args.home));
                match open_cmd {
                    OpenCommands::Home(home) => {
                        PjiApp::new().open_home(home.url);
                    }
                    OpenCommands::PR { number } => {
                        PjiApp::new().open_pr(number);
                    }
                    OpenCommands::Issue { number } => {
                        PjiApp::new().open_issue(number);
                    }
                }
            }
            Commands::Worktree(args) => {
                let wt_cmd = args.command.unwrap_or(WorktreeCommands::Switch { query: None });
                match wt_cmd {
                    WorktreeCommands::List { query } => {
                        PjiApp::new().worktree_list(query);
                    }
                    WorktreeCommands::Switch { query } => {
                        PjiApp::new().worktree_switch(query);
                    }
                    WorktreeCommands::Add {
                        branch,
                        create_branch,
                        path,
                    } => {
                        PjiApp::new().worktree_add(&branch, create_branch, path);
                    }
                    WorktreeCommands::Remove { worktree, force } => {
                        PjiApp::new().worktree_remove(worktree, force);
                    }
                    WorktreeCommands::Prune => {
                        PjiApp::new().worktree_prune();
                    }
                }
            }
        },
        None => {
            // Default to find command when no subcommand is provided
            PjiApp::new().find(cli.query.as_deref().unwrap_or(""));
        }
    }
}
