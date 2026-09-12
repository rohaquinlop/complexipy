use super::*;
use lsp_types::{Position, Range, TextDocumentIdentifier, WorkDoneProgressParams};

const SAMPLE: &str = "def light():\n    return 1\n\n\ndef heavy(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n";

#[test]
fn stale_results_are_never_published() {
    let (server_connection, _client) = Connection::memory();
    let mut server = Server::new(
        server_connection,
        Initialized {
            root: "/repo".to_string(),
            config: LspConfig::default(),
            refresh_support: false,
        },
    );
    let uri: Uri = "file:///repo/sample.py".parse().unwrap();
    server.documents.open(uri.clone(), SAMPLE.to_string(), 7);

    let published = server.publish(&uri, DocumentAnalysis::empty(6));

    assert!(!published);
    assert!(server.analyses.is_empty());
    assert!(
        server
            .connection
            .receiver
            .recv_timeout(Duration::from_millis(50))
            .is_err()
    );
}

#[test]
fn hint_requests_store_the_recomputed_analysis() {
    let (server_connection, _client) = Connection::memory();
    let mut server = Server::new(
        server_connection,
        Initialized {
            root: "/repo".to_string(),
            config: LspConfig {
                max_complexity_allowed: 2,
                ..LspConfig::default()
            },
            refresh_support: false,
        },
    );
    let uri: Uri = "file:///repo/sample.py".parse().unwrap();
    server.documents.open(uri.clone(), SAMPLE.to_string(), 3);

    server.respond_inlay_hints(
        RequestId::from(1),
        InlayHintParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, u32::MAX)),
            work_done_progress_params: WorkDoneProgressParams::default(),
        },
    );

    let stored = server
        .analyses
        .get(&Documents::key(&uri))
        .expect("expected the recomputed analysis to be stored");
    assert_eq!(stored.version, 3);
}
