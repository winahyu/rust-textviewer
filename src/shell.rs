use anyhow::{Context, Result};
use std::process::{Command, Stdio};

const MAX_OUTPUT_BYTES: usize = 1_048_576; // 1 MiB
const MAX_OUTPUT_LINES: usize = 10_000;

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub command: String,
    pub exit_code: Option<i32>,
    pub lines: Vec<String>,
}

/// Run a one-shot command via `/bin/bash -c`, capturing stdout/stderr.
pub fn run_command(command: &str) -> Result<CommandOutput> {
    let output = Command::new("/bin/bash")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to run bash -c {:?}", command))?;

    let stdout = truncate_bytes(String::from_utf8_lossy(&output.stdout).into_owned());
    let stderr = truncate_bytes(String::from_utf8_lossy(&output.stderr).into_owned());
    let exit_code = output.status.code();

    let mut lines = Vec::new();
    if !stdout.is_empty() {
        lines.push("--- stdout ---".to_string());
        lines.extend(stdout.lines().map(|l| l.to_string()));
    }
    if !stderr.is_empty() {
        lines.push("--- stderr ---".to_string());
        lines.extend(stderr.lines().map(|l| l.to_string()));
    }
    if lines.is_empty() {
        lines.push("(no output)".to_string());
    }
    if lines.len() > MAX_OUTPUT_LINES {
        let skipped = lines.len() - MAX_OUTPUT_LINES;
        lines = lines.split_off(skipped);
        lines.insert(0, format!("... truncated {} earlier lines ...", skipped));
    }

    Ok(CommandOutput {
        command: command.to_string(),
        exit_code,
        lines,
    })
}

/// Spawn an interactive `/bin/bash` with inherited stdio.
/// Caller must leave/restore the TUI around this call.
pub fn run_interactive_bash() -> Result<i32> {
    let status = Command::new("/bin/bash")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("failed to spawn interactive /bin/bash")?;
    Ok(status.code().unwrap_or(-1))
}

fn truncate_bytes(mut s: String) -> String {
    if s.len() <= MAX_OUTPUT_BYTES {
        return s;
    }
    // Truncate to nearest char boundary at the limit.
    let mut end = MAX_OUTPUT_BYTES;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s.truncate(end);
    s.push_str("\n... [output truncated] ...");
    s
}
