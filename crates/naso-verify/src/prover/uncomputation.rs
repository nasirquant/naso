//! Quantum Uncomputation Safety Prover.
//!
//! This prover verifies that all temporary qubits allocated via `qalloc`
//! are returned to the |0> state before scope exit. It encodes the
//! quantum circuit as symbolic unitary matrices and proves U_temp |0> = |0>.

#[cfg(feature = "z3")]
use crate::config::SolverConfig;
#[cfg(feature = "z3")]
use crate::error::VerifyError;
#[cfg(feature = "z3")]
use crate::lower::LoweringContext;
#[cfg(feature = "z3")]
use crate::model::VerifyDiagnostic;
#[cfg(feature = "z3")]
use crate::quantum::{GateKind, QuantumTracker};
#[cfg(feature = "z3")]
use crate::solver::verify;
use naso_compiler::ast::{Function, Program};

/// Run the uncomputation prover on all quantum functions in the AST.
#[cfg(feature = "z3")]
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
    contains_quantum_ops(&func.body)
}

/// Check if a block contains quantum operations.
fn contains_quantum_ops(body: &naso_compiler::ast::Block) -> bool {
    if let Some(expr) = &body.expr {
        if contains_quantum_expr(expr) {
            return true;
        }
    }
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
        _ => false,
    }
}

fn contains_quantum_expr(expr: &naso_compiler::ast::Expr) -> bool {
    match &expr.kind {
        naso_compiler::ast::ExprKind::Call(func, args) => {
            if let naso_compiler::ast::ExprKind::Var(name) = &func.kind {
                if is_quantum_gate_name(&name.name) || name.name == "qalloc" || name.name == "qfree"
                {
                    return true;
                }
            }
            args.iter().any(contains_quantum_expr)
        }
        naso_compiler::ast::ExprKind::Let(binding) => contains_quantum_expr(&binding.value),
        naso_compiler::ast::ExprKind::LetInOut(binding) => contains_quantum_expr(&binding.value),
        naso_compiler::ast::ExprKind::LetConsume(binding) => contains_quantum_expr(&binding.value),
        naso_compiler::ast::ExprKind::If(_, then_e, else_e) => {
            contains_quantum_expr(then_e)
                || else_e.as_ref().map_or(false, |e| contains_quantum_expr(e))
        }
        naso_compiler::ast::ExprKind::QuantumOp(_) => true,
        naso_compiler::ast::ExprKind::Block(block) => contains_quantum_ops(block),
        naso_compiler::ast::ExprKind::Binary(_, lhs, rhs) => {
            contains_quantum_expr(lhs) || contains_quantum_expr(rhs)
        }
        naso_compiler::ast::ExprKind::Unary(_, operand) => contains_quantum_expr(operand),
        naso_compiler::ast::ExprKind::MethodCall(receiver, _, args) => {
            contains_quantum_expr(receiver) || args.iter().any(contains_quantum_expr)
        }
        _ => false,
    }
}

fn is_quantum_gate_name(name: &str) -> bool {
    matches!(
        name,
        "H" | "X" | "Y" | "Z" | "S" | "T" | "CX" | "CY" | "CZ" | "RX" | "RY" | "RZ" | "hadamard" | "cnot" | "measure"
    )
}

/// Prove uncomputation for a single quantum function.
#[cfg(feature = "z3")]
fn prove_function_uncomputation(func: &Function) -> Result<Vec<VerifyDiagnostic>, VerifyError> {
    let mut ctx = LoweringContext::new();
    ctx.current_function = Some(func.name.name.clone());

    ctx.quantum.current_function = Some(func.name.name.clone());

    let mut constraints = Vec::new();
    if let Some(body_expr) = &func.body.expr {
        constraints.extend(crate::quantum::encode_quantum_expr(body_expr.as_ref(), &mut ctx.quantum)?);
    }
    for stmt in &func.body.stmts {
        if let naso_compiler::ast::StmtKind::Expr(expr) = &stmt.kind {
            let _ = crate::quantum::encode_quantum_expr(expr, &mut ctx.quantum)?;
        }
    }

    let temp_qubits: Vec<String> = ctx.quantum.temp_qubit_ids().to_vec();

    if temp_qubits.is_empty() {
        return Ok(Vec::new());
    }

    let uncomputation_constraints = ctx.quantum.generate_uncomputation_constraints();
    let gate_constraints = ctx.quantum.generate_gate_constraints();

    for constraint in uncomputation_constraints {
        ctx.script.assert(constraint);
    }
    for constraint in gate_constraints {
        ctx.script.assert(constraint);
    }

    let script = ctx.finalize()?;
    let smt_script = script.to_string();

    let config = SolverConfig::thorough();
    let result = verify(&smt_script, config)?;

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
#[cfg(feature = "z3")]
fn extract_uncomputation_diagnostic(
    result: &crate::solver::VerifyResult,
    func_name: &str,
    qubit: &crate::quantum::SymbolicQubit,
) -> Option<VerifyDiagnostic> {
    match result {
        crate::solver::VerifyResult::Sat(model) => {
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
        crate::solver::VerifyResult::Unsat(_) => None,
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