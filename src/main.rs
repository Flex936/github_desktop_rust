mod git_engine;
mod models;

use git_engine::GitEngine;
use std::path::Path;

fn main() {
    let repo_path = Path::new("/home/alext/Documents/GitHub/github_desktop_rust");

    // 1. Open with our custom engine
    match GitEngine::open_repository(repo_path, 1) {
        Ok(repo) => {
            println!("Successfully opened repository: {}", repo.name);
            println!("Path on disk: {:?}", repo.path);

            // 2. Open a temporary native git2 instance to test our functions
            if let Ok(native_repo) = git2::Repository::open(repo_path) {
                // --- TEST 1: Working Directory Status ---
                println!("\n--- Checking Working Directory Status ---");
                match GitEngine::get_working_directory_status(&native_repo) {
                    Ok(status) => {
                        println!("Found {} changed files.", status.files.len());
                        for file_change in status.files {
                            println!(
                                "  [{:?}] {}",
                                file_change.file_change.status.kind(),
                                file_change.file_change.path.display()
                            );
                        }
                    }
                    Err(e) => eprintln!("Failed to fetch status: {}", e),
                }

                // --- TEST 2: Commit History ---
                // Notice how this is now INSIDE the `if let` block, so `native_repo` exists!
                println!("\n--- Commit History (last 20) ---");
                match GitEngine::get_commit_history(&native_repo, 20) {
                    Ok(commits) => {
                        println!("Loaded {} commit(s).", commits.len());
                        for c in &commits {
                            let merge_marker = if c.is_merge_commit() { " [merge]" } else { "" };
                            println!(
                                "  {} {} — {}{merge_marker}",
                                c.short_sha,
                                c.author.date.format("%Y-%m-%d"),
                                c.summary,
                            );
                        }
                    }
                    Err(e) => eprintln!("Failed to load history: {}", e),
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open repository: {}", e);
        }
    }
}
