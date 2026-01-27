use serde::Serialize;

// ============================================================================
// Constants
// ============================================================================

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const COMMIT: &str = env!("BUILD_COMMIT");
pub const BUILD_DATE: &str = env!("BUILD_DATE");
pub const VERSION_STRING: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (commit: ",
    env!("BUILD_COMMIT"),
    ", built: ",
    env!("BUILD_DATE"),
    ")"
);

// ============================================================================
// BuildInfo
// ============================================================================

#[derive(Debug, Serialize)]
pub struct BuildInfo {
    pub version: &'static str,
    pub commit: &'static str,
    pub build_date: &'static str,
}

impl BuildInfo {
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: VERSION,
            commit: COMMIT,
            build_date: BUILD_DATE,
        }
    }
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_info_new() {
        let info = BuildInfo::new();
        assert!(!info.version.is_empty());
        assert!(!info.commit.is_empty());
        assert!(!info.build_date.is_empty());
    }

    #[test]
    fn test_build_info_default() {
        let info = BuildInfo::default();
        assert_eq!(info.version, VERSION);
        assert_eq!(info.commit, COMMIT);
        assert_eq!(info.build_date, BUILD_DATE);
    }

    #[test]
    fn test_version_string_format() {
        assert!(VERSION_STRING.contains(VERSION));
        assert!(VERSION_STRING.contains("commit:"));
        assert!(VERSION_STRING.contains("built:"));
    }
}
