use clap::Parser;
use cli::{Cli, Commands};
use inquire::{Confirm, Select, Text};
use std::process;
use tabled::{Table, Tabled, settings::Style};
use chrono::{DateTime, Utc, Local, NaiveDate, NaiveTime, NaiveDateTime, TimeZone};
use colored::*;

mod db;
mod cli;

#[derive(Tabled)]
struct TaskDisplay {
    title: String,
    #[tabled(rename = "limit")]
    limit_display: String,
}

#[derive(Tabled)]
struct TaskDisplayFull {
    #[tabled(rename = "v")]
    is_done: String,
    title: String,
    #[tabled(rename = "limit")]
    limit_display: String,
}

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
                prompt_limit(None, true)
            };

            let description = if description.is_some() {
                description
            } else if Confirm::new("Add a description?").with_default(false).prompt().unwrap_or(false) {
                Some(Text::new("Description:").prompt().unwrap_or_default())
            } else {
                None
            };
            
            db::add_task(&conn, &title, limit_dt, description).unwrap();
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
        None => {
            // Interactive mode if no command given
            loop {
                // Request order: List, Add, Done, Show, Edit, Remove, Quit
                let options = vec!["List", "Add", "Done", "Show", "Edit", "Remove", "Quit"];
                let ans = Select::new("Choose an action:", options).prompt().unwrap_or("Quit");
                match ans {
                    "List" => {
                        let tasks = db::get_tasks(&conn).unwrap();
                        let all = Confirm::new("Show completed tasks?").with_default(false).prompt().unwrap_or(false);
                        print_tasks(tasks, all);
                    }
                    "Add" => {
                        let title = Text::new("Task title:").prompt().unwrap_or_default();
                        if !title.is_empty() {
                            let limit = prompt_limit(None, true);
                            let description = if Confirm::new("Add a description?").with_default(false).prompt().unwrap_or(false) {
                                Some(Text::new("Description:").prompt().unwrap_or_default())
                            } else {
                                None
                            };
                            db::add_task(&conn, &title, limit, description).unwrap();
                            println!("Task added.");
                        }
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

fn interactive_edit(conn: &rusqlite::Connection, id: i64) {
    if let Some(mut task) = db::get_task(conn, id).unwrap() {
        task.title = Text::new("Title:").with_default(&task.title).prompt().unwrap_or(task.title);
        if Confirm::new("Edit limit?").with_default(false).prompt().unwrap_or(false) {
            // Pass false here because "Edit limit?" was already confirmed above
            task.limit = prompt_limit(task.limit, false);
        }
        if Confirm::new("Edit description?").with_default(false).prompt().unwrap_or(false) {
            let current_desc = task.description.clone().unwrap_or_default();
            task.description = Some(Text::new("Description:").with_default(&current_desc).prompt().unwrap_or(current_desc));
        }
        db::update_task(conn, &task).unwrap();
        println!("Task updated.");
    }
}

fn prompt_limit(current: Option<DateTime<Utc>>, need_confirm: bool) -> Option<DateTime<Utc>> {
    if need_confirm && !Confirm::new("Set/Change a limit?").with_default(false).prompt().unwrap_or(false) {
        return current;
    }

    let date_str = Text::new("Date (YYYY-MM-DD):")
        .with_default(&Local::now().format("%Y-%m-%d").to_string())
        .prompt()
        .ok()?;
    
    let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok()?;

    if Confirm::new("Set a specific time?").with_default(false).prompt().unwrap_or(false) {
        let time_str = Text::new("Time (HH:MM):")
            .with_default("12:00")
            .prompt()
            .ok()?;
        let time = NaiveTime::parse_from_str(&time_str, "%H:%M").ok()?;
        let dt = NaiveDateTime::new(date, time);
        Local.from_local_datetime(&dt).single().map(|dt| dt.with_timezone(&Utc))
    } else {
        let time = NaiveTime::from_hms_opt(23, 59, 59).unwrap();
        let dt = NaiveDateTime::new(date, time);
        Local.from_local_datetime(&dt).single().map(|dt| dt.with_timezone(&Utc))
    }
}

fn format_limit_color(limit: Option<DateTime<Utc>>) -> String {
    match limit {
        Some(l) => {
            let now = Utc::now();
            let local_l = l.with_timezone(&Local);
            let s = local_l.format("%Y-%m-%d %H:%M").to_string();
            if l < now {
                s.red().to_string()
            } else if l < now + chrono::Duration::days(1) {
                s.yellow().to_string()
            } else {
                s.green().to_string()
            }
        }
        None => "None".to_string(),
    }
}

fn print_tasks(tasks: Vec<db::Task>, show_all: bool) {
    if show_all {
        let display_tasks: Vec<TaskDisplayFull> = tasks.into_iter().map(|t| TaskDisplayFull {
            is_done: if t.is_done { "v".green().to_string() } else { "-".red().to_string() },
            title: t.title,
            limit_display: format_limit_color(t.limit),
        }).collect();
        let mut table = Table::new(display_tasks);
        table.with(Style::blank());
        
        let table_str = table.to_string();
        let lines: Vec<&str> = table_str.lines().collect();
        if lines.len() > 1 {
            let header_len = lines[0].len();
            let separator = "-".repeat(header_len);
            println!("{}", lines[0]);
            println!("{}", separator);
            for line in lines.iter().skip(1) {
                println!("{}", line);
            }
        } else {
            println!("{}", table_str);
        }
    } else {
        let display_tasks: Vec<TaskDisplay> = tasks.into_iter()
            .filter(|t| !t.is_done)
            .map(|t| TaskDisplay {
                title: t.title,
                limit_display: format_limit_color(t.limit),
            }).collect();
        let mut table = Table::new(display_tasks);
        table.with(Style::blank());
        
        let table_str = table.to_string();
        let lines: Vec<&str> = table_str.lines().collect();
        if lines.len() > 1 {
            let header_len = lines[0].len();
            let separator = "-".repeat(header_len);
            println!("{}", lines[0]);
            println!("{}", separator);
            for line in lines.iter().skip(1) {
                println!("{}", line);
            }
        } else {
            println!("{}", table_str);
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
