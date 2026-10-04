struct Parser {
    file_id: FileId,
    tokens: Vec<Token>,
    cursor: usize,
    diagnostics: Diagnostics,
    header: HeaderMode,
}

impl Parser {
    fn new(file_id: FileId, tokens: Vec<Token>) -> Self {
        Self {
            file_id,
            tokens,
            cursor: 0,
            diagnostics: Diagnostics::new(),
            header: HeaderMode::None,
        }
    }

    fn parse_file(mut self) -> Result<ast::File, Diagnostics> {
        let package_span = self.expect(TokenKind::Package, "`package`")?;
        let package_name = self.expect_ident()?;
        self.expect_semi()?;

        let mut imports = Vec::new();
        while self.at(TokenKind::Import) {
            let start = self.bump().span;
            let alias = self.at(TokenKind::Identifier).then(|| self.expect_ident()).transpose()?;
            let path = self.expect_string_lit()?;
            let end = self.expect_semi()?;
            imports.push(ast::ImportDecl {
                alias,
                path,
                span: join_spans(start, end),
            });
        }

        let mut decls = Vec::new();
        while !self.at(TokenKind::Eof) {
            decls.push(self.parse_top_level_decl()?);
            self.expect_semi()?;
        }

        let end = self.current().span;
        if self.diagnostics.is_empty() {
            Ok(ast::File {
                file_id: self.file_id,
                span: Span::new(self.file_id, 0, end.end),
                package: ast::PackageClause {
                    name: package_name.name,
                    span: join_spans(package_span, package_name.span),
                },
                imports,
                decls,
            })
        } else {
            Err(self.diagnostics)
        }
    }

    fn parse_top_level_decl(&mut self) -> Result<ast::TopLevelDecl, Diagnostics> {
        if self.at(TokenKind::Identifier) && self.current().text == "operator" {
            return Ok(ast::TopLevelDecl::Operator(self.parse_operator_decl()?));
        }
        let decl = match self.current().kind {
            TokenKind::Const => ast::TopLevelDecl::Const(self.parse_const_decl()?),
            TokenKind::Var => ast::TopLevelDecl::Var(self.parse_var_decl()?),
            TokenKind::Type => ast::TopLevelDecl::Type(self.parse_type_decl()?),
            TokenKind::Func => ast::TopLevelDecl::Func(self.parse_func_decl()?),
            _ => return Err(self.error_here("expected top-level declaration")),
        };
        Ok(decl)
    }

    fn parse_operator_decl(&mut self) -> Result<ast::OperatorDecl, Diagnostics> {
        let start = self.bump().span;
        let type_params = self.parse_type_parameters(start)?;
        let name = match self.current().kind {
            TokenKind::Plus => {
                self.bump();
                ast::OperatorName::Add
            }
            TokenKind::Minus => {
                self.bump();
                ast::OperatorName::Sub
            }
            TokenKind::Star => {
                self.bump();
                ast::OperatorName::Mul
            }
            TokenKind::Slash => {
                self.bump();
                ast::OperatorName::Div
            }
            TokenKind::Percent => {
                self.bump();
                ast::OperatorName::Rem
            }
            TokenKind::LBracket => {
                self.bump();
                self.expect(TokenKind::RBracket, "`]`")?;
                if self.at(TokenKind::Assign) {
                    self.bump();
                    ast::OperatorName::IndexSet
                } else {
                    ast::OperatorName::Index
                }
            }
            TokenKind::Identifier if self.current().text == "len" => {
                self.bump();
                ast::OperatorName::Len
            }
            _ => return Err(self.error_here("expected overloadable operator")),
        };
        let signature = self.parse_signature()?;
        let body = self.parse_block()?;
        Ok(ast::OperatorDecl {
            name,
            type_params,
            signature,
            span: join_spans(start, body.span),
            body,
        })
    }

    fn parse_const_decl(&mut self) -> Result<ast::ConstDecl, Diagnostics> {
        let start = self.bump().span;
        if self.at(TokenKind::LParen) {
            self.bump();
            let mut specs = Vec::new();
            while !self.at(TokenKind::RParen) {
                specs.push(self.parse_const_spec()?);
                self.expect_semi()?;
            }
            let end = self.expect(TokenKind::RParen, "`)`")?;
            Ok(ast::ConstDecl {
                specs,
                span: join_spans(start, end),
            })
        } else {
            let spec = self.parse_const_spec()?;
            Ok(ast::ConstDecl {
                span: join_spans(start, spec.span),
                specs: vec![spec],
            })
        }
    }

    fn parse_const_spec(&mut self) -> Result<ast::ConstSpec, Diagnostics> {
        let names = self.parse_identifier_list()?;
        let mut end = names
            .last()
            .map(|ident| ident.span)
            .unwrap_or(Span::synthetic());
        let ty = if self.at(TokenKind::Colon) {
            self.bump();
            let ty = self.parse_type()?;
            end = type_span(&ty);
            Some(ty)
        } else {
            None
        };
        self.expect(TokenKind::Assign, "`=`")?;
        let values = self.parse_expression_list()?;
        if let Some(value) = values.last() {
            end = value.span();
        }
        Ok(ast::ConstSpec {
            span: join_spans(names[0].span, end),
            names,
            ty,
            values,
        })
    }

