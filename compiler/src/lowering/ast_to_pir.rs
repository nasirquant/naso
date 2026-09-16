//! AST to PIR Lowering - Main Entry Point
//!
//! This module provides the main lowering function and context.

use crate::ast::Program;
use crate::ir::PirModule;
use super::LoweringError;

/// Lower a typed AST program to PIR
pub fn lower_ast(program: &Program) -> Result<PirModule, LoweringError> {
    let mut ctx = super::LoweringContext::new();
    ctx.lower_program(program)
}