use std::io::{self, Write};
use std::path::{Path, PathBuf};

use url::Url;

pub fn render_frontend_error(error: compiler::FrontendError) -> String {
    error
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn find_project_root(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };

    loop {
        if current.join("ond.toml").is_file() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

pub fn path_from_uri(uri: &str) -> Result<PathBuf, String> {
    Url::parse(uri)
        .map_err(|err| err.to_string())?
        .to_file_path()
        .map_err(|_| format!("unsupported uri `{uri}`"))
}

pub fn uri_from_path(path: &Path) -> Result<String, String> {
    Url::from_file_path(path)
        .map_err(|_| format!("failed to convert path `{}` to uri", path.display()))
        .map(|url| url.to_string())
}

pub fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

pub fn normalize_path(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

pub fn log_stderr(message: &str) {
    let _ = writeln!(io::stderr(), "{message}");
}
