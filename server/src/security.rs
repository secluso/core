//! SPDX-License-Identifier: GPL-3.0-or-later

use std::io;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

// The check_path_sandboxed check only we previously had only sees if it stays under the current user's root directory.
// It still accepts "." because that canonicalizes to the root itself.
// For route parameters like <camera>, we also need a stricter check.
// We look for exactly one normal path component so "." / ".." / nested paths cannot traverse within the account root.
// This protects root-level per-user files like notification_target.json from being treated as if they were inside a camera directory.
pub(crate) fn validate_path_component(component: &str, label: &str) -> io::Result<()> {
    let mut components = Path::new(component).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must be a single non-special path component"),
        )),
    }
}

pub(crate) fn join_validated_child(
    base: &Path,
    component: &str,
    label: &str,
) -> io::Result<PathBuf> {
    // Validate first so callers never accidentally treat the user root itself as a camera directory by joining "." or another special path.
    validate_path_component(component, label)?;
    Ok(base.join(component))
}

pub(crate) fn check_path_sandboxed(base: &Path, target: &Path) -> io::Result<()> {
    let canonical_base = base.canonicalize()?;

    // Walk up the target path until we find an existing ancestor
    let mut current = target;

    while !current.exists() {
        current = current.parent().ok_or_else(|| {
            io::Error::new(ErrorKind::InvalidInput, "No valid parent for target path")
        })?;
    }

    let canonical_check = current.canonicalize()?;

    if !canonical_check.starts_with(&canonical_base) {
        return Err(io::Error::new(
            ErrorKind::PermissionDenied,
            "Access outside allowed directory",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{join_validated_child, validate_path_component};
    use std::path::Path;

    // This tests to ensure that normal paths work okay.
    #[test]
    fn accepts_normal_single_component() {
        validate_path_component("front-door", "camera").unwrap();
    }

    // This tests to ensure that "." cannot collapse the camera path back to the user root directory.
    #[test]
    fn rejects_current_directory_component() {
        assert!(validate_path_component(".", "camera").is_err());
    }

    // This tests to ensure that the attacker cannot escape outside the user root directory
    #[test]
    fn rejects_parent_directory_component() {
        assert!(validate_path_component("..", "camera").is_err());
    }

    // This tests to ensure the attacker cannot jump over a folder into a sub-folder
    #[test]
    fn rejects_nested_component() {
        assert!(validate_path_component("cam/sub", "camera").is_err());
    }

    // This tests to ensure the helper both preserves normal child joins and rejects special components like "." before they can collapse back to the user root.
    #[test]
    fn join_validated_child_uses_validated_component() {
        let joined = join_validated_child(Path::new("/tmp/base"), "cam1", "camera").unwrap();
        assert_eq!(joined, Path::new("/tmp/base").join("cam1"));
        assert!(join_validated_child(Path::new("/tmp/base"), ".", "camera").is_err());
    }
}
