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
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
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
}

impl SourceDb {
    pub fn add_file(&mut self, path: PathBuf, text: String) -> FileId {
        let id = FileId(self.files.len());
        self.files.push(SourceFile::new(path, text));
        id
    }

    pub fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.0]
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }
}
