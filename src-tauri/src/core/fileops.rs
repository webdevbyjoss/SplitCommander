use std::fs;
use std::path::{Path, PathBuf};

use crate::core::security;

/// Copies a file or directory recursively from `src` to `dest_dir/<src_name>`.
/// Fails if destination already exists.
pub fn copy_entry(src: &Path, dest_dir: &Path) -> Result<PathBuf, String> {
    let name = src
        .file_name()
        .ok_or_else(|| "Invalid source path".to_string())?;
    let dest = dest_dir.join(name);

    if dest.exists() {
        return Err(format!("Destination already exists: {}", dest.display()));
    }

    if src.is_dir() {
        copy_dir_recursive(src, &dest)?;
    } else {
        fs::copy(src, &dest).map_err(|e| format!("Copy failed: {}", e))?;
    }

    Ok(dest)
}

/// Copies a file or directory from `src` to `dest_dir/<src_name>`, overwriting if destination exists.
pub fn copy_entry_overwrite(src: &Path, dest_dir: &Path) -> Result<PathBuf, String> {
    let name = src
        .file_name()
        .ok_or_else(|| "Invalid source path".to_string())?;
    let dest = dest_dir.join(name);

    // Remove existing destination if present
    if dest.exists() {
        if dest.is_dir() {
            fs::remove_dir_all(&dest).map_err(|e| format!("Cannot remove existing: {}", e))?;
        } else {
            fs::remove_file(&dest).map_err(|e| format!("Cannot remove existing: {}", e))?;
        }
    }

    if src.is_dir() {
        copy_dir_recursive(src, &dest)?;
    } else {
        fs::copy(src, &dest).map_err(|e| format!("Copy failed: {}", e))?;
    }

    Ok(dest)
}

/// Moves a file or directory from `src` to `dest_dir/<src_name>`.
/// Uses `fs::rename` when possible, falls back to copy+delete for cross-filesystem moves.
pub fn move_entry(src: &Path, dest_dir: &Path) -> Result<PathBuf, String> {
    let name = src
        .file_name()
        .ok_or_else(|| "Invalid source path".to_string())?;
    let dest = dest_dir.join(name);

    if dest.exists() {
        return Err(format!("Destination already exists: {}", dest.display()));
    }

    // Try rename first (instant on same filesystem)
    match fs::rename(src, &dest) {
        Ok(()) => Ok(dest),
        Err(_) => {
            // Cross-filesystem: copy then delete
            if src.is_dir() {
                copy_dir_recursive(src, &dest)?;
                fs::remove_dir_all(src)
                    .map_err(|e| format!("Remove source failed: {}", e))?;
            } else {
                fs::copy(src, &dest).map_err(|e| format!("Copy failed: {}", e))?;
                fs::remove_file(src)
                    .map_err(|e| format!("Remove source failed: {}", e))?;
            }
            Ok(dest)
        }
    }
}

/// Creates a new directory inside `parent` with the given `name`.
/// Rejects names containing `..` traversal via security checks.
pub fn create_directory(parent: &Path, name: &str) -> Result<PathBuf, String> {
    security::check_relative_path(name).map_err(|e| e.to_string())?;

    let new_dir = parent.join(name);
    if new_dir.exists() {
        return Err(format!("Already exists: {}", new_dir.display()));
    }

    fs::create_dir(&new_dir).map_err(|e| format!("Cannot create directory: {}", e))?;
    Ok(new_dir)
}

