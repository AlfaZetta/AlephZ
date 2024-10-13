use clap::{Parser, Subcommand};
use git2::Repository;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use walkdir::WalkDir;
use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::sync::mpsc;
use futures::stream::{self, StreamExt};

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


#[tokio::main]
async fn main() {
    println!("MetaZeta");

    let args = Args::parse();
    let base_path = Path::new(&args.path);


    process_repositories(base_path, &args.action).await;
}


async fn process_repositories(base_path: &Path, action: &Option<Action>) {
    let (tx, mut rx) = mpsc::channel(32);

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {



        let path = entry.path().to_owned();
        if is_git_repo(&path) {
            let tx = tx.clone();
            let action = action.clone();
            tokio::spawn(async move {
                let relative_path = path.strip_prefix(base_path).unwrap_or(&path);
                process_repository(&path, &action, relative_path).await;
                tx.send(()).await.unwrap();
            });
        }
    }

    drop(tx);

    while rx.recv().await.is_some() {}
}


async fn process_repository(path: &Path, action: &Option<Action>, relative_path: &Path) {
    let full_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    println!("Found repository: {:?}", relative_path);

    match action {

        Some(Action::Pull) => pull_repo(&full_path, relative_path).await,
        Some(Action::Update) => {


            pull_repo(&full_path, relative_path).await;
            update_dependencies(&full_path, relative_path).await;
        }
        None => {


            pull_repo(&full_path, relative_path).await;
            update_dependencies(&full_path, relative_path).await;
        }
    }
}

/// Checks if a directory is a Git repository
fn is_git_repo(path: &Path) -> bool {
    Repository::open(path).is_ok()
}

/// Pulls the latest changes in the repository



async fn pull_repo(path: &Path, relative_path: &Path) {
    println!("Pulling repository at {:?}", relative_path);
    run_command(path, "git", &["pull"], "Git", relative_path).await;
}

/// Updates dependencies based on lockfiles


async fn update_dependencies(path: &Path, relative_path: &Path) {
    println!("Updating dependencies for {:?}", relative_path);

    let mut updated = false;

    // Check for Node.js lockfiles
    if path.join("package-lock.json").exists() {
        println!(
            "Detected npm dependencies in {:?}",

            relative_path.join("package-lock.json")
        );


        run_command(path, "npm", &["install"], "npm", relative_path).await;
        updated = true;
    } else if path.join("yarn.lock").exists() {


        println!("Detected Yarn dependencies in {:?}", relative_path.join("yarn.lock"));

        run_command(path, "yarn", &["install"], "Yarn", relative_path).await;
        updated = true;
    } else if path.join("pnpm-lock.yaml").exists() {
        println!(
            "Detected pnpm dependencies in {:?}",

            relative_path.join("pnpm-lock.yaml")
        );


        run_command(path, "pnpm", &["install"], "pnpm", relative_path).await;
        updated = true;
    }

    // Check for Rust lockfile
    if path.join("Cargo.lock").exists() {
        println!(
            "Detected Rust dependencies in {:?}",

            relative_path.join("Cargo.lock")
        );


        run_command(path, "cargo", &["update"], "Cargo", relative_path).await;
        updated = true;
    }

    // Check for Python lockfiles
    if path.join("Pipfile").exists() {


        println!("Detected Pipenv dependencies in {:?}", relative_path.join("Pipfile"));

        run_command(path, "pipenv", &["install"], "Pipenv", relative_path).await;
        updated = true;
    } else if path.join("poetry.lock").exists() {
        println!(
            "Detected Poetry dependencies in {:?}",

            relative_path.join("poetry.lock")
        );


        run_command(path, "poetry", &["update"], "Poetry", relative_path).await;
        updated = true;
    } else if path.join("requirements.txt").exists() {
        println!(
            "Detected pip dependencies in {:?}",

            relative_path.join("requirements.txt")
        );


        run_command(path, "pip", &["install", "-r", "requirements.txt"], "pip", relative_path).await;
        updated = true;
    }

    if !updated {

        println!("No recognized dependency manager found for {:?}", relative_path);
    }
}

/// Helper to run a command in a given directory

async fn run_command(path: &Path, command: &str, args: &[&str], prefix: &str, relative_path: &Path) {
    let mut child = Command::new(command)
        .args(args)
        .current_dir(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let mut stderr = StandardStream::stderr(ColorChoice::Always);

    if let Some(stdout_handle) = child.stdout.take() {
        let prefix = prefix.to_string();


        let relative_path = relative_path.to_path_buf();
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout_handle);
            let mut line = String::new();


            while tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut line).await.unwrap() > 0 {
                print_with_prefix(&mut stdout, &prefix, &line, Color::Green, &relative_path).unwrap();
                line.clear();
            }
        });
    }

    if let Some(stderr_handle) = child.stderr.take() {
        let prefix = prefix.to_string();


        let relative_path = relative_path.to_path_buf();
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr_handle);
            let mut line = String::new();


            while tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut line).await.unwrap() > 0 {
                print_with_prefix(&mut stderr, &prefix, &line, Color::Red, &relative_path).unwrap();
                line.clear();
            }
        });
    }


    let status = child.wait().await.expect("Failed to wait on child process");

    if !status.success() {

        eprintln!("Failed to run {} in {:?}", command, relative_path);
    } else {

        println!("Successfully ran {} in {:?}", command, relative_path);
    }
}


fn print_with_prefix(stream: &mut StandardStream, prefix: &str, message: &str, color: Color, relative_path: &Path) -> io::Result<()> {
    stream.set_color(ColorSpec::new().set_fg(Some(color)))?;

    write!(stream, "[{}][{}] ", relative_path.display(), prefix)?;
    stream.reset()?;
    write!(stream, "{}", message)?;
    stream.flush()
}
