use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use complexipy_core::config::LspConfig;
use complexipy_core::discover_python_files;
use complexipy_core::encoding::decode_python_source;
use complexipy_core::fs_guard::{MAX_SOURCE_BYTES, read_bounded};
use complexipy_core::sanitize::escape_controls;
use crossbeam_channel::{Receiver, Sender, select};
use lsp_types::{Diagnostic, Uri};

use crate::analysis;
use crate::documents::path_to_uri;
use crate::server::{SERVER_NAME, spawn_with_large_stack};
use crate::workspace::{Folder, Workspace};

/// One file a scan analyzes from disk, published under `uri`. A `force`d
/// result is published even when it has no diagnostics.
pub struct ScanFile {
    pub path: String,
    pub uri: Uri,
    pub root: String,
    pub config: LspConfig,
    pub force: bool,
}

pub enum ScanJob {
    Workspace {
        generation: u64,
        workspace: Workspace,
    },
    Files {
        generation: u64,
        files: Vec<ScanFile>,
    },
}

pub enum ScanOutcome {
    Discovered {
        generation: u64,
        paths: Vec<String>,
    },
    Scanned {
        generation: u64,
        path: String,
        uri: Uri,
        diagnostics: Vec<Diagnostic>,
        force: bool,
    },
    Done {
        generation: u64,
    },
}

/// Starts the scan thread and returns its two queues: workspace scans, and
/// file rescans, which run between the files of a workspace scan instead of
/// waiting for it to end. It runs each job against `current`, and stops a
/// job as soon as `current` moves past the job's generation. With a `gate`,
/// it waits for one message on it before each file of a workspace scan.
pub fn spawn_scanner(
    current: Arc<AtomicU64>,
    done: Sender<ScanOutcome>,
    gate: Option<Receiver<()>>,
) -> (Sender<ScanJob>, Sender<ScanJob>) {
    let (jobs, queued) = crossbeam_channel::unbounded::<ScanJob>();
    let (rescans, urgent) = crossbeam_channel::unbounded::<ScanJob>();
    spawn_with_large_stack("scan", || {
        let scanner = Scanner {
            current: Arc::clone(&current),
            done: done.clone(),
            gate: gate.clone(),
            urgent: urgent.clone(),
        };
        let queued = queued.clone();
        move || {
            loop {
                let job = match scanner.urgent.try_recv() {
                    Ok(job) => Ok(job),
                    Err(_) => select! {
                        recv(scanner.urgent) -> job => job,
                        recv(queued) -> job => job,
                    },
                };
                match job {
                    Ok(job) => scanner.run(job),
                    Err(_) => return,
                }
            }
        }
    });

    (jobs, rescans)
}

struct Scanner {
    current: Arc<AtomicU64>,
    done: Sender<ScanOutcome>,
    gate: Option<Receiver<()>>,
    urgent: Receiver<ScanJob>,
}

impl Scanner {
    fn run(&self, job: ScanJob) {
        match job {
            ScanJob::Workspace {
                generation,
                workspace,
            } => {
                if self.scan_workspace(generation, &workspace) {
                    let _ = self.done.send(ScanOutcome::Done { generation });
                }
            }
            ScanJob::Files { generation, files } => {
                for file in files {
                    if !self.scan_file(generation, file) {
                        return;
                    }
                }
            }
        }
    }

    fn is_current(&self, generation: u64) -> bool {
        self.current.load(Ordering::SeqCst) == generation
    }

    fn scan_workspace(&self, generation: u64, workspace: &Workspace) -> bool {
        let mut seen: HashSet<String> = HashSet::new();
        for folder in workspace.folders().iter().filter(|folder| folder.scans()) {
            let files: Vec<ScanFile> = discover(folder, workspace)
                .into_iter()
                .filter(|path| seen.insert(path.clone()))
                .filter_map(|path| {
                    Some(ScanFile {
                        uri: path_to_uri(&path)?,
                        path,
                        root: folder.root.clone(),
                        config: folder.config.clone(),
                        force: false,
                    })
                })
                .collect();
            if !self.is_current(generation) {
                return false;
            }
            let _ = self.done.send(ScanOutcome::Discovered {
                generation,
                paths: files.iter().map(|file| file.path.clone()).collect(),
            });
            for file in files {
                if let Some(gate) = &self.gate {
                    let _ = gate.recv();
                }
                while let Ok(job) = self.urgent.try_recv() {
                    self.run(job);
                }
                if !self.scan_file(generation, file) {
                    return false;
                }
            }
        }

        true
    }

    fn scan_file(&self, generation: u64, file: ScanFile) -> bool {
        if !self.is_current(generation) {
            return false;
        }

        let diagnostics = diagnostics_from_disk(&file.path, &file.root, &file.config);
        let _ = self.done.send(ScanOutcome::Scanned {
            generation,
            path: file.path,
            uri: file.uri,
            diagnostics,
            force: file.force,
        });

        true
    }
}

/// The files the CLI analyzes for `folder`, as lexical paths under its
/// root, keeping only those `folder` owns and does not exclude.
fn discover(folder: &Folder, workspace: &Workspace) -> Vec<String> {
    let root = &folder.root;
    let arguments: Vec<String> = match folder.config.paths.clone().into_vec() {
        paths if paths.is_empty() => vec![root.clone()],
        paths => paths
            .iter()
            .map(|path| Path::new(root).join(path).to_string_lossy().into_owned())
            .collect(),
    };
    let (files, failures) =
        discover_python_files(&arguments, &folder.config.exclude.clone().into_vec(), root);
    for failure in failures {
        eprintln!("{}: {}", SERVER_NAME, escape_controls(&failure));
    }
    let canonical_root = Path::new(root)
        .canonicalize()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| root.clone());

    files
        .into_iter()
        .map(|file| lexical(&file, &canonical_root, root))
        .filter(|path| workspace.for_path(path).root == *root && !folder.is_excluded(path))
        .collect()
}

/// Moves a canonical `file` under `canonical_root` back onto `root`, the
/// form the client and the workspace use for the same folder.
pub fn lexical(file: &str, canonical_root: &str, root: &str) -> String {
    match file
        .strip_prefix(canonical_root.trim_end_matches('/'))
        .and_then(|rest| rest.strip_prefix('/'))
    {
        Some(rest) => format!("{}/{}", root.replace('\\', "/").trim_end_matches('/'), rest),
        None => file.to_string(),
    }
}

/// Reads and decodes `path` as the CLI does, and returns the diagnostics an
/// open document with the same text publishes.
fn diagnostics_from_disk(path: &str, root: &str, config: &LspConfig) -> Vec<Diagnostic> {
    let text = read_bounded(Path::new(path), Some(Path::new(root)), MAX_SOURCE_BYTES)
        .map_err(|error| error.to_string())
        .and_then(|bytes| decode_python_source(&bytes));
    let text = match text {
        Ok(text) => text,
        Err(error) => {
            eprintln!(
                "{}: {}: {}",
                SERVER_NAME,
                escape_controls(path),
                escape_controls(&error)
            );
            return Vec::new();
        }
    };

    match analysis::analyze(text, 0, config) {
        Ok(analysis) => analysis.diagnostics(config),
        Err(failure) => analysis::failure_diagnostics(&failure, config),
    }
}

#[cfg(test)]
mod tests;
