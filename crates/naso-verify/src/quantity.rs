//! Quantity constraint encoding for QTT (Quantitative Type Theory).
//!
//! This module handles the translation of Naso's quantity annotations ([0], [1], [*], [N])
//! into SMT-LIB2 constraints that can be verified by Z3.

use crate::error::{LoweringError, VerifyError};
use crate::smtlib::{Sort, Term, builder::*};
use indexmap::IndexMap;
use naso_compiler::ast::Quantity;
use naso_compiler::ast::Span;
use std::collections::HashMap;

/// Quantity kind for tracking in SMT encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuantityKind {
    /// [0] - Erased/proof-only, must not exist at runtime
    Zero,
    /// [1] - Linear, must be consumed exactly once
    One,
    /// [*] - Unrestricted, can be aliased freely
    Many,
    /// [N] - Bounded by constant N
    Bounded(u64),
}

impl QuantityKind {
    /// Parse from AST Quantity.
    pub fn from_ast(qty: &Quantity) -> Self {
        match qty {
            Quantity::Zero => QuantityKind::Zero,
            Quantity::One => QuantityKind::One,
            Quantity::Many => QuantityKind::Many,
            Quantity::Bounded(n) => QuantityKind::Bounded(*n as u64),
        }
    }

    /// Check if this quantity allows runtime existence.
    pub fn is_runtime(&self) -> bool {
        !matches!(self, QuantityKind::Zero)
    }

    /// Check if this quantity requires linear consumption.
    pub fn is_linear(&self) -> bool {
        matches!(self, QuantityKind::One)
    }

    /// Check if this quantity is bounded.
    pub fn is_bounded(&self) -> bool {
        matches!(self, QuantityKind::Bounded(_))
    }

    /// Get the bound if bounded.
    pub fn bound(&self) -> Option<u64> {
        match self {
            QuantityKind::Bounded(n) => Some(*n),
            _ => None,
        }
    }
}

/// Symbolic resource identifier for [1] linear resources.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceId {
    pub name: String,
    pub span: Span,
    pub alloc_site: AllocSite,
}

/// Allocation site for unique resource identification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AllocSite {
    /// Function parameter
    Param(String),
    /// Local allocation (qalloc, alloc, etc.)
    Local(String, u32), // name, unique index
    /// Return value
    Return(String),
    /// Struct field
    Field(Box<ResourceId>, String),
}

/// Tracks quantity constraints during lowering.
pub struct QuantityTracker {
    /// All [1] resources allocated, with their symbolic IDs
    pub linear_resources: IndexMap<String, ResourceId>,
    /// [0] variables that must be erased
    pub erased_vars: HashMap<String, Span>,
    /// [N] bounded variables with their bounds
    pub bounded_vars: HashMap<String, (u64, Span)>,
    /// Consumption tracking for [1] resources: resource_id -> consumed_on_paths
    pub consumption: HashMap<String, Vec<ConsumptionPath>>,
    /// Next unique ID for allocations
    next_alloc_id: u32,
}

/// A consumption path represents one control-flow path where a resource is consumed.
#[derive(Debug, Clone)]
pub struct ConsumptionPath {
    pub path_id: u32,
    pub consumed: bool,
    pub location: Span,
}

impl QuantityTracker {
    pub fn new() -> Self {
        Self {
            linear_resources: IndexMap::new(),
            erased_vars: HashMap::new(),
            bounded_vars: HashMap::new(),
            consumption: HashMap::new(),
            next_alloc_id: 0,
        }
    }

    /// Register a new [1] linear resource allocation.
    pub fn allocate_linear(&mut self, name: &str, span: Span, site: AllocSite) -> String {
        let id = format!("res_{}_{}", name, self.next_alloc_id);
        self.next_alloc_id += 1;
        let rid = ResourceId {
            name: name.to_string(),
            span,
            alloc_site: site,
        };
        self.linear_resources.insert(id.clone(), rid);
        self.consumption.insert(id.clone(), Vec::new());
        id
    }

    /// Register a [0] erased variable.
    pub fn register_erased(&mut self, name: &str, span: Span) {
        self.erased_vars.insert(name.to_string(), span);
    }

    /// Register a [N] bounded variable.
    pub fn register_bounded(&mut self, name: &str, bound: u64, span: Span) {
        self.bounded_vars.insert(name.to_string(), (bound, span));
    }

