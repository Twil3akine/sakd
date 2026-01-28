use rusqlite::{params, Connection, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub is_done: bool,
    pub limit: Option<DateTime<Utc>>,
    pub description: Option<String>,
}

fn get_db_path() -> PathBuf {
    let mut path = dirs::data_local_dir().expect("Could not find local data directory");
    path.push("sakd");
    if !path.exists() {
        fs::create_dir_all(&path).expect("Could not create data directory");
    }
    path.push("sakd.db");
    path
}

pub fn init_db() -> Result<Connection> {
    let db_path = get_db_path();
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            is_done BOOLEAN NOT NULL DEFAULT 0,
            limit_at TEXT,
            description TEXT
        )",
        [],
    )?;
    Ok(conn)
}

pub fn add_task(conn: &Connection, title: &str, limit: Option<DateTime<Utc>>, description: Option<String>) -> Result<i64> {
    conn.execute(
        "INSERT INTO tasks (title, is_done, limit_at, description) VALUES (?, 0, ?, ?)",
        params![title, limit.map(|t| t.to_rfc3339()), description],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare("SELECT id, title, is_done, limit_at, description FROM tasks")?;
    let task_iter = stmt.query_map([], |row| {
        let limit_str: Option<String> = row.get(3)?;
        let limit = limit_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
        
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            is_done: row.get(2)?,
            limit,
            description: row.get(4)?,
        })
    })?;

    let mut tasks = Vec::new();
    for task in task_iter {
        tasks.push(task?);
    }
    Ok(tasks)
}

pub fn get_task(conn: &Connection, id: i64) -> Result<Option<Task>> {
    let mut stmt = conn.prepare("SELECT id, title, is_done, limit_at, description FROM tasks WHERE id = ?")?;
    let mut task_iter = stmt.query_map(params![id], |row| {
        let limit_str: Option<String> = row.get(3)?;
        let limit = limit_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
        
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            is_done: row.get(2)?,
            limit,
            description: row.get(4)?,
        })
    })?;

    if let Some(task) = task_iter.next() {
        Ok(Some(task?))
    } else {
        Ok(None)
    }
}

pub fn delete_task(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM tasks WHERE id = ?", params![id])?;
    Ok(())
}

pub fn update_task(conn: &Connection, task: &Task) -> Result<()> {
    conn.execute(
        "UPDATE tasks SET title = ?, is_done = ?, limit_at = ?, description = ? WHERE id = ?",
        params![
            task.title,
            task.is_done,
            task.limit.map(|t| t.to_rfc3339()),
            task.description,
            task.id
        ],
    )?;
    Ok(())
}
