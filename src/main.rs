mod app;
mod file_ops;
mod shell;
mod ui;

use crate::app::{App, MenuColumn, Mode, ShellAction};
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use std::io::{stdout, Write};
use std::time::Duration;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut app = App::new();

    if let Some(path) = std::env::args().nth(1) {
        if let Err(e) = app.open_from_path(&path) {
            eprintln!("warning: could not open {path}: {e:#}");
        }
    }

    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if app.should_quit {
            break;
        }

        if !event::poll(Duration::from_millis(200)).context("event poll failed")? {
            continue;
        }

        match event::read().context("event read failed")? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if let Some(action) = handle_key(app, key) {
                    match action {
                        ShellAction::InteractiveBash => {
                            run_interactive_bash(terminal, app)?;
                        }
                    }
                }
            }
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Option<ShellAction> {
    // Global quit
    if (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        || (key.code == KeyCode::Char('q')
            && app.mode == Mode::Normal
            && !key.modifiers.contains(KeyModifiers::CONTROL))
    {
        app.should_quit = true;
        return None;
    }

    // Toggle menu
    if key.code == KeyCode::F(10)
        || (key.code == KeyCode::Char('m') && key.modifiers.contains(KeyModifiers::ALT))
        || (key.modifiers.contains(KeyModifiers::ALT)
            && matches!(key.code, KeyCode::Char('f') | KeyCode::Char('F')))
    {
        if app.mode == Mode::MenuOpen {
            app.close_menu();
        } else if matches!(
            app.mode,
            Mode::Normal | Mode::CommandResult | Mode::Help
        ) {
            app.open_menu();
        }
        return None;
    }

    // Alt alone often arrives as a modifier on the next key; F10 is primary.
    // Also allow plain Alt+letter for menus when in Normal.
    if key.modifiers.contains(KeyModifiers::ALT) && app.mode == Mode::Normal {
        match key.code {
            KeyCode::Char('f') | KeyCode::Char('F') => {
                app.open_menu();
                app.menu_column = MenuColumn::File;
                app.menu_item = 0;
                return None;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                app.open_menu();
                app.menu_column = MenuColumn::Shell;
                app.menu_item = 0;
                return None;
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                app.open_menu();
                app.menu_column = MenuColumn::Help;
                app.menu_item = 0;
                return None;
            }
            _ => {}
        }
    }

    match app.mode {
        Mode::Normal => handle_normal(app, key),
        Mode::MenuOpen => handle_menu(app, key),
        Mode::PromptOpen | Mode::PromptCommand => handle_prompt(app, key),
        Mode::CommandResult => handle_command_result(app, key),
        Mode::Help => handle_help(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) -> Option<ShellAction> {
    match key.code {
        KeyCode::Char('o') | KeyCode::Char('O') => {
            app.start_open_prompt();
            None
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.close_file();
            None
        }
        KeyCode::Char('!') => {
            app.start_command_prompt();
            None
        }
        KeyCode::Char('b') | KeyCode::Char('B') => Some(ShellAction::InteractiveBash),
        KeyCode::Up => {
            app.scroll_up(1);
            None
        }
        KeyCode::Down => {
            app.scroll_down(1);
            None
        }
        KeyCode::PageUp => {
            app.scroll_up(app.viewer_height);
            None
        }
        KeyCode::PageDown => {
            app.scroll_down(app.viewer_height);
            None
        }
        KeyCode::Home => {
            app.scroll_home();
            None
        }
        KeyCode::End => {
            app.scroll_end();
            None
        }
        KeyCode::F(10) => {
            app.open_menu();
            None
        }
        _ => None,
    }
}

fn handle_menu(app: &mut App, key: KeyEvent) -> Option<ShellAction> {
    match key.code {
        KeyCode::Esc => {
            app.close_menu();
            None
        }
        KeyCode::Left => {
            app.menu_move_left();
            None
        }
        KeyCode::Right => {
            app.menu_move_right();
            None
        }
        KeyCode::Up => {
            app.menu_move_up();
            None
        }
        KeyCode::Down => {
            app.menu_move_down();
            None
        }
        KeyCode::Enter => app.activate_menu_item(),
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
            None
        }
        _ => None,
    }
}

fn handle_prompt(app: &mut App, key: KeyEvent) -> Option<ShellAction> {
    match key.code {
        KeyCode::Esc => {
            app.cancel_prompt_or_overlay();
            None
        }
        KeyCode::Enter => {
            app.confirm_prompt();
            None
        }
        KeyCode::Backspace => {
            app.backspace();
            None
        }
        KeyCode::Left => {
            app.cursor_left();
            None
        }
        KeyCode::Right => {
            app.cursor_right();
            None
        }
        KeyCode::Home => {
            app.cursor = 0;
            None
        }
        KeyCode::End => {
            app.cursor = app.prompt.len();
            None
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.insert_char(c);
            None
        }
        _ => None,
    }
}

fn handle_command_result(app: &mut App, key: KeyEvent) -> Option<ShellAction> {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.cancel_prompt_or_overlay();
            None
        }
        KeyCode::Up => {
            app.output_scroll_up(1);
            None
        }
        KeyCode::Down => {
            app.output_scroll_down(1);
            None
        }
        KeyCode::PageUp => {
            app.output_scroll_up(app.viewer_height);
            None
        }
        KeyCode::PageDown => {
            app.output_scroll_down(app.viewer_height);
            None
        }
        KeyCode::Char('!') => {
            app.start_command_prompt();
            None
        }
        KeyCode::Char('b') | KeyCode::Char('B') => Some(ShellAction::InteractiveBash),
        KeyCode::Char('o') | KeyCode::Char('O') => {
            app.start_open_prompt();
            None
        }
        _ => None,
    }
}

fn handle_help(app: &mut App, key: KeyEvent) -> Option<ShellAction> {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
            app.cancel_prompt_or_overlay();
            None
        }
        _ => None,
    }
}

fn run_interactive_bash(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    // Leave the alternate screen / raw mode so the user gets a real bash.
    ratatui::restore();
    let _ = writeln!(
        stdout(),
        "\n--- Interactive bash (type `exit` to return to textviewer) ---\n"
    );
    let _ = stdout().flush();

    match shell::run_interactive_bash() {
        Ok(code) => {
            app.status = format!("Interactive bash exited with code {code}");
        }
        Err(e) => {
            app.status = format!("Interactive bash failed: {e}");
        }
    }

    // Re-enter TUI.
    *terminal = ratatui::init();
    terminal.clear()?;
    Ok(())
}
