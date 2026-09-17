//! Automated provers for quantum uncomputation and linearity.
//!
//! This module provides the high-level prover interface that orchestrates
//! the SMT-based verification of quantum uncomputation safety and
//! [1]-quantity leak detection.

pub mod cfg;
pub mod linearity;
pub mod uncomputation;

use crate::error::{ProverError, VerifyError};
use crate::lower::LoweringContext;
use naso_compiler::ast::Program;

/// Main prover entry point: run all provers on an AST.
pub fn run_all_provers(program: &Program) -> Result<Vec<VerifyDiagnostic>, VerifyError> {
    let mut diagnostics = Vec::new();

    // Run uncomputation prover
    diagnostics.extend(uncomputation::prove_uncomputation(program)?);

    // Run linearity prover
    diagnostics.extend(linearity::prove_linearity(program)?);

    Ok(diagnostics)
}

/// Run only the uncomputation prover.
pub fn run_uncomputation_prover(program: &Program) -> Result<Vec<VerifyDiagnostic>, VerifyError> {
    uncomputation::prove_uncomputation(program)
}

/// Run only the linearity prover.
pub fn run_linearity_prover(program: &Program) -> Result<Vec<VerifyDiagnostic>, VerifyError> {
    linearity::prove_linearity(program)
}

/// Prove a custom verification condition.
pub fn prove_custom_vc(
    program: &Program,
    vc_name: &str,
    predicate: impl FnOnce(&mut LoweringContext) -> Result<(), VerifyError>,
) -> Result<Vec<VerifyDiagnostic>, VerifyError> {
    let mut ctx = LoweringContext::new();
    predicate(&mut ctx)?;
    ctx.finalize()?;
    // In real implementation, would run solver and extract diagnostics
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prover_module_compiles() {
        // Smoke test
    }
}