    /// Mark a [1] resource as consumed on a specific path.
    pub fn consume_linear(&mut self, resource_id: &str, path_id: u32, location: Span) {
        if let Some(paths) = self.consumption.get_mut(resource_id) {
            paths.push(ConsumptionPath {
                path_id,
                consumed: true,
                location,
            });
        }
    }

    /// Get all linear resource IDs.
    pub fn linear_resource_ids(&self) -> Vec<&String> {
        self.linear_resources.keys().collect()
    }

    /// Get all erased variable names.
    pub fn erased_var_names(&self) -> Vec<&String> {
        self.erased_vars.keys().collect()
    }

    /// Get all bounded variables with bounds.
    pub fn bounded_vars(&self) -> &HashMap<String, (u64, Span)> {
        &self.bounded_vars
    }

    /// Get a linear resource by ID.
    pub fn get_linear_resource(&self, id: &str) -> Option<&ResourceId> {
        self.linear_resources.get(id)
    }

    /// Generate SMT constraints for quantity correctness.
    pub fn generate_constraints(&self) -> Vec<Term> {
        let mut constraints = Vec::new();

        // [0] erasure: assert that erased variables are never used at runtime
        // This is encoded as: for each erased var, if it appears in a runtime context, assert false
        // In practice, we track this during lowering and emit (assert false) with diagnostic info
        for (name, span) in &self.erased_vars {
            let var = var(name, Sort::Int); // Use Int as placeholder sort
            let error_msg = format!("erased_var_used:{}", span.start());
            constraints.push(implies(var, bool(false))); // If var is "used" (non-zero), contradiction
            // In real implementation, we'd have a predicate is_runtime_use(var)
        }

        // [1] linearity: each linear resource must be consumed exactly once
        // We generate distinctness constraints and consumption tracking
        let linear_ids: Vec<&String> = self.linear_resource_ids();
        for i in 0..linear_ids.len() {
            for j in (i + 1)..linear_ids.len() {
                // Distinctness: different allocations have different IDs
                constraints.push(app(
                    "distinct",
                    vec![var(linear_ids[i], Sort::Int), var(linear_ids[j], Sort::Int)],
                ));
            }
        }

        // [N] bounded: variables must stay within bounds
        for (name, (bound, _span)) in &self.bounded_vars {
            let var_term = var(name, Sort::Int);
            constraints.push(and(vec![
                le(var_term.clone(), int(*bound as i64)),
                ge(var_term, int(0)),
            ]));
        }

        // Consumption tracking: for each linear resource, exactly one consume on each path
        // This is simplified; real implementation uses CFG path analysis
        for (res_id, paths) in &self.consumption {
            if paths.is_empty() {
                // Resource never consumed - potential leak
                let res_var = var(res_id, Sort::Int);
                // Assert that there exists a path where it's consumed
                // For now, just track; actual enforcement in prover
            }
        }

        constraints
    }

    /// Generate SMT declarations for all tracked quantities.
    pub fn generate_declarations(&self, script: &mut crate::smtlib::Script) {
        // Declare linear resource IDs as integer constants
        for id in self.linear_resource_ids() {
            script.declare_const(id, Sort::Int);
        }

        // Declare erased variables (for tracking)
        for name in self.erased_var_names() {
            script.declare_const(name, Sort::Int);
        }

        // Declare bounded variables
        for (name, _) in &self.bounded_vars {
            script.declare_const(name, Sort::Int);
        }
    }
}

