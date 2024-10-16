use std::io;
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::io::stdout;

use crate::uni::task::{Task, TaskStatus};
use crate::readers::{TaskReader, EditorTaskReader};
use crate::storages::{TaskStorage, TomlStorage};
use super::panes::Pane;
use super::colors::{TaskColors, StandardTaskColors};
use super::popups::{ClosurePopup, Popup, PopupAction};

use ratatui::prelude::*;
use ratatui::DefaultTerminal;
use ratatui::crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal::{
        EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand,
};


#[derive(Default, Debug)]
pub struct Data {
    pub index: Option<usize>,
    pub tasks: Vec<Task>,
    pub filter_zen: bool,
}

impl Data {
    pub fn new(tasks: Vec<Task>) -> Self {
        let index = match tasks.len() {
            0 => None,
            _ => Some(0),
        };
        Self { index, tasks, filter_zen: false }
    }

    fn sort(&mut self) {
        self.tasks.sort_by(|task1, task2| {
            if task1.complete && !task2.complete {
                return Ordering::Greater;
            } else if !task1.complete && task2.complete {
                return Ordering::Less;
            }

            if task1.time.is_some() && task2.time.is_none() {
                return Ordering::Less;
            } else if task1.time.is_none() && task2.time.is_some() {
                return Ordering::Greater;
            } else if task1.time.is_none() && task2.time.is_none() {
                return Ordering::Equal;
            }

            task1.time
                .unwrap()
                .partial_cmp(&task2.time.unwrap())
                .expect("Could not perform the comparison")
        });
    }

    pub fn iter(&self) -> DataIterator {
        DataIterator {
            data: self,
            index: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.iter().collect::<Vec<_>>().len()
    }

    pub fn get(&self, index: usize) -> Option<&Task> {
        self.iter().collect::<Vec<_>>().get(index).map(|x| *x)
    }

    pub fn toggle_task_status(&mut self) {
        if let Some(i) = self.index {
            self.tasks[i].complete = !self.tasks[i].complete;
        }
    }

    pub fn toggle_task_star(&mut self) {
        if let Some(i) = self.index {
            self.tasks[i].starred = !self.tasks[i].starred;
        }
    }

    pub fn toggle_filter_zen(&mut self) {
        self.filter_zen = !self.filter_zen;
        let current_len = self.iter().collect::<Vec<_>>().len();
        self.index = self.index.and_then(|x|
            if current_len == 0 {None}
            else {Some(x.min(current_len - 1))}
        );
        if self.index.is_none() && current_len > 0 {
            self.index = Some(0)
        };
    }
}

pub struct DataIterator<'a> {
    data: &'a Data,
    index: usize,
}

impl<'a> Iterator for DataIterator<'a> {
    type Item = &'a Task;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        self.data
            .tasks
            .iter()
            .filter(|x| !(matches!(x.get_status_now(), TaskStatus::Zen) && self.data.filter_zen))
            .skip(self.index - 1)
            .next()
    }
}


#[derive(Default)]
enum CurrentPane {
    #[default]
    Left,
    Right,
}

pub struct App<'a, T = StandardTaskColors, R =  EditorTaskReader, S = TomlStorage> 
where
    T: TaskColors,
    R: TaskReader,
    S: TaskStorage,
{
    pub data: Data,
    current_pane: CurrentPane,
    left_pane: Box<dyn Pane<T>>,
    right_pane: Box<dyn Pane<T>>,
    current_popup: Option<Box<dyn Popup<T> + 'a>>,
    storage: S,
    exit: bool,
    _reader_marker: PhantomData<R>
}

