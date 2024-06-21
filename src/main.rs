use std::{cmp::Ordering, fmt::Display, fs::{self, create_dir, read_to_string, File}, i64};
use std::{io::{self, BufRead, Write, stdin, stdout, Error}, path::{Path, PathBuf}};
use dirs::{self, home_dir};

use chrono::{offset::LocalResult, DateTime, Datelike, Local, NaiveDate, TimeDelta, TimeZone, Timelike, Weekday};
use colored::{ColoredString, Colorize, Color};
use crossterm::{
    cursor,
    execute,
    terminal::{Clear, ClearType},
};

static DAYS_LEFT: i32 = 2;
static TABLE_PATH: &str = ".local/state/rasker/";
static TABLE_NAME: &str = "kek.csv";
static SEPARATOR: &str =        "------------------------------";
static FLUSH_ERROR: &str = "Could not flush to the standard output";
static STDIN_ERROR: &str = "Could not read from the standard input";

#[derive(Debug, Clone)]
struct Task {
    name: String,
    description: String,
    subject: String,
    time: Option<DateTime<Local>>,
    complete: bool
}

impl Task {
    fn get_delta(&self, target: &DateTime<Local>) -> Option<TimeDelta> {
        self.time.clone().map(|v| v - target)
    }

    fn get_delta_now(&self) -> Option<TimeDelta> {
        self.get_delta(&Local::now())
    }

    fn get_status(&self, duration: &Option<TimeDelta>) -> TaskStatus {
        if duration.is_none() {
            return match self.complete {
                true    => TaskStatus::Zen,
                false   => TaskStatus::Normal
            }
        }

        let duration = duration.unwrap();

        if duration.num_days() < DAYS_LEFT as i64 && !self.complete {
            TaskStatus::Panic
        } else if self.complete {
            TaskStatus::Zen
        } else {
            TaskStatus::Normal
        }
    }

    fn get_status_now(&self) -> TaskStatus {
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

    fn read_tasks(path_to_file: &Path) -> io::Result<Vec<Task>> {
        let file_content = read_to_string(path_to_file)?;
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

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Get the task status
        let duration = self.get_delta_now();
        let task_status = self.get_status(&duration);

        // Format completion status
        let complete_str = match self.complete {
            true => "True".green(),
            false => "False".red()
        };

        // Header
        writeln!(f, "{}", self.name.bold())?;
        writeln!(f, "{}", self.description)?;
        writeln!(f, "{}", self.subject)?;

        let duration_text = duration_label(&duration, true);
        let duration_text = date_format(&duration_text, &task_status);
        writeln!(f, "{}", duration_text)?;

        if let Some(time) = self.time {
            writeln!(f, "{}", time.format("%H:%M %d %B %Y"))?;
        }

        // Complete / incomplete
        write!(f, "Completion: {complete_str}")?;

        Ok(())
    }
}

struct Todo {
    tasks: Vec<Task>,
    filename: PathBuf
}

impl Todo {
    fn new(tasks: Vec<Task>, filename: PathBuf) -> Self {
        Self {
            tasks,
            filename
        }
    }

    fn panic_lookup(&self) {
        for task in &self.tasks {
            if let TaskStatus::Panic = task.get_status_now() {
                println!("\nStuff to do:\n");
                self.print_tasks(TaskLayout::Panic, false);
                break;
            }
        }
    }

    fn run(&mut self) {
        let mut state = PromptState::Start;
        loop {
            state = match &state {
                PromptState::Start => self.start_menu(),
                PromptState::Add => self.add_menu(),
                PromptState::Modify => self.modify_menu(),
                PromptState::Delete => self.delete_menu(),
                PromptState::Check => self.check_menu(),
                PromptState::Calendar => {
                    let calendar = Calendar::new(Local::now(), &self.tasks);
                    calendar.render();
                    println!();
                    PromptState::Start
                }
                PromptState::Sort => {
                    self.sort_tasks();
                    self.print_tasks(TaskLayout::Headers, true);
                    PromptState::Start
                }
                PromptState::Write => {
                    match self.save() {
                        Ok(_) => println!("Successfully written!"),
                        _ => println!("Could not write!")
                    }
                    println!();
                    PromptState::Start
                }
                PromptState::Quit => break
            }
        }
    }

