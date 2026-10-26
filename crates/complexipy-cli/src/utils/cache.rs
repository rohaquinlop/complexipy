use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use blake2::Blake2bVar;
use blake2::digest::{Update, VariableOutput};
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::{Value, json};

use complexipy_core::classes::FileComplexity;
use complexipy_core::fs_guard::{MAX_DATA_BYTES, is_within, read_bounded_text, write_atomic};

const CACHE_DIR_NAME: &str = ".complexipy_cache";
const CACHE_VALUES_DIR: &str = "v/cache";
const FUNCTIONS_CACHE_KEY: &str = "functions";
const MAX_CACHE_ENTRIES: usize = 64;
const MAX_CACHE_BYTES: usize = 32 * 1024 * 1024;
const CACHEDIR_TAG_CONTENT: &str = "Signature: 8a477f597d28d172789f06886806bc55
# This file is a cache directory tag created by complexipy.
# For information about cache directory tags, see:
#\thttps://bford.info/cachedir/spec.html
";
const README_CONTENT: &str = "# complexipy cache directory #

This directory contains data from complexipy's cache, which stores previous
complexity results so future runs can compare per-function complexity changes.

**Do not** commit this to version control.
";

/// Returns the per-function complexities the previous run on the same
/// targets stored, and stores this run's. The store keeps every other
/// entry as raw text, so a run parses and writes back in full only its own
/// entry, and the store is rewritten only when that entry changed.
pub fn remember_previous_functions(
    invocation_path: &str,
    targets: &[String],
    files_complexities: &[FileComplexity],
    cache_dir: Option<&str>,
) -> Option<HashMap<(String, String, String), u64>> {
    let normalized_targets = normalize_targets(invocation_path, targets);
    let cache_key = cache_key_of(&normalized_targets)?;
    let cache_dir_path = resolve_cache_dir(invocation_path, cache_dir);
    let cache_file = cache_value_path(&cache_dir_path, FUNCTIONS_CACHE_KEY);
    if cache_dir.is_none() && !is_within(Path::new(invocation_path), &cache_file) {
        return None;
    }

    ensure_cache_dir_and_supporting_files(&cache_dir_path)?;

    let content =
        read_bounded_text(&cache_file, Some(&cache_dir_path), MAX_DATA_BYTES).unwrap_or_default();
    let mut cache_store = CacheStore::parse(&content).unwrap_or_default();
    let previous_entry = cache_store.entry(&cache_key);
    let previous_map = previous_entry.as_ref().and_then(load_previous_map);

    let targets = json!(normalized_targets);
    let functions = Value::Array(collect_functions(files_complexities));
    let unchanged = previous_entry.as_ref().is_some_and(|entry| {
        entry.get("targets") == Some(&targets) && entry.get("functions") == Some(&functions)
    });
    if !unchanged {
        let entry = json!({
            "targets": targets,
            "functions": functions,
            "updated_at": now_seconds(),
        });
        if cache_store.insert(&cache_key, &entry) {
            cache_store.prune(&cache_key, MAX_CACHE_BYTES);
            persist_cache(&cache_file, &cache_store);
        }
    }
    previous_map
}

fn resolve_cache_dir(invocation_path: &str, cache_dir: Option<&str>) -> PathBuf {
    match cache_dir {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => Path::new(invocation_path).join(CACHE_DIR_NAME),
    }
}

#[cfg(test)]
fn build_cache_key(invocation_path: &str, targets: &[String]) -> Option<String> {
    cache_key_of(&normalize_targets(invocation_path, targets))
}

fn cache_key_of(normalized_targets: &[String]) -> Option<String> {
    if normalized_targets.is_empty() {
        return None;
    }
    Some(hash_targets(&normalized_targets.join("||")))
}

