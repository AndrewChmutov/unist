use std::io::{Read, Seek, Write};
use chrono::{DateTime, FixedOffset, Local};
use std::io::SeekFrom;
use std::io;

use crate::uni::task::Task;

use serde::{Serialize, Deserialize};
use matter::matter;


pub trait TaskReader {
    fn read(template: &Task) -> Result<Task, ()>;
}


#[derive(Serialize, Deserialize)]
pub struct TaskMetadata {
    pub name: String,
    pub subject: String,
    pub time: Option<String>,
    pub complete: bool,
    pub starred: bool,
}

impl TaskMetadata {
    const DATE_FORMAT: &str = "%Y-%m-%d %H:%M %z";

    fn format_time(time: DateTime<FixedOffset>) -> String {
        format!("{}", time.format(Self::DATE_FORMAT))
    }

    fn parse_time(time: Option<String>) -> Result<DateTime<FixedOffset>, ()> {
        match time {
            Some(time) => Ok(DateTime::parse_from_str(&time, Self::DATE_FORMAT)
                .unwrap_or(Local::now().fixed_offset())),
            None => Err(()),
        }
    }

    fn from_task(task: &Task) -> Self {
        Self {
            name: task.name.clone(),
            subject: task.subject.clone(),
            time: Some(Self::format_time(task.time.clone().unwrap_or(Local::now().fixed_offset()))),
            complete: task.complete,
            starred: task.starred,
        }
    }
}

pub struct EditorTaskReader;

impl EditorTaskReader {
    fn to_task(task_proxy: TaskMetadata, description: String) -> Result<Task, ()> {
        Ok(Task {
            name: task_proxy.name,
            subject: task_proxy.subject,
            time: TaskMetadata::parse_time(task_proxy.time).map(|x| x.fixed_offset()).ok(),
            description,
            complete: task_proxy.complete,
            starred: task_proxy.starred,
        })
    }

    fn task_to_string(task: &Task) -> String {
        let metadata = TaskMetadata::from_task(task);
        let metadata_str = serde_yaml::to_string(&metadata).unwrap();
        format!("---\n{}\n---\n{}", metadata_str, &task.description)
    }

    fn from_str_task(task: &str) -> Result<Task, ()> {
        matter(task)
            .ok_or(())
            .map(|(metadata, description)| (serde_yaml::from_str::<TaskMetadata>(&metadata), description))
            .and_then(|(metadata, description)| {
                metadata
                    .map_err(|_| ())
                    .and_then(|metadata| Self::to_task(metadata, description))
            })
    }

    fn _read(template: &Task) -> Result<String, io::Error> {
        let template = Self::task_to_string(template);
        let mut file = tempfile::Builder::new()
            .suffix(".md")
            .tempfile()?;
        file.write_all(template.as_bytes())?;
        file.seek(SeekFrom::Start(0))?;

        edit::edit_file(&file)?;
        file.seek(SeekFrom::Start(0))?;

        let mut modified_task_str = "".to_string();
        file.read_to_string(&mut modified_task_str)?;
        Ok(modified_task_str)
    }
}


impl TaskReader for EditorTaskReader {
    fn read(template: &Task) -> Result<Task, ()> {
        Self::_read(template)
            .map_err(|_| ())
            .and_then(|x| Self::from_str_task(&x))
    }
}