    fn start_menu(&self) -> PromptState {
        println!("What do you like to do?");
        println!("1 - List the tasks");
        println!("2 - Check the task");
        println!("3 - Add the task");
        println!("4 - Modify the task");
        println!("5 - Delete the task");
        println!("6 - Calendar");
        println!("7 - Sort the tasks");
        println!("8 - Write the tasks");
        println!("9 - Quit");

        let mut answer;
        loop {
            let input = prompt_input();
            if input.is_none() {
                return PromptState::Quit;
            }

            answer = input.unwrap();

            if answer.len() != 0 {
                break;
            }
        }


        match answer[0].as_str() {
            "1" | "list" | "l" => {
                let command = match answer.get(1) {
                    Some(command) => command,
                    None => "s"
                };

                match command {
                    "all" | "a"     => self.print_tasks(TaskLayout::All, true),
                    "panic" | "p"   => self.print_tasks(TaskLayout::Panic, true),
                    "zen" | "z"     => self.print_tasks(TaskLayout::Zen, true),
                    "normal" | "n"  => self.print_tasks(TaskLayout::Normal, true),
                    "relevant" | "r"=> self.print_tasks(TaskLayout::Relevant, true),
                    "short" | "s"   => self.print_tasks(TaskLayout::Headers, true),
                    _ => ()
                }
            },
            "2" | "check" | "c"     => return PromptState::Check,
            "3" | "add" | "a"       => return PromptState::Add,
            "4" | "modify" | "m"    => return PromptState::Modify,
            "5" | "delete" | "d"    => return PromptState::Delete,
            "6" | "calendar" | "cal"=> return PromptState::Calendar,
            "7" | "sort" | "s"      => return PromptState::Sort,
            "8" | "write"| "w"      => return PromptState::Write,
            "9" | "quit" | "q"      => return PromptState::Quit,
            _ => println!("No such option: {} \n", answer[0])
        };

        PromptState::Start
    }

    fn add_menu(&mut self) -> PromptState {
        clear_screen();
        println!("Adding a new task.");

        let name = ask_with_prefix("Name: ");
        if name.trim().is_empty() {
            eprintln!("Invalid name");
            return PromptState::Start;
        }

        let description = ask_with_prefix("Description: ");
        if description.trim().is_empty() {
            eprintln!("Invalid description");
            return PromptState::Start;
        }

        let subject = ask_with_prefix("Subject: ");
        if subject.trim().is_empty() {
            eprintln!("Invalid subject");
            return PromptState::Start;
        }


        let date_request = ask_with_prefix("Include date (Y/n): ");

        let time = match date_request.trim().to_lowercase().as_str() {
            "yes" | "y" | "" => {
                match ask_date() {
                    Ok(date) => date,
                    Err(_) => return PromptState::Start,
                }
            }
            _ => None
        };


        let complete = ask_with_prefix("Complete: ");
        let complete: bool = match complete.trim().to_lowercase().parse() {
            Ok(complete) => complete,
            Err(_) => false
        };

        let task = Task {
            name:           name.trim().to_owned(),
            description:    description.trim().to_owned(),
            subject:        subject.trim().to_owned(),
            time,
            complete
        };

        println!("{SEPARATOR}\n{task}\n{SEPARATOR}");
        let answer = ask_with_prefix("Are you sure you want to add such task? (Y/n): ");
        match answer.trim().to_lowercase().as_str() {
            "yes" | "y" | "" => self.tasks.push(task),
            _ => ()
        }

        self.print_tasks(TaskLayout::Headers, true);

        PromptState::Start
    }

    fn modify_menu(&mut self) -> PromptState {
        self.print_tasks(TaskLayout::Headers, true);

        println!("Modifying an existing task.");
        let index = ask_index(&self.tasks);
        if index.is_none() {
            return PromptState::Start;
        }

        let index = index.unwrap();

        println!("Modifying the following task:\n{SEPARATOR}\n{}\n{SEPARATOR}", self.tasks[index]);

        let mut prototype = self.tasks[index].clone();

        let name = ask_with_prefix("Name: ");
        if !name.trim().is_empty() {
            prototype.name = name.trim().to_owned();
        }

        let description = ask_with_prefix("Description: ");
        if !description.trim().is_empty() {
            prototype.description = description.trim().to_owned();
        }

        let subject = ask_with_prefix("Subject: ");
        if !subject.trim().is_empty() {
            prototype.subject = subject.trim().to_owned();
        }

        let date_request = ask_with_prefix("Copy date? (Y/n): ");

        match date_request.trim().to_lowercase().as_str() {
            "yes" | "y" | "" => (),
            _ => {
                prototype.time = match ask_date() {
                    Ok(date) => date,
                    Err(_) => return PromptState::Start,
                }
            }
        };


        let complete = ask_with_prefix("Complete: ");

        if !complete.trim().is_empty() {
            prototype.complete = match complete.trim().to_lowercase().parse() {
                Ok(complete) => complete,
                Err(_) => false
            };
        }

        println!("{SEPARATOR}\n{prototype}\n{SEPARATOR}");
        let answer = ask_with_prefix("Are you sure you want to accept the changes? (Y/n): ");
        match answer.trim().to_lowercase().as_str() {
            "yes" | "y" | "" => self.tasks[index] = prototype,
            _ => ()
        }

        self.print_tasks(TaskLayout::Headers, true);

        PromptState::Start
    }

