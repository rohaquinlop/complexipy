use std::path::Path;

use complexipy_core::ExcludeMatcher;
use complexipy_core::config::{LspConfig, read_complexipy_config};
use lsp_types::Uri;
use serde::Deserialize;

use crate::documents::uri_to_path;
use crate::server::SERVER_NAME;

pub struct Folder {
    pub root: String,
    pub config: LspConfig,
    exclude: ExcludeMatcher,
}

impl Folder {
    pub fn new(root: String, config: LspConfig) -> Self {
        let exclude = ExcludeMatcher::new(&config.exclude.clone().into_vec());

        Self {
            root,
            config,
            exclude,
        }
    }

    pub fn load(root: String) -> Self {
        let Some(source) = read_complexipy_config(&root) else {
            return Self::new(root, LspConfig::default());
        };

        let config = match LspConfig::deserialize(source.value) {
            Ok(config) => config,
            Err(error) => {
                eprintln!(
                    "{}: invalid config in {}: {}",
                    SERVER_NAME,
                    source.path.display(),
                    error
                );
                return Self::new(root, LspConfig::default());
            }
        };

        let folder = Self::new(root, config);
        if !folder.exclude.invalid().is_empty() {
            eprintln!(
                "{}: {}: ignoring invalid exclude patterns: {:?}",
                SERVER_NAME,
                source.path.display(),
                folder.exclude.invalid()
            );
        } else if folder.exclude.overflows() {
            eprintln!(
                "{}: {}: the exclude list of {} patterns is too large to \
                 compile as one program; matching one pattern at a time",
                SERVER_NAME,
                source.path.display(),
                folder.config.exclude.clone().into_vec().len()
            );
        }

        folder
    }

    pub fn is_excluded(&self, path: &str) -> bool {
        self.exclude.is_excluded(path, &self.root)
    }

    fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }

    fn depth(&self) -> usize {
        Path::new(&self.root).components().count()
    }

    fn is_root(&self, root: &str) -> bool {
        Path::new(&self.root) == Path::new(root)
    }
}

pub struct Workspace {
    folders: Vec<Folder>,
    fallback_root: String,
}

impl Workspace {
    #[cfg(test)]
    pub fn new(folders: Vec<Folder>, fallback_root: String) -> Self {
        let mut workspace = Self {
            folders,
            fallback_root,
        };
        workspace.keep_one_folder();

        workspace
    }

    pub fn load(roots: Vec<String>, fallback_root: String) -> Self {
        let mut workspace = Self {
            folders: Vec::new(),
            fallback_root,
        };
        for root in roots {
            workspace.add(root);
        }
        workspace.keep_one_folder();

        workspace
    }

    pub fn for_path(&self, path: &str) -> &Folder {
        let path = Path::new(path);

        self.folders
            .iter()
            .filter(|folder| folder.contains(path))
            .max_by_key(|folder| folder.depth())
            .unwrap_or_else(|| self.first())
    }

    pub fn for_uri(&self, uri: &Uri) -> &Folder {
        match uri_to_path(uri) {
            Some(path) => self.for_path(&path),
            None => self.first(),
        }
    }

    pub fn add(&mut self, root: String) {
        if !self.folders.iter().any(|folder| folder.is_root(&root)) {
            self.folders.push(Folder::load(root));
        }
    }

    pub fn remove(&mut self, root: &str) {
        self.folders.retain(|folder| !folder.is_root(root));
        self.keep_one_folder();
    }

    pub fn reload(&mut self) {
        for folder in &mut self.folders {
            *folder = Folder::load(std::mem::take(&mut folder.root));
        }
    }

    fn first(&self) -> &Folder {
        &self.folders[0]
    }

    fn keep_one_folder(&mut self) {
        if self.folders.is_empty() {
            self.folders.push(Folder::load(self.fallback_root.clone()));
        }
    }
}

#[cfg(test)]
mod tests;