fn hash_targets(joined: &str) -> String {
    let mut hasher = Blake2bVar::new(16).expect("digest size is valid");
    hasher.update(joined.as_bytes());
    let mut output = [0u8; 16];
    hasher
        .finalize_variable(&mut output)
        .expect("output size matches");
    output.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn normalize_targets(invocation_path: &str, targets: &[String]) -> Vec<String> {
    let mut normalized_targets: Vec<String> = targets
        .iter()
        .filter_map(|target| normalize_target(invocation_path, target))
        .collect();
    normalized_targets.sort();
    normalized_targets
}

fn normalize_target(invocation_path: &str, target: &str) -> Option<String> {
    if looks_like_remote(target) {
        return Some(target.to_string());
    }

    let base_path = Path::new(target);
    let base_path = if base_path.is_absolute() {
        base_path.to_path_buf()
    } else {
        Path::new(invocation_path).join(base_path)
    };

    match fs::canonicalize(&base_path) {
        Ok(resolved) => Some(to_posix(&resolved)),
        Err(_) => Some(to_posix(&lexically_clean(&base_path))),
    }
}

fn lexically_clean(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

fn to_posix(path: &Path) -> String {
    let value = path.to_string_lossy();
    if cfg!(windows) {
        value.replace('\\', "/")
    } else {
        value.into_owned()
    }
}

fn looks_like_remote(target: &str) -> bool {
    const PREFIXES: [&str; 8] = [
        "https://github.com",
        "https://gitlab.com",
        "http://github.com",
        "http://gitlab.com",
        "www.github.com",
        "www.gitlab.com",
        "git@github.com",
        "git@gitlab.com",
    ];
    PREFIXES.iter().any(|prefix| target.starts_with(prefix))
}

fn collect_functions(files_complexities: &[FileComplexity]) -> Vec<Value> {
    files_complexities
        .iter()
        .flat_map(|file_complexity| {
            file_complexity.functions.iter().map(move |function| {
                json!({
                    "path": file_complexity.path.clone(),
                    "file_name": file_complexity.file_name.clone(),
                    "function_name": function.name.clone(),
                    "complexity": function.complexity,
                })
            })
        })
        .collect()
}

fn cache_value_path(cache_dir: &Path, key: &str) -> PathBuf {
    cache_dir.join(CACHE_VALUES_DIR).join(key)
}

/// The cache file: an object whose `entries` maps each cache key to one
/// run's results. Entries and any other top-level field stay unparsed text,
/// borrowed from the file's content.
#[derive(Default)]
struct CacheStore<'a> {
    fields: BTreeMap<String, Cow<'a, RawValue>>,
    entries: BTreeMap<String, Cow<'a, RawValue>>,
}

impl<'a> CacheStore<'a> {
    fn parse(content: &'a str) -> Option<Self> {
        let mut fields: BTreeMap<String, &'a RawValue> = serde_json::from_str(content).ok()?;
        let entries: BTreeMap<String, &'a RawValue> =
            serde_json::from_str(fields.remove("entries")?.get()).ok()?;
        Some(Self {
            fields: borrowed(fields),
            entries: borrowed(entries),
        })
    }

    fn entry(&self, cache_key: &str) -> Option<Value> {
        serde_json::from_str(self.entries.get(cache_key)?.get()).ok()
    }

    fn insert(&mut self, cache_key: &str, entry: &Value) -> bool {
        let Ok(raw) = serde_json::value::to_raw_value(entry) else {
            return false;
        };
        self.entries.insert(cache_key.to_string(), Cow::Owned(raw));
        true
    }

    /// Keeps the `MAX_CACHE_ENTRIES` most recently updated entries, then
    /// drops the oldest of those until the entries fit in `max_bytes`.
    /// The entry under `cache_key` is never dropped for its size.
    fn prune(&mut self, cache_key: &str, max_bytes: usize) {
        let bytes: usize = self.entries.values().map(|raw| raw.get().len()).sum();
        if self.entries.len() <= MAX_CACHE_ENTRIES && bytes <= max_bytes {
            return;
        }

        let mut sorted: Vec<(f64, String, Cow<'a, RawValue>)> = std::mem::take(&mut self.entries)
            .into_iter()
            .map(|(key, raw)| (entry_updated_at(&raw), key, raw))
            .collect();
        sorted.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(MAX_CACHE_ENTRIES);
        let mut kept_bytes = 0;
        for (_, key, raw) in sorted {
            kept_bytes += raw.get().len();
            if key == cache_key || kept_bytes <= max_bytes {
                self.entries.insert(key, raw);
            }
        }
    }
}

fn borrowed<'a>(raw: BTreeMap<String, &'a RawValue>) -> BTreeMap<String, Cow<'a, RawValue>> {
    raw.into_iter()
        .map(|(key, value)| (key, Cow::Borrowed(value)))
        .collect()
}

