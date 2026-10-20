//! Source requests survive value folding and are checked separately from emission.
use super::types::{GenericSpecialization, TypePackage};
use crate::{diagnostic::Diagnostic, semantic::TypeId, source::Span};
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Request {
    pub(super) caller: Option<String>,
    pub(super) symbol: String,
    pub(super) span: Span,
}

pub(super) fn requests(universe: &[TypePackage]) -> Vec<Request> {
    universe
        .iter()
        .flat_map(|p| p.generic_requests.borrow().clone())
        .collect()
}

fn identity(symbol: &str, universe: &[TypePackage]) -> Option<(Span, Vec<TypeId>)> {
    universe.iter().find_map(|package| {
        let specialization = package.specializations.borrow().get(symbol)?.clone();
        match specialization {
            GenericSpecialization::Function { name, arguments } => {
                Some((package.generic_functions.get(&name)?.name.span, arguments))
            }
            GenericSpecialization::Method {
                declaration,
                arguments,
            } => Some((package.methods.get(declaration)?.name.span, arguments)),
            GenericSpecialization::Operator {
                declaration,
                arguments,
            } => {
                let declaration = package.operators.get(declaration)?;
                Some((declaration.signature.params.first()?.name.span, arguments))
            }
        }
    })
}

pub(super) fn validate(
    requests: &[Request],
    universe: &[TypePackage],
) -> Option<(Request, Diagnostic)> {
    let mut edges = BTreeMap::<&str, Vec<&Request>>::new();
    let mut nodes = BTreeMap::new();
    let mut variants = BTreeMap::<_, std::collections::BTreeSet<Vec<TypeId>>>::new();
    for request in requests {
        if let Some(caller) = &request.caller {
            edges.entry(caller).or_default().push(request);
        }
        if let Some((declaration, arguments)) = identity(&request.symbol, universe) {
            variants
                .entry((declaration.file.0, declaration.start, declaration.end))
                .or_default()
                .insert(arguments.clone());
            nodes.insert(request.symbol.as_str(), (declaration, arguments));
        }
    }
    let mut reachable = BTreeMap::new();
    let mut pending = requests
        .iter()
        .filter(|r| r.caller.is_none())
        .map(|root| (root, root.span))
        .collect::<Vec<_>>();
    while let Some((request, root)) = pending.pop() {
        if reachable.insert(request.symbol.as_str(), root).is_some() {
            continue;
        }
        for child in edges.get(request.symbol.as_str()).into_iter().flatten() {
            pending.push((child, root));
        }
    }
    // A violating path contains two instances of the same declaration with
    // different arguments. Check reachability between those instances rather
    // than enumerate all paths through dense recursive call graphs. Revisiting
    // the exact same instance only adds a removable cycle to such a path.
    for (symbol, (declaration, arguments)) in &nodes {
        let Some(root) = reachable.get(symbol).copied() else {
            continue;
        };
        if variants[&(declaration.file.0, declaration.start, declaration.end)].len() < 2 {
            continue;
        }
        let mut visited = std::collections::BTreeSet::new();
        let mut pending = edges
            .get(symbol)
            .into_iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        while let Some(request) = pending.pop() {
            if !visited.insert(request.symbol.as_str()) {
                continue;
            }
            if let Some((target_declaration, target_arguments)) = nodes.get(request.symbol.as_str())
                && declaration == target_declaration
                && arguments != target_arguments
            {
                return Some((
                    request.clone(),
                    Diagnostic::error(
                        request.span,
                        "generic instantiation changes type arguments on the same path",
                    )
                    .with_related(
                        *declaration,
                        "generic declaration first requested on this path",
                    )
                    .reanchor(root, "nested generic request changes the arguments"),
                ));
            }
            pending.extend(
                edges
                    .get(request.symbol.as_str())
                    .into_iter()
                    .flatten()
                    .copied(),
            );
        }
    }
    None
}
