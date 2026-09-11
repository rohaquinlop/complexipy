use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant};

use complexipy_core::config::{LspConfig, read_complexipy_config};
use complexipy_core::{is_path_excluded, validate_exclude_patterns};
use crossbeam_channel::RecvTimeoutError;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    Diagnostic, DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    HoverParams, HoverProviderCapability, InitializeParams, InlayHintParams, OneOf,
    PublishDiagnosticsParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    Uri,
};
use serde::Deserialize;

use crate::analysis::{self, DocumentAnalysis};
use crate::documents::{Documents, uri_to_path};

pub const SERVER_NAME: &str = "complexipy-lsp";
pub const DEBOUNCE: Duration = Duration::from_millis(200);

pub const DID_OPEN: &str = "textDocument/didOpen";
pub const DID_CHANGE: &str = "textDocument/didChange";
pub const DID_CLOSE: &str = "textDocument/didClose";
pub const DID_CHANGE_CONFIGURATION: &str = "workspace/didChangeConfiguration";
pub const INLAY_HINT: &str = "textDocument/inlayHint";
pub const HOVER: &str = "textDocument/hover";
pub const INLAY_HINT_REFRESH: &str = "workspace/inlayHint/refresh";
pub const SHUTDOWN: &str = "shutdown";
pub const EXIT: &str = "exit";

pub const EXIT_CODE_CLEAN: i32 = 0;
pub const EXIT_CODE_FAILURE: i32 = 1;

pub fn serve(connection: Connection) -> i32 {
    let initialized = match handshake(&connection) {
        Ok(initialized) => initialized,
        Err(error) => {
            eprintln!("{}: {}", SERVER_NAME, error);
            return EXIT_CODE_FAILURE;
        }
    };

    Server::new(connection, initialized).run()
}

struct Initialized {
    root: String,
    config: LspConfig,
    refresh_support: bool,
}

fn handshake(connection: &Connection) -> Result<Initialized, String> {
    let (id, params) = connection
        .initialize_start()
        .map_err(|error| error.to_string())?;
    let params: InitializeParams =
        serde_json::from_value(params).map_err(|error| error.to_string())?;

    let root = workspace_root(&params);
    let refresh_support = params
        .capabilities
        .workspace
        .and_then(|workspace| workspace.inlay_hint)
        .and_then(|inlay_hint| inlay_hint.refresh_support)
        .unwrap_or(false);

    connection
        .initialize_finish(
            id,
            serde_json::json!({
                "capabilities": capabilities(),
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": env!("CARGO_PKG_VERSION"),
                },
            }),
        )
        .map_err(|error| error.to_string())?;

    Ok(Initialized {
        config: load_config(&root),
        root,
        refresh_support,
    })
}

fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        inlay_hint_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        ..ServerCapabilities::default()
    }
}

#[allow(deprecated)]
fn workspace_root(params: &InitializeParams) -> String {
    params
        .workspace_folders
        .as_ref()
        .and_then(|folders| folders.first())
        .and_then(|folder| uri_to_path(&folder.uri))
        .or_else(|| params.root_uri.as_ref().and_then(uri_to_path))
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|path| path.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| ".".to_string())
}

fn load_config(root: &str) -> LspConfig {
    let Some(source) = read_complexipy_config(root) else {
        return LspConfig::default();
    };

    match LspConfig::deserialize(source.value) {
        Ok(config) => {
            let patterns = config.exclude.clone().into_vec();

            if let Err(error) = validate_exclude_patterns(&patterns) {
                eprintln!("{}: {}: {}", SERVER_NAME, source.path.display(), error);
            }

            config
        }
        Err(error) => {
            eprintln!(
                "{}: invalid config in {}: {}",
                SERVER_NAME,
                source.path.display(),
                error
            );
            LspConfig::default()
        }
    }
}

struct Server {
    connection: Connection,
    root: String,
    config: LspConfig,
    refresh_support: bool,
    documents: Documents,
    analyses: HashMap<String, DocumentAnalysis>,
    dirty: BTreeSet<String>,
    deadline: Option<Instant>,
    next_request_id: i32,
}

