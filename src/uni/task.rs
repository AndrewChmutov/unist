use std::io;
use std::fs;
use std::path::Path;

use chrono::{DateTime, TimeDelta, Local};

use crate::constants;

pub enum TaskStatus {
    Panic,
    Normal,
    Zen,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub description: String,
    pub subject: String,
    pub time: Option<DateTime<Local>>,
    pub complete: bool
}

impl Task {
    pub fn get_delta(&self, target: &DateTime<Local>) -> Option<TimeDelta> {
        self.time.clone().map(|v| v - target)
    }

    pub fn get_delta_now(&self) -> Option<TimeDelta> {
        self.get_delta(&Local::now())
    }

    pub fn get_status(&self, duration: &Option<TimeDelta>) -> TaskStatus {
        if duration.is_none() {
            return match self.complete {
                true    => TaskStatus::Zen,
                false   => TaskStatus::Normal
            }
        }

        let duration = duration.unwrap();

        if duration.num_days() < constants::DAYS_LEFT as i64 && !self.complete {
            TaskStatus::Panic
        } else if self.complete {
            TaskStatus::Zen
        } else {
            TaskStatus::Normal
        }
    }

    pub fn get_status_now(&self) -> TaskStatus {
        let duration = self.get_delta_now();
        self.get_status(&duration)
    }

    fn read_task(csv_declaration: &str) -> Result<Task, ()> {
        let words: Vec<&str> = csv_declaration.split(',').collect();
        if words.len() < 5 {
            return Err(());
        }

        Ok(Task {
            name:           words[0].to_owned(),
            description:    words[1].to_owned(),
            subject:        words[2].to_owned(),
            time:           match DateTime::parse_from_rfc3339(words[3]) {
                                Ok(date) => Some(date.into()),
                                Err(_) => None
                            },
            complete:       words[4].trim().parse::<bool>().map_err(|_| ())?
        })
    }

    pub fn read_tasks(path_to_file: &Path) -> io::Result<Vec<Task>> {
        let file_content = fs::read_to_string(path_to_file)?;
        let mut tasks = vec![];

        for (i, line) in file_content.lines().enumerate() {
            match Task::read_task(line) {
                Ok(task) => tasks.push(task),
                Err(_) => eprintln!("Could not parse line #{i}: {line}")
            }
        }

        Ok(tasks)
    }
}
