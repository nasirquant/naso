//! AST to SMT-LIB2 Lowering Engine.
//!
//! This is the main entry point for translating typed Naso AST into SMT-LIB2
//! verification conditions. It orchestrates the quantity, MVS, quantum, and
//! polyhedral encoders.

use crate::error::{LoweringError, LoweringError::*, VerifyError};
use crate::mvs::{MvsTracker, encode_mvs_function};
use crate::polyhedral::{PolyhedralTracker, encode_polyhedral_function};
use crate::quantity::{QuantityKind, QuantityTracker};
use crate::quantum::{QuantumTracker, encode_quantum_function};
use indexmap::IndexMap;
use naso_compiler::ast::Span;
use naso_compiler::ast::{Expr, Function, Program, Quantity, Type};

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

impl LoweringContext {
    pub fn new() -> Self {
        let mut script = Script::new();
        script.set_logic("QF_UFLIA");
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
    pub fn to_smtlib_string(mut self) -> Result<String, VerifyError> {
        let script = self.finalize()?;
        Ok(script.to_string())
    }
}

impl Default for LoweringContext {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Lower a single function.
fn lower_function(func: &Function, ctx: &mut LoweringContext) -> Result<(), VerifyError> {
    let mut constraints = Vec::new();

    // Enter function scope for all trackers
    ctx.quantity = QuantityTracker::new(); // Fresh tracker per function
    ctx.mvs.enter_function(func.name.clone());
    ctx.quantum.current_function = Some(func.name.clone());
    // Polyhedral tracker is global across functions

    // Process parameters with quantities
    for param in &func.params {
        if let Some(qty) = param.ty.quantity() {
            let qk = QuantityKind::from_ast(&qty);
            match qk {
                QuantityKind::Zero => ctx.quantity.register_erased(&param.name, param.span),
                QuantityKind::One => {
                    ctx.quantity.allocate_linear(
                        &param.name,
                        param.span,
                        crate::quantity::AllocSite::Param(param.name.clone()),
                    );
                }
                QuantityKind::Bounded(n) => {
                    ctx.quantity.register_bounded(&param.name, n, param.span)
                }
                QuantityKind::Many => {}
            }
        }

        // Register inout parameters
        if param.is_inout {
            // MVS encoding handled in encode_mvs_function
        }
    }

    // Encode quantity constraints
    if let Some(body) = &func.body {
        constraints.extend(encode_quantity_expr(body, &mut ctx.quantity)?);
    }

    // Encode MVS constraints
    constraints.extend(encode_mvs_function(func, &mut ctx.mvs)?);

    // Encode quantum constraints
    constraints.extend(encode_quantum_function(func, &mut ctx.quantum)?);

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

/// Lower an expression.
fn lower_expr(expr: &Expr, ctx: &mut LoweringContext) -> Result<(), VerifyError> {
    // For top-level expressions, we mainly track quantities
    let _ = encode_quantity_expr(expr, &mut ctx.quantity)?;
    let _ = encode_quantum_expr(expr, &mut ctx.quantum)?;
    let _ = encode_polyhedral_expr(expr, &mut ctx.polyhedral)?;
    Ok(())
}

pub use crate::mvs::encode_mvs_expr;
pub use crate::polyhedral::encode_polyhedral_expr;
/// Re-export encode functions for modular use.
pub use crate::quantity::encode_quantity_expr;
pub use crate::quantum::encode_quantum_expr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lowering_context_creation() {
        let ctx = LoweringContext::new();
        assert!(!ctx.has_errors());
        assert_eq!(ctx.current_function, None);
    }

    #[test]
    fn test_script_basic() {
        let mut script = Script::new();
        script.set_logic("QF_UFLIA");
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
