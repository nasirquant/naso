//! Type parsing for the Naso parser.

use crate::ast::*;
use crate::ast::expr::Expr;
use crate::ast::ty::NatExpr;
use crate::lexer::TokenKind as TK;
use crate::parser::Parser;

use super::token_span;

impl<'a> Parser<'a> {
    /// Parse a type, including an optional leading quantity marker and
    /// trailing generic arguments for named types (e.g. `[0] Matrix[Rows, Cols]`).
    pub fn parse_type(&mut self) -> Type {
        let start = self.pos;
        let qty = self.parse_quantity();
        let base = self.parse_base_type();
        let kind = match base.kind {
            TypeKind::Named(name, mut args) => {
                if self.at(TK::LBracket) {
                    args = self.parse_type_args();
                }
                TypeKind::Named(name, args)
            }
            other => other,
        };
        let span = self.span_from(start);
        Type::new(kind, qty.unwrap_or(Quantity::Many), span)
    }

    /// Parse `[ ... ]` generic type arguments.
    fn parse_type_args(&mut self) -> Vec<TypeArg> {
        self.expect(TK::LBracket);
        let mut args = Vec::new();
        loop {
            if self.at(TK::RBracket) {
                break;
            }
            args.push(self.parse_type_arg());
            if self.at(TK::RBracket) {
                break;
            }
            self.expect(TK::Comma);
            if self.at(TK::RBracket) {
                break;
            }
        }
        self.expect(TK::RBracket);
        args
    }

    /// Parse a single type argument: a natural-number literal or a type.
    fn parse_type_arg(&mut self) -> TypeArg {
        if let Some(TK::Int(n)) = self.peek() {
            let n = *n;
            self.bump();
            return TypeArg::Nat(NatExpr::from_u64(n as u64));
        }
        TypeArg::Type(self.parse_type())
    }

    /// Parse the base of a type (no leading quantity, no trailing generic
    /// arguments).
    fn parse_base_type(&mut self) -> Type {
        match self.peek() {
            // `()` unit or tuple types or grouped types
            Some(TK::LParen) => {
                let start = self.pos;
                self.bump();
                // Check for empty tuple: ()
                if self.at(TK::RParen) {
                    self.bump();
                    let span = self.span_from(start);
                    return Type::unit(span);
                }
                // Parse first type
                let first = self.parse_type();
                // If no comma, it's a grouped type (T)
                if !self.at(TK::Comma) {
                    self.expect(TK::RParen);
                    return first;
                }
                // Parse tuple types: (T, U, ...)
                let mut items = vec![first];
                while self.at(TK::Comma) {
                    self.bump();
                    // Allow trailing comma
                    if self.at(TK::RParen) {
                        break;
                    }
                    items.push(self.parse_type());
                }
                self.expect(TK::RParen);
                let span = self.span_from(start);
                Type::new(TypeKind::Tuple(items), Quantity::Many, span)
            }
            // Dependent array type: [Type; expr]
            Some(TK::LBracket) => self.parse_array_type(),
            // Projection: inout T
            Some(TK::InOut) => {
                self.bump();
                let inner = self.parse_atom_type();
                Type::new(
                    TypeKind::Projection(Box::new(inner)),
                    Quantity::Many,
                    Span::default(),
                )
            }
            // Reversible type: reversible T
            Some(TK::Reversible) => {
                self.bump();
                let inner = self.parse_atom_type();
                Type::new(
                    TypeKind::Reversible(Box::new(inner)),
                    Quantity::Many,
                    Span::default(),
                )
            }
            Some(TK::Qubit) => {
                self.bump();
                Type::qubit(Span::default())
            }
            Some(TK::Nat) => {
                self.bump();
                Type::nat(Span::default())
            }
            Some(TK::FloatKw) => {
                self.bump();
                Type::new(TypeKind::Float, Quantity::Many, Span::default())
            }
            Some(TK::TypeIdent(_)) | Some(TK::QRegister) => self.parse_named_type(),
            Some(TK::Ident(_)) if self.at_ident("int") => {
                self.bump();
                Type::int(Span::default())
            }
            Some(TK::Ident(_)) if self.at_ident("bool") => {
                self.bump();
                Type::bool(Span::default())
            }
            Some(TK::Ident(_)) if self.at_ident("float") => {
                self.bump();
                Type::new(TypeKind::Float, Quantity::Many, Span::default())
            }
            Some(TK::Ident(_)) if self.at_ident("uint") => {
                self.bump();
                Type::new(TypeKind::UInt, Quantity::Many, Span::default())
            }
            None => self.unexpected("a type"),
            Some(k) => self.unexpected(&format!("a type, found `{k}`")),
        }
    }

