use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use blake2::Blake2bVar;
use blake2::digest::{Update, VariableOutput};
use serde_json::{Value, json};

use complexipy_core::classes::FileComplexity;

const CACHE_DIR_NAME: &str = ".complexipy_cache";
const CACHE_VALUES_DIR: &str = "v/cache";
const FUNCTIONS_CACHE_KEY: &str = "functions";
const MAX_CACHE_ENTRIES: usize = 64;
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

pub fn remember_previous_functions(
    invocation_path: &str,
    targets: &[String],
    files_complexities: &[FileComplexity],
    cache_dir: Option<&str>,
) -> Option<HashMap<(String, String, String), u64>> {
    let cache_key = build_cache_key(invocation_path, targets)?;

    let cache_dir_path = resolve_cache_dir(invocation_path, cache_dir);
    let cache_file = cache_value_path(&cache_dir_path, FUNCTIONS_CACHE_KEY);

    ensure_cache_dir_and_supporting_files(&cache_dir_path)?;

    let mut cache_store = load_cache_store(&cache_file);
    let previous_map = load_previous_map_from_store(&cache_store, &cache_key);

    cache_store["entries"][cache_key.as_str()] = json!({
        "targets": normalize_targets(invocation_path, targets),
        "functions": collect_functions(files_complexities),
        "updated_at": now_seconds(),
    });
    prune_cache_store(&mut cache_store);
    persist_cache(&cache_file, &cache_store);
    previous_map
}

fn resolve_cache_dir(invocation_path: &str, cache_dir: Option<&str>) -> PathBuf {
    match cache_dir {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => Path::new(invocation_path).join(CACHE_DIR_NAME),
    }
}

fn build_cache_key(invocation_path: &str, targets: &[String]) -> Option<String> {
    let normalized_targets = normalize_targets(invocation_path, targets);
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

fn load_cache_store(cache_file: &Path) -> Value {
    match load_cache(cache_file) {
        Some(raw)
            if raw
                .get("entries")
                .is_some_and(|entries| entries.is_object()) =>
        {
            raw
        }
        _ => json!({ "entries": {} }),
    }
}

fn load_previous_map_from_store(
    cache_store: &Value,
    cache_key: &str,
) -> Option<HashMap<(String, String, String), u64>> {
    let entry = cache_store.get("entries")?.get(cache_key)?;
    if !entry.is_object() {
        return None;
    }
    load_previous_map(entry)
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

fn load_cache(cache_file: &Path) -> Option<Value> {
    let content = fs::read_to_string(cache_file).ok()?;
    let raw: Value = serde_json::from_str(&content).ok()?;
    if !raw.is_object() {
        return None;
    }
    Some(raw)
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

fn prune_cache_store(cache_store: &mut Value) {
    let Some(entries) = cache_store
        .get_mut("entries")
        .and_then(|entries| entries.as_object_mut())
    else {
        return;
    };
    if entries.len() <= MAX_CACHE_ENTRIES {
        return;
    }

    let taken = std::mem::take(entries);
    let mut sorted_entries: Vec<(String, Value)> = taken.into_iter().collect();
    sorted_entries.sort_by(|left, right| {
        entry_updated_at(&right.1)
            .partial_cmp(&entry_updated_at(&left.1))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted_entries.truncate(MAX_CACHE_ENTRIES);
    for (key, value) in sorted_entries {
        entries.insert(key, value);
    }
}

fn entry_updated_at(entry: &Value) -> f64 {
    if !entry.is_object() {
        return 0.0;
    }
    let Some(updated_at) = entry.get("updated_at") else {
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

fn persist_cache(cache_file: &Path, payload: &Value) -> bool {
    let Some(parent) = cache_file.parent() else {
        return false;
    };
    if fs::create_dir_all(parent).is_err() {
        return false;
    }
    let Ok(serialized) = serde_json::to_string_pretty(payload) else {
        return false;
    };
    fs::write(cache_file, serialized).is_ok()
}

fn ensure_cache_dir_and_supporting_files(cache_dir: &Path) -> Option<()> {
    fs::create_dir_all(cache_dir).ok()?;
    write_support_file(&cache_dir.join(".gitignore"), "*\n");
    write_support_file(&cache_dir.join("CACHEDIR.TAG"), CACHEDIR_TAG_CONTENT);
    write_support_file(&cache_dir.join("README.md"), README_CONTENT);
    Some(())
}

fn write_support_file(path: &Path, content: &str) {
    if path.exists() {
        return;
    }
    let _ = fs::write(path, content);
}

fn now_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests;
