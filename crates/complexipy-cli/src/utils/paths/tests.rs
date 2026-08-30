use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use crate::types::OutputFormat;
use crate::utils::paths::{
    PathsError, build_output_paths, ensure_directory_destination, is_directory_output_hint,
    normalize_path, resolve_output_paths,
};

fn default_paths(destination: &Path) -> Vec<(OutputFormat, PathBuf)> {
    [
        OutputFormat::Csv,
        OutputFormat::Json,
        OutputFormat::Gitlab,
        OutputFormat::Sarif,
    ]
    .into_iter()
    .map(|format| {
        let filename = format.default_output_filename();
        (format, destination.join(filename))
    })
    .collect()
}

#[test]
fn empty_formats_returns_empty() {
    let dir = tempdir().expect("tempdir should work");

    let result = resolve_output_paths(&[], None, dir.path()).expect("should succeed");

    assert!(result.is_empty());
    assert_eq!(fs::read_dir(dir.path()).expect("should read").count(), 0);
}

#[test]
fn no_output_uses_invocation_path_with_default_filenames() {
    let dir = tempdir().expect("tempdir should work");

    let result =
        resolve_output_paths(&[OutputFormat::Csv], None, dir.path()).expect("should succeed");

    assert_eq!(
        result,
        vec![(
            OutputFormat::Csv,
            dir.path()
                .join("complexipy-results.csv")
                .to_string_lossy()
                .into_owned()
        )]
    );
}

#[test]
fn stdout_destination_rejected() {
    let dir = tempdir().expect("tempdir should work");

    let result = resolve_output_paths(&[OutputFormat::Csv], Some("-"), dir.path());

    assert_eq!(result, Err(PathsError::StdoutNotSupported));
}

#[test]
fn single_format_existing_directory() {
    let dir = tempdir().expect("tempdir should work");
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).expect("should create");

    let result = resolve_output_paths(
        &[OutputFormat::Csv],
        Some(sub.to_str().unwrap()),
        dir.path(),
    )
    .expect("should succeed");

    assert_eq!(
        result,
        vec![(
            OutputFormat::Csv,
            sub.join("complexipy-results.csv")
                .to_string_lossy()
                .into_owned()
        )]
    );
}

#[test]
fn single_format_trailing_separator_hint() {
    let dir = tempdir().expect("tempdir should work");
    let out = dir.path().join("out");
    let output = format!("{}{}", out.to_str().unwrap(), std::path::MAIN_SEPARATOR);

    let result = resolve_output_paths(&[OutputFormat::Json], Some(&output), dir.path())
        .expect("should succeed");

    assert!(out.is_dir());
    assert_eq!(
        result,
        vec![(
            OutputFormat::Json,
            out.join("complexipy-results.json")
                .to_string_lossy()
                .into_owned()
        )]
    );
}

#[test]
fn single_format_plain_file_path() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("results.csv");

    let result = resolve_output_paths(
        &[OutputFormat::Csv],
        Some(file.to_str().unwrap()),
        dir.path(),
    )
    .expect("should succeed");

    assert_eq!(
        result,
        vec![(OutputFormat::Csv, file.to_string_lossy().into_owned())]
    );
}

#[test]
fn single_format_creates_parent_directories() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("nested").join("deep").join("results.json");

    let result = resolve_output_paths(
        &[OutputFormat::Json],
        Some(file.to_str().unwrap()),
        dir.path(),
    )
    .expect("should succeed");

    assert!(file.parent().expect("parent").is_dir());
    assert_eq!(
        result,
        vec![(OutputFormat::Json, file.to_string_lossy().into_owned())]
    );
}

#[test]
fn relative_output_is_absolutized() {
    let dir = tempdir().expect("tempdir should work");
    let output = "rel-out/";

    let result = resolve_output_paths(&[OutputFormat::Csv], Some(output), dir.path())
        .expect("should succeed");

    let expected = std::env::current_dir()
        .expect("cwd")
        .join("rel-out")
        .join("complexipy-results.csv");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].1, expected.to_string_lossy().into_owned());
    assert!(expected.parent().expect("parent").is_dir());
}

#[test]
fn multi_format_existing_file_rejected() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("results.txt");
    fs::write(&file, "x").expect("should write");

    let result = resolve_output_paths(
        &[OutputFormat::Csv, OutputFormat::Json],
        Some(file.to_str().unwrap()),
        dir.path(),
    );

    assert_eq!(result, Err(PathsError::MultiFormatNotDirectory));
}

