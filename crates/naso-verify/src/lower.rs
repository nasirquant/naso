//! AST to SMT-LIB2 Lowering Engine.
//!
//! This is the main entry point for translating typed Naso AST into SMT-LIB2
//! verification conditions. It orchestrates the quantity, MVS, quantum, and
//! polyhedral encoders.

#![allow(unused_imports)]

use crate::error::{LoweringError, LoweringError::*, VerifyError};
use crate::quantity::{QuantityKind, QuantityTracker, encode_quantity_expr};
use crate::quantum::{QuantumTracker, encode_quantum_expr};
use naso_compiler::ast::{Expr, Function, Mutability, Program};

#[cfg(feature = "z3")]
use crate::mvs::{MvsTracker, encode_mvs_function};
#[cfg(feature = "z3")]
use crate::polyhedral::{PolyhedralTracker, encode_polyhedral_expr, encode_polyhedral_function};
#[cfg(feature = "z3")]
use crate::smtlib::{Script, Sort, builder::*};

#[cfg(feature = "z3")]
/// Main lowering context that holds all trackers.
pub struct LoweringContext {
    pub quantity: QuantityTracker,
    pub mvs: MvsTracker,
    pub quantum: QuantumTracker,
    pub polyhedral: PolyhedralTracker,
    pub script: Script,
    pub current_function: Option<String>,
    pub errors: Vec<VerifyError>,
}

#[cfg(feature = "z3")]
impl LoweringContext {
    pub fn new() -> Self {
        let mut script = Script::new();
        script.set_logic("AUFLIA");
        Self {
            quantity: QuantityTracker::new(),
            mvs: MvsTracker::new(),
            quantum: QuantumTracker::new(),
            polyhedral: PolyhedralTracker::new(),
            script,
            current_function: None,
            errors: Vec::new(),
        }
    }

    /// Add an error to the context.
    pub fn add_error(&mut self, err: VerifyError) {
        self.errors.push(err);
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get all errors.
    pub fn errors(&self) -> &[VerifyError] {
        &self.errors
    }

    /// Finalize and return the SMT-LIB2 script.
    pub fn finalize(mut self) -> Result<Script, VerifyError> {
        if self.has_errors() {
            return Err(VerifyError::Lowering(LoweringError::SmtlibGen(format!(
                "Lowering completed with {} errors",
                self.errors.len()
            ))));
        }

        // Generate all declarations
        self.quantity.generate_declarations(&mut self.script);
        self.mvs.generate_declarations(&mut self.script);
        self.quantum.generate_declarations(&mut self.script);
        self.polyhedral.generate_declarations(&mut self.script);

        // Add check-sat and get-model
        self.script.check_sat();
        self.script.get_model();

        Ok(self.script)
    }

    /// Get the SMT-LIB2 script as a string.
    pub fn to_smtlib_string(self) -> Result<String, VerifyError> {
        let script = self.finalize()?;
        Ok(script.to_string())
    }
}

#[cfg(feature = "z3")]
impl Default for LoweringContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "z3")]
/// Lower a typed AST to SMT-LIB2.
pub fn lower_to_smtlib(program: &Program) -> Result<String, VerifyError> {
    let mut ctx = LoweringContext::new();

    // Process each function from program items
    for item in &program.items {
        if let naso_compiler::ast::Item::Function(func) = item {
            ctx.current_function = Some(func.name.name.clone());
            lower_function(func, &mut ctx)?;
            ctx.current_function = None;
        }
    }

    ctx.to_smtlib_string()
}

#[cfg(feature = "z3")]
/// Lower a single function.
fn lower_function(func: &Function, ctx: &mut LoweringContext) -> Result<(), VerifyError> {
    let mut constraints = Vec::new();

    // Enter function scope for all trackers
    ctx.quantity = QuantityTracker::new(); // Fresh tracker per function
    ctx.mvs.enter_function(func.name.name.clone());
    ctx.quantum.current_function = Some(func.name.name.clone());
    // Polyhedral tracker is global across functions

    // Process parameters with quantities
    for param in &func.params {
        let qk = QuantityKind::from_ast(&param.ty.quantity);
        match qk {
            QuantityKind::Zero => ctx.quantity.register_erased(&param.name.name, param.span),
            QuantityKind::One => {
                ctx.quantity.allocate_linear(
                    &param.name.name,
                    param.span,
                    crate::quantity::AllocSite::Param(param.name.name.clone()),
                );
            }
            QuantityKind::Bounded(n) => {
                ctx.quantity
                    .register_bounded(&param.name.name, n, param.span)
            }
            QuantityKind::Many => {}
        }

        // Register inout parameters
        if param.mutability == Mutability::InOut {
            // MVS encoding handled in encode_mvs_function
        }
    }

    // Encode quantity constraints
    if let Some(body_expr) = &func.body.expr {
        constraints.extend(encode_quantity_expr(body_expr.as_ref(), &mut ctx.quantity)?);
    }
    for stmt in &func.body.stmts {
        if let naso_compiler::ast::StmtKind::Expr(expr) = &stmt.kind {
            constraints.extend(encode_quantity_expr(expr, &mut ctx.quantity)?);
        }
    }

    // Encode MVS constraints
    constraints.extend(encode_mvs_function(func, &mut ctx.mvs)?);

    // Encode quantum constraints
    if let Some(body_expr) = &func.body.expr {
        constraints.extend(encode_quantum_expr(body_expr.as_ref(), &mut ctx.quantum)?);
    }
    for stmt in &func.body.stmts {
        if let naso_compiler::ast::StmtKind::Expr(expr) = &stmt.kind {
            constraints.extend(encode_quantum_expr(expr, &mut ctx.quantum)?);
        }
    }

    // Encode polyhedral constraints
    constraints.extend(encode_polyhedral_function(func, &mut ctx.polyhedral)?);

    // Add all constraints to script
    for constraint in constraints {
        ctx.script.assert(constraint);
    }

    // Exit MVS function scope
    ctx.mvs.exit_function();
    ctx.quantum.current_function = None;

    Ok(())
}

#[cfg(feature = "z3")]
/// Lower an expression.
#[allow(dead_code)]
fn lower_expr(expr: &Expr, ctx: &mut LoweringContext) -> Result<(), VerifyError> {
    // For top-level expressions, we mainly track quantities
    let _ = encode_quantity_expr(expr, &mut ctx.quantity)?;
    let _ = encode_quantum_expr(expr, &mut ctx.quantum)?;
    let _ = encode_polyhedral_expr(expr, &mut ctx.polyhedral)?;
    Ok(())
}

/// Re-export encode functions for modular use.
pub use crate::quantity::encode_quantity_expr as encode_quantity;
pub use crate::quantum::encode_quantum_expr as encode_quantum;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lowering_context_creation() {
        #[cfg(feature = "z3")]
        {
            let ctx = LoweringContext::new();
            assert!(!ctx.has_errors());
            assert_eq!(ctx.current_function, None);
        }
    }

    #[test]
    fn test_script_basic() {
        #[cfg(feature = "z3")]
        {
            use crate::smtlib::{Script, Sort, builder::*};

            let mut script = Script::new();
            script.set_logic("AUFLIA");
            script.declare_const("x", Sort::Int);
            script.assert(ge(var("x", Sort::Int), int(0)));
            script.check_sat();
            script.get_model();

            let output = script.to_string();
            assert!(output.contains("set-logic"));
            assert!(output.contains("declare-const"));
            assert!(output.contains("assert"));
            assert!(output.contains("check-sat"));
            assert!(output.contains("get-model"));
        }
    }
}
