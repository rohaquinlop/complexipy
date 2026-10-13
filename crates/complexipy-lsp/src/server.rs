use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant};

use complexipy_core::config::config_file_names;
use crossbeam_channel::RecvTimeoutError;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    Diagnostic, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidChangeWatchedFilesRegistrationOptions, DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, FileSystemWatcher, GlobPattern,
    HoverParams, HoverProviderCapability, InitializeParams, InlayHintParams, MessageType, OneOf,
    PublishDiagnosticsParams, Registration, RegistrationParams, ServerCapabilities,
    ShowMessageParams, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};

use crate::analysis::{self, DocumentAnalysis, ParseFailure};
use crate::documents::{Documents, uri_to_path};
use crate::workspace::Workspace;

pub const SERVER_NAME: &str = "complexipy-lsp";
pub const DEBOUNCE: Duration = Duration::from_millis(200);

pub const DID_OPEN: &str = "textDocument/didOpen";
pub const DID_CHANGE: &str = "textDocument/didChange";
pub const DID_CLOSE: &str = "textDocument/didClose";
pub const DID_CHANGE_CONFIGURATION: &str = "workspace/didChangeConfiguration";
pub const DID_CHANGE_WORKSPACE_FOLDERS: &str = "workspace/didChangeWorkspaceFolders";
pub const DID_CHANGE_WATCHED_FILES: &str = "workspace/didChangeWatchedFiles";
pub const INLAY_HINT: &str = "textDocument/inlayHint";
pub const HOVER: &str = "textDocument/hover";
pub const INLAY_HINT_REFRESH: &str = "workspace/inlayHint/refresh";
pub const SHOW_MESSAGE: &str = "window/showMessage";
pub const REGISTER_CAPABILITY: &str = "client/registerCapability";
const CONFIG_WATCH_ID: &str = "complexipy-config-watch";
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
    workspace: Workspace,
    refresh_support: bool,
    watch_support: bool,
}

fn handshake(connection: &Connection) -> Result<Initialized, String> {
    let (id, params) = connection
        .initialize_start()
        .map_err(|error| error.to_string())?;
    let params: InitializeParams = match serde_json::from_value(params) {
        Ok(params) => params,
        Err(error) => {
            let message = error.to_string();

            let _ = connection.sender.send(
                Response::new_err(id, ErrorCode::InvalidParams as i32, message.clone()).into(),
            );

            return Err(message);
        }
    };

    let (roots, fallback_root) = workspace_roots(&params);
    let workspace_capabilities = params.capabilities.workspace;
    let refresh_support = workspace_capabilities
        .as_ref()
        .and_then(|workspace| workspace.inlay_hint.as_ref())
        .and_then(|inlay_hint| inlay_hint.refresh_support)
        .unwrap_or(false);
    let watch_support = workspace_capabilities
        .as_ref()
        .and_then(|workspace| workspace.did_change_watched_files.as_ref())
        .and_then(|watched_files| watched_files.dynamic_registration)
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
        workspace: Workspace::load(roots, fallback_root),
        refresh_support,
        watch_support,
    })
}

fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        inlay_hint_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                supported: Some(true),
                change_notifications: Some(OneOf::Left(true)),
            }),
            file_operations: None,
        }),
        ..ServerCapabilities::default()
    }
}

#[allow(deprecated)]
fn workspace_roots(params: &InitializeParams) -> (Vec<String>, String) {
    let roots = params
        .workspace_folders
        .iter()
        .flatten()
        .filter_map(|folder| uri_to_path(&folder.uri))
        .collect();
    let fallback_root = params
        .root_uri
        .as_ref()
        .and_then(uri_to_path)
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|path| path.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| ".".to_string());

    (roots, fallback_root)
}

struct Server {
    connection: Connection,
    workspace: Workspace,
    refresh_support: bool,
    watch_support: bool,
    documents: Documents,
    analyses: HashMap<String, DocumentAnalysis>,
    failures: HashMap<String, ParseFailure>,
    dirty: BTreeSet<String>,
    deadline: Option<Instant>,
    next_request_id: i32,
}

impl Server {
    fn new(connection: Connection, initialized: Initialized) -> Self {
        Self {
            connection,
            workspace: initialized.workspace,
            refresh_support: initialized.refresh_support,
            watch_support: initialized.watch_support,
            documents: Documents::default(),
            analyses: HashMap::new(),
            failures: HashMap::new(),
            dirty: BTreeSet::new(),
            deadline: None,
            next_request_id: 1,
        }
    }

