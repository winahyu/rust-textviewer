use crate::app::{App, MenuColumn, Mode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // menu bar
            Constraint::Length(1), // path
            Constraint::Min(3),    // content
            Constraint::Length(1), // status
        ])
        .split(area);

    draw_menu_bar(frame, chunks[0], app);
    draw_path_line(frame, chunks[1], app);

    let content_height = chunks[2].height.saturating_sub(2) as usize;
    app.set_viewer_height(content_height.max(1));

    match app.mode {
        Mode::CommandResult => draw_command_result(frame, chunks[2], app),
        Mode::Help => draw_help(frame, chunks[2]),
        _ => draw_viewer(frame, chunks[2], app),
    }

    draw_status(frame, chunks[3], app);

    if app.mode == Mode::MenuOpen {
        draw_menu_dropdown(frame, chunks[0], app);
    }

    if matches!(app.mode, Mode::PromptOpen | Mode::PromptCommand) {
        draw_prompt(frame, area, app);
    }
}

fn draw_menu_bar(frame: &mut Frame, area: Rect, app: &App) {
    let columns = [MenuColumn::File, MenuColumn::Shell, MenuColumn::Help];
    let mut spans = Vec::new();
    for col in columns {
        let selected = app.mode == Mode::MenuOpen && app.menu_column == col;
        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        };
        spans.push(Span::styled(format!(" {} ", col.label()), style));
    }
    spans.push(Span::styled(
        " ".repeat(area.width as usize),
        Style::default().bg(Color::DarkGray),
    ));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_path_line(frame: &mut Frame, area: Rect, app: &App) {
    let text = format!(" {}", app.path_display());
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::Yellow)),
        area,
    );
}

fn draw_viewer(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Text Viewer ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible: Vec<ListItem> = app
        .lines
        .iter()
        .skip(app.scroll)
        .take(app.viewer_height)
        .enumerate()
        .map(|(i, line)| {
            let num = app.scroll + i + 1;
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{num:>6} │ "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(line.as_str()),
            ]))
        })
        .collect();

    if visible.is_empty() {
        let hint = Paragraph::new("No file open. Press O or use File → Open…")
            .style(Style::default().fg(Color::DarkGray))
            .wrap(Wrap { trim: true });
        frame.render_widget(hint, inner);
    } else {
        frame.render_widget(List::new(visible), inner);
    }
}

fn draw_command_result(frame: &mut Frame, area: Rect, app: &App) {
    let title = app
        .command_output
        .as_ref()
        .map(|o| format!(" Command: {} ", o.command))
        .unwrap_or_else(|| " Command output ".to_string());
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = app
        .command_output
        .as_ref()
        .map(|o| o.lines.as_slice())
        .unwrap_or(&[]);

    let visible: Vec<ListItem> = lines
        .iter()
        .skip(app.output_scroll)
        .take(app.viewer_height)
        .map(|line| ListItem::new(line.as_str()))
        .collect();

    frame.render_widget(List::new(visible), inner);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let help = [
        "Keybindings",
        "",
        "  F10 / Alt     Open or close menu bar",
        "  O             Open file",
        "  C             Close file",
        "  !             Run one-shot shell command",
        "  B             Interactive bash (suspends TUI)",
        "  Q / Ctrl+C    Quit",
        "  ↑ ↓ PgUp/Dn   Scroll",
        "  Home / End    Top / bottom",
        "  Enter / Esc   Confirm / cancel prompts",
        "",
        "Menus: File (Open, Close, Quit) | Shell (Run command, Interactive bash) | Help",
        "",
        "Press Esc to return.",
    ];
    let block = Block::default().borders(Borders::ALL).title(" Help ");
    let paragraph = Paragraph::new(help.join("\n"))
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    frame.render_widget(
        Paragraph::new(format!(" {}", app.status))
            .style(Style::default().fg(Color::White).bg(Color::Blue)),
        area,
    );
}

fn draw_menu_dropdown(frame: &mut Frame, menu_bar: Rect, app: &App) {
    let x_offset = match app.menu_column {
        MenuColumn::File => 0,
        MenuColumn::Shell => 7,
        MenuColumn::Help => 15,
    };
    let items = app.menu_column.items();
    let width = items
        .iter()
        .map(|s| s.len())
        .max()
        .unwrap_or(10)
        .max(12) as u16
        + 4;
    let height = items.len() as u16 + 2;
    let full = frame.area();
    let area = Rect {
        x: menu_bar.x + x_offset,
        y: menu_bar.y + 1,
        width: width.min(full.width.saturating_sub(menu_bar.x + x_offset)),
        height: height.min(full.height.saturating_sub(menu_bar.y + 1)),
    };

    frame.render_widget(Clear, area);

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let style = if i == app.menu_item {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(format!(" {label} "), style))
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black)),
    );
    frame.render_widget(list, area);
}

fn draw_prompt(frame: &mut Frame, full: Rect, app: &App) {
    let title = match app.mode {
        Mode::PromptOpen => " Open file ",
        Mode::PromptCommand => " Run command ",
        _ => " Input ",
    };

    let width = full.width.saturating_mul(3) / 4;
    let height = 3;
    let area = Rect {
        x: full.x + (full.width.saturating_sub(width)) / 2,
        y: full.y + full.height.saturating_div(3),
        width,
        height,
    };

    frame.render_widget(Clear, area);

    let mut display = app.prompt.clone();
    // Show a block cursor marker at the logical cursor position.
    if app.cursor <= display.len() {
        display.insert(app.cursor, '▌');
    }

    let paragraph = Paragraph::new(display).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().bg(Color::Black)),
    );
    frame.render_widget(paragraph, area);
}
