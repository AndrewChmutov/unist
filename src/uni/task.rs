use chrono::{DateTime, FixedOffset, Local, TimeDelta};

use crate::constants;

pub enum TaskStatus {
    Panic,
    Normal,
    Zen,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub name: String,
    pub description: String,
    pub subject: String,
    pub time: Option<DateTime<FixedOffset>>,
    pub complete: bool,
    pub starred: bool,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            name: "[Name]".to_string(),
            description: "Description_goes_here".to_string(),
            subject: "[Subject]".to_string(),
            time: Some(Local::now().fixed_offset()),
            complete: false,
            starred: false,
        }
    }
}

impl Task {
    pub fn get_delta(&self, target: &DateTime<FixedOffset>) -> Option<TimeDelta> {
        self.time.clone().map(|v| v - target)
    }

    pub fn get_delta_now(&self) -> Option<TimeDelta> {
        self.get_delta(&Local::now().fixed_offset())
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    fn time_quantity_format(str: &str, num: i32) -> Option<String> {
        if num == 1 || num == -1 {
            Some(num.to_string() + " " +  str)
        } else if num > 1 || num < -1 {
            Some(num.to_string() + " " + str + "s")
        } else {
            None
        }
    }

    pub fn delta(&self) -> String {
        if let Some(duration) = self.get_delta_now() {
            if duration.abs() < TimeDelta::minutes(1) {
                return "No time!".to_string();
            }
            // Format time until the task
            let days    = duration.num_days() as i32;
            let hours   = duration.num_hours() as i32 - duration.num_days() as i32 * 24;
            let minutes = duration.num_minutes() as i32 - duration.num_hours() as i32 * 60;

            let days = Self::time_quantity_format("day", days);
            let hours = Self::time_quantity_format("hour", hours);
            let minutes = Self::time_quantity_format("minute", minutes);

            let mut units = vec![];
            if let Some(days) = days {units.push(days)}
            if let Some(hours) = hours {units.push(hours)}

            // TODO: Tackle the long/short format
            let long = true;
            if long {
                if let Some(minutes) = minutes {units.push(minutes)}
            }

            if units.len() == 0 {
                return "".to_owned();
            }


            return units.join(" ");
        } else {
            "∞".to_owned()
        }
    }

    pub fn is_default(&self) -> bool {
        let mut default_task = Self::default();
        default_task.time = self.time.clone();

        self == &default_task
    }
}
