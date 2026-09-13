use super::*;
use lsp_types::{
    Position, PublishDiagnosticsParams, Range, TextDocumentIdentifier, WorkDoneProgressParams,
};

const SAMPLE: &str = "def light():\n    return 1\n\n\ndef heavy(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n";
const BROKEN: &str = "def broken(:\n";

fn server(config: LspConfig) -> (Server, Connection) {
    let (connection, client) = Connection::memory();
    let server = Server::new(
        connection,
        Initialized {
            root: "/repo".to_string(),
            config,
            refresh_support: false,
        },
    );

    (server, client)
}

fn strict() -> LspConfig {
    LspConfig {
        max_complexity_allowed: 2,
        ..LspConfig::default()
    }
}

fn uri() -> Uri {
    "file:///repo/sample.py".parse().unwrap()
}

fn hints_params() -> InlayHintParams {
    InlayHintParams {
        text_document: TextDocumentIdentifier { uri: uri() },
        range: Range::new(Position::new(0, 0), Position::new(u32::MAX, u32::MAX)),
        work_done_progress_params: WorkDoneProgressParams::default(),
    }
}

fn drain(client: &Connection) -> Vec<Message> {
    let mut messages = Vec::new();

    while let Ok(message) = client.receiver.recv_timeout(Duration::from_millis(20)) {
        messages.push(message);
    }

    messages
}

fn diagnostics_of(client: &Connection) -> Vec<Diagnostic> {
    drain(client)
        .into_iter()
        .filter_map(|message| match message {
            Message::Notification(notification) => {
                serde_json::from_value::<PublishDiagnosticsParams>(notification.params).ok()
            }
            _ => None,
        })
        .flat_map(|params| params.diagnostics)
        .collect()
}

#[test]
fn stale_results_are_never_published() {
    let (mut server, client) = server(LspConfig::default());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 7, "python".to_string());

    let published = server.publish(&uri(), DocumentAnalysis::empty(6));

    assert!(!published);
    assert!(server.analyses.is_empty());
    assert!(diagnostics_of(&client).is_empty());
}

#[test]
fn hint_requests_store_the_recomputed_analysis() {
    let (mut server, _client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());

    server.respond_inlay_hints(RequestId::from(1), hints_params());

    let stored = server
        .analyses
        .get(&Documents::key(&uri()))
        .expect("expected the recomputed analysis to be stored");
    assert_eq!(stored.version, 3);
}

#[test]
fn a_failing_version_keeps_the_last_successful_analysis() {
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());
    assert!(server.refresh(&uri()));
    assert_eq!(diagnostics_of(&client).len(), 1);

    server.documents.change(&uri(), BROKEN.to_string(), 4);
    assert!(!server.refresh(&uri()));

    let analysis = server
        .current_analysis(&uri())
        .expect("expected the last successful analysis");
    assert_eq!(analysis.version, 3);
    assert_eq!(analysis.functions.len(), 2);

    let failure = server
        .failures
        .get(&Documents::key(&uri()))
        .expect("expected the failure to be recorded");
    assert_eq!(failure.version, 4);

    let diagnostics = diagnostics_of(&client);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(lsp_types::NumberOrString::String(
            crate::analysis::PARSE_ERROR_CODE.to_string()
        ))
    );
}

#[test]
fn a_second_refresh_of_a_failing_version_publishes_nothing() {
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());
    server.documents.change(&uri(), BROKEN.to_string(), 4);
    assert!(!server.refresh(&uri()));
    assert!(!server.refresh(&uri()));

    assert_eq!(diagnostics_of(&client).len(), 1);
}

#[test]
fn a_hint_request_on_a_failing_version_publishes_the_parse_diagnostic() {
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());
    server.refresh(&uri());
    assert_eq!(diagnostics_of(&client).len(), 1);

    server.documents.change(&uri(), BROKEN.to_string(), 4);
    server.respond_inlay_hints(RequestId::from(1), hints_params());

    let diagnostics = diagnostics_of(&client);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(lsp_types::NumberOrString::String(
            crate::analysis::PARSE_ERROR_CODE.to_string()
        ))
    );

    assert!(!server.refresh(&uri()));
    assert!(diagnostics_of(&client).is_empty());
}

#[test]
fn a_successful_parse_clears_the_failure() {
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());
    server.documents.change(&uri(), BROKEN.to_string(), 4);
    server.refresh(&uri());
    assert_eq!(diagnostics_of(&client).len(), 1);

    server.documents.change(&uri(), SAMPLE.to_string(), 5);
    assert!(server.refresh(&uri()));

    assert!(server.failures.is_empty());

    let diagnostics = diagnostics_of(&client);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(lsp_types::NumberOrString::String(
            crate::analysis::DIAGNOSTIC_CODE.to_string()
        ))
    );
}

#[test]
fn a_fresh_cached_analysis_is_reused_without_a_second_parse() {
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());
    server.respond_inlay_hints(RequestId::from(1), hints_params());

    assert!(server.refresh(&uri()));
    assert_eq!(diagnostics_of(&client).len(), 1);
}

#[test]
fn other_languages_are_not_analyzed() {
    let (mut server, _client) = server(strict());
    let json: Uri = "file:///repo/data.json".parse().unwrap();
    server
        .documents
        .open(json.clone(), SAMPLE.to_string(), 1, "json".to_string());

    let analysis = server
        .current_analysis(&json)
        .expect("expected an empty analysis");

    assert!(analysis.functions.is_empty());
    assert!(server.failures.is_empty());
}
