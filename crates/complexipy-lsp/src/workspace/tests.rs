use super::*;
use tempfile::TempDir;

const MISSING: &str = "/complexipy-missing-root";

fn folder(root: &str, max_complexity_allowed: u64) -> Folder {
    Folder::new(
        root.to_string(),
        LspConfig {
            max_complexity_allowed,
            ..LspConfig::default()
        },
    )
}

fn workspace(folders: Vec<Folder>) -> Workspace {
    Workspace::new(folders, MISSING.to_string())
}

fn roots(workspace: &Workspace) -> Vec<&str> {
    workspace
        .folders
        .iter()
        .map(|folder| folder.root.as_str())
        .collect()
}

#[test]
fn a_path_uses_the_deepest_folder_that_contains_it() {
    let workspace = workspace(vec![folder("/w", 1), folder("/w/sub", 2)]);

    assert_eq!(workspace.for_path("/w/sub/x.py").root, "/w/sub");
    assert_eq!(workspace.for_path("/w/x.py").root, "/w");
}

#[test]
fn a_folder_contains_whole_path_components_only() {
    let workspace = workspace(vec![folder("/a/other", 1), folder("/a/proj", 2)]);

    assert_eq!(workspace.for_path("/a/project/x.py").root, "/a/other");
    assert_eq!(workspace.for_path("/a/proj/x.py").root, "/a/proj");
}

#[test]
fn a_path_outside_every_folder_uses_the_first_folder() {
    let workspace = workspace(vec![folder("/w1", 1), folder("/w2", 2)]);

    assert_eq!(workspace.for_path("/elsewhere/x.py").root, "/w1");
}

#[test]
fn a_uri_maps_to_its_folder() {
    let workspace = workspace(vec![folder("/w1", 1), folder("/w2", 2)]);
    let uri: Uri = "file:///w2/x.py".parse().unwrap();

    assert_eq!(workspace.for_uri(&uri).config.max_complexity_allowed, 2);
}

#[test]
fn no_folder_falls_back_to_the_fallback_root() {
    let workspace = Workspace::load(Vec::new(), MISSING.to_string());

    assert_eq!(roots(&workspace), vec![MISSING]);
    assert_eq!(workspace.for_path("/any/x.py").config, LspConfig::default());
}

#[test]
fn adding_a_known_folder_changes_nothing() {
    let mut workspace = workspace(vec![folder("/w1", 1)]);

    workspace.add("/w1/".to_string());

    assert_eq!(roots(&workspace), vec!["/w1"]);
    assert_eq!(
        workspace.for_path("/w1/x.py").config.max_complexity_allowed,
        1
    );
}

#[test]
fn removing_a_folder_keeps_the_order_of_the_rest() {
    let mut workspace = workspace(vec![folder("/w1", 1), folder("/w2", 2), folder("/w3", 3)]);

    workspace.remove("/w1");

    assert_eq!(roots(&workspace), vec!["/w2", "/w3"]);
    assert_eq!(workspace.for_path("/elsewhere/x.py").root, "/w2");
}

#[test]
fn removing_the_last_folder_falls_back_to_the_fallback_root() {
    let mut workspace = workspace(vec![folder("/w1", 1)]);

    workspace.remove("/w1");

    assert_eq!(roots(&workspace), vec![MISSING]);
}

#[test]
fn a_config_file_in_a_folder_root_is_watched() {
    let workspace = workspace(vec![folder("/w1", 1), folder("/w2/", 2)]);

    assert!(workspace.watches("/w1/complexipy.toml"));
    assert!(workspace.watches("/w1/.complexipy.toml"));
    assert!(workspace.watches("/w2/pyproject.toml"));
}

#[test]
fn other_files_are_not_watched() {
    let workspace = workspace(vec![folder("/w1", 1)]);

    assert!(!workspace.watches("/w1/.venv/pkg/pyproject.toml"));
    assert!(!workspace.watches("/w1/setup.cfg"));
    assert!(!workspace.watches("/elsewhere/complexipy.toml"));
}

#[test]
fn the_fallback_root_is_watched_without_folders() {
    let workspace = Workspace::load(Vec::new(), MISSING.to_string());

    assert!(workspace.watches(&format!("{}/complexipy.toml", MISSING)));
}

fn root_with_config(content: &str) -> TempDir {
    let root = TempDir::new().unwrap();
    std::fs::write(root.path().join("complexipy.toml"), content).unwrap();
    root
}

fn load_warnings(content: &str) -> Vec<String> {
    let root = root_with_config(content);
    Folder::load(root.path().display().to_string()).1
}

#[test]
fn an_unknown_rule_id_is_a_warning() {
    let warnings = load_warnings("select = [\"C07\"]\n");

    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("complexipy.toml")
                && warning.contains(&unknown_rule_warning("C07")))
    );
}

#[test]
fn a_rule_set_with_no_active_rule_is_a_warning() {
    let warnings = load_warnings("select = [\"C007\"]\nignore = [\"C007\"]\n");

    assert!(
        warnings
            .iter()
            .any(|warning| warning.ends_with(&no_active_rule_warning()))
    );
}

#[test]
fn a_toml_syntax_error_is_a_warning() {
    let warnings = load_warnings("max-complexity-allowed = [\n");

    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].starts_with("Failed to parse "));
}

#[test]
fn a_clean_config_has_no_warning() {
    assert!(load_warnings("max-complexity-allowed = 10\n").is_empty());
}

#[test]
fn the_workspace_hands_out_each_warning_once() {
    let root = root_with_config("select = [\"C07\"]\n");
    let mut workspace = Workspace::load(Vec::new(), root.path().display().to_string());

    assert_eq!(workspace.take_warnings().len(), 2);
    assert!(workspace.take_warnings().is_empty());

    workspace.reload();
    assert_eq!(workspace.take_warnings().len(), 2);
}
