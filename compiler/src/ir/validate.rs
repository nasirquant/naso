//! IR Validation
//!
//! Comprehensive well-formedness checks for PIR modules.

use super::affine_domain::AffineDomain;
use super::affine_map::AffineMap;
use super::schedule_tree::{ScheduleNode, ScheduleTree, StmtId, ScheduleValidationError};
use super::access_relation::{AccessRelation, AccessRelations, AccessType};
use super::pir_types::{PirModule, PirStatement, PirExpr, ValidationError, QuantityMap};
use crate::ast::{Quantity, Mutability};
use serde::{Deserialize, Serialize};

/// Validate a PIR module comprehensively
pub fn validate_pir(module: &PirModule) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // 1. Validate schedule tree
    if let Err(e) = module.schedule.validate() {
        errors.push(ValidationError::ScheduleError(e.to_string()));
    }

    // 2. Check all statements have corresponding domain nodes in schedule
    let scheduled_domains = module.schedule.collect_domains();
    let scheduled_ids: std::collections::HashSet<_> = scheduled_domains.iter().map(|(id, _)| *id).collect();

    for stmt in &module.statements {
        if !scheduled_ids.contains(&stmt.id) {
            errors.push(ValidationError::UnscheduledStatement(stmt.id));
        }
    }

    // 3. Check for duplicate statement IDs
    let mut seen_ids = std::collections::HashSet::new();
    for stmt in &module.statements {
        if !seen_ids.insert(stmt.id) {
            errors.push(ValidationError::DuplicateStatementId(stmt.id));
        }
    }

    // 4. Validate access relations match statements
    for access in &module.accesses.relations {
        let stmt_exists = module.statements.iter().any(|s| s.id == access.stmt_id);
        if !stmt_exists {
            errors.push(ValidationError::AccessDomainMismatch(access.stmt_id));
        }

        // Check access domain matches statement domain
        let stmt_domain = module.statements.iter()
            .find(|s| s.id == access.stmt_id)
            .map(|s| &s.domain);

        if let Some(sd) = stmt_domain {
            if access.stmt_domain.dims != sd.dims
                || access.stmt_domain.n_iter != sd.n_iter
                || access.stmt_domain.n_param != sd.n_param {
                errors.push(ValidationError::AccessDomainMismatch(access.stmt_id));
            }
        }
    }

    // 5. Quantity consistency: [0] vars should not appear in runtime schedule
    for (var, qty) in &module.quantities {
        if qty == &Quantity::Zero {
            for stmt in &module.statements {
                if expr_contains_var(&stmt.body, var) {
                    errors.push(ValidationError::ZeroQuantityInRuntime(var.clone(), stmt.id));
                }
            }
        }
    }

    // 6. Linearity check: [1] vars should be used exactly once (simplified)
    for (var, qty) in &module.quantities {
        if qty == &Quantity::One {
            let count = count_var_occurrences(module, var);
            if count == 0 {
                errors.push(ValidationError::LinearVarNotUsed(var.clone()));
            } else if count > 1 {
                errors.push(ValidationError::LinearVarUsedMultipleTimes(var.clone(), count));
            }
        }
    }

    // 7. Domain non-emptiness check
    for stmt in &module.statements {
        if stmt.domain.is_empty() {
            errors.push(ValidationError::ScheduleError(
                format!("Statement {} has empty domain", stmt.id)
            ));
        }
    }

    // 8. Access relation consistency
    for access in &module.accesses.relations {
        if access.access_map.pieces.is_empty() {
            errors.push(ValidationError::ScheduleError(
                format!("Access for statement {} has empty map", access.stmt_id)
            ));
        }
    }

    // 9. Schedule domain coverage: all scheduled domains should have statements
    for (stmt_id, domain) in &scheduled_domains {
        let has_stmt = module.statements.iter().any(|s| s.id == *stmt_id);
        if !has_stmt {
            errors.push(ValidationError::ScheduleError(
                format!("Scheduled domain for {} has no corresponding statement", stmt_id)
            ));
        }

        if domain.is_empty() {
            errors.push(ValidationError::ScheduleError(
                format!("Scheduled domain for {} is empty", stmt_id)
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Check if expression contains a variable
fn expr_contains_var(expr: &PirExpr, var: &str) -> bool {
    match expr {
        PirExpr::Var(v) => v == var,
        PirExpr::Binary { left, right, .. } => {
            expr_contains_var(left, var) || expr_contains_var(right, var)
        }
        PirExpr::Let { value, body, .. } => {
            expr_contains_var(value, var) || expr_contains_var(body, var)
        }
        PirExpr::Unary { expr, .. } => expr_contains_var(expr, var),
        PirExpr::Call { args, .. } => args.iter().any(|a| expr_contains_var(a, var)),
        PirExpr::Index { base, indices } => {
            expr_contains_var(base, var) || indices.iter().any(|i| expr_contains_var(i, var))
        }
        PirExpr::Field { base, .. } => expr_contains_var(base, var),
        PirExpr::If { cond, then_branch, else_branch } => {
            expr_contains_var(cond, var) ||
            expr_contains_var(then_branch, var) ||
            expr_contains_var(else_branch, var)
        }
        PirExpr::Reversible { body, inverse } => {
            expr_contains_var(body, var) || expr_contains_var(inverse, var)
        }
        PirExpr::IntLit(_) | PirExpr::FloatLit(_) | PirExpr::BoolLit(_) => false,
    }
}

/// Count variable occurrences in module
fn count_var_occurrences(module: &PirModule, var: &str) -> usize {
    module.statements.iter()
        .map(|s| count_in_expr(&s.body, var))
        .sum()
}

fn count_in_expr(expr: &PirExpr, var: &str) -> usize {
    match expr {
        PirExpr::Var(v) if v == var => 1,
        PirExpr::Binary { left, right, .. } => {
            count_in_expr(left, var) + count_in_expr(right, var)
        }
        PirExpr::Let { value, body, .. } => {
            count_in_expr(value, var) + count_in_expr(body, var)
        }
        PirExpr::Unary { expr, .. } => count_in_expr(expr, var),
        PirExpr::Call { args, .. } => args.iter().map(|a| count_in_expr(a, var)).sum(),
        PirExpr::Index { base, indices } => {
            count_in_expr(base, var) + indices.iter().map(|i| count_in_expr(i, var)).sum::<usize>()
        }
        PirExpr::Field { base, .. } => count_in_expr(base, var),
        PirExpr::If { cond, then_branch, else_branch } => {
            count_in_expr(cond, var) + count_in_expr(then_branch, var) + count_in_expr(else_branch, var)
        }
        PirExpr::Reversible { body, inverse } => {
            count_in_expr(body, var) + count_in_expr(inverse, var)
        }
        _ => 0,
    }
}

/// Validate affine domain structure
pub fn validate_domain(domain: &AffineDomain) -> Result<(), String> {
    if domain.dims != domain.n_iter + domain.n_param {
        return Err("Domain dims != n_iter + n_param".to_string());
    }

    for c in &domain.constraints {
        if c.coefficients.len() != domain.dims {
            return Err(format!("Constraint dimension mismatch: {} vs {}", c.coefficients.len(), domain.dims));
        }
    }

    Ok(())
}

/// Validate affine map structure
pub fn validate_map(map: &AffineMap, expected_input_dims: usize) -> Result<(), String> {
    if map.pieces.is_empty() {
        return Err("Affine map has no pieces".to_string());
    }

    for piece in &map.pieces {
        if piece.domain.dims != expected_input_dims {
            return Err(format!("Piece domain dims {} != expected {}", piece.domain.dims, expected_input_dims));
        }
        if piece.matrix.cols != expected_input_dims {
            return Err(format!("Matrix cols {} != expected {}", piece.matrix.cols, expected_input_dims));
        }
        if piece.matrix.rows == 0 {
            return Err("Matrix has zero rows".to_string());
        }
    }

    // Check pieces have disjoint domains (simplified)
    for i in 0..map.pieces.len() {
        for j in i+1..map.pieces.len() {
            let inter = map.pieces[i].domain.intersection(&map.pieces[j].domain);
            if !inter.is_empty() {
                // Warning: overlapping pieces (not an error, but may cause ambiguity)
            }
        }
    }

    Ok(())
}

/// Validate schedule tree with detailed checks
pub fn validate_schedule_detailed(tree: &ScheduleTree) -> Result<ScheduleValidationReport, Vec<String>> {
    let mut warnings = Vec::new();
    let mut band_count = 0;
    let mut domain_count = 0;
    let mut max_depth = 0;

    fn check_node(node: &ScheduleNode, depth: usize, band_count: &mut usize, domain_count: &mut usize, max_depth: &mut usize, warnings: &mut Vec<String>) {
        *max_depth = (*max_depth).max(depth);

        match node {
            ScheduleNode::Band { members, coincident, child } => {
                *band_count += 1;
                if members.is_empty() {
                    warnings.push(format!("Band at depth {} has no members", depth));
                }
                if members.len() != coincident.len() {
                    warnings.push(format!("Band at depth {}: coincident length mismatch", depth));
                }
                check_node(child, depth + 1, band_count, domain_count, max_depth, warnings);
            }
            ScheduleNode::Filter { domain, child } => {
                if domain.is_empty() {
                    warnings.push(format!("Filter at depth {} has empty domain", depth));
                }
                check_node(child, depth + 1, band_count, domain_count, max_depth, warnings);
            }
            ScheduleNode::Sequence { children } => {
                if children.is_empty() {
                    warnings.push(format!("Sequence at depth {} has no children", depth));
                }
                for c in children {
                    check_node(c, depth + 1, band_count, domain_count, max_depth, warnings);
                }
            }
            ScheduleNode::Context { domain, child } => {
                if domain.is_empty() {
                    warnings.push(format!("Context at depth {} has empty domain", depth));
                }
                check_node(child, depth + 1, band_count, domain_count, max_depth, warnings);
            }
            ScheduleNode::Domain { domain, .. } => {
                *domain_count += 1;
                if domain.is_empty() {
                    warnings.push(format!("Domain node at depth {} is empty", depth));
                }
            }
            ScheduleNode::Extension { child, .. } => {
                check_node(child, depth + 1, band_count, domain_count, max_depth, warnings);
            }
            ScheduleNode::Empty => {
                warnings.push(format!("Empty node at depth {}", depth));
            }
        }
    }

    check_node(&tree.root, 0, &mut band_count, &mut domain_count, &mut max_depth, &mut warnings);

    Ok(ScheduleValidationReport {
        band_count,
        domain_count,
        max_depth,
        parameter_count: tree.parameters.len(),
        warnings,
    })
}

/// Schedule validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleValidationReport {
    pub band_count: usize,
    pub domain_count: usize,
    pub max_depth: usize,
    pub parameter_count: usize,
    pub warnings: Vec<String>,
}

/// Validate access relations for dependence analysis readiness
pub fn validate_accesses_for_dependence(accesses: &AccessRelations) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    for access in &accesses.relations {
        if access.access_map.pieces.is_empty() {
            errors.push(format!("Access {} has no map pieces", access.stmt_id));
        }
    }

    // Check for self-dependence (same stmt read/write)
    for i in 0..accesses.relations.len() {
        for j in i+1..accesses.relations.len() {
            let a = &accesses.relations[i];
            let b = &accesses.relations[j];

            if a.stmt_id == b.stmt_id && a.is_write() && b.is_write() {
                // Potential WAW within same statement - check if domains overlap
                if a.may_alias(b) {
                    errors.push(format!("Potential WAW dependence within statement {}", a.stmt_id));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::affine_domain::AffineDomain;
    use super::super::affine_map::{AffineMap, Matrix};
    use super::super::schedule_tree::{ScheduleNode, ScheduleTree, StmtId};
    use super::super::access_relation::{AccessRelation, AccessRelations, AccessType};
    use super::super::pir_types::{PirModule, PirStatement, PirExpr, QuantityMap};
    use crate::ast::{Quantity, Mutability};

    #[test]
    fn test_validate_pir_success() {
        let domain = AffineDomain::universe(1, 0);
        let mut m = Matrix::new(1, 1);
        m.set(0, 0, 1);
        let map = AffineMap::total(domain.clone(), m);

        let stmt = PirStatement {
            id: StmtId(0),
            domain: domain.clone(),
            body: PirExpr::IntLit(42),
            quantity: Quantity::Many,
            mutability: Mutability::Immutable,
            span: None,
        };

        let schedule = ScheduleTree::new(
            ScheduleNode::band(
                vec![map.clone()],
                vec![false],
                ScheduleNode::domain(StmtId(0), domain.clone()),
            ),
            vec![],
        );

        let mut accesses = AccessRelations::new();
        accesses.add(AccessRelation::new(StmtId(0), domain.clone(), map, AccessType::Write));

        let mut quantities = QuantityMap::new();
        quantities.insert("x".to_string(), Quantity::Many);

        let module = PirModule::new(vec![stmt], schedule, accesses, quantities, vec![]);
        assert!(validate_pir(&module).is_ok());
    }

    #[test]
    fn test_validate_zero_quantity_error() {
        let domain = AffineDomain::universe(1, 0);
        let mut m = Matrix::new(1, 1);
        m.set(0, 0, 1);
        let map = AffineMap::total(domain.clone(), m);

        let stmt = PirStatement {
            id: StmtId(0),
            domain: domain.clone(),
            body: PirExpr::Var("x".to_string()),
            quantity: Quantity::Many,
            mutability: Mutability::Immutable,
            span: None,
        };

        let schedule = ScheduleTree::new(
            ScheduleNode::band(
                vec![map.clone()],
                vec![false],
                ScheduleNode::domain(StmtId(0), domain.clone()),
            ),
            vec![],
        );

        let mut accesses = AccessRelations::new();
        accesses.add(AccessRelation::new(StmtId(0), domain.clone(), map, AccessType::Write));

        let mut quantities = QuantityMap::new();
        quantities.insert("x".to_string(), Quantity::Zero);

        let module = PirModule::new(vec![stmt], schedule, accesses, quantities, vec![]);
        let result = validate_pir(&module);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, ValidationError::ZeroQuantityInRuntime(_, _))));
    }

    #[test]
    fn test_validate_domain() {
        let domain = AffineDomain::universe(2, 1);
        assert!(validate_domain(&domain).is_ok());

        let bad_domain = AffineDomain {
            dims: 3,
            n_iter: 2,
            n_param: 2, // Wrong: 2+2 != 3
            constraints: vec![],
            name: None,
        };
        assert!(validate_domain(&bad_domain).is_err());
    }

    #[test]
    fn test_validate_schedule_detailed() {
        let domain = AffineDomain::universe(2, 0);
        let mut m = Matrix::new(1, 2);
        m.set(0, 0, 1);
        let map = AffineMap::total(domain.clone(), m);

        let tree = ScheduleTree::new(
            ScheduleNode::band(
                vec![map.clone()],
                vec![false],
                ScheduleNode::band(
                    vec![map.clone()],
                    vec![true],
                    ScheduleNode::domain(StmtId(0), domain),
                ),
            ),
            vec![],
        );

        let report = validate_schedule_detailed(&tree).unwrap();
        assert_eq!(report.band_count, 2);
        assert_eq!(report.domain_count, 1);
        assert_eq!(report.max_depth, 2);
    }
}