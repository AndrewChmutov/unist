use std::marker::PhantomData;

use super::{app::Data, colors::TaskColors};

use ratatui::prelude::*;
use ratatui::crossterm::event::KeyEvent;
use ratatui::widgets::{Block, BorderType, Padding, Paragraph, Wrap};

fn centered_rect(max_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Max(max_x),
            Constraint::Fill(1),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}

pub enum PopupAction {
    Close,
    Exit,
    None,
}

pub trait Popup<T: TaskColors> {
    fn size(&self) -> (u16, u16);
    fn title(&self) -> Line { Line::from(" Warning ").fg(T::highlight_desc()) }
    fn paragraph(&self) -> Paragraph;
    fn handle_key_event(&mut self, key_event: &KeyEvent, data: &mut Data) -> PopupAction;
    fn render(&self, frame: &mut Frame, area: Rect) {
        let popup_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title_alignment(Alignment::Center)
            .title(self.title())
            .padding(Padding::uniform(1));

        let paragraph = self.paragraph()
            .block(popup_block);

        let area = centered_rect(self.size().0, self.size().1, area);
        frame.render_widget(paragraph, area);
    }
}

pub struct ClosurePopup<T: TaskColors> {
    pub text: String,
    pub payload: Box<dyn FnMut(&mut Data, &KeyEvent) -> PopupAction>,
    pub confirmation: Box<dyn Fn(&KeyEvent) -> bool>,
    pub cancellation: Box<dyn Fn(&KeyEvent) -> bool>,
    pub _marker: PhantomData<T>
}

impl<'a, T: TaskColors> Popup<T> for ClosurePopup<T> {
    fn size(&self) -> (u16, u16) {
        return (65, 25)
    }
    fn paragraph(&self) -> Paragraph {
        let exit_text = Text::raw(&self.text);
        Paragraph::new(exit_text)
            .wrap(Wrap { trim: false })
    }
    fn handle_key_event(&mut self, key_event: &KeyEvent, data: &mut Data) -> PopupAction {
        if (self.confirmation)(key_event) {
            (self.payload)(data, key_event)
        } else if (self.cancellation)(key_event) {
            PopupAction::Close
        } else {
            PopupAction::None
        }
    }
}
