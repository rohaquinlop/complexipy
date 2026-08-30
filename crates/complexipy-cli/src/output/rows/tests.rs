use std::collections::HashMap;

use crate::output::rows::{
    build_output_rows, has_success_functions, is_function_passing, sort_functions, truncate_top_n,
};
use crate::types::Sort;
use complexipy_core::classes::{FileComplexity, FunctionComplexity};

fn function(name: &str, complexity: u64) -> FunctionComplexity {
    FunctionComplexity {
        name: name.to_string(),
        complexity,
        line_start: 1,
        line_end: 2,
        line_complexities: vec![],
        refactor_plans: vec![],
        additional_refactor_plans: 0,
    }
}

fn file(path: &str, file_name: &str, functions: Vec<FunctionComplexity>) -> FileComplexity {
    FileComplexity {
        path: path.to_string(),
        file_name: file_name.to_string(),
        functions,
        complexity: 0,
    }
}

fn snapshot_map(entries: &[(&str, &str, &str, u64)]) -> HashMap<(String, String, String), u64> {
    entries
        .iter()
        .map(|(path, file_name, name, complexity)| {
            (
                (path.to_string(), file_name.to_string(), name.to_string()),
                *complexity,
            )
        })
        .collect()
}

#[test]
fn sort_asc_by_complexity_stable() {
    let functions = vec![
        function("high", 9),
        function("low", 1),
        function("tie_a", 5),
        function("tie_b", 5),
    ];

    let sorted = sort_functions(&functions, &Sort::Asc);

    let names: Vec<&str> = sorted.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["low", "tie_a", "tie_b", "high"]);
}

#[test]
fn sort_desc_by_complexity() {
    let functions = vec![function("low", 1), function("high", 9)];

    let sorted = sort_functions(&functions, &Sort::Desc);

    let names: Vec<&str> = sorted.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["high", "low"]);
}

#[test]
fn sort_file_name_by_lowercased_name() {
    let functions = vec![
        function("Zebra", 1),
        function("apple", 9),
        function("mango", 5),
    ];

    let sorted = sort_functions(&functions, &Sort::FileName);

    let names: Vec<&str> = sorted.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["apple", "mango", "Zebra"]);
}

#[test]
fn passing_within_threshold() {
    let f = function("f", 5);
    assert!(is_function_passing(&f, "a.py", "a.py", 5, None));
    assert!(is_function_passing(&f, "a.py", "a.py", 10, None));
}

#[test]
fn passing_above_threshold_without_snapshot_fails() {
    let f = function("f", 10);
    assert!(!is_function_passing(&f, "a.py", "a.py", 5, None));
}

#[test]
fn passing_with_snapshot_allows_same_or_better() {
    let f = function("f", 10);
    let map = snapshot_map(&[("a.py", "a.py", "f", 12)]);
    assert!(is_function_passing(&f, "a.py", "a.py", 5, Some(&map)));

    let map = snapshot_map(&[("a.py", "a.py", "f", 10)]);
    assert!(is_function_passing(&f, "a.py", "a.py", 5, Some(&map)));

    let map = snapshot_map(&[("a.py", "a.py", "f", 8)]);
    assert!(!is_function_passing(&f, "a.py", "a.py", 5, Some(&map)));
}

#[test]
fn build_rows_filters_failed_only_and_tracks_totals() {
    let files = vec![
        file(
            "src/a.py",
            "a.py",
            vec![
                function("pass", 2),
                function("fail", 10),
                function("border", 5),
            ],
        ),
        file("src/b.py", "b.py", vec![function("other", 1)]),
    ];

    let (entries, total, all_pass) = build_output_rows(&files, false, Sort::Asc, 5, None);

    assert_eq!(total, 4);
    assert!(!all_pass);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].functions.len(), 3);
    assert_eq!(entries[0].path, "src/a.py");

    let (entries, _, _) = build_output_rows(&files, true, Sort::Asc, 5, None);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].functions.len(), 1);
    assert_eq!(entries[0].functions[0].name, "fail");
}

#[test]
fn build_rows_all_pass_when_within_threshold() {
    let files = vec![file("src/a.py", "a.py", vec![function("f", 2)])];

    let (_, _, all_pass) = build_output_rows(&files, false, Sort::Asc, 5, None);

    assert!(all_pass);
}

#[test]
fn truncate_top_n_groups_by_path() {
    let entries = vec![
        crate::types::FileEntry {
            path: "a.py".to_string(),
            functions: vec![
                crate::types::FunctionRow {
                    name: "low".to_string(),
                    complexity: 1,
                    passed: true,
                    path: "a.py".to_string(),
                    file_name: "a.py".to_string(),
                    refactor_plans: vec![],
                    additional_refactor_plans: 0,
                },
                crate::types::FunctionRow {
                    name: "high".to_string(),
                    complexity: 9,
                    passed: false,
                    path: "a.py".to_string(),
                    file_name: "a.py".to_string(),
                    refactor_plans: vec![],
                    additional_refactor_plans: 0,
                },
            ],
        },
        crate::types::FileEntry {
            path: "b.py".to_string(),
            functions: vec![crate::types::FunctionRow {
                name: "mid".to_string(),
                complexity: 5,
                passed: true,
                path: "b.py".to_string(),
                file_name: "b.py".to_string(),
                refactor_plans: vec![],
                additional_refactor_plans: 0,
            }],
        },
    ];

    let truncated = truncate_top_n(entries, 2);

    assert_eq!(truncated.len(), 2);
    assert_eq!(truncated[0].path, "a.py");
    assert_eq!(truncated[0].functions[0].name, "high");
    assert_eq!(truncated[1].path, "b.py");
}

#[test]
fn has_success_functions_respects_snapshot() {
    let files = vec![file("a.py", "a.py", vec![function("f", 10)])];
    assert!(!has_success_functions(&files, 5, None));

    let map = snapshot_map(&[("a.py", "a.py", "f", 12)]);
    assert!(has_success_functions(&files, 5, Some(&map)));
}
