use std::path::{Component, Path, PathBuf};

/// Whether `candidate`, joined onto `root`, stays inside `root`: lexically,
/// and again after the symlinks of its deepest existing ancestor resolve.
pub fn is_within(root: &Path, candidate: &Path) -> bool {
    let Ok(root) = std::path::absolute(root) else {
        return false;
    };
    let root = lexically_normal(&root);
    let joined = lexically_normal(&root.join(candidate));
    if !joined.starts_with(&root) {
        return false;
    }

    match (root.canonicalize(), resolve_existing_prefix(&joined)) {
        (Ok(real_root), Some(real_prefix)) => real_prefix.starts_with(real_root),
        _ => false,
    }
}

fn resolve_existing_prefix(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| ancestor.symlink_metadata().is_ok())
        .and_then(|ancestor| ancestor.canonicalize().ok())
}

fn lexically_normal(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            other => normal.push(other.as_os_str()),
        }
    }
    normal
}

#[cfg(test)]
mod tests;
