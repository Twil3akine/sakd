use sakd::db;
use sakd::tui;
use std::process;

fn main() {
    let conn = db::init_db().unwrap_or_else(|e| {
        eprintln!("Failed to initialize database: {}", e);
        process::exit(1);
    });

    loop {
        match tui::run_tui(&conn).expect("TUI error") {
            tui::TuiEvent::Quit => break,
        }
    }
}
