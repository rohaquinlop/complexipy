use ignore::Walk;
use wax::walk::{Entry, FileIterator};
use wax::{Glob, any};

pub fn get_paths_to_process(root_path: &str, to_exclude_paths: Vec<&str>) -> Vec<String> {
    let mut files_paths: Vec<String> = Vec::new();
    let glob = Glob::new("**/*.py").unwrap();
    let gitignore_walk = Walk::new(root_path);
    let non_ignored: Vec<String> = gitignore_walk
        .filter_map(|result| match result {
            Ok(entry) => entry.path().to_str().map(String::from),
            Err(_) => None,
        })
        .collect();

    for entry in glob
        .walk(root_path)
        .not(any(to_exclude_paths))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path().to_str().expect("Failed to extract");
            non_ignored.contains(&path.to_string())
        })
    {
        let entry_path = entry.path();
        let file_abs = match entry_path.canonicalize() {
            Ok(p) => p,
            Err(_) => entry_path.to_path_buf(),
        };
        let file_abs_str = file_abs.to_string_lossy().replace('\\', "/");
        files_paths.push(file_abs_str.to_string());
    }

    files_paths
}
