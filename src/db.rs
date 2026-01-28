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
    pub tags: Option<String>,
    pub depends_on: Option<String>,
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
            description TEXT,
            tags TEXT,
            depends_on TEXT
        )",
        [],
    )?;
    
    // Migration for existing tables: add columns if they don't exist
    // sqlite doesn't support ADD COLUMN IF NOT EXISTS in old versions,
    // but these queries are safe if run individually and errors are ignored, 
    // or we can check PRAGMA table_info.
    let _ = conn.execute("ALTER TABLE tasks ADD COLUMN tags TEXT", []);
    let _ = conn.execute("ALTER TABLE tasks ADD COLUMN depends_on TEXT", []);
    
    Ok(conn)
}

pub fn add_task(
    conn: &Connection, 
    title: &str, 
    limit: Option<DateTime<Utc>>, 
    description: Option<String>,
    tags: Option<String>,
    depends_on: Option<String>
) -> Result<i64> {
    conn.execute(
        "INSERT INTO tasks (title, is_done, limit_at, description, tags, depends_on) VALUES (?, 0, ?, ?, ?, ?)",
        params![
            title, 
            limit.map(|t| t.to_rfc3339()), 
            description,
            tags,
            depends_on
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare("SELECT id, title, is_done, limit_at, description, tags, depends_on FROM tasks")?;
    let task_iter = stmt.query_map([], |row| {
        let limit_str: Option<String> = row.get(3)?;
        let limit = limit_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
        
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            is_done: row.get(2)?,
            limit,
            description: row.get(4)?,
            tags: row.get(5)?,
            depends_on: row.get(6)?,
        })
    })?;

    let mut tasks = Vec::new();
    for task in task_iter {
        tasks.push(task?);
    }
    Ok(tasks)
}

pub fn get_task(conn: &Connection, id: i64) -> Result<Option<Task>> {
    let mut stmt = conn.prepare("SELECT id, title, is_done, limit_at, description, tags, depends_on FROM tasks WHERE id = ?")?;
    let mut task_iter = stmt.query_map(params![id], |row| {
        let limit_str: Option<String> = row.get(3)?;
        let limit = limit_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
        
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            is_done: row.get(2)?,
            limit,
            description: row.get(4)?,
            tags: row.get(5)?,
            depends_on: row.get(6)?,
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
        "UPDATE tasks SET title = ?, is_done = ?, limit_at = ?, description = ?, tags = ?, depends_on = ? WHERE id = ?",
        params![
            task.title,
            task.is_done,
            task.limit.map(|t| t.to_rfc3339()),
            task.description,
            task.tags,
            task.depends_on,
            task.id
        ],
    )?;
    Ok(())
}
