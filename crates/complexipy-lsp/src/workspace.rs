use std::path::Path;

use complexipy_core::config::{LspConfig, is_config_file_name, read_complexipy_config};
use complexipy_core::{
    ExcludeMatcher, no_active_rule_warning, registered_rule_ids, unknown_rule_warning,
};
use lsp_types::Uri;
use serde::Deserialize;

use crate::documents::uri_to_path;

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

    pub fn load(root: String) -> (Self, Vec<String>) {
        let (source, mut warnings) = read_complexipy_config(&root);
        let Some(source) = source else {
            return (Self::new(root, LspConfig::default()), warnings);
        };
        let path = source.path.display().to_string();

        let config = match LspConfig::deserialize(source.value) {
            Ok(config) => config,
            Err(error) => {
                warnings.push(format!("invalid config in {}: {}", path, error));
                return (Self::new(root, LspConfig::default()), warnings);
            }
        };

        let (rules, unknown) = config.resolve_rules();
        for rule_id in &unknown {
            warnings.push(format!("{}: {}", path, unknown_rule_warning(rule_id)));
        }
        if !rules.has_active_rule(&registered_rule_ids()) {
            warnings.push(format!("{}: {}", path, no_active_rule_warning()));
        }

        let folder = Self::new(root, config);
        if !folder.exclude.invalid().is_empty() {
            warnings.push(format!(
                "{}: ignoring invalid exclude patterns: {:?}",
                path,
                folder.exclude.invalid()
            ));
        } else if folder.exclude.overflows() {
            warnings.push(format!(
                "{}: the exclude list of {} patterns is too large to compile as \
                 one program; matching one pattern at a time",
                path,
                folder.config.exclude.clone().into_vec().len()
            ));
        }

        (folder, warnings)
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
    warnings: Vec<String>,
}

impl Workspace {
    #[cfg(test)]
    pub fn new(folders: Vec<Folder>, fallback_root: String) -> Self {
        let mut workspace = Self {
            folders,
            fallback_root,
            warnings: Vec::new(),
        };
        workspace.keep_one_folder();

        workspace
    }

    pub fn load(roots: Vec<String>, fallback_root: String) -> Self {
        let mut workspace = Self {
            folders: Vec::new(),
            fallback_root,
            warnings: Vec::new(),
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
            let folder = self.load_folder(root);
            self.folders.push(folder);
        }
    }

    pub fn remove(&mut self, root: &str) {
        self.folders.retain(|folder| !folder.is_root(root));
        self.keep_one_folder();
    }

    pub fn reload(&mut self) {
        let roots: Vec<String> = self.folders.drain(..).map(|folder| folder.root).collect();
        for root in roots {
            let folder = self.load_folder(root);
            self.folders.push(folder);
        }
    }

    pub fn watches(&self, path: &str) -> bool {
        let path = Path::new(path);
        let (Some(name), Some(parent)) = (path.file_name(), path.parent()) else {
            return false;
        };

        name.to_str().is_some_and(is_config_file_name)
            && self
                .folders
                .iter()
                .any(|folder| Path::new(&folder.root) == parent)
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    fn load_folder(&mut self, root: String) -> Folder {
        let (folder, warnings) = Folder::load(root);
        self.warnings.extend(warnings);

        folder
    }

    fn first(&self) -> &Folder {
        &self.folders[0]
    }

    fn keep_one_folder(&mut self) {
        if self.folders.is_empty() {
            let folder = self.load_folder(self.fallback_root.clone());
            self.folders.push(folder);
        }
    }
}

#[cfg(test)]
mod tests;
