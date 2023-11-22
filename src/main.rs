use clap::{Parser, Subcommand};
use dialoguer::Confirm;
use git2::{BranchType, Config, Repository};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::process::{Command, Stdio};

fn get_current_user_info() -> (String, String) {
    let cfg = Config::open_default().unwrap();

    // Retrieve the user's name and email from the configuration
    let user_name = cfg
        .get_string("user.name")
        .unwrap_or_else(|_| "Unknown User".to_string());
    let user_email = cfg
        .get_string("user.email")
        .unwrap_or_else(|_| "Unknown Email".to_string());

    (user_name, user_email)
}

fn get_repo(path: String) -> Repository {
    let repo = Repository::open(path).unwrap();
    repo
}

fn get_remote_branches(repo: Repository) -> Vec<(String, String, String)> {
    // List all remote branches
    let branches = repo.branches(Some(BranchType::Remote)).unwrap();

    let mut branch_info = Vec::new();
    for branch_result in branches {
        let (branch, _) = branch_result.unwrap();
        let branch_name = if let Some(name) = branch.name().unwrap() {
            // Exclude the remote part from the branch name
            name.strip_prefix("origin/").unwrap_or(name)
        } else {
            continue;
        };

        // Get the last commit of the branch
        let commit = branch.get().peel_to_commit().unwrap();
        let author = commit.author();
        let author_name = author.name().unwrap_or("Unknown");
        let author_email = author.email().unwrap_or("Unknown");
        branch_info.push((
            branch_name.to_string(),
            author_name.to_string(),
            author_email.to_string(),
        ));
    }

    branch_info
}

fn delete_branch(repo_path: &Path, branch_name: String) -> Result<(), Box<dyn Error>> {
    // Run the Git command in the specified directory
    let output = Command::new("git")
        .args(["push", "origin", "--delete", branch_name.as_str()])
        .current_dir(repo_path) // Set the current directory for the command
        .stdout(Stdio::piped()) // Capture standard output
        .stderr(Stdio::piped()) // Capture standard error
        .output()?;

    if !output.status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to delete branch",
        )));
    }

    Ok(())
}

fn stats(repo_location: &str) -> Result<(), Box<dyn Error>> {
    let repo = get_repo(repo_location.to_string());
    let branches = get_remote_branches(repo);
    let mut branch_count = HashMap::new();
    let total_branches = branches.len();
    for (_, _, email) in branches {
        *branch_count.entry(email).or_insert(0) += 1;
    }

    println!("Branches per user:");
    for (email, count) in branch_count {
        println!("{}: {}", email, count);
    }
    println!(
        "=========================\n Total Remote Branches: {}",
        total_branches
    );
    Ok(())
}

fn cleanup(repo_location: &str, filter_email: &str) -> Result<(), Box<dyn Error>> {
    let repo_path = Path::new(repo_location);
    let repo = get_repo(repo_location.to_string());
    let branches = get_remote_branches(repo);
    for (branch_name, _, email) in branches {
        if email == filter_email {
            // Ask user if they want to delete this branch
            if Confirm::new()
                .with_prompt(format!(
                    "Do you want to delete the branch '{}'?",
                    branch_name
                ))
                .interact()?
            {
                delete_branch(repo_path, branch_name)?;
            }
        }
    }

    Ok(())
}

fn list(repo_location: &str, filter_email: &str) -> Result<(), Box<dyn Error>> {
    let repo = get_repo(repo_location.to_string());
    let branches = get_remote_branches(repo);
    for (branch_name, _, email) in branches {
        if email == filter_email {
            println!("{}", branch_name);
        }
    }

    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set the location of the repository
    #[arg(short, long, value_name = ".")]
    location: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Stats about the current repo
    Stats {},
    List {
        /// use the email to filter the branches by author
        #[arg(short, long)]
        email: Option<String>,
    },
    /// Delete remote branches that are no more needed
    Cleanup {
        /// use the email to filter the branches by author
        #[arg(short, long)]
        email: Option<String>,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let location = cli.location.unwrap_or(".".to_string());
    let (_, user_email) = get_current_user_info();
    match &cli.command {
        Some(Commands::Stats {}) => {
            return stats(location.as_str());
        }
        Some(Commands::Cleanup { email }) => {
            let filter_email = email.as_deref().unwrap_or(user_email.as_str());
            println!("filter_email: {}\n==========================", filter_email);
            return cleanup(location.as_str(), filter_email);
        }
        Some(Commands::List { email }) => {
            let filter_email = email.as_deref().unwrap_or(user_email.as_str());
            println!("filter_email: {}\n==========================", filter_email);
            return list(location.as_str(), filter_email);
        }
        None => {
            println!(
                "Please provide a subcommand, use help sub command to see the available options"
            );
            Ok(())
        }
    }
}
