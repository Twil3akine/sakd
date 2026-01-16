use clap::Parser;
use cli::{Cli, Commands};
use inquire::{Confirm, Select, Text};
use std::process;
use tabled::{Table, Tabled};
use chrono::{DateTime, Utc};

mod db;
mod cli;

#[derive(Tabled)]
struct TaskDisplay {
    id: i64,
    title: String,
    is_done: String,
    limit: String,
    description: String,
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
            let limit_dt = limit.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
            
            db::add_task(&conn, &title, limit_dt, description).unwrap();
            println!("Task added: {}", title);
        }
        Some(Commands::List { order: _ }) => {
            let tasks = db::get_tasks(&conn).unwrap();
            let display_tasks: Vec<TaskDisplay> = tasks.into_iter().map(|t| TaskDisplay {
                id: t.id,
                title: t.title,
                is_done: if t.is_done { "v".to_string() } else { " ".to_string() },
                limit: t.limit.map(|l| l.to_rfc3339()).unwrap_or_default(),
                description: t.description.unwrap_or_default(),
            }).collect();
            println!("{}", Table::new(display_tasks).to_string());
        }
        Some(Commands::Delete { id }) => {
            let id = resolve_id(&conn, id);
            if let Some(id) = id {
                if Confirm::new("Are you sure?").with_default(false).prompt().unwrap_or(false) {
                    db::delete_task(&conn, id).unwrap();
                    println!("Task {} deleted.", id);
                }
            }
        }
        Some(Commands::Show { id }) => {
            let id = resolve_id(&conn, id);
            if let Some(id) = id {
                if let Some(task) = db::get_task(&conn, id).unwrap() {
                    println!("ID: {}", task.id);
                    println!("Title: {}", task.title);
                    println!("Done: {}", task.is_done);
                    println!("Limit: {:?}", task.limit);
                    println!("Description: {:?}", task.description);
                }
            }
        }
        Some(Commands::Edit { id }) => {
            let id = resolve_id(&conn, id);
            if let Some(id) = id {
                if let Some(mut task) = db::get_task(&conn, id).unwrap() {
                    task.title = Text::new("Title:").with_default(&task.title).prompt().unwrap();
                    task.is_done = Confirm::new("Completed?").with_default(task.is_done).prompt().unwrap();
                    db::update_task(&conn, &task).unwrap();
                    println!("Task {} updated.", id);
                }
            }
        }
        None => {
            // Interactive mode if no command given
            let options = vec!["List", "Add", "Quit"];
            let ans = Select::new("Choose an action:", options).prompt().unwrap();
            match ans {
                "List" => {
                    let tasks = db::get_tasks(&conn).unwrap();
                    let display_tasks: Vec<TaskDisplay> = tasks.into_iter().map(|t| TaskDisplay {
                        id: t.id,
                        title: t.title,
                        is_done: if t.is_done { "v".to_string() } else { " ".to_string() },
                        limit: t.limit.map(|l| l.to_rfc3339()).unwrap_or_default(),
                        description: t.description.unwrap_or_default(),
                    }).collect();
                    println!("{}", Table::new(display_tasks).to_string());
                }
                "Add" => {
                    let title = Text::new("Task title:").prompt().unwrap();
                    db::add_task(&conn, &title, None, None).unwrap();
                    println!("Task added.");
                }
                _ => process::exit(0),
            }
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

    let options: Vec<String> = tasks.iter().map(|t| format!("{}: {}", t.id, t.title)).collect();
    let ans = Select::new("Select a task:", options).prompt().ok()?;
    let id_str = ans.split(':').next()?;
    id_str.parse().ok()
}
