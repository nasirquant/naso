//! Mutable Value Semantics (MVS) encoding for SMT-LIB2.
//!
//! This module translates Naso's `inout` parameters and mutable value semantics
//! into frame conditions and disjointness guarantees in SMT-LIB2.

use crate::error::{LoweringError, VerifyError};
use crate::smtlib::{Sort, Term, builder::*};
use indexmap::IndexMap;
use naso_compiler::ast::Span;

/// Represents an `inout` parameter with its frame condition.
#[derive(Debug, Clone)]
pub struct InoutParam {
    pub name: String,
    pub span: Span,
    pub ty: naso_compiler::ast::Type,
    /// The frame: set of memory locations this inout may modify
    pub frame: Frame,
}

/// Frame condition: a set of disjoint memory regions that may be modified.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    /// Named regions (e.g., "array_data", "struct_field_x")
    pub regions: IndexMap<String, Region>,
    /// Whether the frame is open (may modify unknown regions)
    pub is_open: bool,
}

impl Frame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_region(&mut self, name: String, region: Region) {
        self.regions.insert(name, region);
    }

    pub fn is_disjoint_from(&self, other: &Frame) -> bool {
        if self.is_open || other.is_open {
            return false; // Conservative: open frames may overlap
        }
        // Check all region pairs for overlap
        for (_, r1) in &self.regions {
            for (_, r2) in &other.regions {
                if r1.overlaps(r2) {
                    return false;
                }
            }
        }
        true
    }
}

/// A memory region with base and bounds.
#[derive(Debug, Clone)]
pub struct Region {
    pub base: Term,         // Base address/offset
    pub size: Term,         // Size in bytes/elements
    pub element_sort: Sort, // Element type
}

impl Region {
    pub fn new(base: Term, size: Term, element_sort: Sort) -> Self {
        Self {
            base,
            size,
            element_sort,
        }
    }

    /// Check if this region overlaps with another.
    pub fn overlaps(&self, other: &Region) -> bool {
        // Conservative: if we can't prove disjoint, assume overlap
        // In SMT, this would be a constraint: (overlap r1 r2) = (r1.base < r2.base + r2.size) && (r2.base < r1.base + r1.size)
        // For now, return true (assume overlap) unless we have concrete disjointness info
        true
    }

    /// Generate SMT constraint for disjointness from another region.
    pub fn disjoint_from(&self, other: &Region) -> Term {
        // disjoint = (self.base + self.size <= other.base) OR (other.base + other.size <= self.base)
        let self_end = add(vec![self.base.clone(), self.size.clone()]);
        let other_end = add(vec![other.base.clone(), other.size.clone()]);

        or(vec![
            le(self_end, other.base.clone()),
            le(other_end, self.base.clone()),
        ])
    }
}

/// Tracks MVS state during lowering.
pub struct MvsTracker {
    /// Active inout parameters in current scope
    inout_params: IndexMap<String, InoutParam>,
    /// Frame conditions for each function
    function_frames: IndexMap<String, Frame>,
    /// Current function being processed
    current_function: Option<String>,
}

impl MvsTracker {
    pub fn new() -> Self {
        Self {
            inout_params: IndexMap::new(),
            function_frames: IndexMap::new(),
            current_function: None,
        }
    }

    /// Enter a function scope.
    pub fn enter_function(&mut self, name: String) {
        self.current_function = Some(name.clone());
        self.function_frames.insert(name, Frame::new());
    }

    /// Exit current function scope.
    pub fn exit_function(&mut self) -> Option<Frame> {
        let name = self.current_function.take()?;
        self.function_frames.remove(&name)
    }

    /// Register an inout parameter.
    pub fn register_inout(&mut self, param: InoutParam) {
        let frame = param.frame.clone();
        self.inout_params.insert(param.name.clone(), param);

        // Add to function frame
        if let Some(func) = &self.current_function {
            if let Some(func_frame) = self.function_frames.get_mut(func) {
                for (name, region) in frame.regions {
                    func_frame.add_region(name, region);
                }
            }
        }
    }

    /// Get current function's frame.
    pub fn current_frame(&self) -> Option<&Frame> {
        self.current_function
            .as_ref()
            .and_then(|f| self.function_frames.get(f))
    }

    /// Check if two inout parameters have disjoint frames.
    pub fn check_disjoint(&self, param1: &str, param2: &str) -> Result<bool, VerifyError> {
        let p1 = self.inout_params.get(param1).ok_or_else(|| {
            VerifyError::Lowering(LoweringError::MvsError {
                span: Span::dummy(),
                msg: format!("Unknown inout parameter: {}", param1),
            })
        })?;
        let p2 = self.inout_params.get(param2).ok_or_else(|| {
            VerifyError::Lowering(LoweringError::MvsError {
                span: Span::dummy(),
                msg: format!("Unknown inout parameter: {}", param2),
            })
        })?;

        Ok(p1.frame.is_disjoint_from(&p2.frame))
    }

