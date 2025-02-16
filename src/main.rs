use clap::{Args, Parser, Subcommand};
use pj::app::PJApp;

/// Pj provide a tree structure to manage your git projects.
#[derive(Debug, Parser)]
#[command(name = "pj")]
#[command(version, about = "pj provide a tree structure to manage your git projects.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// init pj: select root dir, create a new config file at `~/.pj/config.toml`
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
    ///  scan all git repo in root dir and write repo info into `~/.pj/repo.toml`
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
    MergeRequest {
        #[arg(short, long)]
        number: Option<u32>,
    },
    Issue {
        #[arg(short, long)]
        number: Option<u32>,
    },
}

#[derive(Debug, Args)]
struct OpenHomeArgs {
    #[arg(short, long)]
    url: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(command) => {
            match command {
                Commands::Init => {
                    PJApp::init();
                }
                Commands::Add { git } => {
                    PJApp::new().add(git.as_str());
                }
                Commands::Remove { git } => {
                    PJApp::new().remove(git.as_str());
                }
                Commands::List => {
                    PJApp::new().list();
                }
                Commands::Find { query } => {
                    PJApp::new().find(query.as_deref().unwrap_or(""));
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
                            // Handle the "home" subcommand
                            println!("Opening git project home: {:?}", home);
                        }
                        OpenCommands::MergeRequest { number } => {
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
