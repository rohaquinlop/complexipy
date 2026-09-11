use std::thread::JoinHandle;
use std::time::Duration;

use complexipy_lsp::server::{
    DID_CHANGE, DID_CHANGE_CONFIGURATION, DID_CLOSE, DID_OPEN, EXIT, EXIT_CODE_CLEAN,
    EXIT_CODE_FAILURE, HOVER, INLAY_HINT, SERVER_NAME, SHUTDOWN, serve,
};
use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, Hover, HoverContents, HoverParams, InitializeParams,
    InitializedParams, InlayHint, InlayHintParams, Position, PublishDiagnosticsParams, Range,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem, Uri,
    VersionedTextDocumentIdentifier, WorkDoneProgressParams, WorkspaceFolder,
};
use tempfile::TempDir;

const SAMPLE: &str = "def light():\n    return 1\n\n\ndef heavy(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n";
const CLEAN: &str = "def light():\n    return 1\n";
const STRICT: &str = "max-complexity-allowed = 2\n";
const TIMEOUT: Duration = Duration::from_secs(10);

struct Session {
    client: Connection,
    server: JoinHandle<i32>,
    root: TempDir,
    next_id: i32,
}

fn start() -> Session {
    let (server, client) = Connection::memory();
    let server = std::thread::spawn(move || serve(server));

    Session {
        client,
        server,
        root: TempDir::new().unwrap(),
        next_id: 1,
    }
}

impl Session {
    fn write_config(&self, content: &str) {
        std::fs::write(self.root.path().join("complexipy.toml"), content).unwrap();
    }

    fn document_uri(&self, name: &str) -> Uri {
        format!("file://{}/{}", self.root.path().display(), name)
            .parse()
            .unwrap()
    }

    fn send(&self, message: impl Into<Message>) {
        self.client.sender.send(message.into()).unwrap();
    }

    fn next_id(&mut self) -> RequestId {
        let id = RequestId::from(self.next_id);
        self.next_id += 1;
        id
    }

    fn receive(&self) -> Message {
        self.client
            .receiver
            .recv_timeout(TIMEOUT)
            .expect("timed out waiting for a message")
    }

    fn initialize(&mut self) -> serde_json::Value {
        let id = self.next_id();
        let root: Uri = format!("file://{}", self.root.path().display())
            .parse()
            .unwrap();
        let params = InitializeParams {
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: root,
                name: "root".to_string(),
            }]),
            ..InitializeParams::default()
        };

        self.send(Request::new(id.clone(), "initialize".to_string(), params));
        let result = self.result(id);
        self.send(Notification::new(
            "initialized".to_string(),
            InitializedParams {},
        ));

        result
    }

    fn response(&mut self, id: RequestId) -> Result<serde_json::Value, lsp_server::ResponseError> {
        loop {
            if let Message::Response(response) = self.receive() {
                assert_eq!(response.id, id);
                return response.response_result;
            }
        }
    }

    fn result(&mut self, id: RequestId) -> serde_json::Value {
        self.response(id).expect("response carried an error")
    }

    fn publish(&mut self, id: RequestId, method: &str, params: impl serde::Serialize) {
        self.send(Request::new(id, method.to_string(), params));
    }

    fn request(&mut self, method: &str, params: impl serde::Serialize) -> serde_json::Value {
        let id = self.next_id();
        self.publish(id.clone(), method, params);
        self.result(id)
    }

    fn notify(&mut self, method: &str, params: impl serde::Serialize) {
        self.send(Notification::new(method.to_string(), params));
    }

    fn open(&mut self, uri: &Uri, text: &str) {
        self.notify(
            DID_OPEN,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "python".to_string(),
                    version: 1,
                    text: text.to_string(),
                },
            },
        );
    }

    fn change(&mut self, uri: &Uri, version: i32, text: &str) {
        self.notify(
            DID_CHANGE,
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: text.to_string(),
                }],
            },
        );
    }

    fn close(&mut self, uri: &Uri) {
        self.notify(
            DID_CLOSE,
            DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            },
        );
    }

    fn diagnostics(&mut self, uri: &Uri) -> Vec<Diagnostic> {
        loop {
            if let Message::Notification(notification) = self.receive()
                && notification.method == "textDocument/publishDiagnostics"
            {
                let params: PublishDiagnosticsParams =
                    serde_json::from_value(notification.params).unwrap();

                if &params.uri == uri {
                    return params.diagnostics;
                }
            }
        }
    }

    fn inlay_hints(&mut self, uri: &Uri) -> Vec<InlayHint> {
        let result = self.request(
            INLAY_HINT,
            InlayHintParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                range: Range::new(Position::new(0, 0), Position::new(u32::MAX, u32::MAX)),
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        );

        serde_json::from_value(result).unwrap()
    }

    fn hover(&mut self, uri: &Uri, line: u32) -> Option<Hover> {
        let result = self.request(
            HOVER,
            HoverParams {
                text_document_position_params: lsp_types::TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(line, 0),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        );

        serde_json::from_value(result).unwrap()
    }

    fn shutdown(mut self) -> i32 {
        let id = self.next_id();
        self.publish(id.clone(), SHUTDOWN, ());
        self.result(id);
        self.send(Notification::new(EXIT.to_string(), ()));

        self.server.join().unwrap()
    }

    fn join(self) -> i32 {
        self.server.join().unwrap()
    }
}

