//! Statement and block parsing for the Naso parser.

use crate::ast::*;
use crate::lexer::TokenKind as TK;
use crate::parser::{Parser, next_id};

use super::expr::is_control_flow_stmt;

impl<'a> Parser<'a> {
    // ===== Blocks =====

    /// Parse a `{ ... }` block.
    pub fn parse_block(&mut self) -> Block {
        let start = self.pos;
        self.expect(TK::LBrace);
        let (stmts, tail) = self.parse_stmt_list();
        self.expect(TK::RBrace);
        Block::new(stmts, tail, self.span_from(start))
    }

    /// Parse a sequence of statements terminated by `}` or EOF.
    ///
    /// Returns the statements plus an optional trailing expression (a
    /// statement without a closing `;` is the block's tail expression, unless
    /// it is a control-flow block construct that reads naturally without one).
    fn parse_stmt_list(&mut self) -> (Vec<Stmt>, Option<Expr>) {
        let mut stmts = Vec::new();
        let mut tail = None;

        loop {
            match self.peek() {
                Some(TK::RBrace) | None => break,
                Some(TK::Semicolon) => {
                    self.bump();
                    let span = Span::default();
                    stmts.push(Stmt::new(StmtKind::Empty, span, next_id()));
                }
                Some(TK::Let) => {
                    stmts.push(self.parse_let_stmt());
                }
                Some(TK::Reversible) => {
                    let start = self.pos;
                    let rb = self.parse_reversible_block();
                    self.eat(TK::Semicolon);
                    let span = self.span_from(start);
                    stmts.push(Stmt::new(StmtKind::Reversible(rb), span, next_id()));
                }
                _ => {
                    let expr = self.parse_expr();
                    if self.at(TK::Semicolon) {
                        self.bump();
                        let span = expr.span;
                        stmts.push(Stmt::new(StmtKind::Expr(expr), span, next_id()));
                    } else if is_control_flow_stmt(&expr.kind) {
                        // `if`/`match`/`for`/`while` are closed by `}` and
                        // need no trailing semicolon.
                        let span = expr.span;
                        stmts.push(Stmt::new(StmtKind::Expr(expr), span, next_id()));
                    } else {
                        tail = Some(expr);
                        break;
                    }
                }
            }
        }

        (stmts, tail)
    }

    // ===== Let statements =====

    fn parse_let_stmt(&mut self) -> Stmt {
        let start = self.pos;
        self.expect(TK::Let);

        // Check for `let inout` or `let consume` keywords
        if self.eat(TK::InOut) {
            // let inout name = value;
            let name = self.parse_ident();
            let ty = self.parse_optional_type_annotation();
            self.expect(TK::Assign);
            let value = self.parse_expr();
            self.eat(TK::Semicolon);
            let span = self.span_from(start);
            Stmt::new(
                StmtKind::LetInOut(LetInOutStmt {
                    name,
                    ty,
                    value,
                    span,
                }),
                span,
                next_id(),
            )
        } else if self.eat(TK::Consume) {
            // let consume name = value;
            let name = self.parse_ident();
            let ty = self.parse_optional_type_annotation();
            self.expect(TK::Assign);
            let value = self.parse_expr();
            self.eat(TK::Semicolon);
            let span = self.span_from(start);
            Stmt::new(
                StmtKind::LetConsume(LetConsumeStmt {
                    name,
                    ty,
                    value,
                    span,
                }),
                span,
                next_id(),
            )
        } else {
            // let [qty] mut? name = value;
            let quantity = self.parse_quantity().unwrap_or(Quantity::Many);
            let mutability = if self.eat(TK::InOut) {
                Mutability::InOut
            } else if self.eat(TK::Consume) {
                Mutability::Consume
            } else {
                Mutability::Immutable
            };
            let name = self.parse_ident();
            let ty = self.parse_optional_type_annotation();
            self.expect(TK::Assign);
            let value = self.parse_expr();
            self.eat(TK::Semicolon);
            let span = self.span_from(start);
            Stmt::new(
                StmtKind::Let(LetStmt {
                    name,
                    ty,
                    quantity,
                    mutability,
                    value,
                    span,
                }),
                span,
                next_id(),
            )
        }
    }

    fn parse_optional_type_annotation(&mut self) -> Option<Type> {
        if self.at(TK::Colon) {
            self.bump();
            Some(self.parse_type())
        } else {
            None
        }
    }

    // ===== Reversible blocks =====

    /// Parse a `reversible { ... }` block (shared by statements and
    /// expressions).
    pub(crate) fn parse_reversible_block(&mut self) -> ReversibleBlock {
        let start = self.pos;
        self.expect(TK::Reversible);
        let body = self.parse_block();
        let span = self.span_from(start);
        ReversibleBlock {
            body,
            uncomputes: Vec::new(),
            span,
        }
    }
}
