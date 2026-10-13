//! End-to-end tests of the server over an in-memory LSP connection.

use std::thread::JoinHandle;
use std::time::Duration;

use complexipy_lsp::server::{
    DID_CHANGE, DID_CHANGE_CONFIGURATION, DID_CHANGE_WATCHED_FILES, DID_CHANGE_WORKSPACE_FOLDERS,
    DID_CLOSE, DID_OPEN, EXIT, EXIT_CODE_CLEAN, EXIT_CODE_FAILURE, HOVER, INLAY_HINT,
    REGISTER_CAPABILITY, SERVER_NAME, SHOW_MESSAGE, SHUTDOWN, serve,
};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId};
use lsp_types::{
    ClientCapabilities, Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidChangeWatchedFilesClientCapabilities, DidChangeWatchedFilesParams,
    DidChangeWatchedFilesRegistrationOptions, DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, FileChangeType, FileEvent, Hover,
    HoverContents, HoverParams, InitializeParams, InitializedParams, InlayHint, InlayHintParams,
    MessageType, Position, PublishDiagnosticsParams, Range, RegistrationParams, ShowMessageParams,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem, Uri,
    VersionedTextDocumentIdentifier, WorkDoneProgressParams, WorkspaceClientCapabilities,
    WorkspaceFolder, WorkspaceFoldersChangeEvent,
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
        self.write_config_in("", content);
    }

    fn write_config_in(&self, folder: &str, content: &str) {
        let folder = self.root.path().join(folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("complexipy.toml"), content).unwrap();
    }

    fn folder(&self, folder: &str) -> WorkspaceFolder {
        WorkspaceFolder {
            uri: format!("file://{}", self.root.path().join(folder).display())
                .parse()
                .unwrap(),
            name: folder.to_string(),
        }
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
        self.initialize_with(&[""])
    }

    fn initialize_with(&mut self, folders: &[&str]) -> serde_json::Value {
        self.initialize_as(folders, ClientCapabilities::default())
    }

    fn initialize_watching(&mut self) -> serde_json::Value {
        self.initialize_as(
            &[""],
            ClientCapabilities {
                workspace: Some(WorkspaceClientCapabilities {
                    did_change_watched_files: Some(DidChangeWatchedFilesClientCapabilities {
                        dynamic_registration: Some(true),
                        ..DidChangeWatchedFilesClientCapabilities::default()
                    }),
                    ..WorkspaceClientCapabilities::default()
                }),
                ..ClientCapabilities::default()
            },
        )
    }

    fn initialize_as(
        &mut self,
        folders: &[&str],
        capabilities: ClientCapabilities,
    ) -> serde_json::Value {
        let id = self.next_id();
        let params = InitializeParams {
            workspace_folders: Some(folders.iter().map(|folder| self.folder(folder)).collect()),
            capabilities,
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

    fn change_with_range(&mut self, uri: &Uri, version: i32, text: &str) {
        self.notify(
            DID_CHANGE,
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: Some(Range::new(Position::new(0, 0), Position::new(0, 0))),
                    range_length: None,
                    text: text.to_string(),
                }],
            },
        );
    }

    fn open_as(&mut self, uri: &Uri, text: &str, language_id: &str) {
        self.notify(
            DID_OPEN,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: language_id.to_string(),
                    version: 1,
                    text: text.to_string(),
                },
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

    fn before_diagnostics(&mut self, uri: &Uri) -> Vec<Message> {
        let mut messages = Vec::new();
        loop {
            match self.receive() {
                Message::Notification(notification)
                    if notification.method == "textDocument/publishDiagnostics" =>
                {
                    let params: PublishDiagnosticsParams =
                        serde_json::from_value(notification.params).unwrap();
                    if &params.uri == uri {
                        return messages;
                    }
                }
                message => messages.push(message),
            }
        }
    }

    fn messages_before_diagnostics(&mut self, uri: &Uri) -> Vec<ShowMessageParams> {
        self.before_diagnostics(uri)
            .into_iter()
            .filter_map(|message| match message {
                Message::Notification(notification) if notification.method == SHOW_MESSAGE => {
                    Some(serde_json::from_value(notification.params).unwrap())
                }
                _ => None,
            })
            .collect()
    }

    fn registrations_before_diagnostics(&mut self, uri: &Uri) -> Vec<RegistrationParams> {
        self.before_diagnostics(uri)
            .into_iter()
            .filter_map(|message| match message {
                Message::Request(request) if request.method == REGISTER_CAPABILITY => {
                    Some(serde_json::from_value(request.params).unwrap())
                }
                _ => None,
            })
            .collect()
    }

    fn files_changed(&mut self, names: &[&str]) {
        let changes = names
            .iter()
            .map(|name| FileEvent::new(self.document_uri(name), FileChangeType::CHANGED))
            .collect();

        self.notify(
            DID_CHANGE_WATCHED_FILES,
            DidChangeWatchedFilesParams { changes },
        );
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
    assert_eq!(
        capabilities["workspace"]["workspaceFolders"],
        serde_json::json!({"supported": true, "changeNotifications": true})
    );
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
fn a_malformed_pattern_keeps_the_valid_exclusions() {
    let mut session = start();
    session.write_config("max-complexity-allowed = 2\nexclude = [\"[unclosed\", \"sample.py\"]\n");
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
fn an_unparseable_initialize_gets_an_error_response() {
    let mut session = start();
    let id = session.next_id();
    session.send(Request::new(
        id.clone(),
        "initialize".to_string(),
        serde_json::json!({ "rootUri": 42 }),
    ));

    let error = session
        .response(id)
        .expect_err("initialize should be rejected");

    assert_eq!(error.code, ErrorCode::InvalidParams as i32);
    assert_eq!(session.join(), EXIT_CODE_FAILURE);
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
fn stray_messages_during_shutdown_keep_the_exit_code_clean() {
    let mut session = start();
    session.initialize();
    let id = session.next_id();
    session.publish(id.clone(), SHUTDOWN, ());
    session.result(id);

    session.send(Message::Response(lsp_server::Response::new_ok(
        RequestId::from(9),
        serde_json::Value::Null,
    )));
    session.notify("textDocument/didChange", serde_json::json!({}));
    session.send(Notification::new(EXIT.to_string(), ()));

    assert_eq!(session.join(), EXIT_CODE_CLEAN);
}

#[test]
fn range_carrying_changes_are_rejected() {
    let mut session = start();
    session.write_config(STRICT);
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);
    assert_eq!(session.diagnostics(&uri).len(), 1);

    session.change_with_range(&uri, 2, "# comment\n");

    assert_eq!(session.inlay_hints(&uri).len(), 1);
    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn non_python_documents_stay_silent() {
    let mut session = start();
    session.write_config(STRICT);
    session.initialize();
    let uri = session.document_uri("data.json");
    session.open_as(&uri, "{\"a\": 1}", "json");

    assert!(session.diagnostics(&uri).is_empty());
    assert!(session.inlay_hints(&uri).is_empty());
    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn a_transient_syntax_error_keeps_the_last_analysis() {
    let mut session = start();
    session.write_config(STRICT);
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);
    assert_eq!(session.diagnostics(&uri).len(), 1);

    session.change(&uri, 2, "def broken(:\n");

    let diagnostics = session.diagnostics(&uri);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(lsp_types::NumberOrString::String(
            complexipy_lsp::analysis::PARSE_ERROR_CODE.to_string()
        ))
    );
    let hints = session.inlay_hints(&uri);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].position, Position::new(4, 16));

    session.change(&uri, 3, SAMPLE);

    let diagnostics = session.diagnostics(&uri);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(lsp_types::NumberOrString::String(
            complexipy_lsp::analysis::DIAGNOSTIC_CODE.to_string()
        ))
    );
    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
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

#[test]
fn each_folder_reads_its_own_config() {
    let mut session = start();
    session.write_config_in("strict", STRICT);
    session.write_config_in("lenient", "max-complexity-allowed = 15\n");
    session.initialize_with(&["strict", "lenient"]);

    let strict = session.document_uri("strict/sample.py");
    session.open(&strict, SAMPLE);
    assert_eq!(session.diagnostics(&strict).len(), 1);
    assert_eq!(session.inlay_hints(&strict).len(), 1);

    let lenient = session.document_uri("lenient/sample.py");
    session.open(&lenient, SAMPLE);
    assert!(session.diagnostics(&lenient).is_empty());
    assert!(session.inlay_hints(&lenient).is_empty());

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn exclude_patterns_stay_in_their_folder() {
    let mut session = start();
    session.write_config_in(
        "hidden",
        "max-complexity-allowed = 2\nexclude = [\"sample.py\"]\n",
    );
    session.write_config_in("shown", STRICT);
    session.initialize_with(&["shown", "hidden"]);

    let hidden = session.document_uri("hidden/sample.py");
    session.open(&hidden, SAMPLE);
    assert!(session.diagnostics(&hidden).is_empty());

    let shown = session.document_uri("shown/sample.py");
    session.open(&shown, SAMPLE);
    assert_eq!(session.diagnostics(&shown).len(), 1);

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn folders_added_and_removed_at_runtime_take_effect() {
    let mut session = start();
    session.write_config_in("first", "max-complexity-allowed = 15\n");
    session.write_config_in("added", STRICT);
    session.initialize_with(&["first"]);
    let uri = session.document_uri("added/sample.py");
    session.open(&uri, SAMPLE);
    assert!(session.diagnostics(&uri).is_empty());

    let added = session.folder("added");
    session.notify(
        DID_CHANGE_WORKSPACE_FOLDERS,
        DidChangeWorkspaceFoldersParams {
            event: WorkspaceFoldersChangeEvent {
                added: vec![added.clone()],
                removed: Vec::new(),
            },
        },
    );
    assert_eq!(session.diagnostics(&uri).len(), 1);

    session.notify(
        DID_CHANGE_WORKSPACE_FOLDERS,
        DidChangeWorkspaceFoldersParams {
            event: WorkspaceFoldersChangeEvent {
                added: Vec::new(),
                removed: vec![added],
            },
        },
    );
    assert!(session.diagnostics(&uri).is_empty());

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn config_problems_reach_the_client_once() {
    let mut session = start();
    session.write_config("select = [\"C07\"]\n");
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);

    let messages = session.messages_before_diagnostics(&uri);
    assert_eq!(messages.len(), 2);
    assert!(
        messages
            .iter()
            .all(|message| message.typ == MessageType::WARNING)
    );
    assert!(messages[0].message.contains("'C07'"));

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn a_clean_config_sends_no_message() {
    let mut session = start();
    session.write_config(STRICT);
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);

    assert!(session.messages_before_diagnostics(&uri).is_empty());

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn registers_the_config_watch_when_the_client_supports_it() {
    let mut session = start();
    session.initialize_watching();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);

    let registrations = session.registrations_before_diagnostics(&uri);
    assert_eq!(registrations.len(), 1);
    let registration = &registrations[0].registrations[0];
    assert_eq!(registration.method, DID_CHANGE_WATCHED_FILES);
    let options: DidChangeWatchedFilesRegistrationOptions =
        serde_json::from_value(registration.register_options.clone().unwrap()).unwrap();
    let globs: Vec<String> = options
        .watchers
        .into_iter()
        .map(|watcher| match watcher.glob_pattern {
            lsp_types::GlobPattern::String(glob) => glob,
            lsp_types::GlobPattern::Relative(_) => panic!("expected a plain glob"),
        })
        .collect();
    assert_eq!(
        globs,
        vec![
            "**/complexipy.toml",
            "**/.complexipy.toml",
            "**/pyproject.toml"
        ]
    );

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn registers_nothing_without_client_support() {
    let mut session = start();
    session.initialize();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);

    assert!(session.registrations_before_diagnostics(&uri).is_empty());

    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn a_config_edit_in_the_root_reloads() {
    let mut session = start();
    session.initialize_watching();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);
    assert!(session.diagnostics(&uri).is_empty());

    session.write_config(STRICT);
    session.files_changed(&["complexipy.toml"]);

    assert_eq!(session.diagnostics(&uri).len(), 1);
    assert_eq!(session.inlay_hints(&uri).len(), 1);
    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn a_nested_config_edit_does_not_reload() {
    let mut session = start();
    session.initialize_watching();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);
    assert!(session.diagnostics(&uri).is_empty());

    session.write_config(STRICT);
    session.files_changed(&[".venv/pkg/pyproject.toml"]);
    session.change(&uri, 2, SAMPLE);

    assert!(session.diagnostics(&uri).is_empty());
    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}

#[test]
fn a_malformed_config_edit_warns_once_and_uses_the_defaults() {
    let mut session = start();
    session.write_config(STRICT);
    session.initialize_watching();
    let uri = session.document_uri("sample.py");
    session.open(&uri, SAMPLE);
    assert_eq!(session.diagnostics(&uri).len(), 1);

    session.write_config("max-complexity-allowed = [\n");
    session.files_changed(&["complexipy.toml", "pyproject.toml"]);

    let messages = session.messages_before_diagnostics(&uri);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].message.starts_with("Failed to parse "));
    assert!(session.inlay_hints(&uri).is_empty());
    assert_eq!(session.shutdown(), EXIT_CODE_CLEAN);
}
