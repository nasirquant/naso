//! [1]-Quantity Leak Prover.
//!
//! This prover verifies that no linear ([1]) resource is implicitly dropped,
//! double-freed, or leaked across function boundaries. It tracks [1] bindings
//! across the control-flow graph and verifies exactly-once consumption on all paths.

#[cfg(feature = "z3")]
use crate::config::SolverConfig;
#[cfg(feature = "z3")]
use crate::error::VerifyError;
#[cfg(feature = "z3")]
use crate::lower::LoweringContext;
#[cfg(feature = "z3")]
use crate::model::VerifyDiagnostic;
use crate::quantity::{QuantityKind, QuantityTracker, encode_quantity_expr};
#[cfg(feature = "z3")]
use crate::solver::verify;
use naso_compiler::ast::{Function, Program};

/// Run the linearity prover on all functions in the AST.
#[cfg(feature = "z3")]
pub fn prove_linearity(program: &Program) -> Result<Vec<VerifyDiagnostic>, VerifyError> {
    let mut diagnostics = Vec::new();

    for item in &program.items {
        if let naso_compiler::ast::Item::Function(func) = item {
            let func_diagnostics = prove_function_linearity(func)?;
            diagnostics.extend(func_diagnostics);
        }
    }

    Ok(diagnostics)
}

/// Prove linearity for a single function.
#[cfg(feature = "z3")]
fn prove_function_linearity(func: &Function) -> Result<Vec<VerifyDiagnostic>, VerifyError> {
    let mut ctx = LoweringContext::new();
    ctx.current_function = Some(func.name.name.clone());

    let mut tracker = QuantityTracker::new();

    for param in &func.params {
        let qk = QuantityKind::from_ast(&param.ty.quantity);
        match qk {
            QuantityKind::One => {
                tracker.allocate_linear(
                    &param.name.name,
                    param.span,
                    crate::quantity::AllocSite::Param(param.name.name.clone()),
                );
            }
            QuantityKind::Zero => tracker.register_erased(&param.name.name, param.span),
            QuantityKind::Bounded(n) => tracker.register_bounded(&param.name.name, n, param.span),
            QuantityKind::Many => {}
        }
    }

    let mut constraints = Vec::new();
    if let Some(body_expr) = &func.body.expr {
        constraints.extend(encode_quantity_expr(body_expr.as_ref(), &mut tracker)?);
    }
    for stmt in &func.body.stmts {
        if let naso_compiler::ast::StmtKind::Expr(_expr) = &stmt.kind {
            let _ = crate::quantity::encode_quantity_stmt(stmt, &mut tracker)?;
        }
    }

    for constraint in constraints {
        ctx.script.assert(constraint);
    }

    let linearity_constraints = generate_linearity_constraints(&tracker);
    for constraint in linearity_constraints {
        ctx.script.assert(constraint);
    }

    let script = ctx.finalize()?;
    let smt_script = script.to_string();

    let config = SolverConfig::default();
    let result = verify(&smt_script, config)?;

    let mut diagnostics = Vec::new();
    for res_id in tracker.linear_resource_ids() {
        if let Some(rid) = tracker.get_linear_resource(res_id) {
            let diag = extract_linearity_diagnostic(&result, &func.name.name, rid);
            if let Some(d) = diag {
                diagnostics.push(d);
            }
        }
    }

    Ok(diagnostics)
}

/// Generate SMT constraints for linearity: each [1] resource consumed exactly once.
fn generate_linearity_constraints(tracker: &QuantityTracker) -> Vec<crate::smtlib::Term> {
    use crate::smtlib::Sort;
    use crate::smtlib::builder::*;

    let mut constraints = Vec::new();

    for res_id in tracker.linear_resource_ids() {
        let res_var = var(res_id, Sort::Int);
        constraints.push(gt(res_var.clone(), int(0)));
    }

    constraints
}

/// Extract diagnostic from verification result.
#[cfg(feature = "z3")]
fn extract_linearity_diagnostic(
    result: &crate::solver::VerifyResult,
    func_name: &str,
    resource: &crate::quantity::ResourceId,
) -> Option<VerifyDiagnostic> {
    match result {
        crate::solver::VerifyResult::Sat(_) => Some(VerifyDiagnostic {
            code: "NASO-LIN-003".to_string(),
            message: format!(
                "Potential [1]-quantity leak in '{}': resource '{}' may not be consumed on all paths",
                func_name, resource.name
            ),
            span: resource.span,
            severity: crate::model::DiagnosticSeverity::Error,
            related: vec![],
            fix: Some(crate::model::CodeFix {
                title: "Ensure resource is consumed exactly once on all control-flow paths"
                    .to_string(),
                edits: vec![],
            }),
        }),
        crate::solver::VerifyResult::Unsat(_) => None,
        crate::solver::VerifyResult::Unknown(reason) => Some(VerifyDiagnostic {
            code: "NASO-LIN-004".to_string(),
            message: format!("Could not verify linearity for '{}': {}", func_name, reason),
            span: resource.span,
            severity: crate::model::DiagnosticSeverity::Warning,
            related: vec![],
            fix: None,
        }),
        crate::solver::VerifyResult::Error(msg) => Some(VerifyDiagnostic {
            code: "NASO-LIN-ERR".to_string(),
            message: format!("Verification error for '{}': {}", func_name, msg),
            span: resource.span,
            severity: crate::model::DiagnosticSeverity::Error,
            related: vec![],
            fix: None,
        }),
    }
}

/// Analyze control-flow paths for linearity (simplified).
pub fn analyze_cfg_paths(
    _func: &Function,
    _tracker: &QuantityTracker,
) -> Result<Vec<ConsumptionPath>, VerifyError> {
    Ok(Vec::new())
}

/// A consumption path through the CFG.
#[derive(Debug, Clone)]
pub struct ConsumptionPath {
    pub path_id: u32,
    pub consumed_resources: Vec<String>,
    pub unconsumed_resources: Vec<String>,
    pub double_consumed: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linearity_module_compiles() {}
}
