use crate::uni::task::{Task, TaskStatus};

use ratatui::style::{Color, palette::tailwind};

pub trait TaskColors: 'static {
    fn highlight_table() -> Color { Color::Gray }
    fn highlight_desc() -> Color { Color::Gray }
    fn highlight_border() -> Color { Color::Gray }

    #[allow(unused)]
    fn task_color(_status: &Task) -> Color;
}
// #b8bb26

pub struct StandardTaskColors;
impl TaskColors for StandardTaskColors {
    fn highlight_table() -> Color { tailwind::GRAY.c600 }
    fn highlight_desc() -> Color { Color::from_u32(0xfabd2f) }
    fn highlight_border() -> Color { Color::Rgb(142, 192, 124) }

    fn task_color(task: &Task) -> Color {
        match task.get_status_now() {
            TaskStatus::Panic => Color::Rgb(251, 73, 52),
            TaskStatus::Normal => Color::White,
            TaskStatus::Zen => Color::from_u32(0x6b7280),
        }
    }
}
