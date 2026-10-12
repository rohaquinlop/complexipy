use std::fs;

use complexipy_core::classes::{FileComplexity, FunctionComplexity};
use complexipy_core::cognitive_complexity::code_complexity_shared;
use complexipy_core::fix::fixable;
use complexipy_core::{AnalysisOptions, RuleSet};
use tempfile::tempdir;

use super::{run_fix_pass, write_patched};

const TWO_SPOTS: &str = "def f(a, b, c, d):\n    if a:\n        if b:\n            return 1\n    if c:\n        if d:\n            return 2\n    return 0\n";

const MIXED: &str = "def one(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n\n\ndef two(a, b, c, d):\n    if (a and b) or (c and d):\n        return 2\n    return 0\n";

const LOW_AND_HIGH: &str = "def low(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n\n\ndef high(a, b, c):\n    for x in c:\n        if a:\n            if b:\n                return x\n    return 0\n";

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

#[cfg(unix)]
#[test]
fn write_patched_writes_through_a_symlink() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().expect("tempdir should work");
    let real = dir.path().join("real.py");
    fs::write(&real, "def a():\n    if x:\n        return 1\n").expect("should write");
    let link = dir.path().join("link.py");
    symlink(&real, &link).expect("symlink should work");

    write_patched(&link, "def a():\n    pass\n").expect("should patch");

    assert!(
        fs::symlink_metadata(&link)
            .expect("link exists")
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_to_string(&real).expect("should read"),
        "def a():\n    pass\n"
    );
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
            complexity: 6,
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
        0,
    );

    assert!(result.wrote);
    let fixed = fs::read_to_string(&file).expect("should read");
    assert!(fixed.contains("if a and b:"));
    assert!(fixed.contains("if c and d:"));
    assert!(result.console.contains("pass 2:"));
    assert!(result.console.contains("Fixed C007 at two.py:5-7"));
    assert!(!result.console.contains("two.py:4-6"));
}

#[test]
fn nothing_fixable_reports_no_fixes() {
    let dir = tempdir().expect("tempdir should work");

    let result = run_fix_pass(
        &[],
        false,
        dir.path().to_str().unwrap(),
        false,
        &options(),
        0,
    );

    assert_eq!(result.console, "No fixes to apply.");
    assert!(!result.wrote);
}

#[test]
fn non_fixable_plans_are_reported_once_each() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("mixed.py");
    fs::write(&file, MIXED).expect("should write");

    let complexity = code_complexity_shared(MIXED, &options()).expect("should analyze");
    let expected_skips = complexity
        .functions
        .iter()
        .flat_map(|function| function.refactor_plans.iter())
        .filter(|plan| !fixable(plan))
        .count();
    let files = vec![FileComplexity {
        path: "mixed.py".to_string(),
        file_name: "mixed.py".to_string(),
        functions: complexity.functions,
        complexity: 0,
    }];

    let result = run_fix_pass(
        &files,
        false,
        dir.path().to_str().unwrap(),
        false,
        &options(),
        0,
    );

    assert!(result.wrote);
    let fixed = fs::read_to_string(&file).expect("should read");
    assert!(fixed.contains("if a and b:"));
    assert!(expected_skips > 0);
    assert_eq!(result.console.matches("Fixed C007").count(), 1);
    assert_eq!(
        result.console.matches("not safe to auto-apply").count(),
        expected_skips
    );
    assert!(result.console.contains("Skipped C005 at mixed.py:9-9"));
    assert!(!result.console.contains("mixed.py:8-8"));
}

fn fix_source(source: &str, max_complexity_allowed: u64, dry_run: bool) -> (String, String) {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("code.py");
    fs::write(&file, source).expect("should write");
    let complexity = code_complexity_shared(source, &options()).expect("should analyze");
    let files = vec![FileComplexity {
        path: "code.py".to_string(),
        file_name: "code.py".to_string(),
        functions: complexity.functions,
        complexity: 0,
    }];

    let result = run_fix_pass(
        &files,
        dry_run,
        dir.path().to_str().unwrap(),
        false,
        &options(),
        max_complexity_allowed,
    );

    (
        fs::read_to_string(&file).expect("should read"),
        result.console,
    )
}

fn function_text<'a>(source: &'a str, name: &str) -> &'a str {
    let start = source
        .find(&format!("def {}(", name))
        .expect("function exists");
    let end = source[start + 1..]
        .find("\ndef ")
        .map_or(source.len(), |offset| start + 1 + offset);
    &source[start..end]
}

#[test]
fn functions_at_or_under_the_threshold_are_not_fixed() {
    let (fixed, console) = fix_source(LOW_AND_HIGH, 6, false);

    assert_eq!(fixed, LOW_AND_HIGH);
    assert_eq!(console, "No fixes to apply.");
}

#[test]
fn only_functions_over_the_threshold_are_fixed() {
    let (fixed, _) = fix_source(LOW_AND_HIGH, 3, false);

    assert_eq!(
        function_text(&fixed, "low"),
        function_text(LOW_AND_HIGH, "low")
    );
    assert_ne!(
        function_text(&fixed, "high"),
        function_text(LOW_AND_HIGH, "high")
    );
}

#[test]
fn every_function_over_the_threshold_is_fixed() {
    let (fixed, _) = fix_source(LOW_AND_HIGH, 2, false);

    assert_ne!(
        function_text(&fixed, "low"),
        function_text(LOW_AND_HIGH, "low")
    );
    assert_ne!(
        function_text(&fixed, "high"),
        function_text(LOW_AND_HIGH, "high")
    );
}

#[test]
fn a_function_that_drops_to_the_threshold_stops_changing() {
    let source = format!(
        "def many(a, b):\n{}    return 0\n",
        "    if a:\n        if b:\n            return 1\n".repeat(6)
    );
    let threshold = 13;

    let (fixed, console) = fix_source(&source, threshold, false);

    let after = code_complexity_shared(&fixed, &options()).expect("should analyze");
    assert_eq!(after.functions[0].complexity, threshold);
    assert_eq!(console.matches("Fixed C007").count(), 5);
    assert!(!console.contains("pass 2:"));
}

#[test]
fn dry_run_previews_only_functions_over_the_threshold() {
    let (untouched, preview) = fix_source(LOW_AND_HIGH, 3, true);
    let (_, applied) = fix_source(LOW_AND_HIGH, 3, false);

    assert_eq!(untouched, LOW_AND_HIGH);
    assert_eq!(preview.matches("@@ -").count(), 1);
    assert!(preview.contains("@@ -9,5 +9,4 @@"));
    assert!(applied.starts_with(&preview));
}
