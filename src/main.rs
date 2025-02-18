use clap::{Args, Parser, Subcommand};
use pji::app::PjiApp;

/// pji provide a tree structure to manage your git projects.
#[derive(Debug, Parser)]
#[command(name = "pji")]
#[command(version, about = "pji provide a tree structure to manage your git projects.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// config root directory for your repos
    Config,
    /// add a git project
    Add {
        /// git project url
        git: String,
    },
    /// remove a git project
    Remove {
        /// git project url
        git: String,
    },
    /// list all git projects
    List,
    /// fuzz search git projects
    Find { query: Option<String> },
    /// scan all git repo in root dir and save repo info
    Scan,
    /// download all missing repos
    Pull,
    /// open a git project home page in browser
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
    /// open a git project home page in browser
    Home(OpenHomeArgs),
    /// open a git project pull request page in browser
    PR {
        /// pull request number
        number: Option<u32>,
    },
    /// open a git project issue page in browser
    Issue {
        /// issue number
        number: Option<u32>,
    },
}

#[derive(Debug, Args)]
struct OpenHomeArgs {
    /// git project name. If it's empty pji will open project based on current directory
    url: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(command) => {
            match command {
                Commands::Config => {
                    PjiApp::new().start_config();
                }
                Commands::Add { git } => {
                    PjiApp::new().add(git.as_str());
                }
                Commands::Remove { git } => {
                    PjiApp::new().remove(git.as_str());
                }
                Commands::List => {
                    PjiApp::new().list();
                }
                Commands::Find { query } => {
                    PjiApp::new().find(query.as_deref().unwrap_or(""));
                }
                Commands::Scan => {
                    // Handle the "update" command
                    println!("Updating git projects...");
                }
                Commands::Pull => {
                    // Handle the "pull" command
                    println!("Pulling git projects...");
                }
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
            }
        }
        None => {
            // Handle the default case
            println!("No command provided");
        }
    }
}