    /// Generate frame condition axioms for current function.
    pub fn generate_frame_axioms(&self) -> Vec<Term> {
        let mut axioms = Vec::new();

        if let Some(frame) = self.current_frame() {
            // For each pair of regions in the frame, assert disjointness
            let region_names: Vec<&String> = frame.regions.keys().collect();
            for i in 0..region_names.len() {
                for j in (i + 1)..region_names.len() {
                    let r1 = &frame.regions[region_names[i]];
                    let r2 = &frame.regions[region_names[j]];
                    axioms.push(r1.disjoint_from(r2));
                }
            }
        }

        axioms
    }

    /// Generate SMT declarations for inout parameters.
    pub fn generate_declarations(&self, script: &mut crate::smtlib::Script) {
        for (name, param) in &self.inout_params {
            // Declare the inout parameter as an array (memory model)
            let array_sort = Sort::Array(Box::new(Sort::Int), Box::new(Sort::Int)); // Simplified
            script.declare_const(&format!("inout_{}", name), array_sort);

            // Declare frame regions
            for (rname, region) in &param.frame.regions {
                script.declare_const(&format!("frame_{}_{}", name, rname), Sort::Int); // base
                script.declare_const(&format!("frame_{}_{}_size", name, rname), Sort::Int); // size
            }
        }
    }
}

impl Default for MvsTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode MVS constraints for a function with inout parameters.
pub fn encode_mvs_function(
    func: &naso_compiler::ast::Function,
    tracker: &mut MvsTracker,
) -> Result<Vec<Term>, VerifyError> {
    let mut constraints = Vec::new();

    // Enter function scope
    tracker.enter_function(func.name.clone());

    // Register inout parameters
    for param in &func.params {
        if param.is_inout {
            let region_name = format!("param_{}", param.name);
            let base = var(&format!("inout_{}_base", param.name), Sort::Int);
            let size = var(&format!("inout_{}_size", param.name), Sort::Int);
            let region = Region::new(base, size, Sort::Int);

            let mut frame = Frame::new();
            frame.add_region(region_name, region);

            tracker.register_inout(InoutParam {
                name: param.name.clone(),
                span: param.span,
                ty: param.ty.clone(),
                frame,
            });
        }
    }

    // Generate frame axioms (disjointness of inout regions)
    constraints.extend(tracker.generate_frame_axioms());

    // Process function body
    if let Some(body) = &func.body {
        constraints.extend(encode_mvs_expr(body, tracker)?);
    }

    // Exit function scope
    tracker.exit_function();

    Ok(constraints)
}

/// Encode MVS constraints for an expression.
pub fn encode_mvs_expr(
    expr: &naso_compiler::ast::Expr,
    tracker: &mut MvsTracker,
) -> Result<Vec<Term>, VerifyError> {
    let mut constraints = Vec::new();

    match expr {
        naso_compiler::ast::Expr::Inout {
            target,
            value,
            span,
        } => {
            // Inout assignment: target <- value
            // Frame condition: target's frame must be disjoint from all other inout frames
            if let naso_compiler::ast::Expr::Var(name, _, _) = target.as_ref() {
                if let Some(param) = tracker.inout_params.get(name) {
                    // Check disjointness from all other inout params
                    for (other_name, other_param) in &tracker.inout_params {
                        if other_name != name {
                            if !param.frame.is_disjoint_from(&other_param.frame) {
                                return Err(VerifyError::Lowering(LoweringError::MvsError {
                                    span: *span,
                                    msg: format!(
                                        "inout parameter '{}' may alias with '{}'",
                                        name, other_name
                                    ),
                                }));
                            }
                        }
                    }

                    // Generate frame condition: modified locations must be within frame
                    // This is a simplified encoding; real implementation uses memory model
                    let target_var = var(&format!("inout_{}", name), Sort::Int);
                    let frame_base = var(
                        &format!("frame_{}_param_{}_base", func_name(), name),
                        Sort::Int,
                    );
                    let frame_size = var(
                        &format!("frame_{}_param_{}_size", func_name(), name),
                        Sort::Int,
                    );

                    // Assert: target is within frame
                    constraints.push(and(vec![
                        ge(target_var.clone(), frame_base),
                        le(target_var, add(vec![frame_base, frame_size])),
                    ]));
                }
            }
        }
        naso_compiler::ast::Expr::Call(func, args, _) => {
            // Check if calling a function with inout params
            if let naso_compiler::ast::Expr::Var(fname, _, _) = func.as_ref() {
                // Would need to look up callee's frame and check disjointness
            }
            for arg in args {
                constraints.extend(encode_mvs_expr(arg, tracker)?);
            }
        }
        naso_compiler::ast::Expr::Let(bindings, body, _) => {
            for (_, _, init, _) in bindings {
                if let Some(init_expr) = init {
                    constraints.extend(encode_mvs_expr(init_expr, tracker)?);
                }
            }
            constraints.extend(encode_mvs_expr(body, tracker)?);
        }
        _ => {
            // Recurse
            expr.visit_exprs(&mut |e| {
                if let Ok(cs) = encode_mvs_expr(e, tracker) {
                    constraints.extend(cs);
                }
            });
        }
    }

    Ok(constraints)
}

/// Helper to get current function name (placeholder).
fn func_name() -> String {
    "current".to_string()
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
            | naso_compiler::ast::Expr::If(_, _, then_e, else_e, _) => {
                // Delegate to children
            }
            _ => {}
        }
    }
}
