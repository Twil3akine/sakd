use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use rusqlite::Connection;
use std::io;

use crate::db::{self, Task};

pub enum InputMode {
    Normal,
}

pub struct App<'a> {
    pub tasks: Vec<Task>,
    pub filtered_tasks: Vec<Task>,
    pub state: ListState,
    pub input_mode: InputMode,
    pub show_done: bool,
    pub conn: &'a Connection,
}

impl<'a> App<'a> {
    pub fn new(conn: &'a Connection) -> Result<Self> {
        let tasks = db::get_tasks(conn)?;
        let mut app = App {
            tasks,
            filtered_tasks: Vec::new(),
            state: ListState::default(),
            input_mode: InputMode::Normal,
            show_done: false,
            conn,
        };
        app.update_filtered_tasks();
        Ok(app)
    }

    fn update_filtered_tasks(&mut self) {
        let current_index = self.state.selected();
        let selected_id = current_index
            .and_then(|i| self.filtered_tasks.get(i))
            .map(|t| t.id);

        self.filtered_tasks = if self.show_done {
            self.tasks.clone()
        } else {
            self.tasks.iter().filter(|t| !t.is_done).cloned().collect()
        };

        if let Some(id) = selected_id {
            if let Some(new_index) = self.filtered_tasks.iter().position(|t| t.id == id) {
                self.state.select(Some(new_index));
            } else if let Some(i) = current_index {
                if self.filtered_tasks.is_empty() {
                    self.state.select(None);
                } else {
                    let new_i = i.min(self.filtered_tasks.len() - 1);
                    self.state.select(Some(new_i));
                }
            }
        }

        if self.state.selected().is_none() && !self.filtered_tasks.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn refresh_tasks(&mut self) -> Result<()> {
        self.tasks = db::get_tasks(self.conn)?;
        self.update_filtered_tasks();
        Ok(())
    }

    pub fn toggle_done_visibility(&mut self) {
        self.show_done = !self.show_done;
        self.update_filtered_tasks();
    }

    pub fn next(&mut self) {
        if self.filtered_tasks.is_empty() { return; }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.filtered_tasks.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.filtered_tasks.is_empty() { return; }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_tasks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn toggle_status(&mut self) -> Result<()> {
        if let Some(i) = self.state.selected() {
            let task_id = self.filtered_tasks[i].id;
            if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
                task.is_done = !task.is_done;
                db::update_task(self.conn, task)?;
            }
            self.update_filtered_tasks();
        }
        Ok(())
    }

    pub fn delete_task(&mut self) -> Result<()> {
        if let Some(i) = self.state.selected() {
            let task_id = self.filtered_tasks[i].id;
            db::delete_task(self.conn, task_id)?;
            self.refresh_tasks()?;
        }
        Ok(())
    }
}

pub enum TuiEvent {
    Add,
    Edit(i64),
    Quit,
}

pub fn run_tui(conn: &Connection) -> Result<TuiEvent> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(conn)?;
    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<TuiEvent> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match app.input_mode {
                    InputMode::Normal => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(TuiEvent::Quit),
                            KeyCode::Char('j') | KeyCode::Down => app.next(),
                            KeyCode::Char('k') | KeyCode::Up => app.previous(),
                            KeyCode::Char(' ') | KeyCode::Enter => {
                                app.toggle_status()?;
                            }
                            KeyCode::Char('d') => {
                                app.delete_task()?;
                            }
                            KeyCode::Char('h') => {
                                app.toggle_done_visibility();
                            }
                            KeyCode::Char('a') => {
                                return Ok(TuiEvent::Add);
                            }
                            KeyCode::Char('e') => {
                                if let Some(i) = app.state.selected() {
                                    let task_id = app.filtered_tasks[i].id;
                                    return Ok(TuiEvent::Edit(task_id));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Min(0),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ]
            .as_ref(),
        )
        .split(chunks[0]);

    let tasks: Vec<ListItem> = app
        .filtered_tasks
        .iter()
        .map(|i| {
            let status = if i.is_done { "[v]" } else { "[ ]" };
            let content = format!("{} {}", status, i.title);
            let style = if i.is_done {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            
            ListItem::new(content).style(style)
        })
        .collect();

    let tasks_list = List::new(tasks)
        .block(Block::default().borders(Borders::ALL).title(format!(" Tasks ({}) ", if app.show_done { "All" } else { "Active" })))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(tasks_list, main_chunks[0], &mut app.state);

    // Detail Panel
    let selected_index = app.state.selected();
    let detail_block = Block::default().borders(Borders::ALL).title(" Details ");
    
    if let Some(i) = selected_index {
        if let Some(task) = app.filtered_tasks.get(i) {
            let mut details = Vec::new();
            details.push(format!("Title: {}", task.title));
            details.push(format!("Status: {}", if task.is_done { "Completed" } else { "In Progress" }));
            
            if let Some(limit) = task.limit {
                details.push(format!("Limit: {}", limit.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M")));
            } else {
                details.push("Limit: None".to_string());
            }

            if let Some(tags) = &task.tags {
                details.push(format!("Tags: {}", tags));
            }

            if let Some(desc) = &task.description {
                details.push("".to_string());
                details.push("Description:".to_string());
                details.push(desc.clone());
            }

            let detail_text = Paragraph::new(details.join("\n")).block(detail_block);
            f.render_widget(detail_text, main_chunks[1]);
        } else {
            f.render_widget(Paragraph::new("No task selected").block(detail_block), main_chunks[1]);
        }
    } else {
        f.render_widget(Paragraph::new("No task selected").block(detail_block), main_chunks[1]);
    }

    let help_text = "j/k: Nav | Space: Toggle | h: Show/Hide Done | a: Add | e: Edit | d: Del | q: Quit";

    let help_msg = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title(" Help "));
    f.render_widget(help_msg, chunks[1]);
}
