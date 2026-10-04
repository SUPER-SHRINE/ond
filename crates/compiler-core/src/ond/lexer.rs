struct Lexer<'a> {
    file_id: FileId,
    text: &'a str,
    chars: Peekable<std::str::CharIndices<'a>>,
    tokens: Vec<Token>,
    diagnostics: Diagnostics,
}

impl<'a> Lexer<'a> {
    fn new(file_id: FileId, text: &'a str) -> Self {
        Self {
            file_id,
            text,
            chars: text.char_indices().peekable(),
            tokens: Vec::new(),
            diagnostics: Diagnostics::new(),
        }
    }

    fn lex(&mut self) -> Result<Vec<Token>, Diagnostics> {
        for (index, ch) in self.text.char_indices() {
            if ch == '\0' || ch == '\u{feff}' && index != 0 {
                self.diagnostics.push(Diagnostic::error(
                    Span::new(self.file_id, index, index + ch.len_utf8()),
                    "NUL or non-leading byte order mark in source",
                ));
            }
        }
        if !self.diagnostics.is_empty() {
            return Err(mem::take(&mut self.diagnostics));
        }
        while let Some((index, ch)) = self.bump() {
            match ch {
                ' ' | '\t' | '\r' => {}
                '\n' => self.maybe_insert_semi(index),
                '/' if self.consume('/') => {
                    self.skip_line_comment();
                }
                '/' if self.consume('*') => {
                    let newline = self.skip_block_comment(index);
                    if newline {
                        self.maybe_insert_semi(index);
                    }
                }
                '\u{feff}' if index == 0 => {}
                ch if crate::identifier::start(ch) => self.lex_identifier(index),
                '0'..='9' => self.lex_number(index),
                '"' => self.lex_quoted_string(index),
                '`' => self.lex_raw_string(index),
                '(' => self.push_simple(TokenKind::LParen, index, 1),
                ')' => self.push_simple(TokenKind::RParen, index, 1),
                '{' => self.push_simple(TokenKind::LBrace, index, 1),
                '}' => self.push_simple(TokenKind::RBrace, index, 1),
                '[' => self.push_simple(TokenKind::LBracket, index, 1),
                ']' => self.push_simple(TokenKind::RBracket, index, 1),
                ',' => self.push_simple(TokenKind::Comma, index, 1),
                ';' => self.push_simple(TokenKind::Semi, index, 1),
                '.' if self.chars.peek().is_some_and(|(_, ch)| ch.is_ascii_digit()) => {
                    self.lex_number(index)
                }
                '.' if self.text[index..].starts_with("...") => {
                    self.bump();
                    self.bump();
                    self.push_simple(TokenKind::Ellipsis, index, 3);
                }
                '.' => self.push_simple(TokenKind::Dot, index, 1),
                ':' if self.consume('=') => self.push_simple(TokenKind::ColonAssign, index, 2),
                ':' => self.push_simple(TokenKind::Colon, index, 1),
                '=' if self.consume('=') => self.push_simple(TokenKind::EqEq, index, 2),
                '=' => self.push_simple(TokenKind::Assign, index, 1),
                '-' if self.consume('>') => self.push_simple(TokenKind::Arrow, index, 2),
                '-' if self.consume('-') => self.push_simple(TokenKind::Decrement, index, 2),
                '-' if self.consume('=') => self.push_simple(TokenKind::MinusAssign, index, 2),
                '-' => self.push_simple(TokenKind::Minus, index, 1),
                '+' if self.consume('+') => self.push_simple(TokenKind::Increment, index, 2),
                '+' if self.consume('=') => self.push_simple(TokenKind::PlusAssign, index, 2),
                '+' => self.push_simple(TokenKind::Plus, index, 1),
                '*' if self.consume('=') => self.push_simple(TokenKind::StarAssign, index, 2),
                '*' => self.push_simple(TokenKind::Star, index, 1),
                '%' if self.consume('=') => self.push_simple(TokenKind::PercentAssign, index, 2),
                '%' => self.push_simple(TokenKind::Percent, index, 1),
                '&' if self.consume('&') => self.push_simple(TokenKind::AndAnd, index, 2),
                '&' if self.consume('^') => {
                    if self.consume('=') {
                        self.push_simple(TokenKind::AmpCaretAssign, index, 3);
                    } else {
                        self.push_simple(TokenKind::AmpCaret, index, 2);
                    }
                }
                '&' if self.consume('=') => self.push_simple(TokenKind::AmpAssign, index, 2),
                '&' => self.push_simple(TokenKind::Amp, index, 1),
                '|' if self.consume('|') => self.push_simple(TokenKind::OrOr, index, 2),
                '|' if self.consume('=') => self.push_simple(TokenKind::PipeAssign, index, 2),
                '|' => self.push_simple(TokenKind::Pipe, index, 1),
                '^' if self.consume('=') => self.push_simple(TokenKind::CaretAssign, index, 2),
                '^' => self.push_simple(TokenKind::Caret, index, 1),
                '<' if self.consume('<') => {
                    if self.consume('=') {
                        self.push_simple(TokenKind::ShiftLeftAssign, index, 3);
                    } else {
                        self.push_simple(TokenKind::ShiftLeft, index, 2);
                    }
                }
                '<' if self.consume('=') => self.push_simple(TokenKind::Le, index, 2),
                '<' => self.push_simple(TokenKind::Lt, index, 1),
                '>' if self.consume('>') => {
                    if self.consume('=') {
                        self.push_simple(TokenKind::ShiftRightAssign, index, 3);
                    } else {
                        self.push_simple(TokenKind::ShiftRight, index, 2);
                    }
                }
                '>' if self.consume('=') => self.push_simple(TokenKind::Ge, index, 2),
                '>' => self.push_simple(TokenKind::Gt, index, 1),
                '!' if self.consume('=') => self.push_simple(TokenKind::NotEq, index, 2),
                '!' => self.push_simple(TokenKind::Bang, index, 1),
                '/' if self.consume('=') => self.push_simple(TokenKind::SlashAssign, index, 2),
                '/' => self.push_simple(TokenKind::Slash, index, 1),
                other => self.diagnostics.push(Diagnostic::error(
                    Span::new(self.file_id, index, index + other.len_utf8()),
                    format!("unexpected character `{other}`"),
                )),
            }
        }

        self.maybe_insert_semi(self.text.len());
        let eof = Span::new(self.file_id, self.text.len(), self.text.len());
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            text: String::new(),
            span: eof,
        });

        if self.diagnostics.is_empty() {
            Ok(mem::take(&mut self.tokens))
        } else {
            Err(mem::take(&mut self.diagnostics))
        }
    }

    fn bump(&mut self) -> Option<(usize, char)> {
        self.chars.next()
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.chars.peek().is_some_and(|(_, ch)| *ch == expected) {
            self.chars.next();
            true
        } else {
            false
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some((_, ch)) = self.chars.peek() {
            if *ch == '\n' {
                break;
            }
            self.chars.next();
        }
    }

    fn skip_block_comment(&mut self, start: usize) -> bool {
        let mut saw_newline = false;
        while let Some((_, ch)) = self.bump() {
            if ch == '\n' {
                saw_newline = true;
            }
            if ch == '*' && self.consume('/') {
                return saw_newline;
            }
        }
        self.diagnostics.push(Diagnostic::error(
            Span::new(self.file_id, start, self.text.len()),
            "unterminated block comment",
        ));
        saw_newline
    }

    fn lex_identifier(&mut self, start: usize) {
        let end = self.consume_while(start, crate::identifier::continuation);
        let text = self.text[start..end].to_string();
        let kind = match text.as_str() {
            "package" => TokenKind::Package,
            "import" => TokenKind::Import,
            "const" => TokenKind::Const,
            "var" => TokenKind::Var,
            "type" => TokenKind::Type,
            "func" => TokenKind::Func,
            "struct" => TokenKind::Struct,
            "interface" => TokenKind::Interface,
            "return" => TokenKind::Return,
            "defer" => TokenKind::Defer,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            "as" => TokenKind::As,
            "new" => TokenKind::New,
            _ => TokenKind::Identifier,
        };
        self.tokens.push(Token {
            kind,
            text,
            span: Span::new(self.file_id, start, end),
        });
    }

    fn lex_number(&mut self, start: usize) {
        let (length, result) = crate::numeric_literal::scan(&self.text[start..]);
        let end = start + length;
        while self.chars.peek().is_some_and(|(index, _)| *index < end) {
            self.chars.next();
        }
        let span = Span::new(self.file_id, start, end);
        match result {
            Ok(kind) => self.tokens.push(Token {
                kind: match kind {
                    crate::numeric_literal::Kind::Integer => TokenKind::IntLit,
                    crate::numeric_literal::Kind::Float => TokenKind::FloatLit,
                },
                text: self.text[start..end].into(),
                span,
            }),
            Err(message) => self.diagnostics.push(Diagnostic::error(span, message)),
        }
    }

    fn lex_quoted_string(&mut self, start: usize) {
        let mut escaped = false;
        let mut end = start + 1;
        while let Some((index, ch)) = self.bump() {
            end = index + ch.len_utf8();
            if ch == '\n' {
                break;
            }
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => {
                    self.push_string(start, end);
                    return;
                }
                '\n' => break,
                _ => {}
            }
        }
        self.diagnostics.push(Diagnostic::error(
            Span::new(self.file_id, start, end),
            "unterminated string literal",
        ));
    }

    fn lex_raw_string(&mut self, start: usize) {
        let mut end = start + 1;
        while let Some((index, ch)) = self.bump() {
            end = index + ch.len_utf8();
            if ch == '`' {
                self.push_string(start, end);
                return;
            }
        }
        self.diagnostics.push(Diagnostic::error(
            Span::new(self.file_id, start, end),
            "unterminated raw string literal",
        ));
    }

    fn push_string(&mut self, start: usize, end: usize) {
        let text = &self.text[start..end];
        let span = Span::new(self.file_id, start, end);
        match crate::string_literal::decode(text) {
            Ok(_) => self.tokens.push(Token {
                kind: TokenKind::StringLit,
                text: text.into(),
                span,
            }),
            Err(message) => self.diagnostics.push(Diagnostic::error(span, message)),
        }
    }

    fn push_simple(&mut self, kind: TokenKind, start: usize, width: usize) {
        self.tokens.push(Token {
            kind,
            text: self.text[start..start + width].to_string(),
            span: Span::new(self.file_id, start, start + width),
        });
    }

    fn maybe_insert_semi(&mut self, offset: usize) {
        let Some(last) = self.tokens.last() else {
            return;
        };
        if can_end_statement(last.kind) {
            self.tokens.push(Token {
                kind: TokenKind::Semi,
                text: ";".to_string(),
                span: Span::new(self.file_id, offset, offset),
            });
        }
    }

    fn consume_while(&mut self, start: usize, predicate: impl Fn(char) -> bool) -> usize {
        let mut end = start + self.text[start..].chars().next().map_or(0, char::len_utf8);
        while let Some((index, ch)) = self.chars.peek().copied() {
            if !predicate(ch) {
                break;
            }
            self.chars.next();
            end = index + ch.len_utf8();
        }
        end
    }
}

fn can_end_statement(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::IntLit
            | TokenKind::FloatLit
            | TokenKind::StringLit
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil
            | TokenKind::Break
            | TokenKind::Continue
            | TokenKind::Return
            | TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::RBrace
            | TokenKind::Increment
            | TokenKind::Decrement
    )
}
