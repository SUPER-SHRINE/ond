//! Context for the ambiguity between `flag {}` and a named composite literal.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HeaderMode {
    None,
    /// The first clause may introduce an initializer ending at a semicolon.
    Init,
    /// A condition or post clause immediately preceding the statement body.
    Body,
}

impl Parser {
    pub(super) fn in_header<T>(
        &mut self,
        mode: HeaderMode,
        parse: impl FnOnce(&mut Self) -> Result<T, Diagnostics>,
    ) -> Result<T, Diagnostics> {
        let saved = self.header;
        self.header = mode;
        let result = parse(self);
        self.header = saved;
        result
    }

    pub(super) fn header_values(&mut self) -> Result<Vec<ast::Expr>, Diagnostics> {
        if self.header == HeaderMode::Init {
            // `if x := S{}; ...` has an unambiguous initializer value.
            self.in_header(HeaderMode::None, |p| p.parse_expression_list())
        } else {
            self.parse_expression_list()
        }
    }

    pub(super) fn empty_composite_in_header(&self) -> bool {
        if self.header == HeaderMode::None {
            return true;
        }
        // A bare name followed by {} owns an empty statement body. Preserve
        // composites when a postfix/binary operator continues the expression,
        // or when a separate body follows: S{}.Flag, S{} == other, S{} {}.
        // Delimited subexpressions (arguments, indices, parentheses, literal
        // elements) temporarily use None, so f(S{}) remains unchanged.
        let closing = self
            .tokens
            .iter()
            .enumerate()
            .skip(self.cursor + 1)
            .find(|(_, token)| token.kind != TokenKind::Semi)
            .map(|(i, _)| i);
        let next = closing.and_then(|i| self.tokens.get(i + 1)).map(|t| t.kind);
        next.is_some_and(|kind| {
            matches!(
                kind,
                TokenKind::Dot
                    | TokenKind::As
                    | TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBrace
            ) || current_binary(kind).is_some()
        })
    }
}
