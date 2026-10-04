use compiler::ir::ast;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexedSymbolKind {
    Function,
    Constant,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSymbol {
    pub name: String,
    pub kind: IndexedSymbolKind,
    pub detail: String,
    pub exported: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectSymbolIndex {
    pub loaded: compiler::LoadedProject,
    pub ast: ast::Project,
    packages: BTreeMap<String, Vec<IndexedSymbol>>,
}

impl ProjectSymbolIndex {
    pub fn new(loaded: compiler::LoadedProject, ast: ast::Project) -> Self {
        let packages = ast
            .packages
            .iter()
            .map(|package| {
                let symbols = package
                    .files
                    .iter()
                    .flat_map(|file| file.decls.iter())
                    .flat_map(indexed_symbols_for_decl)
                    .collect();
                (package.logical_path.clone(), symbols)
            })
            .collect();
        Self {
            loaded,
            ast,
            packages,
        }
    }

    pub fn package_symbols(&self, logical_path: &str) -> &[IndexedSymbol] {
        self.packages
            .get(logical_path)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

fn indexed_symbols_for_decl(decl: &ast::TopLevelDecl) -> Vec<IndexedSymbol> {
    match decl {
        ast::TopLevelDecl::Const(decl) => decl
            .specs
            .iter()
            .flat_map(|spec| {
                spec.names.iter().map(|name| IndexedSymbol {
                    name: name.name.clone(),
                    kind: IndexedSymbolKind::Constant,
                    detail: spec.ty.as_ref().map_or_else(
                        || "const".to_string(),
                        |ty| format!("const {}: {}", name.name, type_name(ty)),
                    ),
                    exported: is_exported(&name.name),
                })
            })
            .collect(),
        ast::TopLevelDecl::Type(decl) => decl
            .specs
            .iter()
            .map(|spec| IndexedSymbol {
                name: spec.name.name.clone(),
                kind: IndexedSymbolKind::Type,
                detail: format!("type {} {}", spec.name.name, type_name(&spec.ty)),
                exported: is_exported(&spec.name.name),
            })
            .collect(),
        ast::TopLevelDecl::Func(decl) if decl.receiver.is_none() => vec![IndexedSymbol {
            name: decl.name.name.clone(),
            kind: IndexedSymbolKind::Function,
            detail: function_signature(decl),
            exported: is_exported(&decl.name.name),
        }],
        ast::TopLevelDecl::Func(_) => Vec::new(),
        ast::TopLevelDecl::Var(_) => Vec::new(),
        ast::TopLevelDecl::Operator(_) => Vec::new(),
    }
}

fn function_signature(function: &ast::FuncDecl) -> String {
    let params = function
        .signature
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name.name, type_name(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut signature = format!("{}({params})", function.name.name);
    match function.signature.results.as_slice() {
        [] => {}
        [result] => signature.push_str(&format!(" -> {}", type_name(result))),
        results => {
            let results = results.iter().map(type_name).collect::<Vec<_>>().join(", ");
            signature.push_str(&format!(" -> ({results})"));
        }
    }
    signature
}

fn type_name(ty: &ast::Type) -> String {
    match ty {
        ast::Type::Named(path) => path
            .segments
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join("."),
        ast::Type::Apply {
            base, arguments, ..
        } => format!(
            "{}[{}]",
            base.segments
                .iter()
                .map(|segment| segment.name.as_str())
                .collect::<Vec<_>>()
                .join("."),
            arguments
                .iter()
                .map(type_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ast::Type::Resolved(_, _) => "<resolved>".to_string(),
        ast::Type::Pointer(inner, _) => format!("*{}", type_name(inner)),
        ast::Type::Array { element, .. } => format!("[...]{}", type_name(element)),
        ast::Type::Struct { .. } => "struct".to_string(),
        ast::Type::Interface { .. } => "interface".to_string(),
        ast::Type::Func { .. } => "func".to_string(),
    }
}

fn is_exported(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
}
