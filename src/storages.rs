use std::io;
use std::{fs, path::PathBuf};

use crate::uni::task::Task;

use chrono::DateTime;
use serde::{Serialize, Deserialize};


pub trait TaskStorage: Sized + Clone + 'static {
    fn new(path: PathBuf) -> Self;
    fn should_save(&self, tasks: &Vec<Task>) -> bool;
    fn read(&self) -> Result<Vec<Task>, io::Error>;
    fn write(&self, tasks: &Vec<Task>) -> Result<(), io::Error>;
}


#[derive(Serialize, Deserialize)]
pub struct TaskEntry {
    pub name: String,
    pub description: String,
    pub subject: String,
    pub time: Option<String>,
    pub complete: bool,
    pub starred: bool,
}

#[derive(Serialize, Deserialize)]
struct Tasks {
    tasks: Vec<TaskEntry>
}

impl TaskEntry {
    fn from_task(task: &Task) -> Self {
        Self {
            name: task.name.clone(),
            description: task.description.clone(),
            subject: task.subject.clone(),
            time: task.time.map(|x| x.to_rfc3339()),
            complete: task.complete,
            starred: task.starred,
        }
    }
    fn to_task(self) -> Result<Task, ()> {
        let time = match self.time {
            Some(time) => {
                Some(DateTime::parse_from_rfc3339(&time).map_err(|_| ())?)
            }
            None => None
        };
        Ok(Task {
            name: self.name,
            description: self.description,
            subject: self.subject,
            time,
            complete: self.complete,
            starred: self.starred,
        })
    }
}

#[derive(Clone)]
pub struct TomlStorage {
    path: PathBuf
}

impl TomlStorage {
    fn dump<'a>(&self, tasks: &Vec<Task>) -> String {
        let tasks = Tasks {
            tasks: tasks.iter().map(TaskEntry::from_task).collect()
        };
        toml::to_string(&tasks).unwrap()
    }
}

impl TaskStorage for TomlStorage {
    fn new(path: PathBuf) -> Self {
        Self {
            path
        }
    }


    fn read(&self) -> Result<Vec<Task>, std::io::Error> {
        let content = fs::read_to_string(&self.path)?;
        let task_entries = toml::from_str::<Tasks>(&content).expect(&format!("Could not parse the file: {}", self.path.display()));
        let mut tasks = vec![];
        for task_entry in task_entries.tasks {
            let name = task_entry.name.clone();
            tasks.push(task_entry.to_task().expect(&format!("Could not parse the task {}", name)));
        };
        Ok(tasks)
    }

    fn should_save(&self, tasks: &Vec<Task>) -> bool {
        fs::read_to_string(&self.path).unwrap() != self.dump(tasks)
    }

    fn write(&self, tasks: &Vec<Task>) -> Result<(), std::io::Error> {
        fs::write(&self.path, self.dump(tasks))
    }
}