    fn parse_var_decl(&mut self) -> Result<ast::VarDecl, Diagnostics> {
        let start = self.bump().span;
        if self.at(TokenKind::LParen) {
            self.bump();
            let mut specs = Vec::new();
            while !self.at(TokenKind::RParen) {
                specs.push(self.parse_var_spec()?);
                self.expect_semi()?;
            }
            let end = self.expect(TokenKind::RParen, "`)`")?;
            Ok(ast::VarDecl {
                specs,
                span: join_spans(start, end),
            })
        } else {
            let spec = self.parse_var_spec()?;
            Ok(ast::VarDecl {
                span: join_spans(start, spec.span),
                specs: vec![spec],
            })
        }
    }

    fn parse_var_spec(&mut self) -> Result<ast::VarSpec, Diagnostics> {
        let names = self.parse_identifier_list()?;
        let mut ty = None;
        let mut values = Vec::new();
        let mut end = names
            .last()
            .map(|ident| ident.span)
            .unwrap_or(Span::synthetic());
        if self.at(TokenKind::Colon) {
            self.bump();
            let parsed = self.parse_type()?;
            end = type_span(&parsed);
            ty = Some(parsed);
        }
        if self.at(TokenKind::Assign) {
            self.bump();
            values = self.parse_expression_list()?;
            if let Some(value) = values.last() {
                end = value.span();
            }
        }
        Ok(ast::VarSpec {
            span: join_spans(names[0].span, end),
            names,
            ty,
            values,
        })
    }

    fn parse_type_decl(&mut self) -> Result<ast::TypeDecl, Diagnostics> {
        let start = self.bump().span;
        if self.at(TokenKind::LParen) {
            self.bump();
            let mut specs = Vec::new();
            while !self.at(TokenKind::RParen) {
                specs.push(self.parse_type_spec()?);
                self.expect_semi()?;
            }
            let end = self.expect(TokenKind::RParen, "`)`")?;
            Ok(ast::TypeDecl {
                specs,
                span: join_spans(start, end),
            })
        } else {
            let spec = self.parse_type_spec()?;
            Ok(ast::TypeDecl {
                span: join_spans(start, spec.span),
                specs: vec![spec],
            })
        }
    }

    fn parse_type_spec(&mut self) -> Result<ast::TypeSpec, Diagnostics> {
        let name = self.expect_binding_ident()?;
        let type_params = self.parse_type_parameters(name.span)?;
        let ty = self.parse_type()?;
        Ok(ast::TypeSpec {
            span: join_spans(name.span, type_span(&ty)),
            name,
            type_params,
            ty,
        })
    }

    fn parse_func_decl(&mut self) -> Result<ast::FuncDecl, Diagnostics> {
        let start = self.bump().span;
        let receiver = if self.at(TokenKind::LParen) {
            self.bump();
            let name = self.expect_binding_ident()?;
            self.expect(TokenKind::Colon, "`:`")?;
            let ty = self.parse_type()?;
            let span = join_spans(name.span, type_span(&ty));
            self.expect(TokenKind::RParen, "`)`")?;
            Some(Box::new(ast::Field { name, ty, span }))
        } else {
            None
        };
        let name = self.expect_binding_ident()?;
        let type_params = self.parse_type_parameters(name.span)?;
        let signature = self.parse_signature()?;
        let body = self.parse_block()?;
        Ok(ast::FuncDecl {
            span: join_spans(start, body.span),
            receiver,
            name,
            type_params,
            signature,
            body,
        })
    }

