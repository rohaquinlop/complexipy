use super::*;

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
