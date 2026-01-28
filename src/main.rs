use clap::Parser;
use cli::{Cli, Commands};
use inquire::{Confirm, Select, Text};
use std::process;
use chrono::{DateTime, Utc, Local, NaiveDate, NaiveTime, NaiveDateTime, TimeZone};
use colored::*;
use unicode_width::UnicodeWidthStr;

mod db;
mod cli;
mod tui;
mod utils;

fn main() {
    let cli = Cli::parse();
    let conn = db::init_db().unwrap_or_else(|e| {
        eprintln!("Failed to initialize database: {}", e);
        process::exit(1);
    });

    match cli.command {
        Some(Commands::Add { title, limit, description }) => {
            let title = title.unwrap_or_else(|| {
                Text::new("Task title:").prompt().unwrap_or_else(|_| process::exit(0))
            });
            
            let limit_dt = if limit.is_some() {
                limit.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)))
            } else {
                prompt_limit(None)
            };

            let description = if description.is_some() {
                description
            } else {
                let desc = Text::new("Description:").prompt().unwrap_or_default();
                if desc.is_empty() { None } else { Some(desc) }
            };
            
            db::add_task(&conn, &title, limit_dt, description, None, None).unwrap();
            println!("Task added: {}\n", title);
        }
        Some(Commands::Done { id }) => {
            let id = resolve_id(&conn, id);
            if let Some(id) = id {
                if let Some(mut task) = db::get_task(&conn, id).unwrap() {
                    task.is_done = true;
                    db::update_task(&conn, &task).unwrap();
                    println!("Task marked as done.\n");
                }
            }
        }
        Some(Commands::List { all, order: _ }) => {
            let tasks = db::get_tasks(&conn).unwrap();
            print_tasks(tasks, all);
            println!();
        }
        Some(Commands::Remove { id }) => {
            let id = resolve_id(&conn, id);
            if let Some(id) = id {
                if Confirm::new("Are you sure you want to remove this task?").with_default(false).prompt().unwrap_or(false) {
                    db::delete_task(&conn, id).unwrap();
                    println!("Task removed.\n");
                }
            }
        }
        Some(Commands::Show { id }) => {
            let id = resolve_id(&conn, id);
            if let Some(id) = id {
                if let Some(task) = db::get_task(&conn, id).unwrap() {
                    println!("\n{}", "--- Task Details ---".cyan().bold());
                    println!("{}: {}", "ID".bold(), task.id);
                    println!("{}: {}", "Title".bold(), task.title);
                    println!("{}: {}", "Done".bold(), if task.is_done { "Yes".green() } else { "No".red() });
                    println!("{}: {}", "Limit".bold(), format_limit_color(task.limit));
                    println!("{}: {}", "Description".bold(), task.description.unwrap_or_else(|| "None".to_string()));
                    println!();
                }
            }
        }
        Some(Commands::Edit { id }) => {
            let id = resolve_id(&conn, id);
            if let Some(id) = id {
                interactive_edit(&conn, id);
                println!();
            }
        }
        Some(Commands::Tui) => {
            loop {
                match tui::run_tui(&conn).expect("TUI error") {
                    tui::TuiEvent::Quit => break,
                }
            }
        }
        None => {
            // Interactive mode if no command given
            loop {
                // Order: List, Add, Done, Show, Edit, Remove, Tui, Quit
                let options = vec!["List", "Add", "Done", "Show", "Edit", "Remove", "Tui", "Quit"];
                let ans = Select::new("Choose an action:", options).prompt().unwrap_or("Quit");
                match ans {
                    "List" => {
                        let tasks = db::get_tasks(&conn).unwrap();
                        let all = Confirm::new("Show completed tasks?").with_default(false).prompt().unwrap_or(false);
                        print_tasks(tasks, all);
                    }
                    "Add" => {
                        interactive_add(&conn);
                    }
                    "Done" => {
                        if let Some(id) = resolve_id(&conn, None) {
                            if let Some(mut task) = db::get_task(&conn, id).unwrap() {
                                task.is_done = true;
                                db::update_task(&conn, &task).unwrap();
                                println!("Task marked as done.");
                            }
                        }
                    }
                    "Show" => {
                        if let Some(id) = resolve_id(&conn, None) {
                            if let Some(task) = db::get_task(&conn, id).unwrap() {
                                println!("\n{}", "--- Task Details ---".cyan().bold());
                                println!("{}: {}", "ID".bold(), task.id);
                                println!("{}: {}", "Title".bold(), task.title);
                                println!("{}: {}", "Done".bold(), if task.is_done { "Yes".green() } else { "No".red() });
                                println!("{}: {}", "Limit".bold(), format_limit_color(task.limit));
                                println!("{}: {}", "Description".bold(), task.description.unwrap_or_else(|| "None".to_string()));
                            }
                        }
                    }
                    "Edit" => {
                        if let Some(id) = resolve_id(&conn, None) {
                            interactive_edit(&conn, id);
                        }
                    }
                    "Tui" => {
                        loop {
                            match tui::run_tui(&conn).expect("TUI error") {
                                tui::TuiEvent::Quit => break,
                            }
                        }
                    }
                    "Remove" => {
                        if let Some(id) = resolve_id(&conn, None) {
                            if Confirm::new("Are you sure?").with_default(false).prompt().unwrap_or(false) {
                                db::delete_task(&conn, id).unwrap();
                                println!("Task removed.");
                            }
                        }
                    }
                    _ => break,
                }
                println!(); // Add a newline after each action
            }
        }
    }
}

