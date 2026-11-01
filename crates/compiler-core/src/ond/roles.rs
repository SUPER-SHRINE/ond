use super::*;
use crate::roles::{RoleConfiguration, RoleRegion};
use std::collections::BTreeSet;

pub(crate) fn role_identifier(name: &str) -> bool {
    tokenize(FileId(0), name).is_ok_and(|tokens| {
        tokens.first().is_some_and(|token| {
            token.kind == TokenKind::Identifier
                && token.text == name
                && token.span.start == 0
                && token.span.end == name.len()
        }) && tokens
            .iter()
            .skip(1)
            .all(|token| matches!(token.kind, TokenKind::Semi | TokenKind::Eof))
    })
}

pub(crate) fn role_regions(
    file: FileId,
    text: &str,
    configuration: Option<&RoleConfiguration>,
) -> Result<Vec<RoleRegion>, Diagnostics> {
    let tokens = tokenize(file, text)?;
    let mut regions = Vec::new();
    let mut stack: Vec<Option<BTreeSet<String>>> = Vec::new();
    let mut cursor = 0;
    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if is_role_start(&tokens, cursor) {
            let Some(configuration) = configuration else {
                return Err(Diagnostics::from_single(Diagnostic::error(
                    token.span,
                    "project has no role definitions",
                )));
            };
            let mut names = BTreeSet::new();
            cursor += 1;
            loop {
                let token = &tokens[cursor];
                if token.kind != TokenKind::Identifier {
                    return Err(Diagnostics::from_single(Diagnostic::error(
                        token.span,
                        "expected role identifier",
                    )));
                }
                if !configuration.names.contains(&token.text) {
                    return Err(Diagnostics::from_single(Diagnostic::error(
                        token.span,
                        format!("unknown role `{}`", token.text),
                    )));
                }
                if !names.insert(token.text.clone()) {
                    return Err(Diagnostics::from_single(Diagnostic::error(
                        token.span,
                        "duplicate role name",
                    )));
                }
                cursor += 1;
                if tokens[cursor].kind != TokenKind::Comma {
                    break;
                }
                cursor += 1;
            }
            if tokens[cursor].kind != TokenKind::LBrace {
                return Err(Diagnostics::from_single(Diagnostic::error(
                    tokens[cursor].span,
                    "expected `{`",
                )));
            }
            let parent = stack
                .iter()
                .rev()
                .find_map(Option::as_ref)
                .unwrap_or(&configuration.names);
            let targets = parent
                .intersection(&names)
                .cloned()
                .collect::<BTreeSet<_>>();
            let close = matching_brace(&tokens, cursor)?;
            regions.push(RoleRegion {
                start: tokens[cursor].span.start,
                end: tokens[close].span.end,
                targets: targets.clone(),
            });
            // Normal brace handling below enters the branch. Context is obtained
            // from its region so nested role blocks use the same parent algebra.
        }
        match tokens[cursor].kind {
            TokenKind::LBrace => {
                let context = regions
                    .iter()
                    .rev()
                    .find(|region| region.start == tokens[cursor].span.start)
                    .map(|region| region.targets.clone());
                stack.push(context);
            }
            TokenKind::RBrace => {
                stack.pop();
            }
            _ => {}
        }
        cursor += 1;
    }
    // Source-order nesting makes the most deeply containing region last.
    regions.sort_by_key(|region| (region.start, std::cmp::Reverse(region.end)));
    Ok(regions)
}

