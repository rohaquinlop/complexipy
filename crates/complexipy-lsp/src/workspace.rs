use std::path::Path;

use complexipy_core::config::{LspConfig, is_config_file_name, read_complexipy_config};
use complexipy_core::{
    ExcludeMatcher, no_active_rule_warning, registered_rule_ids, unknown_rule_warning,
    unmatchable_exclude_patterns,
};
use lsp_types::Uri;

use crate::documents::{lexically_normalized, uri_to_path};

pub struct Folder {
    pub root: String,
    pub config: LspConfig,
    exclude: ExcludeMatcher,
    anchors: Vec<String>,
    has_config: bool,
}

impl Folder {
    /// A folder whose own configuration file holds `config`.
    pub fn new(root: String, config: LspConfig) -> Self {
        let exclude = ExcludeMatcher::new(&config.exclude.clone().into_vec());
        let anchors = config
            .paths
            .clone()
            .into_vec()
            .iter()
            .map(|path| {
                let joined = Path::new(&root).join(path);
                lexically_normalized(&joined.to_string_lossy().replace('\\', "/"))
            })
            .collect();

        Self {
            root,
            config,
            exclude,
            anchors,
            has_config: true,
        }
    }

    pub fn load(root: String) -> (Self, Vec<String>) {
        let (source, mut warnings) = read_complexipy_config::<LspConfig>(&root);
        let Some(source) = source else {
            let folder = Self {
                has_config: false,
                ..Self::new(root, LspConfig::default())
            };
            return (folder, warnings);
        };
        let path = source.path.display().to_string();
        let config = source.value;

        let (rules, unknown) = config.resolve_rules();
        for rule_id in &unknown {
            warnings.push(format!("{}: {}", path, unknown_rule_warning(rule_id)));
        }
        if !rules.has_active_rule(&registered_rule_ids()) {
            warnings.push(format!("{}: {}", path, no_active_rule_warning()));
        }

        let folder = Self::new(root, config);
        let unmatchable = unmatchable_exclude_patterns(&folder.config.exclude.clone().into_vec());
        if !unmatchable.is_empty() {
            warnings.push(format!(
                "{}: these exclude patterns can never match, because paths are matched \
                 relative to the folder without a leading `/` or `./` and never end \
                 with `/`: {:?}",
                path, unmatchable
            ));
        }
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

    /// Matches `path` relative to the folder, and relative to each `paths`
    /// entry of the configuration that contains it, as a CLI run in the
    /// folder does.
    pub fn is_excluded(&self, path: &str) -> bool {
        self.exclude.is_excluded(path, &self.root)
            || self
                .anchors
                .iter()
                .any(|anchor| self.exclude.is_excluded_inside(path, anchor))
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

    /// The deepest folder with its own configuration that contains `path`,
    /// or the deepest folder that contains it when none has one.
    pub fn for_path(&self, path: &str) -> &Folder {
        let path = Path::new(path);
        let mut containing: Vec<&Folder> = self
            .folders
            .iter()
            .filter(|folder| folder.contains(path))
            .collect();
        containing.sort_by_key(|folder| std::cmp::Reverse(folder.depth()));

        containing
            .iter()
            .find(|folder| folder.has_config)
            .or(containing.first())
            .copied()
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
