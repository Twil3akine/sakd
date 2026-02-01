use rusqlite::{params, Connection, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub enum Priority {
    High,
    Medium,
    Low,
    None,
}

impl Priority {
    pub fn to_int(&self) -> i32 {
        match self {
            Priority::High => 3,
            Priority::Medium => 2,
            Priority::Low => 1,
            Priority::None => 0,
        }
    }

    pub fn from_int(i: i32) -> Self {
        match i {
            3 => Priority::High,
            2 => Priority::Medium,
            1 => Priority::Low,
            _ => Priority::None,
        }
    }

    pub fn to_symbol(&self) -> &'static str {
        match self {
            Priority::High => "!!!",
            Priority::Medium => "!! ",
            Priority::Low => "!  ",
            Priority::None => "   ",
        }
    }
}

#[derive(Clone)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub is_done: bool,
    pub limit: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub dependencies: Vec<i64>,
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

    // Migrations
    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(tasks)")?
        .query_map([], |row| row.get(1))?
        .collect::<Result<Vec<String>, _>>()?;

    if !columns.contains(&"priority".to_string()) {
        conn.execute("ALTER TABLE tasks ADD COLUMN priority INTEGER DEFAULT 0", [])?;
    }
    if !columns.contains(&"status".to_string()) {
        conn.execute("ALTER TABLE tasks ADD COLUMN status TEXT DEFAULT 'todo'", [])?;
        // Sync is_done to status
        conn.execute("UPDATE tasks SET status = 'done' WHERE is_done = 1", [])?;
    }

    // New tables for tags and dependencies
    conn.execute(
        "CREATE TABLE IF NOT EXISTS task_tags (
            task_id INTEGER,
            tag TEXT,
            PRIMARY KEY (task_id, tag),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS task_dependencies (
            task_id INTEGER,
            depends_on_id INTEGER,
            PRIMARY KEY (task_id, depends_on_id),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (depends_on_id) REFERENCES tasks(id) ON DELETE CASCADE
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
    priority: Priority,
    tags: Vec<String>,
    dependencies: Vec<i64>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO tasks (title, is_done, limit_at, description, priority) VALUES (?, 0, ?, ?, ?)",
        params![
            title, 
            limit.map(|t| t.to_rfc3339()), 
            description,
            priority.to_int(),
        ],
    )?;
    let task_id = conn.last_insert_rowid();

    for tag in tags {
        conn.execute("INSERT INTO task_tags (task_id, tag) VALUES (?, ?)", params![task_id, tag])?;
    }

    for dep_id in dependencies {
        conn.execute("INSERT INTO task_dependencies (task_id, depends_on_id) VALUES (?, ?)", params![task_id, dep_id])?;
    }

    Ok(task_id)
}

pub fn get_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare("SELECT id, title, is_done, limit_at, description, priority FROM tasks ORDER BY is_done ASC, priority DESC, limit_at IS NULL, limit_at ASC")?;
    let task_rows = stmt.query_map([], |row| {
        let task_id: i64 = row.get(0)?;
        let limit_str: Option<String> = row.get(3)?;
        let limit = limit_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
        
        Ok((task_id, row.get(1)?, row.get(2)?, limit, row.get(4)?, Priority::from_int(row.get(5)?)))
    })?;

    let mut tasks = Vec::new();
    for r in task_rows {
        let (id, title, is_done, limit, description, priority) = r?;
        
        // Tags
        let mut tag_stmt = conn.prepare("SELECT tag FROM task_tags WHERE task_id = ?")?;
        let tags = tag_stmt.query_map(params![id], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;

        // Dependencies
        let mut dep_stmt = conn.prepare("SELECT depends_on_id FROM task_dependencies WHERE task_id = ?")?;
        let dependencies = dep_stmt.query_map(params![id], |r| r.get(0))?.collect::<Result<Vec<i64>, _>>()?;

        tasks.push(Task {
            id,
            title,
            is_done,
            limit,
            description,
            priority,
            tags,
            dependencies,
        });
    }
    Ok(tasks)
}

pub fn get_task(conn: &Connection, id: i64) -> Result<Option<Task>> {
    let mut stmt = conn.prepare("SELECT id, title, is_done, limit_at, description, priority FROM tasks WHERE id = ?")?;
    let mut task_iter = stmt.query_map(params![id], |row| {
        let limit_str: Option<String> = row.get(3)?;
        let limit = limit_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
        
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, limit, row.get(4)?, Priority::from_int(row.get(5)?)))
    })?;

    if let Some(res) = task_iter.next() {
        let (id, title, is_done, limit, description, priority) = res?;
        
        // Tags
        let mut tag_stmt = conn.prepare("SELECT tag FROM task_tags WHERE task_id = ?")?;
        let tags = tag_stmt.query_map(params![id], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;

        // Dependencies
        let mut dep_stmt = conn.prepare("SELECT depends_on_id FROM task_dependencies WHERE task_id = ?")?;
        let dependencies = dep_stmt.query_map(params![id], |r| r.get(0))?.collect::<Result<Vec<i64>, _>>()?;

        Ok(Some(Task {
            id,
            title,
            is_done,
            limit,
            description,
            priority,
            tags,
            dependencies,
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
        "UPDATE tasks SET title = ?, is_done = ?, limit_at = ?, description = ?, priority = ? WHERE id = ?",
        params![
            task.title,
            task.is_done,
            task.limit.map(|t| t.to_rfc3339()),
            task.description,
            task.priority.to_int(),
            task.id
        ],
    )?;

    // Update Tags
    conn.execute("DELETE FROM task_tags WHERE task_id = ?", params![task.id])?;
    for tag in &task.tags {
        conn.execute("INSERT INTO task_tags (task_id, tag) VALUES (?, ?)", params![task.id, tag])?;
    }

    // Update Dependencies
    conn.execute("DELETE FROM task_dependencies WHERE task_id = ?", params![task.id])?;
    for dep_id in &task.dependencies {
        conn.execute("INSERT INTO task_dependencies (task_id, depends_on_id) VALUES (?, ?)", params![task.id, dep_id])?;
    }

    Ok(())
}
