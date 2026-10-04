//! Declaration signatures and function-type signatures share type-list parsing.
use super::*;

impl Parser {
    pub(super) fn parse_signature(&mut self) -> Result<ast::Signature, Diagnostics> {
        let signature = self.parse_type_signature(true)?;
        let params = signature
            .params
            .into_iter()
            .map(|parameter| ast::Field {
                name: parameter
                    .name
                    .expect("declaration parser requires parameter names"),
                ty: parameter.ty,
                span: parameter.span,
            })
            .collect();
        Ok(ast::Signature {
            params,
            results: signature.results,
            span: signature.span,
        })
    }

    pub(super) fn parse_type_signature(
        &mut self,
        require_names: bool,
    ) -> Result<ast::TypeSignature, Diagnostics> {
        let start = self.expect(TokenKind::LParen, "`(`")?;
        let mut params = Vec::new();
        if !self.at(TokenKind::RParen) {
            loop {
                let name = if require_names
                    || (self.at(TokenKind::Identifier)
                        && self.peek_kind(1) == Some(TokenKind::Colon))
                {
                    let name = self.expect_ident()?;
                    if require_names {
                        Self::check_binding_ident(&name)?;
                    }
                    self.expect(TokenKind::Colon, "`:`")?;
                    Some(name)
                } else {
                    None
                };
                let ty = self.parse_type()?;
                let span = name
                    .as_ref()
                    .map_or(type_span(&ty), |name| join_spans(name.span, type_span(&ty)));
                params.push(ast::TypeParameter { name, ty, span });
                if !self.at(TokenKind::Comma) {
                    break;
                }
                self.bump();
                if self.at(TokenKind::RParen) {
                    break;
                }
            }
        }
        let mut end = self.expect(TokenKind::RParen, "`)`")?;
        let mut results = Vec::new();
        if self.at(TokenKind::Arrow) {
            self.bump();
            if self.at(TokenKind::LParen) {
                self.bump();
                if self.at(TokenKind::RParen) {
                    return Err(self.error_here(
                        "empty result list is not allowed; omit `->` for zero return values",
                    ));
                }
                if !self.at(TokenKind::RParen) {
                    loop {
                        let ty = self.parse_type()?;
                        results.push(ty);
                        if !self.at(TokenKind::Comma) {
                            break;
                        }
                        self.bump();
                        if self.at(TokenKind::RParen) {
                            break;
                        }
                    }
                }
                end = self.expect(TokenKind::RParen, "`)`")?;
            } else {
                let ty = self.parse_type()?;
                end = type_span(&ty);
                results.push(ty);
            }
        }
        Ok(ast::TypeSignature {
            params,
            results,
            span: join_spans(start, end),
        })
    }
}
