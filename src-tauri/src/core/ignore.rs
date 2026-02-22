use glob_match::glob_match;

/// Default macOS noise patterns to ignore.
pub const MACOS_NOISE: &[&str] = &[
    ".DS_Store",
    "._*",
    ".Spotlight-V100",
    ".Trashes",
    ".fseventsd",
    ".TemporaryItems",
    ".VolumeIcon.icns",
    "__MACOSX",
    "Thumbs.db",
];

pub struct IgnoreRules {
    patterns: Vec<String>,
}

impl IgnoreRules {
    /// Creates rules from user patterns merged with macOS noise preset.
    pub fn new(user_patterns: &[String]) -> Self {
        let mut patterns: Vec<String> = MACOS_NOISE.iter().map(|s| s.to_string()).collect();
        patterns.extend_from_slice(user_patterns);
        Self { patterns }
    }

    /// Returns true if the given relative path (or its filename) should be ignored.
    pub fn is_ignored(&self, rel_path: &str) -> bool {
        let filename = rel_path.rsplit('/').next().unwrap_or(rel_path);
        self.patterns
            .iter()
            .any(|pattern| glob_match(pattern, filename) || glob_match(pattern, rel_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ds_store_ignored() {
        let rules = IgnoreRules::new(&[]);
        assert!(rules.is_ignored(".DS_Store"));
        assert!(rules.is_ignored("some/nested/.DS_Store"));
    }

    #[test]
    fn test_dot_underscore_ignored() {
        let rules = IgnoreRules::new(&[]);
        assert!(rules.is_ignored("._foo"));
        assert!(rules.is_ignored("deep/path/._bar"));
    }

    #[test]
    fn test_spotlight_ignored() {
        let rules = IgnoreRules::new(&[]);
        assert!(rules.is_ignored(".Spotlight-V100"));
    }

    #[test]
    fn test_normal_files_not_ignored() {
        let rules = IgnoreRules::new(&[]);
        assert!(!rules.is_ignored("readme.md"));
        assert!(!rules.is_ignored("src/main.rs"));
        assert!(!rules.is_ignored("package.json"));
    }

    #[test]
    fn test_user_patterns() {
        let rules = IgnoreRules::new(&["*.log".to_string(), "node_modules".to_string()]);
        assert!(rules.is_ignored("debug.log"));
        assert!(rules.is_ignored("node_modules"));
        // Also still ignores macOS noise
        assert!(rules.is_ignored(".DS_Store"));
    }
}
