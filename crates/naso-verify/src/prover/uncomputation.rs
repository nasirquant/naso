//! Quantum Uncomputation Safety Prover.
//!
//! This prover verifies that all temporary qubits allocated via `qalloc`
//! are returned to the |0> state before scope exit. It encodes the
//! quantum circuit as symbolic unitary matrices and proves U_temp |0> = |0>.

use crate::config::SolverConfig;
use crate::error::VerifyError;
use crate::lower::LoweringContext;
use crate::model::VerifyDiagnostic;
use crate::quantum::{GateKind, QuantumTracker};
use crate::solver::verify;
use naso_compiler::ast::Span;
use naso_compiler::ast::{Function, Program};

/// Run the uncomputation prover on all quantum functions in the AST.
pub fn prove_uncomputation(program: &Program) -> Result<Vec<VerifyDiagnostic>, VerifyError> {
    let mut diagnostics = Vec::new();

    for item in &program.items {
        if let naso_compiler::ast::Item::Function(func) = item {
            if is_quantum_function(func) {
                let func_diagnostics = prove_function_uncomputation(func)?;
                diagnostics.extend(func_diagnostics);
            }
        }
    }

    Ok(diagnostics)
}

/// Check if a function contains quantum operations.
fn is_quantum_function(func: &Function) -> bool {
    // Check if function body contains quantum operations
    if let Some(body) = &func.body {
        contains_quantum_ops(body)
    } else {
        false
    }
}

/// Check if an expression contains quantum operations.
fn contains_quantum_ops(body: &naso_compiler::ast::Block) -> bool {
    // Check body expression
    if let Some(expr) = &body.expr {
        if contains_quantum_expr(expr) {
            return true;
        }
    }
    // Check statements
    for stmt in &body.stmts {
        if contains_quantum_stmt(stmt) {
            return true;
        }
    }
    false
}

fn contains_quantum_stmt(stmt: &naso_compiler::ast::Stmt) -> bool {
    match &stmt.kind {
        naso_compiler::ast::StmtKind::Expr(expr) => contains_quantum_expr(expr),
        naso_compiler::ast::StmtKind::Let(binding) => contains_quantum_expr(&binding.value),
        naso_compiler::ast::StmtKind::LetInOut(binding) => contains_quantum_expr(&binding.value),
        naso_compiler::ast::StmtKind::LetConsume(binding) => contains_quantum_expr(&binding.value),
        naso_compiler::ast::StmtKind::Assign(lhs, rhs) => {
            contains_quantum_expr(lhs) || contains_quantum_expr(rhs)
        }
        _ => false,
    }
}

fn contains_quantum_expr(expr: &naso_compiler::ast::Expr) -> bool {
    match &expr.kind {
        naso_compiler::ast::ExprKind::Call(func, args, _) => {
            if let naso_compiler::ast::ExprKind::Var(name, _, _) = &func.kind {
                if is_quantum_gate_name(&name.name) || name.name == "qalloc" || name.name == "qfree"
                {
                    return true;
                }
            }
            args.iter().any(contains_quantum_expr)
        }
        naso_compiler::ast::ExprKind::Let(binding) => contains_quantum_expr(&binding.value),
        naso_compiler::ast::ExprKind::If(_, then_e, else_e, _) => {
            contains_quantum_expr(then_e)
                || else_e.as_ref().map_or(false, |e| contains_quantum_expr(e))
        }
        naso_compiler::ast::ExprKind::QuantumOp(_) => true,
        _ => false,
    }
}

fn is_quantum_gate_name(name: &str) -> bool {
    matches!(
        name,
        "H" | "X"
            | "Y"
            | "Z"
            | "S"
            | "T"
            | "CX"
            | "CY"
            | "CZ"
            | "RX"
            | "RY"
            | "RZ"
            | "hadamard"
            | "cnot"
            | "measure"
    )
}