fn matching_brace(tokens: &[Token], start: usize) -> Result<usize, Diagnostics> {
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.kind {
            TokenKind::LBrace => depth += 1,
            TokenKind::RBrace => {
                depth -= 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }
    Err(Diagnostics::from_single(Diagnostic::error(
        tokens[start].span,
        "unclosed role block",
    )))
}

fn is_role_start(tokens: &[Token], cursor: usize) -> bool {
    tokens[cursor].kind == TokenKind::Identifier
        && tokens[cursor].text == "role"
        && tokens.get(cursor + 1).is_some_and(|token| {
            matches!(
                token.kind,
                TokenKind::Identifier | TokenKind::LBrace | TokenKind::Comma
            )
        })
        && (cursor == 0
            || matches!(
                tokens[cursor - 1].kind,
                TokenKind::Semi | TokenKind::LBrace | TokenKind::RBrace
            ))
}

pub fn parse_snapshot_file(
    sources: &crate::source::SourceDb,
    file: FileId,
) -> Result<ast::File, Diagnostics> {
    parse_context_file(sources, file, false)
}

pub fn parse_context_file(
    sources: &crate::source::SourceDb,
    file: FileId,
    all: bool,
) -> Result<ast::File, Diagnostics> {
    let source = sources.file(file);
    let tokens = tokenize(file, source.text())?;
    let mut roles = sources.roles(file).cloned();
    let regions = role_regions(
        file,
        source.text(),
        roles
            .as_ref()
            .and_then(|roles| roles.configuration.as_ref()),
    )?;
    if let Some(roles) = &mut roles {
        roles.regions = regions;
    }
    let mut parser = Parser::new(file, tokens, source.text());
    parser.roles = roles;
    parser.all_roles = all;
    parser.parse_file()
}

impl Parser {
    pub(super) fn role_start(&self) -> bool {
        is_role_start(&self.tokens, self.cursor)
    }
    fn role_selected(&self, span: Span) -> bool {
        self.all_roles
            || self
                .roles
                .as_ref()
                .is_none_or(|roles| roles.selected(span.start))
    }
    fn role_header(&mut self) -> Result<(), Diagnostics> {
        self.bump();
        self.expect_ident()?;
        while self.at(TokenKind::Comma) {
            self.bump();
            self.expect_ident()?;
        }
        Ok(())
    }
    pub(super) fn source_items(
        &mut self,
        imports: &mut Vec<ast::ImportDecl>,
        decls: &mut Vec<ast::TopLevelDecl>,
        nested: bool,
    ) -> Result<(), Diagnostics> {
        while !(self.at(TokenKind::Eof) || nested && self.at(TokenKind::RBrace)) {
            if self.role_start() {
                self.role_header()?;
                self.expect(TokenKind::LBrace, "`{`")?;
                self.source_items(imports, decls, true)?;
                self.expect(TokenKind::RBrace, "`}`")?;
            } else if self.at(TokenKind::Import) {
                let start = self.bump().span;
                let alias = self
                    .at(TokenKind::Identifier)
                    .then(|| self.expect_ident())
                    .transpose()?;
                let path = self.expect_string_lit()?;
                let end = self.expect_semi()?;
                // Keep every import until projection-order validation below.
                imports.push(ast::ImportDecl {
                    alias,
                    path,
                    span: join_spans(start, end),
                });
                continue;
            } else {
                decls.push(self.parse_top_level_decl()?);
            }
            self.expect_semi()?;
        }
        Ok(())
    }
    pub(super) fn validate_import_order(
        &self,
        imports: &[ast::ImportDecl],
        decls: &[ast::TopLevelDecl],
    ) -> Result<(), Diagnostics> {
        for import in imports {
            for declaration in decls {
                let span = declaration_span(declaration);
                if span.start < import.span.start && self.contexts_overlap(span, import.span) {
                    return Err(Diagnostics::from_single(Diagnostic::error(
                        import.span,
                        "import must precede declarations in every role projection",
                    )));
                }
            }
        }
        Ok(())
    }
    fn contexts_overlap(&self, a: Span, b: Span) -> bool {
        let Some(roles) = &self.roles else {
            return true;
        };
        match (roles.context(a.start), roles.context(b.start)) {
            (Some(a), Some(b)) => !a.is_disjoint(b),
            (Some(targets), None) | (None, Some(targets)) => !targets.is_empty(),
            _ => true,
        }
    }
    pub(super) fn project_items(
        &self,
        imports: &mut Vec<ast::ImportDecl>,
        decls: &mut Vec<ast::TopLevelDecl>,
    ) {
        imports.retain(|import| self.role_selected(import.span));
        decls.retain(|decl| self.role_selected(declaration_span(decl)));
    }
    pub(super) fn role_statement(&mut self) -> Result<ast::Stmt, Diagnostics> {
        self.role_header()?;
        let mut block = self.parse_block()?;
        if !self.role_selected(block.span) {
            block.statements.clear();
        }
        Ok(ast::Stmt::Block(block))
    }
}

pub fn declaration_span(decl: &ast::TopLevelDecl) -> Span {
    match decl {
        ast::TopLevelDecl::Const(d) => d.span,
        ast::TopLevelDecl::Var(d) => d.span,
        ast::TopLevelDecl::Type(d) => d.span,
        ast::TopLevelDecl::Func(d) => d.span,
        ast::TopLevelDecl::Operator(d) => d.span,
    }
}
