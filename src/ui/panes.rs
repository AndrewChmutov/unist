use std::{borrow::BorrowMut, cell::RefCell};
use std::marker::PhantomData;

use super::{colors::TaskColors, popups::ClosurePopup};
use super::app::Data;
use super::popups::{Popup, PopupAction};
use crate::uni::task::Task;

use unicode_width::UnicodeWidthStr;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers}, layout::{Alignment, Constraint, Layout, Rect}, style::{palette::tailwind, Modifier, Style, Stylize}, text::{Line, Span, Text, ToText}, widgets::{block::{Position, Title}, Block, BorderType, Paragraph, Row, Table, TableState, Wrap}, Frame
};

pub trait Pane<T: TaskColors> {
    fn create_block(&self, title: &str, active: bool) -> Block {
        let title = Title::from(format!(" {title} ").bold());
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title.alignment(Alignment::Center));

        if active {
            block.border_style(Style::default().fg(T::highlight_border()))
        } else {
            block
        }
    }

    fn render(&mut self, frame: &mut Frame, chunk: Rect, data: &Data, active: bool);
    fn handle_key_event(&mut self, _key_event: KeyEvent, _data: &mut Data) -> Option<Box<dyn Popup<T>>> {None}

    fn enter(&mut self) {}
    fn leave(&mut self) {}
}

pub struct TasksPane {
    table_state: RefCell<TableState>,
    show_numbers: bool,
}

impl TasksPane {
    const BAR: &str = " > ";

    pub fn new() -> Self {
        Self {
            table_state: RefCell::new(TableState::default().with_selected(0)),
            show_numbers: false,
        }
    }

    fn make_header(&self) -> Row {
        let mut headers = vec![];
        headers.push(" ");
        if self.show_numbers { headers.push("No") }
        headers.extend(["Subject", "Name", "Time Left"]);
        headers.into_iter().collect::<Row>()
    }

    fn make_row<T: TaskColors>(&self, i: usize, task: &Task, highlighted: bool) -> Row {
        let mut cells = vec![];
        cells.push(if task.starred {"*".to_string()} else {" ".to_string()});
        if self.show_numbers { cells.push(i.to_string()) }
        cells.extend([task.subject().to_string(), task.name().to_string(), task.delta()]);
        let row = cells.into_iter().collect::<Row>().fg(T::task_color(task));

        if highlighted {
            row.add_modifier(Modifier::REVERSED)
        } else {
            row
        }
    }

    fn make_rows<T: TaskColors>(&self, data: &Data) -> Vec<Row> {
        data
            .iter()
            .enumerate()
            .map(|(i, task)| self.make_row::<T>(i, task, i == data.index.unwrap()))
            .collect()
    }

    fn make_constraints(&self, data: &Data) -> Vec<Constraint> {
        let index_len = data.len().to_string().len();
        let (subject_len, name_len, delta_len) = data
            .iter()
            .map(|task| (task.subject(), task.name(), task.delta()))
            .map(|(s, n, d)| (s.width(), n.width(), d.as_str().width()))
            .fold((usize::MIN, usize::MIN, usize::MIN), |(ms, mn, md), (s, n, d)| (ms.max(s), mn.max(n), md.max(d)));

        let mut constraints = vec![];
        constraints.push(Constraint::Length(1));
        if self.show_numbers { constraints.push(Constraint::Max(index_len as u16 + 1))}
        constraints.push(Constraint::Length(subject_len as u16 + 1));
        constraints.push(Constraint::Min(name_len as u16 + 1));
        constraints.push(Constraint::Min(delta_len as u16 + 1));
        constraints
    }