/// Deletes a file or directory (recursively for directories).
pub fn delete_entry(target: &Path) -> Result<(), String> {
    if target.is_dir() {
        fs::remove_dir_all(target).map_err(|e| format!("Delete failed: {}", e))
    } else {
        fs::remove_file(target).map_err(|e| format!("Delete failed: {}", e))
    }
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir(dest)
        .map_err(|e| format!("Cannot create {}: {}", dest.display(), e))?;

    for entry in
        fs::read_dir(src).map_err(|e| format!("Cannot read {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)
                .map_err(|e| format!("Copy {} failed: {}", src_path.display(), e))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sc_fileops_{}", name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_copy_file() {
        let dir = test_dir("copy_file");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("dst")).unwrap();
        fs::write(dir.join("src/test.txt"), "hello").unwrap();

        let result = copy_entry(&dir.join("src/test.txt"), &dir.join("dst"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), dir.join("dst/test.txt"));
        assert!(dir.join("dst/test.txt").exists());
        assert_eq!(fs::read_to_string(dir.join("dst/test.txt")).unwrap(), "hello");
        // Source still exists
        assert!(dir.join("src/test.txt").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_copy_dir_recursive() {
        let dir = test_dir("copy_dir");
        fs::create_dir_all(dir.join("src/sub")).unwrap();
        fs::write(dir.join("src/a.txt"), "aaa").unwrap();
        fs::write(dir.join("src/sub/b.txt"), "bbb").unwrap();
        fs::create_dir_all(dir.join("dst")).unwrap();

        let result = copy_entry(&dir.join("src"), &dir.join("dst"));
        assert!(result.is_ok());
        assert!(dir.join("dst/src/a.txt").exists());
        assert!(dir.join("dst/src/sub/b.txt").exists());
        assert_eq!(fs::read_to_string(dir.join("dst/src/sub/b.txt")).unwrap(), "bbb");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_copy_collision() {
        let dir = test_dir("copy_collision");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("dst")).unwrap();
        fs::write(dir.join("src/test.txt"), "hello").unwrap();
        fs::write(dir.join("dst/test.txt"), "existing").unwrap();

        let result = copy_entry(&dir.join("src/test.txt"), &dir.join("dst"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
        // Original not overwritten
        assert_eq!(fs::read_to_string(dir.join("dst/test.txt")).unwrap(), "existing");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_move_file() {
        let dir = test_dir("move_file");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("dst")).unwrap();
        fs::write(dir.join("src/test.txt"), "hello").unwrap();

        let result = move_entry(&dir.join("src/test.txt"), &dir.join("dst"));
        assert!(result.is_ok());
        assert!(dir.join("dst/test.txt").exists());
        assert_eq!(fs::read_to_string(dir.join("dst/test.txt")).unwrap(), "hello");
        // Source removed
        assert!(!dir.join("src/test.txt").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_move_collision() {
        let dir = test_dir("move_collision");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("dst")).unwrap();
        fs::write(dir.join("src/test.txt"), "new").unwrap();
        fs::write(dir.join("dst/test.txt"), "existing").unwrap();

        let result = move_entry(&dir.join("src/test.txt"), &dir.join("dst"));
        assert!(result.is_err());
        // Source still exists, dest not overwritten
        assert!(dir.join("src/test.txt").exists());
        assert_eq!(fs::read_to_string(dir.join("dst/test.txt")).unwrap(), "existing");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_directory() {
        let dir = test_dir("mkdir");

        let result = create_directory(&dir, "new_folder");
        assert!(result.is_ok());
        assert!(dir.join("new_folder").is_dir());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_directory_traversal_rejected() {
        let dir = test_dir("mkdir_traversal");

        let result = create_directory(&dir, "../escape");
        assert!(result.is_err());
        assert!(!dir.join("../escape").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_directory_collision() {
        let dir = test_dir("mkdir_collision");
        fs::create_dir(dir.join("existing")).unwrap();

        let result = create_directory(&dir, "existing");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Already exists"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_delete_file() {
        let dir = test_dir("delete_file");
        fs::write(dir.join("doomed.txt"), "bye").unwrap();

        let result = delete_entry(&dir.join("doomed.txt"));
        assert!(result.is_ok());
        assert!(!dir.join("doomed.txt").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_copy_overwrite_file() {
        let dir = test_dir("copy_overwrite");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("dst")).unwrap();
        fs::write(dir.join("src/test.txt"), "new content").unwrap();
        fs::write(dir.join("dst/test.txt"), "old content").unwrap();

        let result = copy_entry_overwrite(&dir.join("src/test.txt"), &dir.join("dst"));
        assert!(result.is_ok());
        assert_eq!(fs::read_to_string(dir.join("dst/test.txt")).unwrap(), "new content");
        assert!(dir.join("src/test.txt").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_copy_overwrite_no_existing() {
        let dir = test_dir("copy_overwrite_new");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("dst")).unwrap();
        fs::write(dir.join("src/test.txt"), "hello").unwrap();

        let result = copy_entry_overwrite(&dir.join("src/test.txt"), &dir.join("dst"));
        assert!(result.is_ok());
        assert_eq!(fs::read_to_string(dir.join("dst/test.txt")).unwrap(), "hello");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_delete_dir() {
        let dir = test_dir("delete_dir");
        fs::create_dir_all(dir.join("doomed/sub")).unwrap();
        fs::write(dir.join("doomed/sub/file.txt"), "bye").unwrap();

        let result = delete_entry(&dir.join("doomed"));
        assert!(result.is_ok());
        assert!(!dir.join("doomed").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
