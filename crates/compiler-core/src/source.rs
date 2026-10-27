use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(file: FileId, start: usize, end: usize) -> Self {
        Self { file, start, end }
    }

    pub fn synthetic() -> Self {
        Self {
            file: FileId(usize::MAX),
            start: 0,
            end: 0,
        }
    }

    pub fn is_synthetic(self) -> bool {
        self.file.0 == usize::MAX
    }
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    path: PathBuf,
    text: String,
    line_starts: Vec<usize>,
    display_path: Option<String>,
}

impl SourceFile {
    pub fn new(path: PathBuf, text: String) -> Self {
        let mut line_starts = vec![0];
        for (index, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(index + 1);
            }
        }
        Self {
            path,
            text,
            line_starts,
            display_path: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Return the project-relative or logical library path recorded by a project loader.
    /// Files added directly to a [`SourceDb`] have no recorded display path.
    pub fn display_path(&self) -> Option<&str> {
        self.display_path.as_deref()
    }

    pub fn slice(&self, span: Span) -> &str {
        &self.text[span.start..span.end]
    }

    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let line = self.line_starts.partition_point(|&start| start <= offset);
        let line_index = line.saturating_sub(1);
        let line_start = self.line_starts[line_index];
        (line_index + 1, offset.saturating_sub(line_start) + 1)
    }

    pub fn offset_at_line_col(&self, line: usize, col: usize) -> usize {
        let line_index = line
            .saturating_sub(1)
            .min(self.line_starts.len().saturating_sub(1));
        let line_start = self.line_starts[line_index];
        let line_end = self
            .line_starts
            .get(line_index + 1)
            .copied()
            .unwrap_or(self.text.len());
        let max_col = line_end.saturating_sub(line_start) + 1;
        line_start + col.saturating_sub(1).min(max_col.saturating_sub(1))
    }
}

#[derive(Debug, Clone, Default)]
pub struct SourceDb {
    files: Vec<SourceFile>,
    roles: std::collections::BTreeMap<FileId, crate::roles::SourceRoles>,
    dependency_inputs: crate::roles::DependencyInputs,
}

impl SourceDb {
    pub fn dependency_inputs(&self) -> &crate::roles::DependencyInputs {
        &self.dependency_inputs
    }
    pub fn set_dependency_inputs(&mut self, inputs: crate::roles::DependencyInputs) {
        self.dependency_inputs = inputs;
    }
    pub fn roles(&self, file: FileId) -> Option<&crate::roles::SourceRoles> {
        self.roles.get(&file)
    }
    pub fn set_roles(&mut self, file: FileId, roles: crate::roles::SourceRoles) {
        self.roles.insert(file, roles);
    }
    pub fn select_role(&mut self, project: &Path, role: &str) -> Result<(), String> {
        let mut found = false;
        for source in self
            .roles
            .values_mut()
            .filter(|source| source.project == project)
        {
            let configuration = source
                .configuration
                .as_mut()
                .ok_or_else(|| "project has no role definitions".to_string())?;
            if !configuration.names.contains(role) {
                return Err(format!("unknown role `{role}`"));
            }
            configuration.selected = Some(role.into());
            found = true;
        }
        if found {
            self.dependency_inputs
                .selected_roles
                .insert(project.to_path_buf(), Some(role.into()));
            Ok(())
        } else {
            Err("project has no role definitions".into())
        }
    }
    pub fn add_file(&mut self, path: PathBuf, text: String) -> FileId {
        let id = FileId(self.files.len());
        self.files.push(SourceFile::new(path, text));
        id
    }

    pub(crate) fn add_file_with_display_path(
        &mut self,
        path: PathBuf,
        text: String,
        display_path: String,
    ) -> FileId {
        let id = self.add_file(path, text);
        self.files[id.0].display_path = Some(display_path);
        id
    }

    pub fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.0]
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }
}