/// Prove uncomputation for a single quantum function.
fn prove_function_uncomputation(func: &Function) -> Result<Vec<VerifyDiagnostic>, VerifyError> {
    let mut ctx = LoweringContext::new();
    ctx.current_function = Some(func.name.name.clone());

    // Enter function scope for quantum tracker
    ctx.quantum.current_function = Some(func.name.name.clone());

    // Encode quantum operations
    if let Some(body) = &func.body {
        let _ = crate::quantum::encode_quantum_expr(&body.expr, &mut ctx.quantum)?;
        for stmt in &body.stmts {
            let _ = crate::quantum::encode_quantum_stmt(stmt, &mut ctx.quantum)?;
        }
    }

    // Get temporary qubits
    let temp_qubits: Vec<String> = ctx.quantum.temp_qubit_ids().to_vec();

    if temp_qubits.is_empty() {
        return Ok(Vec::new()); // No temps to verify
    }

    // Generate uncomputation constraints
    let uncomputation_constraints = ctx.quantum.generate_uncomputation_constraints();
    let gate_constraints = ctx.quantum.generate_gate_constraints();

    // Add all constraints to script
    for constraint in uncomputation_constraints {
        ctx.script.assert(constraint);
    }
    for constraint in gate_constraints {
        ctx.script.assert(constraint);
    }

    // Also add quantity and other constraints for completeness
    // (In practice, we'd run the full lowering pipeline)

    // Finalize and solve
    let script = ctx.finalize()?;
    let smt_script = script.to_string();

    // Use thorough config for verification
    let config = SolverConfig::thorough();
    let result = verify(&smt_script, config)?;

    // Extract diagnostics from result
    let mut diagnostics = Vec::new();
    for temp_id in temp_qubits {
        if let Some(qubit) = ctx.quantum.get_qubit(&temp_id) {
            let diag = extract_uncomputation_diagnostic(&result, &func.name.name, qubit);
            if let Some(d) = diag {
                diagnostics.push(d);
            }
        }
    }

    Ok(diagnostics)
}

/// Extract diagnostic from verification result.
fn extract_uncomputation_diagnostic(
    result: &crate::solver::VerifyResult,
    func_name: &str,
    qubit: &crate::quantum::SymbolicQubit,
) -> Option<VerifyDiagnostic> {
    match result {
        crate::solver::VerifyResult::Sat(model) => {
            // SAT means the negation of the property holds - i.e., final state != |0>
            // This is a counterexample: uncomputation failed
            let final_state = model.get_int(&qubit.state_var).unwrap_or(2);
            Some(VerifyDiagnostic {
                code: "NASO-UNC-001".to_string(),
                message: format!(
                    "Quantum uncomputation failed in '{}': temporary qubit '{}' not returned to |0> (final state: {})",
                    func_name,
                    qubit.id,
                    state_to_string(final_state)
                ),
                span: qubit.span,
                severity: crate::model::DiagnosticSeverity::Error,
                related: vec![],
                fix: Some(crate::model::CodeFix {
                    title: "Add explicit uncomputation before scope exit".to_string(),
                    edits: vec![],
                }),
            })
        }
        crate::solver::VerifyResult::Unsat(_) => {
            // UNSAT means the property holds (final state = |0> is proven)
            None
        }
        crate::solver::VerifyResult::Unknown(reason) => Some(VerifyDiagnostic {
            code: "NASO-UNC-002".to_string(),
            message: format!(
                "Could not verify uncomputation for '{}': {}",
                func_name, reason
            ),
            span: qubit.span,
            severity: crate::model::DiagnosticSeverity::Warning,
            related: vec![],
            fix: None,
        }),
        crate::solver::VerifyResult::Error(msg) => Some(VerifyDiagnostic {
            code: "NASO-UNC-ERR".to_string(),
            message: format!("Verification error for '{}': {}", func_name, msg),
            span: qubit.span,
            severity: crate::model::DiagnosticSeverity::Error,
            related: vec![],
            fix: None,
        }),
    }
}

/// Convert state integer to string.
fn state_to_string(state: i64) -> &'static str {
    match state {
        0 => "|0⟩",
        1 => "|1⟩",
        2 => "superposition",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_to_string() {
        assert_eq!(state_to_string(0), "|0⟩");
        assert_eq!(state_to_string(1), "|1⟩");
        assert_eq!(state_to_string(2), "superposition");
    }
}
