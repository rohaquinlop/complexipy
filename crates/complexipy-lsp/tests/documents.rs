use complexipy_lsp::documents::{Documents, uri_to_path};
use lsp_types::Uri;

fn uri(value: &str) -> Uri {
    value.parse().unwrap()
}

#[test]
fn keeps_text_and_version() {
    let mut documents = Documents::default();
    let target = uri("file:///repo/a.py");

    documents.open(target.clone(), "first".to_string(), 1);
    documents.change(&target, "second".to_string(), 2);

    let document = documents.get(&target).unwrap();
    assert_eq!(document.text, "second");
    assert_eq!(document.version, 2);
}

#[test]
fn ignores_changes_to_unopened_documents() {
    let mut documents = Documents::default();

    documents.change(&uri("file:///repo/a.py"), "text".to_string(), 1);

    assert!(documents.get(&uri("file:///repo/a.py")).is_none());
}

#[test]
fn close_removes_the_document() {
    let mut documents = Documents::default();
    let target = uri("file:///repo/a.py");

    documents.open(target.clone(), "text".to_string(), 1);
    documents.close(&target);

    assert!(documents.get(&target).is_none());
    assert!(documents.keys().is_empty());
}

#[test]
fn keys_track_open_documents() {
    let mut documents = Documents::default();

    documents.open(uri("file:///repo/a.py"), "a".to_string(), 1);
    documents.open(uri("file:///repo/b.py"), "b".to_string(), 1);

    let mut keys = documents.keys();
    keys.sort();
    assert_eq!(keys, vec!["file:///repo/a.py", "file:///repo/b.py"]);
}

#[test]
fn decodes_file_urls() {
    assert_eq!(
        uri_to_path(&uri("file:///tmp/project/sample.py")).as_deref(),
        Some("/tmp/project/sample.py")
    );
}

#[test]
fn decodes_percent_escapes() {
    assert_eq!(
        uri_to_path(&uri("file:///tmp/my%20project/a%2Bb.py")).as_deref(),
        Some("/tmp/my project/a+b.py")
    );
}

#[test]
fn decodes_windows_drive_paths() {
    assert_eq!(
        uri_to_path(&uri("file:///C:/repo/a.py")).as_deref(),
        Some("C:/repo/a.py")
    );
}

#[test]
fn strips_query_and_fragment() {
    assert_eq!(
        uri_to_path(&uri("file:///tmp/a.py?version=1#frag")).as_deref(),
        Some("/tmp/a.py")
    );
}

#[test]
fn rejects_non_file_schemes() {
    assert!(uri_to_path(&uri("https://example.com/a.py")).is_none());
}

#[test]
fn decodes_utf8_escapes() {
    assert_eq!(
        uri_to_path(&uri("file:///tmp/caf%C3%A9.py")).as_deref(),
        Some("/tmp/caf\u{e9}.py")
    );
}
