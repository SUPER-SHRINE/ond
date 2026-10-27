//! Declaration visibility and import conflicts in a source snapshot.
use super::{PackageId, Span, TypePackage};

impl TypePackage {
    pub(in crate::pipeline::hir) fn visibility_error(
        &self,
        kind: &str,
        name: &str,
        declaration: Span,
        caller: Span,
    ) -> String {
        let reason = if self.test_files.contains(&declaration.file)
            && !self.test_files.contains(&caller.file)
        {
            "test-only"
        } else {
            "role-limited"
        };
        format!("{reason} {kind} `{name}` is not visible here")
    }

    pub(in crate::pipeline::hir) fn value_visible_from(&self, name: &str, caller: Span) -> bool {
        let declaration = self
            .constants
            .get(name)
            .map(|(_, value)| value.span())
            .or_else(|| self.globals.get(name).map(|spec| spec.span))
            .or_else(|| self.functions.get(name).map(|signature| signature.span))
            .or_else(|| {
                self.generic_functions
                    .get(name)
                    .map(|function| function.span)
            });
        declaration.is_some_and(|declaration| self.visible_from(declaration, caller))
    }

    pub(in crate::pipeline::hir) fn visible_from(&self, declaration: Span, caller: Span) -> bool {
        (!self.test_files.contains(&declaration.file) || self.test_files.contains(&caller.file))
            && crate::roles::visible(&self.sources, declaration, caller)
    }

    pub(in crate::pipeline::hir) fn import_from(
        &self,
        name: &str,
        caller: Span,
    ) -> Option<PackageId> {
        let span = self.import_spans.get(&(caller.file, name.into()))?;
        self.visible_from(*span, caller)
            .then(|| self.imports.get(&caller.file)?.get(name).copied())
            .flatten()
    }

    pub(in crate::pipeline::hir) fn declaration_conflicts_with_import(
        &self,
        declaration: Span,
        import: Span,
    ) -> bool {
        if self.test_files.contains(&declaration.file) && !self.test_files.contains(&import.file) {
            return false;
        }
        crate::roles::overlap(&self.sources, declaration, import)
    }
}
