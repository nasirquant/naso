//! Polyhedral loop invariant encoding for SMT-LIB2.
//!
//! This module translates Naso's polyhedral loop constructs (forall, tile, fuse)
//! into quantified SMT-LIB2 formulas over iteration domains.

use crate::error::{LoweringError, VerifyError};
use crate::smtlib::{Sort, Term, builder::*};
use indexmap::IndexMap;
use naso_compiler::ast::Span;

/// Polyhedral iteration domain: a set of integer tuples defined by affine constraints.
#[derive(Debug, Clone)]
pub struct IterationDomain {
    /// Dimension names (e.g., ["i", "j", "k"])
    pub dims: Vec<String>,
    /// Affine constraints: A * x + b >= 0
    /// Each constraint is (coefficients, constant)
    pub constraints: Vec<(Vec<i64>, i64)>,
}

impl IterationDomain {
    pub fn new(dims: Vec<String>) -> Self {
        Self {
            dims,
            constraints: Vec::new(),
        }
    }

    pub fn add_constraint(&mut self, coeffs: Vec<i64>, constant: i64) {
        self.constraints.push((coeffs, constant));
    }

    /// Generate SMT formula for: x in domain
    pub fn in_domain(&self, vars: &[Term]) -> Term {
        let mut conds = Vec::new();
        for (coeffs, constant) in &self.constraints {
            // Sum(coeffs[i] * vars[i]) + constant >= 0
            let mut sum_terms = Vec::new();
            for (i, coeff) in coeffs.iter().enumerate() {
                if *coeff != 0 {
                    sum_terms.push(mul(vec![int(*coeff), vars[i].clone()]));
                }
            }
            let sum = if sum_terms.is_empty() {
                int(0)
            } else if sum_terms.len() == 1 {
                sum_terms[0].clone()
            } else {
                add(sum_terms)
            };
            conds.push(ge(add(vec![sum, int(*constant)]), int(0)));
        }
        if conds.is_empty() {
            bool(true)
        } else if conds.len() == 1 {
            conds[0].clone()
        } else {
            and(conds)
        }
    }
}

/// Loop invariant: a property that holds at loop entry and is preserved by each iteration.
#[derive(Debug, Clone)]
pub struct LoopInvariant {
    /// The quantified variables (loop indices)
    pub vars: Vec<(String, Sort)>,
    /// The iteration domain
    pub domain: IterationDomain,
    /// The invariant predicate
    pub predicate: Term,
}

impl LoopInvariant {
    pub fn new(vars: Vec<(String, Sort)>, domain: IterationDomain, predicate: Term) -> Self {
        Self {
            vars,
            domain,
            predicate,
        }
    }

    /// Generate SMT formula: forall vars in domain. predicate
    pub fn to_smt(&self) -> Term {
        let var_terms: Vec<Term> = self
            .vars
            .iter()
            .map(|(name, sort)| var(name, sort.clone()))
            .collect();

        let in_dom = self.domain.in_domain(&var_terms);
        let body = implies(in_dom, self.predicate.clone());

        forall(self.vars.clone(), body)
    }
}

/// Tiling information for polyhedral optimization.
#[derive(Debug, Clone)]
pub struct TilingInfo {
    /// Original loop indices
    pub original_indices: Vec<String>,
    /// Tile sizes
    pub tile_sizes: Vec<u64>,
    /// Tile indices (outer loops)
    pub tile_indices: Vec<String>,
    /// Intra-tile indices (inner loops)
    pub intra_indices: Vec<String>,
}

/// Fusion information for polyhedral optimization.
#[derive(Debug, Clone)]
pub struct FusionInfo {
    /// Loops being fused
    pub loop_ids: Vec<String>,
    /// Fused iteration domain
    pub fused_domain: IterationDomain,
    /// Schedule: mapping from original to fused indices
    pub schedule: IndexMap<String, Vec<Term>>, // original_var -> affine expression in fused vars
}

/// Tracks polyhedral state during lowering.
pub struct PolyhedralTracker {
    /// Current loop invariants being tracked
    invariants: Vec<LoopInvariant>,
    /// Tiling information
    tilings: IndexMap<String, TilingInfo>,
    /// Fusion information
    fusions: IndexMap<String, FusionInfo>,
    /// Current loop nesting level
    loop_depth: u32,
    /// Loop index variables at each level
    loop_indices: Vec<String>,
}

impl PolyhedralTracker {
    pub fn new() -> Self {
        Self {
            invariants: Vec::new(),
            tilings: IndexMap::new(),
            fusions: IndexMap::new(),
            loop_depth: 0,
            loop_indices: Vec::new(),
        }
    }

    /// Enter a forall loop.
    pub fn enter_forall(&mut self, index: String, domain: IterationDomain) {
        self.loop_depth += 1;
        self.loop_indices.push(index);
    }

    /// Exit a forall loop.
    pub fn exit_forall(&mut self) -> Option<String> {
        self.loop_depth = self.loop_depth.saturating_sub(1);
        self.loop_indices.pop()
    }

    /// Add a loop invariant at current nesting level.
    pub fn add_invariant(&mut self, predicate: Term) {
        if self.loop_depth == 0 {
            return; // Not in a loop
        }

        // Build domain from current loop indices
        let mut domain = IterationDomain::new(self.loop_indices.clone());
        // In real implementation, domain constraints come from loop bounds
        // For now, assume 0 <= i < N for each index

        let vars: Vec<(String, Sort)> = self
            .loop_indices
            .iter()
            .map(|name| (name.clone(), Sort::Int))
            .collect();

        self.invariants
            .push(LoopInvariant::new(vars, domain, predicate));
    }