fn interactive_add(conn: &rusqlite::Connection) {
    let title = Text::new("Task title:").prompt().unwrap_or_default();
    if !title.is_empty() {
        let limit = prompt_limit(None);
        let desc = Text::new("Description:").prompt().unwrap_or_default();
        let description = if desc.is_empty() { None } else { Some(desc) };
        db::add_task(conn, &title, limit, description, None, None).unwrap();
        println!("Task added.");
    }
}

fn interactive_edit(conn: &rusqlite::Connection, id: i64) {
    if let Some(mut task) = db::get_task(conn, id).unwrap() {
        task.title = Text::new("Title:").with_default(&task.title).prompt().unwrap_or(task.title);
        
        task.limit = prompt_limit(task.limit);
        
        let current_desc = task.description.clone().unwrap_or_default();
        let desc = Text::new("Description:").with_default(&current_desc).prompt().unwrap_or(current_desc);
        task.description = if desc.is_empty() { None } else { Some(desc) };

        db::update_task(conn, &task).unwrap();
        println!("Task updated.");
    }
}

fn prompt_limit(current: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
    let current_local = current.map(|c| c.with_timezone(&Local));
    let now = Local::now();
    let default_date = current_local
        .map(|c| c.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| now.format("%Y-%m-%d").to_string());

    let date_str = Text::new("Date (YYYY-MM-DD/Shortcut):")
        .with_help_message("Shortcuts: t (today), tm (tomorrow), 2d, 1w, mon-sun")
        .with_default(&default_date)
        .prompt()
        .ok()?;
    
    if date_str.trim().is_empty() {
        return None;
    }

    let date = utils::parse_shortcut_date(&date_str).or_else(|| NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok())?;

    let default_time = current_local
        .map(|c| c.format("%H:%M").to_string())
        .unwrap_or_else(|| "23:59".to_string());

    let time_str = Text::new("Time (HH:MM/Shortcut):")
        .with_help_message("Shortcuts: last (23:59), morning (09:00), noon (12:00), 1h")
        .with_default(&default_time)
        .prompt()
        .ok()?;

    let time = if time_str.trim().is_empty() {
        NaiveTime::from_hms_opt(23, 59, 0).unwrap()
    } else {
        utils::parse_shortcut_time(&time_str).or_else(|| NaiveTime::parse_from_str(&time_str, "%H:%M").ok())
            .unwrap_or_else(|| NaiveTime::from_hms_opt(23, 59, 0).unwrap())
    };

    let dt = NaiveDateTime::new(date, time);
    Local.from_local_datetime(&dt).single().map(|dt| dt.with_timezone(&Utc))
}

// Shortcuts moved to utils.rs

fn format_limit_color(limit: Option<DateTime<Utc>>) -> String {
    match limit {
        Some(l) => {
            let now = Utc::now();
            let local_l = l.with_timezone(&Local);
            let s = local_l.format("%Y-%m-%d %H:%M").to_string();
            
            if l < now {
                // Overdue: Bright Magenta and Bold
                s.magenta().bold().to_string()
            } else if l < now + chrono::Duration::days(1) {
                // Today: Red
                s.red().to_string()
            } else if l < now + chrono::Duration::days(3) {
                // Within 3 days: Yellow
                s.yellow().to_string()
            } else if l < now + chrono::Duration::days(7) {
                // Within 1 week: Green
                s.green().to_string()
            } else {
                // Beyond 1 week: Grey (using bright_black for grey)
                s.bright_black().to_string()
            }
        }
        None => "None".bright_black().to_string(),
    }
}

fn pad_title(title: &str, width: usize) -> String {
    let title_width = title.width();
    if title_width >= width {
        title.to_string()
    } else {
        format!("{}{}", title, " ".repeat(width - title_width))
    }
}

fn print_tasks(tasks: Vec<db::Task>, show_all: bool) {
    if show_all {
        println!("  v    {}  {}", pad_title("title", 25), "limit");
        println!("--------------------------------------------------");
        for t in tasks {
            let status = if t.is_done { "v".green() } else { "-".red() };
            let limit = format_limit_color(t.limit);
            println!("  {}    {}  {}", status, pad_title(&t.title, 25), limit);
        }
    } else {
        println!("  {}  {}", pad_title("title", 25), "limit");
        println!("--------------------------------------------------");
        for t in tasks.into_iter().filter(|t| !t.is_done) {
            let limit = format_limit_color(t.limit);
            println!("  {}  {}", pad_title(&t.title, 25), limit);
        }
    }
}

fn resolve_id(conn: &rusqlite::Connection, id: Option<i64>) -> Option<i64> {
    if let Some(id) = id {
        if db::get_task(conn, id).unwrap().is_some() {
            return Some(id);
        }
        println!("ID {} not found.", id);
    }

    let tasks = db::get_tasks(conn).unwrap();
    if tasks.is_empty() {
        println!("No tasks available.");
        return None;
    }

    let options: Vec<String> = tasks.iter()
        .filter(|t| !t.is_done)
        .map(|t| format!("{}: {}", t.id, t.title)).collect();
    
    if options.is_empty() {
        println!("No active tasks available.");
        return None;
    }

    let ans = Select::new("Select a task:", options).prompt().ok()?;
    let id_str = ans.split(':').next()?;
    id_str.parse().ok()
}