    fn parse_type(&mut self) -> Result<ast::Type, Diagnostics> {
        match self.current().kind {
            TokenKind::Identifier => {
                let path = self.parse_path()?;
                if self.at(TokenKind::LBracket) {
                    let start = path.span;
                    self.bump();
                    let mut arguments = Vec::new();
                    loop {
                        arguments.push(self.parse_type()?);
                        if !self.at(TokenKind::Comma) {
                            break;
                        }
                        self.bump();
                    }
                    let end = self.expect(TokenKind::RBracket, "`]`")?;
                    Ok(ast::Type::Apply {
                        base: path,
                        arguments,
                        span: join_spans(start, end),
                    })
                } else {
                    Ok(ast::Type::Named(path))
                }
            }
            TokenKind::Star => {
                let start = self.bump().span;
                let inner = self.parse_type()?;
                let end = type_span(&inner);
                Ok(ast::Type::Pointer(Box::new(inner), join_spans(start, end)))
            }
            TokenKind::LBracket => {
                let start = self.bump().span;
                let len = if self.at(TokenKind::Ellipsis) {
                    self.bump();
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                };
                self.expect(TokenKind::RBracket, "`]`")?;
                let element = self.parse_type()?;
                Ok(ast::Type::Array {
                    span: join_spans(start, type_span(&element)),
                    len,
                    element: Box::new(element),
                })
            }
            TokenKind::Struct => {
                let start = self.bump().span;
                self.expect(TokenKind::LBrace, "`{`")?;
                let mut fields = Vec::new();
                while !self.at(TokenKind::RBrace) {
                    let name = self.expect_ident()?;
                    self.expect(TokenKind::Colon, "`:`")?;
                    let ty = self.parse_type()?;
                    let span = join_spans(name.span, type_span(&ty));
                    fields.push(ast::Field { name, ty, span });
                    self.expect_semi()?;
                }
                let end = self.expect(TokenKind::RBrace, "`}`")?;
                Ok(ast::Type::Struct {
                    fields,
                    span: join_spans(start, end),
                })
            }
            TokenKind::Interface => {
                let start = self.bump().span;
                self.expect(TokenKind::LBrace, "`{`")?;
                let mut methods = Vec::new();
                while !self.at(TokenKind::RBrace) {
                    let name = self.expect_ident()?;
                    let signature = self.parse_type_signature(false)?;
                    let span = join_spans(name.span, signature.span);
                    methods.push(ast::InterfaceMethod {
                        name,
                        signature,
                        span,
                    });
                    self.expect_semi()?;
                }
                let end = self.expect(TokenKind::RBrace, "`}`")?;
                Ok(ast::Type::Interface {
                    methods,
                    span: join_spans(start, end),
                })
            }
            TokenKind::Func => {
                let start = self.bump().span;
                let signature = self.parse_type_signature(false)?;
                Ok(ast::Type::Func {
                    span: join_spans(start, signature.span),
                    signature,
                })
            }
            _ => Err(self.error_here("expected type")),
        }
    }

    fn parse_type_parameters(&mut self, name: Span) -> Result<Vec<ast::Ident>, Diagnostics> {
        // Adjacency distinguishes `type Box[T] ...` from the pre-existing
        // `type Buffer [N]u8` array declaration.  Without it both forms have an
        // identical token sequence when the array length is an identifier.
        if !self.at(TokenKind::LBracket)
            || self.current().span.start != name.end
            || self.peek_kind(1) != Some(TokenKind::Identifier)
            || !matches!(
                self.peek_kind(2),
                Some(TokenKind::Comma | TokenKind::RBracket)
            )
        {
            return Ok(Vec::new());
        }
        self.bump();
        let mut parameters = Vec::new();
        loop {
            let parameter = self.expect_binding_ident()?;
            if parameters
                .iter()
                .any(|existing: &ast::Ident| existing.name == parameter.name)
            {
                return Err(Diagnostics::from_single(Diagnostic::error(
                    parameter.span,
                    format!("duplicate type parameter `{}`", parameter.name),
                )));
            }
            parameters.push(parameter);
            if !self.at(TokenKind::Comma) {
                break;
            }
            self.bump();
        }
        self.expect(TokenKind::RBracket, "`]`")?;
        Ok(parameters)
    }

    fn parse_block(&mut self) -> Result<ast::Block, Diagnostics> {
        let start = self.expect(TokenKind::LBrace, "`{`")?;
        let mut statements = Vec::new();
        while !self.at(TokenKind::RBrace) {
            if self.at(TokenKind::Semi) {
                self.bump();
                continue;
            }
            statements.push(self.parse_statement()?);
            self.expect_semi()?;
        }
        let end = self.expect(TokenKind::RBrace, "`}`")?;
        Ok(ast::Block {
            statements,
            span: join_spans(start, end),
        })
    }

    fn parse_statement(&mut self) -> Result<ast::Stmt, Diagnostics> {
        match self.current().kind {
            TokenKind::Const => Ok(ast::Stmt::Const(self.parse_const_decl()?)),
            TokenKind::Var => Ok(ast::Stmt::Var(self.parse_var_decl()?)),
            TokenKind::Return => {
                let start = self.bump().span;
                let values = if self.at(TokenKind::Semi) || self.at(TokenKind::RBrace) {
                    Vec::new()
                } else {
                    self.parse_expression_list()?
                };
                let end = values.last().map(ast::Expr::span).unwrap_or(start);
                Ok(ast::Stmt::Return(ast::ReturnStmt {
                    span: join_spans(start, end),
                    values,
                }))
            }
            TokenKind::Defer => {
                let start = self.bump().span;
                let block = self.parse_block()?;
                Ok(ast::Stmt::Defer(ast::DeferStmt {
                    span: join_spans(start, block.span),
                    block,
                }))
            }
            TokenKind::Identifier
                if self.current().text == "trap"
                    && self.peek_kind(1) == Some(TokenKind::StringLit) =>
            {
                let start = self.bump().span;
                let message = self.bump();
                let decoded = crate::string_literal::decode(&message.text)
                    .and_then(|bytes| {
                        String::from_utf8(bytes)
                            .map_err(|_| "trap message must be valid UTF-8".to_string())
                    })
                    .map_err(|error| {
                        Diagnostics::from_single(Diagnostic::error(message.span, error))
                    })?;
                Ok(ast::Stmt::Trap {
                    message: decoded,
                    span: join_spans(start, message.span),
                })
            }
            TokenKind::Break => {
                let span = self.bump().span;
                Ok(ast::Stmt::Break(span))
            }
            TokenKind::Continue => {
                let span = self.bump().span;
                Ok(ast::Stmt::Continue(span))
            }
            TokenKind::If => Ok(ast::Stmt::If(self.parse_if_stmt()?)),
            TokenKind::For => Ok(ast::Stmt::For(self.parse_for_stmt()?)),
            TokenKind::LBrace => Ok(ast::Stmt::Block(self.parse_block()?)),
            _ => checked_simple_statement(self.parse_simple_stmt()?),
        }
    }

