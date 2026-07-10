use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::errors::TexError;

pub fn write_atomic(target: &Path, bytes: &[u8], mode: u32) -> Result<(), TexError> {
    let parent = target.parent().ok_or_else(|| {
        TexError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "target has no parent dir",
        ))
    })?;

    fs::create_dir_all(parent).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied {
            path: parent.to_path_buf(),
        },
        _ => TexError::Io(e),
    })?;

    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied {
            path: parent.to_path_buf(),
        },
        _ => TexError::Io(e),
    })?;

    tmp.as_file_mut().write_all(bytes)?;
    tmp.as_file_mut().sync_all()?;

    let mut perms = tmp.as_file().metadata()?.permissions();
    perms.set_mode(mode);
    tmp.as_file().set_permissions(perms)?;

    tmp.persist(target).map_err(|e| match e.error.kind() {
        std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied {
            path: target.to_path_buf(),
        },
        _ => TexError::Io(e.error),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_new_file_with_custom_mode_0644() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("out.tex");
        write_atomic(&target, b"content", 0o644).unwrap();

        assert!(target.exists());
        assert_eq!(std::fs::read(&target).unwrap(), b"content");
        let mode = std::fs::metadata(&target).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o644);
    }

    #[test]
    fn writes_new_file_with_custom_mode_0600() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("secret.toml");
        write_atomic(&target, b"[a]\nx = 1\n", 0o600).unwrap();

        let mode = std::fs::metadata(&target).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn overwrites_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("out.tex");
        std::fs::write(&target, b"original").unwrap();

        write_atomic(&target, b"replaced", 0o644).unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"replaced");
    }

    #[test]
    fn creates_parent_dirs_automatically() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("nested").join("deep").join("out.tex");
        write_atomic(&target, b"body", 0o644).unwrap();

        assert!(target.exists());
        assert_eq!(std::fs::read(&target).unwrap(), b"body");
    }
}
