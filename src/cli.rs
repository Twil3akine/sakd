use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sakd")]
#[command(about = "Fastest, most useful CLI task manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new task
    #[command(alias = "a")]
    Add {
        /// Task title
        title: Option<String>,
        /// Task limit date (e.g. 2026-01-16T20:00:00)
        #[arg(short, long)]
        limit: Option<String>,
        /// Task description
        #[arg(short, long)]
        description: Option<String>,
        /// Task priority (high, medium, low, none)
        #[arg(long)]
        priority: Option<String>,
        /// Task tags (comma separated)
        #[arg(long)]
        tags: Option<String>,
        /// Task dependencies (comma separated IDs)
        #[arg(long)]
        dep: Option<String>,
    },
    /// Mark a task as done
    #[command(alias = "d")]
    Done {
        /// Task ID
        id: Option<i64>,
    },
    /// Remove a task
    #[command(alias = "r")]
    Remove {
        /// Task ID
        id: Option<i64>,
    },
    /// List all tasks
    #[command(alias = "l")]
    List {
        /// Show all tasks including completed ones
        #[arg(short, long)]
        all: bool,
        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
        /// Filter by priority
        #[arg(long)]
        priority: Option<String>,
        /// Order by (name, limit, etc.)
        #[arg(short, long)]
        order: Option<String>,
    },
    /// Show task details
    #[command(alias = "s")]
    Show {
        /// Task ID
        id: Option<i64>,
    },
    /// Edit task details
    #[command(alias = "e")]
    Edit {
        /// Task ID
        id: Option<i64>,
    },
}
