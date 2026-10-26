use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use complexipy_core::classes::RefactorPlan;
use complexipy_core::config::{LspConfig, config_file_names};
use complexipy_core::sanitize::escape_controls;
use crossbeam_channel::{Receiver, Sender, select};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CancelParams, Diagnostic, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidChangeWatchedFilesRegistrationOptions, DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, FileSystemWatcher, GlobPattern, Hover,
    HoverParams, HoverProviderCapability, InitializeParams, InlayHintParams, MessageType,
    NumberOrString, OneOf, Position, PublishDiagnosticsParams, Range, Registration,
    RegistrationParams, ServerCapabilities, ShowMessageParams, TextDocumentIdentifier,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};
use serde::Deserialize;

use crate::analysis::{self, DocumentAnalysis, ParseFailure};
use crate::documents::{Documents, uri_to_path};
use crate::workspace::Workspace;

pub const SERVER_NAME: &str = "complexipy-lsp";
pub const DEBOUNCE: Duration = Duration::from_millis(200);
const WORKER_STACK_SIZE: usize = 256 * 1024 * 1024;

pub const DID_OPEN: &str = "textDocument/didOpen";
pub const DID_CHANGE: &str = "textDocument/didChange";
pub const DID_CLOSE: &str = "textDocument/didClose";
pub const DID_CHANGE_CONFIGURATION: &str = "workspace/didChangeConfiguration";
pub const DID_CHANGE_WORKSPACE_FOLDERS: &str = "workspace/didChangeWorkspaceFolders";
pub const DID_CHANGE_WATCHED_FILES: &str = "workspace/didChangeWatchedFiles";
pub const CANCEL_REQUEST: &str = "$/cancelRequest";
pub const INLAY_HINT: &str = "textDocument/inlayHint";
pub const HOVER: &str = "textDocument/hover";
pub const LINE_COMPLEXITIES: &str = "complexipy/lineComplexities";
pub const INLAY_HINT_REFRESH: &str = "workspace/inlayHint/refresh";
pub const SHOW_MESSAGE: &str = "window/showMessage";
pub const CONFIG_WARNING_MESSAGE: &str = "complexipy found a problem in the project configuration. See the complexipy output log for details.";
pub const REGISTER_CAPABILITY: &str = "client/registerCapability";
const CONFIG_WATCH_ID: &str = "complexipy-config-watch";
pub const SHUTDOWN: &str = "shutdown";
pub const EXIT: &str = "exit";

pub const EXIT_CODE_CLEAN: i32 = 0;
pub const EXIT_CODE_FAILURE: i32 = 1;

const MAX_LOGGED_ERROR_CHARS: usize = 200;

pub fn serve(connection: Connection) -> i32 {
    let initialized = match handshake(&connection) {
        Ok(initialized) => initialized,
        Err(error) => {
            eprintln!("{}: {}", SERVER_NAME, escape_controls(&shortened(&error)));
            return EXIT_CODE_FAILURE;
        }
    };

    Server::new(connection, initialized).run()
}