impl<'a, T: TaskColors, R: TaskReader, S: TaskStorage> App<'a, T, R, S> where
{
    pub fn new(left_pane: Box<dyn Pane<T>>, right_pane: Box<dyn Pane<T>>, path: PathBuf) -> io::Result<Self> {
        let storage = S::new(path);
        let tasks = storage.read()?;
        Ok(Self {
            data: Data::new(tasks),
            current_pane: CurrentPane::Left,
            left_pane,
            right_pane,
            current_popup: None,
            storage,
            exit: false,
            _reader_marker: PhantomData,
        })
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events(&mut terminal)?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ]).split(frame.area());

        self.data.sort();

        let left_active = matches!(self.current_pane, CurrentPane::Left);
        self.current_popup = if let Some(popup) = self.current_popup.take() {
            popup.render(frame, frame.area());
            Some(popup)
        } else {
            None
        };
        if self.current_popup.is_some() {
            return
        }

        self.left_pane.render(frame, chunks[0], &self.data, left_active);
        self.right_pane.render(frame, chunks[1], &self.data, !left_active);
    }

    fn handle_events(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(terminal, key_event)?
            }
            _ => ()
        }
        Ok(())
    }

    pub fn save(&self) -> io::Result<()> {
        self.storage.write(&self.data.tasks)
    }

    fn edit(&mut self) -> Result<(), ()> {
        if let Some(old_task) = self.data.index.and_then(|x| self.data.tasks.get(x)) {
            let new_task = R::read(&old_task)?;
            self.data.tasks[self.data.index.unwrap()] = new_task;
        };
        Ok(())
    }

    fn add_default(&mut self) {
        self.data.tasks.push(Task::default())
    }

    fn handle_key_event(&mut self, terminal: &mut DefaultTerminal, key_event: KeyEvent) -> io::Result<()> {
        let mut should_stop = false;
        self.current_popup = match self.current_popup.take() {
            // Popup exists
            Some(mut popup) => {
                should_stop = true;
                // Popup persists
                match popup.handle_key_event(&key_event, &mut self.data) {
                    PopupAction::Exit => {
                        self.exit = true;
                        None
                    },
                    PopupAction::Close => {
                        None
                    },
                    PopupAction::None => {
                        Some(popup)
                    }
                }
            },
            None => None,
        };
        if should_stop {return Ok(())}
        // dbg!(format!("{}", self.current_popup.is_none().to_string()));

        match key_event.code {
            KeyCode::Char('q') => self.exit()?,
            KeyCode::Char('h') => {
                self.left_pane.enter();
                self.right_pane.leave();
                self.current_pane = CurrentPane::Left;
            },
            KeyCode::Char('l') => {
                self.right_pane.enter();
                self.left_pane.leave();
                self.current_pane = CurrentPane::Right;
            },
            KeyCode::Char('f') => self.data.toggle_filter_zen(),
            KeyCode::Char('w') => { self.save().unwrap(); },
            KeyCode::Char('e') => {
                stdout().execute(LeaveAlternateScreen)?;
                // disable_raw_mode()?;
                self.edit().unwrap();
                stdout().execute(EnterAlternateScreen)?;
                // enable_raw_mode()?;
                terminal.clear()?;
            },
            KeyCode::Char('p') => {
                self.add_default();
            }
            _ => {
                self.current_popup = match self.current_pane {
                    CurrentPane::Left => self.left_pane.handle_key_event(key_event, &mut self.data),
                    CurrentPane::Right => self.right_pane.handle_key_event(key_event, &mut self.data),
                };
            }
        };
        Ok(())
    }

    fn exit(&mut self) -> io::Result<()> {
        if !self.storage.should_save(&self.data.tasks) {
            self.exit = true;
            return Ok(());
        }
        let storage = self.storage.clone();
        let popup = ClosurePopup {
            payload: Box::new(move |data: &mut Data, key_event: &KeyEvent| {
                match key_event.code {
                    KeyCode::Char('y') | KeyCode::Enter => {
                        storage.write(&data.tasks).unwrap();
                        PopupAction::Exit
                    },
                    KeyCode::Char('n') => PopupAction::Exit,
                    _ => PopupAction::None,
                }
            }),
            text: "You have unsaved progress. Save it?".to_string(),
            confirmation: Box::new(|key_event: &KeyEvent| {
                [KeyCode::Enter, KeyCode::Char('y'), KeyCode::Char('n')].contains(&key_event.code)
            }),
            cancellation: Box::new(|key_event: &KeyEvent| {key_event.code == KeyCode::Esc}),
            _marker: PhantomData,
        };
        self.current_popup = Some(Box::new(popup));
        Ok(())
    }
}
