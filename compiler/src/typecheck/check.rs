//! Type checking functions for the Naso type checker
//!
//! Implements the bidirectional typing rules: check mode (type-directed)
//! Full Quantitative Type Theory (QTT) enforcement:
//! - [0] erased variables: compile-time only, cannot appear at runtime
//! - [1] linear variables: consumed exactly once across all branches
//! - [N] bounded variables: consumed at most N times
//! - [*] unrestricted: any number of uses

use crate::ast::*;
use crate::ast::expr::{LetBinding, LetInOutBinding, LetConsumeBinding};
use crate::typecheck::error::TypeError;
use crate::typecheck::*;
use crate::typecheck::inference::infer_expr;
use crate::typecheck::constraints::{qty_subtype, is_erasable};

/// Check an expression against an expected type (Check mode)
pub fn check_expr(
    checker: &mut TypeChecker,
    expr: &Expr,
    expected: &Type,
) -> Result<(), TypeError> {
    // First, synthesize the type of the expression
    let inferred = infer_expr(checker, expr)?;

    // Unify with expected type (including quantity)
    unify::unify_types(checker, &inferred, expected)?;

    // Additional QTT checks for quantity
    check_quantity_consumption(checker, expr, &inferred, expected)?;

    Ok(())
}

/// Check quantity consumption rules
fn check_quantity_consumption(
    checker: &mut TypeChecker,
    expr: &Expr,
    inferred: &Type,
    expected: &Type,
) -> Result<(), TypeError> {
    // If expected quantity is Zero, the expression must be erasable
    if expected.quantity == Quantity::Zero && !is_erasable(inferred.quantity) {
        return Err(TypeError::QuantityMismatch {
            expected: Quantity::Zero,
            found: inferred.quantity,
            span: expr.span,
        });
    }

    // If inferred is Zero but expected is not, that's an error
    if inferred.quantity == Quantity::Zero && expected.quantity != Quantity::Zero {
        return Err(TypeError::ErasedVariableUsedAtRuntime {
            name: inferred.to_ident(),
            span: expr.span,
        });
    }

    // Check subtyping: inferred qty must be <= expected qty
    if !qty_subtype(inferred.quantity, expected.quantity) {
        return Err(TypeError::QuantityMismatch {
            expected: expected.quantity,
            found: inferred.quantity,
            span: expr.span,
        });
    }

    Ok(())
}

/// Check a statement
pub fn check_stmt(checker: &mut TypeChecker, stmt: &Stmt) -> Result<(), TypeError> {
    match &stmt.kind {
        StmtKind::Let(let_stmt) => check_let(checker, let_stmt),
        StmtKind::LetInOut(let_inout) => check_let_inout(checker, let_inout),
        StmtKind::Expr(expr) => {
            let _ = infer_expr(checker, expr)?;
            Ok(())
        }
        StmtKind::Return(opt_expr) => check_return(checker, opt_expr.as_ref()),
        StmtKind::Item(item) => check_item(checker, item),
        StmtKind::Reversible(block) => check_reversible(checker, block),
        StmtKind::Break(opt_expr) => check_break(checker, opt_expr.as_ref()),
        StmtKind::Continue => Ok(()),
        StmtKind::Empty => Ok(()),
        StmtKind::LetConsume(let_consume) => check_let_consume(checker, let_consume),
        StmtKind::Error => Ok(()),
    }
}
fn check_let(checker: &mut TypeChecker, let_stmt: &LetStmt) -> Result<(), TypeError> {
    // Convert LetStmt to LetBinding for type checking
    let binding = LetBinding {
        name: let_stmt.name.clone(),
        ty: let_stmt.ty.clone(),
        quantity: let_stmt.quantity,
        mutability: let_stmt.mutability,
        value: let_stmt.value.clone(),
        span: let_stmt.span,
    };

    // Infer the type of the initializer
    let init_ty = infer_expr(checker, &binding.value)?;

    // Check if Qubit type requires quantity One
    if matches!(init_ty.kind, TypeKind::Qubit) && binding.quantity != Quantity::One {
        return Err(TypeError::QubitQuantityMismatch {
            found: binding.quantity,
            span: binding.span,
        });
    }

    // If explicit type annotation, check against it
    if let Some(ann_ty) = &binding.ty {
        checker.check_expr(&binding.value, ann_ty)?;
        // Use the annotated type for binding (after checking)
        checker.env.bind_var(
            binding.name.clone(),
            ann_ty.clone(),
            binding.quantity,
            binding.mutability,
        );
    } else {
        // Bind with inferred type
        checker.env.bind_var(
            binding.name.clone(),
            init_ty,
            binding.quantity,
            binding.mutability,
        );
    }

    // Validate quantity/mutability combinations
    validate_binding_quantity_mutability(&binding.name, binding.quantity, binding.mutability, binding.span)?;

    Ok(())
}

