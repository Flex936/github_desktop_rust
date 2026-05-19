mod app;
mod git_engine;
mod models;

use std::path::PathBuf;

use app::{App, Message, update, view};
use iced::Task;

fn main() -> iced::Result {
    // In iced 0.14, `iced::application` takes the boot function first,
    // then the update function, then the view function.
    iced::application(
        || {
            let repo_path = PathBuf::from("/home/alext/Documents/GitHub/github_desktop_rust");
            let initial_task = Task::done(Message::LoadRepository(repo_path));
            (App::new(), initial_task)
        },
        update,
        view,
    )
    .title("GitHub Desktop (Rust)")
    .run()
}
