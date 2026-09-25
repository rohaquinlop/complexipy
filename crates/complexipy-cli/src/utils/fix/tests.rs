use std::fs;

use complexipy_core::classes::{FileComplexity, FunctionComplexity};
use complexipy_core::cognitive_complexity::code_complexity_shared;
use complexipy_core::{AnalysisOptions, RuleSet};
use tempfile::tempdir;

use super::{run_fix_pass, write_patched};

const TWO_SPOTS: &str = "def f(a, b, c, d):\n    if a:\n        if b:\n            return 1\n    if c:\n        if d:\n            return 2\n    return 0\n";

fn options() -> AnalysisOptions {
    AnalysisOptions {
        check_script: true,
        no_ignore: true,
        with_plans: true,
        rules: RuleSet::default(),
    }
}

#[test]
fn write_patched_replaces_content_and_leaves_no_temp_file() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("a.py");
    fs::write(&file, "def a():\n    if x:\n        return 1\n").expect("should write");

    write_patched(&file, "def a():\n    pass\n").expect("should patch");

    assert_eq!(
        fs::read_to_string(&file).expect("should read"),
        "def a():\n    pass\n"
    );
    let leftovers: Vec<String> = fs::read_dir(dir.path())
        .expect("should list")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(leftovers, vec!["a.py".to_string()]);
}

#[test]
fn write_patched_refuses_unparseable_text() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("a.py");
    fs::write(&file, "def a():\n    pass\n").expect("should write");

    let result = write_patched(&file, "def a(): (:");

    assert!(result.is_err());
    assert_eq!(
        fs::read_to_string(&file).expect("should read"),
        "def a():\n    pass\n"
    );
}

#[test]
fn later_passes_fix_plans_that_were_not_surfaced_at_first() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("two.py");
    fs::write(&file, TWO_SPOTS).expect("should write");

    let complexity = code_complexity_shared(TWO_SPOTS, &options()).expect("should analyze");
    let first = complexity
        .functions
        .into_iter()
        .flat_map(|function| function.refactor_plans)
        .next()
        .expect("one fixable plan exists");
    let files = vec![FileComplexity {
        path: "two.py".to_string(),
        file_name: "two.py".to_string(),
        functions: vec![FunctionComplexity {
            name: "f".to_string(),
            complexity: 0,
            line_start: 1,
            line_end: 8,
            line_complexities: vec![],
            refactor_plans: vec![first],
            additional_refactor_plans: 0,
        }],
        complexity: 0,
    }];

    let result = run_fix_pass(
        &files,
        false,
        dir.path().to_str().unwrap(),
        false,
        &options(),
    );

    assert!(result.wrote);
    let fixed = fs::read_to_string(&file).expect("should read");
    assert!(fixed.contains("if a and b:"));
    assert!(fixed.contains("if c and d:"));
    assert!(result.console.contains("pass 2:"));
}

#[test]
fn nothing_fixable_reports_no_fixes() {
    let dir = tempdir().expect("tempdir should work");

    let result = run_fix_pass(&[], false, dir.path().to_str().unwrap(), false, &options());

    assert_eq!(result.console, "No fixes to apply.");
    assert!(!result.wrote);
}
