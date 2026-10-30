//! Workspace scan: discovery parity with the CLI and diagnostics from disk.

use super::*;
use crate::documents::uri_to_path;
use complexipy_core::config::StringOrList;
use tempfile::TempDir;

const HEAVY: &str =
    "def heavy(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n";

fn strict() -> LspConfig {
    let mut config = LspConfig {
        max_complexity_allowed: 2,
        ..LspConfig::default()
    };
    config.lsp.workspace_diagnostics = true;
    config
}

fn tree(files: &[(&str, &[u8])]) -> TempDir {
    let root = TempDir::new().unwrap();
    for (name, content) in files {
        let path = root.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    root
}

fn root_of(dir: &TempDir) -> String {
    dir.path().to_string_lossy().replace('\\', "/")
}

fn discovered(root: &str, config: LspConfig) -> Vec<String> {
    let workspace = Workspace::new(
        vec![Folder::new(root.to_string(), config)],
        root.to_string(),
    );
    let mut files = discover(&workspace.folders()[0], &workspace);
    files.sort();
    files
}

#[test]
fn discovery_keeps_what_the_cli_analyzes_and_the_folder_does_not_exclude() {
    let dir = tree(&[
        ("heavy.py", HEAVY.as_bytes()),
        (".git/HEAD", b""),
        (".gitignore", b"ignored.py\n"),
        ("ignored.py", HEAVY.as_bytes()),
        (".hidden/hidden.py", HEAVY.as_bytes()),
        ("skip/excluded.py", HEAVY.as_bytes()),
        ("notes.txt", b"x"),
    ]);
    let root = root_of(&dir);
    let config = LspConfig {
        exclude: StringOrList::Many(vec!["skip/**".to_string()]),
        ..strict()
    };

    assert_eq!(
        discovered(&root, config),
        vec![format!("{}/heavy.py", root)]
    );
}

#[test]
fn explicit_paths_entries_follow_the_folder_exclusion() {
    let dir = tree(&[
        ("src/kept.py", HEAVY.as_bytes()),
        ("lone.py", HEAVY.as_bytes()),
        ("outside.py", HEAVY.as_bytes()),
    ]);
    let root = root_of(&dir);
    let config = LspConfig {
        paths: StringOrList::Many(vec!["src".to_string(), "lone.py".to_string()]),
        exclude: StringOrList::Many(vec!["lone.py".to_string()]),
        ..strict()
    };

    assert_eq!(
        discovered(&root, config),
        vec![format!("{}/src/kept.py", root)]
    );
}

#[test]
fn a_nested_folder_with_its_own_config_owns_its_files() {
    let dir = tree(&[
        ("outer.py", HEAVY.as_bytes()),
        ("inner/inner.py", HEAVY.as_bytes()),
    ]);
    let root = root_of(&dir);
    let inner = format!("{}/inner", root);
    let workspace = Workspace::new(
        vec![
            Folder::new(root.clone(), strict()),
            Folder::new(inner.clone(), strict()),
        ],
        root.clone(),
    );

    assert_eq!(
        discover(&workspace.folders()[0], &workspace),
        vec![format!("{}/outer.py", root)]
    );
    assert_eq!(
        discover(&workspace.folders()[1], &workspace),
        vec![format!("{}/inner.py", inner)]
    );
}

#[test]
fn disk_diagnostics_match_the_open_document() {
    let latin1: &[u8] =
        b"# -*- coding: latin-1 -*-\ndef heavy(a, b):\n    s = '\xe9'\n    if a:\n        if b:\n            return s\n    return 0\n";
    let dir = tree(&[("heavy.py", HEAVY.as_bytes()), ("latin.py", latin1)]);
    let root = root_of(&dir);
    let config = strict();
    let expected = analysis::analyze(HEAVY, 0, &config)
        .unwrap()
        .diagnostics(&config);

    assert_eq!(expected.len(), 1);
    assert_eq!(
        diagnostics_from_disk(&format!("{}/heavy.py", root), &root, &config),
        expected
    );
    assert_eq!(
        diagnostics_from_disk(&format!("{}/latin.py", root), &root, &config).len(),
        1
    );
}

#[test]
fn ignored_functions_and_disabled_diagnostics_stay_silent() {
    let ignored = format!("# complexipy: ignore\n{}", HEAVY);
    let dir = tree(&[
        ("ignored.py", ignored.as_bytes()),
        ("broken.py", b"def broken(:\n"),
    ]);
    let root = root_of(&dir);
    let mut quiet = strict();
    quiet.lsp.diagnostics = false;

    assert!(diagnostics_from_disk(&format!("{}/ignored.py", root), &root, &strict()).is_empty());
    assert_eq!(
        diagnostics_from_disk(&format!("{}/broken.py", root), &root, &strict()).len(),
        1
    );
    assert!(diagnostics_from_disk(&format!("{}/broken.py", root), &root, &quiet).is_empty());
}

#[test]
fn canonical_paths_return_to_the_lexical_root() {
    assert_eq!(
        lexical("/private/var/t/pkg/a.py", "/private/var/t", "/var/t"),
        "/var/t/pkg/a.py"
    );
    assert_eq!(
        lexical("//?/C:/repo/a.py", "//?/C:/repo", "C:/repo"),
        "C:/repo/a.py"
    );
    assert_eq!(
        lexical("/elsewhere/a.py", "/private/var/t", "/var/t"),
        "/elsewhere/a.py"
    );
}

#[test]
fn file_uris_round_trip_through_their_paths() {
    for (path, uri) in [
        ("/repo/a b/c.py", "file:///repo/a%20b/c.py"),
        ("C:/repo/c.py", "file:///C:/repo/c.py"),
        ("/repo/caf\u{e9}.py", "file:///repo/caf%C3%A9.py"),
    ] {
        let built = path_to_uri(path).unwrap();
        assert_eq!(built.as_str(), uri);
        assert_eq!(uri_to_path(&built).as_deref(), Some(path));
    }
}

#[test]
fn a_newer_generation_stops_the_scan() {
    let dir = tree(&[("heavy.py", HEAVY.as_bytes())]);
    let root = root_of(&dir);
    let current = Arc::new(AtomicU64::new(2));
    let (done, outcomes) = crossbeam_channel::unbounded();
    let scanner = Scanner {
        current,
        done,
        gate: None,
        urgent: crossbeam_channel::never(),
    };

    scanner.run(ScanJob::Workspace {
        generation: 1,
        workspace: Workspace::new(vec![Folder::new(root.clone(), strict())], root),
    });

    assert!(outcomes.try_recv().is_err());
}

#[test]
fn a_newer_generation_stops_the_scan_between_files() {
    let dir = tree(&[("a.py", HEAVY.as_bytes()), ("b.py", HEAVY.as_bytes())]);
    let root = root_of(&dir);
    let current = Arc::new(AtomicU64::new(1));
    let (done, outcomes) = crossbeam_channel::unbounded();
    let (gate, held) = crossbeam_channel::unbounded();
    let (jobs, _rescans) = spawn_scanner(Arc::clone(&current), done, Some(held));

    jobs.send(ScanJob::Workspace {
        generation: 1,
        workspace: Workspace::new(vec![Folder::new(root.clone(), strict())], root),
    })
    .unwrap();
    gate.send(()).unwrap();
    let timeout = std::time::Duration::from_secs(10);
    assert!(matches!(
        outcomes.recv_timeout(timeout).unwrap(),
        ScanOutcome::Discovered { .. }
    ));
    assert!(matches!(
        outcomes.recv_timeout(timeout).unwrap(),
        ScanOutcome::Scanned { .. }
    ));

    current.store(2, Ordering::SeqCst);
    gate.send(()).unwrap();
    assert!(
        outcomes
            .recv_timeout(std::time::Duration::from_millis(200))
            .is_err()
    );
}

#[test]
fn file_rescans_run_before_the_rest_of_a_workspace_scan() {
    let dir = tree(&[("a.py", HEAVY.as_bytes()), ("b.py", HEAVY.as_bytes())]);
    let root = root_of(&dir);
    let current = Arc::new(AtomicU64::new(1));
    let (done, outcomes) = crossbeam_channel::unbounded();
    let (gate, held) = crossbeam_channel::unbounded();
    let (jobs, rescans) = spawn_scanner(current, done, Some(held));
    let timeout = std::time::Duration::from_secs(10);

    jobs.send(ScanJob::Workspace {
        generation: 1,
        workspace: Workspace::new(vec![Folder::new(root.clone(), strict())], root.clone()),
    })
    .unwrap();
    assert!(matches!(
        outcomes.recv_timeout(timeout).unwrap(),
        ScanOutcome::Discovered { .. }
    ));
    let closed = format!("{}/b.py", root);
    rescans
        .send(ScanJob::Files {
            generation: 1,
            files: vec![ScanFile {
                uri: path_to_uri(&closed).unwrap(),
                path: closed.clone(),
                root,
                config: strict(),
                force: true,
            }],
        })
        .unwrap();
    gate.send(()).unwrap();

    match outcomes.recv_timeout(timeout).unwrap() {
        ScanOutcome::Scanned { path, force, .. } => {
            assert_eq!(path, closed);
            assert!(force);
        }
        _ => panic!("expected the rescan first"),
    }
}