#[test]
fn multi_format_plain_path_rejected() {
    let dir = tempdir().expect("tempdir should work");
    let out = dir.path().join("out");

    let result = resolve_output_paths(
        &[OutputFormat::Csv, OutputFormat::Json],
        Some(out.to_str().unwrap()),
        dir.path(),
    );

    assert_eq!(result, Err(PathsError::MultiFormatNoTrailingSeparator));
}

#[test]
fn multi_format_trailing_separator_creates_directory() {
    let dir = tempdir().expect("tempdir should work");
    let out = dir.path().join("out");
    let output = format!("{}{}", out.to_str().unwrap(), std::path::MAIN_SEPARATOR);

    let result = resolve_output_paths(
        &[OutputFormat::Csv, OutputFormat::Json],
        Some(&output),
        dir.path(),
    )
    .expect("should succeed");

    assert!(out.is_dir());
    assert_eq!(
        result,
        vec![
            (
                OutputFormat::Csv,
                out.join("complexipy-results.csv")
                    .to_string_lossy()
                    .into_owned()
            ),
            (
                OutputFormat::Json,
                out.join("complexipy-results.json")
                    .to_string_lossy()
                    .into_owned()
            ),
        ]
    );
}

#[test]
fn multi_format_existing_directory_without_hint_rejected() {
    let dir = tempdir().expect("tempdir should work");
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).expect("should create");

    let result = resolve_output_paths(
        &[OutputFormat::Csv, OutputFormat::Json],
        Some(sub.to_str().unwrap()),
        dir.path(),
    );

    assert_eq!(result, Err(PathsError::MultiFormatNoTrailingSeparator));
}

#[test]
fn multi_format_existing_directory_with_hint() {
    let dir = tempdir().expect("tempdir should work");
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).expect("should create");
    let output = format!("{}{}", sub.to_str().unwrap(), std::path::MAIN_SEPARATOR);

    let result = resolve_output_paths(
        &[OutputFormat::Csv, OutputFormat::Sarif],
        Some(&output),
        dir.path(),
    )
    .expect("should succeed");

    assert_eq!(
        result,
        vec![
            (
                OutputFormat::Csv,
                sub.join("complexipy-results.csv")
                    .to_string_lossy()
                    .into_owned()
            ),
            (
                OutputFormat::Sarif,
                sub.join("complexipy-results.sarif")
                    .to_string_lossy()
                    .into_owned()
            ),
        ]
    );
}

#[test]
fn build_output_paths_uses_default_filenames() {
    let dir = tempdir().expect("tempdir should work");

    let result = build_output_paths(
        dir.path(),
        &[
            OutputFormat::Csv,
            OutputFormat::Json,
            OutputFormat::Gitlab,
            OutputFormat::Sarif,
        ],
    );

    assert_eq!(
        result,
        default_paths(dir.path())
            .into_iter()
            .map(|(format, path)| (format, path.to_string_lossy().into_owned()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn directory_hint_detects_trailing_separators() {
    assert!(is_directory_output_hint("out/"));
    assert!(!is_directory_output_hint("out"));
    assert!(!is_directory_output_hint(""));
}

#[test]
fn normalize_path_returns_cleaned_path_when_ends_with_file_name() {
    assert_eq!(normalize_path("src/a.py", "a.py"), "src/a.py");
    assert_eq!(normalize_path("src/a.py/", "a.py"), "src/a.py");
}

#[test]
fn normalize_path_joins_path_and_file_name() {
    assert_eq!(normalize_path("src", "a.py"), "src/a.py");
    assert_eq!(normalize_path("src/", "a.py"), "src/a.py");
}

#[test]
fn normalize_path_empty_path_returns_file_name() {
    assert_eq!(normalize_path("", "a.py"), "a.py");
    assert_eq!(normalize_path("/", "a.py"), "a.py");
}

#[test]
fn normalize_path_empty_file_name_returns_cleaned() {
    assert_eq!(normalize_path("src/a.py", ""), "src/a.py");
}

#[test]
fn normalize_path_strips_all_trailing_slashes() {
    assert_eq!(normalize_path("src///", "a.py"), "src/a.py");
}

#[test]
fn ensure_directory_destination_validates() {
    let dir = tempdir().expect("tempdir should work");
    let file = dir.path().join("results.txt");
    fs::write(&file, "x").expect("should write");

    assert_eq!(
        ensure_directory_destination(&file, false),
        Err(PathsError::MultiFormatNotDirectory)
    );
    assert_eq!(
        ensure_directory_destination(&dir.path().join("new"), false),
        Err(PathsError::MultiFormatNoTrailingSeparator)
    );
    assert!(ensure_directory_destination(&dir.path().join("new"), true).is_ok());
    assert!(dir.path().join("new").is_dir());
}
