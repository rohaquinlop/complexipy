use super::*;
use crate::documents::path_to_uri;
use crate::workspace::Folder;
use complexipy_core::config::LspConfig;
use lsp_types::{
    Position, PublishDiagnosticsParams, Range, TextDocumentIdentifier, WorkDoneProgressParams,
};

const SAMPLE: &str = "def light():\n    return 1\n\n\ndef heavy(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n";
const BROKEN: &str = "def broken(:\n";

fn server(config: LspConfig) -> (Server, Connection) {
    server_at("/repo", config, None)
}

fn server_at(root: &str, config: LspConfig, gate: Option<Receiver<()>>) -> (Server, Connection) {
    let (connection, client) = Connection::memory();
    let server = Server::with_scan_gate(
        connection,
        Initialized {
            workspace: Workspace::new(
                vec![Folder::new(root.to_string(), config)],
                root.to_string(),
            ),
            refresh_support: false,
            watch_support: false,
        },
        gate,
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

fn hints_request(id: i32) -> Request {
    Request::new(RequestId::from(id), INLAY_HINT.to_string(), hints_params())
}

fn hover_request(id: i32, line: u32) -> Request {
    Request::new(
        RequestId::from(id),
        HOVER.to_string(),
        HoverParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri() },
                position: Position::new(line, 0),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        },
    )
}

/// Feeds the analysis thread's results back until no work is left, as the
/// message loop does.
fn settle(server: &mut Server) {
    while !server.analyzing.is_empty() || !server.planning.is_empty() {
        let outcome = server
            .outcomes
            .recv_timeout(Duration::from_secs(10))
            .expect("expected the analysis thread to answer");
        server.accept(outcome);
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

fn responses_of(client: &Connection) -> Vec<Response> {
    drain(client)
        .into_iter()
        .filter_map(|message| match message {
            Message::Response(response) => Some(response),
            _ => None,
        })
        .collect()
}

fn stored_version(server: &Server) -> Option<i32> {
    server
        .analyses
        .get(&Documents::key(&uri()))
        .map(|analysis| analysis.version)
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
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());

    server.handle_request(hints_request(1));
    settle(&mut server);

    assert_eq!(stored_version(&server), Some(3));
    let responses = responses_of(&client);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].id, RequestId::from(1));
}

#[test]
fn a_failing_version_keeps_the_last_successful_analysis() {
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());
    server.refresh(&uri());
    settle(&mut server);
    assert_eq!(diagnostics_of(&client).len(), 1);

    server.documents.change(&uri(), BROKEN.to_string(), 4);
    assert!(!server.refresh(&uri()));
    settle(&mut server);

    let analysis = server
        .analyses
        .get(&Documents::key(&uri()))
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
    settle(&mut server);
    assert!(!server.refresh(&uri()));
    settle(&mut server);

    assert_eq!(diagnostics_of(&client).len(), 1);
}

#[test]
fn a_hint_request_on_a_failing_version_publishes_the_parse_diagnostic() {
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());
    server.refresh(&uri());
    settle(&mut server);
    assert_eq!(diagnostics_of(&client).len(), 1);

    server.documents.change(&uri(), BROKEN.to_string(), 4);
    server.handle_request(hints_request(1));
    settle(&mut server);

    let diagnostics = diagnostics_of(&client);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(lsp_types::NumberOrString::String(
            crate::analysis::PARSE_ERROR_CODE.to_string()
        ))
    );

    assert!(!server.refresh(&uri()));
    settle(&mut server);
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
    settle(&mut server);
    assert_eq!(diagnostics_of(&client).len(), 1);

    server.documents.change(&uri(), SAMPLE.to_string(), 5);
    server.refresh(&uri());
    settle(&mut server);

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
    server.handle_request(hints_request(1));
    settle(&mut server);

    assert!(server.refresh(&uri()));
    assert!(server.analyzing.is_empty());
    assert_eq!(diagnostics_of(&client).len(), 1);
}

#[test]
fn other_languages_are_not_analyzed() {
    let (mut server, _client) = server(strict());
    let json: Uri = "file:///repo/data.json".parse().unwrap();
    server
        .documents
        .open(json.clone(), SAMPLE.to_string(), 1, "json".to_string());

    server.dispatch(
        RequestId::from(1),
        PendingRequest::Hints {
            uri: json.clone(),
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, u32::MAX)),
        },
    );

    assert!(server.analyzing.is_empty());
    let analysis = server
        .analyses
        .get(&Documents::key(&json))
        .expect("expected an empty analysis");
    assert!(analysis.functions.is_empty());
    assert!(server.failures.is_empty());
}

#[test]
fn hover_on_a_failing_version_builds_plans_from_the_last_good_text() {
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());
    server.refresh(&uri());
    settle(&mut server);
    server.documents.change(&uri(), BROKEN.to_string(), 4);
    drain(&client);

    server.handle_request(hover_request(1, 4));
    settle(&mut server);

    let hover = responses_of(&client)
        .into_iter()
        .find_map(|response| response.response_result.ok())
        .expect("expected a hover response");
    assert!(hover.to_string().contains("Top refactor (`C007`)"));
}