    fn run(mut self) -> i32 {
        if self.watch_support {
            self.register_config_watch();
        }
        self.report_warnings();

        loop {
            match self.next_message() {
                Ok(Some(Message::Request(request))) => {
                    if request.method == SHUTDOWN {
                        return self.shutdown(request);
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

    fn shutdown(&self, request: Request) -> i32 {
        self.send(Response::new_ok(request.id, ()));

        self.await_exit()
    }

    fn await_exit(&self) -> i32 {
        loop {
            match self.connection.receiver.recv() {
                Ok(Message::Notification(notification)) if notification.method == EXIT => {
                    return EXIT_CODE_CLEAN;
                }
                Ok(_) => {}
                Err(_) => return EXIT_CODE_CLEAN,
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
                    self.documents.open(
                        item.uri.clone(),
                        item.text,
                        item.version,
                        item.language_id,
                    );
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
            DID_CHANGE_WATCHED_FILES => {
                if let Ok(params) =
                    notification.extract::<DidChangeWatchedFilesParams>(DID_CHANGE_WATCHED_FILES)
                {
                    self.change_watched_files(params);
                }
            }
            DID_CHANGE_WORKSPACE_FOLDERS => {
                if let Ok(params) = notification
                    .extract::<DidChangeWorkspaceFoldersParams>(DID_CHANGE_WORKSPACE_FOLDERS)
                {
                    self.change_folders(params);
                }
            }
            _ => {}
        }
    }

    fn change_document(&mut self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        if self.documents.get(&uri).is_none() {
            return;
        }

        if params
            .content_changes
            .iter()
            .any(|change| change.range.is_some())
        {
            eprintln!(
                "{}: {}: ignoring an incremental change, complexipy declares full document sync",
                SERVER_NAME,
                uri.as_str()
            );
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
        self.failures.remove(&key);
        self.dirty.remove(&key);
        self.send_diagnostics(uri, Vec::new(), None);
    }

    fn reload_config(&mut self) {
        self.workspace.reload();
        self.report_warnings();
        self.reanalyze_all();
    }

    fn change_watched_files(&mut self, params: DidChangeWatchedFilesParams) {
        if params
            .changes
            .iter()
            .filter_map(|change| uri_to_path(&change.uri))
            .any(|path| self.workspace.watches(&path))
        {
            self.reload_config();
        }
    }

    fn register_config_watch(&mut self) {
        let watchers = config_file_names()
            .map(|file_name| FileSystemWatcher {
                glob_pattern: GlobPattern::String(format!("**/{}", file_name)),
                kind: None,
            })
            .collect();
        let registration = Registration {
            id: CONFIG_WATCH_ID.to_string(),
            method: DID_CHANGE_WATCHED_FILES.to_string(),
            register_options: serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                watchers,
            })
            .ok(),
        };
        let id = self.next_request_id();

        self.send(Request::new(
            id,
            REGISTER_CAPABILITY.to_string(),
            RegistrationParams {
                registrations: vec![registration],
            },
        ));
    }

    fn change_folders(&mut self, params: DidChangeWorkspaceFoldersParams) {
        for folder in params.event.removed {
            if let Some(root) = uri_to_path(&folder.uri) {
                self.workspace.remove(&root);
            }
        }
        for folder in params.event.added {
            if let Some(root) = uri_to_path(&folder.uri) {
                self.workspace.add(root);
            }
        }
        self.report_warnings();
        self.reanalyze_all();
    }

    fn report_warnings(&mut self) {
        for warning in self.workspace.take_warnings() {
            eprintln!("{}: {}", SERVER_NAME, warning);
            self.send(Notification::new(
                SHOW_MESSAGE.to_string(),
                ShowMessageParams {
                    typ: MessageType::WARNING,
                    message: warning,
                },
            ));
        }
    }

    fn reanalyze_all(&mut self) {
        self.analyses.clear();
        self.failures.clear();

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

            published |= self.refresh(&uri);
        }

        if published && self.refresh_support {
            self.request_hint_refresh();
        }
    }

    fn refresh(&mut self, uri: &Uri) -> bool {
        let key = Documents::key(uri);
        let Some((text, version)) = self
            .documents
            .get(uri)
            .map(|document| (document.text.clone(), document.version))
        else {
            return false;
        };

        if self
            .failures
            .get(&key)
            .is_some_and(|failure| failure.version == version)
        {
            return false;
        }

        if let Some(analysis) = self.analyses.get(&key).cloned()
            && !analysis::is_stale(analysis.version, Some(version))
        {
            return self.publish(uri, analysis);
        }

        match self.compute(&text, version, uri) {
            Ok(analysis) => self.publish(uri, analysis),
            Err(failure) => {
                self.handle_failure(uri, version, failure);

                false
            }
        }
    }

    fn compute(
        &self,
        text: &str,
        version: i32,
        uri: &Uri,
    ) -> Result<DocumentAnalysis, ParseFailure> {
        if !self.documents.is_python(uri) || self.is_excluded(uri) {
            return Ok(DocumentAnalysis::empty(version));
        }

        analysis::analyze(text, version, &self.workspace.for_uri(uri).config)
    }

    fn current_analysis(&mut self, uri: &Uri) -> Option<DocumentAnalysis> {
        let key = Documents::key(uri);
        let (text, version) = self
            .documents
            .get(uri)
            .map(|document| (document.text.clone(), document.version))?;

        if self
            .failures
            .get(&key)
            .is_some_and(|failure| failure.version == version)
        {
            return self.analyses.get(&key).cloned();
        }

        if let Some(analysis) = self.analyses.get(&key).cloned()
            && !analysis::is_stale(analysis.version, Some(version))
        {
            return Some(analysis);
        }

        match self.compute(&text, version, uri) {
            Ok(analysis) => {
                self.failures.remove(&key);
                self.analyses.insert(key, analysis.clone());

                Some(analysis)
            }
            Err(failure) => {
                self.handle_failure(uri, version, failure);

                self.analyses.get(&Documents::key(uri)).cloned()
            }
        }
    }

    fn handle_failure(&mut self, uri: &Uri, version: i32, failure: ParseFailure) {
        let diagnostics = self.failure_diagnostics(uri, &failure);

        eprintln!("{}: {}: {}", SERVER_NAME, uri.as_str(), failure.message);
        self.failures.insert(Documents::key(uri), failure);
        self.send_diagnostics(uri.clone(), diagnostics, Some(version));
    }

    fn failure_diagnostics(&self, uri: &Uri, failure: &ParseFailure) -> Vec<Diagnostic> {
        if self.workspace.for_uri(uri).config.lsp.diagnostics {
            vec![failure.diagnostic()]
        } else {
            Vec::new()
        }
    }

    fn is_excluded(&self, uri: &Uri) -> bool {
        let Some(path) = uri_to_path(uri) else {
            return false;
        };

        self.workspace.for_path(&path).is_excluded(&path)
    }

    fn publish(&mut self, uri: &Uri, analysis: DocumentAnalysis) -> bool {
        if analysis::is_stale(analysis.version, self.documents.version(uri)) {
            return false;
        }

        let version = analysis.version;
        let diagnostics = analysis.diagnostics(&self.workspace.for_uri(uri).config);

        self.failures.remove(&Documents::key(uri));
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
        let id = self.next_request_id();

        self.send(Request::new(id, INLAY_HINT_REFRESH.to_string(), ()));
    }

    fn next_request_id(&mut self) -> RequestId {
        let id = RequestId::from(self.next_request_id);
        self.next_request_id += 1;

        id
    }

    fn respond_inlay_hints(&mut self, id: RequestId, params: InlayHintParams) {
        let uri = params.text_document.uri;
        let hints = match self.current_analysis(&uri) {
            Some(analysis) => {
                analysis.hints(&self.workspace.for_uri(&uri).config, Some(params.range))
            }
            None => Vec::new(),
        };

        self.send(Response::new_ok(id, hints));
    }

    fn respond_hover(&mut self, id: RequestId, params: HoverParams) {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let hover = match self.current_analysis(&uri) {
            Some(_) => {
                let config = &self.workspace.for_uri(&uri).config;
                self.analyses
                    .get_mut(&Documents::key(&uri))
                    .and_then(|analysis| analysis.hover(config, position))
            }
            None => None,
        };

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
