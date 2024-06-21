use chrono::{DateTime, Datelike, Local, NaiveDate};
use crate::uni::task::Task;
use colored::{Colorize, ColoredString, Color};


pub struct Calendar<'a> {
    date: DateTime<Local>,
    tasks: &'a Vec<Task>
}

impl<'a> Calendar<'a> {
    pub fn new(date: DateTime<Local>, tasks: &'a Vec<Task>) -> Self {
        Calendar { date, tasks }
    }

    pub fn render_month_buffer_ym(&self, year: i32, month: u32) -> Vec<String> {
        let (first_day, num_days) = Self::get_month_info_ym(year, month);
        let weekday_labels = "Mo Tu We Th Fr Sa Su ";
        let mut result = vec![];

        let lspaces = " ".repeat(weekday_labels.len() / 2 - 2);
        let rspaces = " ".repeat(weekday_labels.len() - lspaces.len() - 3);
        result.push(format!(
            "{}{}{}",
            lspaces,
            Self::get_month_name_m(month),
            rspaces
        ));

        result.push(weekday_labels.to_owned());

        let mut current = "".to_owned();

        for _ in 0..first_day {
            current += "   ";
        }

        for day in 1..=num_days {
            let date = NaiveDate::from_ymd_opt(year, month, day).expect("Could not set the date");
            let task_count = self.tasks
                .iter()
                .filter(|t| t.time.map_or(false, |d| d.date_naive() == date))
                .count();
            let mut colored_day = self.color_day(day, task_count);

            if date == Local::now().date_naive() {
                colored_day = colored_day.on_color(Color::TrueColor {
                    r: 96u8,
                    g: 96u8,
                    b: 96u8
                });
            }

            current += format!("{:>2} ", colored_day).as_str();
            if (day + first_day) % 7 == 0 {
                result.push(current);
                current = "".to_owned();
            }
        }

        let total = first_day + num_days;
        let filled = if total % 7 == 0 {
            total
        } else {
            (total as f32 / 7f32).floor() as u32 * 7 + 7
        };
        let padding_len = (filled - total) as usize;
        let padding = "   ".repeat(padding_len);
        result.push(current + &padding);

        result
    }

    pub fn render_month_buffer_m(&self, month: u32) -> Vec<String> {
        let year = self.date.year();

        self.render_month_buffer_ym(year, month)
    }

    pub fn render_month_buffer(&self) -> Vec<String> {
        let year = self.date.year();
        let month = self.date.month();

        self.render_month_buffer_ym(year, month)
    }

    pub fn render(&self) {
        let lines = self.render_month_buffer();
        for line in lines.iter() {
            println!("{line}");
        }

        println!();
    }

    pub fn render3(&self) {
        let (previous_year, previous_month) = if self.date.month() == 1 {
            (self.date.year() - 1, 12u32)
        } else {
            (self.date.year(), self.date.month() - 1)
        };

        let (next_year, next_month) = if self.date.month() == 12 {
            (self.date.year() + 1, 1)
        } else {
            (self.date.year(), self.date.month() + 1)
        };

        let months = [
            self.render_month_buffer_ym(previous_year, previous_month),
            self.render_month_buffer(),
            self.render_month_buffer_ym(next_year, next_month)
        ];

        let month_width = months[0][0].len();
        let max_height = months
            .iter()
            .map(|el| el.len())
            .max().unwrap_or(0);

        let hpadding = " ".repeat(month_width);
        let vpadding = " ".repeat(2);
        for i in 0..max_height {
            for j in 0..months.len() {
                print!("{}{}", months[j].get(i).unwrap_or(&hpadding), vpadding);
            }

            println!();
        }
    }

    fn get_month_info_ym(year: i32, month: u32) -> (u32, u32) {
        let current_month_first_day = NaiveDate::from_ymd_opt(year, month, 1)
            .unwrap();

        let next_month_first_day = NaiveDate::from_ymd_opt(year, month + 1, 1)
            .unwrap_or(NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap());

        let first_day = current_month_first_day
            .weekday()
            .num_days_from_monday();

        let num_days = next_month_first_day
            .signed_duration_since(current_month_first_day)
            .num_days() as u32;

        (first_day, num_days)
    }

    fn get_month_info_m(&self, month: u32) -> (u32, u32) {
        let year = self.date.month();

        Self::get_month_info_ym(year as i32, month)
    }

    fn get_month_info(&self) -> (u32, u32) {
        let year = self.date.year();
        let month = self.date.month();

        Self::get_month_info_ym(year, month)
    }

    fn get_month_name_m(month: u32) -> String {
        match month {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "Unknown",
        }.to_owned()
    }

    fn get_month_name(&self) -> String {
        let month = self.date.month();

        Self::get_month_name_m(month)
    }

    fn color_day(&self, day: u32, task_count: usize) -> ColoredString {
        match task_count {
            0 => day.to_string().white(),
            1 | 2 => day.to_string().yellow(),
            _ => day.to_string().red(),
        }
    }
}
