//! naso-verify: SMT-based formal verification engine for the Naso programming language.
//!
//! This crate provides:
//! - AST to SMT-LIB2 lowering for QTT quantity constraints, linear resources, MVS, quantum uncomputation, and polyhedral invariants
//! - Z3 solver FFI bridge with incremental solving, model extraction, and unsat core support
//! - Automated provers for quantum uncomputation safety and [1]-quantity leak detection
//! - CLI integration via `naso verify` command

pub mod cache;
pub mod cli;
pub mod config;
pub mod error;
pub mod lower;
pub mod model;
pub mod mvs;
pub mod output;
pub mod polyhedral;
pub mod prover;
pub mod quantity;
pub mod quantum;
pub mod smtlib;
pub mod solver;

use crate::config::SolverConfig;
use crate::error::VerifyError;
use crate::solver::VerifyResult;
use naso_compiler::ast::Program;

/// Main verification entry point: lower AST to SMT-LIB2 and solve.
pub fn verify(program: &Program, config: SolverConfig) -> Result<VerifyResult, VerifyError> {
    let smt_script = lower::lower_to_smtlib(program)?;
    solver::verify(&smt_script, config)
}

/// Verify with default configuration (QF_UFLIA logic, 30s timeout, models enabled).
pub fn verify_default(program: &Program) -> Result<VerifyResult, VerifyError> {
    verify(program, SolverConfig::default())
}

/// Re-export CLI types for compiler integration
pub use cli::{VerifyCliConfig, VerifyMode, parse_verify_args};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_module_compiles() {
        // Smoke test: ensure the crate compiles and public API is accessible
        let _ = SolverConfig::default();
    }
}