impl Server {
    fn new(connection: Connection, initialized: Initialized) -> Self {
        Self {
            connection,
            root: initialized.root,
            config: initialized.config,
            refresh_support: initialized.refresh_support,
            documents: Documents::default(),
            analyses: HashMap::new(),
            dirty: BTreeSet::new(),
            deadline: None,
            next_request_id: 1,
        }
    }

    fn run(mut self) -> i32 {
        loop {
            match self.next_message() {
                Ok(Some(Message::Request(request))) => {
                    if request.method == SHUTDOWN {
                        return match self.connection.handle_shutdown(&request) {
                            Ok(_) => EXIT_CODE_CLEAN,
                            Err(error) => {
                                eprintln!("{}: {}", SERVER_NAME, error);
                                EXIT_CODE_FAILURE
                            }
                        };
                    }

                    self.handle_request(request);
                }
                Ok(Some(Message::Notification(notification))) => {
                    if notification.method == EXIT {
                        return EXIT_CODE_FAILURE;
                    }

                    self.handle_notification(notification);
                }
                Ok(Some(Message::Response(_))) => {}
                Ok(None) => self.flush_dirty(),
                Err(()) => return EXIT_CODE_FAILURE,
            }
        }
    }

    fn next_message(&self) -> Result<Option<Message>, ()> {
        match self.deadline {
            Some(deadline) => {
                let timeout = deadline.saturating_duration_since(Instant::now());

                match self.connection.receiver.recv_timeout(timeout) {
                    Ok(message) => Ok(Some(message)),
                    Err(RecvTimeoutError::Timeout) => Ok(None),
                    Err(RecvTimeoutError::Disconnected) => Err(()),
                }
            }
            None => self.connection.receiver.recv().map(Some).map_err(|_| ()),
        }
    }

    fn handle_request(&mut self, request: Request) {
        match request.method.as_str() {
            INLAY_HINT => {
                let id = request.id.clone();
                match request.extract::<InlayHintParams>(INLAY_HINT) {
                    Ok((id, params)) => self.respond_inlay_hints(id, params),
                    Err(_) => self.respond_invalid_params(id),
                }
            }
            HOVER => {
                let id = request.id.clone();
                match request.extract::<HoverParams>(HOVER) {
                    Ok((id, params)) => self.respond_hover(id, params),
                    Err(_) => self.respond_invalid_params(id),
                }
            }
            other => {
                self.send(Response::new_err(
                    request.id,
                    ErrorCode::MethodNotFound as i32,
                    format!("unsupported request: {}", other),
                ));
            }
        }
    }

    fn handle_notification(&mut self, notification: Notification) {
        match notification.method.as_str() {
            DID_OPEN => {
                if let Ok(params) = notification.extract::<DidOpenTextDocumentParams>(DID_OPEN) {
                    let item = params.text_document;
                    self.documents
                        .open(item.uri.clone(), item.text, item.version);
                    self.mark_dirty(item.uri);
                }
            }
            DID_CHANGE => {
                if let Ok(params) = notification.extract::<DidChangeTextDocumentParams>(DID_CHANGE)
                {
                    self.change_document(params);
                }
            }
            DID_CLOSE => {
                if let Ok(params) = notification.extract::<DidCloseTextDocumentParams>(DID_CLOSE) {
                    self.close_document(params);
                }
            }
            DID_CHANGE_CONFIGURATION => self.reload_config(),
            _ => {}
        }
    }

    fn change_document(&mut self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        if self.documents.get(&uri).is_none() {
            return;
        }

        let Some(change) = params.content_changes.into_iter().next_back() else {
            return;
        };

        self.documents
            .change(&uri, change.text, params.text_document.version);
        self.mark_dirty(uri);
    }

    fn close_document(&mut self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        let key = Documents::key(&uri);
        self.documents.close(&uri);
        self.analyses.remove(&key);
        self.dirty.remove(&key);
        self.send_diagnostics(uri, Vec::new(), None);
    }

    fn reload_config(&mut self) {
        self.config = load_config(&self.root);

        for key in self.documents.keys() {
            self.mark_dirty_key(key);
        }
    }

    fn mark_dirty(&mut self, uri: Uri) {
        self.mark_dirty_key(Documents::key(&uri));
    }

    fn mark_dirty_key(&mut self, key: String) {
        self.dirty.insert(key);
        self.deadline = Some(Instant::now() + DEBOUNCE);
    }