    fn delete_menu(&mut self) -> PromptState {
        self.print_tasks(TaskLayout::Headers, true);
        println!("Which task would you like to delete?");

        if let Some(index) = ask_index(&self.tasks) {
            self.tasks.remove(index);
        }

        self.print_tasks(TaskLayout::Headers, true);

        PromptState::Start
    }

    fn check_menu(&mut self) -> PromptState {
        self.print_tasks(TaskLayout::Headers, true);
        println!("Which task would you like to check?");

        if let Some(index) = ask_index(&self.tasks) {
            self.tasks[index as usize].complete ^= true;
        }

        self.print_tasks(TaskLayout::Headers, true);

        PromptState::Start
    }

    fn save(&self) -> io::Result<()> {
        let answer = ask_with_prefix("\nDo you want to save the tasks? (Y/n): ");
        match answer.trim().to_lowercase().as_str() {
            "y" | "yes" | "" => {
                println!("Saving tasks to the {TABLE_NAME}...");
                self.write_tasks(&self.filename)
            }
            _ => Ok(())
        }
    }

    fn sort_tasks(&mut self) {
        self.tasks.sort_by(|task1, task2| {
            if task1.complete && !task2.complete {
                return Ordering::Less;
            } else if !task1.complete && task2.complete {
                return Ordering::Greater;
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

    fn write_tasks(&self, path_to_file: &Path) -> io::Result<()> {
        let mut to_write = "".to_owned();

        for task in &self.tasks {
            to_write.push_str(&task.name);
            to_write.push(',');
            to_write.push_str(&task.description);
            to_write.push(',');
            to_write.push_str(&task.subject);
            to_write.push(',');
            // to_write.push_str(&task.time.to_rfc3339());
            to_write.push_str(&task.time.map_or("None".to_owned(), |v| v.to_rfc3339()));
            to_write.push(',');
            to_write.push_str(&task.complete.to_string());
            to_write.push('\n');
        }

        fs::write(path_to_file, to_write)?;

        Ok(())
    }

    fn print_tasks(&self, task_layout: TaskLayout, clear: bool) {
        if clear {
            clear_screen();
        }

        if matches!(task_layout, TaskLayout::Headers) {
            for (i, task) in self.tasks.iter().enumerate() {
                let duration = task.get_delta_now();
                let task_status = task.get_status(&duration);
                let mut name_and_time = task.name.clone();
                name_and_time.push_str(" (");
                name_and_time.push_str(&duration_label(&duration, false));
                name_and_time.push(')');

                let name_and_time = date_format(&name_and_time, &task_status);
                println!("{}. {}", i + 1, &name_and_time);
            }
            println!("{SEPARATOR}");
            return;
        }


        let predicate: Box<dyn Fn(&Task) -> bool> = match task_layout {
            TaskLayout::All => Box::new(|_| true),
            TaskLayout::Normal =>
                Box::new(
                    |v| matches!(v.get_status_now(), TaskStatus::Normal)),
            TaskLayout::Zen =>
                Box::new(
                    |v| matches!(v.get_status_now(), TaskStatus::Zen)),
            TaskLayout::Panic =>
                Box::new(
                    |v| matches!(v.get_status_now(), TaskStatus::Panic)),
            TaskLayout::Relevant =>
                Box::new(
                    |v| matches!(v.get_status_now(), TaskStatus::Panic
                                 | TaskStatus::Normal)),
            TaskLayout::Headers => Box::new(|_| false)
        };

        for task in self.tasks.iter().filter(|v| predicate(v)) {
            println!("{task}");
            println!("{SEPARATOR}");
        }
    }
}


struct Calendar<'a> {
    date: DateTime<Local>,
    tasks: &'a Vec<Task>
}

impl<'a> Calendar<'a> {
    fn new(date: DateTime<Local>, tasks: &'a Vec<Task>) -> Self {
        Calendar { date, tasks }
    }

    fn render(&self) {
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


enum TaskStatus {
    Panic,
    Normal,
    Zen,
}

enum PromptState {
    Start,
    Add,
    Modify,
    Delete,
    Check,
    Calendar,
    Write,
    Sort,
    Quit
}

enum TaskLayout {
    All,
    Normal,
    Panic,
    Zen,
    Relevant,
    Headers
}


fn duration_label(duration: &Option<TimeDelta>, long: bool) -> String {
    if let Some(duration) = duration {
        // Format time until the task
        let days    = duration.num_days() as i32;
        let hours   = duration.num_hours() as i32 - duration.num_days() as i32 * 24;
        let minutes = duration.num_minutes() as i32 - duration.num_hours() as i32 * 60;

        let days = time_quantity_format("day", days);
        let hours = time_quantity_format("hour", hours);
        let minutes = time_quantity_format("minute", minutes);

        let mut units = vec![];
        if let Some(days) = days {units.push(days)}
        if let Some(hours) = hours {units.push(hours)}

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

fn clear_screen() {
    // println!("\n{BOLD_SEPARATOR}");
    // print!("\x1B[2J\x1B[1;1H");
    // stdout().flush().expect(FLUSH_ERROR);
    execute!(stdout(), Clear(ClearType::All)).expect(FLUSH_ERROR);
    execute!(stdout(), crossterm::terminal::Clear(ClearType::Purge)).unwrap();
    execute!(
        stdout(),
        Clear(ClearType::All),  // Clear the entire visible screen
        cursor::MoveTo(0, 0)            // Move cursor to the top-left corner
    ).unwrap();
}


fn date_format(str: &str, task_status: &TaskStatus) -> ColoredString {
    match task_status {
        TaskStatus::Panic => str.red(),
        TaskStatus::Normal => str.bright_blue(),
        TaskStatus::Zen => str.white()
    }
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




fn prompt_input() -> Option<Vec<String>> {
    let input = ask_with_prefix("> ");
    if input == "" {
        None
    } else {
        Some(input
            .to_lowercase()
            .split(" ")
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| v.to_owned())
            .collect::<Vec<String>>())
    }

}


fn ask_with_prefix(prefix: &str) -> String {
    print!("{}", prefix);
    stdout().flush().expect(FLUSH_ERROR);

    let mut buf = "".to_owned();
    stdin().lock()
        .read_line(&mut buf)
        .expect(STDIN_ERROR);

    buf
}



fn ask_index(tasks: &Vec<Task>) -> Option<usize> {
    let choice = ask_with_prefix("> ");
    let index: i32 = match choice.trim().parse::<i32>() {
        Ok(index) => index - 1,
        Err(_) => {
            eprintln!("Could not parse usize\n");
            return None;
        }
    };

    if index < 0 || index >= tasks.len() as i32 {
        eprintln!("Out of bounds: {index}\n");
        return None;
    }

    Some(index as usize)
}



fn ask_number_date(prefix: &str) -> Option<i32> {
    let number = ask_with_prefix(prefix);
    let trimmed = number.trim();
    match trimmed.parse::<i32>() {
        Ok(v) => Some(v),
        Err(_) => {
            if trimmed == "" {
                None
            } else {
                Some(-1)
            }
        }
    }
}

fn ask_date() -> Result<Option<DateTime<Local>>, ()> {
    let now = Local::now();
    let year = ask_number_date("Year: ").unwrap_or(now.year() as i32);
    let month = ask_number_date("Month: ").unwrap_or(now.month() as i32);
    if month < 1 || month > 12 {
        eprintln!("Invalid month value: {month}");
        return Err(());
    }

    let day = ask_number_date("Day: ").unwrap_or(now.day() as i32);
    if day < 1 || day > 31 {
        eprintln!("Invalid day value: {day}");
        return Err(());
    }

    let hour = ask_number_date("Hour: ").unwrap_or(now.hour() as i32);
    if hour > 23 {
        eprintln!("Invalid hour value: {hour}");
        return Err(());
    }

    let min = ask_number_date("Minute: ").unwrap_or(now.minute() as i32);
    if min > 59 {
        eprintln!("Invalid minute value: {min}");
    }

    match Local.with_ymd_and_hms(year, month as u32, day as u32, hour as u32, min as u32, 0) {
        chrono::offset::LocalResult::Single(date) => Ok(Some(date)),
        chrono::offset::LocalResult::Ambiguous(_, _) |
            chrono::offset::LocalResult::None => {

            eprintln!("Ambigious or invalid date");
            Err(())
        }
    }
}



fn main() -> io::Result<()> {
    clear_screen();
    let mut dir = match home_dir() {
        Some(dir) => dir,
        None => return Err(Error::other("Could not find a home directory")),
    };

    dir.push(Path::new(TABLE_PATH));

    // prologue
    if !dir.exists() {
        println!("Creating dir");
        create_dir(&dir)?;
        println!("Created dir");
    }

    dir.push(Path::new(TABLE_NAME));
    if !dir.exists() {
        File::create(&dir)?;
    }

    println!("Loading tasks from {TABLE_NAME}...");
    let tasks = Task::read_tasks(Path::new(&dir))?;
    let mut app = Todo::new(tasks, dir.to_owned());

    app.panic_lookup();
    app.run();
    app.save()?;

    Ok(())
}
