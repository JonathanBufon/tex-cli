use std::path::{Component, Path, PathBuf};

use crate::errors::TexError;

pub fn config_file_path() -> Result<PathBuf, TexError> {
    let base = dirs::config_dir().ok_or(TexError::HomeDirUnavailable)?;
    Ok(base.join("tex").join("config.toml"))
}

pub fn expand_user_path(raw: &str) -> Result<PathBuf, TexError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(PathBuf::from(raw));
    }

    let expanded: PathBuf = if trimmed == "~" {
        dirs::home_dir().ok_or(TexError::HomeDirUnavailable)?
    } else if let Some(rest) = trimmed.strip_prefix("~/") {
        let home = dirs::home_dir().ok_or(TexError::HomeDirUnavailable)?;
        home.join(rest)
    } else if Path::new(trimmed).is_absolute() {
        PathBuf::from(trimmed)
    } else {
        let cwd = std::env::current_dir()?;
        cwd.join(trimmed)
    };

    Ok(normalize(&expanded))
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let popped = out.pop();
                if !popped {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilde_alone_expands_to_home() {
        let expanded = expand_user_path("~").unwrap();
        assert_eq!(expanded, dirs::home_dir().unwrap());
    }

    #[test]
    fn tilde_slash_expands_to_home_subpath() {
        let expanded = expand_user_path("~/foo").unwrap();
        assert_eq!(expanded, dirs::home_dir().unwrap().join("foo"));
    }

    #[test]
    fn relative_path_canonicalizes_against_cwd() {
        let expanded = expand_user_path("./bar").unwrap();
        let expected = std::env::current_dir().unwrap().join("bar");
        assert_eq!(expanded, normalize(&expected));
    }

    #[test]
    fn absolute_path_normalized_but_not_touched() {
        let expanded = expand_user_path("/tmp/foo").unwrap();
        assert_eq!(expanded, PathBuf::from("/tmp/foo"));
    }

    #[test]
    fn double_dot_collapses() {
        let expanded = expand_user_path("/tmp/foo/../bar").unwrap();
        assert_eq!(expanded, PathBuf::from("/tmp/bar"));
    }

    #[test]
    fn current_dir_component_is_dropped() {
        let expanded = expand_user_path("/tmp/./foo").unwrap();
        assert_eq!(expanded, PathBuf::from("/tmp/foo"));
    }

    #[test]
    fn config_file_path_ends_in_tex_config_toml() {
        let p = config_file_path().unwrap();
        assert!(p.ends_with("tex/config.toml"));
    }
}