    /// Parse a bare named type (TypeIdent or the reserved `QRegister` keyword,
    /// which still denotes a named type in type position). Generic arguments
    /// are handled in `parse_type`.
    fn parse_named_type(&mut self) -> Type {
        let tok = self.bump().expect("type name token");
        let span = token_span(&tok);
        let name = match &tok.kind {
            TK::TypeIdent(s) => Ident::new(s.clone(), span),
            TK::QRegister => Ident::new("QRegister".to_string(), span),
            other => panic!("expected named type, found `{other}`"),
        };
        Type::new(TypeKind::Named(name, Vec::new()), Quantity::Many, span)
    }

    /// Parse an atomic type (used for projection/reversible wrappers).
    fn parse_atom_type(&mut self) -> Type {
        self.parse_type()
    }

    /// Parse array type: [Type; expr] or [Type]
    fn parse_array_type(&mut self) -> Type {
        let start = self.pos;
        self.expect(TK::LBracket);
        
        // Parse element type
        let elem_ty = self.parse_type();
        
        // Check for size expression: [Type; expr]
        let size = if self.eat(TK::Semicolon) {
            let expr = self.parse_expr();
            // Convert expr to NatExpr - simplified for now
            let nat_expr = self.expr_to_nat_expr(expr);
            Some(nat_expr)
        } else {
            None
        };
        
        self.expect(TK::RBracket);
        let span = self.span_from(start);
        Type::new(TypeKind::Array(Box::new(elem_ty), size), Quantity::Many, span)
    }

    /// Convert expression to NatExpr (simplified)
    fn expr_to_nat_expr(&mut self, expr: Expr) -> NatExpr {
        match &expr.kind {
            ExprKind::Var(ident) => NatExpr::Var(ident.clone()),
            ExprKind::Literal(Literal::Int(n)) => NatExpr::from_u64(*n as u64),
            _ => NatExpr::Var(Ident::new("unknown", expr.span)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_program;

    #[test]
    fn parses_quantified_named_type() {
        let prog = parse_program("fn f(x: [0] Matrix[Rows, Cols]) {}").expect("parse failed");
        let func = match &prog.items[0] {
            Item::Function(f) => f,
            other => panic!("expected function, got {other:?}"),
        };
        let ty = &func.params[0].ty;
        assert_eq!(ty.quantity, Quantity::Zero);
        match &ty.kind {
            TypeKind::Named(name, args) => {
                assert_eq!(name.name, "Matrix");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected Named type, got {other:?}"),
        }
    }

    #[test]
    fn parses_tuple_type() {
        let prog = parse_program("fn f() -> (Qubit, Qubit) { (qalloc(1), qalloc(1)) }").expect("parse failed");
        let func = match &prog.items[0] {
            Item::Function(f) => f,
            other => panic!("expected function, got {other:?}"),
        };
        // Check return type is tuple
        match &func.ret_ty.as_ref().unwrap().kind {
            TypeKind::Tuple(elems) => {
                assert_eq!(elems.len(), 2);
                match (&elems[0].kind, &elems[1].kind) {
                    (TypeKind::Qubit, TypeKind::Qubit) => {}
                    other => panic!("expected (Qubit, Qubit), got {:?}", other),
                }
            }
            other => panic!("expected tuple type, got {:?}", other),
        }
    }

    #[test]
    fn parses_grouped_type() {
        let prog = parse_program("fn f(x: (Int)) { x }").expect("parse failed");
        let func = match &prog.items[0] {
            Item::Function(f) => f,
            other => panic!("expected function, got {other:?}"),
        };
        // Grouped type (Int) should be parsed as Int, not Tuple
        match &func.params[0].ty.kind {
            TypeKind::Int => {}
            other => panic!("expected Int, got {:?}", other),
        }
    }

    #[test]
    fn parses_unit_type() {
        let prog = parse_program("fn f() -> () { () }").expect("parse failed");
        let func = match &prog.items[0] {
            Item::Function(f) => f,
            other => panic!("expected function, got {other:?}"),
        };
        match &func.ret_ty.as_ref().unwrap().kind {
            TypeKind::Unit => {}
            other => panic!("expected unit type, got {:?}", other),
        }
    }
}