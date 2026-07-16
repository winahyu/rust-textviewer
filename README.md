# rust-textviewer

Linux TUI text file viewer with a File menu and bash shell support.

Built with [ratatui](https://crates.io/crates/ratatui) and [crossterm](https://crates.io/crates/crossterm).

## Requirements

- Rust toolchain (`cargo`)
- Linux with `/bin/bash` (shell features)
- A terminal that supports an alternate screen buffer

## Build & run

```bash
cargo build --release
cargo run
cargo run -- path/to/file.txt
```

## Features

- **File → Open…** — open a text file (lossy UTF-8 decoding)
- **File → Close** — clear the current buffer
- **File → Quit** — exit cleanly and restore the terminal
- **Shell → Run command…** — one-shot `bash -c` with stdout/stderr overlay
- **Shell → Interactive bash** — suspend the TUI, run a real bash session, return on `exit`

## Keybindings

| Key | Action |
|-----|--------|
| `F10` / `Alt+F` | Open File menu (`Alt+S` Shell, `Alt+H` Help) |
| `O` | Open file |
| `C` | Close file |
| `!` | Run one-shot shell command |
| `B` | Interactive bash |
| `Q` / `Ctrl+C` | Quit |
| `↑` `↓` `PgUp` `PgDn` | Scroll |
| `Home` / `End` | Top / bottom |
| `Enter` / `Esc` | Confirm / cancel prompts and overlays |

## Notes

- View-only: this app does not edit or save files.
- One-shot command output is capped (~1 MiB / 10k lines) to keep the TUI responsive.
- Interactive bash leaves raw mode and the alternate screen until the shell exits.
