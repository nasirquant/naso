//! Naso Statement AST
//!
//! Top-level statements and declarations.

use crate::ast::{Attribute, GenericParam, Ident, Mutability, NodeId, Quantity, Span, Type};
use crate::ast::expr::Expr;
use crate::ast::pattern::Pattern;
use crate::ast::ty::Type as AstType;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
    pub id: NodeId,
}

impl Stmt {
    pub fn new(kind: StmtKind, span: Span, id: NodeId) -> Self {
        Self { kind, span, id }
    }
}

/// Statement kinds
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StmtKind {
    /// Let binding
    Let(LetStmt),
    /// Inout let binding
    LetInOut(LetInOutStmt),
    /// Consume let binding
    LetConsume(LetConsumeStmt),
    /// Expression statement
    Expr(Expr),
    /// Item declaration (function, type, etc.)
    Item(crate::ast::Item),
    /// Reversible block statement
    Reversible(crate::ast::expr::ReversibleBlock),
    /// Return statement
    Return(Option<Expr>),
    /// Break statement
    Break(Option<Expr>),
    /// Continue statement
    Continue,
    /// Empty statement
    Empty,
    /// Error statement
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetStmt {
    pub name: Ident,
    pub ty: Option<AstType>,
    pub quantity: Quantity,
    pub mutability: Mutability,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetInOutStmt {
    pub name: Ident,
    pub ty: Option<AstType>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetConsumeStmt {
    pub name: Ident,
    pub ty: Option<AstType>,
    pub value: Expr,
    pub span: Span,
}

impl fmt::Display for StmtKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StmtKind::Let(l) => write!(f, "let {}{:?} = {:?};", l.mutability, l.name, l.value),
            StmtKind::LetInOut(l) => write!(f, "let inout {:?} = {:?};", l.name, l.value),
            StmtKind::LetConsume(l) => write!(f, "let consume {:?} = {:?};", l.name, l.value),
            StmtKind::Expr(e) => write!(f, "{:?};", e),
            StmtKind::Item(i) => write!(f, "{:?}", i),
            StmtKind::Reversible(r) => write!(f, "reversible {{ ... }}"),
            StmtKind::Return(opt) => {
                if let Some(e) = opt {
                    write!(f, "return {:?};", e)
                } else {
                    write!(f, "return;")
                }
            }
            StmtKind::Break(opt) => {
                if let Some(e) = opt {
                    write!(f, "break {:?};", e)
                } else {
                    write!(f, "break;")
                }
            }
            StmtKind::Continue => write!(f, "continue;"),
            StmtKind::Empty => write!(f, ";"),
            StmtKind::Error => write!(f, "<error>"),
        }
    }
}