    fn parse_if_stmt(&mut self) -> Result<ast::IfStmt, Diagnostics> {
        let start = self.bump().span;
        let first = if self.at(TokenKind::Semi) {
            None
        } else {
            Some(self.in_header(HeaderMode::Init, |p| p.parse_simple_or_expr())?)
        };
        let (init, condition) = if self.at(TokenKind::Semi) {
            self.bump();
            let condition = self.in_header(HeaderMode::Body, |p| p.parse_expression())?;
            (
                first
                    .map(|first| first.into_stmt().map(Box::new))
                    .transpose()?,
                condition,
            )
        } else {
            (
                None,
                first
                    .ok_or_else(|| self.error_here("expected condition"))?
                    .into_expr()?,
            )
        };
        let then_block = self.parse_block()?;
        let else_branch = if self.at(TokenKind::Else) {
            self.bump();
            Some(Box::new(match self.current().kind {
                TokenKind::If => ast::Stmt::If(self.parse_if_stmt()?),
                TokenKind::LBrace => ast::Stmt::Block(self.parse_block()?),
                _ => return Err(self.error_here("expected `if` or block after `else`")),
            }))
        } else {
            None
        };
        let end = else_branch
            .as_ref()
            .map(|stmt| stmt_span(stmt))
            .unwrap_or(then_block.span);
        Ok(ast::IfStmt {
            init,
            condition,
            then_block,
            else_branch,
            span: join_spans(start, end),
        })
    }

    fn parse_for_stmt(&mut self) -> Result<ast::ForStmt, Diagnostics> {
        let start = self.bump().span;
        let kind = if self.at(TokenKind::LBrace) {
            ast::ForKind::Infinite
        } else {
            let first = if self.at(TokenKind::Semi) {
                None
            } else {
                Some(self.in_header(HeaderMode::Init, |p| p.parse_simple_or_expr())?)
            };
            if self.at(TokenKind::Semi) {
                self.bump();
                let condition = if self.at(TokenKind::Semi) {
                    None
                } else {
                    Some(self.in_header(HeaderMode::Body, |p| p.parse_expression())?)
                };
                self.expect(TokenKind::Semi, "`;`")?;
                let post = if self.at(TokenKind::LBrace) {
                    None
                } else {
                    Some(Box::new(checked_simple_statement(
                        self.in_header(HeaderMode::Body, |p| p.parse_simple_stmt())?,
                    )?))
                };
                ast::ForKind::ThreeClause {
                    init: first
                        .map(|first| first.into_stmt().map(Box::new))
                        .transpose()?,
                    condition,
                    post,
                }
            } else if let Some(SimpleOrExpr::Expr(condition)) = first {
                ast::ForKind::While(condition)
            } else {
                return Err(self.error_here("expected loop condition or `;`"));
            }
        };
        let body = self.parse_block()?;
        Ok(ast::ForStmt {
            span: join_spans(start, body.span),
            kind,
            body,
        })
    }

    fn parse_simple_stmt(&mut self) -> Result<ast::Stmt, Diagnostics> {
        let exprs = self.parse_expression_list()?;
        if self.at(TokenKind::ColonAssign) {
            let span = self.bump().span;
            let values = self.header_values()?;
            let mut names = Vec::with_capacity(exprs.len());
            for expr in exprs {
                match expr {
                    ast::Expr::Name(path) if path.segments.len() == 1 => {
                        let name = path.segments.into_iter().next().unwrap();
                        Self::check_binding_ident(&name)?;
                        names.push(name)
                    }
                    other => {
                        return Err(Diagnostics::from_single(Diagnostic::error(
                            other.span(),
                            "left-hand side of `:=` must be identifier",
                        )));
                    }
                }
            }
            Ok(ast::Stmt::ShortVar(ast::ShortVarDecl {
                span: join_spans(
                    names[0].span,
                    values.last().map(ast::Expr::span).unwrap_or(span),
                ),
                names,
                values,
            }))
        } else if self.at(TokenKind::Assign) {
            self.bump();
            let values = self.header_values()?;
            let start = exprs[0].span();
            let end = values.last().map(ast::Expr::span).unwrap_or(start);
            Ok(ast::Stmt::Assign(ast::AssignStmt {
                span: join_spans(start, end),
                targets: exprs,
                values,
                op: None,
                implicit_one: false,
            }))
        } else if let Some(op) = current_assignment(self.current().kind) {
            if exprs.len() != 1 {
                return Err(self.error_here("compound assignment requires one target"));
            }
            self.bump();
            let value = self.parse_expression()?;
            let start = exprs[0].span();
            let end = value.span();
            Ok(ast::Stmt::Assign(ast::AssignStmt {
                span: join_spans(start, end),
                targets: exprs,
                values: vec![value],
                op: Some(op),
                implicit_one: false,
            }))
        } else if matches!(self.current().kind, TokenKind::Increment | TokenKind::Decrement) {
            if exprs.len() != 1 {
                return Err(self.error_here("increment or decrement requires one target"));
            }
            let token = self.bump();
            let op = if token.kind == TokenKind::Increment {
                ast::BinaryOp::Add
            } else {
                ast::BinaryOp::Sub
            };
            let start = exprs[0].span();
            Ok(ast::Stmt::Assign(ast::AssignStmt {
                span: join_spans(start, token.span),
                targets: exprs,
                values: Vec::new(),
                op: Some(op),
                implicit_one: true,
            }))
        } else if exprs.len() == 1 {
            let expr = exprs.into_iter().next().unwrap();
            let span = expr.span();
            Ok(ast::Stmt::Expr(ast::ExprStmt { expr, span }))
        } else {
            Err(self.error_here("expected assignment or single expression statement"))
        }
    }

