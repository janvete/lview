use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{App, Screen};

/// Returns true if the app should keep running.
pub fn handle_event(app: &mut App, event: Event) -> bool {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(app, key),
        _ => true,
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    // Global quit
    if key.code == KeyCode::Char('q')
        && !matches!(app.screen, Screen::Search | Screen::PickerSearch)
    {
        return false;
    }

    match app.screen {
        Screen::Picker => handle_picker(app, key),
        Screen::Viewer => handle_viewer(app, key),
        Screen::Search => handle_search(app, key),
        Screen::PickerSearch => handle_picker_search(app, key),
    }
}

fn handle_picker(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => return false,
        KeyCode::Char('j') | KeyCode::Down => app.next_log(),
        KeyCode::Char('k') | KeyCode::Up => app.previous_log(),
        KeyCode::Enter => {
            if let Err(e) = app.open_selected_log() {
                app.message = Some(format!("Failed to open log: {}", e));
            }
        }
        KeyCode::Char('/') => app.enter_search(),
        KeyCode::Char('r') => {
            // Async discovery is triggered from main loop
            app.loading = true;
        }
        _ => {}
    }
    true
}

fn handle_viewer(app: &mut App, key: KeyEvent) -> bool {
    let Some(viewer) = app.viewer.as_mut() else {
        app.screen = Screen::Picker;
        return true;
    };

    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => app.close_viewer(),
        KeyCode::Char('j') | KeyCode::Down => viewer.scroll_down(1),
        KeyCode::Char('k') | KeyCode::Up => viewer.scroll_up(1),
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => viewer.scroll_down(10),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => viewer.scroll_up(10),
        KeyCode::Char('g') => viewer.scroll_to_top(),
        KeyCode::Char('G') => viewer.scroll_to_bottom(),
        KeyCode::Char('l') | KeyCode::Char(' ') => viewer.toggle_live(),
        KeyCode::Char('/') => app.enter_search(),
        KeyCode::Char('n') => {
            viewer.next_search_result();
        }
        KeyCode::Char('N') => {
            viewer.prev_search_result();
        }
        KeyCode::Char('s') => {
            if let Some(path) = app.save_viewer_buffer() {
                app.message = Some(format!("Saved to {}", path));
            }
        }
        _ => {}
    }
    true
}

fn handle_search(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => app.exit_search(),
        KeyCode::Enter => app.confirm_search(),
        KeyCode::Backspace => {
            if let Some(query) = app.current_filter_query_mut() {
                query.pop();
                app.apply_current_filter();
            }
        }
        KeyCode::Char(c) => {
            if let Some(query) = app.current_filter_query_mut() {
                query.push(c);
                app.apply_current_filter();
            }
        }
        _ => {}
    }
    true
}

fn handle_picker_search(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => app.exit_search(),
        KeyCode::Enter => app.confirm_search(),
        KeyCode::Backspace => {
            if let Some(query) = app.current_filter_query_mut() {
                query.pop();
                app.apply_current_filter();
            }
        }
        KeyCode::Char(c) => {
            if let Some(query) = app.current_filter_query_mut() {
                query.push(c);
                app.apply_current_filter();
            }
        }
        _ => {}
    }
    true
}
