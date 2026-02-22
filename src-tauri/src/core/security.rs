use std::io;
use std::path::{Path, PathBuf};

/// Validates that a resolved path is confined within the given root.
/// Canonicalizes both paths to resolve symlinks and `..` components.
pub fn validate_confinement(root: &Path, target: &Path) -> Result<PathBuf, SecurityError> {
    let canonical_root = root.canonicalize().map_err(|e| SecurityError::IoError {
        path: root.to_path_buf(),
        source: e,
    })?;
    let canonical_target = target.canonicalize().map_err(|e| SecurityError::IoError {
        path: target.to_path_buf(),
        source: e,
    })?;

    if canonical_target.starts_with(&canonical_root) {
        Ok(canonical_target)
    } else {
        Err(SecurityError::EscapedRoot {
            root: canonical_root,
            target: canonical_target,
        })
    }
}

/// Fast pre-check: rejects relative paths containing `..` traversal.
pub fn check_relative_path(rel_path: &str) -> Result<(), SecurityError> {
    for component in rel_path.split('/') {
        if component == ".." {
            return Err(SecurityError::TraversalAttempt {
                path: rel_path.to_string(),
            });
        }
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Path escaped root: {target:?} is not under {root:?}")]
    EscapedRoot { root: PathBuf, target: PathBuf },
    #[error("Path traversal attempt detected: {path}")]
    TraversalAttempt { path: String },
    #[error("IO error for {path:?}: {source}")]
    IoError { path: PathBuf, source: io::Error },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_confinement_valid() {
        let dir = std::env::temp_dir().join("sc_sec_test_valid");
        let _ = fs::create_dir_all(&dir);
        let child = dir.join("child.txt");
        fs::write(&child, "test").unwrap();

        let result = validate_confinement(&dir, &child);
        assert!(result.is_ok());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_confinement_escape() {
        let dir = std::env::temp_dir().join("sc_sec_test_escape");
        let _ = fs::create_dir_all(&dir);

        // Try to escape to parent
        let escaped = dir.join("..").join("..").join("etc");
        // This may or may not exist, but the confinement check should fail
        // if the path resolves outside root
        if escaped.canonicalize().is_ok() {
            let result = validate_confinement(&dir, &escaped);
            assert!(result.is_err());
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_relative_path_clean() {
        assert!(check_relative_path("src/main.rs").is_ok());
        assert!(check_relative_path("deep/nested/path/file.txt").is_ok());
    }

    #[test]
    fn test_relative_path_traversal() {
        assert!(check_relative_path("../etc/passwd").is_err());
        assert!(check_relative_path("foo/../../bar").is_err());
    }
}