#[test]
fn advertises_full_sync_inlay_hints_and_hover() {
    let mut session = start();
    let result = session.initialize();
    let capabilities = &result["capabilities"];

    assert_eq!(capabilities["textDocumentSync"], serde_json::json!(1));
    assert_eq!(capabilities["inlayHintProvider"], serde_json::json!(true));
    assert_eq!(capabilities["hoverProvider"], serde_json::json!(true));
    assert!(capabilities.get("codeActionProvider").is_none());
    assert_eq!(result["serverInfo"]["name"], serde_json::json!(SERVER_NAME));

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn publishes_diagnostics_and_hints_above_the_limit() {
    let mut session = start();
    session.write_config(STRICT);
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);

    let diagnostics = session.diagnostics(&uri);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message,
        "cognitive complexity 3 exceeds the allowed 2"
    );
    assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::WARNING));
    assert_eq!(diagnostics[0].range.start, Position::new(4, 0));

    let hints = session.inlay_hints(&uri);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].position, Position::new(4, 16));

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn stays_quiet_with_the_default_limit() {
    let mut session = start();
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);

    assert!(session.diagnostics(&uri).is_empty());
    assert!(session.inlay_hints(&uri).is_empty());

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn hints_are_available_before_the_debounce_fires() {
    let mut session = start();
    session.write_config(STRICT);
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);

    let hints = session.inlay_hints(&uri);
    assert_eq!(hints.len(), 1);

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn clears_diagnostics_after_an_edit_removes_complexity() {
    let mut session = start();
    session.write_config(STRICT);
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);
    assert_eq!(session.diagnostics(&uri).len(), 1);

    session.change(&uri, 2, CLEAN);

    assert!(session.diagnostics(&uri).is_empty());
    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn clears_diagnostics_when_the_document_closes() {
    let mut session = start();
    session.write_config(STRICT);
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);
    assert_eq!(session.diagnostics(&uri).len(), 1);

    session.close(&uri);

    assert!(session.diagnostics(&uri).is_empty());
    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn excluded_documents_produce_no_output() {
    let mut session = start();
    session.write_config("max-complexity-allowed = 2\nexclude = [\"sample.py\"]\n");
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);

    assert!(session.diagnostics(&uri).is_empty());
    assert!(session.inlay_hints(&uri).is_empty());

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn reloads_config_on_request() {
    let mut session = start();
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);
    assert!(session.diagnostics(&uri).is_empty());

    session.write_config(STRICT);
    session.notify(DID_CHANGE_CONFIGURATION, serde_json::json!({}));

    assert_eq!(session.diagnostics(&uri).len(), 1);
    assert_eq!(session.inlay_hints(&uri).len(), 1);
    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn hover_reports_the_function_total() {
    let mut session = start();
    session.write_config(STRICT);
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);

    let hover = session.hover(&uri, 4).expect("expected a hover");
    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markup content");
    };

    assert!(content.value.contains("**heavy**: cognitive complexity 3"));
    assert!(session.hover(&uri, 2).is_none());

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn unknown_requests_get_an_error_response() {
    let mut session = start();
    session.initialize();
    let id = session.next_id();
    session.publish(id.clone(), "textDocument/codeAction", serde_json::json!({}));

    assert!(session.response(id).is_err());
    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn hints_for_unopened_documents_are_empty() {
    let mut session = start();
    session.initialize();
    let uri = session.document_uri("missing.py");

    assert!(session.inlay_hints(&uri).is_empty());
    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn exit_without_shutdown_is_a_failure() {
    let mut session = start();
    session.initialize();
    session.send(Notification::new(EXIT.to_string(), ()));

    assert_eq!(session.join(), EXIT_CODE_FAILURE);
}

#[test]
fn advertises_the_configured_server_version() {
    let mut session = start();
    let result = session.initialize();

    assert_eq!(
        result["serverInfo"]["version"],
        serde_json::json!(env!("CARGO_PKG_VERSION"))
    );

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}
