use std::process::Command;

/// Validate that `file_path` stays within `worktree_path` after canonicalization.
/// Returns the canonical full path on success.
fn safe_join(worktree_path: &str, file_path: &str) -> Result<std::path::PathBuf, String> {
    let joined = std::path::Path::new(worktree_path).join(file_path);
    let canonical = joined
        .canonicalize()
        .map_err(|e| format!("Cannot resolve path '{}': {}", file_path, e))?;
    let canonical_root = std::path::Path::new(worktree_path)
        .canonicalize()
        .map_err(|e| format!("Cannot resolve worktree root: {}", e))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!("file_path '{}' escapes the worktree boundary", file_path));
    }
    Ok(canonical)
}

/// Execute a shell command in the given worktree directory.
/// Returns combined stdout+stderr, truncated to 10 000 chars.
pub fn execute_command_in_worktree(worktree_path: &str, command: &str) -> Result<String, String> {
    let output = if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", command])
            .current_dir(worktree_path)
            .output()
            .map_err(|e| e.to_string())?
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(worktree_path)
            .output()
            .map_err(|e| e.to_string())?
    };

    let mut result = String::new();
    result.push_str(&String::from_utf8_lossy(&output.stdout));
    result.push_str(&String::from_utf8_lossy(&output.stderr));

    if result.len() > 10_000 {
        result.truncate(10_000);
        result.push_str("\n[...truncated]");
    }

    Ok(result)
}

/// Apply search-and-replace edit to a file in the worktree.
pub fn apply_diff_to_file(
    worktree_path: &str,
    file_path: &str,
    old_content: &str,
    new_content: &str,
) -> Result<String, String> {
    let canonical = safe_join(worktree_path, file_path)?;

    let contents = std::fs::read_to_string(&canonical)
        .map_err(|e| format!("Failed to read '{}': {}", file_path, e))?;

    if !contents.contains(old_content) {
        return Err(format!("old_content not found in '{}'", file_path));
    }

    let new_contents = contents.replacen(old_content, new_content, 1);
    std::fs::write(&canonical, &new_contents)
        .map_err(|e| format!("Failed to write '{}': {}", file_path, e))?;

    Ok(format!("Applied edit to '{}'", file_path))
}

/// Read a file from the worktree.
pub fn read_file_from_worktree(
    worktree_path: &str,
    file_path: &str,
    full: bool,
) -> Result<String, String> {
    let canonical = safe_join(worktree_path, file_path)?;

    let contents = std::fs::read_to_string(&canonical)
        .map_err(|e| format!("Failed to read '{}': {}", file_path, e))?;

    if full || contents.lines().count() <= 200 {
        Ok(contents)
    } else {
        let truncated: String = contents.lines().take(200).collect::<Vec<_>>().join("\n");
        Ok(format!(
            "{}\n[...truncated at 200 lines. Use full=true to see all]",
            truncated
        ))
    }
}