/// Check inout let binding
fn check_let_inout(checker: &mut TypeChecker, stmt: &LetInOutStmt) -> Result<(), TypeError> {
    // Convert LetInOutStmt to LetInOutBinding for type checking
    let binding = LetInOutBinding {
        name: stmt.name.clone(),
        ty: stmt.ty.clone(),
        value: stmt.value.clone(),
        span: stmt.span,
    };

    // The value must be a place expression (variable, field access, etc.)
    // with quantity One (unique ownership)
    let value_ty = infer_expr(checker, &binding.value)?;

    // Check that the value has quantity One (linear)
    if value_ty.quantity != Quantity::One {
        return Err(TypeError::InOutRequiresUnique {
            found_qty: value_ty.quantity,
            span: binding.span,
        });
    }

    // Verify it's a place expression (not a temporary)
    if !is_place_expr(&binding.value) {
        return Err(TypeError::InOutRequiresUnique {
            found_qty: Quantity::Many, // Not a place
            span: binding.span,
        });
    }

    // Extract the place for alias tracking
    let place = expr_to_place(&binding.value)?;

    // Start inout borrow (checks for aliasing)
    checker
        .env
        .borrow_inout(binding.name.clone(), place, binding.span)?;

    // Bind as inout with the inferred type
    checker.env.bind_var(
        binding.name.clone(),
        value_ty,
        Quantity::One,
        Mutability::InOut,
    );

    Ok(())
}

/// Check return statement
fn check_return(checker: &mut TypeChecker, opt_expr: Option<&Expr>) -> Result<(), TypeError> {
    if let Some(expr) = opt_expr {
        let expr_ty = infer_expr(checker, expr)?;
        if let Some(expected) = checker.current_fn_ret.clone() {
            unify::unify_types(checker, &expr_ty, &expected)?;
        }
    } else {
        // Empty return - check against Unit
        if let Some(expected) = checker.current_fn_ret.clone() {
            unify::unify_types(checker, &Type::unit(Span::default()), &expected)?;
        }
    }
    Ok(())
}