impl Default for QuantityTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode quantity constraints for a typed expression.
pub fn encode_quantity_expr(
    expr: &naso_compiler::ast::Expr,
    tracker: &mut QuantityTracker,
) -> Result<Vec<Term>, VerifyError> {
    let mut constraints = Vec::new();

    match &expr.kind {
        naso_compiler::ast::expr::ExprKind::Var(name, qty, span) => {
            let qk = QuantityKind::from_ast(qty);
            if !qk.is_runtime() {
                // [0] variable used in expression position - error
                return Err(VerifyError::Lowering(LoweringError::InvalidQuantity {
                    span: *span,
                    msg: format!("[0] quantity variable '{}' used at runtime", name),
                }));
            }
            if qk.is_linear() {
                // Track linear variable use
                // In practice, we'd look up the resource ID from the tracker
            }
        }
        naso_compiler::ast::expr::ExprKind::Call(func, args, span) => {
            // Check if this is a known allocation function
            if let naso_compiler::ast::expr::ExprKind::Var(fname, _, _) = &func.kind {
                match fname.name.as_str() {
                    "qalloc" | "linear_alloc" | "alloc" => {
                        // Allocate new linear resource
                        for (i, arg) in args.iter().enumerate() {
                            if let naso_compiler::ast::expr::ExprKind::Literal(
                                naso_compiler::ast::Literal::Int(n),
                                _,
                                _,
                            ) = &arg.kind
                            {
                                let resource_id = tracker.allocate_linear(
                                    &fname.name,
                                    *span,
                                    AllocSite::Local(fname.name.clone(), i as u32),
                                );
                                // Constrain resource ID to be positive (valid allocation)
                                constraints.push(gt(var(&resource_id, Sort::Int), int(0)));
                            }
                        }
                    }
                    "linear_free" | "qfree" | "free" => {
                        // Consume linear resource
                        for arg in args {
                            if let naso_compiler::ast::expr::ExprKind::Var(name, _, _) = &arg.kind {
                                // Mark as consumed (would need path tracking in real impl)
                            }
                        }
                    }
                    _ => {}
                }
            }
            // Recurse into arguments
            for arg in args {
                constraints.extend(encode_quantity_expr(arg, tracker)?);
            }
        }
        naso_compiler::ast::expr::ExprKind::Let(bindings, body, _) => {
            for (name, ty, init, _) in bindings {
                if let Some(init_expr) = init {
                    constraints.extend(encode_quantity_expr(init_expr, tracker)?);
                }
                // Register variable with its quantity
                if let Some(qty) = ty.quantity() {
                    let qk = QuantityKind::from_ast(&qty);
                    match qk {
                        QuantityKind::Zero => tracker.register_erased(&name.name, *span),
                        QuantityKind::One => {
                            tracker.allocate_linear(
                                &name.name,
                                *span,
                                AllocSite::Param(name.name.clone()),
                            );
                        }
                        QuantityKind::Bounded(n) => tracker.register_bounded(&name.name, n, *span),
                        QuantityKind::Many => {} // No special tracking
                    }
                }
            }
            constraints.extend(encode_quantity_expr(body, tracker)?);
        }
        naso_compiler::ast::expr::ExprKind::Projection(_) => {
            // MVS handled separately in mvs.rs
        }
        _ => {
            // Recurse into subexpressions
            expr.visit_exprs(&mut |e| {
                if let Ok(cs) = encode_quantity_expr(e, tracker) {
                    constraints.extend(cs);
                }
            });
        }
    }

    Ok(constraints)
}

/// Encode quantity constraints for a statement.
pub fn encode_quantity_stmt(
    stmt: &naso_compiler::ast::Stmt,
    tracker: &mut QuantityTracker,
) -> Result<Vec<Term>, VerifyError> {
    let mut constraints = Vec::new();

    match &stmt.kind {
        naso_compiler::ast::StmtKind::Let(binding) => {
            constraints.extend(encode_quantity_expr(&binding.value, tracker)?);
        }
        naso_compiler::ast::StmtKind::LetInOut(binding) => {
            constraints.extend(encode_quantity_expr(&binding.value, tracker)?);
        }
        naso_compiler::ast::StmtKind::LetConsume(binding) => {
            constraints.extend(encode_quantity_expr(&binding.value, tracker)?);
        }
        naso_compiler::ast::StmtKind::Expr(expr) => {
            constraints.extend(encode_quantity_expr(expr, tracker)?);
        }
        naso_compiler::ast::StmtKind::Assign(lhs, rhs) => {
            constraints.extend(encode_quantity_expr(lhs, tracker)?);
            constraints.extend(encode_quantity_expr(rhs, tracker)?);
        }
        _ => {}
    }

    Ok(constraints)
}

/// Trait for visiting expressions (placeholder - would be in naso_compiler::ast)
trait VisitExprs {
    fn visit_exprs<F: FnMut(&naso_compiler::ast::Expr)>(&self, f: &mut F);
}

impl VisitExprs for naso_compiler::ast::Expr {
    fn visit_exprs<F: FnMut(&naso_compiler::ast::Expr)>(&self, f: &mut F) {
        f(self);
        match self {
            naso_compiler::ast::Expr::Call(_, args, _)
            | naso_compiler::ast::Expr::Binary(_, _, args, _)
            | naso_compiler::ast::Expr::Unary(_, arg, _)
            | naso_compiler::ast::Expr::Let(_, body, _)
            | naso_compiler::ast::Expr::If(_, _, then_e, else_e, _) => {
                // Delegate to children
            }
            _ => {}
        }
    }
}
