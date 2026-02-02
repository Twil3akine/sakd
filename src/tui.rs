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
use crate::utils;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PopupStep {
    Title,
    Tags,
    Date,
    Time,
    Description,
}

pub enum InputMode {
    Normal,
    Adding(PopupStep),
    Editing(i64, PopupStep),
    Deleting(i64),
    FilteringTag,
    Helping,
}

pub struct PopupData {
    pub title: String,
    pub tags: String,
    pub date: String,
    pub time: String,
    pub description: String,
}

impl Default for PopupData {
    fn default() -> Self {
        Self {
            title: String::new(),
            tags: String::new(),
            date: String::new(),
            time: String::new(),
            description: String::new(),
        }
    }
}

pub struct App<'a> {
    pub tasks: Vec<Task>,
    pub filtered_tasks: Vec<Task>,
    pub state: ListState,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub popup_data: PopupData,
    pub show_done: bool,
    pub tag_filter: Option<String>,
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
            input_buffer: String::new(),
            popup_data: PopupData::default(),
            show_done: false,
            tag_filter: None,
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

        let mut filtered: Vec<Task> = self.tasks.iter()
            .filter(|t| self.show_done || !t.is_done)
            .filter(|t| self.tag_filter.as_ref().map_or(true, |f| t.tags.iter().any(|tag| tag.to_lowercase().contains(&f.to_lowercase()))))
            .cloned()
            .collect();

        // Default sort: is_done ASC, limit ASC
        filtered.sort_by(|a, b| {
            a.is_done.cmp(&b.is_done)
                .then_with(|| a.limit.is_none().cmp(&b.limit.is_none()))
                .then_with(|| a.limit.cmp(&b.limit))
        });
        
        self.filtered_tasks = filtered;

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

    pub fn start_add_popup(&mut self) {
        self.popup_data = PopupData {
            title: String::new(),
            tags: String::new(),
            date: String::new(),
            time: String::new(),
            description: String::new(),
        };
        self.input_buffer.clear();
        self.input_mode = InputMode::Adding(PopupStep::Title);
    }

    pub fn start_edit_popup(&mut self) {
        if let Some(i) = self.state.selected() {
            let task = &self.filtered_tasks[i];
            self.popup_data = PopupData {
                title: task.title.clone(),
                tags: task.tags.join(", "),
                date: task.limit.map(|l| l.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
                time: task.limit.map(|l| l.with_timezone(&chrono::Local).format("%H:%M").to_string())
                    .unwrap_or_default(),
                description: task.description.clone().unwrap_or_default(),
            };
            self.input_buffer = self.popup_data.title.clone();
            self.input_mode = InputMode::Editing(task.id, PopupStep::Title);
        }
    }

    pub fn next_popup_step(&mut self) -> Result<()> {
        let (next_step, is_done) = match &self.input_mode {
            InputMode::Adding(step) | InputMode::Editing(_, step) => match step {
                PopupStep::Title => {
                    self.popup_data.title = self.input_buffer.clone();
                    (PopupStep::Tags, false)
                }
                PopupStep::Tags => {
                    self.popup_data.tags = self.input_buffer.clone();
                    (PopupStep::Date, false)
                }
                PopupStep::Date => {
                    self.popup_data.date = self.input_buffer.clone();
                    (PopupStep::Time, false)
                }
                PopupStep::Time => {
                    self.popup_data.time = self.input_buffer.clone();
                    (PopupStep::Description, false)
                }
                PopupStep::Description => {
                    self.popup_data.description = self.input_buffer.clone();
                    (PopupStep::Description, true)
                }
            },
            _ => return Ok(()),
        };

        if is_done {
            self.save_popup()?;
            self.input_mode = InputMode::Normal;
            self.input_buffer.clear();
        } else {
            match &mut self.input_mode {
                InputMode::Adding(s) => *s = next_step,
                InputMode::Editing(_, s) => *s = next_step,
                _ => {}
            }

            self.input_buffer = match next_step {
                PopupStep::Title => self.popup_data.title.clone(),
                PopupStep::Tags => self.popup_data.tags.clone(),
                PopupStep::Date => self.popup_data.date.clone(),
                PopupStep::Time => self.popup_data.time.clone(),
                PopupStep::Description => self.popup_data.description.clone(),
            };
        }
        Ok(())
    }

    pub fn save_popup(&mut self) -> Result<()> {
        let date_str = self.popup_data.date.trim();
        let time_str = self.popup_data.time.trim();
        
        let limit = if date_str.is_empty() {
            None
        } else {
            let time = if time_str.is_empty() { "23:59" } else { time_str };
            utils::parse_full_date_time(date_str, time)
        };

        let description = if self.popup_data.description.is_empty() {
            None
        } else {
            Some(self.popup_data.description.clone())
        };

        let tags = utils::parse_tags(&self.popup_data.tags);

        match self.input_mode {
            InputMode::Adding(_) => {
                db::add_task(self.conn, &self.popup_data.title, limit, description, tags)?;
            }
            InputMode::Editing(id, _) => {
                if let Some(mut task) = db::get_task(self.conn, id)? {
                    task.title = self.popup_data.title.clone();
                    task.limit = limit;
                    task.description = description;
                    task.tags = tags;
                    db::update_task(self.conn, &task)?;
                }
            }
            _ => {}
        }
        self.refresh_tasks()?;
        Ok(())
    }

}

