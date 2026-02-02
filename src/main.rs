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
        Some(Commands::Add { title, limit, description, priority, tags, dep }) => {
            let title = title.unwrap_or_else(|| {
                Text::new("Task title:").prompt().unwrap_or_else(|_| process::exit(0))
            });
            
            let priority_val = if let Some(p) = priority {
                utils::parse_priority(&p)
            } else {
                let options = vec![" ", "Low", "Medium", "High"];
                let ans = Select::new("Priority:", options).prompt().unwrap_or(" ");
                if ans == " " { db::Priority::None } else { utils::parse_priority(ans) }
            };

            let tags_val = if let Some(t) = tags {
                utils::parse_tags(&t)
            } else {
                let ans = Text::new("Tags (comma separated):").prompt().unwrap_or_default();
                utils::parse_tags(&ans)
            };

            let dep_val = if let Some(d) = dep {
                d.split(',').filter_map(|s| s.trim().parse::<i64>().ok()).collect()
            } else {
                let ans = Text::new("Dependencies (comma separated IDs):").prompt().unwrap_or_default();
                ans.split(',').filter_map(|s| s.trim().parse::<i64>().ok()).collect()
            };

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
            
            db::add_task(&conn, &title, limit_dt, description, priority_val, tags_val, dep_val).unwrap();
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
        Some(Commands::List { all, tag, priority, order: _ }) => {
            let mut tasks = db::get_tasks(&conn).unwrap();
            
            if let Some(t) = tag {
                tasks.retain(|task| task.tags.iter().any(|tag_str| tag_str.contains(&t)));
            }
            if let Some(p) = priority {
                let p_val = utils::parse_priority(&p);
                tasks.retain(|task| task.priority == p_val);
            }

            print_tasks(&tasks, all);
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
                    println!("{}: {}", "Priority".bold(), format!("{:?}", task.priority));
                    println!("{}: {}", "Tags".bold(), task.tags.join(", "));
                    if !task.dependencies.is_empty() {
                        println!("{}: {:?}", "Depends on".bold(), task.dependencies);
                        let all_tasks = db::get_tasks(&conn).unwrap();
                        if utils::has_incomplete_dependencies(&task, &all_tasks) {
                            println!("{}", "Warning: Some dependencies are not completed!".yellow().bold());
                        }
                    }
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
                let options = vec!["一覧", "追加", "完了", "詳細", "編集", "削除", "Tui形式", "終了"];
                let ans = Select::new("アクションを選択してください:", options).prompt().unwrap_or("終了");
                match ans {
                    "一覧" => {
                        let tasks = db::get_tasks(&conn).unwrap();
                        let all = Confirm::new("完了したタスクも含めますか？").with_default(false).prompt().unwrap_or(false);
                        print_tasks(&tasks, all);
                    }
                    "追加" => {
                        interactive_add(&conn);
                    }
                    "完了" => {
                        if let Some(id) = resolve_id(&conn, None) {
                            if let Some(mut task) = db::get_task(&conn, id).unwrap() {
                                task.is_done = true;
                                db::update_task(&conn, &task).unwrap();
                                println!("タスクを完了にしました。");
                            }
                        }
                    }
                    "詳細" => {
                        if let Some(id) = resolve_id(&conn, None) {
                            if let Some(task) = db::get_task(&conn, id).unwrap() {
                                println!("\n{}", "--- タスク詳細 ---".cyan().bold());
                                println!("{}: {}", "ID".bold(), task.id);
                                println!("{}: {}", "タイトル".bold(), task.title);
                                println!("{}: {}", "優先度".bold(), format!("{:?}", task.priority));
                                println!("{}: {}", "タグ".bold(), task.tags.join(", "));
                                println!("{}: {}", "依存関係".bold(), task.dependencies.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "));
                                println!("{}: {}", "完了".bold(), if task.is_done { "はい".green() } else { "いいえ".red() });
                                println!("{}: {}", "期限".bold(), format_limit_color(task.limit));
                                println!("{}: {}", "詳細".bold(), task.description.unwrap_or_else(|| "なし".to_string()));
                            }
                        }
                    }
                    "編集" => {
                        if let Some(id) = resolve_id(&conn, None) {
                            interactive_edit(&conn, id);
                        }
                    }
                    "Tui形式" => {
                        loop {
                            match tui::run_tui(&conn).expect("TUI error") {
                                tui::TuiEvent::Quit => break,
                            }
                        }
                    }
                    "削除" => {
                        if let Some(id) = resolve_id(&conn, None) {
                            if Confirm::new("本当によろしいですか？").with_default(false).prompt().unwrap_or(false) {
                                db::delete_task(&conn, id).unwrap();
                                println!("タスクを削除しました。");
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
        let priority_options = vec![" ", "Low", "Medium", "High"];
        let priority_ans = Select::new("優先度:", priority_options).prompt().unwrap_or(" ");
        let priority = if priority_ans == " " { db::Priority::None } else { utils::parse_priority(priority_ans) };

        let tags_ans = Text::new("タグ (カンマ区切り):").prompt().unwrap_or_default();
        let tags = utils::parse_tags(&tags_ans);

        let dep_ans = Text::new("依存ID (カンマ区切り数値):").prompt().unwrap_or_default();
        let dependencies = dep_ans.split(',').filter_map(|s| s.trim().parse::<i64>().ok()).collect();

        let limit = prompt_limit(None);
        let desc = Text::new("詳細:").prompt().unwrap_or_default();
        let description = if desc.is_empty() { None } else { Some(desc) };
        db::add_task(conn, &title, limit, description, priority, tags, dependencies).unwrap();
        println!("タスクを追加しました。");
    }
}

fn interactive_edit(conn: &rusqlite::Connection, id: i64) {
    if let Some(mut task) = db::get_task(conn, id).unwrap() {
        task.title = Text::new("タイトル:").with_default(&task.title).prompt().unwrap_or(task.title);
        
        let priority_options = vec![" ", "Low", "Medium", "High"];
        let priority_ans = Select::new("優先度:", priority_options).with_starting_cursor(match task.priority {
            db::Priority::None => 0,
            db::Priority::Low => 1,
            db::Priority::Medium => 2,
            db::Priority::High => 3,
        }).prompt().unwrap_or(" ");
        task.priority = if priority_ans == " " { db::Priority::None } else { utils::parse_priority(priority_ans) };

        let tags_str = task.tags.join(", ");
        let tags_ans = Text::new("タグ (カンマ区切り):").with_default(&tags_str).prompt().unwrap_or(tags_str);
        task.tags = utils::parse_tags(&tags_ans);

        let dep_str = task.dependencies.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ");
        let dep_ans = Text::new("依存ID (カンマ区切り数値):").with_default(&dep_str).prompt().unwrap_or(dep_str);
        task.dependencies = dep_ans.split(',').filter_map(|s| s.trim().parse::<i64>().ok()).collect();

        task.limit = prompt_limit(task.limit);
        
        let current_desc = task.description.clone().unwrap_or_default();
        let desc = Text::new("詳細:").with_default(&current_desc).prompt().unwrap_or(current_desc);
        task.description = if desc.is_empty() { None } else { Some(desc) };

        db::update_task(conn, &task).unwrap();
        println!("タスクを更新しました。");
    }
}

fn prompt_limit(current: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
    let current_local = current.map(|c| c.with_timezone(&Local));
    let (default_date, date_help) = if let Some(local) = current_local {
        (local.format("%Y/%m/%d").to_string(), " (Enterで現在値を維持)")
    } else {
        (String::new(), " (空欄で指定なし)")
    };

    let date_str = Text::new("日付 (YYYY/MM/DD または MM/DD/省略形式):")
        .with_help_message(&format!("省略形式: t (今日), tm (明日), 2d, 1w, mon-sun{}", date_help))
        .with_default(&default_date)
        .prompt()
        .ok()?;
    
    if date_str.trim().is_empty() {
        return None;
    }

    let date = utils::parse_shortcut_date(&date_str)
        .or_else(|| NaiveDate::parse_from_str(&date_str, "%Y/%m/%d").ok())
        .or_else(|| {
            let now = Local::now().date_naive();
            NaiveDate::parse_from_str(&date_str, "%m/%d").ok()
                .map(|d| NaiveDate::from_ymd_opt(now.year(), d.month(), d.day())).flatten()
        })?;

    let (default_time, time_help) = if let Some(local) = current_local {
        (local.format("%H:%M").to_string(), " (Enterで現在値を維持)")
    } else {
        ("23:59".to_string(), " (Enterで23:59)")
    };

    let time_str = Text::new("時刻 (HH:MM/省略形式):")
        .with_help_message(&format!("省略形式: last (23:59), morning (09:00), noon (12:00), 1h{}", time_help))
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
            let s = local_l.format("%Y/%m/%d %H:%M").to_string();
            
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
        None => "なし".bright_black().to_string(),
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

fn print_tasks(tasks: &[db::Task], show_all: bool) {
    let all_tasks = tasks.to_vec();
    if show_all {
        println!("  st  P   {}  {}", pad_title("タイトル", 25), "期限");
        println!("------------------------------------------------------------");
        for t in tasks {
            let status = if t.is_done { "v ".green() } else { "- ".red() };
            let prio = t.priority.to_symbol();
            let limit = format_limit_color(t.limit);
            let dep_warn = if utils::has_incomplete_dependencies(t, &all_tasks) { "*".yellow().bold() } else { " ".normal() };
            println!("  {} {} {} {}  {}", status, prio, dep_warn, pad_title(&t.title, 25), limit);
        }
    } else {
        println!("  P   {}  {}", pad_title("タイトル", 25), "期限");
        println!("------------------------------------------------------------");
        for t in tasks.iter().filter(|t| !t.is_done) {
            let prio = t.priority.to_symbol();
            let limit = format_limit_color(t.limit);
            let dep_warn = if utils::has_incomplete_dependencies(t, &all_tasks) { "*".yellow().bold() } else { " ".normal() };
            println!("  {} {} {}  {}", prio, dep_warn, pad_title(&t.title, 25), limit);
        }
    }
}

fn resolve_id(conn: &rusqlite::Connection, id: Option<i64>) -> Option<i64> {
    if let Some(id) = id {
        if db::get_task(conn, id).unwrap().is_some() {
            return Some(id);
        }
        println!("ID {} が見つかりません。", id);
    }

    let tasks = db::get_tasks(conn).unwrap();
    if tasks.is_empty() {
        println!("タスクがありません。");
        return None;
    }

    let options: Vec<String> = tasks.iter()
        .filter(|t| !t.is_done)
        .map(|t| format!("{}: {}", t.id, t.title)).collect();
    
    if options.is_empty() {
        println!("実行中のタスクがありません。");
        return None;
    }

    let ans = Select::new("タスクを選択してください:", options).prompt().ok()?;
    let id_str = ans.split(':').next()?;
    id_str.parse().ok()
}