/// Check item (type, function, etc.)
fn check_item(checker: &mut TypeChecker, item: &Item) -> Result<(), TypeError> {
    match item {
        Item::Function(f) => checker.check_function(f),
        Item::TypeDef(t) => {
            checker.env.insert_type_def(t.clone());
            Ok(())
        }
        Item::Const(c) => {
            checker.env.insert_const(c.clone());
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Check reversible block
fn check_reversible(checker: &mut TypeChecker, block: &ReversibleBlock) -> Result<(), TypeError> {
    let prev_reversible = checker.in_reversible;
    checker.in_reversible = true;

    let guard = checker.env.enter_scope();

    // Check body statements in pure mode
    for stmt in &block.body.stmts {
        check_stmt(checker, stmt)?;

        // Verify no impure operations
        check_pure_statement(checker, stmt)?;
    }

    // Verify all variables have uncomputation steps (stub)
    // In real implementation, check uncompute DAG

    checker.env.exit_scope(guard)?;
    checker.in_reversible = prev_reversible;
    Ok(())
}

/// Check that a statement is pure (no I/O, measurement, etc.)
fn check_pure_statement(checker: &mut TypeChecker, stmt: &Stmt) -> Result<(), TypeError> {
    match &stmt.kind {
        StmtKind::Expr(expr) => check_pure_expr(checker, expr),
        StmtKind::Let(_) | StmtKind::LetInOut(_) => Ok(()), // Bindings are pure
        StmtKind::Return(_) => Ok(()),
        StmtKind::Item(_) => Ok(()),
        _ => Err(TypeError::ImpureInReversible {
            operation: "statement".to_string(),
            span: stmt.span,
        }),
    }
}

/// Check that an expression is pure
fn check_pure_expr(checker: &mut TypeChecker, expr: &Expr) -> Result<(), TypeError> {
    match &expr.kind {
        ExprKind::QuantumOp(qop) => match qop {
            QuantumOp::Measure(_) | QuantumOp::Hamiltonian(_, _) => Err(TypeError::ImpureInReversible {
                operation: "quantum measurement/hamiltonian".to_string(),
                span: expr.span,
            }),
            _ => Ok(()),
        },
        ExprKind::Call(callee, _) => {
            // Check if callee is impure
            let callee_ty = infer_expr(checker, callee)?;
            if let TypeKind::Function(_, ret) = &callee_ty.kind {
                if ret.quantity == Quantity::Zero {
                    // Could be pure, but need more analysis
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Check break expression
fn check_break(checker: &mut TypeChecker, opt_expr: Option<&Expr>) -> Result<(), TypeError> {
    // Would need loop context tracking - simplified
    if let Some(expr) = opt_expr {
        infer_expr(checker, expr)?;
    }
    Ok(())
}

/// Check function definition
pub fn check_function(checker: &mut TypeChecker, func: &Function) -> Result<(), TypeError> {
    let prev_ret = checker.current_fn_ret.clone();
    checker.current_fn_ret = func.ret_ty.clone();

    let guard = checker.env.enter_scope();

    // Bind function parameters with their quantities and mutabilities
    for param in &func.params {
        checker.env.bind_var(
            param.name.clone(),
            param.ty.clone(),
            param.quantity,
            param.mutability,
        );

        // Validate parameter quantity/mutability
        validate_binding_quantity_mutability(
            &param.name,
            param.quantity,
            param.mutability,
            param.span,
        )?;
    }

    // Check function body
    check_block(checker, &func.body)?;

    checker.env.exit_scope(guard)?;
    checker.current_fn_ret = prev_ret;
    Ok(())
}

/// Check block
pub fn check_block(checker: &mut TypeChecker, block: &Block) -> Result<(), TypeError> {
    let guard = checker.env.enter_scope();

    for stmt in &block.stmts {
        check_stmt(checker, stmt)?;
    }

    if let Some(expr) = &block.expr {
        infer_expr(checker, expr)?;
    }

    checker.env.exit_scope(guard)?;
    Ok(())
}

/// Check pattern against scrutinee type
pub fn check_pattern(
    checker: &mut TypeChecker,
    pattern: &Pattern,
    scrutinee_ty: &Type,
) -> Result<type_env::PatternBindings, TypeError> {
    let mut bindings = type_env::PatternBindings::new();

    match &pattern.kind {
        PatternKind::Ident(ident) => {
            let info = type_env::VarInfo::new(
                scrutinee_ty.clone(),
                pattern.quantity,
                Mutability::Immutable,
                pattern.span,
            );
            bindings.insert(ident.clone(), info);
            Ok(bindings)
        }
        PatternKind::Wildcard => Ok(bindings),
        PatternKind::Literal(lit) => {
            let lit_ty = literal_type(lit, pattern.span);
            unify::unify_types(checker, &lit_ty, scrutinee_ty)?;
            Ok(bindings)
        }
        PatternKind::Struct(name, fields) => {
            // Look up struct type - clone the fields we need to avoid borrow issues
            let struct_fields = checker
                .env
                .lookup_type(name)
                .and_then(|td| {
                    if let TypeDefKind::Struct(fields) = &td.kind {
                        Some(fields.clone())
                    } else {
                        None
                    }
                });

            if let Some(struct_fields) = struct_fields {
                for field_pat in fields {
                    if let Some(field_def) = struct_fields.iter().find(|f| f.name == field_pat.name) {
                        let field_bindings = checker.check_pattern(&field_pat.pattern, &field_def.ty)?;
                        bindings.extend(field_bindings);
                    }
                }
            }
            Ok(bindings)
        }
        PatternKind::Variant(enum_name, variant_name, fields) => {
            // Look up enum variant - clone the fields we need to avoid borrow issues
            let variant_fields = checker
                .env
                .lookup_type(enum_name)
                .and_then(|td| {
                    if let TypeDefKind::Enum(variants) = &td.kind {
                        variants.iter().find(|v| v.name == *variant_name).map(|v| v.fields.clone())
                    } else {
                        None
                    }
                });

            if let Some(variant_fields) = variant_fields {
                for (field_pat, field_def) in fields.iter().zip(&variant_fields) {
                    let field_bindings = checker.check_pattern(field_pat, &field_def.ty)?;
                    bindings.extend(field_bindings);
                }
            }
            Ok(bindings)
        }
        PatternKind::Tuple(patterns) => {
            if let TypeKind::Tuple(types) = &scrutinee_ty.kind {
                for (pat, ty) in patterns.iter().zip(types) {
                    let field_bindings = checker.check_pattern(pat, ty)?;
                    bindings.extend(field_bindings);
                }
            }
            Ok(bindings)
        }
        PatternKind::Or(a, b) => {
            // All alternatives must bind the same variables with same types
            let mut first_bindings = None;
            for pat in [a.as_ref(), b.as_ref()] {
                let b = checker.check_pattern(pat, scrutinee_ty)?;
                if first_bindings.is_none() {
                    first_bindings = Some(b);
                } else {
                    // Verify compatibility - simplified
                }
            }
            Ok(first_bindings.unwrap_or_default())
        }
        PatternKind::Array(patterns) => {
            if let TypeKind::Array(elem_ty, _) = &scrutinee_ty.kind {
                for pat in patterns {
                    let field_bindings = checker.check_pattern(pat, elem_ty)?;
                    bindings.extend(field_bindings);
                }
            }
            Ok(bindings)
        }
        PatternKind::Range(start, end) => {
            // Range pattern - check both bounds
            let start_ty = Type::new(TypeKind::Int, Quantity::Many, pattern.span);
            let end_ty = Type::new(TypeKind::Int, Quantity::Many, pattern.span);
            unify::unify_types(checker, &start_ty, scrutinee_ty)?;
            unify::unify_types(checker, &end_ty, scrutinee_ty)?;
            Ok(bindings)
        }
        PatternKind::Ref(inner) => {
            // Reference pattern - check inner pattern
            checker.check_pattern(inner, scrutinee_ty)
        }
        PatternKind::InOut(inner) => {
            // Inout pattern - check inner pattern
            checker.check_pattern(inner, scrutinee_ty)
        }
        PatternKind::Consume(inner) => {
            // Consume pattern - check inner pattern
            checker.check_pattern(inner, scrutinee_ty)
        }
        PatternKind::Guard(pat, guard_expr) => {
            let bindings = checker.check_pattern(pat, scrutinee_ty)?;
            checker.check_expr(guard_expr, &Type::bool(guard_expr.span))?;
            Ok(bindings)
        }
        _ => Ok(bindings),
    }
}

/// Get type of a literal
fn literal_type(lit: &Literal, span: Span) -> Type {
    match lit {
        Literal::Int(_) => Type::new(TypeKind::Int, Quantity::Many, span),
        Literal::UInt(_) => Type::new(TypeKind::UInt, Quantity::Many, span),
        Literal::Float(_) => Type::new(TypeKind::Float, Quantity::Many, span),
        Literal::Bool(_) => Type::new(TypeKind::Bool, Quantity::Many, span),
        Literal::String(_) => Type::new(TypeKind::String, Quantity::Many, span),
        Literal::Char(_) => Type::new(TypeKind::Char, Quantity::Many, span),
        Literal::Unit => Type::unit(span),
    }
}

/// Validate quantity/mutability combination
fn validate_binding_quantity_mutability(
    _name: &Ident,
    qty: Quantity,
    mutability: Mutability,
    span: Span,
) -> Result<(), TypeError> {
    match (qty, mutability) {
        (Quantity::Zero, Mutability::InOut) => Err(TypeError::InOutRequiresUnique {
            found_qty: Quantity::Zero,
            span,
        }),
        (Quantity::Zero, Mutability::Consume) => Err(TypeError::QuantityMismatch {
            expected: Quantity::One,
            found: Quantity::Zero,
            span,
        }),
        (Quantity::One, Mutability::InOut) => Ok(()), // Valid: linear inout
        (Quantity::One, Mutability::Consume) => Ok(()), // Valid: linear consume
        (Quantity::Bounded(_), Mutability::InOut) => Ok(()), // Valid: bounded inout
        (Quantity::Bounded(_), Mutability::Consume) => Err(TypeError::QuantityMismatch {
            expected: Quantity::One,
            found: qty,
            span,
        }),
        (Quantity::Many, Mutability::InOut) => Err(TypeError::InOutRequiresUnique {
            found_qty: Quantity::Many,
            span,
        }),
        (Quantity::Many, Mutability::Consume) => Err(TypeError::QuantityMismatch {
            expected: Quantity::One,
            found: Quantity::Many,
            span,
        }),
        _ => Ok(()),
    }
}

/// Check if an expression is a place (assignable)
fn is_place_expr(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        ExprKind::Var(_)
            | ExprKind::Field(_, _)
            | ExprKind::Index(_, _)
            | ExprKind::Projection(_)
            | ExprKind::Unary(UnOp::Deref, _)
    )
}

/// Convert expression to Place for alias tracking
fn expr_to_place(expr: &Expr) -> Result<type_env::Place, TypeError> {
    match &expr.kind {
        ExprKind::Var(ident) => Ok(type_env::Place::Var(ident.clone())),
        ExprKind::Field(base, field) => {
            let base_place = expr_to_place(base)?;
            Ok(type_env::Place::Field(Box::new(base_place), field.clone()))
        }
        ExprKind::Index(base, index) => {
            let base_place = expr_to_place(base)?;
            // For simplicity, use a string key
            Ok(type_env::Place::Index(
                Box::new(base_place),
                format!("{:?}", index),
            ))
        }
        ExprKind::Projection(base) => {
            let base_place = expr_to_place(base)?;
            Ok(type_env::Place::Deref(Box::new(base_place)))
        }
        ExprKind::Unary(UnOp::Deref, base) => {
            let base_place = expr_to_place(base)?;
            Ok(type_env::Place::Deref(Box::new(base_place)))
        }
        _ => Err(TypeError::InOutRequiresUnique {
            found_qty: Quantity::Many,
            span: expr.span,
        }),
    }
}

/// Check consume let binding
fn check_let_consume(checker: &mut TypeChecker, stmt: &LetConsumeStmt) -> Result<(), TypeError> {
    // Convert LetConsumeStmt to LetConsumeBinding for type checking
    let binding = LetConsumeBinding {
        name: stmt.name.clone(),
        ty: stmt.ty.clone(),
        value: stmt.value.clone(),
        span: stmt.span,
    };

    // The value must be a place expression with quantity One (linear)
    let value_ty = infer_expr(checker, &binding.value)?;

    // Check that the value has quantity One (linear)
    if value_ty.quantity != Quantity::One {
        return Err(TypeError::QuantityMismatch {
            expected: Quantity::One,
            found: value_ty.quantity,
            span: binding.span,
        });
    }

    // Verify it's a place expression
    if !is_place_expr(&binding.value) {
        return Err(TypeError::QuantityMismatch {
            expected: Quantity::One,
            found: Quantity::Many,
            span: binding.span,
        });
    }

    // Extract the place for alias tracking
    let place = expr_to_place(&binding.value)?;

    // Mark the source variable as moved
    checker.env.move_var(&place_to_ident(&place), binding.span)?;

    // Bind as consume with the inferred type
    checker.env.bind_var(
        binding.name.clone(),
        value_ty,
        Quantity::One,
        Mutability::Consume,
    );

    Ok(())
}

/// Convert Place back to Ident for move tracking
fn place_to_ident(place: &type_env::Place) -> Ident {
    match place {
        type_env::Place::Var(ident) => ident.clone(),
        type_env::Place::Field(base, _) => place_to_ident(base),
        type_env::Place::Index(base, _) => place_to_ident(base),
        type_env::Place::Deref(base) => place_to_ident(base),
    }
}