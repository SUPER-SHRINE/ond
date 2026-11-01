//! 規範caseの完全入力。CLI processとdriver runtimeの両方から使う。
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
pub(super) const RUNNER: &str = "[project]\nname=\"runner\"\nfinal=true\n[dependencies]\nchild={path=\"../child\",role=\"Test\"}\n";
pub(super) const CHILD: &str = "[project]\nname=\"child\"\n[roles]\nnames=[\"Test\"]\n";
pub(super) const MAIN: &str =
    "package main\nimport \"child\"\nvar Result:i32\nfunc main(){Result=child.Value[i32](7)}\n";
pub(super) const CHILD_SOURCE: &str = "package child\nfunc Value[T](x:T)->i32{role Test {return helper()};return 10}\nfunc CommonApi()->i32{return 1}\nrole Test {func helper()->i32{return 20};func TestApi()->i32{return 2}}\n";
pub(super) const NO_IMPORT: &str = "package main\nfunc main(){}\n";
pub(super) struct ProjectFixture {
    pub(super) base: PathBuf,
    pub(super) root: PathBuf,
}
impl ProjectFixture {
    pub(super) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "ond-role-edge-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&base).unwrap();
        let f = Self {
            root: base.join("runner"),
            base,
        };
        f.reset();
        f
    }
    pub(super) fn put(&self, path: &str, text: &str) {
        let path = self.base.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    pub(super) fn reset(&self) {
        self.put("runner/ond.toml", RUNNER);
        self.put("runner/main.ond", MAIN);
        self.put("child/ond.toml", CHILD);
        self.put("child/lib.ond", CHILD_SOURCE);
    }
    pub(super) fn runner(&self, text: &str) {
        self.put("runner/ond.toml", text);
    }
    pub(super) fn diamond(&self) {
        self.runner("[project]\nname=\"runner\"\nfinal=true\n[dependencies]\ndirect={path=\"../child\",role=\"Test\"}\nmiddle={path=\"../bridge\"}\n");
        self.put("runner/main.ond","package main\nimport \"child\"\nimport \"bridge\"\nvar Result:i32\nfunc main(){Result=child.Value[i32](7)+bridge.Read()}\n");
        self.put(
            "bridge/ond.toml",
            "[project]\nname=\"bridge\"\n[dependencies]\nleaf={path=\"../child\"}\n",
        );
        self.put(
            "bridge/lib.ond",
            "package bridge\nimport \"child\"\nfunc Read()->i32{return child.Value[i32](1)}\n",
        );
    }
}
impl Drop for ProjectFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}
