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

    pub fn render(&self) {
        // Render the calendar and apply coloring logic
        let (first_day, num_days) = self.get_month_info();
        let weekday_labels = "Mo Tu We Th Fr Sa Su";
        println!("{}{}", " ".repeat(weekday_labels.len() / 2 - 2), self.get_month_name());
        println!("{}", weekday_labels);

        for _ in 0..first_day {
            print!("   ");
        }

        for day in 1..=num_days {
            let date = self.date.date_naive().with_day(day).expect("Could not set day correctly");
            let task_count = self.tasks.iter().filter(|t| t.time.map_or(false, |d| d.date_naive() == date)).count();
            let mut colored_day = self.color_day(day, task_count);

            if date == Local::now().date_naive() {
                colored_day = colored_day.on_color(Color::TrueColor {
                    r: 96u8,
                    g: 96u8,
                    b: 96u8
                });
            }

            print!("{:>2} ", colored_day);
            if (day + first_day) % 7 == 0 {
                println!();
            }
        }
        println!();
    }

    fn get_month_info(&self) -> (u32, u32) {
        let year = self.date.year();
        let month = self.date.month();

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

    fn get_month_name(&self) -> &str {
        match self.date.month() {
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
        }
    }

    fn color_day(&self, day: u32, task_count: usize) -> ColoredString {
        match task_count {
            0 => day.to_string().white(),
            1 | 2 => day.to_string().yellow(),
            _ => day.to_string().red(),
        }
    }
}