    fn table<T: TaskColors>(&self, data: &Data) -> Table {
        let highlight_style = Style::default()
            .bg(T::highlight_table())
            .add_modifier(Modifier::BOLD);

        let t = Table::new(
            self.make_rows::<T>(data),
            self.make_constraints(data)
        )
        .header(self.make_header())
        // .row_highlight_style(selected_row_style)
        // .column_highlight_style(selected_col_style)
        // .highlight_style(highlight_style)
        .highlight_symbol(Text::from(vec![
            Self::BAR.into(),
        ]));
        // .bg(self.colors.buffer_bg)
        // .highlight_spacing(HighlightSpacing::Always);
        // frame.render_stateful_widget(t, area, &mut self.table_state);
        t
    }

    fn first(&self, data: &mut Data) {
        self.table_state.borrow_mut().select_first();
        data.index = self.table_state.borrow().selected();
    }

    fn last(&self, data: &mut Data) {
        self.table_state.borrow_mut().select_last();
        data.index = self.table_state.borrow().selected().map(|x| x.min(data.len() - 1));
    }

    fn next(&self, data: &mut Data) {
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i >= data.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.borrow_mut().select(Some(i));
        data.index = Some(i);
    }

    fn previous(&self, data: &mut Data) {
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    data.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.borrow_mut().select(Some(i));
        data.index = Some(i);
    }

    fn toggle_numbers(&mut self) { self.show_numbers = !self.show_numbers; }

    fn remove<T: TaskColors>(&mut self, data: &mut Data) -> Option<Box<dyn Popup<T>>> {
        if let Some(task) = data.index.and_then(|x| data.tasks.get(x)) {
            return if task.is_default() {
                data.tasks.remove(data.index.unwrap());
                None
            }
            else {
                let closure_popup = ClosurePopup {
                    text: format!("Would you like to remove task \"{}: {}\"", &task.subject, &task.name),
                    payload: Box::new(|data, _key_event| {
                        data.tasks.remove(data.index.unwrap());
                        PopupAction::Close
                    }),
                    confirmation: Box::new(|key_event: &KeyEvent| {key_event.code == KeyCode::Char('d')}),
                    cancellation: Box::new(|key_event: &KeyEvent| {key_event.code == KeyCode::Esc}),
                    _marker: PhantomData,
                };
                Some(Box::new(closure_popup))
            }
        }
        None
    }
}


impl<T: TaskColors> Pane<T> for TasksPane {
    fn render(&mut self, frame: &mut Frame, chunk: Rect, data: &Data, active: bool) {
        self.table_state.borrow_mut().select(data.index);
        let table = self
            .table::<T>(data)
            .block(<TasksPane as Pane<T>>::create_block(self, "Tasks", active));
        frame.render_stateful_widget(table, chunk, &mut *self.table_state.borrow_mut());
    }

    fn handle_key_event(&mut self, key_event: KeyEvent, data: &mut Data) -> Option<Box<dyn Popup<T>>> {
        match key_event.code {
            KeyCode::Char('G') => {self.last(data); None}
            KeyCode::Char('g') => {self.first(data); None}
            KeyCode::Char('j') => {self.next(data); None}
            KeyCode::Char('k') => {self.previous(data); None}
            KeyCode::Char('i') => {self.toggle_numbers(); None}
            KeyCode::Char('c') => {data.toggle_task_status(); None}
            KeyCode::Char('s') => {data.toggle_task_star(); None}
            KeyCode::Char('d') => {self.remove(data)}
            _ => None
        }
    }
}

#[derive(Default, Clone)]
enum DescriptionEntry {
    #[default]
    Header,
    Deadline,
    Description,
}

#[derive(Default)]
pub struct DescriptionPane {
    current_entry: Option<DescriptionEntry>
}

impl DescriptionPane {
    fn render_header<T: TaskColors>(&self, frame: &mut Frame, chunk: Rect, task: &Task, active: bool) {
        let mut header = Line::raw(format!("{}: {}", task.subject(), task.name()))
            .alignment(Alignment::Center)
            .add_modifier(Modifier::BOLD)
            .fg(T::highlight_desc());

        if active { header = header.bg(tailwind::GRAY.c700) };

        frame.render_widget(header, chunk);
    }