pub enum TuiEvent {
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
                            KeyCode::Char('r') => {
                                if let Some(i) = app.state.selected() {
                                    let task_id = app.filtered_tasks[i].id;
                                    app.input_mode = InputMode::Deleting(task_id);
                                }
                            }
                            KeyCode::Char('h') => {
                                app.toggle_done_visibility();
                            }
                            KeyCode::Char('a') => {
                                app.start_add_popup();
                            }
                             KeyCode::Char('e') => {
                                app.start_edit_popup();
                             }
                            KeyCode::Char('f') => {
                                app.input_mode = InputMode::FilteringTag;
                                app.input_buffer = app.tag_filter.clone().unwrap_or_default();
                             }
                             KeyCode::Char('?') => {
                                app.input_mode = InputMode::Helping;
                             }
                             _ => {}
                         }
                    }
                    InputMode::Helping => {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter => {
                                app.input_mode = InputMode::Normal;
                            }
                            _ => {}
                        }
                    }
                    InputMode::Adding(_) | InputMode::Editing(_, _) => {
                        match key.code {
                            KeyCode::Enter => {
                                app.next_popup_step()?;
                            }
                            KeyCode::Esc => {
                                app.input_mode = InputMode::Normal;
                                app.input_buffer.clear();
                            }
                            KeyCode::Char(c) => {
                                app.input_buffer.push(c);
                            }
                            KeyCode::Backspace => {
                                app.input_buffer.pop();
                            }
                            _ => {}
                        }
                    }
                    InputMode::Deleting(id) => {
                        match key.code {
                            KeyCode::Enter => {
                                db::delete_task(app.conn, id)?;
                                app.refresh_tasks()?;
                                app.input_mode = InputMode::Normal;
                            }
                            KeyCode::Esc | KeyCode::Char('n') => {
                                app.input_mode = InputMode::Normal;
                            }
                            _ => {}
                        }
                    }
                    InputMode::FilteringTag => {
                        match key.code {
                            KeyCode::Enter => {
                                app.tag_filter = if app.input_buffer.is_empty() { None } else { Some(app.input_buffer.clone()) };
                                app.update_filtered_tasks();
                                app.input_mode = InputMode::Normal;
                                app.input_buffer.clear();
                            }
                            KeyCode::Esc => {
                                app.input_mode = InputMode::Normal;
                                app.input_buffer.clear();
                            }
                            KeyCode::Backspace => {
                                app.input_buffer.pop();
                            }
                            KeyCode::Char(c) => {
                                app.input_buffer.push(c);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn ui(f: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ]
            .as_ref(),
        )
        .split(f.size());

    let tasks: Vec<ListItem> = app
        .filtered_tasks
        .iter()
        .map(|i| {
            let status = if i.is_done { "[v]" } else { "[ ]" };
            
            let content = format!("{:>2}: {} {}", i.id, status, i.title);
            let style = if i.is_done {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            
            ListItem::new(content).style(style)
        })
        .collect();

    let tasks_list = List::new(tasks)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " Tasks ({}) [Filter: Tag:{}] ", 
            if app.show_done { "All" } else { "Active" },
            app.tag_filter.as_ref().unwrap_or(&"None".to_string()),
        )))
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
            details.push(format!("Tags:  {}", task.tags.join(", ")));
            
            if let Some(limit) = task.limit {
                details.push(format!("Limit: {}", limit.with_timezone(&chrono::Local).format("%Y/%m/%d %H:%M")));
            } else {
                details.push("Limit: None".to_string());
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

    // Popup for Add/Edit
    match &app.input_mode {
        InputMode::Adding(step) | InputMode::Editing(_, step) => {
            let area = centered_rect(60, 20, f.size());
            let title = match &app.input_mode {
                InputMode::Adding(_) => " Adding Task... ",
                InputMode::Editing(_, _) => " Editing Task... ",
                _ => "",
            };
            
            let (prompt, help) = match step {
                PopupStep::Title => ("Title:", "(Required)"),
                PopupStep::Tags => ("Tags:", "Comma separated"),
                PopupStep::Date => ("Date:", "YYYY/MM/DD or MM/DD ( shortcuts: t, tm, 2d, 1w... )"),
                PopupStep::Time => ("Time:", "HH:MM ( shortcuts: last, noon, 1h... )"),
                PopupStep::Description => ("Desc:", "(Optional)"),
            };

            let popup_block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            
            let mut lines = Vec::new();
            lines.push(ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(format!("{} ", prompt), Style::default().add_modifier(Modifier::BOLD)),
                ratatui::text::Span::raw(app.input_buffer.clone()),
            ]));
            lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(format!("  {}", help), Style::default().fg(Color::DarkGray))));

            let popup_text = Paragraph::new(lines)
                .block(popup_block)
                .alignment(ratatui::layout::Alignment::Left);
            
            f.render_widget(ratatui::widgets::Clear, area);
            f.render_widget(popup_text, area);
        }
        InputMode::Deleting(_) => {
            let area = centered_rect(50, 15, f.size());
            let popup_block = Block::default()
                .title(" Confirm Delete ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red));
            
            let popup_text = Paragraph::new("\n  Are you sure you want to delete this task?\n\n  [Enter] Delete  [Esc/n] Cancel")
                .block(popup_block)
                .alignment(ratatui::layout::Alignment::Center);
            
            f.render_widget(ratatui::widgets::Clear, area);
            f.render_widget(popup_text, area);
        }
        InputMode::FilteringTag => {
            let area = centered_rect(60, 20, f.size());
            let block = Block::default().title(" Filter by Tag ").borders(Borders::ALL);
            let text = Paragraph::new(app.input_buffer.as_str())
                .block(block)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(ratatui::widgets::Clear, area);
            f.render_widget(text, area);
        }
        InputMode::Helping => {
            let area = centered_rect(80, 80, f.size());
            let block = Block::default().title(" Detailed Help ").borders(Borders::ALL).border_style(Style::default().fg(Color::Green));
            let help_content = vec![
                "=== Basic Operations ===",
                "j/k or \u{2193}/\u{2191}: Select Task",
                "Space/Enter: Toggle Done/Todo",
                "a: Add Task",
                "e: Edit Task",
                "r: Remove Task",
                "h: Toggle Completed Visibility",
                "f: Filter by Tag",
                "q/Esc: Quit",
                "",
                "=== Input Format (Add/Edit) ===",
                "Date: YYYY/MM/DD, MM/DD",
                "      Shortcuts: t (today), tm (tomorrow), 2d, 1w...",
                "      Day: mon, tue, wed, thu, fri, sat, sun",
                "Time: HH:MM",
                "      Shortcuts: last (23:59), noon (12:00), 1h...",
                "",
                "[Esc/?: Close]"
            ];
            let text = Paragraph::new(help_content.join("\n"))
                .block(block)
                .alignment(ratatui::layout::Alignment::Left);
            f.render_widget(ratatui::widgets::Clear, area);
            f.render_widget(text, area);
        }
        _ => {}
    }
}
