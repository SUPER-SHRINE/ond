use super::SourceFile;
use crate::LoadedProject;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

pub(super) struct Fixture {
    root: PathBuf,
}

impl Fixture {
    pub fn new(files: &[SourceFile], manifest: bool) -> Result<Self, String> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "ond-conformance-{}-{time}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).map_err(|e| e.to_string())?;
        let fixture = Self { root };
        if manifest {
            std::fs::write(fixture.root.join("ond.toml"), "").map_err(|e| e.to_string())?;
        }
        for file in files {
            let path = fixture.root.join(file.path);
            std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(path, file.text).map_err(|e| e.to_string())?;
        }
        Ok(fixture)
    }

    pub fn load(&self) -> Result<LoadedProject, crate::FrontendError> {
        crate::load_project(&self.root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
