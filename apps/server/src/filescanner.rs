use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize)]
pub struct FileNode {
    pub name: String,
    pub relative_path: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub children: Option<Vec<FileNode>>,
}

static CURRENT_ROOT: OnceLock<PathBuf> = OnceLock::new();

#[derive(Default)]
struct TreeNode {
    folders: BTreeMap<String, TreeNode>,
    files: Vec<String>,
}

pub fn set_current_root(root: &str) {
    let _ = CURRENT_ROOT.set(PathBuf::from(root));
}

fn current_root() -> Result<&'static PathBuf, String> {
    CURRENT_ROOT
        .get()
        .ok_or_else(|| "No root path set. Open a file explorer first.".to_string())
}

fn resolve_existing_path(relative_path: &str) -> Result<PathBuf, String> {
    let root = current_root()?;
    let canonical = root
        .join(relative_path)
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;

    if !canonical.starts_with(root) {
        return Err("Path is outside the root directory".to_string());
    }
    Ok(canonical)
}

fn resolve_for_create(relative_path: &str) -> Result<PathBuf, String> {
    let root = current_root()?;
    let candidate = root.join(relative_path);
    let parent = candidate
        .parent()
        .ok_or_else(|| "Invalid target path".to_string())?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("Failed to resolve parent path: {}", e))?;

    if !canonical_parent.starts_with(root) {
        return Err("Path is outside the root directory".to_string());
    }
    Ok(candidate)
}

pub fn scan_directory(root_path: &str) -> Result<FileNode, String> {
    let root = Path::new(root_path);
    if !root.exists() {
        return Err(format!("Path does not exist: {}", root_path));
    }
    if !root.is_dir() {
        return Err(format!("Path is not a directory: {}", root_path));
    }

    let absolute_root = root
        .canonicalize()
        .map_err(|e| format!("Failed to resolve root path: {}", e))?;
    set_current_root(absolute_root.to_string_lossy().as_ref());

    let mut tree = TreeNode::default();
    let walker = ignore::WalkBuilder::new(&absolute_root)
        .hidden(false)
        .git_ignore(true)
        .ignore(true)
        .git_exclude(false)
        .git_global(false)
        .build();

    for entry in walker {
        let entry = entry.map_err(|e| format!("Failed to walk directory: {}", e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("hurl") {
            continue;
        }

        let relative = path
            .strip_prefix(&absolute_root)
            .map_err(|e| format!("Failed to compute relative path: {}", e))?;
        insert_file(&mut tree, relative);
    }

    let children = build_children(&tree, "");
    if children.is_empty() {
        return Err("No .hurl files found in directory".to_string());
    }

    Ok(FileNode {
        name: root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root.to_string_lossy().to_string()),
        relative_path: String::new(),
        node_type: "folder".to_string(),
        children: Some(children),
    })
}

fn insert_file(tree: &mut TreeNode, relative: &Path) {
    let mut current = tree;
    let mut parts = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();

    if let Some(file_name) = parts.pop() {
        for part in parts {
            current = current.folders.entry(part).or_default();
        }
        current.files.push(file_name);
    }
}

fn build_children(tree: &TreeNode, base: &str) -> Vec<FileNode> {
    let mut out = Vec::new();

    for (folder_name, folder_node) in &tree.folders {
        let rel = if base.is_empty() {
            folder_name.clone()
        } else {
            format!("{}/{}", base, folder_name)
        };

        let children = build_children(folder_node, &rel);
        if !children.is_empty() {
            out.push(FileNode {
                name: folder_name.clone(),
                relative_path: rel,
                node_type: "folder".to_string(),
                children: Some(children),
            });
        }
    }

    let mut files = tree.files.clone();
    files.sort();
    for file_name in files {
        let rel = if base.is_empty() {
            file_name.clone()
        } else {
            format!("{}/{}", base, file_name)
        };
        out.push(FileNode {
            name: file_name,
            relative_path: rel,
            node_type: "file".to_string(),
            children: None,
        });
    }

    out
}

pub fn read_file(relative_path: &str) -> Result<String, String> {
    let path = resolve_existing_path(relative_path)?;
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", relative_path));
    }
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
}

pub fn create_file(relative_path: &str, content: Option<&str>) -> Result<String, String> {
    let path = resolve_for_create(relative_path)?;

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err(format!(
                "Parent directory does not exist: {}",
                parent.display()
            ));
        }
    }
    if path.exists() {
        return Err(format!("File already exists: {}", relative_path));
    }

    std::fs::write(&path, content.unwrap_or(""))
        .map_err(|e| format!("Failed to create file: {}", e))?;
    Ok(relative_path.to_string())
}

pub fn write_file(relative_path: &str, content: &str) -> Result<String, String> {
    let path = resolve_existing_path(relative_path)?;
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", relative_path));
    }

    std::fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))?;
    read_file(relative_path)
}