    /// Register tiling transformation.
    pub fn register_tiling(&mut self, loop_id: String, info: TilingInfo) {
        self.tilings.insert(loop_id, info);
    }

    /// Register fusion transformation.
    pub fn register_fusion(&mut self, fusion_id: String, info: FusionInfo) {
        self.fusions.insert(fusion_id, info);
    }

    /// Get all invariants.
    pub fn invariants(&self) -> &[LoopInvariant] {
        &self.invariants
    }

    /// Generate SMT constraints for all invariants.
    pub fn generate_invariant_constraints(&self) -> Vec<Term> {
        self.invariants.iter().map(|inv| inv.to_smt()).collect()
    }

    /// Generate SMT declarations for loop indices.
    pub fn generate_declarations(&self, script: &mut crate::smtlib::Script) {
        for inv in &self.invariants {
            for (name, sort) in &inv.vars {
                script.declare_const(name, sort.clone());
            }
        }
    }
}

impl Default for PolyhedralTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode polyhedral constructs for a function.
pub fn encode_polyhedral_function(
    func: &naso_compiler::ast::Function,
    tracker: &mut PolyhedralTracker,
) -> Result<Vec<Term>, VerifyError> {
    let mut constraints = Vec::new();

    if let Some(body) = &func.body {
        constraints.extend(encode_polyhedral_expr(body, tracker)?);
    }

    // Add all invariant constraints
    constraints.extend(tracker.generate_invariant_constraints());

    Ok(constraints)
}

/// Encode polyhedral constructs in an expression.
pub fn encode_polyhedral_expr(
    expr: &naso_compiler::ast::Expr,
    tracker: &mut PolyhedralTracker,
) -> Result<Vec<Term>, VerifyError> {
    let mut constraints = Vec::new();

    match expr {
        naso_compiler::ast::Expr::Forall {
            index,
            domain,
            body,
            span,
        } => {
            // Parse domain bounds (simplified)
            let mut iter_domain = IterationDomain::new(vec![index.clone()]);

            // Extract bounds from domain expression
            // For now, assume simple 0 <= index < N
            if let naso_compiler::ast::Expr::Range { start, end, .. } = domain.as_ref() {
                if let (Some(start_e), Some(end_e)) = (start, end) {
                    // start <= index
                    if let naso_compiler::ast::Expr::LitInt(s, _, _) = start_e.as_ref() {
                        iter_domain.add_constraint(vec![1], -(*s as i64)); // i - s >= 0
                    }
                    // index < end  =>  index - end + 1 <= 0  =>  -index + end - 1 >= 0
                    if let naso_compiler::ast::Expr::LitInt(e, _, _) = end_e.as_ref() {
                        iter_domain.add_constraint(vec![-1], (*e as i64) - 1); // -i + e - 1 >= 0
                    }
                }
            }

            tracker.enter_forall(index.clone(), iter_domain.clone());

            // Process body
            constraints.extend(encode_polyhedral_expr(body, tracker)?);

            tracker.exit_forall();
        }
        naso_compiler::ast::Expr::Tile {
            loop_id,
            tile_sizes,
            body,
            ..
        } => {
            // Register tiling
            let info = TilingInfo {
                original_indices: vec![loop_id.clone()],
                tile_sizes: tile_sizes.clone(),
                tile_indices: vec![format!("{}_tile", loop_id)],
                intra_indices: vec![format!("{}_inner", loop_id)],
            };
            tracker.register_tiling(loop_id.clone(), info);

            constraints.extend(encode_polyhedral_expr(body, tracker)?);
        }
        naso_compiler::ast::Expr::Fuse { loop_ids, body, .. } => {
            // Register fusion
            let info = FusionInfo {
                loop_ids: loop_ids.clone(),
                fused_domain: IterationDomain::new(loop_ids.clone()),
                schedule: IndexMap::new(),
            };
            tracker.register_fusion(format!("fuse_{}", loop_ids.join("_")), info);

            constraints.extend(encode_polyhedral_expr(body, tracker)?);
        }
        naso_compiler::ast::Expr::Call(func, args, _) => {
            for arg in args {
                constraints.extend(encode_polyhedral_expr(arg, tracker)?);
            }
        }
        naso_compiler::ast::Expr::Let(bindings, body, _) => {
            for (_, _, init, _) in bindings {
                if let Some(init_expr) = init {
                    constraints.extend(encode_polyhedral_expr(init_expr, tracker)?);
                }
            }
            constraints.extend(encode_polyhedral_expr(body, tracker)?);
        }
        _ => {
            expr.visit_exprs(&mut |e| {
                if let Ok(cs) = encode_polyhedral_expr(e, tracker) {
                    constraints.extend(cs);
                }
            });
        }
    }

    Ok(constraints)
}

/// Trait for visiting expressions.
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
            | naso_compiler::ast::Expr::If(_, _, then_e, else_e, _)
            | naso_compiler::ast::Expr::Forall { body, .. }
            | naso_compiler::ast::Expr::Tile { body, .. }
            | naso_compiler::ast::Expr::Fuse { body, .. } => {
                // Delegate to children
            }
            _ => {}
        }
    }
}