    fn flush_dirty(&mut self) {
        self.deadline = None;
        let dirty = std::mem::take(&mut self.dirty);
        let mut published = false;

        for key in dirty {
            let Some(uri) = self
                .documents
                .get_keyed(&key)
                .map(|document| document.uri.clone())
            else {
                continue;
            };

            published |= self.analyze_and_publish(&uri);
        }

        if published && self.refresh_support {
            self.request_hint_refresh();
        }
    }

    fn analyze_and_publish(&mut self, uri: &Uri) -> bool {
        let Some(document) = self.documents.get(uri) else {
            return false;
        };

        let text = document.text.clone();
        let version = document.version;
        let analysis = self.compute(&text, version, uri);

        self.publish(uri, analysis)
    }

    fn compute(&self, text: &str, version: i32, uri: &Uri) -> DocumentAnalysis {
        if self.is_excluded(uri) {
            return DocumentAnalysis::empty(version);
        }

        match analysis::analyze(text, version, &self.config) {
            Ok(analysis) => analysis,
            Err(error) => {
                eprintln!("{}: {}: {}", SERVER_NAME, uri.as_str(), error);
                DocumentAnalysis::empty(version)
            }
        }
    }

    fn current_analysis(&mut self, uri: &Uri) -> Option<DocumentAnalysis> {
        let document = self.documents.get(uri)?;
        let version = document.version;
        let text = document.text.clone();

        if let Some(analysis) = self.analyses.get(&Documents::key(uri))
            && !analysis::is_stale(analysis.version, Some(version))
        {
            return Some(analysis.clone());
        }

        let analysis = self.compute(&text, version, uri);
        self.analyses.insert(Documents::key(uri), analysis.clone());

        Some(analysis)
    }

    fn is_excluded(&self, uri: &Uri) -> bool {
        let Some(path) = uri_to_path(uri) else {
            return false;
        };

        let patterns = self.config.exclude.clone().into_vec();
        is_path_excluded(&path, &self.root, &patterns)
    }

    fn publish(&mut self, uri: &Uri, analysis: DocumentAnalysis) -> bool {
        if analysis::is_stale(analysis.version, self.documents.version(uri)) {
            return false;
        }

        let text = self
            .documents
            .get(uri)
            .map(|document| document.text.clone())
            .unwrap_or_default();
        let version = analysis.version;
        let diagnostics = analysis.diagnostics(&text, &self.config);

        self.send_diagnostics(uri.clone(), diagnostics, Some(version));
        self.analyses.insert(Documents::key(uri), analysis);

        true
    }

    fn send_diagnostics(&self, uri: Uri, diagnostics: Vec<Diagnostic>, version: Option<i32>) {
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version,
        };

        self.send(Notification::new(
            "textDocument/publishDiagnostics".to_string(),
            params,
        ));
    }

    fn request_hint_refresh(&mut self) {
        let id = RequestId::from(self.next_request_id);
        self.next_request_id += 1;

        self.send(Request::new(id, INLAY_HINT_REFRESH.to_string(), ()));
    }

    fn respond_inlay_hints(&mut self, id: RequestId, params: InlayHintParams) {
        let hints = match self.current_analysis(&params.text_document.uri) {
            Some(analysis) => {
                let text = self
                    .documents
                    .get(&params.text_document.uri)
                    .map(|document| document.text.clone())
                    .unwrap_or_default();
                analysis.hints(&text, &self.config, Some(params.range))
            }
            None => Vec::new(),
        };

        self.send(Response::new_ok(id, hints));
    }

    fn respond_hover(&mut self, id: RequestId, params: HoverParams) {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;

        let hover = self.current_analysis(&uri).and_then(|analysis| {
            let text = self
                .documents
                .get(&uri)
                .map(|document| document.text.clone())
                .unwrap_or_default();
            analysis.hover(&text, &self.config, position)
        });

        self.send(Response::new_ok(id, hover));
    }

    fn respond_invalid_params(&self, id: RequestId) {
        self.send(Response::new_err(
            id,
            ErrorCode::InvalidParams as i32,
            "invalid parameters".to_string(),
        ));
    }

    fn send(&self, message: impl Into<Message>) {
        let _ = self.connection.sender.send(message.into());
    }
}

#[cfg(test)]
mod tests;
