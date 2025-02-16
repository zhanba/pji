use clap::{Args, Parser, Subcommand};
use pji::app::PjiApp;

/// Pj provide a tree structure to manage your git projects.
#[derive(Debug, Parser)]
#[command(name = "pji")]
#[command(version, about = "pji provide a tree structure to manage your git projects.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// init pji: select root dir, create a new config file at `~/.pji/config.toml`
    Init,
    /// add a git project
    Add {
        /// git project to add
        git: String,
    },
    /// remove a git project
    Remove {
        /// git project to remove
        git: String,
    },
    /// list all git projects
    List,
    /// fuzz search git project
    Find { query: Option<String> },
    ///  scan all git repo in root dir and write repo info into `~/.pji/repo.toml`
    Update,
    /// check root dir and download all missing repos
    Pull,
    /// open a git project page in browser
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
    Home(OpenHomeArgs),
    PR { number: Option<u32> },
    Issue { number: Option<u32> },
}

#[derive(Debug, Args)]
struct OpenHomeArgs {
    url: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(command) => {
            match command {
                Commands::Init => {
                    PjiApp::init();
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
                Commands::Update => {
                    // Handle the "update" command
                    println!("Updating git projects...");
                }
                Commands::Pull => {
                    // Handle the "pull" command
                    println!("Pulling git projects...");
                }
                Commands::Open(args) => {
                    // Handle the "open" command
                    println!("Opening git project: {:?}", args.home);

                    let open_cmd = args.command.unwrap_or(OpenCommands::Home(args.home));
                    match open_cmd {
                        OpenCommands::Home(home) => {
                            PjiApp::new().open_home(home.url);
                        }
                        OpenCommands::PR { number } => {
                            // Handle the "merge_request" subcommand
                            println!("Opening merge request: {:?}", number);
                        }
                        OpenCommands::Issue { number } => {
                            // Handle the "issue" subcommand
                            println!("Opening issue: {:?}", number);
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

    // Continued program logic goes here...
}