#[test]
fn a_cancelled_request_is_answered_as_cancelled() {
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());

    server.handle_request(hints_request(7));
    server.handle_notification(Notification::new(
        CANCEL_REQUEST.to_string(),
        CancelParams {
            id: NumberOrString::Number(7),
        },
    ));
    settle(&mut server);

    let responses = responses_of(&client);
    assert_eq!(responses.len(), 1);
    assert_eq!(
        responses[0]
            .response_result
            .as_ref()
            .err()
            .map(|error| error.code),
        Some(ErrorCode::RequestCanceled as i32)
    );
}

#[test]
fn an_edit_to_one_document_keeps_the_deadline_of_another() {
    let (mut server, _client) = server(strict());
    let other: Uri = "file:///repo/other.py".parse().unwrap();
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 1, "python".to_string());
    server
        .documents
        .open(other.clone(), SAMPLE.to_string(), 1, "python".to_string());

    server.mark_dirty(uri());
    let first = server.deadlines[&Documents::key(&uri())];
    std::thread::sleep(Duration::from_millis(5));
    server.mark_dirty(other.clone());

    assert_eq!(server.deadlines[&Documents::key(&uri())], first);
    assert!(server.deadlines[&Documents::key(&other)] > first);
}

#[test]
fn shutdown_answers_the_requests_still_waiting() {
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());
    server.handle_request(hints_request(1));

    let (exit_sender, exit_receiver) = crossbeam_channel::bounded(1);
    let client_sender = client.sender.clone();
    std::thread::spawn(move || {
        let code = server.shutdown(Request::new(RequestId::from(2), SHUTDOWN.to_string(), ()));
        let _ = exit_sender.send(code);
    });
    client_sender
        .send(Notification::new(EXIT.to_string(), ()).into())
        .unwrap();

    assert_eq!(
        exit_receiver.recv_timeout(Duration::from_secs(10)),
        Ok(EXIT_CODE_CLEAN)
    );
    let ids: Vec<RequestId> = responses_of(&client)
        .into_iter()
        .map(|response| response.id)
        .collect();
    assert_eq!(ids, vec![RequestId::from(1), RequestId::from(2)]);
}

#[test]
fn a_flush_keeps_one_analysis_queued_so_a_request_goes_next() {
    let (mut server, client) = server(strict());
    let uris: Vec<Uri> = (0..3)
        .map(|index| format!("file:///repo/doc{index}.py").parse().unwrap())
        .collect();
    for uri in &uris {
        server
            .documents
            .open(uri.clone(), SAMPLE.to_string(), 1, "python".to_string());
        server.mark_dirty(uri.clone());
    }
    for deadline in server.deadlines.values_mut() {
        *deadline = Instant::now();
    }

    server.flush_due();
    assert_eq!(server.analyzing.len(), 1);
    assert_eq!(server.flushing.len(), 2);

    server.handle_request(Request::new(
        RequestId::from(1),
        INLAY_HINT.to_string(),
        InlayHintParams {
            text_document: TextDocumentIdentifier {
                uri: uris[2].clone(),
            },
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, u32::MAX)),
            work_done_progress_params: WorkDoneProgressParams::default(),
        },
    ));
    assert_eq!(server.analyzing.len(), 2);

    settle(&mut server);
    assert!(server.flushing.is_empty());
    assert_eq!(diagnostics_of(&client).len(), 3);
}

#[test]
fn concurrent_requests_share_one_analysis() {
    let (mut server, client) = server(strict());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 3, "python".to_string());

    server.handle_request(hints_request(1));
    server.handle_request(hover_request(2, 4));
    assert_eq!(server.analyzing.len(), 1);
    settle(&mut server);

    let mut ids: Vec<RequestId> = responses_of(&client)
        .into_iter()
        .map(|response| response.id)
        .collect();
    ids.sort_by_key(|id| id.to_string());
    assert_eq!(ids, vec![RequestId::from(1), RequestId::from(2)]);
}

fn scanning() -> LspConfig {
    let mut config = strict();
    config.lsp.workspace_diagnostics = true;
    config
}

fn warnings() -> Vec<Diagnostic> {
    analysis::analyze(SAMPLE, 0, &strict())
        .unwrap()
        .diagnostics(&strict())
}

fn scanned(generation: u64, path: &str) -> ScanOutcome {
    ScanOutcome::Scanned {
        generation,
        path: path.to_string(),
        uri: path_to_uri(path).unwrap(),
        diagnostics: warnings(),
        force: false,
    }
}

fn published(client: &Connection) -> Vec<PublishDiagnosticsParams> {
    drain(client)
        .into_iter()
        .filter_map(|message| match message {
            Message::Notification(notification) => serde_json::from_value(notification.params).ok(),
            _ => None,
        })
        .collect()
}

#[test]
fn scan_results_of_an_older_generation_are_dropped() {
    let (mut server, client) = server(scanning());
    server.scan_generation.store(2, Ordering::SeqCst);

    server.accept_scan(scanned(1, "/repo/a.py"));
    assert!(published(&client).is_empty());

    server.accept_scan(scanned(2, "/repo/a.py"));
    assert_eq!(published(&client)[0].diagnostics, warnings());
}

