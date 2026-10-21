use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

pub const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_CONFIG_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_DATA_BYTES: u64 = 64 * 1024 * 1024;

/// Reads a regular file of at most `limit` bytes. A device, a FIFO, or a
/// directory is refused before it is opened, and when `root` is given, a
/// path that resolves outside `root`, through a symlink at any component,
/// is refused too.
pub fn read_bounded(path: &Path, root: Option<&Path>, limit: u64) -> io::Result<Vec<u8>> {
    if let Some(root) = root
        && !fs::canonicalize(path)?.starts_with(fs::canonicalize(root)?)
    {
        return Err(refused("resolves outside the project directory"));
    }
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(refused("is not a regular file"));
    }
    if metadata.len() > limit {
        return Err(too_large(limit));
    }

    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(refused("is not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(too_large(limit));
    }
    Ok(bytes)
}

/// [`read_bounded`] for a UTF-8 text file.
pub fn read_bounded_text(path: &Path, root: Option<&Path>, limit: u64) -> io::Result<String> {
    String::from_utf8(read_bounded(path, root, limit)?)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn refused(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, reason.to_string())
}

fn too_large(limit: u64) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("is larger than {} MiB", limit / (1024 * 1024)),
    )
}

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