    fn render_deadline<T: TaskColors>(&self, frame: &mut Frame, chunk: Rect, task: &Task, active: bool) {
        let date_str = task.
            time
            .map(|x| x.to_rfc2822())
            .unwrap_or("None".to_string());

        let mut date_span = Span::raw(format!("Deadline: {date_str}"));

        if active { date_span = date_span.bg(tailwind::GRAY.c700) };

        frame.render_widget(date_span, chunk);
    }

    fn render_description<T: TaskColors>(&self, frame: &mut Frame, chunk: Rect, task: &Task, active: bool) {
        let inner_chunks = Layout::vertical([
            Constraint::Length(1), Constraint::Fill(1)
        ]).split(chunk);
        let label_chunk = inner_chunks[0];
        let description_chunk = Layout::horizontal([Constraint::Fill(1)]).horizontal_margin(1).split(inner_chunks[1])[0];

        let mut span_label = Span::raw("Description");
        let mut span_description = Paragraph::new(Text::from(task.description.as_str())).wrap( Wrap { trim: false} );

        if active {
            span_label = span_label.bg(tailwind::GRAY.c700);
            span_description = span_description.bg(tailwind::GRAY.c700);
        }

        frame.render_widget(span_label, label_chunk);
        frame.render_widget(span_description, description_chunk);
    }

    fn next(&mut self) {
        self.current_entry = self.current_entry.take().map(|x| {
            match x {
                DescriptionEntry::Header => DescriptionEntry::Deadline,
                DescriptionEntry::Deadline => DescriptionEntry::Description,
                DescriptionEntry::Description => DescriptionEntry::Header,
            }
        });
    }

    fn previous(&mut self) {
        self.current_entry = self.current_entry.take().map(|x| {
            match x {
                DescriptionEntry::Header => DescriptionEntry::Description,
                DescriptionEntry::Deadline => DescriptionEntry::Header,
                DescriptionEntry::Description => DescriptionEntry::Deadline,
            }
        });
    }
}

impl<T: TaskColors> Pane<T> for DescriptionPane {
    fn render(&mut self, frame: &mut Frame, chunk: Rect, data: &Data, active: bool) {
        {
            let block = <DescriptionPane as Pane<T>>::create_block(self, "Description", active);
            frame.render_widget(block, chunk);
        }

        let inner = Layout::horizontal([
            Constraint::Fill(1)
        ]).margin(2).split(chunk)[0];

        let inner_chunks = Layout::vertical([
            Constraint::Length(2),  // Name
            Constraint::Length(1),  // Deadline
            Constraint::Fill(1)     // Description
        ]).split(inner);

        let task = match data.index.and_then(|x| data.get(x)) {
            Some(task) => task,
            None => return,
        };

        let (header_active, deadline_active, description_active) = match self
            .current_entry.clone()
            {
                Some(DescriptionEntry::Header) => (true, false, false),
                Some(DescriptionEntry::Deadline) => (false, true, false),
                Some(DescriptionEntry::Description) => (false, false, true),
                _ => (false, false, false),
            };

        self.render_header::<T>(frame, inner_chunks[0], task, header_active);
        self.render_deadline::<T>(frame, inner_chunks[1], task, deadline_active);
        self.render_description::<T>(frame, inner_chunks[2], task, description_active);
    }

    fn handle_key_event(&mut self, key_event: KeyEvent, _data: &mut Data) -> Option<Box<dyn Popup<T>>> {
        match key_event.code {
            KeyCode::Char('j') => self.next(),
            KeyCode::Char('k') => self.previous(),
            _ => ()
        };
        None
    }

    fn enter(&mut self) {
        self.current_entry = Some(DescriptionEntry::default());
    }

    fn leave(&mut self) {
        self.current_entry = None;
    }
}