    fn parse_simple_or_expr(&mut self) -> Result<SimpleOrExpr, Diagnostics> {
        let checkpoint = self.cursor;
        match self.parse_simple_stmt() {
            Ok(ast::Stmt::Expr(expr_stmt)) => Ok(SimpleOrExpr::Expr(expr_stmt.expr)),
            Ok(stmt) => Ok(SimpleOrExpr::Stmt(stmt)),
            Err(_) => {
                self.cursor = checkpoint;
                self.parse_expression().map(SimpleOrExpr::Expr)
            }
        }
    }

    fn parse_expression_list(&mut self) -> Result<Vec<ast::Expr>, Diagnostics> {
        let mut values = vec![self.parse_expression()?];
        while self.at(TokenKind::Comma) {
            self.bump();
            values.push(self.parse_expression()?);
        }
        Ok(values)
    }

    fn parse_expression(&mut self) -> Result<ast::Expr, Diagnostics> {
        self.parse_binary_expr(1)
    }

    fn parse_binary_expr(&mut self, min_prec: u8) -> Result<ast::Expr, Diagnostics> {
        let mut lhs = self.parse_unary_expr()?;
        loop {
            let Some((op, prec)) = current_binary(self.current().kind) else {
                break;
            };
            if prec < min_prec {
                break;
            }
            self.bump();
            let rhs = self.parse_binary_expr(prec + 1)?;
            let span = join_spans(lhs.span(), rhs.span());
            lhs = ast::Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_unary_expr(&mut self) -> Result<ast::Expr, Diagnostics> {
        let Some(op) = current_unary(self.current().kind) else {
            return self.parse_postfix_expr();
        };
        let span = self.bump().span;
        let expr = self.parse_unary_expr()?;
        let end = expr.span();
        Ok(ast::Expr::Unary {
            op,
            span: join_spans(span, end),
            expr: Box::new(expr),
        })
    }

    fn parse_postfix_expr(&mut self) -> Result<ast::Expr, Diagnostics> {
        let mut expr = self.parse_primary_expr()?;
        loop {
            match self.current().kind {
                TokenKind::LParen => {
                    let start = expr.span();
                    self.bump();
                    let mut args = Vec::new();
                    if !self.at(TokenKind::RParen) {
                        loop {
                            args.push(self.in_header(HeaderMode::None, |p| p.parse_expression())?);
                            if !self.at(TokenKind::Comma) {
                                break;
                            }
                            self.bump();
                            if self.at(TokenKind::RParen) {
                                break;
                            }
                        }
                    }
                    let end = self.expect(TokenKind::RParen, "`)`")?;
                    expr = ast::Expr::Call {
                        span: join_spans(start, end),
                        callee: Box::new(expr),
                        args,
                    };
                }
                TokenKind::LBracket => {
                    let start = expr.span();
                    let type_application = matches!(&expr, ast::Expr::Name(_)).then(|| {
                        let saved = self.cursor;
                        self.bump();
                        let mut arguments = Vec::new();
                        loop {
                            let Ok(argument) = self.parse_type() else {
                                self.cursor = saved;
                                return None;
                            };
                            arguments.push(argument);
                            if !self.at(TokenKind::Comma) {
                                break;
                            }
                            self.bump();
                        }
                        if !self.at(TokenKind::RBracket) {
                            self.cursor = saved;
                            return None;
                        }
                        let end = self.bump().span;
                        let allowed = matches!(&expr, ast::Expr::Name(path)
                            if matches!(path.segments.as_slice(), [name] if name.name == "alloc"))
                            || self.at(TokenKind::LParen)
                            || self.named_composite_literal_starts();
                        if !allowed {
                            self.cursor = saved;
                            return None;
                        }
                        Some((arguments, end))
                    });
                    if let Some((arguments, end)) = type_application.flatten() {
                        if self.at(TokenKind::LBrace) {
                            let ast::Expr::Name(path) = expr else { unreachable!() };
                            expr = self.parse_composite_literal(ast::Type::Apply {
                                base: path,
                                arguments,
                                span: join_spans(start, end),
                            })?;
                            continue;
                        }
                        expr = ast::Expr::TypeApply {
                            span: join_spans(start, end),
                            base: Box::new(expr),
                            arguments,
                        };
                    } else {
                        self.bump();
                        let index = self.in_header(HeaderMode::None, |p| p.parse_expression())?;
                        let end = self.expect(TokenKind::RBracket, "`]`")?;
                        expr = ast::Expr::Index {
                            span: join_spans(start, end),
                            base: Box::new(expr),
                            index: Box::new(index),
                        };
                    }
                }
                TokenKind::Dot => {
                    let start = expr.span();
                    self.bump();
                    let field = self.expect_ident()?;
                    expr = ast::Expr::Selector {
                        span: join_spans(start, field.span),
                        base: Box::new(expr),
                        field,
                    };
                }
                TokenKind::As => {
                    let start = expr.span();
                    self.bump();
                    let ty = self.parse_type()?;
                    expr = ast::Expr::Cast {
                        span: join_spans(start, type_span(&ty)),
                        expr: Box::new(expr),
                        ty,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary_expr(&mut self) -> Result<ast::Expr, Diagnostics> {
        match self.current().kind {
            TokenKind::Identifier => {
                if matches!(self.current().text.as_str(), "sizeof" | "alignof")
                    && self.peek_kind(1) == Some(TokenKind::LParen)
                {
                    let token = self.bump();
                    self.bump();
                    let ty = self.parse_type()?;
                    let end = self.expect(TokenKind::RParen, "`)`")?;
                    return Ok(ast::Expr::LayoutQuery {
                        ty,
                        alignment: token.text == "alignof",
                        span: join_spans(token.span, end),
                    });
                }
                let path = self.parse_path()?;
                if self.at(TokenKind::LBrace) && self.named_composite_literal_starts() {
                    self.parse_composite_literal(ast::Type::Named(path))
                } else {
                    Ok(ast::Expr::Name(path))
                }
            }
            TokenKind::IntLit => {
                let token = self.bump();
                Ok(ast::Expr::Literal(ast::Literal::Int(
                    token.text, token.span,
                )))
            }
            TokenKind::FloatLit => {
                let token = self.bump();
                Ok(ast::Expr::Literal(ast::Literal::Float(
                    token.text, token.span,
                )))
            }
            TokenKind::StringLit => {
                let token = self.bump();
                Ok(ast::Expr::Literal(ast::Literal::String(
                    token.text, token.span,
                )))
            }
            TokenKind::True => {
                let span = self.bump().span;
                Ok(ast::Expr::Literal(ast::Literal::Bool(true, span)))
            }
            TokenKind::False => {
                let span = self.bump().span;
                Ok(ast::Expr::Literal(ast::Literal::Bool(false, span)))
            }
            TokenKind::Nil => {
                let span = self.bump().span;
                Ok(ast::Expr::Literal(ast::Literal::Nil(span)))
            }
            TokenKind::LParen => {
                self.bump();
                let expr = self.in_header(HeaderMode::None, |p| p.parse_expression())?;
                self.expect(TokenKind::RParen, "`)`")?;
                Ok(expr)
            }
            TokenKind::New => {
                let start = self.bump().span;
                self.expect(TokenKind::LParen, "`(`")?;
                let ty = self.parse_type()?;
                let end = self.expect(TokenKind::RParen, "`)`")?;
                Ok(ast::Expr::New {
                    span: join_spans(start, end),
                    ty,
                })
            }
            TokenKind::LBracket => {
                let ty = self.parse_type()?;
                self.parse_composite_literal(ty)
            }
            _ => Err(self.error_here("expected expression")),
        }
    }

    fn parse_composite_literal(&mut self, ty: ast::Type) -> Result<ast::Expr, Diagnostics> {
        let start = type_span(&ty);
        self.expect(TokenKind::LBrace, "`{`")?;
        let mut elements = Vec::new();
        if !self.at(TokenKind::RBrace) {
            loop {
                let first = self.in_header(HeaderMode::None, |p| p.parse_expression())?;
                let (key, value, span) = if self.at(TokenKind::Colon) {
                    self.bump();
                    let value = self.in_header(HeaderMode::None, |p| p.parse_expression())?;
                    let span = join_spans(first.span(), value.span());
                    (Some(first), value, span)
                } else {
                    let span = first.span();
                    (None, first, span)
                };
                elements.push(ast::CompositeElement { key, value, span });
                if !self.at(TokenKind::Comma) {
                    break;
                }
                self.bump();
                if self.at(TokenKind::RBrace) {
                    break;
                }
            }
        }
        let end = self.expect(TokenKind::RBrace, "`}`")?;
        Ok(ast::Expr::Composite {
            span: join_spans(start, end),
            ty,
            elements,
        })
    }

    fn parse_path(&mut self) -> Result<ast::Path, Diagnostics> {
        let first = self.expect_ident()?;
        let mut segments = vec![first];
        while self.at(TokenKind::Dot) && self.peek_kind(1) == Some(TokenKind::Identifier) {
            self.bump();
            segments.push(self.expect_ident()?);
        }
        let start = segments
            .first()
            .map(|segment| segment.span)
            .unwrap_or(Span::synthetic());
        let end = segments.last().map(|segment| segment.span).unwrap_or(start);
        Ok(ast::Path {
            span: join_spans(start, end),
            segments,
        })
    }

    fn parse_identifier_list(&mut self) -> Result<Vec<ast::Ident>, Diagnostics> {
        let mut names = vec![self.expect_binding_ident()?];
        while self.at(TokenKind::Comma) {
            self.bump();
            names.push(self.expect_binding_ident()?);
        }
        Ok(names)
    }

    fn check_binding_ident(name: &ast::Ident) -> Result<(), Diagnostics> {
        if crate::identifier::reserved_builtin(&name.name) {
            let category = if crate::identifier::builtin_type(&name.name) {
                "builtin type name"
            } else {
                "builtin name"
            };
            Err(Diagnostics::from_single(Diagnostic::error(
                name.span,
                format!("{category} `{}` cannot be used as a declaration name", name.name),
            )))
        } else {
            Ok(())
        }
    }

    fn expect_binding_ident(&mut self) -> Result<ast::Ident, Diagnostics> {
        let name = self.expect_ident()?;
        Self::check_binding_ident(&name)?;
        Ok(name)
    }

    fn expect_ident(&mut self) -> Result<ast::Ident, Diagnostics> {
        if !self.at(TokenKind::Identifier) {
            return Err(self.error_here("expected identifier"));
        }
        let token = self.bump();
        Ok(ast::Ident {
            name: token.text,
            span: token.span,
        })
    }

    fn expect_string_lit(&mut self) -> Result<String, Diagnostics> {
        if !self.at(TokenKind::StringLit) {
            return Err(self.error_here("expected string literal"));
        }
        Ok(self.bump().text)
    }

    fn expect_semi(&mut self) -> Result<Span, Diagnostics> {
        if self.at(TokenKind::RBrace) || self.at(TokenKind::RParen) {
            Ok(self.current().span)
        } else {
            self.expect(TokenKind::Semi, "`;` or newline")
        }
    }

    fn expect(&mut self, kind: TokenKind, expected: &str) -> Result<Span, Diagnostics> {
        if self.at(kind) {
            Ok(self.bump().span)
        } else {
            Err(self.error_here(format!("expected {expected}")))
        }
    }

    fn error_here(&self, message: impl Into<String>) -> Diagnostics {
        Diagnostics::from_single(Diagnostic::error(self.current().span, message))
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn peek_kind(&self, offset: usize) -> Option<TokenKind> {
        self.tokens
            .get(self.cursor + offset)
            .map(|token| token.kind)
    }

    fn peek_kind_skipping_semi(&self, mut offset: usize) -> Option<TokenKind> {
        while let Some(kind) = self.peek_kind(offset) {
            if kind != TokenKind::Semi {
                return Some(kind);
            }
            offset += 1;
        }
        None
    }

    fn named_composite_literal_starts(&self) -> bool {
        if !self.at(TokenKind::LBrace) {
            return false;
        }
        // Outside a control header, a following brace can only start a literal.
        // Named arrays allow positional elements and arbitrary constant keys,
        // not just the `field: value` form used by structs.
        if self.header == HeaderMode::None {
            return true;
        }
        match self.peek_kind_skipping_semi(1) {
            Some(TokenKind::RBrace) => self.empty_composite_in_header(),
            Some(TokenKind::Identifier) => {
                self.peek_kind_skipping_semi(2) == Some(TokenKind::Colon)
            }
            _ => false,
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        self.cursor += 1;
        token
    }
}

fn checked_simple_statement(stmt: ast::Stmt) -> Result<ast::Stmt, Diagnostics> {
    if let ast::Stmt::Expr(expr) = &stmt {
        if !matches!(expr.expr, ast::Expr::Call { .. }) {
            return Err(Diagnostics::from_single(Diagnostic::error(
                expr.span,
                "expression statement must be a function call",
            )));
        }
    }
    Ok(stmt)
}

enum SimpleOrExpr {
    Stmt(ast::Stmt),
    Expr(ast::Expr),
}

impl SimpleOrExpr {
    fn into_stmt(self) -> Result<ast::Stmt, Diagnostics> {
        match self {
            Self::Stmt(stmt) => checked_simple_statement(stmt),
            Self::Expr(expr @ ast::Expr::Call { .. }) => Ok(ast::Stmt::Expr(ast::ExprStmt {
                span: expr.span(),
                expr,
            })),
            Self::Expr(expr) => Err(Diagnostics::from_single(Diagnostic::error(
                expr.span(),
                "expected simple statement",
            ))),
        }
    }

    fn into_expr(self) -> Result<ast::Expr, Diagnostics> {
        match self {
            Self::Expr(expr) => Ok(expr),
            Self::Stmt(stmt) => Err(Diagnostics::from_single(Diagnostic::error(
                stmt_span(&stmt),
                "expected expression",
            ))),
        }
    }
}

fn current_unary(kind: TokenKind) -> Option<ast::UnaryOp> {
    Some(match kind {
        TokenKind::Plus => ast::UnaryOp::Plus,
        TokenKind::Minus => ast::UnaryOp::Minus,
        TokenKind::Bang => ast::UnaryOp::Not,
        TokenKind::Caret => ast::UnaryOp::BitNot,
        TokenKind::Star => ast::UnaryOp::Deref,
        TokenKind::Amp => ast::UnaryOp::AddrOf,
        _ => return None,
    })
}

fn current_binary(kind: TokenKind) -> Option<(ast::BinaryOp, u8)> {
    Some(match kind {
        TokenKind::OrOr => (ast::BinaryOp::OrOr, 1),
        TokenKind::AndAnd => (ast::BinaryOp::AndAnd, 2),
        TokenKind::EqEq => (ast::BinaryOp::Eq, 3),
        TokenKind::NotEq => (ast::BinaryOp::Ne, 3),
        TokenKind::Lt => (ast::BinaryOp::Lt, 3),
        TokenKind::Le => (ast::BinaryOp::Le, 3),
        TokenKind::Gt => (ast::BinaryOp::Gt, 3),
        TokenKind::Ge => (ast::BinaryOp::Ge, 3),
        TokenKind::Plus => (ast::BinaryOp::Add, 4),
        TokenKind::Minus => (ast::BinaryOp::Sub, 4),
        TokenKind::Pipe => (ast::BinaryOp::BitOr, 4),
        TokenKind::Caret => (ast::BinaryOp::BitXor, 4),
        TokenKind::Star => (ast::BinaryOp::Mul, 5),
        TokenKind::Slash => (ast::BinaryOp::Div, 5),
        TokenKind::Percent => (ast::BinaryOp::Rem, 5),
        TokenKind::ShiftLeft => (ast::BinaryOp::ShiftLeft, 5),
        TokenKind::ShiftRight => (ast::BinaryOp::ShiftRight, 5),
        TokenKind::Amp => (ast::BinaryOp::BitAnd, 5),
        TokenKind::AmpCaret => (ast::BinaryOp::BitAndNot, 5),
        _ => return None,
    })
}

fn current_assignment(kind: TokenKind) -> Option<ast::BinaryOp> {
    Some(match kind {
        TokenKind::PlusAssign => ast::BinaryOp::Add,
        TokenKind::MinusAssign => ast::BinaryOp::Sub,
        TokenKind::StarAssign => ast::BinaryOp::Mul,
        TokenKind::SlashAssign => ast::BinaryOp::Div,
        TokenKind::PercentAssign => ast::BinaryOp::Rem,
        TokenKind::AmpAssign => ast::BinaryOp::BitAnd,
        TokenKind::PipeAssign => ast::BinaryOp::BitOr,
        TokenKind::CaretAssign => ast::BinaryOp::BitXor,
        TokenKind::AmpCaretAssign => ast::BinaryOp::BitAndNot,
        TokenKind::ShiftLeftAssign => ast::BinaryOp::ShiftLeft,
        TokenKind::ShiftRightAssign => ast::BinaryOp::ShiftRight,
        _ => return None,
    })
}

fn join_spans(start: Span, end: Span) -> Span {
    Span::new(start.file, start.start, end.end)
}

fn type_span(ty: &ast::Type) -> Span {
    match ty {
        ast::Type::Named(path) => path.span,
        ast::Type::Apply { span, .. } => *span,
        ast::Type::Resolved(_, span) => *span,
        ast::Type::Pointer(_, span)
        | ast::Type::Array { span, .. }
        | ast::Type::Struct { span, .. }
        | ast::Type::Interface { span, .. }
        | ast::Type::Func { span, .. } => *span,
    }
}

fn stmt_span(stmt: &ast::Stmt) -> Span {
    match stmt {
        ast::Stmt::Const(decl) => decl.span,
        ast::Stmt::Var(decl) => decl.span,
        ast::Stmt::ShortVar(decl) => decl.span,
        ast::Stmt::Assign(decl) => decl.span,
        ast::Stmt::Expr(decl) => decl.span,
        ast::Stmt::Return(decl) => decl.span,
        ast::Stmt::Break(span) | ast::Stmt::Continue(span) => *span,
        ast::Stmt::Defer(statement) => statement.span,
        ast::Stmt::Trap { span, .. } => *span,
        ast::Stmt::Block(block) => block.span,
        ast::Stmt::If(stmt) => stmt.span,
        ast::Stmt::For(stmt) => stmt.span,
    }
}

impl Diagnostics {
    fn from_single(diagnostic: Diagnostic) -> Self {
        let mut diagnostics = Diagnostics::new();
        diagnostics.push(diagnostic);
        diagnostics
    }
}
