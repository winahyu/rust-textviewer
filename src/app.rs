use crate::file_ops;
use crate::shell::{self, CommandOutput};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    MenuOpen,
    PromptOpen,
    PromptCommand,
    CommandResult,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuColumn {
    File,
    Shell,
    Help,
}

impl MenuColumn {
    pub fn next(self) -> Self {
        match self {
            Self::File => Self::Shell,
            Self::Shell => Self::Help,
            Self::Help => Self::File,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::File => Self::Help,
            Self::Shell => Self::File,
            Self::Help => Self::Shell,
        }
    }

    pub fn item_count(self) -> usize {
        match self {
            Self::File => 3,
            Self::Shell => 2,
            Self::Help => 1,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Shell => "Shell",
            Self::Help => "Help",
        }
    }

    pub fn items(self) -> &'static [&'static str] {
        match self {
            Self::File => &["Open…", "Close", "Quit"],
            Self::Shell => &["Run command…", "Interactive bash"],
            Self::Help => &["Keybindings"],
        }
    }
}

pub struct App {
    pub path: Option<PathBuf>,
    pub lines: Vec<String>,
    pub scroll: usize,
    pub mode: Mode,
    pub menu_column: MenuColumn,
    pub menu_item: usize,
    pub prompt: String,
    pub cursor: usize,
    pub status: String,
    pub command_output: Option<CommandOutput>,
    pub output_scroll: usize,
    pub should_quit: bool,
    pub viewer_height: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            path: None,
            lines: Vec::new(),
            scroll: 0,
            mode: Mode::Normal,
            menu_column: MenuColumn::File,
            menu_item: 0,
            prompt: String::new(),
            cursor: 0,
            status: "F10 Menu | O Open | C Close | ! Cmd | B Bash | Q Quit".to_string(),
            command_output: None,
            output_scroll: 0,
            should_quit: false,
            viewer_height: 20,
        }
    }

    pub fn open_from_path(&mut self, path: &str) -> anyhow::Result<()> {
        let (path, lines) = file_ops::open_path(path)?;
        let count = lines.len();
        self.path = Some(path.clone());
        self.lines = lines;
        self.scroll = 0;
        self.status = format!("Opened {} ({} lines)", path.display(), count);
        Ok(())
    }

    pub fn close_file(&mut self) {
        self.path = None;
        self.lines.clear();
        self.scroll = 0;
        self.status = "File closed".to_string();
    }

    pub fn set_viewer_height(&mut self, height: usize) {
        self.viewer_height = height.max(1);
        self.clamp_scroll();
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.max_scroll();
        self.scroll = (self.scroll + n).min(max);
    }

    pub fn scroll_home(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_end(&mut self) {
        self.scroll = self.max_scroll();
    }

    fn max_scroll(&self) -> usize {
        self.lines.len().saturating_sub(self.viewer_height)
    }

    fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        if self.scroll > max {
            self.scroll = max;
        }
    }

    pub fn output_scroll_up(&mut self, n: usize) {
        self.output_scroll = self.output_scroll.saturating_sub(n);
    }

    pub fn output_scroll_down(&mut self, n: usize) {
        let len = self
            .command_output
            .as_ref()
            .map(|o| o.lines.len())
            .unwrap_or(0);
        let max = len.saturating_sub(self.viewer_height);
        self.output_scroll = (self.output_scroll + n).min(max);
    }

    pub fn open_menu(&mut self) {
        self.mode = Mode::MenuOpen;
        self.menu_column = MenuColumn::File;
        self.menu_item = 0;
    }

    pub fn close_menu(&mut self) {
        if self.mode == Mode::MenuOpen {
            self.mode = Mode::Normal;
        }
    }

    pub fn start_open_prompt(&mut self) {
        self.mode = Mode::PromptOpen;
        self.prompt.clear();
        self.cursor = 0;
        self.status = "Enter file path, Enter to open, Esc to cancel".to_string();
    }

    pub fn start_command_prompt(&mut self) {
        self.mode = Mode::PromptCommand;
        self.prompt.clear();
        self.cursor = 0;
        self.status = "Enter shell command, Enter to run, Esc to cancel".to_string();
    }

    pub fn show_help(&mut self) {
        self.mode = Mode::Help;
        self.status = "Esc to close help".to_string();
    }

    pub fn insert_char(&mut self, c: char) {
        if self.mode != Mode::PromptOpen && self.mode != Mode::PromptCommand {
            return;
        }
        self.prompt.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.mode != Mode::PromptOpen && self.mode != Mode::PromptCommand {
            return;
        }
        if self.cursor == 0 {
            return;
        }
        let prev = self.prompt[..self.cursor]
            .chars()
            .next_back()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        let start = self.cursor - prev;
        self.prompt.drain(start..self.cursor);
        self.cursor = start;
    }

    pub fn cursor_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.prompt[..self.cursor]
            .chars()
            .next_back()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        self.cursor -= prev;
    }

    pub fn cursor_right(&mut self) {
        if self.cursor >= self.prompt.len() {
            return;
        }
        let next = self.prompt[self.cursor..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        self.cursor += next;
    }

    pub fn confirm_prompt(&mut self) {
        match self.mode {
            Mode::PromptOpen => {
                let path = self.prompt.clone();
                match self.open_from_path(&path) {
                    Ok(()) => self.mode = Mode::Normal,
                    Err(e) => {
                        self.status = format!("Open failed: {e}");
                        self.mode = Mode::Normal;
                    }
                }
            }
            Mode::PromptCommand => {
                let cmd = self.prompt.trim().to_string();
                if cmd.is_empty() {
                    self.status = "Empty command".to_string();
                    self.mode = Mode::Normal;
                    return;
                }
                match shell::run_command(&cmd) {
                    Ok(output) => {
                        let code = output.exit_code.unwrap_or(-1);
                        self.status = format!("Command exited with code {code}");
                        self.output_scroll = 0;
                        self.command_output = Some(output);
                        self.mode = Mode::CommandResult;
                    }
                    Err(e) => {
                        self.status = format!("Command failed: {e}");
                        self.mode = Mode::Normal;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn cancel_prompt_or_overlay(&mut self) {
        match self.mode {
            Mode::PromptOpen | Mode::PromptCommand | Mode::Help | Mode::CommandResult => {
                self.mode = Mode::Normal;
                self.command_output = None;
                self.status = "F10 Menu | O Open | C Close | ! Cmd | B Bash | Q Quit".to_string();
            }
            Mode::MenuOpen => self.close_menu(),
            Mode::Normal => {}
        }
    }

    pub fn activate_menu_item(&mut self) -> Option<ShellAction> {
        let action = match (self.menu_column, self.menu_item) {
            (MenuColumn::File, 0) => {
                self.start_open_prompt();
                None
            }
            (MenuColumn::File, 1) => {
                self.close_file();
                self.mode = Mode::Normal;
                None
            }
            (MenuColumn::File, 2) => {
                self.should_quit = true;
                None
            }
            (MenuColumn::Shell, 0) => {
                self.start_command_prompt();
                None
            }
            (MenuColumn::Shell, 1) => {
                self.mode = Mode::Normal;
                Some(ShellAction::InteractiveBash)
            }
            (MenuColumn::Help, 0) => {
                self.show_help();
                None
            }
            _ => {
                self.mode = Mode::Normal;
                None
            }
        };
        action
    }

    pub fn menu_move_up(&mut self) {
        if self.menu_item > 0 {
            self.menu_item -= 1;
        } else {
            self.menu_item = self.menu_column.item_count().saturating_sub(1);
        }
    }

    pub fn menu_move_down(&mut self) {
        let max = self.menu_column.item_count().saturating_sub(1);
        if self.menu_item < max {
            self.menu_item += 1;
        } else {
            self.menu_item = 0;
        }
    }

    pub fn menu_move_left(&mut self) {
        self.menu_column = self.menu_column.prev();
        self.menu_item = self
            .menu_item
            .min(self.menu_column.item_count().saturating_sub(1));
    }

    pub fn menu_move_right(&mut self) {
        self.menu_column = self.menu_column.next();
        self.menu_item = self
            .menu_item
            .min(self.menu_column.item_count().saturating_sub(1));
    }

    pub fn path_display(&self) -> String {
        self.path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(no file)".to_string())
    }
}

/// Actions that need the event loop to suspend the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellAction {
    InteractiveBash,
}
