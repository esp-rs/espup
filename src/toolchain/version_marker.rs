//! Completion state for toolchains whose versions share an installation directory.

use std::{fs, io, path::PathBuf};

pub(super) struct VersionMarker {
    path: PathBuf,
}

impl VersionMarker {
    pub(super) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(super) fn matches(&self, version: &str) -> bool {
        fs::read_to_string(&self.path).is_ok_and(|installed| installed == version)
    }

    /// Invalidate before changing installed files, so failures cannot reuse old state.
    pub(super) fn invalidate(&self) -> io::Result<()> {
        match fs::remove_file(&self.path) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }

    /// Record a version only after all of its archives have been extracted.
    pub(super) fn complete(&self, version: &str) -> io::Result<()> {
        fs::write(&self.path, version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_versions_does_not_reuse_stale_markers() {
        let dir = tempfile::tempdir().unwrap();
        let marker = VersionMarker::new(dir.path().join(".espup-installed"));
        // Legacy per-version files are not evidence of the currently installed version.
        fs::write(dir.path().join("version-a"), "").unwrap();
        assert!(!marker.matches("version-a"));
        marker.invalidate().unwrap();
        marker.complete("version-a").unwrap();
        assert!(marker.matches("version-a"));
        marker.invalidate().unwrap();
        assert!(!marker.matches("version-a"));
        assert!(!marker.matches("version-b"));
        marker.complete("version-b").unwrap();
        assert!(marker.matches("version-b"));
        assert!(!marker.matches("version-a"));
        marker.invalidate().unwrap();
        marker.complete("version-a").unwrap();
        assert!(marker.matches("version-a"));
        assert!(!marker.matches("version-b"));
    }

    #[test]
    fn extended_installations_require_a_matching_completion_marker() {
        let dir = tempfile::tempdir().unwrap();
        let libs = VersionMarker::new(dir.path().join(".espup-installed"));
        let extended = VersionMarker::new(dir.path().join(".espup-installed-extended"));
        libs.complete("version-a").unwrap();
        assert!(!extended.matches("version-a"));
        libs.invalidate().unwrap();
        extended.invalidate().unwrap();
        // An interrupted extended install cannot be mistaken for a completed install.
        assert!(!libs.matches("version-a"));
        assert!(!extended.matches("version-a"));
        libs.complete("version-a").unwrap();
        extended.complete("version-a").unwrap();
        assert!(libs.matches("version-a"));
        assert!(extended.matches("version-a"));
        libs.invalidate().unwrap();
        extended.invalidate().unwrap();
        libs.complete("version-b").unwrap();
        assert!(!extended.matches("version-a"));
        assert!(!extended.matches("version-b"));
    }
}
