use std::path::Path;
use dirs::{self, home_dir};
use std::io::{self, Error};
use std::fs::{self, File};

mod constants;
mod uni;

use uni::{todo::Todo, task::Task};

fn main() -> io::Result<()> {
    uni::todo::clear_screen();
    let mut dir = match home_dir() {
        Some(dir) => dir,
        None => return Err(Error::other("Could not find a home directory")),
    };

    dir.push(Path::new(constants::TABLE_PATH));

    // prologue
    if !dir.exists() {
        println!("Creating dir");
        fs::create_dir(&dir)?;
        println!("Created dir");
    }

    dir.push(Path::new(constants::TABLE_NAME));
    if !dir.exists() {
        File::create(&dir)?;
    }

    println!("Loading tasks from {}...", constants::TABLE_NAME);
    let tasks = Task::read_tasks(Path::new(&dir))?;
    let mut app = Todo::new(tasks, dir.to_owned());

    app.panic_lookup();
    app.run();
    app.save()?;

    Ok(())
}
