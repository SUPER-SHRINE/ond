//! Reject infinitely sized value types; pointer/function edges break size recursion.
use super::*;

pub(super) fn validate(project: &Project) -> Vec<ValidationError> {
    let types = &project.types;
    fn visit(id: TypeId, types: &TypeTable, state: &mut [u8]) -> bool {
        let Some(mark) = state.get(id.0 as usize).copied() else {
            return false;
        };
        if mark == 1 {
            return true;
        }
        if mark == 2 {
            return false;
        }
        state[id.0 as usize] = 1;
        let edges = match types.kind(id) {
            Some(TypeKind::Defined { underlying }) => vec![*underlying],
            Some(TypeKind::Array { element, .. }) => vec![*element],
            Some(TypeKind::Struct(s)) => s.fields.iter().map(|f| f.ty).collect(),
            _ => Vec::new(),
        };
        let cycle = edges.into_iter().any(|next| visit(next, types, state));
        state[id.0 as usize] = 2;
        cycle
    }
    let mut state = vec![0; types.definitions().len()];
    let mut errors = Vec::new();
    for builtin in TypeTable::new().definitions() {
        if types.kind(builtin.id) != Some(&builtin.kind) {
            errors.push(ValidationError {
                function: "<type-table>".into(),
                block: None,
                message: format!("{} does not have its canonical builtin type", builtin.id),
            });
        }
    }
    for definition in types.definitions() {
        if visit(definition.id, types, &mut state) {
            errors.push(ValidationError {
                function: "<type-table>".into(),
                block: None,
                message: format!("{} has a recursive value layout", definition.id),
            });
        }
        if let TypeKind::Struct(s) = &definition.kind {
            for field in &s.fields {
                let exported = field
                    .name
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_uppercase);
                if exported != field.private_owner.is_none()
                    || field
                        .private_owner
                        .is_some_and(|owner| !project.packages.iter().any(|p| p.id == owner))
                {
                    errors.push(ValidationError {
                        function: "<type-table>".into(),
                        block: None,
                        message: format!(
                            "{} field `{}` has invalid private owner",
                            definition.id, field.name
                        ),
                    });
                }
            }
            let mut names = std::collections::BTreeSet::new();
            if s.fields.iter().any(|f| !names.insert(&f.name)) {
                errors.push(ValidationError {
                    function: "<type-table>".into(),
                    block: None,
                    message: format!("{} has duplicate fields", definition.id),
                });
            }
        }
    }
    errors
}
