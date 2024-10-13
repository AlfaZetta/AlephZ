use clap::{Parser, Subcommand};
use git2::Repository;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Command-line arguments for the script
#[derive(Parser)]
struct Args {
    #[clap(default_value = ".")]
    path: String,

    #[clap(subcommand)]
    action: Option<Action>,
}

/// Subcommands for the script
#[derive(Subcommand)]
enum Action {
    /// Just pull all repos
    Pull,
    /// Pull and update dependencies
    Update,
}

fn main() {
    println!("MetaZeta");

    let args = Args::parse();
    let base_path = Path::new(&args.path);

    // Walk the directory tree
    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if is_git_repo(path) {
            let full_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            println!("Found repository: {:?}", full_path);

            if let Some(action) = &args.action {
                match action {
                    Action::Pull => pull_repo(&full_path),
                    Action::Update => {
                        pull_repo(&full_path);
                        update_dependencies(&full_path);
                    }
                }
            } else {
                pull_repo(&full_path);
                update_dependencies(&full_path);
            }
        }
    }
}

/// Checks if a directory is a Git repository
fn is_git_repo(path: &Path) -> bool {
    Repository::open(path).is_ok()
}

/// Pulls the latest changes in the repository
fn pull_repo(path: &Path) {
    println!("Pulling repository at {:?}", path);
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("pull")
        .output()
        .expect("Failed to execute git pull");

    if !output.status.success() {
        eprintln!(
            "Failed to pull {:?}: {}",
            path,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Updates dependencies based on lockfiles
fn update_dependencies(path: &Path) {
    println!("Updating dependencies for {:?}", path);

    let mut updated = false;

    // Check for Node.js lockfiles
    if path.join("package-lock.json").exists() {
        println!(
            "Detected npm dependencies in {:?}",
            path.join("package-lock.json")
        );
        run_command(path, "npm", &["install"]);
        updated = true;
    } else if path.join("yarn.lock").exists() {
        println!("Detected Yarn dependencies in {:?}", path.join("yarn.lock"));
        run_command(path, "yarn", &["install"]);
        updated = true;
    } else if path.join("pnpm-lock.yaml").exists() {
        println!(
            "Detected pnpm dependencies in {:?}",
            path.join("pnpm-lock.yaml")
        );
        run_command(path, "pnpm", &["install"]);
        updated = true;
    }

    // Check for Rust lockfile
    if path.join("Cargo.lock").exists() {
        println!(
            "Detected Rust dependencies in {:?}",
            path.join("Cargo.lock")
        );
        run_command(path, "cargo", &["update"]);
        updated = true;
    }

    // Check for Python lockfiles
    if path.join("Pipfile").exists() {
        println!("Detected Pipenv dependencies in {:?}", path.join("Pipfile"));
        run_command(path, "pipenv", &["install"]);
        updated = true;
    } else if path.join("poetry.lock").exists() {
        println!(
            "Detected Poetry dependencies in {:?}",
            path.join("poetry.lock")
        );
        run_command(path, "poetry", &["update"]);
        updated = true;
    } else if path.join("requirements.txt").exists() {
        println!(
            "Detected pip dependencies in {:?}",
            path.join("requirements.txt")
        );
        run_command(path, "pip", &["install", "-r", "requirements.txt"]);
        updated = true;
    }

    if !updated {
        println!("No recognized dependency manager found for {:?}", path);
    }
}

/// Helper to run a command in a given directory
fn run_command(path: &Path, command: &str, args: &[&str]) {
    let output = Command::new(command)
        .args(args)
        .current_dir(path)
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        eprintln!(
            "Failed to run {} in {:?}: {}",
            command,
            path,
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        println!("Successfully ran {} in {:?}", command, path);
    }
}
