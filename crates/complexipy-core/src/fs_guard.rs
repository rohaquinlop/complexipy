use std::collections::hash_map::RandomState;
use std::fs::{self, File, OpenOptions};
use std::hash::{BuildHasher, Hasher};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Replaces `path` with `contents` through a new temporary file in the same
/// directory and a rename: a reader sees the old file or the new one, never
/// a partial one, and a symlink at `path` is replaced instead of followed.
/// The new file keeps the permission bits of the regular file it replaces,
/// minus setuid and setgid, and its group where the platform allows.
pub fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| refused("has no file name"))?
        .to_string_lossy();
    let previous = fs::symlink_metadata(path)
        .ok()
        .filter(|metadata| metadata.is_file());

    let (temp_path, mut file) = create_temp(parent, &name)?;
    let written = file
        .write_all(contents)
        .and_then(|()| keep_metadata(&file, previous.as_ref()))
        .and_then(|()| {
            drop(file);
            fs::rename(&temp_path, path)
        });
    if written.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    written
}

fn create_temp(dir: &Path, name: &str) -> io::Result<(PathBuf, File)> {
    for _ in 0..16 {
        let candidate = dir.join(format!(".{}.{:016x}.tmp", name, random_suffix()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no free temporary file name",
    ))
}

fn random_suffix() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u128(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos()),
    );
    hasher.finish()
}

#[cfg(unix)]
fn keep_metadata(file: &File, previous: Option<&fs::Metadata>) -> io::Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt, fchown};

    let Some(previous) = previous else {
        return Ok(());
    };
    let _ = fchown(file, None, Some(previous.gid()));
    file.set_permissions(fs::Permissions::from_mode(previous.mode() & 0o777))
}

#[cfg(not(unix))]
fn keep_metadata(file: &File, previous: Option<&fs::Metadata>) -> io::Result<()> {
    match previous {
        Some(previous) => file.set_permissions(previous.permissions()),
        None => Ok(()),
    }
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
