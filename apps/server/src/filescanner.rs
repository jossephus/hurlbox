use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,          // Full absolute path (for reading)
    pub relative_path: String, // Relative path for display
    #[serde(rename = "type")]
    pub node_type: String, // "file" or "folder"
    pub children: Option<Vec<FileNode>>,
}

const DEFAULT_IGNORED_NAMES: &[&str] = &[
    ".git",
    ".svn",
    ".hg",
    ".DS_Store",
    "node_modules",
    "target",
    "__pycache__",
    ".pytest_cache",
    ".venv",
    "venv",
    ".env",
    "dist",
    "build",
    ".next",
    ".nuxt",
    "*.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.lock",
];

fn is_ignored(name: &str, ignored_patterns: &HashSet<String>) -> bool {
    // Check hidden files (starting with .)
    if name.starts_with('.') && name != "." && name != ".." && name != ".hurl" {
        return true;
    }

    // Check against ignored patterns
    if ignored_patterns.contains(name) {
        return true;
    }

    // Check default ignored names
    for &ignored in DEFAULT_IGNORED_NAMES {
        if name == ignored {
            return true;
        }
    }

    false
}

fn parse_gitignore(dir_path: &Path) -> HashSet<String> {
    let mut patterns = HashSet::new();

    // Add default patterns
    for &pattern in DEFAULT_IGNORED_NAMES {
        patterns.insert(pattern.to_string());
    }

    // Try to read .gitignore
    let gitignore_path = dir_path.join(".gitignore");
    if let Ok(content) = std::fs::read_to_string(&gitignore_path) {
        for line in content.lines() {
            let trimmed = line.trim();
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            // Remove trailing slash for directories
            let pattern = trimmed.trim_end_matches('/');
            patterns.insert(pattern.to_string());
        }
    }

    patterns
}

pub fn scan_directory(root_path: &str) -> Result<FileNode, String> {
    let path = Path::new(root_path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", root_path));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", root_path));
    }

    let ignored_patterns = parse_gitignore(path);
    let result = build_file_tree(path, root_path, &ignored_patterns);

    // Return None if no hurl files found
    match result {
        Some(node) => Ok(node),
        None => Err("No .hurl files found in directory".to_string()),
    }
}

fn build_file_tree(
    path: &Path,
    root_path: &str,
    ignored_patterns: &HashSet<String>,
) -> Option<FileNode> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    // Skip ignored files/folders
    if is_ignored(&name, ignored_patterns) {
        return None;
    }

    let full_path = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string();

    let relative_path = if path == Path::new(root_path) {
        String::new()
    } else {
        path.strip_prefix(root_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    };

    if path.is_dir() {
        let mut children: Vec<FileNode> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(path) {
            let mut sorted_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            sorted_entries.sort_by(|a, b| {
                let a_is_dir = a.path().is_dir();
                let b_is_dir = b.path().is_dir();
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                }
            });

            for entry in sorted_entries {
                let entry_path = entry.path();
                if let Some(child) = build_file_tree(&entry_path, root_path, ignored_patterns) {
                    children.push(child);
                }
            }
        }

        // Only include folder if it has children (contains hurl files)
        if children.is_empty() {
            None
        } else {
            Some(FileNode {
                name,
                path: full_path,
                relative_path,
                node_type: "folder".to_string(),
                children: Some(children),
            })
        }
    } else {
        // Only include .hurl files
        if name.ends_with(".hurl") {
            Some(FileNode {
                name,
                path: full_path,
                relative_path,
                node_type: "file".to_string(),
                children: None,
            })
        } else {
            None
        }
    }
}

pub fn read_file(file_path: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", file_path));
    }

    std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
}

pub fn create_file(file_path: &str, content: Option<&str>) -> Result<String, String> {
    let path = Path::new(file_path);

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err(format!(
                "Parent directory does not exist: {}",
                parent.display()
            ));
        }
    }

    if path.exists() {
        return Err(format!("File already exists: {}", file_path));
    }

    let content = content.unwrap_or("");
    std::fs::write(path, content).map_err(|e| format!("Failed to create file: {}", e))?;

    path.canonicalize()
        .map_err(|e| format!("Failed to resolve file path: {}", e))
        .map(|p| p.to_string_lossy().to_string())
}
