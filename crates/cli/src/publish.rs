//! Stage all binary/sidecar bytes before replacing any published output.
use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
pub fn publish(files: &[(PathBuf, Vec<u8>)]) -> Result<(), Vec<String>> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut paths = std::collections::BTreeSet::new();
    for (path, _) in files {
        let path = std::path::absolute(path).map_err(|e| vec![e.to_string()])?;
        let key = path.to_string_lossy().to_string();
        let key = if cfg!(windows) {
            key.to_lowercase()
        } else {
            key
        };
        if !paths.insert(key) {
            return Err(vec![
                "image output path conflicts with a sidecar path".into(),
            ]);
        }
    }
    let mut staged = Vec::new();
    let operation = (|| -> std::io::Result<()> {
        for (path, bytes) in files {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| std::path::Path::new("."));
            fs::create_dir_all(parent)?;
            if path.exists() && !path.is_file() {
                return Err(std::io::Error::other("output is not a regular file"));
            }
            let previous = if path.exists() {
                Some(fs::read(path)?)
            } else {
                None
            };
            let temp = parent.join(format!(
                ".ond-output-{}-{}.tmp",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            staged.push((temp.clone(), path.clone(), previous));
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        for (index, (temp, path, _)) in staged.iter().enumerate() {
            if let Err(error) = fs::rename(temp, path) {
                for (_, published, previous) in staged[..index].iter().rev() {
                    let restore = if let Some(bytes) = previous {
                        ond_cli::write_artifact(published, bytes)
                    } else {
                        fs::remove_file(published).map_err(|e| vec![e.to_string()])
                    };
                    if let Err(restore) = restore {
                        return Err(std::io::Error::other(format!(
                            "{error}; restoring {}: {}",
                            published.display(),
                            restore.join("; ")
                        )));
                    }
                }
                return Err(error);
            }
        }
        Ok(())
    })();
    for (temp, _, _) in staged {
        let _ = fs::remove_file(temp);
    }
    operation.map_err(|e| vec![format!("publishing executable: {e}")])
}
