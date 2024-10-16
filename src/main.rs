use std::fs::{self, File};
use std::io::{self, stdin, stdout, BufRead, Error, Write};
use std::path::Path;


mod constants;
mod readers;
mod storages;
mod ui;
mod uni;

use ui::{
    app::App,
    panes::{DescriptionPane, TasksPane},
};

use readers::EditorTaskReader;
use ui::colors::StandardTaskColors;
use storages::TomlStorage;

use dirs::{self, home_dir};

// use uni::{todo::Todo, task::Task};
fn ask_with_prefix(prefix: &str) -> String {
    print!("{}", prefix);
    stdout().flush().expect(constants::FLUSH_ERROR);

    let mut buf = "".to_owned();
    stdin()
        .lock()
        .read_line(&mut buf)
        .expect(constants::STDIN_ERROR);

    buf
}

fn main() -> io::Result<()> {
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

    let mut terminal = ratatui::init();
    terminal.clear()?;
    let mut app = App::<StandardTaskColors, EditorTaskReader, TomlStorage>::new(
        Box::new(TasksPane::new()),
        Box::new(DescriptionPane::default()),
        dir.clone(),
    )?;
    let app_result = app.run(terminal);
    ratatui::restore();
    app_result?;

    // if app.should_save() {
    //     let answer = ask_with_prefix("\nDo you want to save the tasks? (Y/n): ");
    //     match answer.trim().to_lowercase().as_str() {
    //         "y" | "yes" | "" => {
    //             println!("Saving tasks to the {}...", constants::TABLE_NAME);
    //             app.save()
    //         }
    //         _ => Ok(()),
    //     }
    // } else {
    //     Ok(())
    // }
    Ok(())
}
