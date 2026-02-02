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
    pub tags: Vec<String>,
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
    
    // Core tasks table
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

    // New table for tags
    conn.execute(
        "CREATE TABLE IF NOT EXISTS task_tags (
            task_id INTEGER,
            tag TEXT,
            PRIMARY KEY (task_id, tag),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        )",
        [],
    )?;
    
    Ok(conn)
}

pub fn add_task(
    conn: &Connection, 
    title: &str, 
    limit: Option<DateTime<Utc>>, 
    description: Option<String>,
    tags: Vec<String>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO tasks (title, is_done, limit_at, description) VALUES (?, 0, ?, ?)",
        params![
            title, 
            limit.map(|t| t.to_rfc3339()), 
            description,
        ],
    )?;
    let task_id = conn.last_insert_rowid();

    for tag in tags {
        conn.execute("INSERT INTO task_tags (task_id, tag) VALUES (?, ?)", params![task_id, tag])?;
    }

    Ok(task_id)
}

pub fn get_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare("SELECT id, title, is_done, limit_at, description FROM tasks ORDER BY is_done ASC, limit_at IS NULL, limit_at ASC")?;
    let task_rows = stmt.query_map([], |row| {
        let task_id: i64 = row.get(0)?;
        let limit_str: Option<String> = row.get(3)?;
        let limit = limit_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
        
        Ok((task_id, row.get(1)?, row.get(2)?, limit, row.get(4)?))
    })?;

    let mut tasks = Vec::new();
    for r in task_rows {
        let (id, title, is_done, limit, description) = r?;
        
        // Tags
        let mut tag_stmt = conn.prepare("SELECT tag FROM task_tags WHERE task_id = ?")?;
        let tags = tag_stmt.query_map(params![id], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;

        tasks.push(Task {
            id,
            title,
            is_done,
            limit,
            description,
            tags,
        });
    }
    Ok(tasks)
}

pub fn get_task(conn: &Connection, id: i64) -> Result<Option<Task>> {
    let mut stmt = conn.prepare("SELECT id, title, is_done, limit_at, description FROM tasks WHERE id = ?")?;
    let mut task_iter = stmt.query_map(params![id], |row| {
        let limit_str: Option<String> = row.get(3)?;
        let limit = limit_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
        
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, limit, row.get(4)?))
    })?;

    if let Some(res) = task_iter.next() {
        let (id, title, is_done, limit, description) = res?;
        
        // Tags
        let mut tag_stmt = conn.prepare("SELECT tag FROM task_tags WHERE task_id = ?")?;
        let tags = tag_stmt.query_map(params![id], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;

        Ok(Some(Task {
            id,
            title,
            is_done,
            limit,
            description,
            tags,
        }))
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

    // Update Tags
    conn.execute("DELETE FROM task_tags WHERE task_id = ?", params![task.id])?;
    for tag in &task.tags {
        conn.execute("INSERT INTO task_tags (task_id, tag) VALUES (?, ?)", params![task.id, tag])?;
    }

    Ok(())
}
