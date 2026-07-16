use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Read a file as lossy UTF-8 text and split into lines.
pub fn read_file_lines(path: &Path) -> Result<Vec<String>> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(text.lines().map(|l| l.to_string()).collect())
}

/// Resolve and open a path entered by the user.
pub fn open_path(input: &str) -> Result<(PathBuf, Vec<String>)> {
    let trimmed = input.trim();
    anyhow::ensure!(!trimmed.is_empty(), "path is empty");
    let path = PathBuf::from(trimmed);
    let lines = read_file_lines(&path)?;
    Ok((path, lines))
}
