//! Documentation listing/reading for the desktop Help viewer.
//!
//! Docs live in `apps/airtable-sync/docs/` in the source tree and are bundled
//! as a Tauri resource (`docs/`) for packaged builds — see
//! [`resolve_root`] for how the two are reconciled.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Manager;

/// One entry in the Help sidebar's table of contents.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocEntry {
    /// Path relative to the docs root (`/`-separated).
    pub path: String,
    /// Display name, derived from the file name.
    pub name: String,
    /// Nesting depth, for indentation.
    pub depth: u32,
}

/// Locates the docs root: the bundled resource directory in a packaged app,
/// falling back to the source tree in `tauri dev` (whose working directory is
/// `src-tauri/`, same heuristic as `main.rs`'s config/log path resolution).
pub fn resolve_root<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> PathBuf {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("docs");
        if bundled.is_dir() {
            return bundled;
        }
    }
    for candidate in ["docs", "../docs"] {
        let path = PathBuf::from(candidate);
        if path.is_dir() {
            return path;
        }
    }
    PathBuf::from("../docs")
}

/// Lists the docs root's Markdown entries, `index.md` first, then alphabetical.
///
/// The `plan/` subtree holds implementation planning notes rather than user
/// docs and is skipped.
pub fn list(root: &Path) -> Result<Vec<DocEntry>, String> {
    let mut paths = Vec::new();
    collect_markdown(root, root, &mut paths)?;
    paths.sort();
    if let Some(index) = paths.iter().position(|path| path == "index.md") {
        let entry = paths.remove(index);
        paths.insert(0, entry);
    }

    Ok(paths
        .into_iter()
        .map(|path| DocEntry {
            depth: path.matches('/').count() as u32,
            name: display_name(&path),
            path,
        })
        .collect())
}

/// Reads a Markdown file relative to `root`.
pub fn read(root: &Path, rel_path: &str) -> Result<String, String> {
    let rel = rel_path.trim().trim_start_matches('/');
    if rel.is_empty() || rel.contains("..") {
        return Err("invalid document path".to_string());
    }

    let path = root.join(rel);
    if !path.is_file() {
        return Err(format!("document not found: {rel}"));
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return Err("only .md files can be read".to_string());
    }

    fs::read_to_string(&path).map_err(|error| format!("failed to read {rel}: {error}"))
}

fn collect_markdown(dir: &Path, root: &Path, paths: &mut Vec<String>) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(dir)
        .map_err(|error| format!("failed to read {}: {error}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("plan") {
                continue;
            }
            collect_markdown(&path, root, paths)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| "path outside docs directory".to_string())?;
            paths.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

fn display_name(path: &str) -> String {
    if path == "index.md" {
        return "Overview".to_string();
    }
    let file = path.rsplit('/').next().unwrap_or(path);
    humanize_segment(file.strip_suffix(".md").unwrap_or(file))
}

fn humanize_segment(segment: &str) -> String {
    segment
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn display_name_special_cases_index() {
        assert_eq!(display_name("index.md"), "Overview");
    }

    #[test]
    fn display_name_humanizes_top_level_file() {
        assert_eq!(display_name("config.md"), "Config");
    }

    /// Builds a scratch docs tree, runs `f`, then removes it.
    fn with_fixture_root(f: impl FnOnce(&Path)) {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("airtable-sync-docs-test-{id}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("plan")).unwrap();
        fs::write(root.join("index.md"), "# Overview").unwrap();
        fs::write(root.join("architecture.md"), "# Architecture").unwrap();
        fs::write(root.join("plan").join("milestone-1.md"), "# Milestone 1").unwrap();

        f(&root);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn list_puts_index_first_and_skips_plan() {
        with_fixture_root(|root| {
            let entries = list(root).unwrap();
            let paths: Vec<_> = entries.iter().map(|e| e.path.as_str()).collect();
            assert_eq!(paths, vec!["index.md", "architecture.md"]);
        });
    }

    #[test]
    fn read_rejects_traversal_and_non_markdown() {
        with_fixture_root(|root| {
            assert!(read(root, "../index.md").is_err());
            assert!(read(root, "index.md").is_ok());
            assert_eq!(read(root, "index.md").unwrap(), "# Overview");
        });
    }
}