fn shortened(text: &str) -> String {
    match text.char_indices().nth(MAX_LOGGED_ERROR_CHARS) {
        Some((end, _)) => format!("{}...", &text[..end]),
        None => text.to_string(),
    }
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

/// Work the analysis thread runs, off the message loop.
enum Job {
    Analyze {
        key: String,
        uri: Uri,
        text: Arc<str>,
        version: i32,
        generation: u64,
        config: LspConfig,
    },
    Plans {
        key: String,
        version: i32,
        line_start: u64,
        source: Arc<str>,
        generation: u64,
        config: LspConfig,
    },
}

enum Outcome {
    Analyzed {
        key: String,
        uri: Uri,
        version: i32,
        generation: u64,
        result: Result<DocumentAnalysis, ParseFailure>,
    },
    Planned {
        key: String,
        version: i32,
        line_start: u64,
        generation: u64,
        plans: Vec<RefactorPlan>,
    },
}

impl Job {
    fn run(self) -> Outcome {
        match self {
            Job::Analyze {
                key,
                uri,
                text,
                version,
                generation,
                config,
            } => Outcome::Analyzed {
                key,
                uri,
                version,
                generation,
                result: analysis::analyze(text, version, &config),
            },
            Job::Plans {
                key,
                version,
                line_start,
                source,
                generation,
                config,
            } => Outcome::Planned {
                key,
                version,
                line_start,
                generation,
                plans: analysis::plans_for(&source, line_start, &config),
            },
        }
    }
}

/// Starts the analysis thread, with a large stack where the platform can
/// reserve one and the default stack otherwise.
fn spawn_worker() -> (Sender<Job>, Receiver<Outcome>) {
    let (jobs, queued) = crossbeam_channel::unbounded::<Job>();
    let (done, outcomes) = crossbeam_channel::unbounded::<Outcome>();
    let spawn = |stack_size: Option<usize>| {
        let queued = queued.clone();
        let done = done.clone();
        let builder = thread::Builder::new().name(format!("{}-analysis", SERVER_NAME));
        let builder = match stack_size {
            Some(size) => builder.stack_size(size),
            None => builder,
        };
        builder.spawn(move || {
            for job in queued {
                if done.send(job.run()).is_err() {
                    break;
                }
            }
        })
    };
    spawn(Some(WORKER_STACK_SIZE))
        .or_else(|_| spawn(None))
        .expect("failed to start the analysis thread");

    (jobs, outcomes)
}

/// An analysis the worker runs for one version of a document: whether the
/// result is published as diagnostics, and the requests waiting for it.
struct PendingAnalysis {
    generation: u64,
    publish: bool,
    waiting: Vec<RequestId>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LineComplexitiesParams {
    text_document: TextDocumentIdentifier,
}

enum PendingRequest {
    Hints { uri: Uri, range: Range },
    Hover { uri: Uri, position: Position },
    LineComplexities { uri: Uri },
}

impl PendingRequest {
    fn uri(&self) -> &Uri {
        match self {
            PendingRequest::Hints { uri, .. }
            | PendingRequest::Hover { uri, .. }
            | PendingRequest::LineComplexities { uri } => uri,
        }
    }
}

enum Event {
    Message(Message),
    Outcome(Outcome),
    Due,
    Disconnected,
}

struct Server {
    connection: Connection,
    workspace: Workspace,
    refresh_support: bool,
    watch_support: bool,
    documents: Documents,
    analyses: HashMap<String, DocumentAnalysis>,
    failures: HashMap<String, ParseFailure>,
    deadlines: BTreeMap<String, Instant>,
    next_request_id: i32,
    worker: Sender<Job>,
    outcomes: Receiver<Outcome>,
    generation: u64,
    analyzing: HashMap<(String, i32), PendingAnalysis>,
    planning: HashMap<(String, i32, u64), Vec<RequestId>>,
    requests: HashMap<RequestId, PendingRequest>,
    refresh_due: bool,
    flushing: VecDeque<String>,
}

impl Server {
    fn new(connection: Connection, initialized: Initialized) -> Self {
        let (worker, outcomes) = spawn_worker();

        Self {
            connection,
            workspace: initialized.workspace,
            refresh_support: initialized.refresh_support,
            watch_support: initialized.watch_support,
            documents: Documents::default(),
            analyses: HashMap::new(),
            failures: HashMap::new(),
            deadlines: BTreeMap::new(),
            next_request_id: 1,
            worker,
            outcomes,
            generation: 0,
            analyzing: HashMap::new(),
            planning: HashMap::new(),
            requests: HashMap::new(),
            refresh_due: false,
            flushing: VecDeque::new(),
        }
    }

    fn run(mut self) -> i32 {
        if self.watch_support {
            self.register_config_watch();
        }
        self.report_warnings();

        loop {
            match self.next_event() {
                Event::Message(Message::Request(request)) => {
                    if request.method == SHUTDOWN {
                        return self.shutdown(request);
                    }

                    self.handle_request(request);
                }
                Event::Message(Message::Notification(notification)) => {
                    if notification.method == EXIT {
                        return EXIT_CODE_FAILURE;
                    }

                    self.handle_notification(notification);
                }
                Event::Message(Message::Response(_)) => {}
                Event::Outcome(outcome) => self.accept(outcome),
                Event::Due => self.flush_due(),
                Event::Disconnected => return EXIT_CODE_FAILURE,
            }
        }
    }

    fn shutdown(&mut self, request: Request) -> i32 {
        while !self.requests.is_empty() {
            let Ok(outcome) = self.outcomes.recv() else {
                break;
            };
            self.accept(outcome);
        }
        self.send(Response::new_ok(request.id, ()));

        self.await_exit()
    }

    fn await_exit(&self) -> i32 {
        loop {
            match self.connection.receiver.recv() {
                Ok(Message::Notification(notification)) if notification.method == EXIT => {
                    return EXIT_CODE_CLEAN;
                }
                Ok(Message::Request(request)) => self.send(Response::new_err(
                    request.id,
                    ErrorCode::InvalidRequest as i32,
                    "the server is shutting down".to_string(),
                )),
                Ok(_) => {}
                Err(_) => return EXIT_CODE_CLEAN,
            }
        }
    }

    fn next_event(&self) -> Event {
        let receiver = &self.connection.receiver;
        let outcomes = &self.outcomes;

        match self.deadlines.values().min() {
            Some(deadline) => select! {
                recv(receiver) -> message => message.map_or(Event::Disconnected, Event::Message),
                recv(outcomes) -> outcome => outcome.map_or(Event::Disconnected, Event::Outcome),
                default(deadline.saturating_duration_since(Instant::now())) => Event::Due,
            },
            None => select! {
                recv(receiver) -> message => message.map_or(Event::Disconnected, Event::Message),
                recv(outcomes) -> outcome => outcome.map_or(Event::Disconnected, Event::Outcome),
            },
        }
    }

    fn handle_request(&mut self, request: Request) {
        match request.method.as_str() {
            INLAY_HINT => {
                let id = request.id.clone();
                match request.extract::<InlayHintParams>(INLAY_HINT) {
                    Ok((id, params)) => self.dispatch(
                        id,
                        PendingRequest::Hints {
                            uri: params.text_document.uri,
                            range: params.range,
                        },
                    ),
                    Err(_) => self.respond_invalid_params(id),
                }
            }
            HOVER => {
                let id = request.id.clone();
                match request.extract::<HoverParams>(HOVER) {
                    Ok((id, params)) => self.dispatch(
                        id,
                        PendingRequest::Hover {
                            uri: params.text_document_position_params.text_document.uri,
                            position: params.text_document_position_params.position,
                        },
                    ),
                    Err(_) => self.respond_invalid_params(id),
                }
            }
            LINE_COMPLEXITIES => {
                let id = request.id.clone();
                match request.extract::<LineComplexitiesParams>(LINE_COMPLEXITIES) {
                    Ok((id, params)) => self.dispatch(
                        id,
                        PendingRequest::LineComplexities {
                            uri: params.text_document.uri,
                        },
                    ),
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
            CANCEL_REQUEST => {
                if let Ok(params) = notification.extract::<CancelParams>(CANCEL_REQUEST) {
                    self.cancel(params);
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
                escape_controls(uri.as_str())
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
        self.deadlines.remove(&key);
        self.send_diagnostics(uri, Vec::new(), None);
    }

    fn cancel(&mut self, params: CancelParams) {
        let id = match params.id {
            NumberOrString::Number(number) => RequestId::from(number),
            NumberOrString::String(text) => RequestId::from(text),
        };

        if self.requests.remove(&id).is_some() {
            self.send(Response::new_err(
                id,
                ErrorCode::RequestCanceled as i32,
                "request cancelled".to_string(),
            ));
        }
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

    /// Logs each configuration warning to stderr, and shows the editor one
    /// fixed message: a warning quotes repository text, which an editor
    /// could render as a clickable link.
    fn report_warnings(&mut self) {
        let warnings = self.workspace.take_warnings();
        if warnings.is_empty() {
            return;
        }
        for warning in &warnings {
            eprintln!("{}: {}", SERVER_NAME, escape_controls(warning));
        }
        self.send(Notification::new(
            SHOW_MESSAGE.to_string(),
            ShowMessageParams {
                typ: MessageType::WARNING,
                message: CONFIG_WARNING_MESSAGE.to_string(),
            },
        ));
    }

    /// Drops every analysis and marks every document dirty. Work already
    /// queued with the previous configuration is discarded when it returns,
    /// and the requests waiting on it start over with the new one.
    fn reanalyze_all(&mut self) {
        self.analyses.clear();
        self.failures.clear();
        self.generation += 1;

        let mut waiting: Vec<RequestId> = std::mem::take(&mut self.analyzing)
            .into_values()
            .flat_map(|pending| pending.waiting)
            .collect();
        waiting.extend(self.planning.drain().flat_map(|(_, ids)| ids));

        for key in self.documents.keys() {
            self.mark_dirty_key(key);
        }
        for id in waiting {
            if let Some(request) = self.requests.remove(&id) {
                self.dispatch(id, request);
            }
        }
    }

    fn mark_dirty(&mut self, uri: Uri) {
        self.mark_dirty_key(Documents::key(&uri));
    }

    fn mark_dirty_key(&mut self, key: String) {
        self.deadlines.insert(key, Instant::now() + DEBOUNCE);
    }

    /// Refreshes every document whose quiet period has passed. Each document
    /// keeps its own deadline, so edits to one never delay another.
    fn flush_due(&mut self) {
        let now = Instant::now();
        let due: Vec<String> = self
            .deadlines
            .iter()
            .filter(|(_, deadline)| **deadline <= now)
            .map(|(key, _)| key.clone())
            .collect();

        for key in due {
            self.deadlines.remove(&key);
            if !self.flushing.contains(&key) {
                self.flushing.push_back(key);
            }
        }

        self.pump_flush();
        self.request_refresh_when_idle();
    }

    /// Refreshes the documents a flush is waiting on, keeping at most one of
    /// their analyses queued, so a request queues its own analysis right
    /// behind the one that is running instead of behind the whole flush.
    fn pump_flush(&mut self) {
        while !self.analyzing.values().any(|pending| pending.publish) {
            let Some(key) = self.flushing.pop_front() else {
                return;
            };
            let Some(uri) = self
                .documents
                .get_keyed(&key)
                .map(|document| document.uri.clone())
            else {
                continue;
            };

            self.refresh_due |= self.refresh(&uri);
        }
    }

    /// Publishes the analysis of the document's current version. Returns
    /// whether it published now; when the analysis still has to run, it is
    /// queued and published once it returns.
    fn refresh(&mut self, uri: &Uri) -> bool {
        let key = Documents::key(uri);
        let Some((text, version)) = self
            .documents
            .get(uri)
            .map(|document| (Arc::clone(&document.text), document.version))
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

        if self
            .analyses
            .get(&key)
            .is_some_and(|analysis| !analysis::is_stale(analysis.version, Some(version)))
        {
            return self.publish_stored(uri);
        }

        if self.skips(uri) {
            return self.publish(uri, DocumentAnalysis::empty(version));
        }

        self.analyze(uri, key, text, version, true, None);

        false
    }

    fn skips(&self, uri: &Uri) -> bool {
        !self.documents.is_python(uri) || self.is_excluded(uri)
    }

    fn analyze(
        &mut self,
        uri: &Uri,
        key: String,
        text: Arc<str>,
        version: i32,
        publish: bool,
        waiting: Option<RequestId>,
    ) {
        let slot = (key, version);
        if let Some(pending) = self.analyzing.get_mut(&slot) {
            pending.publish |= publish;
            pending.waiting.extend(waiting);
            return;
        }

        let config = self.workspace.for_uri(uri).config.clone();
        let (key, version) = slot.clone();
        self.analyzing.insert(
            slot,
            PendingAnalysis {
                generation: self.generation,
                publish,
                waiting: waiting.into_iter().collect(),
            },
        );
        let _ = self.worker.send(Job::Analyze {
            key,
            uri: uri.clone(),
            text,
            version,
            generation: self.generation,
            config,
        });
    }

    fn accept(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Analyzed {
                key,
                uri,
                version,
                generation,
                result,
            } => self.accept_analysis(key, uri, version, generation, result),
            Outcome::Planned {
                key,
                version,
                line_start,
                generation,
                plans,
            } => self.accept_plans(key, version, line_start, generation, plans),
        }
        self.pump_flush();
        self.request_refresh_when_idle();
    }

    fn accept_analysis(
        &mut self,
        key: String,
        uri: Uri,
        version: i32,
        generation: u64,
        result: Result<DocumentAnalysis, ParseFailure>,
    ) {
        let slot = (key, version);
        if self
            .analyzing
            .get(&slot)
            .is_none_or(|pending| pending.generation != generation)
        {
            return;
        }
        let Some(pending) = self.analyzing.remove(&slot) else {
            return;
        };
        let (key, version) = slot;

        if self.documents.version(&uri) == Some(version) {
            match result {
                Ok(analysis) => {
                    self.failures.remove(&key);
                    self.analyses.insert(key, analysis);
                    if pending.publish {
                        self.refresh_due |= self.publish_stored(&uri);
                    }
                }
                Err(failure) => self.handle_failure(&uri, version, failure),
            }
        }

        for id in pending.waiting {
            if let Some(request) = self.requests.remove(&id) {
                self.dispatch(id, request);
            }
        }
    }

    fn accept_plans(
        &mut self,
        key: String,
        version: i32,
        line_start: u64,
        generation: u64,
        plans: Vec<RefactorPlan>,
    ) {
        let waiting = self
            .planning
            .remove(&(key.clone(), version, line_start))
            .unwrap_or_default();
        if generation == self.generation
            && let Some(analysis) = self.analyses.get_mut(&key)
            && analysis.version == version
        {
            analysis.insert_plans(line_start, plans);
        }

        for id in waiting {
            if let Some(request) = self.requests.remove(&id) {
                self.dispatch(id, request);
            }
        }
    }

    /// Answers a request from the analysis of the document's current
    /// version, or from the last one that parsed when the current version
    /// does not. When that analysis, or the plans a hover shows, still has
    /// to run, the request waits for it and is dispatched again then.
    fn dispatch(&mut self, id: RequestId, request: PendingRequest) {
        let uri = request.uri().clone();
        let key = Documents::key(&uri);
        let Some((text, version)) = self
            .documents
            .get(&uri)
            .map(|document| (Arc::clone(&document.text), document.version))
        else {
            self.answer(id, request, &key);
            return;
        };

        let failed = self
            .failures
            .get(&key)
            .is_some_and(|failure| failure.version == version);
        let fresh = self
            .analyses
            .get(&key)
            .is_some_and(|analysis| !analysis::is_stale(analysis.version, Some(version)));
        if failed || fresh {
            self.answer(id, request, &key);
            return;
        }

        if self.skips(&uri) {
            self.failures.remove(&key);
            self.analyses
                .insert(key.clone(), DocumentAnalysis::empty(version));
            self.answer(id, request, &key);
            return;
        }

        self.requests.insert(id.clone(), request);
        self.analyze(&uri, key, text, version, false, Some(id));
    }

    fn answer(&mut self, id: RequestId, request: PendingRequest, key: &str) {
        match request {
            PendingRequest::Hints { uri, range } => {
                let hints = match self.analyses.get(key) {
                    Some(analysis) => {
                        analysis.hints(&self.workspace.for_uri(&uri).config, Some(range))
                    }
                    None => Vec::new(),
                };

                self.send(Response::new_ok(id, hints));
            }
            PendingRequest::LineComplexities { uri } => {
                let config = &self.workspace.for_uri(&uri).config;
                let result = match self.analyses.get(key) {
                    Some(analysis) => analysis.line_complexities(config),
                    None => DocumentAnalysis::empty(0).line_complexities(config),
                };

                self.send(Response::new_ok(id, result));
            }
            PendingRequest::Hover { uri, position } => {
                let Some(analysis) = self.analyses.get(key) else {
                    self.send(Response::new_ok(id, None::<Hover>));
                    return;
                };

                if let Some(line_start) =
                    analysis.plans_needed(&self.workspace.for_uri(&uri).config, position)
                {
                    let slot = (key.to_string(), analysis.version, line_start);
                    let source = analysis.source();
                    let waiting = self.planning.entry(slot.clone()).or_default();
                    let queued = !waiting.is_empty();
                    waiting.push(id.clone());
                    self.requests.insert(
                        id,
                        PendingRequest::Hover {
                            uri: uri.clone(),
                            position,
                        },
                    );
                    if !queued {
                        let (key, version, line_start) = slot;
                        let _ = self.worker.send(Job::Plans {
                            key,
                            version,
                            line_start,
                            source,
                            generation: self.generation,
                            config: self.workspace.for_uri(&uri).config.clone(),
                        });
                    }
                    return;
                }

                let config = &self.workspace.for_uri(&uri).config;
                let hover = self
                    .analyses
                    .get_mut(key)
                    .and_then(|analysis| analysis.hover(config, position));

                self.send(Response::new_ok(id, hover));
            }
        }
    }

    fn handle_failure(&mut self, uri: &Uri, version: i32, failure: ParseFailure) {
        let diagnostics = self.failure_diagnostics(uri, &failure);

        eprintln!(
            "{}: {}: {}",
            SERVER_NAME,
            escape_controls(uri.as_str()),
            escape_controls(&failure.message)
        );
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

        self.analyses.insert(Documents::key(uri), analysis);
        self.publish_stored(uri)
    }

    fn publish_stored(&mut self, uri: &Uri) -> bool {
        let key = Documents::key(uri);
        let Some(analysis) = self.analyses.get(&key) else {
            return false;
        };
        let version = analysis.version;
        let diagnostics = analysis.diagnostics(&self.workspace.for_uri(uri).config);

        self.failures.remove(&key);
        self.send_diagnostics(uri.clone(), diagnostics, Some(version));

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

    /// Asks the client to refresh inlay hints once the diagnostics a flush
    /// started have all been published.
    fn request_refresh_when_idle(&mut self) {
        if !self.refresh_due
            || !self.flushing.is_empty()
            || self.analyzing.values().any(|pending| pending.publish)
        {
            return;
        }

        self.refresh_due = false;
        if self.refresh_support {
            self.request_hint_refresh();
        }
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
