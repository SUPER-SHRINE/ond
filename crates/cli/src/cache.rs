use crate::DriverResult;
use ond_protocol::{ArtifactRef, PackageArtifact};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn encode<T: Serialize>(value: &T) -> DriverResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| vec![e.to_string()])
}
pub fn read_json<T: DeserializeOwned>(path: &Path) -> DriverResult<T> {
    let bytes = fs::read(path).map_err(|e| vec![format!("{}: {e}", path.display())])?;
    serde_json::from_slice(&bytes).map_err(|e| vec![format!("{}: {e}", path.display())])
}
pub fn read_artifact(reference: &ArtifactRef) -> DriverResult<PackageArtifact> {
    let bytes = fs::read(&reference.path)
        .map_err(|e| vec![format!("{}: {e}", reference.path.display())])?;
    if hash(&bytes) != reference.content_hash {
        return Err(vec![format!(
            "artifact checksum mismatch: {}",
            reference.package
        )]);
    }
    let artifact: PackageArtifact =
        serde_json::from_slice(&bytes).map_err(|e| vec![e.to_string()])?;
    artifact.contract.validate().map_err(|e| vec![e])?;
    artifact.interface.validate().map_err(|e| vec![e])?;
    if artifact.interface.package != artifact.package {
        return Err(vec!["export data package mismatch".into()]);
    }
    if artifact.package != reference.package
        || artifact.build_id != reference.build_id
        || artifact.object.name != artifact.package
        || artifact.debug.package != artifact.package
    {
        return Err(vec![format!(
            "artifact identity mismatch: {}",
            reference.package
        )]);
    }
    Ok(artifact)
}
pub fn lookup(directory: &Path, build_id: &str) -> Option<(ArtifactRef, PackageArtifact)> {
    let reference: ArtifactRef = read_json(&directory.join(format!("{build_id}.json"))).ok()?;
    // Cache indices never authorize reads outside their own cache directory.
    if reference.path.parent()? != directory {
        return None;
    }
    let artifact = read_artifact(&reference).ok()?;
    Some((reference, artifact))
}
pub fn publish(directory: &Path, artifact: &PackageArtifact) -> DriverResult<ArtifactRef> {
    let bytes = encode(artifact)?;
    let content_hash = hash(&bytes);
    let path = directory.join(format!("{}-{content_hash}.ondpkg", artifact.build_id));
    atomic_write(&path, &bytes)?;
    let reference = ArtifactRef {
        package: artifact.package.clone(),
        build_id: artifact.build_id.clone(),
        path,
        content_hash,
    };
    atomic_write(
        &directory.join(format!("{}.json", artifact.build_id)),
        &encode(&reference)?,
    )?;
    Ok(reference)
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> DriverResult<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .ok_or_else(|| vec!["output has no parent".into()])?;
    fs::create_dir_all(parent).map_err(|e| vec![e.to_string()])?;
    let (temp, mut file) = loop {
        let temp = parent.join(format!(
            ".ond-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => break (temp, file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(vec![e.to_string()]),
        }
    };
    let result = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    let result = result.and_then(|_| fs::rename(&temp, path));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map_err(|e| vec![format!("writing {}: {e}", path.display())])
}
