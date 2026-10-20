//! File-backed dependency fixtures for CLI integration tests.
use std::{fs, path::Path};

pub(crate) fn source(root: &Path, name: &str, text: &str) {
    let (namespace, relative) = name.split_once('/').unwrap();
    let directory = root.join(".ond/dependency-fixtures").join(namespace);
    fs::create_dir_all(directory.join(relative).parent().unwrap()).unwrap();
    fs::write(directory.join(relative), text).unwrap();
    fs::write(
        directory.join("ond.toml"),
        format!("[project]\nname={namespace:?}\n"),
    )
    .unwrap();
    let path = root.join("ond.toml");
    let mut manifest: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    manifest
        .as_table_mut()
        .unwrap()
        .entry("dependencies")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .unwrap()
        .insert(
            namespace.into(),
            toml::Value::Table(toml::map::Map::from_iter([(
                "path".into(),
                toml::Value::String(format!(".ond/dependency-fixtures/{namespace}")),
            )])),
        );
    fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
}
