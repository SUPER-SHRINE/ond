//! Statement位置だけのasm認識。矢印の字句解釈はbinding directionに閉じる。
use super::*;
use crate::inline_asm::{Direction, Instruction, Label, Operation, Program, Register};

impl Parser {
    pub(super) fn asm_start(&self) -> bool {
        if self.current().kind != TokenKind::Identifier || self.current().text != "asm" {
            return false;
        }
        let mut at = self.cursor + 1;
        if self
            .tokens
            .get(at)
            .is_some_and(|t| t.kind == TokenKind::Identifier && t.text == "noreturn")
        {
            at += 1;
        }
        if self
            .tokens
            .get(at)
            .is_some_and(|t| t.kind == TokenKind::LParen)
        {
            let mut depth = 1;
            at += 1;
            while let Some(token) = self.tokens.get(at) {
                match token.kind {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth -= 1,
                    TokenKind::Eof => return false,
                    _ => {}
                }
                at += 1;
                if depth == 0 {
                    break;
                }
            }
            if depth != 0 {
                return false;
            }
        }
        self.tokens
            .get(at)
            .is_some_and(|t| t.kind == TokenKind::LBrace)
    }
    fn asm_register(&mut self) -> Result<Register, Diagnostics> {
        let token = self.current().clone();
        let number = token
            .text
            .strip_prefix('r')
            .and_then(|s| s.parse::<u8>().ok());
        match number {
            Some(number)
                if number <= 15
                    && token.text == format!("r{number}")
                    && token.kind == TokenKind::Identifier =>
            {
                self.bump();
                Ok(Register {
                    number,
                    span: token.span,
                })
            }
            _ => Err(self.error_here("asm register must be r0 through r15")),
        }
    }
    pub(super) fn parse_asm(&mut self) -> Result<ast::InlineAssembly, Diagnostics> {
        let start = self.bump().span;
        let noreturn =
            self.current().kind == TokenKind::Identifier && self.current().text == "noreturn";
        if noreturn {
            self.bump();
        }
        let mut bindings = Vec::new();
        if self.at(TokenKind::LParen) {
            self.bump();
            while !self.at(TokenKind::RParen) {
                let register = self.asm_register()?;
                let direction = if self.at(TokenKind::Arrow) {
                    self.bump();
                    Direction::Output
                } else {
                    let first = self.expect(TokenKind::Lt, "asm direction `<-`, `->`, or `<->`")?;
                    if self.current().span.start != first.end {
                        return Err(self.error_here("asm direction must be contiguous"));
                    }
                    if self.at(TokenKind::Arrow) {
                        self.bump();
                        Direction::InOut
                    } else {
                        self.expect(TokenKind::Minus, "`-`")?;
                        Direction::Input
                    }
                };
                let value = self.parse_expression()?;
                bindings.push(ast::AsmBinding {
                    register,
                    direction,
                    value,
                });
                if !self.at(TokenKind::Comma) {
                    break;
                }
                self.bump();
            }
            self.expect(TokenKind::RParen, "`)`")?;
        }
        self.expect(TokenKind::LBrace, "`{`")?;
        let mut instructions = Vec::new();
        let mut labels = Vec::new();
        while !self.at(TokenKind::RBrace) {
            if self.at(TokenKind::Semi) {
                self.bump();
                continue;
            }
            if self.at(TokenKind::Eof) {
                return Err(self.error_here("expected asm block end"));
            }
            let begin = self.bump();
            if begin.kind != TokenKind::Identifier {
                return Err(self.error_here("expected asm mnemonic or label"));
            }
            if self.at(TokenKind::Colon) {
                self.bump();
                labels.push(Label {
                    name: begin.text,
                    position: instructions.len(),
                    span: begin.span,
                });
            } else {
                let rd = self.asm_register()?;
                self.expect(TokenKind::Comma, "`,`")?;
                let rs1 = self.asm_register()?;
                self.expect(TokenKind::Comma, "`,`")?;
                let operation = if begin.text == "JZ"
                    && self.at(TokenKind::Identifier)
                    && self.peek_kind(1) != Some(TokenKind::Comma)
                {
                    if rd.number != 0 {
                        return Err(Diagnostics::from_single(Diagnostic::error(
                            rd.span,
                            "asm local JZ destination must be r0",
                        )));
                    }
                    let target = self.bump();
                    Operation::LocalBranch {
                        condition: rs1,
                        target: target.text,
                        target_span: target.span,
                    }
                } else {
                    let rs2 = self.asm_register()?;
                    self.expect(TokenKind::Comma, "`,`")?;
                    let (immediate, immediate_span) = self.asm_immediate()?;
                    if begin.text == "JZ" {
                        if rd.number != 0
                            || rs1.number != 0
                            || rs2.number == 0
                            || immediate != 0
                            || &self.source[immediate_span.start..immediate_span.end] != "0"
                        {
                            return Err(Diagnostics::from_single(Diagnostic::error(
                                begin.span,
                                "external asm JZ requires r0,r0,r1..r15,0",
                            )));
                        }
                        Operation::ExternalBranch { target: rs2 }
                    } else {
                        let opcode = match begin.text.as_str() {
                            "ADD" => 0,
                            "NAND" => 1,
                            "SHIFT" => 2,
                            "MUL" => 3,
                            "LDB" => 4,
                            "LDH" => 5,
                            "LDW" => 7,
                            "STB" => 8,
                            "STH" => 9,
                            "STW" => 11,
                            "CMP" => 13,
                            _ => {
                                return Err(Diagnostics::from_single(Diagnostic::error(
                                    begin.span,
                                    "unknown asm mnemonic",
                                )));
                            }
                        };
                        Operation::Machine {
                            opcode,
                            rd,
                            rs1,
                            rs2,
                            immediate,
                            immediate_span,
                        }
                    }
                };
                let end = self.tokens[self.cursor - 1].span;
                if self.source[begin.span.start..end.end].contains('\n') {
                    return Err(Diagnostics::from_single(Diagnostic::error(
                        begin.span,
                        "asm instruction cannot cross a source line break",
                    )));
                }
                instructions.push(Instruction {
                    operation,
                    span: join_spans(begin.span, end),
                });
            }
            // 改行はcomment内も含めて命令区切り。外側のsemicolon挿入に頼らない。
            let previous = self.tokens[self.cursor - 1].span.end;
            if !self.at(TokenKind::Semi)
                && !self.at(TokenKind::RBrace)
                && !self.source[previous..self.current().span.start].contains('\n')
            {
                return Err(self.error_here("expected asm newline or semicolon"));
            }
        }
        let end = self.bump().span;
        Ok(ast::InlineAssembly {
            noreturn,
            bindings,
            program: Program {
                instructions,
                labels,
                end,
            },
            span: join_spans(start, end),
        })
    }
    fn asm_immediate(&mut self) -> Result<(i16, Span), Diagnostics> {
        let start = self.current().span;
        let negative = self.at(TokenKind::Minus);
        if negative || self.at(TokenKind::Plus) {
            self.bump();
        }
        let token = self.current().clone();
        self.expect(TokenKind::IntLit, "asm integer immediate")?;
        let value = crate::numeric_literal::integer(&token.text)
            .map(|value| {
                if negative {
                    -i128::from(value)
                } else {
                    i128::from(value)
                }
            })
            .and_then(|value| i16::try_from(value).ok())
            .ok_or_else(|| {
                Diagnostics::from_single(Diagnostic::error(
                    join_spans(start, token.span),
                    "asm immediate is outside signed 16-bit range",
                ))
            })?;
        Ok((value, join_spans(start, token.span)))
    }
}