fn load_previous_map(raw: &Value) -> Option<HashMap<(String, String, String), u64>> {
    let functions = raw.get("functions")?;
    if !functions.is_array() {
        return None;
    }

    let mut mapping = HashMap::new();
    for entry in functions.as_array()? {
        if !entry.is_object() {
            continue;
        }
        let key = build_function_key(
            entry
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or(""),
            entry
                .get("file_name")
                .and_then(|value| value.as_str())
                .unwrap_or(""),
            entry
                .get("function_name")
                .and_then(|value| value.as_str())
                .unwrap_or(""),
        );
        let Some(key) = key else {
            continue;
        };
        let complexity = entry.get("complexity");
        if complexity.is_some_and(|value| value.is_boolean()) {
            continue;
        }
        let Some(complexity) = complexity.and_then(python_int) else {
            continue;
        };
        mapping.insert(key, complexity);
    }

    if mapping.is_empty() {
        None
    } else {
        Some(mapping)
    }
}

fn python_int(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => {
            if let Some(value) = number.as_u64() {
                Some(value)
            } else if let Some(value) = number.as_i64() {
                u64::try_from(value).ok()
            } else if let Some(value) = number.as_f64() {
                if value >= 0.0 {
                    Some(value.trunc() as u64)
                } else {
                    None
                }
            } else {
                None
            }
        }
        Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    }
}

fn build_function_key(
    path: &str,
    file_name: &str,
    function_name: &str,
) -> Option<(String, String, String)> {
    if function_name.is_empty() {
        return None;
    }
    Some((
        path.to_string(),
        file_name.to_string(),
        function_name.to_string(),
    ))
}

#[derive(Deserialize)]
struct EntryStamp {
    updated_at: Option<Value>,
}

fn entry_updated_at(entry: &RawValue) -> f64 {
    let Ok(EntryStamp {
        updated_at: Some(updated_at),
    }) = serde_json::from_str(entry.get())
    else {
        return 0.0;
    };
    if updated_at.is_boolean() {
        return 0.0;
    }
    if let Some(number) = updated_at.as_f64() {
        return number;
    }
    if let Some(text) = updated_at.as_str() {
        return text.parse::<f64>().unwrap_or(0.0);
    }
    0.0
}

fn persist_cache(cache_file: &Path, store: &CacheStore) -> bool {
    let Some(parent) = cache_file.parent() else {
        return false;
    };
    if fs::create_dir_all(parent).is_err() {
        return false;
    }
    let Ok(entries) = serde_json::value::to_raw_value(&store.entries) else {
        return false;
    };
    let mut document: BTreeMap<&str, &RawValue> = store
        .fields
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_ref()))
        .collect();
    document.insert("entries", &entries);
    let Ok(serialized) = serde_json::to_string_pretty(&document) else {
        return false;
    };
    write_atomic(cache_file, serialized.as_bytes()).is_ok()
}

fn ensure_cache_dir_and_supporting_files(cache_dir: &Path) -> Option<()> {
    fs::create_dir_all(cache_dir).ok()?;
    write_support_file(&cache_dir.join(".gitignore"), "*\n");
    write_support_file(&cache_dir.join("CACHEDIR.TAG"), CACHEDIR_TAG_CONTENT);
    write_support_file(&cache_dir.join("README.md"), README_CONTENT);
    Some(())
}

fn write_support_file(path: &Path, content: &str) {
    if fs::symlink_metadata(path).is_ok() {
        return;
    }
    let _ = write_atomic(path, content.as_bytes());
}

fn now_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests;