#[test]
fn an_open_document_keeps_its_own_diagnostics() {
    let (mut server, client) = server(scanning());
    let open: Uri = "file:///repo/a%20b.py".parse().unwrap();
    server
        .documents
        .open(open, SAMPLE.to_string(), 1, "python".to_string());

    server.accept_scan(scanned(0, "/repo/a b.py"));

    assert!(published(&client).is_empty());
    assert!(server.scanned.is_empty());
}

#[test]
fn turning_the_scan_off_clears_what_it_published() {
    let (mut server, client) = server(strict());
    server.accept_scan(scanned(0, "/repo/a.py"));
    drain(&client);

    server.start_scan();
    let done = server
        .scan_outcomes
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    server.accept_scan(done);

    let cleared = published(&client);
    assert_eq!(cleared.len(), 1);
    assert_eq!(cleared[0].uri.as_str(), "file:///repo/a.py");
    assert!(cleared[0].diagnostics.is_empty());
    assert!(server.scanned.is_empty());
}

#[test]
fn closing_a_document_clears_it_when_its_folder_does_not_scan() {
    let (mut server, client) = server(strict());
    server.scannable.insert("/repo/sample.py".to_string());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 1, "python".to_string());

    server.close_document(DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri() },
    });

    let cleared = published(&client);
    assert_eq!(cleared.len(), 1);
    assert!(cleared[0].diagnostics.is_empty());
}

#[test]
fn created_files_start_one_scan_after_the_debounce() {
    let (mut server, _client) = server(scanning());
    let changes = (0..3)
        .map(|index| {
            lsp_types::FileEvent::new(
                format!("file:///repo/new{}.py", index).parse().unwrap(),
                FileChangeType::CREATED,
            )
        })
        .collect();

    server.change_watched_files(DidChangeWatchedFilesParams { changes });
    assert_eq!(server.scan_generation.load(Ordering::SeqCst), 0);
    assert!(server.scan_due.is_some());

    server.scan_due = Some(Instant::now());
    server.scan_when_due();
    assert_eq!(server.scan_generation.load(Ordering::SeqCst), 1);
    assert!(server.scan_due.is_none());
}

#[test]
fn hover_answers_while_a_scan_is_held() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("heavy.py"), SAMPLE).unwrap();
    let root = dir.path().to_string_lossy().replace('\\', "/");
    let (gate, held) = crossbeam_channel::unbounded();
    let (mut server, client) = server_at(&root, scanning(), Some(held));
    let open = path_to_uri(&format!("{}/sample.py", root)).unwrap();
    server
        .documents
        .open(open.clone(), SAMPLE.to_string(), 1, "python".to_string());

    server.start_scan();
    server.handle_request(Request::new(
        RequestId::from(1),
        HOVER.to_string(),
        HoverParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: open },
                position: Position::new(4, 0),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        },
    ));
    settle(&mut server);

    let responses = responses_of(&client);
    assert_eq!(responses.len(), 1);
    assert!(
        responses[0]
            .response_result
            .as_ref()
            .is_ok_and(|hover| !hover.is_null())
    );
    drop(gate);
}

#[test]
fn a_closed_document_is_cleared_by_a_later_scan() {
    let (mut server, client) = server(scanning());
    server.scannable.insert("/repo/sample.py".to_string());
    server
        .documents
        .open(uri(), SAMPLE.to_string(), 1, "python".to_string());
    server.close_document(DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri() },
    });
    server.scan_generation.store(5, Ordering::SeqCst);

    server.accept_scan(ScanOutcome::Scanned {
        generation: 5,
        path: "/repo/sample.py".to_string(),
        uri: uri(),
        diagnostics: Vec::new(),
        force: false,
    });

    let cleared = published(&client);
    assert_eq!(cleared.len(), 1);
    assert_eq!(cleared[0].uri, uri());
    assert!(cleared[0].diagnostics.is_empty());
}

#[test]
fn scan_results_keep_the_uri_the_client_closed() {
    let (mut server, client) = server(scanning());
    let closed: Uri = "file:///repo/a%2Epy".parse().unwrap();
    server.scannable.insert("/repo/a.py".to_string());
    server
        .documents
        .open(closed.clone(), SAMPLE.to_string(), 1, "python".to_string());
    server.close_document(DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier {
            uri: closed.clone(),
        },
    });
    server.scan_generation.store(5, Ordering::SeqCst);

    server.accept_scan(scanned(5, "/repo/a.py"));
    server.accept_scan(ScanOutcome::Scanned {
        generation: 5,
        path: "/repo/a.py".to_string(),
        uri: path_to_uri("/repo/a.py").unwrap(),
        diagnostics: Vec::new(),
        force: false,
    });

    let uris: Vec<Uri> = published(&client)
        .into_iter()
        .map(|params| params.uri)
        .collect();
    assert_eq!(uris, vec![closed.clone(), closed]);
}
