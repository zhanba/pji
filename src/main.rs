use clap::{Args, Parser, Subcommand};
use pji::app::PjiApp;

/// A CLI for managing, finding, and opening Git repositories.
#[derive(Debug, Parser)]
#[command(name = "pji")]
#[command(version, about = "A CLI for managing, finding, and opening Git repositories.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
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
        },
        None => {
            // Handle the default case
            println!("No command provided");
        }
    }
}
